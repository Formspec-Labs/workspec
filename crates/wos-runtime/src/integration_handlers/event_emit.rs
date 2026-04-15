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

use super::IntegrationBindingHandler;
use super::request_response::{InvocationContext, build_event_data_from_binding};

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
                .unwrap_or("com.wos.integration.event-emit")
                .to_string(),
            subject: Some(subject),
            time: Some(
                now_iso
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
            ),
            data_content_type: Some("application/json".to_string()),
            data: Some(event_data),
        };

        // Dispatch the envelope to the outbound service channel.
        // For emit-only bindings the service response is not mapped back to case state.
        let envelope_json = envelope.to_provenance_data();
        ctx.service
            .invoke(service_ref, &envelope_json, Some(&invocation_id))
            .map_err(|e| RuntimeError::Service(e.to_string()))?;

        let provenance = ProvenanceRecord {
            record_kind: ProvenanceKind::EventEmitted,
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(envelope.to_provenance_data()),
        };

        Ok(vec![provenance])
    }
}

/// Derive the CloudEvents `subject` for a binding invocation.
///
/// If the binding declares an explicit `subject` extension field, that value is
/// used verbatim, enabling custom routing keys or override templates.
/// Otherwise the canonical WOS correlation format is used:
/// `{instanceId}:{bindingId}:{invocationId}`.
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

/// Generate a stable invocation identifier for this binding execution.
///
/// Incorporates the current step-result count so it is unique per invocation
/// within an instance's lifetime without requiring an external UUID source.
fn next_invocation_id(record: &RuntimeRecord, service_ref: &str) -> String {
    let seq = record.step_results.len();
    format!("{service_ref}-emit-{seq}")
}
