// Rust guideline compliant 2026-02-21

//! Typed Integration Profile model consumed by the runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Discriminator for integration binding kinds defined by the Integration Profile spec (S3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationBindingKind {
    /// Synchronous request/response HTTP-style invocation.
    RequestResponse,
    /// Emit a CloudEvent to an external broker.
    EventEmit,
    /// Consume an inbound CloudEvent from an external source.
    EventConsume,
    /// Bidirectional callback pattern (webhook / async reply).
    Callback,
    /// Multi-step API orchestration via an Arazzo sequence.
    ArazzoSequence,
    /// Non-HTTP tool invocation (CWL-informed descriptor).
    Tool,
    /// External policy engine evaluation (XACML, OPA, Cedar).
    PolicyEngine,
}

/// Integration content — the embedded `bindings` block of a `$wosWorkflow`
/// document (ADR 0076 D-1). Represents the interior shape of the `bindings`
/// block: named binding declarations and their type descriptors. Type name
/// retained for consumer compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationProfileDocument {
    /// Optional JSON Schema URI for editor validation.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Workflow targeted by this profile.
    #[serde(default)]
    pub target_workflow: TargetWorkflow,

    /// Profile document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Named bindings keyed by `invokeService.serviceRef`.
    pub bindings: HashMap<String, IntegrationBinding>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Target workflow metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetWorkflow {
    /// Canonical target workflow URL.
    #[serde(default)]
    pub url: String,

    /// Compatible kernel version range.
    #[serde(default)]
    pub compatible_versions: Option<String>,
}

/// Named integration binding declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationBinding {
    /// Binding type discriminator.
    #[serde(rename = "type")]
    pub kind: IntegrationBindingKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Request validation contract.
    #[serde(default)]
    pub request_contract: Option<IntegrationContractRef>,

    /// Response validation contract.
    #[serde(default)]
    pub response_contract: Option<IntegrationContractRef>,

    /// Case-state-to-request input mapping.
    #[serde(default)]
    pub input_mapping: HashMap<String, String>,

    /// Policy-engine context mapping.
    #[serde(default)]
    pub context_mapping: HashMap<String, String>,

    /// Event data mapping for event-emitting bindings.
    #[serde(default)]
    pub data_mapping: HashMap<String, String>,

    /// Response-to-case-state output mapping.
    #[serde(default)]
    pub output_binding: HashMap<String, String>,

    /// FEL expression for idempotency key construction.
    #[serde(default)]
    pub idempotency_key_expression: Option<String>,

    /// Extension data — collects all unrecognized JSON fields.
    ///
    /// CloudEvents binding handlers read metadata from this map:
    /// - `"source"` — CE `source` attribute for outbound events
    /// - `"eventType"` — CE `type` attribute for outbound events
    /// - `"subject"` — explicit CE `subject` override (default: `{processId}:{bindingId}:{invocationId}`)
    /// - `"expectedUntil"` — ISO 8601 deadline for callback resolution
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Contract reference for integration request or response validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationContractRef {
    /// Referenced Formspec Definition URI.
    pub definition_ref: String,
}

#[cfg(test)]
mod tests {
    use super::IntegrationBindingKind;

    /// Verify that `IntegrationBindingKind` round-trips through serde with the
    /// expected kebab-case wire format (e.g., `RequestResponse` → `"request-response"`).
    #[test]
    fn integration_binding_kind_serializes_to_kebab_case() {
        let cases = [
            (IntegrationBindingKind::RequestResponse, "request-response"),
            (IntegrationBindingKind::EventEmit, "event-emit"),
            (IntegrationBindingKind::EventConsume, "event-consume"),
            (IntegrationBindingKind::Callback, "callback"),
            (IntegrationBindingKind::ArazzoSequence, "arazzo-sequence"),
            (IntegrationBindingKind::Tool, "tool"),
            (IntegrationBindingKind::PolicyEngine, "policy-engine"),
        ];

        for (variant, expected_str) in cases {
            let serialized = serde_json::to_value(variant).expect("serialization must succeed");
            assert_eq!(
                serialized.as_str(),
                Some(expected_str),
                "{variant:?} must serialize to \"{expected_str}\""
            );
            let deserialized: IntegrationBindingKind =
                serde_json::from_value(serialized).expect("deserialization must succeed");
            assert_eq!(deserialized, variant, "round-trip must preserve variant");
        }
    }
}
