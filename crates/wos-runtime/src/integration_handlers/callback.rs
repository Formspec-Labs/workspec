// Rust guideline compliant 2026-04-14

//! Handler for `callback` integration bindings.
//!
//! Explicit deferred phases persist an immutable native request plus callback
//! registration without invoking the provider. Matching ingress validates the
//! native envelope, selected binding snapshot and expected callback type before
//! applying output mappings or removing either half. Owner-managed transport and
//! receipt admission remain external; CloudEvent syntax is not authentication.
//!
//! Bindings without dispatchMode retain the maintained immediate behavior and
//! legacy phase inference. Explicit inbound phases fail closed on unknown
//! subjects, so an authored follow-on event cannot mistake a no-op for admission.

use wos_core::eval::ObservedAction;
use wos_core::instance::{PendingCallback, PendingOutboundEvent};
use wos_core::model::kernel::KernelDocument;
use wos_core::{ProvenanceKind, ProvenanceRecord};

use crate::cloudevents::CloudEvent;
use crate::integration::{
    CallbackPhase, IntegrationBinding, IntegrationBindingKind, IntegrationDispatchMode,
};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::request_response::{
    InvocationContext, apply_output_binding, build_event_data_from_binding,
};
use super::{IntegrationBindingHandler, next_outbound_event_id};

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
        binding
            .validate_dispatch()
            .map_err(RuntimeError::Integration)?;
        if binding.dispatch_mode == Some(IntegrationDispatchMode::Deferred) {
            validate_pending_requests(&record.process).map_err(RuntimeError::Integration)?;
            return match observed.action.callback_phase {
                Some(CallbackPhase::Outbound) => {
                    handle_outbound(ctx, record, kernel, observed, service_ref, binding, now_iso)
                }
                Some(CallbackPhase::Inbound) => {
                    handle_inbound(record, observed, service_ref, binding)
                }
                None => Err(RuntimeError::Integration(
                    "deferred callback requires explicit callbackPhase".into(),
                )),
            };
        }
        if let Some(phase) = observed.action.callback_phase {
            return match phase {
                CallbackPhase::Outbound => {
                    handle_outbound(ctx, record, kernel, observed, service_ref, binding, now_iso)
                }
                CallbackPhase::Inbound => handle_inbound(record, observed, service_ref, binding),
            };
        }
        match classify_invocation(observed, &record.process.pending_callbacks) {
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
    let subject = data.get("subject").and_then(|v| v.as_str()).unwrap_or("");
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
    let deferred = binding.dispatch_mode == Some(IntegrationDispatchMode::Deferred);
    // Refuse count exhaustion before request mapping, ID construction or sequence allocation.
    if deferred && record.process.pending_outbound_events.len() >= ctx.deferred_callback_max_count {
        return Err(RuntimeError::Integration(
            "deferred callback pending-count limit exceeded".into(),
        ));
    }
    let next_sequence = if deferred {
        record
            .process
            .next_deferred_callback_sequence
            .checked_add(1)
            .ok_or_else(|| {
                RuntimeError::Integration("deferred callback sequence exhausted".into())
            })?
    } else {
        record.process.next_deferred_callback_sequence
    };
    let outbound_event_id = if deferred {
        format!(
            "{}:{service_ref}:deferred:{}",
            record.process.process_id, record.process.next_deferred_callback_sequence
        )
    } else {
        next_outbound_event_id(record, service_ref, "cb")
    };
    let subject = compute_subject(
        binding,
        record.process.correlation_process_id(),
        service_ref,
        &outbound_event_id,
    );

    if deferred
        && (subject.trim().is_empty()
            || record.process.pending_callbacks.contains_key(&subject)
            || record.provenance_log.iter().any(|record| {
                record.record_kind == ProvenanceKind::CallbackPending
                    && record
                        .data
                        .as_ref()
                        .and_then(|data| data.get("subject"))
                        .and_then(serde_json::Value::as_str)
                        == Some(subject.as_str())
            }))
    {
        return Err(RuntimeError::Integration(
            "callback subject already registered".into(),
        ));
    }
    let event_data = build_event_data_from_binding(binding, kernel, observed, &record.process)?;

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
    envelope
        .validate_ingress()
        .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    if deferred {
        let mut size = LimitedJsonBytes::new(ctx.deferred_callback_max_bytes);
        for pending in record.process.pending_outbound_events.values() {
            serde_json::to_writer(&mut size, &pending.request).map_err(|_| {
                RuntimeError::Integration("deferred callback byte limit exceeded".into())
            })?;
        }
        serde_json::to_writer(&mut size, &envelope_json).map_err(|_| {
            RuntimeError::Integration("deferred callback byte limit exceeded".into())
        })?;
        // Both persisted halves are installed only after every precondition succeeds.
        record.process.pending_outbound_events.insert(
            subject.clone(),
            PendingOutboundEvent {
                definition_url: record.process.definition_url.clone(),
                definition_version: record.process.definition_version.clone(),
                binding_id: service_ref.into(),
                invocation_id: outbound_event_id.clone(),
                subject: subject.clone(),
                request: envelope_json,
                binding_snapshot: binding.clone(),
            },
        );
        record.process.next_deferred_callback_sequence = next_sequence;
    } else {
        ctx.service
            .invoke(service_ref, &envelope_json, None)
            .map_err(|e| RuntimeError::Service(e.to_string()))?;
    }

    // Read deadline from binding extensions if present.
    let expected_until = binding
        .extensions
        .get("expectedUntil")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // Register the pending callback for later resolution.
    record.process.pending_callbacks.insert(
        subject.clone(),
        PendingCallback {
            dispatch_mode: deferred.then_some(IntegrationDispatchMode::Deferred),
            invocation_id: outbound_event_id.clone(),
            binding_id: service_ref.to_string(),
            expected_until: expected_until.clone(),
        },
    );

    let mut pending_data = serde_json::json!({
        "subject":subject,"bindingId":service_ref,"invocationId":outbound_event_id,"expectedUntil":expected_until,
    });
    if deferred {
        pending_data["dispatchMode"] = serde_json::json!("deferred");
        pending_data["deferredRequest"] =
            serde_json::to_value(record.process.pending_outbound_events.get(&subject))
                .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    }
    let pending_provenance = ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::CallbackPending,
        timestamp: String::new(),
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(pending_data),
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

    let deferred = record
        .process
        .pending_outbound_events
        .contains_key(&subject)
        || binding.dispatch_mode == Some(IntegrationDispatchMode::Deferred);
    validate_callback_inbound(record, observed, service_ref, binding)?;

    // Apply output binding using the envelope's data payload.
    let event_data = envelope
        .data
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    // Stage all mappings before mutating state or consuming either pending half.
    let mut mapped_state = record.process.case_state.clone();
    let updates = apply_output_binding(&mut mapped_state, &binding.output_binding, &event_data)?;

    let original_invocation = record
        .process
        .pending_callbacks
        .get(&subject)
        .map(|pending| pending.invocation_id.clone());
    record.process.case_state = mapped_state;
    record.process.pending_callbacks.remove(&subject);
    if deferred {
        record.process.pending_outbound_events.remove(&subject);
    }
    let mut provenance = Vec::new();

    if !updates.is_empty() {
        provenance.push(ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::DataMapping,
            timestamp: String::new(),
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "serviceRef": service_ref,
                "integrationType": "callback",
                "updatedPaths": updates,
            })),
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
        });
    }

    let mut received_data = envelope.to_provenance_data();
    // Annotate with correlation subject for easy extraction from provenance.
    if let Some(obj) = received_data.as_object_mut() {
        obj.insert("correlationSubject".to_string(), serde_json::json!(subject));
        if deferred {
            obj.insert(
                "invocationId".into(),
                serde_json::json!(original_invocation),
            );
            obj.insert("bindingId".into(), serde_json::json!(service_ref));
        }
    }

    provenance.push(ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: ProvenanceKind::CallbackReceived,
        timestamp: String::new(),
        actor_id: observed.actor_id.clone(),
        from_state: None,
        to_state: None,
        event: None,
        data: Some(received_data),
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
    });

    Ok(provenance)
}

