//! The synthesis loop — generate → lint → repair → repeat.
//!
//! Pure orchestration over the [`crate::Prompter`] and [`crate::ToolContext`]
//! traits. No network code, no provider-specific code, no IO beyond what the
//! traits perform internally.

use serde::{Deserialize, Serialize};

use crate::errors::SynthError;
use crate::prompter::{Completion, Prompter};
use crate::prompts::{Layer, build_generate_prompt, build_repair_prompt};
use crate::tool_context::{LintFinding, Severity, ToolContext};
use crate::trace::{IterationRecord, SynthTrace};

/// Result of running the loop to completion (or to the iteration cap).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SynthOutcome {
    /// All error-severity findings cleared (and conformance passed if checked).
    Converged {
        /// The final document as raw JSON text.
        document: String,
        /// Full loop trace.
        trace: SynthTrace,
    },
    /// Iteration cap hit with errors still present.
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
        let findings = tools.lint_document(&attempt).await?;
        let conformance = tools.run_conformance(&attempt).await?;

        let errors_remain = findings.iter().any(|f| f.severity == Severity::Error);
        let conformance_blocks = matches!(&conformance, Some(v) if !v.passed);

        trace.push(IterationRecord {
            index,
            attempt: attempt.clone(),
            lint_findings: findings.clone(),
            conformance: conformance.clone(),
            input_tokens: completion.input_tokens,
            output_tokens: completion.output_tokens,
            cache_read_tokens: completion.cache_read_tokens,
        });

        if !errors_remain && !conformance_blocks {
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

        let (next_attempt, next_completion) = repair(provider, &attempt, &findings, layer).await?;
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
    use crate::tool_context::{ConformanceVerdict, ToolError};
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
        async fn lint_document(&self, _doc: &str) -> Result<Vec<LintFinding>, ToolError> {
            Ok(self
                .finding_sets
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default())
        }

        async fn run_conformance(
            &self,
            _doc: &str,
        ) -> Result<Option<ConformanceVerdict>, ToolError> {
            Ok(None)
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
            r#"{"$wosKernel":"1.0","dirty":true}"#,
            r#"{"$wosKernel":"1.0","clean":true}"#,
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
    fn fences_stripped_from_attempt() {
        let provider = ScriptedPrompter::new(vec!["```json\n{\"$wosKernel\":\"1.0\"}\n```"]);
        let tools = ScriptedTools::new(vec![vec![]]);

        let outcome = pollster::block_on(synthesize(&provider, &tools, "test", Layer::Kernel, 1))
            .expect("loop should not error");

        match outcome {
            SynthOutcome::Converged { document, .. } => {
                assert_eq!(document, r#"{"$wosKernel":"1.0"}"#);
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
