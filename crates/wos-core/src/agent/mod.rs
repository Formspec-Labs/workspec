// Rust guideline compliant 2026-02-21

//! Agent invocation port (ADR 0064).
//!
//! WOS governs agents as first-class actors (`ActorKind::Agent`); per-agent
//! runtime declarations live in the workflow's `agents[]` embedded block. This
//! module defines the substrate-neutral port through which the runtime invokes
//! agents, mirroring the established port/adapter posture used by
//! `DurableRuntime` and `EventStore`.
//!
//! # Architecture
//!
//! ```text
//!   wos-core::agent (this module — port + types)
//!      ↑               ↑                ↑
//!   wos-agent-stub  wos-agent-anthropic  wos-agent-mcp  …  (adapters)
//! ```
//!
//! Each [`InvokerSpec`] discriminator branch corresponds to one adapter crate.
//! Deployment binds the discriminator to a concrete [`AgentInvoker`]
//! implementation; multi-substrate deployments dispatch per agent declaration
//! via a discriminator-keyed map. Multi-agent orchestrators (A2A, AutoGen,
//! CrewAI) are *one kind* of [`AgentInvoker`], not a special spec concept.
//!
//! # Sync, not async
//!
//! [`AgentInvoker::invoke`] is intentionally synchronous, matching the existing
//! `IntegrationBindingHandler` pattern in `wos-runtime`. Production adapters
//! that wrap network-bound substrates (Anthropic SDK, MCP servers, HTTP
//! services) are responsible for managing their own async runtime
//! (`tokio::task::block_in_place` or running the WOS runtime on a multi-thread
//! executor). Keeping the spec port sync makes the conformance stub trivial
//! and lets each adapter own its concurrency story.
//!
//! # Determinism
//!
//! Adapters MUST be deterministic when configured deterministically. The
//! conformance stub ([`StubInvoker`] in `wos-agent-stub`) is the canonical
//! deterministic implementation: it returns canned responses keyed by
//! `(agent_id, capability_id, invocation_index)` so trace-parity tests
//! reproduce byte-for-byte.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::ai::AgentDeclaration;

/// The substrate-neutral port through which the runtime invokes agents.
///
/// Implementations live in adapter crates (`wos-agent-stub`,
/// `wos-agent-anthropic`, `wos-agent-mcp`, …). The spec is portable across
/// substrates because the runtime never names a concrete adapter; deployment
/// binds the [`InvokerSpec`] discriminator to an implementation.
pub trait AgentInvoker: Send + Sync {
    /// Invoke `decl` with `task` under `ctx`. Returns the agent's output plus
    /// confidence, citations, and telemetry on success; or an
    /// [`AgentInvocationError`] on a substrate failure.
    ///
    /// The runtime is responsible for evaluating deontic constraints,
    /// confidence floors, and autonomy caps against the result; the invoker
    /// only produces the raw evidence.
    fn invoke(
        &self,
        decl: &AgentDeclaration,
        task: &AgentTask,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentResult, AgentInvocationError>;
}

/// Discriminator selecting which substrate adapter handles an agent
/// invocation. Carried on [`AgentDeclaration::invoker`] (added in this PR).
///
/// Each variant maps to one adapter crate. Adding a substrate is a non-
/// breaking change: declare a new variant, ship a new adapter crate,
/// deployments opt in by binding the variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InvokerSpec {
    /// Direct Anthropic SDK call. Adapter: `wos-agent-anthropic`.
    Anthropic {
        /// Anthropic model identifier (e.g., `claude-opus-4-7`).
        model: String,
        /// Optional opaque adapter-specific configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config_ref: Option<String>,
    },

    /// Claude Agent SDK invocation. Adapter: `wos-agent-sdk-claude`.
    ClaudeAgentSdk {
        /// Reference to a Claude Agent SDK config document.
        config_ref: String,
    },

    /// MCP server tool invocation. Adapter: `wos-agent-mcp`.
    Mcp {
        /// MCP server identifier (resolved at deploy time).
        server: String,
        /// Tool name on the MCP server.
        tool: String,
    },

    /// A2A multi-agent orchestrator invocation. Adapter: `wos-agent-a2a`.
    A2A {
        /// Orchestrator identifier (resolved at deploy time).
        orchestrator: String,
    },

