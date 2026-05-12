// Rust guideline compliant 2026-02-21

//! Deterministic in-memory `AgentInvoker` adapter used by WOS conformance
//! fixtures (ADR 0064).
//!
//! `StubInvoker` resolves invocations against the canned responses declared
//! on `InvokerSpec::Stub`. It is the canonical reference implementation of
//! the `AgentInvoker` port for trace-parity testing: the response stream is
//! a function of `(agent_id, capability_id, invocation_index)`, so fixtures
//! reproduce byte-for-byte.
//!
//! Production substrate adapters live in separate crates
//! (`wos-agent-anthropic`, `wos-agent-mcp`, …) and are tracked as follow-ups
//! in `TODO.md`. They share the spec port defined in `wos_core::agent`.

use std::collections::HashMap;
use std::sync::Mutex;

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerSpec,
    StubResponse,
};
use wos_core::model::ai::AgentDeclaration;

/// Deterministic stub adapter. Tracks per-`(agent_id, capability_id)`
/// invocation counts internally so the runtime does not need to thread the
/// counter through every dispatch site.
#[derive(Default)]
pub struct StubInvoker {
    counters: Mutex<HashMap<(String, String), u32>>,
}

impl StubInvoker {
    /// Construct a fresh stub invoker with all counters at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all per-`(agent, capability)` invocation counters. Useful between
    /// fixture runs in the same process.
    pub fn reset(&self) {
        if let Ok(mut counters) = self.counters.lock() {
            counters.clear();
        }
    }

    fn next_index(&self, agent_id: &str, capability_id: &str) -> u32 {
        let mut counters = self
            .counters
            .lock()
            .expect("StubInvoker counters poisoned — a previous invocation panicked");
        let key = (agent_id.to_string(), capability_id.to_string());
        let entry = counters.entry(key).or_insert(0);
        let index = *entry;
        *entry += 1;
        index
    }
}

impl AgentInvoker for StubInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        let responses = match decl.invoker.as_ref() {
            Some(InvokerSpec::Stub { responses }) => responses,
            Some(other) => {
                return Err(AgentInvocationError::InvokerMismatch(format!(
                    "StubInvoker bound but agent '{}' declares invoker.kind != 'stub' \
                     (got {other:?}); deployment binding is wrong",
                    decl.id
                )));
            }
            None => {
                return Err(AgentInvocationError::InvokerMismatch(format!(
                    "agent '{}' has no `invoker` declared; StubInvoker requires \
                     InvokerSpec::Stub with canned responses",
                    decl.id
                )));
            }
        };

        let invocation_index = self.next_index(&decl.id, &task.capability_id);

        // Resolve the canned response: prefer responses keyed to this
        // capability id; fall back to capability-agnostic responses (capability_id
        // omitted). Within each filter, take the response at `invocation_index`
        // (zero-based) so successive invocations of the same capability stream
        // through the declarations in document order.
        let response =
            pick_response(responses, &task.capability_id, invocation_index).ok_or_else(|| {
                AgentInvocationError::StubExhausted {
                    capability_id: task.capability_id.clone(),
                    invocation_index,
                }
            })?;

        Ok(AgentResult {
            output: response.output.clone(),
            confidence: response.confidence.unwrap_or(1.0),
            citations: response.citations.clone(),
            telemetry: None,
        })
    }
}

