// Rust guideline compliant 2026-04-14

//! Handler for `event-emit` integration bindings.
//!
//! Builds a CloudEvents 1.0 envelope from binding metadata and the FEL
//! `data_mapping`, dispatches it to the external service (outbound channel),
//! and emits `EventEmitted` provenance with the full envelope captured in
//! `data`.

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::cloudevents::CloudEvent;
use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::request_response::{InvocationContext, build_event_data_from_binding};
use super::{IntegrationBindingHandler, next_outbound_event_id};

/// Handler for outbound CloudEvent emission bindings.
pub(crate) struct EventEmitHandler;

impl IntegrationBindingHandler for EventEmitHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::EventEmit
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
        // `outbound_event_id` is the CloudEvent `id` for this emission — a unique
        // event identifier, not an idempotency key.
        let outbound_event_id = next_outbound_event_id(record, service_ref, "emit");
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
                .unwrap_or("com.wos.integration.event-emit")
                .to_string(),
            subject: Some(subject),
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

        // Dispatch the envelope to the outbound service channel.
        // The outbound_event_id is the CloudEvent `id`, not an idempotency key — each
        // event emission is a new event. Pass None for idempotency.
        let envelope_json = envelope.to_provenance_data();
        ctx.service
            .invoke(service_ref, &envelope_json, None)
            .map_err(|e| RuntimeError::Service(e.to_string()))?;

        let provenance = ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::EventEmitted,
            timestamp: String::new(),
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(envelope.to_provenance_data()),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        };

        Ok(vec![provenance])
    }
}

/// Derive the CloudEvents `subject` for a binding invocation.
///
/// If the binding declares an explicit `subject` extension field, that value is
/// used verbatim, enabling custom routing keys or override templates.
/// Otherwise the canonical WOS correlation format is used:
/// `{instanceId}:{bindingId}:{outbound_event_id}`.
fn compute_subject(
    binding: &IntegrationBinding,
    instance_id: &str,
    binding_id: &str,
    outbound_event_id: &str,
) -> String {
    if let Some(template) = binding.extensions.get("subject").and_then(|v| v.as_str()) {
        return template.to_string();
    }
    format!("{instance_id}:{binding_id}:{outbound_event_id}")
}
