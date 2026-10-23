// Rust guideline compliant 2026-02-21

//! Anthropic agent invocation adapter (ADR 0064).
//!
//! WOS governs agents as first-class actors (`ActorKind::Agent`). This adapter
//! connects `AgentInvoker` to Anthropic's Claude models using `anthropic-sdk`.
//!
//! # Streaming Collection
//!
//! The adapter uses the streaming-callback pattern established in
//! `wos-synth-anthropic`: chunks are collected into an `Arc<Mutex<String>>`
//! and extracted via `std::mem::take` upon stream completion. Poisoned mutex
//! states are recovered rather than panicked to avoid dropping partial output.
//!
//! # Sync / Async Bridge
//!
//! `AgentInvoker::invoke` is synchronous. This adapter bridges to the async
//! Anthropic SDK using `tokio::task::block_in_place` when running inside an
//! existing Tokio multi-thread runtime, or spins up a dedicated current-thread
//! runtime otherwise.

use std::sync::{Arc, Mutex};

use wos_core::agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentResult, AgentTask, InvokerKind,
    InvokerSpec,
};
use wos_core::model::ai::{AgentDeclaration, AgentType, ConfidenceMethod, ConfidenceReport};

/// Callback hook type for testing streaming chunk collection without network dependency.
pub type StreamCollectorHook =
    Arc<dyn Fn(&AgentDeclaration, &AgentTask, &str) -> Result<Vec<String>, String> + Send + Sync>;

/// Anthropic SDK adapter.
#[derive(Clone, Default)]
pub struct AnthropicInvoker {
    api_key: Option<String>,
    stream_hook: Option<StreamCollectorHook>,
}

impl std::fmt::Debug for AnthropicInvoker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicInvoker")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("has_stream_hook", &self.stream_hook.is_some())
            .finish()
    }
}

impl AnthropicInvoker {
    /// Construct an Anthropic invoker.
    ///
    /// Reads `ANTHROPIC_API_KEY` from the environment if present.
    #[must_use]
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            stream_hook: None,
        }
    }

    /// Set an explicit API key.
    #[must_use]
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Provide a stream hook for testing chunk collection and error recovery offline.
    #[must_use]
    pub fn with_stream_hook(mut self, hook: StreamCollectorHook) -> Self {
        self.stream_hook = Some(hook);
        self
    }
}

impl AgentInvoker for AnthropicInvoker {
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        task: &AgentTask,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError> {
        match decl.invoker.as_ref() {
            Some(InvokerSpec::Anthropic { .. }) => {}
            other => {
                return Err(AgentInvocationError::InvokerMismatch(format!(
                    "AnthropicInvoker bound to agent '{}' but declaration's invoker.kind is {:?}; \
                     deployment routed the wrong kind to this adapter (expected `{}`)",
                    decl.id,
                    other.map(InvokerKind::from),
                    InvokerKind::Anthropic.as_str()
                )));
            }
        }

        let system_prompt = format!(
            "You are agent '{}' of type '{:?}' executing capability '{}' in workflow process '{}'.",
            decl.id, decl.agent_type, task.capability_id, ctx.process_id
        );

        let prompt = match &task.prompt {
            Some(p) => p.clone(),
            None => serde_json::to_string(&serde_json::json!({
                "capabilityId": task.capability_id,
                "inputs": task.inputs,
                "caseState": ctx.case_state,
            }))
            .unwrap_or_default(),
        };

        let collected: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let collector_for_callback = Arc::clone(&collected);

        let stream_hook = self.stream_hook.clone();
        let api_key = self.api_key.clone();
        let model_id = decl.model_identifier.clone();
        let decl_clone = decl.clone();
        let task_clone = task.clone();
        let prompt_for_async = prompt.clone();

        run_async(async move {
            if let Some(hook) = stream_hook {
                let chunks = hook(&decl_clone, &task_clone, &prompt_for_async)
                    .map_err(AgentInvocationError::Substrate)?;
                for chunk in chunks {
                    let collector = Arc::clone(&collector_for_callback);
                    match collector.lock() {
                        Ok(mut guard) => guard.push_str(&chunk),
                        Err(poisoned) => poisoned.into_inner().push_str(&chunk),
                    }
                }
                Ok(())
            } else {
                let key = api_key.unwrap_or_default();
                if key.trim().is_empty() {
                    return Err(AgentInvocationError::Substrate(
                        "Anthropic API key is not configured; set ANTHROPIC_API_KEY environment variable or supply via AnthropicInvoker::with_api_key".into(),
                    ));
                }

                let client = anthropic_sdk::Client::new()
                    .auth(&key)
                    .model(&model_id)
                    .max_tokens(4096)
                    .temperature(0.0)
                    .system(&system_prompt)
                    .messages(&serde_json::json!([
                        { "role": "user", "content": prompt_for_async }
                    ]))
                    .stream(true)
                    .build()
                    .map_err(|e| AgentInvocationError::Substrate(e.to_string()))?;

                client
                    .execute(move |chunk| {
                        let collector = Arc::clone(&collector_for_callback);
                        async move {
                            match collector.lock() {
                                Ok(mut guard) => guard.push_str(&chunk),
                                Err(poisoned) => poisoned.into_inner().push_str(&chunk),
                            }
                        }
                    })
                    .await
                    .map_err(|e| AgentInvocationError::Substrate(e.to_string()))?;

                Ok(())
            }
        })?;

        let text = match collected.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };

