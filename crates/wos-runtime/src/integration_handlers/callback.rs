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
//! exactly as event-consume does. The handler distinguishes outbound vs.
//! inbound by checking whether `pending_callbacks` already contains the
//! event's subject.

use wos_core::eval::ObservedAction;
use wos_core::instance::PendingCallback;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::cloudevents::CloudEvent;
use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::IntegrationBindingHandler;
use super::request_response::{
    InvocationContext, apply_output_binding, build_event_data_from_binding,
};

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
        // Determine whether this invocation is an outbound fire or an inbound resolution.
        // If the action data looks like a CloudEvent with a subject that matches a
        // pending entry, treat it as an inbound resolution; otherwise treat as outbound.
        let is_inbound = is_inbound_resolution(observed, &record.instance.pending_callbacks);

        if is_inbound {
            handle_inbound(record, observed, service_ref, binding)
        } else {
            handle_outbound(ctx, record, kernel, observed, service_ref, binding, now_iso)
        }
    }
}

/// Returns `true` when the event data payload is a CloudEvent whose `subject`
/// matches a registered pending-callback key.
///
/// The event data (from the triggering `EventEntry.data`) is carried in
/// `observed.event_data`. The presence of a matching subject in
/// `pending_callbacks` distinguishes inbound resolutions from outbound fires.
fn is_inbound_resolution(
    observed: &ObservedAction,
    pending: &std::collections::HashMap<String, PendingCallback>,
) -> bool {
    let Some(data) = &observed.event_data else {
        return false;
    };
    let Some(subject) = data.get("subject").and_then(|v| v.as_str()) else {
        return false;
    };
    pending.contains_key(subject)
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
    let invocation_id = next_invocation_id(record, service_ref);
    let subject = compute_subject(
        binding,
        &record.instance.instance_id,
        service_ref,
        &invocation_id,
    );

    let event_data =
        build_event_data_from_binding(binding, kernel, observed, &record.instance)?;

    let envelope = CloudEvent {
        id: invocation_id.clone(),
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
                .unwrap_or_else(|_| chrono::Utc::now()),
        ),
        data_content_type: Some("application/json".to_string()),
        data: Some(event_data),
    };

    // Dispatch the outbound envelope.
    let envelope_json = envelope.to_provenance_data();
    ctx.service
        .invoke(service_ref, &envelope_json, Some(&invocation_id))
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
            invocation_id: invocation_id.clone(),
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
            "invocationId": invocation_id,
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
    invocation_id: &str,
) -> String {
    if let Some(template) = binding
        .extensions
        .get("subject")
        .and_then(|v| v.as_str())
    {
        return template.to_string();
    }
    format!("{instance_id}:{binding_id}:{invocation_id}")
}

/// Generate a stable invocation identifier for this callback firing.
fn next_invocation_id(record: &RuntimeRecord, service_ref: &str) -> String {
    let seq = record.step_results.len();
    format!("{service_ref}-cb-{seq}")
}
