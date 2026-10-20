//! The synthesis loop — generate → lint → repair → repeat.
//!
//! Pure orchestration over the [`crate::Prompter`] and [`crate::ToolContext`]
//! traits. No network code, no provider-specific code, no IO beyond what the
//! traits perform internally.

use serde::{Deserialize, Serialize};

use crate::errors::SynthError;
use crate::prompter::{Completion, Prompter};
use crate::prompts::{Layer, build_generate_prompt, build_repair_prompt};
use crate::tool_context::{CheckDisposition, LintFinding, ToolContext};
use crate::trace::{IterationRecord, RunInterruption, SynthTrace};

/// Result of running the loop to completion (or to the iteration cap).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SynthOutcome {
    /// All requested synthesis checks passed, including independent replay.
    Converged {
        /// The final document as raw JSON text.
        document: String,
        /// Full loop trace.
        trace: SynthTrace,
    },
    /// Explicit static-only draft; requested static checks passed, behavior remains unchecked.
    Draft {
        /// The draft document as raw JSON text.
        document: String,
        /// Full evidence, including replay not-run.
        trace: SynthTrace,
    },
    /// Provider repair failed after checks ran; their complete evidence remains available.
    Interrupted {
        /// Last checked candidate, unchanged by the failed provider call.
        last_attempt: String,
        /// Full evidence and original interruption detail.
        trace: SynthTrace,
    },
    /// Iteration cap hit with required checks still incomplete or unsuccessful.
    Unconverged {
        /// Last attempt the loop produced.
        last_attempt: String,
        /// Findings still outstanding.
        last_findings: Vec<LintFinding>,
        /// Full loop trace.
        trace: SynthTrace,
    },
}

/// Run the synthesis loop.
///
/// `max_iterations` is the total cap including the initial generation call;
/// values < 1 are treated as 1.
pub async fn synthesize(
    provider: &dyn Prompter,
    tools: &dyn ToolContext,
    problem: &str,
    layer: Layer,
    max_iterations: u32,
) -> Result<SynthOutcome, SynthError> {
    let cap = max_iterations.max(1);
    let mut trace = SynthTrace::new();

    // Iteration 0 — initial generation.
    let (mut attempt, mut completion) = generate(provider, problem, layer).await?;

    for index in 0..cap {
        let assessment = tools.check_candidate(&attempt).await?;
        let disposition = assessment.disposition;
        let findings = assessment.findings.clone();

        trace.push(IterationRecord {
            index,
            attempt: attempt.clone(),
            assessment,
            input_tokens: completion.input_tokens,
            output_tokens: completion.output_tokens,
            cache_read_tokens: completion.cache_read_tokens,
        });

        if disposition == CheckDisposition::DraftOnly {
            return Ok(SynthOutcome::Draft {
                document: attempt,
                trace,
            });
        }
        if disposition == CheckDisposition::ChecksPassed {
            return Ok(SynthOutcome::Converged {
                document: attempt,
                trace,
            });
        }

        if index + 1 >= cap {
            return Ok(SynthOutcome::Unconverged {
                last_attempt: attempt,
                last_findings: findings,
                trace,
            });
        }

        let (next_attempt, next_completion) =
            match repair(provider, &attempt, &findings, layer).await {
                Ok(completion) => completion,
                Err(error) => {
                    trace.interruption = Some(RunInterruption {
                        operation: "repair_provider".into(),
                        error: error.to_string(),
                    });
                    return Ok(SynthOutcome::Interrupted {
                        last_attempt: attempt,
                        trace,
                    });
                }
            };
        attempt = next_attempt;
        completion = next_completion;
    }

    // Cap == 0 shouldn't reach here because we clamped above, but keep the
    // type system happy.
    Ok(SynthOutcome::Unconverged {
        last_attempt: attempt,
        last_findings: Vec::new(),
        trace,
    })
}

async fn generate(
    provider: &dyn Prompter,
    problem: &str,
    layer: Layer,
) -> Result<(String, Completion), SynthError> {
    let (system, user, anchors) = build_generate_prompt(problem, layer);
    let completion = provider.complete(&system, &user, &anchors).await?;
    let text = strip_fences(&completion.text).to_string();
    Ok((text, completion))
}