/// Derive the CloudEvents `subject` for a callback outbound invocation.
///
/// Canonical format: `{correlationInstanceId}:{bindingId}:{invocationId}`.
/// The binding may override this via an `extensions.subject` field.
fn compute_subject(
    binding: &IntegrationBinding,
    correlation_process_id: &str,
    binding_id: &str,
    outbound_event_id: &str,
) -> String {
    if let Some(template) = binding.extensions.get("subject").and_then(|v| v.as_str()) {
        return template.to_string();
    }
    format!("{correlation_process_id}:{binding_id}:{outbound_event_id}")
}

/// Counts exact serialized request bytes without allocating a second request buffer.
struct LimitedJsonBytes {
    remaining: usize,
}
impl LimitedJsonBytes {
    fn new(limit: usize) -> Self {
        Self { remaining: limit }
    }
}
impl std::io::Write for LimitedJsonBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("deferred callback byte limit exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn validate_pending_requests(
    process: &wos_core::instance::WorkflowProcess,
) -> Result<(), String> {
    process.validate_deferred_callbacks()?;
    let mut bytes = LimitedJsonBytes::new(1024 * 1024);
    if process.pending_outbound_events.len() > 1000 {
        return Err("deferred callback count limit exceeded".into());
    }
    for pending in process.pending_outbound_events.values() {
        let event: CloudEvent =
            serde_json::from_value(pending.request.clone()).map_err(|error| error.to_string())?;
        event
            .validate_ingress()
            .map_err(|error| error.to_string())?;
        if event.to_provenance_data() != pending.request {
            return Err(
                "stored deferred request differs from native CloudEvent representation".into(),
            );
        }
        serde_json::to_writer(&mut bytes, &pending.request)
            .map_err(|_| "deferred callback byte limit exceeded".to_string())?;
    }
    Ok(())
}

