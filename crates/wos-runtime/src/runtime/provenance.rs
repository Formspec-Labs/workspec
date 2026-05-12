// Rust guideline compliant 2026-02-21

//! Provenance append-path helpers.
//!
//! This module owns the runtime-side shaping of provenance records immediately
//! before persistence: timestamp stamping, Semantic Profile field population,
//! compensation records, and task validation provenance construction.

use wos_core::instance::CaseInstance;
use wos_core::model::kernel::{ActorKind, KernelDocument};
use wos_core::provenance::{ProvenanceAuditTier, ProvenanceKind, ProvenanceRecord};

use crate::binding::SubmissionValidation;
use crate::custody::CustodyAppendReceipt;

/// Stamp every record whose `timestamp` is empty with `now_iso`.
///
/// Records that already carry a timestamp are left untouched. This is the
/// single authoritative point where an empty `ProvenanceRecord::timestamp` is
/// filled in on the append path.
pub fn stamp_provenance(records: &mut [ProvenanceRecord], now_iso: &str) {
    for record in records {
        if record.timestamp.is_empty() {
            record.timestamp = now_iso.to_string();
        }
    }
}

/// Populate push-stamped Semantic Profile fields on `records`.
///
/// This is the sole append-path site where these fields are filled in, so
/// every record handed to the store downstream carries the full SP-required
/// shape. Each field is set only when it is currently `None` or empty.
pub fn populate_provenance_record_fields(
    records: &mut [ProvenanceRecord],
    kernel: &KernelDocument,
    definition_version: &str,
) {
    for record in records {
        if record.audit_layer.is_none() {
            record.audit_layer = Some(
                ProvenanceAuditTier::from(record.record_kind)
                    .as_str()
                    .to_string(),
            );
        }

        if record.actor_type.is_none()
            && let Some(actor_id) = record.actor_id.as_deref()
            && let Some(actor) = kernel.actors.iter().find(|a| a.id == actor_id)
        {
            record.actor_type = Some(match actor.kind {
                ActorKind::Human => "human".to_string(),
                ActorKind::System => "system".to_string(),
                ActorKind::Agent => "agent".to_string(),
            });
        }

        if record.definition_version.is_none() && !definition_version.is_empty() {
            record.definition_version = Some(definition_version.to_string());
        }

        if record.lifecycle_state.is_none() {
            match record.record_kind {
                ProvenanceKind::StateTransition => {
                    if let Some(from) = record.from_state.as_deref() {
                        record.lifecycle_state = Some(from.to_string());
                    }
                }
                ProvenanceKind::CaseStateMutation => {
                    let state = record
                        .data
                        .as_ref()
                        .and_then(|d| d.get("lifecycleState"))
                        .and_then(serde_json::Value::as_str);
                    if let Some(state) = state {
                        record.lifecycle_state = Some(state.to_string());
                    }
                }
                _ => {}
            }
        }

        match record.record_kind {
            ProvenanceKind::StateTransition => {
                if record.inputs.is_empty()
                    && let Some(event) = record.event.as_deref()
                {
                    record.inputs = vec![event.to_string()];
                }
                if record.outputs.is_empty()
                    && let Some(to_state) = record.to_state.as_deref()
                {
                    record.outputs = vec![to_state.to_string()];
                }
            }
            ProvenanceKind::CaseStateMutation => {
                if record.inputs.is_empty()
                    && let Some(path) = record
                        .data
                        .as_ref()
                        .and_then(|d| d.get("path"))
                        .and_then(serde_json::Value::as_str)
                {
                    record.inputs = vec![path.to_string()];
                }
                if record.outputs.is_empty()
                    && let Some(new_value) = record.data.as_ref().and_then(|d| d.get("newValue"))
                {
                    record.outputs = vec![stringify_scalar(new_value)];
                }
            }
            _ => {}
        }

        if record.input_digest.is_none() {
            record.input_digest = digest_of(&record.inputs);
        }
        if record.output_digest.is_none() {
            record.output_digest = digest_of(&record.outputs);
        }
    }
}

/// Stamps workflow identity into signature decision payloads.
///
/// F-11 keeps workflow identity in the WOS profile payload, not in the
/// primitive integrity-event envelope. Until `DecisionEvent` is a first-class
/// type, signature admission/affirmation provenance records are the runtime's
/// decision payloads and carry the bound case ledger plus workflow process.
pub fn stamp_signature_decision_identity(
    records: &mut [ProvenanceRecord],
    instance: &CaseInstance,
) {
    let case_ledger_id = instance.effective_case_ledger_id();
    if !CaseInstance::is_case_id(case_ledger_id) {
        return;
    }
    let process_id = instance.effective_process_id();
    let process_id = CaseInstance::is_process_id(process_id).then_some(process_id);

    for record in records {
        if !matches!(
            record.record_kind,
            ProvenanceKind::SignatureAffirmation | ProvenanceKind::SignatureAdmissionFailed
        ) {
            continue;
        }
        let Some(data) = record
            .data
            .as_mut()
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        data.entry("caseLedgerId".to_string())
            .or_insert_with(|| serde_json::Value::String(case_ledger_id.to_string()));
        if let Some(process_id) = process_id {
            data.entry("processId".to_string())
                .or_insert_with(|| serde_json::Value::String(process_id.to_string()));
        }
    }
}

