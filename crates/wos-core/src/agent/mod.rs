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

    /// Per-`(agent_id, capability_id)` invocation index, starting at 0. Used
    /// by the stub adapter to look up canned responses; production adapters
    /// MAY use it for retry-aware logging or telemetry.
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
    #[error("stub exhausted: no canned response for capability='{capability_id}' index={invocation_index}")]
    StubExhausted {
        /// Capability that was requested.
        capability_id: String,
        /// Invocation index the stub looked for.
        invocation_index: u32,
    },
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
        assert!(matches!(back, InvokerSpec::Anthropic { ref model, .. } if model == "claude-opus-4-7"));
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
}
