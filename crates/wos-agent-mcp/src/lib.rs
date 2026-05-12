// Rust guideline compliant 2026-02-21

//! MCP-server [`AgentInvoker`] adapter (skeleton).
//!
//! Implements [`wos_core::agent::AgentInvoker`] against [`InvokerSpec::Mcp`].
//! Each agent declaration names a `server` (resolved at deploy time) and a
//! `tool` on that server; the adapter forwards [`AgentTask`] to the named
//! tool.
//!
//! Body is `unimplemented!()` pending an MCP client crate decision.

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::AgentDeclaration;

/// MCP-server adapter.
#[derive(Debug, Default)]
pub struct McpInvoker {
    _private: (),
}

impl McpInvoker {
    /// Construct a stub adapter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentInvoker for McpInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        _task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::Mcp { .. }) => {
                unimplemented!(
                    "wos-agent-mcp is a skeleton; the MCP client integration is tracked as a \
                     follow-up to ADR 0064. See TODO.md."
                )
            }
            other => Err(AgentInvocationError::InvokerMismatch(format!(
                "McpInvoker bound to agent '{}' but declaration's invoker.kind is {:?}; \
                 deployment routed the wrong kind to this adapter (expected `{}`)",
                decl.id,
                other.map(InvokerKind::from),
                InvokerKind::Mcp.as_str()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::AgentType;

    #[test]
    fn refuses_non_mcp_spec() {
        let inv = McpInvoker::new();
        let decl = AgentDeclaration {
            id: "a".into(),
            kind: "agent".into(),
            agent_type: AgentType::Deterministic,
            model_identifier: "m".into(),
            model_version: "1".into(),
            description: None,
            capabilities: vec![],
            model_version_policy: None,
            confidence_decay: None,
            fallback_chain: vec![],
            cascading_invocations: vec![],
            deontic_constraints: None,
            invoker: Some(InvokerSpec::Anthropic {
                model: "x".into(),
                config_ref: None,
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