        if text.trim().is_empty() {
            return Err(AgentInvocationError::Substrate(
                "Anthropic returned no text".into(),
            ));
        }

        let (output, confidence_report, citations, telemetry) = parse_agent_response(&text, decl);

        confidence_report
            .validate()
            .map_err(AgentInvocationError::ContractViolation)?;

        Ok(AgentResult {
            output,
            confidence_report,
            citations,
            telemetry,
        })
    }
}

fn default_confidence_report(decl: &AgentDeclaration) -> ConfidenceReport {
    ConfidenceReport {
        overall: 1.0,
        method: match decl.agent_type {
            AgentType::Deterministic => ConfidenceMethod::Declared,
            AgentType::Statistical => ConfidenceMethod::Heuristic,
            AgentType::Generative => ConfidenceMethod::Declared,
        },
        explanation: Some(format!(
            "Anthropic adapter execution for model {}@{}",
            decl.model_identifier, decl.model_version
        )),
        field_level: None,
        metadata: Default::default(),
    }
}

fn parse_agent_response(
    text: &str,
    decl: &AgentDeclaration,
) -> (
    serde_json::Value,
    ConfidenceReport,
    Vec<serde_json::Value>,
    Option<serde_json::Value>,
) {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
        if let serde_json::Value::Object(map) = parsed {
            let output = map
                .get("output")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(map.clone()));
            let confidence_report = map
                .get("confidenceReport")
                .and_then(|v| serde_json::from_value::<ConfidenceReport>(v.clone()).ok())
                .unwrap_or_else(|| default_confidence_report(decl));
            let citations = map
                .get("citations")
                .and_then(|v| serde_json::from_value::<Vec<serde_json::Value>>(v.clone()).ok())
                .unwrap_or_default();
            let telemetry = map.get("telemetry").cloned();
            return (output, confidence_report, citations, telemetry);
        }
        return (parsed, default_confidence_report(decl), Vec::new(), None);
    }
    (
        serde_json::json!({ "text": text }),
        default_confidence_report(decl),
        Vec::new(),
        None,
    )
}

