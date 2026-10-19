// Rust guideline compliant 2026-02-21

//! Explicitly unsupported Anthropic agent invocation adapter.
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
            Some(InvokerSpec::Anthropic { .. }) => Err(AgentInvocationError::Substrate(format!(
                "unsupported adapter execution/model mapping for admitted {}@{}; owning provider configuration/resolution operation is unavailable",
                decl.model_identifier, decl.model_version
            ))),
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
        let mut source = serde_json::json!({"id":"extractor","agentType":AgentType::Generative,"modelIdentifier":"m","modelVersion":"1"});
        if let Some(invoker) = invoker {
            source["invoker"] = serde_json::to_value(invoker).unwrap();
        }
        serde_json::from_value(source).unwrap()
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
        let decl = agent(Some(InvokerSpec::Stub {
            responses: vec![],
            metadata: Default::default(),
        }));
        let case = serde_json::json!({});
        let ctx = AgentContext {
            process_id: "i",
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
            process_id: "i",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must refuse missing invoker");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }
}
