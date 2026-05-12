// Rust guideline compliant 2026-02-21

//! A2A multi-agent orchestrator [`AgentInvoker`] adapter (skeleton).
//!
//! Implements [`wos_core::agent::AgentInvoker`] against [`InvokerSpec::A2A`].
//! Multi-agent orchestrators are *one kind* of `AgentInvoker`, not a special
//! WOS concept (ADR 0064 §2.2): the orchestrator is the substrate; WOS sees a
//! single declaration and the orchestrator handles its own internal fan-out.
//!
//! Body is `unimplemented!()` pending the A2A client crate decision.

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::AgentDeclaration;

/// A2A multi-agent orchestrator adapter.
#[derive(Debug, Default)]
pub struct A2AInvoker {
    _private: (),
}

impl A2AInvoker {
    /// Construct a stub adapter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentInvoker for A2AInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        _task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::A2A { .. }) => {
                unimplemented!(
                    "wos-agent-a2a is a skeleton; the A2A orchestrator integration is tracked as \
                     a follow-up to ADR 0064. See TODO.md."
                )
            }
            other => Err(AgentInvocationError::InvokerMismatch(format!(
                "A2AInvoker bound to agent '{}' but declaration's invoker.kind is {:?}; \
                 deployment routed the wrong kind to this adapter (expected `{}`)",
                decl.id,
                other.map(InvokerKind::from),
                InvokerKind::A2A.as_str()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::AgentType;

    #[test]
    fn refuses_non_a2a_spec() {
        let inv = A2AInvoker::new();
        let decl = AgentDeclaration {
            id: "a".into(),
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
            invoker: Some(InvokerSpec::Mcp {
                server: "s".into(),
                tool: "t".into(),
            }),
            extensions: Default::default(),
        };
        let task = AgentTask {
            capability_id: "c".into(),
            prompt: None,
            inputs: Default::default(),
            correlation_key: None,
        };
        let case = serde_json::json!({});
        let ctx = AgentContext {
            process_id: "i",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv.invoke(&decl, &task, &ctx).unwrap_err();
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }
}