async fn repair(
    provider: &dyn Prompter,
    prior: &str,
    findings: &[LintFinding],
    layer: Layer,
) -> Result<(String, Completion), SynthError> {
    let (system, user, anchors) = build_repair_prompt(prior, findings, layer);
    let completion = provider.complete(&system, &user, &anchors).await?;
    let text = strip_fences(&completion.text).to_string();
    Ok((text, completion))
}

/// Strip optional fenced-code wrappers the model may add.
///
/// Handles four shapes the model emits in practice:
///   1. ` ```json\n{...}\n``` ` (canonical fenced code block)
///   2. ` ```\n{...}\n``` ` (no language tag)
///   3. ` ```wos\n{...}\n``` ` (non-JSON language tag)
///   4. bare `{...}` (no fence)
fn strip_fences(text: &str) -> &str {
    let trimmed = text.trim();

    let Some(inner) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let Some(body) = inner.strip_suffix("```") else {
        // Open fence without close — treat the whole string as content.
        return trimmed;
    };

    // After the opening ``` the model may include a language tag (`json`,
    // `javascript`, `wos`, ...) optionally followed by whitespace.
    let body = body.trim();
    let language_stripped = strip_fence_language(body);

    language_stripped.trim()
}

fn strip_fence_language(body: &str) -> &str {
    let Some(first) = body.chars().next() else {
        return body;
    };
    if first == '{' || first == '[' {
        return body;
    }

    let Some(json_start) = body.find(['{', '[']) else {
        return body;
    };
    let language = body[..json_start].trim();
    if language.is_empty()
        || language
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+'))
    {
        return &body[json_start..];
    }

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompter::{CacheAnchor, PrompterError};
    use crate::tool_context::{CandidateAssessment, CheckRecord, Severity, ToolError, ToolResult};
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Test prompter that returns a FIFO queue of canned responses.
    ///
    /// FIFO (`pop_front`) so test bodies read in chronological order — the
    /// first declared response is what the loop sees on its first call.
    struct ScriptedPrompter {
        responses: Mutex<VecDeque<String>>,
    }

    impl ScriptedPrompter {
        fn new(responses: Vec<&str>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().map(String::from).collect()),
            }
        }
    }

    #[async_trait]
    impl Prompter for ScriptedPrompter {
        async fn complete(
            &self,
            _system: &str,
            _user: &str,
            _anchors: &[CacheAnchor],
        ) -> Result<Completion, PrompterError> {
            let next = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| PrompterError::UnexpectedPrompt("queue empty".into()))?;
            Ok(Completion {
                text: next,
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 3,
            })
        }
    }

    /// Test ToolContext with a FIFO queue of finding sets.
    struct ScriptedTools {
        finding_sets: Mutex<VecDeque<Vec<LintFinding>>>,
    }

    impl ScriptedTools {
        fn new(sets: Vec<Vec<LintFinding>>) -> Self {
            Self {
                finding_sets: Mutex::new(sets.into()),
            }
        }
    }

    #[async_trait]
    impl ToolContext for ScriptedTools {
        async fn check_candidate(&self, _doc: &str) -> Result<CandidateAssessment, ToolError> {
            let findings = self
                .finding_sets
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            let disposition = if findings.is_empty() {
                CheckDisposition::ChecksPassed
            } else {
                CheckDisposition::Blocked
            };
            Ok(CandidateAssessment {
                intake_error: None,
                findings,
                validation: CheckRecord {
                    id: "static".into(),
                    arguments: serde_json::json!({}),
                    result: ToolResult::Completed {
                        report: serde_json::json!({"validation":{"status":"pass"}}),
                    },
                },
                replays: vec![CheckRecord {
                    id: "scripted".into(),
                    arguments: serde_json::json!({}),
                    result: ToolResult::Completed {
                        report: serde_json::json!({"passed":true}),
                    },
                }],
                disposition,
            })
        }
    }

    fn err(rule: &str) -> LintFinding {
        LintFinding {
            rule_id: rule.into(),
            severity: Severity::Error,
            message: "test".into(),
            path: None,
            suggested_fix: None,
            related_docs: vec![],
        }
    }

    #[test]
    fn converges_when_repair_succeeds() {
        // Iter 0: dirty doc + lint error → triggers repair.
        // Iter 1: clean doc + no lint findings → converged.
        let provider = ScriptedPrompter::new(vec![
            r#"{"$wosWorkflow":"1.0","dirty":true}"#,
            r#"{"$wosWorkflow":"1.0","clean":true}"#,
        ]);
        let tools = ScriptedTools::new(vec![vec![err("K-001")], vec![]]);

        let outcome = pollster::block_on(synthesize(
            &provider,
            &tools,
            "test problem",
            Layer::Kernel,
            5,
        ))
        .expect("loop should not error");

        match outcome {
            SynthOutcome::Converged { document, trace } => {
                assert!(document.contains("clean"), "got: {document}");
                assert_eq!(trace.iterations.len(), 2);
            }
            other => panic!("expected converged, got {other:?}"),
        }
    }

    #[test]
    fn unconverged_when_cap_hit() {
        let provider =
            ScriptedPrompter::new(vec![r#"{"a":4}"#, r#"{"a":3}"#, r#"{"a":2}"#, r#"{"a":1}"#]);
        let tools = ScriptedTools::new(vec![
            vec![err("K-001")],
            vec![err("K-001")],
            vec![err("K-001")],
            vec![err("K-001")],
        ]);

        let outcome = pollster::block_on(synthesize(&provider, &tools, "test", Layer::Kernel, 3))
            .expect("loop should not error");

        match outcome {
            SynthOutcome::Unconverged { trace, .. } => {
                assert_eq!(trace.iterations.len(), 3, "should hit cap exactly");
            }
            other => panic!("expected unconverged, got {other:?}"),
        }
    }

    #[test]
    fn block_finding_triggers_repair_and_prevents_convergence_at_cap() {
        for cap in [1, 2] {
            let provider = ScriptedPrompter::new(vec!["{}", "{\"repaired\":true}"]);
            let mut block = err("PUB-LINT-001");
            block.severity = serde_json::from_value(serde_json::json!("block")).unwrap();
            let tools = ScriptedTools::new(vec![vec![block], vec![]]);
            let outcome =
                pollster::block_on(synthesize(&provider, &tools, "test", Layer::Kernel, cap))
                    .unwrap();
            let trace = match outcome {
                SynthOutcome::Unconverged { trace, .. } if cap == 1 => trace,
                SynthOutcome::Converged { trace, .. } if cap == 2 => trace,
                other => panic!("block must prevent convergence until repaired: {other:?}"),
            };
            assert_eq!(trace.iterations.len(), cap as usize);
            assert_eq!(
                serde_json::to_value(&trace).unwrap()["iterations"][0]["assessment"]["findings"][0]
                    ["severity"],
                "block"
            );
        }
    }

    #[test]
    fn fences_stripped_from_attempt() {
        let provider = ScriptedPrompter::new(vec!["```json\n{\"$wosWorkflow\":\"1.0\"}\n```"]);
        let tools = ScriptedTools::new(vec![vec![]]);

        let outcome = pollster::block_on(synthesize(&provider, &tools, "test", Layer::Kernel, 1))
            .expect("loop should not error");

        match outcome {
            SynthOutcome::Converged { document, .. } => {
                assert_eq!(document, r#"{"$wosWorkflow":"1.0"}"#);
            }
            other => panic!("expected converged, got {other:?}"),
        }
    }

    #[test]
    fn strip_fences_handles_canonical_form() {
        assert_eq!(strip_fences("```json\n{\"a\":1}\n```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_handles_no_newline_after_language_tag() {
        // Single-line fenced block — the prior implementation silently
        // retained the literal `json` token. Repaired by Finding 4.
        assert_eq!(strip_fences("```json{\"a\":1}```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_handles_uppercase_language_tag() {
        assert_eq!(strip_fences("```JSON\n{\"a\":1}\n```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_handles_non_json_language_tag() {
        assert_eq!(strip_fences("```wos\n{\"a\":1}\n```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_handles_no_language_tag() {
        assert_eq!(strip_fences("```\n{\"a\":1}\n```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_passes_through_bare_json() {
        assert_eq!(strip_fences("{\"a\":1}"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_fences_passes_through_unclosed_fence() {
        // Open fence without close — return as-is rather than corrupting.
        let input = "```json\n{\"a\":1}";
        assert_eq!(strip_fences(input), input);
    }
}