    /// Generic HTTP/OpenAPI service invocation. Adapter: `wos-agent-http`.
    Http {
        /// Endpoint URL.
        endpoint: String,
        /// Optional URI to an OpenAPI / JSON Schema describing the contract.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schema_ref: Option<String>,
    },

    /// Deterministic in-memory stub. Adapter: `wos-agent-stub`.
    /// Used by conformance fixtures so trace-parity tests reproduce
    /// byte-for-byte.
    Stub {
        /// Canned responses keyed by `(capability_id, invocation_index)`.
        /// Indexed in declaration order; the invoker increments the index per
        /// `(agent_id, capability_id)` pair.
        #[serde(default)]
        responses: Vec<StubResponse>,
    },
}

/// One canned response in [`InvokerSpec::Stub`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StubResponse {
    /// Capability id this response satisfies. When omitted, matches any
    /// capability — useful for single-capability fixtures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,

    /// Output payload returned by the invoker.
    pub output: serde_json::Value,

    /// Reported confidence. When omitted, defaults to `1.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,

    /// Optional citation array passed through unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<serde_json::Value>,
}

/// One unit of work handed to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    /// Capability the runtime expects this invocation to exercise. MUST match
    /// one of `AgentDeclaration::capabilities[].id`. The invoker MAY use it to
    /// dispatch to a sub-tool or template; the runtime uses it to look up the
    /// declared input/output contract refs and apply governance.
    pub capability_id: String,

    /// Free-form prompt or directive. Invokers SHOULD treat this as the
    /// authoritative human-language request; structured inputs go in
    /// `inputs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Structured inputs validated against the capability's
    /// `inputContractRef`. Invokers SHOULD pass these through unchanged to
    /// the substrate and surface them in the agent's tool-call surface where
    /// applicable.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Optional opaque correlation key used by the runtime to link this
    /// invocation to a parent transition / case event for provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_key: Option<String>,
}

/// Context the runtime hands to the invoker for this call. Carries read-only
/// references; invokers MUST NOT mutate case state or provenance directly —
/// they return [`AgentResult`] and the runtime applies governance + emits
/// records.
#[derive(Debug)]
pub struct AgentContext<'a> {
    /// Workflow instance id (case TypeID).
    pub instance_id: &'a str,

    /// Per-invocation index supplied by the runtime (often 0 until the runtime
    /// threads a counter). The canonical stub adapter (`wos-agent-stub`) keeps
    /// its own per-`(agent_id, capability_id)` counter via `next_index()` and
    /// does not read this field; other adapters MAY use it for canned routing,
    /// retry-aware logging, or telemetry.
    pub invocation_index: u32,

    /// Snapshot of the case state at invocation time, in canonical JSON form.
    /// Invokers SHOULD treat this as advisory context and reference fields
    /// only via the capability's declared `inputContractRef` to stay
    /// substrate-portable.
    pub case_state: &'a serde_json::Value,
}

/// What the invoker returns on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    /// Output payload. Validated against the capability's `outputContractRef`
    /// by the runtime before being projected into case state.
    pub output: serde_json::Value,

    /// Reported confidence in `[0, 1]`. The runtime compares this against the
    /// agent's `confidence_decay`-adjusted floor (see `wos_core::confidence`)
    /// to decide whether to commit or fall through to the fallback chain.
    pub confidence: f64,

    /// Optional citation array. Pass-through; the runtime emits them as
    /// provenance evidence on the `CapabilityInvocationRecord`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<serde_json::Value>,

    /// Optional telemetry blob. Emitted alongside the
    /// `CapabilityInvocationRecord`; opaque to the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<serde_json::Value>,
}

/// Failure modes returned by an [`AgentInvoker`].
#[derive(Debug, Error)]
pub enum AgentInvocationError {
    /// The substrate refused the request (network, auth, quota, etc.).
    #[error("substrate failure: {0}")]
    Substrate(String),

    /// The substrate returned a payload that does not match the agent's
    /// output contract. The runtime falls through to the fallback chain.
    #[error("contract violation: {0}")]
    ContractViolation(String),

    /// The agent's declared invoker spec does not match this adapter. The
    /// runtime SHOULD route to the correct adapter or fail fast.
    #[error("invoker mismatch: {0}")]
    InvokerMismatch(String),

