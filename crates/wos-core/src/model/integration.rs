// Rust guideline compliant 2026-02-21

//! Typed integration declarations shared by process persistence and the runtime.

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "serde_json::Value")]
pub struct IntegrationBinding {
    /// Explicit callback dispatch; absence retains immediate behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_mode: Option<IntegrationDispatchMode>,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationContractRef {
    /// Referenced Formspec Definition URI.
    pub definition_ref: String,
}

/// Callback dispatch policy; omitted fields retain immediate provider invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationDispatchMode {
    /// Invoke the current provider during action execution.
    Immediate,
    /// Persist an immutable request for owner-managed postcommit delivery.
    Deferred,
}

/// Explicit role of a callback action in its request/receipt association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallbackPhase {
    /// Register a fresh outbound request, irrespective of initiating event data.
    Outbound,
    /// Resolve a receipt against an existing outbound association.
    Inbound,
}

impl IntegrationBinding {
    /// Checks dispatch declarations independently of workflow action phase.
    ///
    /// # Errors
    /// Rejects dispatch on noncallbacks and missing/malformed deferred receipt types.
    pub fn validate_dispatch(&self) -> Result<(), String> {
        if self.dispatch_mode.is_some() && self.kind != IntegrationBindingKind::Callback {
            return Err("dispatchMode is valid only for callback bindings".into());
        }
        if self.dispatch_mode == Some(IntegrationDispatchMode::Deferred)
            && self
                .extensions
                .get("callbackEventType")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err("deferred callback requires nonblank callbackEventType".into());
        }
        Ok(())
    }
}

impl TryFrom<serde_json::Value> for IntegrationBinding {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let mut fields = value
            .as_object()
            .cloned()
            .ok_or("binding must be an object")?;
        fn field<T: serde::de::DeserializeOwned + Default>(
            fields: &mut serde_json::Map<String, serde_json::Value>,
            name: &str,
        ) -> Result<T, String> {
            fields
                .remove(name)
                .map(serde_json::from_value)
                .transpose()
                .map(|value| value.unwrap_or_default())
                .map_err(|error| error.to_string())
        }
        let kind = serde_json::from_value(fields.remove("type").ok_or("binding type is missing")?)
            .map_err(|error| error.to_string())?;
        // Presence is intentional: null is not an omitted dispatch declaration.
        let dispatch_mode = fields
            .remove("dispatchMode")
            .map(serde_json::from_value::<IntegrationDispatchMode>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let binding = Self {
            kind,
            dispatch_mode,
            description: field(&mut fields, "description")?,
            request_contract: field(&mut fields, "requestContract")?,
            response_contract: field(&mut fields, "responseContract")?,
            input_mapping: field(&mut fields, "inputMapping")?,
            context_mapping: field(&mut fields, "contextMapping")?,
            data_mapping: field(&mut fields, "dataMapping")?,
            output_binding: field(&mut fields, "outputBinding")?,
            idempotency_key_expression: field(&mut fields, "idempotencyKeyExpression")?,
            extensions: fields.into_iter().collect(),
        };
        binding.validate_dispatch()?;
        Ok(binding)
    }
}

/// Validates callback phases across all authored action paths before runtime effects.
///
/// # Errors
/// Rejects phases on unsupported actions/bindings and missing deferred callback phases.
pub fn validate_callback_actions(
    kernel: &super::kernel::KernelDocument,
    profile: Option<&IntegrationProfileDocument>,
) -> Result<(), String> {
    use super::kernel::{Action, ActionKind, State};
    fn action(value: &Action, profile: Option<&IntegrationProfileDocument>) -> Result<(), String> {
        let binding = value
            .service_ref
            .as_ref()
            .and_then(|id| profile.and_then(|p| p.bindings.get(id)));
        if value.callback_phase.is_some()
            && (value.action != ActionKind::InvokeService
                || binding.is_none_or(|b| b.kind != IntegrationBindingKind::Callback))
        {
            return Err("callbackPhase requires invokeService with callback binding".into());
        }
        if value.action == ActionKind::InvokeService {
            if let Some(binding) = binding {
                binding.validate_dispatch()?;
                if binding.dispatch_mode == Some(IntegrationDispatchMode::Deferred)
                    && value.callback_phase.is_none()
                {
                    return Err("deferred callback requires explicit callbackPhase".into());
                }
            }
        }
        if let Some(compensation) = &value.compensating_action {
            action(compensation, profile)?;
        }
        Ok(())
    }
    fn state(value: &State, profile: Option<&IntegrationProfileDocument>) -> Result<(), String> {
        for value in value.on_entry.iter().chain(value.on_exit.iter()).chain(
            value
                .transitions
                .iter()
                .flat_map(|transition| transition.actions.iter()),
        ) {
            action(value, profile)?;
        }
        for nested in value.states.values().chain(
            value
                .regions
                .values()
                .flat_map(|region| region.states.values()),
        ) {
            state(nested, profile)?;
        }
        if let Some(body) = &value.body {
            state(body, profile)?;
        }
        Ok(())
    }
    if let Some(profile) = profile {
        for binding in profile.bindings.values() {
            binding.validate_dispatch()?;
        }
    }
    for value in kernel.lifecycle.states.values() {
        state(value, profile)?;
    }
    Ok(())
}
