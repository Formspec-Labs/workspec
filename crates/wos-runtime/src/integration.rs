// Rust guideline compliant 2026-02-21

//! Typed Integration Profile model consumed by the runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// WOS Integration Profile Document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationProfileDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosIntegrationProfile")]
    pub wos_integration_profile: String,

    /// Optional JSON Schema URI for editor validation.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Workflow targeted by this profile.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetWorkflow {
    /// Canonical target workflow URL.
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
    pub kind: String,

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

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Contract reference for integration request or response validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationContractRef {
    /// Referenced Formspec Definition URI.
    pub definition_ref: String,
}
