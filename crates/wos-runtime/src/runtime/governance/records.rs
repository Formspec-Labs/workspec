// Rust guideline compliant 2026-02-21
//! Validates immutable due-process identity and monotonic retained evidence.
use super::*;
use super::{admission::*, operations::state};
use crate::store::StoreError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::HashSet;
use wos_core::ProvenanceKind;
use wos_core::model::due_process::*;

fn refused() -> StoreError {
    StoreError::InvalidProvenanceEvidence(
        "native due-process evidence is missing, changed, duplicated or rebound".into(),
    )
}
fn exactly<'a>(
    record: &'a RuntimeRecord,
    id: &str,
    kind: ProvenanceKind,
) -> Result<&'a wos_core::ProvenanceRecord, StoreError> {
    let mut matches = record.provenance_log.iter().filter(|p| p.id == id);
    let found = matches
        .next()
        .filter(|p| p.record_kind == kind)
        .ok_or_else(refused)?;
    if matches.next().is_some() {
        return Err(refused());
    }
    Ok(found)
}

fn public_request<'a>(
    state: &'a DueProcessState,
    operation: &OperationState,
) -> Result<&'a GovernanceRequest, StoreError> {
    let mut matches = state.replays.iter().filter(|replay| {
        replay.result
            == GovernanceResult::Committed {
                operation_record_id: operation.record_id.clone(),
                outcome: operation.outcome.clone(),
            }
            && replay.request.operation_ref == operation.operation_ref
            && replay.request.determination_id == operation.determination_id
            && replay.request.actor == operation.actor
            && replay.request.command.operation_kind() == operation.kind
    });
    let request = &matches.next().ok_or_else(refused)?.request;
    if matches.next().is_some() {
        return Err(refused());
    }
    Ok(request)
}