fn run_async<F: std::future::Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for anthropic invocation");
        rt.block_on(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::model::ai::{AgentDeclaration, AgentType};

    fn agent(invoker: Option<InvokerSpec>, agent_type: AgentType) -> AgentDeclaration {
        let mut source = serde_json::json!({
            "id": "extractor",
            "agentType": agent_type,
            "modelIdentifier": "claude-opus-4-7",
            "modelVersion": "1.0.0"
        });
        if let Some(invoker) = invoker {
            source["invoker"] = serde_json::to_value(invoker).unwrap();
        }
        serde_json::from_value(source).unwrap()
    }

    fn anthropic_spec() -> InvokerSpec {
        InvokerSpec::Anthropic {
            config_ref: None,
            metadata: Default::default(),
        }
    }

    fn task() -> AgentTask {
        AgentTask {
            capability_id: "extract".into(),
            prompt: Some("Extract document entities".into()),
            inputs: Default::default(),
            correlation_key: None,
        }
    }

    fn context() -> serde_json::Value {
        serde_json::json!({ "caseNumber": "CASE-12345" })
    }

    #[test]
    fn refuses_non_anthropic_spec_with_invoker_mismatch() {
        let inv = AnthropicInvoker::new();
        let decl = agent(
            Some(InvokerSpec::Stub {
                responses: vec![],
                metadata: Default::default(),
            }),
            AgentType::Generative,
        );
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
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
        let decl = agent(None, AgentType::Generative);
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
            invocation_index: 0,
            case_state: &case,
        };
        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must refuse missing invoker");
        assert!(matches!(err, AgentInvocationError::InvokerMismatch(_)));
    }

    #[test]
    fn streaming_collection_assembles_chunks_correctly() {
        let hook: StreamCollectorHook = Arc::new(|_decl, _task, _prompt| {
            Ok(vec![
                "{\"output\": {\"summary\": ".to_string(),
                "\"Valid verification\"}, ".to_string(),
                "\"confidenceReport\": {\"overall\": 0.96, \"method\": \"calibrated\"}}".to_string(),
            ])
        });

        let inv = AnthropicInvoker::new().with_stream_hook(hook);
        let decl = agent(Some(anthropic_spec()), AgentType::Generative);
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
            invocation_index: 0,
            case_state: &case,
        };

        let result = inv
            .invoke(&decl, &task(), &ctx)
            .expect("streaming chunks must assemble cleanly");

        assert_eq!(result.output["summary"], "Valid verification");
        assert!((result.confidence_report.overall - 0.96).abs() < f64::EPSILON);
        assert_eq!(
            result.confidence_report.method,
            ConfidenceMethod::Calibrated
        );
    }

    #[test]
    fn propagates_confidence_report_across_agent_types() {
        let types = [
            (AgentType::Deterministic, ConfidenceMethod::Declared),
            (AgentType::Statistical, ConfidenceMethod::Heuristic),
            (AgentType::Generative, ConfidenceMethod::Declared),
        ];

        for (agent_type, expected_method) in types {
            let hook: StreamCollectorHook = Arc::new(|_decl, _task, _prompt| {
                Ok(vec!["{\"verified\": true}".to_string()])
            });

            let inv = AnthropicInvoker::new().with_stream_hook(hook);
            let decl = agent(Some(anthropic_spec()), agent_type);
            let case = context();
            let ctx = AgentContext {
                process_id: "proc-1",
                invocation_index: 0,
                case_state: &case,
            };

            let result = inv.invoke(&decl, &task(), &ctx).expect("must succeed");
            assert_eq!(result.output["verified"], true);
            assert_eq!(result.confidence_report.method, expected_method);
            assert!((result.confidence_report.overall - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn propagates_citations_and_telemetry() {
        let hook: StreamCollectorHook = Arc::new(|_decl, _task, _prompt| {
            Ok(vec![serde_json::to_string(&serde_json::json!({
                "output": { "score": 42 },
                "citations": [{ "source": "https://example.com/statute", "title": "Section 104" }],
                "telemetry": { "tokens": 128 }
            }))
            .unwrap()])
        });

        let inv = AnthropicInvoker::new().with_stream_hook(hook);
        let decl = agent(Some(anthropic_spec()), AgentType::Generative);
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
            invocation_index: 0,
            case_state: &case,
        };

        let result = inv.invoke(&decl, &task(), &ctx).expect("must succeed");
        assert_eq!(result.output["score"], 42);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0]["source"], "https://example.com/statute");
        assert_eq!(result.telemetry.as_ref().unwrap()["tokens"], 128);
    }

    #[test]
    fn fails_with_substrate_when_no_api_key_configured() {
        let inv = AnthropicInvoker {
            api_key: None,
            stream_hook: None,
        };
        let decl = agent(Some(anthropic_spec()), AgentType::Generative);
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
            invocation_index: 0,
            case_state: &case,
        };

        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must fail without api key");
        assert!(matches!(err, AgentInvocationError::Substrate(_)));
        assert!(err.to_string().contains("API key is not configured"));
    }

    #[test]
    fn fails_with_substrate_when_empty_stream() {
        let hook: StreamCollectorHook = Arc::new(|_decl, _task, _prompt| Ok(vec!["   ".to_string()]));
        let inv = AnthropicInvoker::new().with_stream_hook(hook);
        let decl = agent(Some(anthropic_spec()), AgentType::Generative);
        let case = context();
        let ctx = AgentContext {
            process_id: "proc-1",
            invocation_index: 0,
            case_state: &case,
        };

        let err = inv
            .invoke(&decl, &task(), &ctx)
            .expect_err("must fail on empty stream");
        assert!(matches!(err, AgentInvocationError::Substrate(_)));
        assert!(err.to_string().contains("returned no text"));
    }

    #[test]
    fn genuine_anthropic_sdk_client_builds_request() {
        // Confirms anthropic-sdk Client::new().build() succeeds and configures Request properly.
        let client = anthropic_sdk::Client::new()
            .auth("test-dummy-key")
            .model("claude-opus-4-7")
            .max_tokens(1024)
            .temperature(0.0)
            .system("System prompt")
            .messages(&serde_json::json!([
                { "role": "user", "content": "Hello Claude" }
            ]))
            .stream(true);

        let req = client.build();
        assert!(req.is_ok(), "Client::build() must succeed with valid inputs");
    }

    #[test]
    fn validates_and_invokes_conformance_fixtures_per_agent_type() {
        let fixtures = [
            ("deterministic-agent.json", AgentType::Deterministic, ConfidenceMethod::Declared),
            ("statistical-agent.json", AgentType::Statistical, ConfidenceMethod::Heuristic),
            ("generative-agent.json", AgentType::Generative, ConfidenceMethod::Declared),
        ];

        let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/conformance/anthropic");

        for (file_name, expected_type, expected_method) in fixtures {
            let path = base_dir.join(file_name);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read fixture at {:?}: {}", path, e));
            let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

            let policy = wos_core::load_native_ai_policy(&doc)
                .expect("fixture must be valid native AI policy")
                .expect("policy must be present");

            let agents = policy.agents();
            assert_eq!(agents.len(), 1);
            let decl = &agents[0];
            assert_eq!(decl.agent_type, expected_type);

            let hook: StreamCollectorHook = Arc::new(|decl, task, _prompt| {
                Ok(vec![serde_json::to_string(&serde_json::json!({
                    "output": { "executedCapability": task.capability_id, "agent": decl.id }
                }))
                .unwrap()])
            });

            let inv = AnthropicInvoker::new().with_stream_hook(hook);
            let case = serde_json::json!({ "status": "active" });
            let ctx = AgentContext {
                process_id: "proc-conformance",
                invocation_index: 0,
                case_state: &case,
            };
            let task = AgentTask {
                capability_id: decl.capabilities[0].id.clone(),
                prompt: None,
                inputs: Default::default(),
                correlation_key: None,
            };

            let result = inv.invoke(decl, &task, &ctx).expect("invocation must succeed");
            assert_eq!(result.confidence_report.method, expected_method);
            assert_eq!(result.output["executedCapability"], decl.capabilities[0].id);
            assert_eq!(result.output["agent"], decl.id);
        }
    }
}

