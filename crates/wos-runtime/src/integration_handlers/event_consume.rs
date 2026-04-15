// Rust guideline compliant 2026-04-14

//! Handler for `event-consume` integration bindings.
//!
//! Expects the triggering action's `data` field to contain a full CloudEvents
//! 1.0 envelope JSON object. The handler validates ingress (rejecting events
//! with empty required fields), applies the binding's `output_binding` to map
//! the envelope's `data` payload into case state, and emits `EventConsumed`
//! provenance with the full envelope.
//!
//! # Inbound event delivery
//!
//! In the conformance harness, inbound CloudEvents are delivered by including
//! the full CloudEvent JSON object in the `data` field of an event-sequence
//! entry. The runtime passes this `data` value through as `observed.action.data`.
//! Production runtimes may inject inbound events via an adapter layer; the
//! handler interface is the same.

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::cloudevents::{CloudEvent, CloudEventError};
use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::IntegrationBindingHandler;
use super::request_response::{InvocationContext, apply_output_binding};

/// Handler for inbound CloudEvent consumption bindings.
pub(crate) struct EventConsumeHandler;

impl IntegrationBindingHandler for EventConsumeHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::EventConsume
    }

    fn execute(
        &self,
        _ctx: &InvocationContext<'_>,
        record: &mut RuntimeRecord,
        _kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        _now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        // Extract the CloudEvent envelope from the triggering event data payload.
        // The runtime event payload is carried in `observed.event_data` (set by the
        // evaluator from the EventEntry's `data` field).
        let envelope_json = observed.event_data.clone().ok_or_else(|| {
            RuntimeError::Integration(format!(
                "event-consume binding '{service_ref}': no CloudEvent envelope in event data"
            ))
        })?;

        let envelope: CloudEvent = serde_json::from_value(envelope_json).map_err(|e| {
            RuntimeError::Integration(format!(
                "event-consume binding '{service_ref}': failed to parse CloudEvent envelope: {e}"
            ))
        })?;

        // Reject at the binding boundary if required CE attributes are invalid.
        envelope.validate_ingress().map_err(|e| {
            RuntimeError::Integration(format!(
                "event-consume binding '{service_ref}': {}", ingress_rejection_message(e)
            ))
        })?;

        // Apply output binding: map envelope data into case state.
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
                timestamp: String::new(),
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": "event-consume",
                    "updatedPaths": updates,
                })),
            });
        }

        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::EventConsumed,
            timestamp: String::new(),
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(envelope.to_provenance_data()),
        });

        Ok(provenance)
    }
}

/// Format a human-readable rejection message for a CloudEvent ingress error.
///
/// This text surfaces in `RuntimeError::Integration`, which propagates to the
/// conformance result failure list and makes fixture failures easy to diagnose.
fn ingress_rejection_message(err: CloudEventError) -> String {
    format!("EventIngressInvalid — {err}")
}