fn pick_response<'a>(
    responses: &'a [StubResponse],
    capability_id: &str,
    invocation_index: u32,
) -> Option<&'a StubResponse> {
    // Phase 1: capability-targeted responses, in declaration order.
    let targeted: Vec<&StubResponse> = responses
        .iter()
        .filter(|r| r.capability_id.as_deref() == Some(capability_id))
        .collect();
    if !targeted.is_empty() {
        return targeted.get(invocation_index as usize).copied();
    }
    // Phase 2: capability-agnostic responses (capabilityId omitted).
    let agnostic: Vec<&StubResponse> = responses
        .iter()
        .filter(|r| r.capability_id.is_none())
        .collect();
    agnostic.get(invocation_index as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::{AgentDeclaration, AgentType};

    fn agent_with_stub(invoker: InvokerSpec) -> AgentDeclaration {
        AgentDeclaration {
            id: "triageAgent".into(),
            kind: "agent".into(),
            agent_type: AgentType::Generative,
            model_identifier: "test-model".into(),
            model_version: "1.0".into(),
            description: None,
            capabilities: vec![],
            model_version_policy: None,
            confidence_decay: None,
            fallback_chain: vec![],
            cascading_invocations: vec![],
            deontic_constraints: None,
            invoker: Some(invoker),
            extensions: Default::default(),
        }
    }

    fn task(capability: &str) -> AgentTask {
        AgentTask {
            capability_id: capability.into(),
            prompt: None,
            inputs: Default::default(),
            correlation_key: None,
        }
    }

    fn ctx<'a>(case_state: &'a serde_json::Value) -> AgentContext<'a> {
        AgentContext {
            process_id: "test_case_01abc",
            invocation_index: 0,
            case_state,
        }
    }

    #[test]
    fn stub_returns_canned_response_for_targeted_capability() {
        let agent = agent_with_stub(InvokerSpec::Stub {
            responses: vec![StubResponse {
                capability_id: Some("triage".into()),
                output: serde_json::json!({"category": "benefits"}),
                confidence: Some(0.92),
                citations: vec![],
            }],
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let result = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .unwrap();
        assert_eq!(result.output["category"], "benefits");
        assert_eq!(result.confidence, 0.92);
    }

    #[test]
    fn stub_streams_through_responses_in_declaration_order() {
        let agent = agent_with_stub(InvokerSpec::Stub {
            responses: vec![
                StubResponse {
                    capability_id: Some("triage".into()),
                    output: serde_json::json!({"category": "benefits"}),
                    confidence: Some(0.9),
                    citations: vec![],
                },
                StubResponse {
                    capability_id: Some("triage".into()),
                    output: serde_json::json!({"category": "appeals"}),
                    confidence: Some(0.7),
                    citations: vec![],
                },
            ],
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let first = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .unwrap();
        let second = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .unwrap();
        assert_eq!(first.output["category"], "benefits");
        assert_eq!(second.output["category"], "appeals");
    }

    #[test]
    fn stub_falls_back_to_capability_agnostic_response_when_no_targeted_match() {
        let agent = agent_with_stub(InvokerSpec::Stub {
            responses: vec![StubResponse {
                capability_id: None,
                output: serde_json::json!({"any": true}),
                confidence: None,
                citations: vec![],
            }],
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let result = invoker
            .invoke(&agent, &task("anyCapability"), &ctx(&case_state))
            .unwrap();
        assert_eq!(result.output["any"], true);
        assert_eq!(
            result.confidence, 1.0,
            "missing confidence in StubResponse MUST default to 1.0"
        );
    }

    #[test]
    fn stub_exhausted_error_when_responses_run_out() {
        let agent = agent_with_stub(InvokerSpec::Stub {
            responses: vec![StubResponse {
                capability_id: Some("triage".into()),
                output: serde_json::json!({"ok": true}),
                confidence: None,
                citations: vec![],
            }],
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .unwrap();
        let err = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .expect_err("second invocation MUST exhaust the stub");
        match err {
            AgentInvocationError::StubExhausted {
                capability_id,
                invocation_index,
            } => {
                assert_eq!(capability_id, "triage");
                assert_eq!(invocation_index, 1);
            }
            other => panic!("expected StubExhausted, got {other:?}"),
        }
    }

    #[test]
    fn stub_invoker_mismatch_when_agent_uses_different_invoker_spec() {
        let agent = agent_with_stub(InvokerSpec::Anthropic {
            model: "claude-opus-4-7".into(),
            config_ref: None,
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let err = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .expect_err("StubInvoker MUST refuse non-stub InvokerSpec");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }

    #[test]
    fn stub_invoker_mismatch_when_agent_has_no_invoker() {
        let mut agent = agent_with_stub(InvokerSpec::Stub { responses: vec![] });
        agent.invoker = None;
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let err = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .expect_err("StubInvoker MUST refuse agents without invoker declarations");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }

    #[test]
    fn stub_per_capability_counters_are_independent() {
        let agent = agent_with_stub(InvokerSpec::Stub {
            responses: vec![
                StubResponse {
                    capability_id: Some("triage".into()),
                    output: serde_json::json!({"who": "triage-0"}),
                    confidence: None,
                    citations: vec![],
                },
                StubResponse {
                    capability_id: Some("classify".into()),
                    output: serde_json::json!({"who": "classify-0"}),
                    confidence: None,
                    citations: vec![],
                },
            ],
        });
        let invoker = StubInvoker::new();
        let case_state = serde_json::json!({});
        let triage = invoker
            .invoke(&agent, &task("triage"), &ctx(&case_state))
            .unwrap();
        let classify = invoker
            .invoke(&agent, &task("classify"), &ctx(&case_state))
            .unwrap();
        assert_eq!(triage.output["who"], "triage-0");
        assert_eq!(classify.output["who"], "classify-0");
    }
}
