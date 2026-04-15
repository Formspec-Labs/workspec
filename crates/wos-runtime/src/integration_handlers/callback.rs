// Rust guideline compliant 2026-04-14

//! Handler for `callback` integration bindings.
//!
//! Implements a two-phase CloudEvents callback pattern:
//!
//! **Outbound phase** (binding fires during a transition action):
//! The handler emits a CloudEvent outbound request and registers a
//! `PendingCallback` in `instance.pending_callbacks`. The correlation key
//! (CloudEvents `subject`) is `{instanceId}:{bindingId}:{invocationId}`.
//! A `CallbackPending` provenance record captures the subject and optional
//! deadline.
//!
//! **Inbound phase** (a response event arrives carrying a matching subject):
//! When the action data contains a CloudEvent whose `subject` matches a key in
//! `instance.pending_callbacks`, the pending entry is removed, the output
//! binding is applied, and `CallbackReceived` provenance is emitted. Inbound
//! events whose subject does not match any pending entry are silently dropped
//! (no case-state change, no provenance record beyond an optional debug log).
//!
//! # Subject routing in fixtures
//!
//! Fixture event-sequence entries that represent inbound callback responses
//! place the full CloudEvent JSON in the `data` field of the event entry,
//! exactly as event-consume does. The handler uses a three-way classification:
//!
//! - **Outbound** — `observed.event_data` contains no CloudEvent envelope
//!   (i.e., the `id` field is absent or not a CloudEvent). The binding fires
//!   outbound and registers a `PendingCallback`.
//! - **InboundCorrelated** — `observed.event_data` contains a CloudEvent
//!   whose `subject` matches a key in `pending_callbacks`. The pending entry
//!   is resolved and `CallbackReceived` is emitted.
//! - **InboundUncorrelated** — `observed.event_data` contains a CloudEvent
//!   whose `subject` does NOT match any pending entry. The event is silently
//!   dropped: no service invocation, no new `PendingCallback`, no provenance.

use wos_core::eval::ObservedAction;
use wos_core::instance::PendingCallback;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::cloudevents::CloudEvent;
use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::{IntegrationBindingHandler, next_outbound_event_id};
use super::request_response::{
    InvocationContext, apply_output_binding, build_event_data_from_binding,
};

/// Three-way classification of a callback binding invocation.
enum CallbackInvocationKind {
    /// No CloudEvent in the action data — fire an outbound request.
    Outbound,
    /// CloudEvent whose subject matches a pending-callback entry — resolve it.
    InboundCorrelated,
    /// CloudEvent whose subject does NOT match any pending entry — silent drop.
    InboundUncorrelated,
}

/// Handler for bidirectional CloudEvents callback bindings.
pub(crate) struct CallbackHandler;

impl IntegrationBindingHandler for CallbackHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::Callback
    }

    fn execute(
        &self,
        ctx: &InvocationContext<'_>,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        match classify_invocation(observed, &record.instance.pending_callbacks) {
            CallbackInvocationKind::Outbound => {
                handle_outbound(ctx, record, kernel, observed, service_ref, binding, now_iso)
            }
            CallbackInvocationKind::InboundCorrelated => {
                handle_inbound(record, observed, service_ref, binding)
            }
            CallbackInvocationKind::InboundUncorrelated => {
                // Silently drop — no service invocation, no pending registration, no provenance.
                Ok(Vec::new())
            }
        }
    }
}

/// Classify a callback invocation into one of three kinds.
///
/// An invocation is *inbound* if `observed.event_data` contains a JSON object
/// with an `id` field (the minimal indicator of a CloudEvents envelope). Inbound
/// is *correlated* if the envelope's `subject` matches a key in `pending_callbacks`.
fn classify_invocation(
    observed: &ObservedAction,
    pending: &std::collections::HashMap<String, PendingCallback>,
) -> CallbackInvocationKind {
    let Some(data) = &observed.event_data else {
        return CallbackInvocationKind::Outbound;
    };
    // A CloudEvent envelope must have `id`, `source`, and `specversion`.
    // Presence of `id` as a string is the minimal guard against non-event payloads.
    if data.get("id").and_then(|v| v.as_str()).is_none() {
        return CallbackInvocationKind::Outbound;
    }
    let subject = data
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if pending.contains_key(subject) {
        CallbackInvocationKind::InboundCorrelated
    } else {
        CallbackInvocationKind::InboundUncorrelated
    }
}