fn appeal_operation<'a>(
    state: &'a DueProcessState,
    appeal: &AppealState,
    id: &str,
    kind: OperationKind,
) -> Result<(&'a OperationState, &'a GovernanceRequest), StoreError> {
    let operation = state
        .operations
        .iter()
        .find(|op| {
            op.record_id == id
                && op.kind == kind
                && op.determination_id.as_ref() == Some(&appeal.determination_id)
        })
        .ok_or_else(refused)?;
    Ok((operation, public_request(state, operation)?))
}
pub(crate) fn validate(record: &RuntimeRecord) -> Result<(), StoreError> {
    let state = state(record);
    let witness = record
        .creation
        .as_ref()
        .and_then(|c| c.reference_due_process.as_ref());
    if let Some(witness) = witness {
        if record
            .process
            .governance_state
            .as_ref()
            .and_then(|g| g.due_process.as_ref())
            .is_none()
        {
            return Err(refused());
        }
        for p in record
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == ProvenanceKind::StateTransition)
        {
            if p.transition_tags.iter().any(|t| t == "adverse-decision")
                && !state.determinations.iter().any(|d| d.id == p.id)
            {
                return Err(refused());
            }
            if let Some(location) = p
                .data
                .as_ref()
                .and_then(|data| data.get("nativeTransitionLocation"))
            {
                let location: TransitionLocation =
                    serde_json::from_value(location.clone()).map_err(|_| refused())?;
                if witness
                    .execution
                    .operations
                    .iter()
                    .any(|o| o.transition == location)
                    && !state.operations.iter().any(|o| o.record_id == p.id)
                {
                    return Err(refused());
                }
                if witness
                    .execution
                    .determinations
                    .iter()
                    .any(|d| d.transition == location)
                    && !state.determinations.iter().any(|d| d.id == p.id)
                {
                    return Err(refused());
                }
            }
        }
    } else if state != DueProcessState::default() {
        return Err(refused());
    }

    if state == DueProcessState::default() {
        if record.provenance_log.iter().any(|p| {
            matches!(
                p.record_kind,
                ProvenanceKind::DeterminationReasoning
                    | ProvenanceKind::DeterminationCounterfactual
            )
        }) {
            return Err(refused());
        }
        return Ok(());
    }
    let mut ids = HashSet::new();
    for d in &state.determinations {
        if !ids.insert(&d.id)
            || d.firing.provenance_record_id != d.id
            || d.snapshot.record_id != d.id
            || timestamp(&d.determined_at).is_err()
        {
            return Err(refused());
        }
        let actual = exactly(record, &d.id, ProvenanceKind::StateTransition)?;
        let snapshot = actual.case_file_snapshot.as_ref().ok_or_else(refused)?;
        if actual.actor_id.as_ref() != Some(&d.determining_human)
            || actual.timestamp != d.determined_at
            || snapshot.sha256 != d.snapshot.sha256
            || wos_events::provenance::CaseFileSnapshot::from_case_state(&snapshot.value)
                != *snapshot
        {
            return Err(refused());
        }
        if d.service_pointers.is_empty()
            || d.service_pointers
                .iter()
                .any(|p| snapshot.value.pointer(p).is_none())
        {
            return Err(refused());
        }
        let reason = record
            .provenance_log
            .iter()
            .filter(|p| {
                p.record_kind == ProvenanceKind::DeterminationReasoning
                    && p.data
                        .as_ref()
                        .and_then(|d| d.get("relatedTransition"))
                        .and_then(serde_json::Value::as_str)
                        == Some(d.id.as_str())
            })
            .count();
        let counters: Vec<_> = record
            .provenance_log
            .iter()
            .filter(|p| {
                p.record_kind == ProvenanceKind::DeterminationCounterfactual
                    && p.data
                        .as_ref()
                        .and_then(|d| d.get("relatedTransition"))
                        .and_then(serde_json::Value::as_str)
                        == Some(d.id.as_str())
            })
            .collect();
        if reason != 1
            || !counters.iter().any(|p| {
                p.data
                    .as_ref()
                    .and_then(|d| d.get("type"))
                    .is_some_and(|v| v == "positive")
            })
            || !counters.iter().any(|p| {
                p.data
                    .as_ref()
                    .and_then(|d| d.get("type"))
                    .is_some_and(|v| v == "negative")
            })
        {
            return Err(refused());
        }
        if let Some(signature) = &d.signature {
            exactly(
                record,
                &signature.affirmation_record_id,
                ProvenanceKind::SignatureAffirmation,
            )?;
            let accepted: Vec<_> = record
                .artifacts
                .values()
                .filter(|a| {
                    a.kind == crate::store::TaskArtifactKind::Accepted
                        && a.task_id == signature.task_id
                        && a.actor_id == signature.supervisor_human
                })
                .collect();
            if accepted.len() != 1 {
                return Err(refused());
            }
            let response = &accepted[0].response;
            let payload = integrity_canonical::build_signed_payload(
                response.to_string().as_bytes(),
                integrity_canonical::DigestAlgorithm::Sha256,
            )
            .map_err(|_| refused())?;
            if signature.signed_content_sha256 != format!("sha256:{}", payload.digest)
                || response
                    .pointer("/data/determinationId")
                    .and_then(serde_json::Value::as_str)
                    != Some(d.id.as_str())
                || response
                    .pointer("/data/determinationSnapshotSha256")
                    .and_then(serde_json::Value::as_str)
                    != Some(d.snapshot.sha256.as_str())
                || response
                    .pointer("/data/determinationFiringRecordId")
                    .and_then(serde_json::Value::as_str)
                    != Some(d.firing.provenance_record_id.as_str())
                || response
                    .pointer("/data/signatureTaskId")
                    .and_then(serde_json::Value::as_str)
                    != Some(signature.task_id.as_str())
            {
                return Err(refused());
            }
        }
        if let Some(effect) = &d.effect {
            if !state.operations.iter().any(|op| {
                op.record_id == effect.operation_record_id
                    && op.kind == OperationKind::ApplyAdverseEffect
                    && op.determination_id.as_ref() == Some(&d.id)
            }) {
                return Err(refused());
            }
        }
    }
    let mut operation_ids = HashSet::new();
    for op in &state.operations {
        if !operation_ids.insert(&op.record_id) || op.record_id != op.firing.provenance_record_id {
            return Err(refused());
        }
        let actual = exactly(record, &op.record_id, ProvenanceKind::StateTransition)?;
        let binding = witness
            .and_then(|witness| {
                witness
                    .execution
                    .operations
                    .iter()
                    .find(|binding| binding.id == op.operation_ref)
            })
            .ok_or_else(refused)?;
        if actual.actor_id.as_ref() != Some(&op.actor.acting_actor)
            || actual.timestamp != op.performed_at
            || op.kind != binding.kind
            || op.firing.location != binding.transition
            || witness
                .is_none_or(|witness| op.firing.definition_sha256 != witness.definition_sha256)
            || actual
                .data
                .as_ref()
                .and_then(|data| data.get("nativeTransitionLocation"))
                != serde_json::to_value(&op.firing.location).ok().as_ref()
            || (op.kind == OperationKind::ExtendPreNoticeWindow) != op.determination_id.is_none()
            || op
                .determination_id
                .as_ref()
                .is_some_and(|id| !state.determinations.iter().any(|d| &d.id == id))
        {
            return Err(refused());
        }
        // Every committed milestone must retain its corresponding saved fact.
        // Checking only facts that remain would allow erasure on initial restore.
        let retained = match &op.outcome {
            GovernanceOutcome::NoticeStaged { notice_id } => {
                op.kind == OperationKind::IssueNotice
                    && state
                        .notices
                        .iter()
                        .any(|n| &n.id == notice_id && n.id == op.record_id)
            }
            GovernanceOutcome::AppealFiled { appeal_id } => {
                op.kind == OperationKind::FileAppeal
                    && state.appeals.iter().any(|a| &a.id == appeal_id)
            }
            GovernanceOutcome::HearingScheduled {
                appeal_id,
                scheduled_for,
            } => {
                op.kind == OperationKind::ScheduleHearing
                    && state.appeals.iter().any(|a| {
                        &a.id == appeal_id
                            && a.schedule.as_ref().is_some_and(|s| {
                                s.operation_record_id == op.record_id
                                    && &s.scheduled_for == scheduled_for
                            })
                    })
            }
            GovernanceOutcome::HearingHeld { appeal_id, held_at } => {
                op.kind == OperationKind::HoldHearing
                    && state.appeals.iter().any(|a| {
                        &a.id == appeal_id
                            && a.hearing.as_ref().is_some_and(|h| {
                                h.operation_record_id == op.record_id && &h.held_at == held_at
                            })
                    })
            }
            GovernanceOutcome::DispositionRecorded { appeal_id, outcome } => {
                op.kind == OperationKind::RecordDisposition
                    && state.appeals.iter().any(|a| {
                        &a.id == appeal_id
                            && a.disposition.as_ref().is_some_and(|d| {
                                d.operation_record_id == op.record_id && &d.outcome == outcome
                            })
                    })
            }
            GovernanceOutcome::ServiceTerminationRequested { appeal_id } => {
                op.kind == OperationKind::RequestServiceTermination
                    && state.appeals.iter().any(|a| {
                        &a.id == appeal_id
                            && a.termination
                                .as_ref()
                                .is_some_and(|t| t.operation_record_id == op.record_id)
                    })
            }
            GovernanceOutcome::PreNoticeWindowExtended {
                override_record_id,
                new_deadline,
            } => {
                op.kind == OperationKind::ExtendPreNoticeWindow
                    && state.extensions.iter().any(|e| {
                        e.operation_record_id == op.record_id
                            && record.provenance_log.iter().any(|p| {
                                p.id == *override_record_id
                                    && p.record_kind == ProvenanceKind::OverrideRecorded
                                    && p.data.as_ref()
                                        == serde_json::to_value(&e.override_record).ok().as_ref()
                            })
                            && &e.new_deadline == new_deadline
                    })
            }
            GovernanceOutcome::AdverseEffectApplied { applied_at } => {
                op.kind == OperationKind::ApplyAdverseEffect
                    && state.determinations.iter().any(|d| {
                        d.effect.as_ref().is_some_and(|e| {
                            e.operation_record_id == op.record_id && &e.applied_at == applied_at
                        })
                    })
            }
        };
        if !retained {
            return Err(refused());
        }
    }
    let mut notice_ids = HashSet::new();
    for n in &state.notices {
        if !unique(&n.transport_results)
            || n.transport_results.len() > n.attempts
            || n.transport_results
                .iter()
                .any(|r| validate_transport_result(r).is_err())
            || n.transport_results
                .iter()
                .filter(|r| !matches!(r, TransportResult::Failed { .. }))
                .count()
                > 1
        {
            return Err(refused());
        }
        if !notice_ids.insert(&n.id)
            || !state
                .determinations
                .iter()
                .any(|d| d.id == n.determination_id && d.signature.is_some())
            || !state.operations.iter().any(|op| {
                op.record_id == n.id
                    && op.outcome
                        == GovernanceOutcome::NoticeStaged {
                            notice_id: n.id.clone(),
                        }
            })
        {
            return Err(refused());
        }
        let bytes = STANDARD
            .decode(&n.artifact_bytes_base64)
            .map_err(|_| refused())?;
        if STANDARD.encode(&bytes) != n.artifact_bytes_base64
            || bytes_digest(&bytes) != n.artifact_sha256
            || timestamp(&n.issue_at).is_err()
            || timestamp(&n.appeal_deadline).is_err()
            || timestamp(&n.effect_eligible_at).is_err()
            || n.variants
                .iter()
                .any(|v| bytes_digest(v.body.as_bytes()) != v.body_sha256)
        {
            return Err(refused());
        }
        let artifact: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| refused())?;
        let issue = state
            .operations
            .iter()
            .find(|op| op.record_id == n.id)
            .ok_or_else(refused)?;
        if n.issue_at != issue.performed_at
            || artifact
                .pointer("/machineReadable/appealDeadline")
                .and_then(serde_json::Value::as_str)
                != Some(n.appeal_deadline.as_str())
            || artifact
                .pointer("/machineReadable/effectEligibleAt")
                .and_then(serde_json::Value::as_str)
                != Some(n.effect_eligible_at.as_str())
        {
            return Err(refused());
        }
        for acknowledgement in &n.acknowledgements {
            if !n.transport_results.iter().any(|result| match result {
                TransportResult::AcceptedOnly {
                    acknowledgement: actual,
                }
                | TransportResult::Qualified {
                    acknowledgement: actual,
                    ..
                } => actual == acknowledgement,
                TransportResult::Failed { .. } => false,
            }) {
                return Err(refused());
            }
        }
        let work = super::transport::work(record, n);
        if let Some(receipt) = &n.receipt {
            if !n.transport_results.iter().any(|result| matches!(result, TransportResult::Qualified { acknowledgement, receipt: actual } if actual == receipt && n.acknowledgements.contains(acknowledgement))) { return Err(refused()); }
            let ack = n
                .acknowledgements
                .iter()
                .find(|a| a.adapter_result_id == receipt.adapter_result_id)
                .ok_or_else(refused)?;
            if n.status != NoticeStatus::Qualified
                || !super::transport::acknowledgement_matches(&work, ack)
                || !super::transport::receipt_matches(&work, ack, receipt)
            {
                return Err(refused());
            }
            let artifact: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| refused())?;
            let determination = state
                .determinations
                .iter()
                .find(|d| d.id == n.determination_id)
                .ok_or_else(refused)?;
            let expected_data = serde_json::json!({"source":"deterministic","noticeId":n.id,
                "determinationId":determination.id,"snapshotSha256":determination.snapshot.sha256,
                "machineReadable":artifact["machineReadable"],"acknowledgement":ack,"receipt":receipt});
            let sent = record
                .provenance_log
                .iter()
                .filter(|p| {
                    p.record_kind == ProvenanceKind::NoticeSent
                        && p.data
                            .as_ref()
                            .and_then(|d| d.get("noticeId"))
                            .and_then(serde_json::Value::as_str)
                            == Some(n.id.as_str())
                })
                .count();
            if sent != 1
                || !record.provenance_log.iter().any(|p| {
                    p.record_kind == ProvenanceKind::NoticeSent
                        && p.data.as_ref() == Some(&expected_data)
                        && p.timestamp == receipt.qualified_at
                        && p.actor_id.as_ref()
                            == determination
                                .signature
                                .as_ref()
                                .map(|s| &s.supervisor_human)
                })
            {
                return Err(refused());
            }
        } else if n.status == NoticeStatus::Qualified {
            return Err(refused());
        }
    }
    for sent in record
        .provenance_log
        .iter()
        .filter(|p| p.record_kind == ProvenanceKind::NoticeSent)
    {
        if let Some(id) = sent
            .data
            .as_ref()
            .and_then(|d| d.get("noticeId"))
            .and_then(serde_json::Value::as_str)
        {
            if !state
                .notices
                .iter()
                .any(|n| n.id == id && n.receipt.is_some() && n.status == NoticeStatus::Qualified)
            {
                return Err(refused());
            }
        }
    }
    let mut appeals = HashSet::new();
    for a in &state.appeals {
        if !appeals.insert(&a.id)
            || !state.notices.iter().any(|n| {
                n.id == a.notice_id
                    && n.determination_id == a.determination_id
                    && n.receipt.is_some()
            })
            || exactly(record, &a.id, ProvenanceKind::AppealFiled)?
                .actor_id
                .as_ref()
                != Some(&a.filed_by)
        {
            return Err(refused());
        }
        let filed = exactly(record, &a.id, ProvenanceKind::AppealFiled)?;
        let filing_id = filed
            .data
            .as_ref()
            .and_then(|data| data.get("operationRecordId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(refused)?;
        let filing = state
            .operations
            .iter()
            .find(|op| op.record_id == filing_id)
            .ok_or_else(refused)?;
        let filing_request = public_request(&state, filing)?;
        if filing.kind != OperationKind::FileAppeal
            || filing.outcome
                != (GovernanceOutcome::AppealFiled {
                    appeal_id: a.id.clone(),
                })
            || filing.determination_id.as_ref() != Some(&a.determination_id)
            || filing.performed_at != a.filed_at
            || filed.timestamp != a.filed_at
            || filing.actor.acting_actor != a.filed_by
            || filing_request.command
                != (GovernanceCommand::FileAppeal {
                    channel: a.channel.clone(),
                    continuation_choice: a.continuation_choice.clone(),
                })
            || filed.data.as_ref()
                != Some(
                    &serde_json::json!({"determinationId":a.determination_id,"noticeId":a.notice_id,"operationRecordId":filing_id,"channel":a.channel,"continuationChoice":a.continuation_choice}),
                )
        {
            return Err(refused());
        }
        if let Some(schedule) = &a.schedule {
            let (op, request) = appeal_operation(
                &state,
                a,
                &schedule.operation_record_id,
                OperationKind::ScheduleHearing,
            )?;
            let assigned = record.process.actor_bindings.as_ref().and_then(|bindings| {
                bindings
                    .iter()
                    .find(|binding| binding.declared_actor == schedule.reviewer_actor)
            });
            if request.command
                != (GovernanceCommand::ScheduleHearing {
                    appeal_id: a.id.clone(),
                    reviewer_actor: schedule.reviewer_actor.clone(),
                    scheduled_for: schedule.scheduled_for.clone(),
                })
                || op.outcome
                    != (GovernanceOutcome::HearingScheduled {
                        appeal_id: a.id.clone(),
                        scheduled_for: schedule.scheduled_for.clone(),
                    })
                || schedule.scheduled_at != op.performed_at
                || assigned.is_none_or(|binding| binding.assigned_actor != schedule.reviewer_human)
            {
                return Err(refused());
            }
        }
        if let Some(hearing) = &a.hearing {
            let (op, request) = appeal_operation(
                &state,
                a,
                &hearing.operation_record_id,
                OperationKind::HoldHearing,
            )?;
            if request.command
                != (GovernanceCommand::HoldHearing {
                    appeal_id: a.id.clone(),
                    evidence_refs: hearing.evidence_refs.clone(),
                })
                || op.outcome
                    != (GovernanceOutcome::HearingHeld {
                        appeal_id: a.id.clone(),
                        held_at: hearing.held_at.clone(),
                    })
                || hearing.held_at != op.performed_at
                || hearing.reviewer_human != op.actor.acting_actor
                || a.schedule.as_ref().is_none_or(|schedule| {
                    schedule.reviewer_human != hearing.reviewer_human
                        || schedule.reviewer_actor != op.actor.declared_actor
                })
            {
                return Err(refused());
            }
        }
        if let Some(disposition) = &a.disposition {
            let (op, request) = appeal_operation(
                &state,
                a,
                &disposition.operation_record_id,
                OperationKind::RecordDisposition,
            )?;
            if request.command
                != (GovernanceCommand::RecordDisposition {
                    appeal_id: a.id.clone(),
                    outcome: disposition.outcome.clone(),
                    rationale: disposition.rationale.clone(),
                    evidence_refs: disposition.evidence_refs.clone(),
                })
                || op.outcome
                    != (GovernanceOutcome::DispositionRecorded {
                        appeal_id: a.id.clone(),
                        outcome: disposition.outcome.clone(),
                    })
                || disposition.recorded_at != op.performed_at
                || a.hearing
                    .as_ref()
                    .is_none_or(|hearing| hearing.reviewer_human != op.actor.acting_actor)
            {
                return Err(refused());
            }
        }
        if let Some(termination) = &a.termination {
            let (op, request) = appeal_operation(
                &state,
                a,
                &termination.operation_record_id,
                OperationKind::RequestServiceTermination,
            )?;
            if request.command
                != (GovernanceCommand::RequestServiceTermination {
                    appeal_id: a.id.clone(),
                    rationale: termination.rationale.clone(),
                })
                || op.outcome
                    != (GovernanceOutcome::ServiceTerminationRequested {
                        appeal_id: a.id.clone(),
                    })
                || termination.requested_at != op.performed_at
                || termination.requested_by != a.filed_by
                || termination.requested_by != op.actor.acting_actor
            {
                return Err(refused());
            }
        }
        let expected_continuation = if a.disposition.is_some() || a.termination.is_some() {
            ContinuationStatus::Released
        } else if a.continuation_choice == ContinuationChoice::Waive {
            ContinuationStatus::Waived
        } else {
            ContinuationStatus::Frozen
        };
        if a.continuation != expected_continuation {
            return Err(refused());
        }
        for op_id in a
            .schedule
            .iter()
            .map(|s| &s.operation_record_id)
            .chain(a.hearing.iter().map(|h| &h.operation_record_id))
            .chain(a.disposition.iter().map(|d| &d.operation_record_id))
            .chain(a.termination.iter().map(|t| &t.operation_record_id))
        {
            if !operation_ids.contains(op_id) {
                return Err(refused());
            }
        }
        if a.hearing.is_some() && a.schedule.is_none()
            || a.disposition.is_some() && a.hearing.is_none()
        {
            return Err(refused());
        }
    }
    if record
        .provenance_log
        .iter()
        .any(|p| p.record_kind == ProvenanceKind::AppealFiled && !appeals.contains(&p.id))
    {
        return Err(refused());
    }
    for e in &state.extensions {
        if !operation_ids.contains(&e.operation_record_id)
            || !record.provenance_log.iter().any(|p| {
                p.record_kind == ProvenanceKind::OverrideRecorded
                    && p.data.as_ref() == serde_json::to_value(&e.override_record).ok().as_ref()
            })
        {
            return Err(refused());
        }
    }
    let mut tokens = HashSet::new();
    for replay in &state.replays {
        if !tokens.insert(&replay.request.idempotency_token) {
            return Err(refused());
        }
        let GovernanceResult::Committed {
            operation_record_id,
            outcome,
        } = &replay.result
        else {
            return Err(refused());
        };
        if !state.operations.iter().any(|op| {
            &op.record_id == operation_record_id
                && &op.outcome == outcome
                && op.operation_ref == replay.request.operation_ref
                && op.actor == replay.request.actor
                && op.determination_id == replay.request.determination_id
                && op.kind == replay.request.command.operation_kind()
        }) {
            return Err(refused());
        }
    }
    Ok(())
}

pub(crate) fn validate_update(
    current: &RuntimeRecord,
    previous: &RuntimeRecord,
) -> Result<(), StoreError> {
    validate(current)?;
    validate(previous)?;
    let old = state(previous);
    let new = state(current);
    if !new.operations.starts_with(&old.operations)
        || !new.extensions.starts_with(&old.extensions)
        || !new.replays.starts_with(&old.replays)
    {
        return Err(refused());
    }
    for d in old.determinations {
        let next = new
            .determinations
            .iter()
            .find(|n| n.id == d.id)
            .ok_or_else(refused)?;
        let mut same = next.clone();
        same.signature = d.signature.clone();
        same.effect = d.effect.clone();
        if same != d
            || d.signature.is_some() && next.signature != d.signature
            || d.effect.is_some() && next.effect != d.effect
        {
            return Err(refused());
        }
    }
    for n in old.notices {
        let next = new
            .notices
            .iter()
            .find(|v| v.id == n.id)
            .ok_or_else(refused)?;
        let mut same = next.clone();
        same.status = n.status.clone();
        same.attempts = n.attempts;
        same.acknowledgements = n.acknowledgements.clone();
        same.transport_results = n.transport_results.clone();
        same.receipt = n.receipt.clone();
        same.failure = n.failure.clone();
        if same != n
            || next.attempts < n.attempts
            || !next.acknowledgements.starts_with(&n.acknowledgements)
            || !next.transport_results.starts_with(&n.transport_results)
            || n.receipt.is_some() && next.receipt != n.receipt
            || n.status == NoticeStatus::Qualified && next.status != n.status
        {
            return Err(refused());
        }
    }
    for a in old.appeals {
        let next = new
            .appeals
            .iter()
            .find(|n| n.id == a.id)
            .ok_or_else(refused)?;
        let mut same = next.clone();
        same.schedule = a.schedule.clone();
        same.hearing = a.hearing.clone();
        same.disposition = a.disposition.clone();
        same.termination = a.termination.clone();
        same.continuation = a.continuation.clone();
        if same != a
            || a.schedule.is_some() && next.schedule != a.schedule
            || a.hearing.is_some() && next.hearing != a.hearing
            || a.disposition.is_some() && next.disposition != a.disposition
            || a.termination.is_some() && next.termination != a.termination
        {
            return Err(refused());
        }
    }
    if previous
        .creation
        .as_ref()
        .is_some_and(|c| c.reference_due_process.is_some())
    {
        // Native facts, snapshot bytes, task content, and original provenance stay immutable.
        if current.provenance_log.len() < previous.provenance_log.len() {
            return Err(refused());
        }
        for (original, next) in previous.provenance_log.iter().zip(&current.provenance_log) {
            // Custody may add its separately validated stamp; all original
            // provenance fields and their order remain immutable.
            if original.canonical_event_hash.is_some()
                && next.canonical_event_hash != original.canonical_event_hash
            {
                return Err(refused());
            }
            let mut without_new_custody = next.clone();
            without_new_custody.canonical_event_hash = original.canonical_event_hash.clone();
            if serde_json::to_value(&without_new_custody).map_err(|_| refused())?
                != serde_json::to_value(original).map_err(|_| refused())?
            {
                return Err(refused());
            }
        }
        for (id, artifact) in &previous.artifacts {
            let next = current.artifacts.get(id).ok_or_else(refused)?;
            if next.artifact_id != artifact.artifact_id
                || next.task_id != artifact.task_id
                || next.kind != artifact.kind
                || next.response != artifact.response
                || next.actor_id != artifact.actor_id
                || next.recorded_at != artifact.recorded_at
            {
                return Err(refused());
            }
        }
    }
    Ok(())
}