pub(crate) fn validate_pending_update(
    incoming: &RuntimeRecord,
    previous: &RuntimeRecord,
) -> Result<(), String> {
    validate_pending_record(previous)?;
    validate_pending_record(incoming)?;
    incoming
        .process
        .validate_deferred_callback_update(&previous.process)?;
    for (subject, old) in &previous.process.pending_outbound_events {
        if let Some(retained) = incoming.process.pending_outbound_events.get(subject) {
            if old != retained
                || previous.process.pending_callbacks.get(subject)
                    != incoming.process.pending_callbacks.get(subject)
            {
                return Err("retained deferred request and registration are immutable".into());
            }
        } else {
            let matched = incoming
                .provenance_log
                .iter()
                .skip(previous.provenance_log.len())
                .any(|record| {
                    if record.record_kind != ProvenanceKind::CallbackReceived {
                        return false;
                    }
                    let Some(data) = record.data.as_ref() else {
                        return false;
                    };
                    let Ok(receipt) = serde_json::from_value::<CloudEvent>(data.clone()) else {
                        return false;
                    };
                    receipt.validate_ingress().is_ok()
                        && receipt.subject.as_deref() == Some(subject.as_str())
                        && receipt.id != old.invocation_id
                        && old
                            .binding_snapshot
                            .extensions
                            .get("callbackEventType")
                            .and_then(serde_json::Value::as_str)
                            == Some(receipt.event_type.as_str())
                });
            if !matched {
                return Err(
                    "deferred request cannot disappear without matching admitted receipt".into(),
                );
            }
        }
    }
    Ok(())
}

/// Preflights deferred ingress before any observed provider action can run.
pub(crate) fn validate_callback_inbound(
    record: &RuntimeRecord,
    observed: &ObservedAction,
    service_ref: &str,
    binding: &IntegrationBinding,
) -> Result<(), RuntimeError> {
    validate_pending_requests(&record.process).map_err(RuntimeError::Integration)?;
    let envelope: CloudEvent =
        serde_json::from_value(observed.event_data.clone().ok_or_else(|| {
            RuntimeError::Integration("inbound callback requires native CloudEvent".into())
        })?)
        .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    envelope
        .validate_ingress()
        .map_err(|error| RuntimeError::Integration(error.to_string()))?;
    let subject = envelope
        .subject
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| RuntimeError::Integration("inbound callback requires subject".into()))?;
    let callback = record
        .process
        .pending_callbacks
        .get(subject)
        .ok_or_else(|| RuntimeError::Integration("unknown callback subject".into()))?;
    if callback.binding_id != service_ref {
        return Err(RuntimeError::Integration(
            "callback subject belongs to another binding".into(),
        ));
    }
    if let Some(pending) = record.process.pending_outbound_events.get(subject) {
        if pending.binding_id != service_ref || pending.binding_snapshot != *binding {
            return Err(RuntimeError::Integration(
                "deferred callback binding differs from original request".into(),
            ));
        }
        if envelope.id == pending.invocation_id {
            return Err(RuntimeError::Integration(
                "receipt requires its own event identity".into(),
            ));
        }
        if binding
            .extensions
            .get("callbackEventType")
            .and_then(serde_json::Value::as_str)
            != Some(envelope.event_type.as_str())
        {
            return Err(RuntimeError::Integration(
                "unexpected deferred callback event type".into(),
            ));
        }
    } else if binding.dispatch_mode == Some(IntegrationDispatchMode::Deferred) {
        return Err(RuntimeError::Integration(
            "deferred callback lacks native request".into(),
        ));
    }
    // Resolve every output against an isolated copy; none may partly mutate case state.
    let mut mapped = record.process.case_state.clone();
    apply_output_binding(
        &mut mapped,
        &binding.output_binding,
        &envelope.data.unwrap_or_else(|| serde_json::json!({})),
    )?;
    Ok(())
}