    /// The stub adapter has no canned response for the
    /// `(capability_id, invocation_index)` pair the runtime requested.
    #[error(
        "stub exhausted: no canned response for capability='{capability_id}' index={invocation_index}"
    )]
    StubExhausted {
        /// Capability that was requested.
        capability_id: String,
        /// Invocation index the stub looked for.
        invocation_index: u32,
    },
}

/// Substrate discriminator independent of the [`InvokerSpec`] payload.
///
/// Used as the lookup key in [`AgentInvokerRegistry`]: the runtime takes the
/// `InvokerSpec` from an agent declaration, projects to the `InvokerKind`,
/// and asks the registry for the bound adapter. Each variant maps 1:1 to one
/// `InvokerSpec` variant and to one adapter crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvokerKind {
    /// `wos-agent-anthropic`.
    Anthropic,
    /// `wos-agent-sdk-claude`.
    ClaudeAgentSdk,
    /// `wos-agent-mcp`.
    Mcp,
    /// `wos-agent-a2a`.
    A2A,
    /// `wos-agent-http`.
    Http,
    /// `wos-agent-stub`.
    Stub,
}

impl InvokerKind {
    /// Stable string form used for diagnostics, error messages, and the
    /// schema's `agents[].invoker.kind` discriminator.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::ClaudeAgentSdk => "claudeAgentSdk",
            Self::Mcp => "mcp",
            Self::A2A => "a2a",
            Self::Http => "http",
            Self::Stub => "stub",
        }
    }
}

impl From<&InvokerSpec> for InvokerKind {
    fn from(spec: &InvokerSpec) -> Self {
        match spec {
            InvokerSpec::Anthropic { .. } => Self::Anthropic,
            InvokerSpec::ClaudeAgentSdk { .. } => Self::ClaudeAgentSdk,
            InvokerSpec::Mcp { .. } => Self::Mcp,
            InvokerSpec::A2A { .. } => Self::A2A,
            InvokerSpec::Http { .. } => Self::Http,
            InvokerSpec::Stub { .. } => Self::Stub,
        }
    }
}

/// Deployment-time registry of [`AgentInvoker`] adapters keyed by
/// [`InvokerKind`].
///
/// The runtime owns one registry per `WosRuntime` instance. At agent
/// invocation time the runtime takes the agent's declared `InvokerSpec`,
/// projects to an `InvokerKind`, and looks up the bound adapter. If no
/// adapter is registered for the spec the runtime fails fast with
/// [`AgentInvocationError::InvokerMismatch`] — this surfaces deployment
/// configuration drift loudly rather than silently falling through to a
/// fallback chain.
///
/// The registry is `Send + Sync` so a single instance can be shared across
/// runtime threads. Each contained `AgentInvoker` is also `Send + Sync` per
/// the trait bound.
#[derive(Default)]
pub struct AgentInvokerRegistry {
    invokers: std::collections::HashMap<InvokerKind, Box<dyn AgentInvoker + Send + Sync>>,
}

impl AgentInvokerRegistry {
    /// Empty registry. The runtime fails any agent invocation against this
    /// until an adapter is registered.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `invoker` as the handler for `kind`. Replaces any prior
    /// registration for the same kind (deployment authors typically wire each
    /// kind exactly once at startup; replacement is allowed for test
    /// scenarios).
    pub fn register(&mut self, kind: InvokerKind, invoker: Box<dyn AgentInvoker + Send + Sync>) {
        self.invokers.insert(kind, invoker);
    }

    /// Builder-form registration. Useful when constructing a registry
    /// inline for runtime DI (`AgentInvokerRegistry::new().with(...)`).
    #[must_use]
    pub fn with(mut self, kind: InvokerKind, invoker: Box<dyn AgentInvoker + Send + Sync>) -> Self {
        self.register(kind, invoker);
        self
    }

    /// Lookup the adapter bound to `spec`'s discriminator. Returns `None` if
    /// no adapter is registered for this kind.
    #[must_use]
    pub fn lookup(&self, spec: &InvokerSpec) -> Option<&(dyn AgentInvoker + Send + Sync)> {
        self.invokers
            .get(&InvokerKind::from(spec))
            .map(|b| b.as_ref())
    }

