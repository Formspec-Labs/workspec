// Rust guideline compliant 2026-02-21

//! Explicitly unsupported A2A agent invocation adapter.
//!
//! The adapter rejects mismatched invoker kinds. Matching declarations return
//! an unsupported execution/model-mapping error until the provider integration
//! exists. The admitted agent declaration owns model identity; no duplicate
//! selector or successful invocation evidence is invented here.

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
            Some(InvokerSpec::A2A { .. }) => Err(AgentInvocationError::Substrate(format!(
                "unsupported adapter execution/model mapping for admitted {}@{}; owning provider configuration/resolution operation is unavailable",
                decl.model_identifier, decl.model_version
            ))),
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
        let decl = serde_json::from_value::<AgentDeclaration>(serde_json::json!({"id":"a","agentType":AgentType::Generative,"modelIdentifier":"m","modelVersion":"1","invoker":Some(InvokerSpec::Mcp {
                server: "s".into(),
                tool: "t".into(),
 metadata:Default::default(),
})})).unwrap();
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