/// Outbound phase: emit a CloudEvent, register `PendingCallback`, emit `CallbackPending`.
fn handle_outbound(
    ctx: &InvocationContext<'_>,
    record: &mut RuntimeRecord,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    service_ref: &str,
    binding: &IntegrationBinding,
    now_iso: &str,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    // `outbound_event_id` is the CloudEvent `id` for this emission — a unique
    // event identifier, not an idempotency key. Each callback fire is a new event.
    let outbound_event_id = next_outbound_event_id(record, service_ref, "cb");
    let subject = compute_subject(
        binding,
        &record.instance.instance_id,
        service_ref,
        &outbound_event_id,
    );

    let event_data =
        build_event_data_from_binding(binding, kernel, observed, &record.instance)?;

    let envelope = CloudEvent {
        id: outbound_event_id.clone(),
        source: binding
            .extensions
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("urn:wos:runtime")
            .to_string(),
        spec_version: "1.0".to_string(),
        event_type: binding
            .extensions
            .get("eventType")
            .and_then(|v| v.as_str())
            .unwrap_or("com.wos.integration.callback.request")
            .to_string(),
        subject: Some(subject.clone()),
        time: Some(
            now_iso
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| {
                    RuntimeError::Integration(format!(
                        "invalid ISO timestamp in event context: {e}"
                    ))
                })?,
        ),
        data_content_type: Some("application/json".to_string()),
        data: Some(event_data),
    };

    // Dispatch the outbound envelope.
    // The event ID is a CloudEvent `id`, not an idempotency key — each callback
    // fire is a distinct event. Pass None for idempotency.
    let envelope_json = envelope.to_provenance_data();
    ctx.service
        .invoke(service_ref, &envelope_json, None)
        .map_err(|e| RuntimeError::Service(e.to_string()))?;

    // Read deadline from binding extensions if present.
    let expected_until = binding
        .extensions
        .get("expectedUntil")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // Register the pending callback for later resolution.
    record.instance.pending_callbacks.insert(
        subject.clone(),
        PendingCallback {
            invocation_id: outbound_event_id.clone(),
            binding_id: service_ref.to_string(),
            expected_until: expected_until.clone(),
        },
    );

    let pending_provenance = ProvenanceRecord {
        record_kind: ProvenanceKind::CallbackPending,
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(serde_json::json!({
            "subject": subject,
            "bindingId": service_ref,
            "invocationId": outbound_event_id,
            "expectedUntil": expected_until,
        })),
    };

    Ok(vec![pending_provenance])
}

/// Inbound phase: resolve a pending callback, apply output binding, emit `CallbackReceived`.
fn handle_inbound(
    record: &mut RuntimeRecord,
    observed: &ObservedAction,
    service_ref: &str,
    binding: &IntegrationBinding,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    let envelope_json = observed.event_data.clone().ok_or_else(|| {
        RuntimeError::Integration(format!(
            "callback binding '{service_ref}': no CloudEvent envelope in event data"
        ))
    })?;

    let envelope: CloudEvent = serde_json::from_value(envelope_json).map_err(|e| {
        RuntimeError::Integration(format!(
            "callback binding '{service_ref}': failed to parse inbound CloudEvent: {e}"
        ))
    })?;

    // Validate ingress — required fields must not be empty.
    envelope.validate_ingress().map_err(|e| {
        RuntimeError::Integration(format!(
            "callback binding '{service_ref}': EventIngressInvalid — {e}"
        ))
    })?;

    let subject = envelope.subject.clone().ok_or_else(|| {
        RuntimeError::Integration(format!(
            "callback binding '{service_ref}': inbound CloudEvent has no subject for correlation"
        ))
    })?;

    // Remove the pending entry.
    record.instance.pending_callbacks.remove(&subject);

    // Apply output binding using the envelope's data payload.
    let event_data = envelope.data.clone().unwrap_or_else(|| serde_json::json!({}));
    let updates = apply_output_binding(
        &mut record.instance.case_state,
        &binding.output_binding,
        &event_data,
    )?;

    let mut provenance = Vec::new();

    if !updates.is_empty() {
        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::DataMapping,
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "serviceRef": service_ref,
                "integrationType": "callback",
                "updatedPaths": updates,
            })),
        });
    }

    let mut received_data = envelope.to_provenance_data();
    // Annotate with correlation subject for easy extraction from provenance.
    if let Some(obj) = received_data.as_object_mut() {
        obj.insert("correlationSubject".to_string(), serde_json::json!(subject));
    }

    provenance.push(ProvenanceRecord {
        record_kind: ProvenanceKind::CallbackReceived,
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(received_data),
    });

    Ok(provenance)
}

/// Derive the CloudEvents `subject` for a callback outbound invocation.
///
/// Canonical format: `{instanceId}:{bindingId}:{invocationId}`.
/// The binding may override this via an `extensions.subject` field.
fn compute_subject(
    binding: &IntegrationBinding,
    instance_id: &str,
    binding_id: &str,
    outbound_event_id: &str,
) -> String {
    if let Some(template) = binding
        .extensions
        .get("subject")
        .and_then(|v| v.as_str())
    {
        return template.to_string();
    }
    format!("{instance_id}:{binding_id}:{outbound_event_id}")
}

