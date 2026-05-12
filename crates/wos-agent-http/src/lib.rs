// Rust guideline compliant 2026-02-21

//! Generic HTTP / OpenAPI [`AgentInvoker`] adapter (skeleton).
//!
//! Implements [`wos_core::agent::AgentInvoker`] against [`InvokerSpec::Http`].
//! Used for deployments where the agent service exposes an OpenAPI/JSON
//! Schema endpoint and the deployer wants to keep WOS substrate-portable
//! without committing to MCP, A2A, or a vendor SDK.
//!
//! Body is `unimplemented!()` pending the HTTP client crate decision (likely
//! `reqwest`/`hyper`) and idempotency-key handling.

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::AgentDeclaration;

/// Generic HTTP/OpenAPI adapter.
#[derive(Debug, Default)]
pub struct HttpInvoker {
    _private: (),
}

impl HttpInvoker {
    /// Construct a stub adapter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentInvoker for HttpInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        _task: &AgentTask,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::Http { .. }) => {
                unimplemented!(
                    "wos-agent-http is a skeleton; the HTTP/OpenAPI client integration is tracked \
                     as a follow-up to ADR 0064. See TODO.md."
                )
            }
            other => Err(AgentInvocationError::InvokerMismatch(format!(
                "HttpInvoker bound to agent '{}' but declaration's invoker.kind is {:?}; \
                 deployment routed the wrong kind to this adapter (expected `{}`)",
                decl.id,
                other.map(InvokerKind::from),
                InvokerKind::Http.as_str()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::AgentType;

    #[test]
    fn refuses_non_http_spec() {
        let inv = HttpInvoker::new();
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
            invoker: Some(InvokerSpec::A2A {
                orchestrator: "x".into(),
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
