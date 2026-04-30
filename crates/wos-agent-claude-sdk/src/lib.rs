// Rust guideline compliant 2026-02-21

//! Claude Agent SDK [`AgentInvoker`] adapter (skeleton).
//!
//! Implements [`wos_core::agent::AgentInvoker`] against
//! [`InvokerSpec::ClaudeAgentSdk`]. The body is `unimplemented!()` until the
//! Claude Agent SDK integration lands; the crate exists now so deployments
//! can declare the dependency and the workspace declares the adapter
//! surface (ADR 0064 §5).
//!
//! Routing contract mirrors `wos-agent-anthropic`: refuse non-matching
//! `InvokerSpec` with [`AgentInvocationError::InvokerMismatch`].

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::AgentDeclaration;

/// Claude Agent SDK adapter.
#[derive(Debug, Default)]
pub struct ClaudeAgentSdkInvoker {
    _private: (),
}

impl ClaudeAgentSdkInvoker {
    /// Construct a stub adapter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentInvoker for ClaudeAgentSdkInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        _task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::ClaudeAgentSdk { .. }) => {
                unimplemented!(
                    "wos-agent-claude-sdk is a skeleton; the Claude Agent SDK integration is \
                     tracked as a follow-up to ADR 0064. See TODO.md."
                )
            }
            other => Err(AgentInvocationError::InvokerMismatch(format!(
                "ClaudeAgentSdkInvoker bound to agent '{}' but declaration's invoker.kind is \
                 {:?}; deployment routed the wrong kind to this adapter (expected `{}`)",
                decl.id,
                other.map(InvokerKind::from),
                InvokerKind::ClaudeAgentSdk.as_str()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::AgentType;

    #[test]
    fn refuses_non_claude_sdk_spec() {
        let inv = ClaudeAgentSdkInvoker::new();
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
            invoker: Some(InvokerSpec::Stub { responses: vec![] }),
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
            instance_id: "i",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv.invoke(&decl, &task, &ctx).unwrap_err();
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }
}