/// Applying a custody append receipt when the record already carries a
/// different Trellis hash.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustodyReceiptStampError {
    #[error(
        "custody receipt conflicts with existing canonical_event_hash: existing={existing}, attempted={attempted}"
    )]
    Conflict { existing: String, attempted: String },
}

/// Stamps Trellis's admitting `canonical_event_hash` onto a provenance record.
///
/// # Errors
///
/// Returns [`CustodyReceiptStampError::Conflict`] when the record already
/// holds a different hash; idempotent retries pass when the attempted hash
/// matches the stored value.
pub fn stamp_custody_receipt(
    record: &mut ProvenanceRecord,
    receipt: &CustodyAppendReceipt,
) -> Result<(), CustodyReceiptStampError> {
    let attempted = receipt.canonical_event_hash.clone();
    match &record.canonical_event_hash {
        None => {
            record.canonical_event_hash = Some(attempted);
            Ok(())
        }
        Some(existing) if existing == &attempted => Ok(()),
        Some(existing) => Err(CustodyReceiptStampError::Conflict {
            existing: existing.clone(),
            attempted,
        }),
    }
}

pub(super) fn compensation_provenance(
    kernel: &KernelDocument,
    persisted_provenance: &[ProvenanceRecord],
    appended_provenance: &[ProvenanceRecord],
) -> Vec<ProvenanceRecord> {
    let compensation_started_now = appended_provenance.iter().any(|record| {
        record.record_kind == ProvenanceKind::StateTransition
            && record.to_state.as_deref() == Some("compensating")
    });
    if !compensation_started_now {
        return Vec::new();
    }

    let transitions: Vec<(&str, &str)> = persisted_provenance
        .iter()
        .chain(appended_provenance.iter())
        .filter(|record| record.record_kind == ProvenanceKind::StateTransition)
        .filter_map(|record| Some((record.from_state.as_deref()?, record.to_state.as_deref()?)))
        .collect();

    let mut visited: Vec<&str> = vec![kernel.lifecycle.initial_state.as_str()];
    for (_, to) in &transitions {
        if *to != "compensating" && *to != "compensated" && *to != "done" {
            visited.push(to);
        }
    }

    let mut provenance = Vec::new();
    let fail_transition = transitions.iter().find(|(_, to)| *to == "compensating");
    if visited.len() >= 3 {
        let mut reversed = visited;
        reversed.reverse();
        provenance.push(ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::CompensationExecuted,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "order": reversed })),
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
        provenance.push(ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::CompensationScopeBoundary,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "innerScopeOnly": true })),
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
    } else if visited.len() == 2
        && let Some((from, _)) = fail_transition
    {
        let compensated: Vec<&str> = visited
            .iter()
            .filter(|state| **state != *from)
            .copied()
            .collect();
        provenance.push(ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::CompensationExecuted,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "pivotStep": from,
                "compensated": compensated,
                "excluded": [*from],
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

    provenance
}

pub(super) fn contract_validation_record(
    task_id: &str,
    actor_id: &str,
    response: &serde_json::Value,
    validation: &SubmissionValidation,
) -> ProvenanceRecord {
    ProvenanceRecord::contract_validation(
        task_id,
        Some(actor_id),
        serde_json::json!({
            "response": response,
            "validationOutcome": validation.validation_outcome,
        }),
    )
}

fn stringify_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn digest_of(items: &[String]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    use sha2::{Digest, Sha256};
    let payload = serde_json::to_string(items).unwrap_or_default();
    Some(format!("{:x}", Sha256::digest(payload.as_bytes())))
}

#[cfg(test)]
mod stamp_custody_receipt_tests {
    use super::*;
    use crate::custody::CustodyAppendReceipt;

    const HASH_A: &str = "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c";
    const HASH_B: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn stamps_when_absent() {
        let mut record = ProvenanceRecord::unmatched_event("e", None);
        stamp_custody_receipt(
            &mut record,
            &CustodyAppendReceipt {
                canonical_event_hash: HASH_A.to_string(),
            },
        )
        .expect("stamp");
        assert_eq!(record.canonical_event_hash.as_deref(), Some(HASH_A));
    }

    #[test]
    fn no_op_when_hash_unchanged() {
        let mut record = ProvenanceRecord::unmatched_event("e", None);
        record.canonical_event_hash = Some(HASH_A.to_string());
        stamp_custody_receipt(
            &mut record,
            &CustodyAppendReceipt {
                canonical_event_hash: HASH_A.to_string(),
            },
        )
        .expect("idempotent");
        assert_eq!(record.canonical_event_hash.as_deref(), Some(HASH_A));
    }

    #[test]
    fn conflict_when_hash_differs() {
        let mut record = ProvenanceRecord::unmatched_event("e", None);
        record.canonical_event_hash = Some(HASH_A.to_string());
        let err = stamp_custody_receipt(
            &mut record,
            &CustodyAppendReceipt {
                canonical_event_hash: HASH_B.to_string(),
            },
        )
        .expect_err("conflict");
        assert_eq!(
            err,
            CustodyReceiptStampError::Conflict {
                existing: HASH_A.to_string(),
                attempted: HASH_B.to_string(),
            }
        );
        assert_eq!(record.canonical_event_hash.as_deref(), Some(HASH_A));
    }
}