    /// Lookup by `InvokerKind` directly, when the caller already has the
    /// discriminator (e.g., from a serde-deserialized config).
    #[must_use]
    pub fn lookup_kind(&self, kind: InvokerKind) -> Option<&(dyn AgentInvoker + Send + Sync)> {
        self.invokers.get(&kind).map(|b| b.as_ref())
    }

    /// True when no adapters are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.invokers.is_empty()
    }

    /// Iterate over registered kinds. Order is unspecified (HashMap).
    pub fn registered_kinds(&self) -> impl Iterator<Item = InvokerKind> + '_ {
        self.invokers.keys().copied()
    }
}

impl std::fmt::Debug for AgentInvokerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kinds: Vec<&'static str> = self.invokers.keys().map(|k| k.as_str()).collect();
        f.debug_struct("AgentInvokerRegistry")
            .field("registered", &kinds)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoker_spec_anthropic_round_trips() {
        let spec = InvokerSpec::Anthropic {
            model: "claude-opus-4-7".into(),
            config_ref: None,
        };
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "anthropic",
                "model": "claude-opus-4-7"
            }),
            "InvokerSpec::Anthropic MUST tag-serialize as kind='anthropic' \
             with model passed through"
        );
        let back: InvokerSpec = serde_json::from_value(json).unwrap();
        assert!(
            matches!(back, InvokerSpec::Anthropic { ref model, .. } if model == "claude-opus-4-7")
        );
    }

    #[test]
    fn invoker_spec_stub_round_trips() {
        let spec = InvokerSpec::Stub {
            responses: vec![StubResponse {
                capability_id: Some("triage".into()),
                output: serde_json::json!({"category": "benefits"}),
                confidence: Some(0.92),
                citations: vec![],
            }],
        };
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["kind"], "stub");
        assert_eq!(json["responses"][0]["capabilityId"], "triage");
        let back: InvokerSpec = serde_json::from_value(json).unwrap();
        match back {
            InvokerSpec::Stub { responses } => {
                assert_eq!(responses.len(), 1);
                assert_eq!(responses[0].capability_id.as_deref(), Some("triage"));
                assert_eq!(responses[0].confidence, Some(0.92));
            }
            other => panic!("expected Stub, got {other:?}"),
        }
    }

    #[test]
    fn invoker_spec_mcp_round_trips() {
        let json = serde_json::json!({
            "kind": "mcp",
            "server": "agent-router",
            "tool": "triage"
        });
        let spec: InvokerSpec = serde_json::from_value(json).unwrap();
        assert!(matches!(
            spec,
            InvokerSpec::Mcp { ref server, ref tool }
                if server == "agent-router" && tool == "triage"
        ));
    }

    // ── InvokerKind / AgentInvokerRegistry ──────────────────────────────────

    #[test]
    fn invoker_kind_projects_from_every_spec_variant() {
        // Each InvokerSpec variant MUST project to a distinct InvokerKind so
        // the registry can route by kind without ambiguity.
        assert_eq!(
            InvokerKind::from(&InvokerSpec::Anthropic {
                model: "claude-opus-4-7".into(),
                config_ref: None,
            }),
            InvokerKind::Anthropic
        );
        assert_eq!(
            InvokerKind::from(&InvokerSpec::ClaudeAgentSdk {
                config_ref: "x".into(),
            }),
            InvokerKind::ClaudeAgentSdk
        );
        assert_eq!(
            InvokerKind::from(&InvokerSpec::Mcp {
                server: "s".into(),
                tool: "t".into(),
            }),
            InvokerKind::Mcp
        );
        assert_eq!(
            InvokerKind::from(&InvokerSpec::A2A {
                orchestrator: "o".into(),
            }),
            InvokerKind::A2A
        );
        assert_eq!(
            InvokerKind::from(&InvokerSpec::Http {
                endpoint: "https://example.test".into(),
                schema_ref: None,
            }),
            InvokerKind::Http
        );
        assert_eq!(
            InvokerKind::from(&InvokerSpec::Stub { responses: vec![] }),
            InvokerKind::Stub
        );
    }

    #[test]
    fn invoker_kind_str_matches_schema_discriminator_values() {
        // The string form MUST match the `kind` const values declared in the
        // schema's `InvokerSpec` $def. If this drifts, the schema and the
        // runtime registry no longer agree on which adapter handles which
        // declaration.
        assert_eq!(InvokerKind::Anthropic.as_str(), "anthropic");
        assert_eq!(InvokerKind::ClaudeAgentSdk.as_str(), "claudeAgentSdk");
        assert_eq!(InvokerKind::Mcp.as_str(), "mcp");
        assert_eq!(InvokerKind::A2A.as_str(), "a2a");
        assert_eq!(InvokerKind::Http.as_str(), "http");
        assert_eq!(InvokerKind::Stub.as_str(), "stub");
    }

    /// Test-only invoker that records which agent + capability it received
    /// and returns a canned output. Used to exercise registry lookup
    /// independently of any production adapter.
    struct RecordingInvoker {
        label: &'static str,
    }

    impl AgentInvoker for RecordingInvoker {
        fn invoke(
            &self,
            decl: &crate::model::ai::AgentDeclaration,
            task: &AgentTask,
            _ctx: &AgentContext<'_>,
        ) -> Result<AgentResult, AgentInvocationError> {
            Ok(AgentResult {
                output: serde_json::json!({
                    "handledBy": self.label,
                    "agentId": decl.id,
                    "capability": task.capability_id,
                }),
                confidence: 1.0,
                citations: vec![],
                telemetry: None,
            })
        }
    }

    #[test]
    fn registry_lookup_routes_by_invoker_spec_discriminator() {
        let mut registry = AgentInvokerRegistry::new();
        registry.register(
            InvokerKind::Stub,
            Box::new(RecordingInvoker { label: "stub" }),
        );
        registry.register(
            InvokerKind::Anthropic,
            Box::new(RecordingInvoker { label: "anthropic" }),
        );

        let stub_spec = InvokerSpec::Stub { responses: vec![] };
        let stub_invoker = registry.lookup(&stub_spec).expect("stub bound");

        let anthropic_spec = InvokerSpec::Anthropic {
            model: "claude-opus-4-7".into(),
            config_ref: None,
        };
        let anth_invoker = registry.lookup(&anthropic_spec).expect("anthropic bound");

        // Confirm they are different adapters by inspecting the recording
        // invoker's label through a fake invocation.
        use crate::model::ai::{AgentDeclaration, AgentType};
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
            invoker: None,
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
        let stub_result = stub_invoker.invoke(&decl, &task, &ctx).unwrap();
        let anth_result = anth_invoker.invoke(&decl, &task, &ctx).unwrap();
        assert_eq!(stub_result.output["handledBy"], "stub");
        assert_eq!(anth_result.output["handledBy"], "anthropic");
    }

    #[test]
    fn registry_lookup_returns_none_for_unregistered_kind() {
        let registry = AgentInvokerRegistry::new();
        let spec = InvokerSpec::Stub { responses: vec![] };
        assert!(registry.lookup(&spec).is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_with_chains_registrations() {
        let registry = AgentInvokerRegistry::new()
            .with(
                InvokerKind::Stub,
                Box::new(RecordingInvoker { label: "stub" }),
            )
            .with(
                InvokerKind::Mcp,
                Box::new(RecordingInvoker { label: "mcp" }),
            );
        assert!(!registry.is_empty());
        let mut kinds: Vec<&'static str> = registry
            .registered_kinds()
            .map(InvokerKind::as_str)
            .collect();
        kinds.sort();
        assert_eq!(kinds, vec!["mcp", "stub"]);
    }

    #[test]
    fn registry_register_replaces_prior_binding_for_same_kind() {
        let mut registry = AgentInvokerRegistry::new();
        registry.register(
            InvokerKind::Stub,
            Box::new(RecordingInvoker { label: "first" }),
        );
        registry.register(
            InvokerKind::Stub,
            Box::new(RecordingInvoker { label: "second" }),
        );
        let spec = InvokerSpec::Stub { responses: vec![] };
        let invoker = registry.lookup(&spec).unwrap();
        // Smoke-test the second binding is the one that survived.
        use crate::model::ai::{AgentDeclaration, AgentType};
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
            invoker: None,
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
        let result = invoker.invoke(&decl, &task, &ctx).unwrap();
        assert_eq!(result.output["handledBy"], "second");
    }
}
