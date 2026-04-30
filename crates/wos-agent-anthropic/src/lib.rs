// Rust guideline compliant 2026-02-21

//! Anthropic SDK [`AgentInvoker`] adapter (skeleton).
//!
//! Implements [`wos_core::agent::AgentInvoker`] against
//! [`InvokerSpec::Anthropic`]. The body is `unimplemented!()` until the
//! Anthropic SDK integration lands; the crate exists now so deployments can
//! declare the dependency, the workspace declares the adapter surface, and
//! `wos_core::AgentInvokerRegistry` can wire it without a forward reference.
//!
//! # Routing contract
//!
//! [`AnthropicInvoker::invoke`] MUST refuse to handle agents whose
//! `InvokerSpec` is anything other than [`InvokerSpec::Anthropic`]. The
//! defensive guard exists because deployment-time DI is the only thing that
//! routes a declaration to its adapter; a misconfiguration that bound this
//! adapter under the wrong [`InvokerKind`] should fail loudly with
//! [`AgentInvocationError::InvokerMismatch`] rather than silently produce
//! wrong outputs.
//!
//! # Status
//!
//! Skeleton. Real Anthropic SDK calls, retry semantics, prompt caching, and
//! tool-use surface land in a follow-up; tracked in `TODO.md`.

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::AgentDeclaration;

/// Anthropic SDK adapter.
#[derive(Debug, Default)]
pub struct AnthropicInvoker {
    _private: (),
}

impl AnthropicInvoker {
    /// Construct a stub adapter. Real configuration (API key resolution,
    /// retry policy, prompt cache scope) attaches in the follow-up.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentInvoker for AnthropicInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        _task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::Anthropic { .. }) => {
                unimplemented!(
                    "wos-agent-anthropic is a skeleton; the Anthropic SDK integration is tracked \
                     as a follow-up to ADR 0064. See TODO.md."
                )
            }
            other => Err(AgentInvocationError::InvokerMismatch(format!(
                "AnthropicInvoker bound to agent '{}' but declaration's invoker.kind is {:?}; \
                 deployment routed the wrong kind to this adapter (expected `{}`)",
                decl.id,
                other.map(InvokerKind::from),
                InvokerKind::Anthropic.as_str()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::{AgentDeclaration, AgentType};

    fn agent(invoker: Option<InvokerSpec>) -> AgentDeclaration {
        AgentDeclaration {
            id: "extractor".into(),
            kind: "agent".into(),
            agent_type: AgentType::Generative,
            model_identifier: "m".into(),
            model_version: "1".into(),
            description: None,
            capabilities: vec![],
            model_version_policy: None,
            confidence_decay: None,
            fallback_chain: vec![],
            cascading_invocations: vec![],
            deontic_constraints: None,
            invoker,
            extensions: Default::default(),
        }
    }

    fn task() -> AgentTask {
        AgentTask {
            capability_id: "c".into(),
            prompt: None,
            inputs: Default::default(),
            correlation_key: None,
        }
    }

    #[test]
    fn refuses_non_anthropic_spec_with_invoker_mismatch() {
        let inv = AnthropicInvoker::new();
        let decl = agent(Some(InvokerSpec::Stub { responses: vec![] }));
        let case = serde_json::json!({});
        let ctx = AgentContext {
            instance_id: "i",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must refuse Stub spec");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }

    #[test]
    fn refuses_missing_invoker_with_invoker_mismatch() {
        let inv = AnthropicInvoker::new();
        let decl = agent(None);
        let case = serde_json::json!({});
        let ctx = AgentContext {
            instance_id: "i",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must refuse missing invoker");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }
}