/// Validates original deferred witnesses in both directions, including consumed requests.
pub(crate) fn validate_pending_record(record: &RuntimeRecord) -> Result<(), String> {
    validate_pending_requests(&record.process)?;
    let mut receipts: std::collections::HashMap<String, Vec<(String, CloudEvent)>> =
        std::collections::HashMap::new();
    for provenance in record
        .provenance_log
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::CallbackReceived)
    {
        let Some(data) = provenance.data.as_ref() else {
            continue;
        };
        if data.get("invocationId").is_none() {
            continue;
        }
        let invocation = data
            .get("invocationId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or("deferred receipt invocation is malformed")?;
        let binding = data
            .get("bindingId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or("deferred receipt binding is malformed")?;
        let event: CloudEvent =
            serde_json::from_value(data.clone()).map_err(|error| error.to_string())?;
        event
            .validate_ingress()
            .map_err(|error| error.to_string())?;
        receipts
            .entry(invocation.into())
            .or_default()
            .push((binding.into(), event));
    }
    let mut subjects = std::collections::HashSet::new();
    let mut invocation_ids = std::collections::HashSet::new();
    for provenance in &record.provenance_log {
        if provenance.record_kind != ProvenanceKind::CallbackPending {
            continue;
        }
        let Some(data) = provenance.data.as_ref() else {
            continue;
        };
        if data.get("dispatchMode").is_none() && data.get("deferredRequest").is_none() {
            continue;
        }
        if data.get("dispatchMode").and_then(serde_json::Value::as_str) != Some("deferred") {
            return Err("original deferred callback witness has invalid dispatch mode".into());
        }
        let request: PendingOutboundEvent = serde_json::from_value(
            data.get("deferredRequest")
                .cloned()
                .ok_or("deferred callback witness lacks original request")?,
        )
        .map_err(|error| error.to_string())?;
        request.binding_snapshot.validate_dispatch()?;
        let event: CloudEvent =
            serde_json::from_value(request.request.clone()).map_err(|error| error.to_string())?;
        event
            .validate_ingress()
            .map_err(|error| error.to_string())?;
        if request.subject.trim().is_empty()
            || request.definition_url.trim().is_empty()
            || request.definition_version.trim().is_empty()
            || request.binding_snapshot.dispatch_mode != Some(IntegrationDispatchMode::Deferred)
            || event.to_provenance_data() != request.request
            || event.id != request.invocation_id
            || event.subject.as_deref() != Some(request.subject.as_str())
            || data.get("subject").and_then(serde_json::Value::as_str)
                != Some(request.subject.as_str())
            || data.get("bindingId").and_then(serde_json::Value::as_str)
                != Some(request.binding_id.as_str())
            || data.get("invocationId").and_then(serde_json::Value::as_str)
                != Some(request.invocation_id.as_str())
            || !subjects.insert(request.subject.clone())
        {
            return Err("original deferred callback witness is inconsistent or repeated".into());
        }
        let prefix = format!(
            "{}:{}:deferred:",
            record.process.process_id, request.binding_id
        );
        let sequence = request
            .invocation_id
            .strip_prefix(&prefix)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or("original deferred invocation identity is malformed")?;
        if sequence >= record.process.next_deferred_callback_sequence
            || !invocation_ids.insert(request.invocation_id.clone())
        {
            return Err(
                "original deferred invocation identity exceeds sequence or is repeated".into(),
            );
        }
        let matched = receipts.remove(&request.invocation_id).unwrap_or_default();
        if matched.iter().any(|(binding, receipt)| {
            binding != &request.binding_id
                || receipt.subject.as_deref() != Some(request.subject.as_str())
                || receipt.id == request.invocation_id
                || request
                    .binding_snapshot
                    .extensions
                    .get("callbackEventType")
                    .and_then(serde_json::Value::as_str)
                    != Some(receipt.event_type.as_str())
        }) {
            return Err("deferred received witness differs from original request".into());
        }
        if let Some(pending) = record.process.pending_outbound_events.get(&request.subject) {
            if pending != &request || !matched.is_empty() {
                return Err("retained deferred request differs from original witness or was already consumed".into());
            }
        } else if matched.len() != 1 {
            return Err(
                "removed deferred request requires exactly one matching received witness".into(),
            );
        }
    }
    if !receipts.is_empty() {
        return Err("deferred received witness lacks original request".into());
    }
    for subject in record.process.pending_outbound_events.keys() {
        if !subjects.contains(subject) {
            return Err("deferred request lacks original callback witness".into());
        }
    }
    Ok(())
}
