// Rust guideline compliant 2026-02-21
//! Applies admitted commands to a private prospective due-process state.
use super::admission::*;
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use wos_core::model::due_process::*;
use wos_core::{ProvenanceKind, ProvenanceRecord};

// Literal template facts share the existing rendering context. Their only sources
// are this determination's successful typed records and immutable snapshot.
fn notice_facts(
    variables: &mut serde_json::Map<String, Value>,
    records: &[ProvenanceRecord],
    determination: &DeterminationState,
    snapshot: &Value,
) -> Result<(), RefusalCode> {
    let associated = || {
        records.iter().filter(|record| {
            record
                .data
                .as_ref()
                .and_then(|data| data.get("relatedTransition"))
                .and_then(Value::as_str)
                == Some(determination.id.as_str())
        })
    };
    let mut reasons =
        associated().filter(|record| record.record_kind == ProvenanceKind::DeterminationReasoning);
    let reason = reasons.next().ok_or(RefusalCode::ExplanationRejected)?;
    if reasons.next().is_some() {
        return Err(RefusalCode::ExplanationRejected);
    }
    let reason: DeterminationReasoningData = serde_json::from_value(
        reason
            .data
            .clone()
            .ok_or(RefusalCode::ExplanationRejected)?,
    )
    .map_err(|_| RefusalCode::ExplanationRejected)?;
    if reason.evaluation.location != determination.firing.location {
        return Err(RefusalCode::ExplanationRejected);
    }
    let mut insert = |name, value| {
        if variables.insert(name, value).is_some() {
            Err(RefusalCode::ExplanationRejected)
        } else {
            Ok(())
        }
    };
    insert("reasonCode".into(), Value::String(reason.rule_id))?;
    for input in &reason.evaluation.inputs {
        insert(format!("inputs.{}", input.name), input.value.value())?;
    }
    for record in associated()
        .filter(|record| record.record_kind == ProvenanceKind::DeterminationCounterfactual)
    {
        let counter: DeterminationCounterfactualData = serde_json::from_value(
            record
                .data
                .clone()
                .ok_or(RefusalCode::ExplanationRejected)?,
        )
        .map_err(|_| RefusalCode::ExplanationRejected)?;
        if counter.original_snapshot_sha256 != determination.snapshot.sha256
            || counter.original_evaluation != reason.evaluation
        {
            return Err(RefusalCode::ExplanationRejected);
        }
        for (index, change) in counter.alternative.changes.iter().enumerate() {
            let original = snapshot
                .pointer(&change.case_pointer)
                .and_then(Scalar::from_value)
                .ok_or(RefusalCode::ExplanationRejected)?;
            let prefix = format!("counterfactuals.{}.changes.{index}", counter.alternative.id);
            insert(format!("{prefix}.oldValue"), original.value())?;
            insert(format!("{prefix}.newValue"), change.value.value())?;
        }
    }
    Ok(())
}

pub(super) fn state(record: &RuntimeRecord) -> DueProcessState {
    record
        .process
        .governance_state
        .as_ref()
        .and_then(|g| g.due_process.clone())
        .unwrap_or_default()
}
pub(super) fn retain(record: &mut RuntimeRecord, state: DueProcessState) {
    record
        .process
        .governance_state
        .get_or_insert_with(Default::default)
        .due_process = Some(state);
}
fn pending<'a>(
    state: &'a mut DueProcessState,
    request: &GovernanceRequest,
) -> Result<&'a mut AppealState, RefusalCode> {
    let appeal_id = match &request.command {
        GovernanceCommand::ScheduleHearing { appeal_id, .. }
        | GovernanceCommand::HoldHearing { appeal_id, .. }
        | GovernanceCommand::RecordDisposition { appeal_id, .. }
        | GovernanceCommand::RequestServiceTermination { appeal_id, .. } => appeal_id,
        _ => return Err(RefusalCode::TargetRejected),
    };
    state
        .appeals
        .iter_mut()
        .find(|a| {
            &a.id == appeal_id && Some(&a.determination_id) == request.determination_id.as_ref()
        })
        .filter(|a| a.disposition.is_none())
        .ok_or(RefusalCode::OperationStateRejected)
}
fn independent(human: &str, determination: &DeterminationState) -> Result<(), RefusalCode> {
    if !human.starts_with("actor:human:")
        || human == determination.determining_human
        || determination
            .signature
            .as_ref()
            .is_none_or(|s| s.supervisor_human == human)
    {
        return Err(RefusalCode::ActorRejected);
    }
    Ok(())
}
pub(super) fn apply(
    runtime: &super::super::WosRuntime,
    cap: &ReferenceDueProcessCapabilities,
    kernel: &KernelDocument,
    raw: &Value,
    policy: &DueProcessExecution,
    record: &mut RuntimeRecord,
    state: &mut DueProcessState,
    operation: &OperationBinding,
    request: &GovernanceRequest,
    firing: &FiringRef,
    prospective: &[ProvenanceRecord],
    now: &str,
) -> Result<(GovernanceOutcome, Vec<ProvenanceRecord>), RefusalCode> {
    let minter =
        crate::runtime::process_minter(&record.process).map_err(|_| RefusalCode::StoreRejected)?;
    if state.operations.len() >= policy.limits.max_operations {
        return Err(RefusalCode::LimitReached);
    }
    let mut extra = Vec::new();
    let target = if operation.kind != OperationKind::ExtendPreNoticeWindow {
        let id = request
            .determination_id
            .as_ref()
            .ok_or(RefusalCode::TargetRejected)?;
        let matches: Vec<_> = state
            .determinations
            .iter()
            .filter(|d| &d.id == id && d.binding_ref == operation.determination_ref)
            .collect();
        if matches.len() != 1 {
            return Err(RefusalCode::TargetRejected);
        }
        Some(matches[0].clone())
    } else {
        if request.determination_id.is_some() {
            return Err(RefusalCode::TargetRejected);
        }
        None
    };
    let adverse = raw
        .pointer("/governance/dueProcess/adverseDecisionPolicy")
        .ok_or(RefusalCode::InvalidConfiguration)?;
    let actor = &request.actor.acting_actor;
    let result = match &request.command {
        GovernanceCommand::IssueNotice => {
            let mut determination = target.clone().ok_or(RefusalCode::TargetRejected)?;
            if state
                .notices
                .iter()
                .any(|n| n.determination_id == determination.id)
            {
                return Err(RefusalCode::OperationStateRejected);
            }
            if state.notices.len() >= policy.limits.max_notices
                || state.notices.iter().filter(|n| n.receipt.is_none()).count()
                    >= policy.limits.max_outstanding_notice_work
            {
                return Err(RefusalCode::LimitReached);
            }
            let signature =
                cap.signature_verifier
                    .verify(record, kernel, &determination, timestamp(now)?)?;
            if !signature.supervisor_human.starts_with("actor:human:") {
                return Err(RefusalCode::SignatureRejected);
            }
            if let Some(original) = &determination.signature {
                if original != &signature {
                    return Err(RefusalCode::SignatureRejected);
                }
            }
            determination.signature = Some(signature);
            let all: Vec<_> = record
                .provenance_log
                .iter()
                .chain(prospective)
                .chain(extra.iter())
                .cloned()
                .collect();
            let explanation = wos_core::explain::assemble_explanation(
                &all,
                &determination.id,
                &["adverse-decision".into()],
                now,
            );
            if explanation.reasoning.is_empty()
                || explanation.positive_counterfactual.is_empty()
                || explanation.negative_counterfactual.is_empty()
            {
                return Err(RefusalCode::ExplanationRejected);
            }
            let appeal_deadline = add_days(
                now,
                duration(
                    adverse
                        .pointer("/appealMechanism/appealWindow")
                        .and_then(Value::as_str),
                )?,
            )?;
            let effect_eligible_at = add_days(
                now,
                duration(adverse.get("noticeGracePeriod").and_then(Value::as_str))?,
            )?;
            let snapshot = record
                .provenance_log
                .iter()
                .chain(prospective)
                .find(|p| p.id == determination.snapshot.record_id)
                .and_then(|p| p.case_file_snapshot.as_ref())
                .ok_or(RefusalCode::TargetRejected)?;
            let recipient = snapshot
                .value
                .pointer(&policy.notice.recipient_pointer)
                .and_then(Value::as_str)
                .filter(|s| text(s, 1024))
                .ok_or(RefusalCode::InvalidConfiguration)?
                .to_string();
            let machine = serde_json::json!({"kind":"adverseDecisionNotice","determinationId":determination.id,
                "firing":determination.firing,"snapshotSha256":determination.snapshot.sha256,
                "explanation":explanation,"sourceRefs":policy.notice.source_refs,"templates":policy.notice.templates,
                "recipient":recipient,"appealDeadline":appeal_deadline,"effectEligibleAt":effect_eligible_at,
                "appealChannels":policy.appeal.channels,"continuation":policy.appeal.continuation});
            let mut variables = machine
                .as_object()
                .cloned()
                .ok_or(RefusalCode::InvalidConfiguration)?;
            variables.insert("noticeIssuedAt".into(), Value::String(now.into()));
            notice_facts(&mut variables, &all, &determination, &snapshot.value)?;
            let mut variants = Vec::new();
            for selection in &policy.notice.templates {
                let template = cap
                    .templates
                    .iter()
                    .flat_map(|d| d.templates.iter())
                    .find(|(key, t)| {
                        *key == &selection.template_key
                            && t.locale_ref.as_deref() == Some(selection.locale.as_str())
                    })
                    .map(|(_, v)| v)
                    .ok_or(RefusalCode::InvalidConfiguration)?;
                if template
                    .required_variables
                    .iter()
                    .any(|v| !variables.contains_key(v))
                {
                    return Err(RefusalCode::UnsupportedProfile);
                }
                let render = |value: &str| -> Result<String, RefusalCode> {
                    let mut rendered = String::new();
                    let mut remaining = value;
                    while let Some((literal, token)) = remaining.split_once("{{") {
                        if literal.contains("}}") {
                            return Err(RefusalCode::UnsupportedProfile);
                        }
                        rendered.push_str(literal);
                        let (name, rest) = token
                            .split_once("}}")
                            .ok_or(RefusalCode::UnsupportedProfile)?;
                        let value = variables.get(name).ok_or(RefusalCode::UnsupportedProfile)?;
                        // Bound facts are literal text, never another template.
                        if let Some(value) = value.as_str() {
                            rendered.push_str(value);
                        } else {
                            rendered.push_str(&value.to_string());
                        }
                        remaining = rest;
                    }
                    if remaining.contains("}}") {
                        return Err(RefusalCode::UnsupportedProfile);
                    }
                    rendered.push_str(remaining);
                    Ok(rendered)
                };
                let mut body = String::new();
                if let Some(subject) = &template.subject {
                    body.push_str(&render(subject)?);
                    body.push('\n');
                }
                for section in &template.sections {
                    if let Some(title) = &section.title {
                        body.push_str(&render(title)?);
                        body.push('\n');
                    }
                    if let Some(content) = &section.content {
                        body.push_str(&render(content)?);
                        body.push('\n');
                    } else if section.required {
                        return Err(RefusalCode::UnsupportedProfile);
                    }
                }
                variants.push(NoticeVariant {
                    locale: selection.locale.clone(),
                    template_key: selection.template_key.clone(),
                    review: selection.review.clone(),
                    body_sha256: bytes_digest(body.as_bytes()),
                    body,
                });
            }
            let artifact = serde_json::json!({"machineReadable":machine,"variants":variants});
            let bytes = wos_events::provenance::CaseFileSnapshot::from_case_state(&artifact)
                .jcs_canonical
                .into_bytes();
            if bytes.len() > policy.limits.max_notice_bytes {
                return Err(RefusalCode::LimitReached);
            }
            let artifact_sha256 = bytes_digest(&bytes);
            let work_digest = digest(
                &serde_json::json!({"process":record.process.process_id,"notice":firing.provenance_record_id,"artifact":artifact_sha256}),
            );
            let key = format!("notice.{}", work_digest.trim_start_matches("sha256:"));
            state.notices.push(NoticeState {
                id: firing.provenance_record_id.clone(),
                determination_id: determination.id.clone(),
                artifact_sha256,
                artifact_bytes_base64: STANDARD.encode(&bytes),
                issue_at: now.into(),
                appeal_deadline,
                effect_eligible_at,
                recipient,
                channel: policy.notice.delivery_channel.clone(),
                variants,
                transport_key: key.clone(),
                correlation: key,
                status: NoticeStatus::Pending,
                attempts: 0,
                acknowledgements: Vec::new(),
                transport_results: Vec::new(),
                receipt: None,
                failure: None,
            });
            let original = state
                .determinations
                .iter_mut()
                .find(|d| d.id == determination.id)
                .ok_or(RefusalCode::TargetRejected)?;
            original.signature = determination.signature;
            GovernanceOutcome::NoticeStaged {
                notice_id: firing.provenance_record_id.clone(),
            }
        }
        GovernanceCommand::FileAppeal {
            channel,
            continuation_choice,
        } => {
            let determination = target.as_ref().ok_or(RefusalCode::TargetRejected)?;
            if !policy
                .appeal
                .standing_actors
                .contains(&request.actor.declared_actor)
                || !actor.starts_with("actor:human:")
            {
                return Err(RefusalCode::ActorRejected);
            }
            if !policy.appeal.channels.contains(channel) {
                return Err(RefusalCode::InvalidConfiguration);
            }
            if state
                .appeals
                .iter()
                .any(|a| a.determination_id == determination.id)
            {
                return Err(RefusalCode::OperationStateRejected);
            }
            if state.appeals.len() >= policy.limits.max_appeals {
                return Err(RefusalCode::LimitReached);
            }
            let notice = state
                .notices
                .iter()
                .find(|n| {
                    n.determination_id == determination.id && n.status == NoticeStatus::Qualified
                })
                .ok_or(RefusalCode::NoticePending)?;
            if timestamp(now)? > timestamp(&notice.appeal_deadline)? {
                return Err(RefusalCode::ClockRejected);
            }
            let continuation = match (&policy.appeal.continuation.choice, continuation_choice) {
                (ContinuationChoicePolicy::ElectionRequired, ContinuationChoice::Unspecified) => {
                    return Err(RefusalCode::OperationStateRejected);
                }
                (_, ContinuationChoice::Waive) => ContinuationStatus::Waived,
                _ => ContinuationStatus::Frozen,
            };
            let mut filed = ProvenanceRecord::blank(&minter, ProvenanceKind::AppealFiled);
            filed.actor_id = Some(actor.clone());
            filed.timestamp = now.into();
            filed.data = Some(
                serde_json::json!({"determinationId":determination.id,"noticeId":notice.id,"operationRecordId":firing.provenance_record_id,"channel":channel,"continuationChoice":continuation_choice}),
            );
            let appeal_id = filed.id.clone();
            state.appeals.push(AppealState {
                id: appeal_id.clone(),
                determination_id: determination.id.clone(),
                notice_id: notice.id.clone(),
                filed_at: now.into(),
                filed_by: actor.clone(),
                channel: channel.clone(),
                continuation_choice: continuation_choice.clone(),
                continuation,
                hearing_deadline: add_days(now, policy.appeal.hearing_deadline.calendar_days)?,
                schedule: None,
                hearing: None,
                disposition: None,
                termination: None,
            });
            extra.push(filed);
            GovernanceOutcome::AppealFiled { appeal_id }
        }
        GovernanceCommand::ScheduleHearing {
            reviewer_actor,
            scheduled_for,
            ..
        } => {
            let determination = target.as_ref().ok_or(RefusalCode::TargetRejected)?;
            let reviewer = canonical_actor(kernel, &record.process, reviewer_actor)?;
            if !policy.appeal.reviewer_actors.contains(reviewer_actor) {
                return Err(RefusalCode::ActorRejected);
            }
            independent(&reviewer, determination)?;
            let appeal = pending(state, request)?;
            if appeal.schedule.is_some() || appeal.hearing.is_some() {
                return Err(RefusalCode::OperationStateRejected);
            }
            if timestamp(scheduled_for)? < timestamp(now)?
                || timestamp(scheduled_for)? > timestamp(&appeal.hearing_deadline)?
                || timestamp(now)? > timestamp(&appeal.hearing_deadline)?
            {
                return Err(RefusalCode::ClockRejected);
            }
            appeal.schedule = Some(HearingSchedule {
                operation_record_id: firing.provenance_record_id.clone(),
                scheduled_at: now.into(),
                scheduled_for: scheduled_for.clone(),
                reviewer_actor: reviewer_actor.clone(),
                reviewer_human: reviewer,
            });
            GovernanceOutcome::HearingScheduled {
                appeal_id: appeal.id.clone(),
                scheduled_for: scheduled_for.clone(),
            }
        }
        GovernanceCommand::HoldHearing { evidence_refs, .. } => {
            let determination = target.as_ref().ok_or(RefusalCode::TargetRejected)?;
            independent(actor, determination)?;
            sources(cap, evidence_refs, EvidenceRole::HearingEvidence)?;
            let appeal = pending(state, request)?;
            let schedule = appeal
                .schedule
                .as_ref()
                .ok_or(RefusalCode::OperationStateRejected)?;
            if schedule.reviewer_human != *actor
                || schedule.reviewer_actor != request.actor.declared_actor
            {
                return Err(RefusalCode::ActorRejected);
            }
            if appeal.hearing.is_some() {
                return Err(RefusalCode::OperationStateRejected);
            }
            if timestamp(now)? < timestamp(&schedule.scheduled_for)?
                || timestamp(now)? > timestamp(&appeal.hearing_deadline)?
            {
                return Err(RefusalCode::ClockRejected);
            }
            appeal.hearing = Some(HeldHearing {
                operation_record_id: firing.provenance_record_id.clone(),
                held_at: now.into(),
                reviewer_human: actor.clone(),
                evidence_refs: evidence_refs.clone(),
            });
            GovernanceOutcome::HearingHeld {
                appeal_id: appeal.id.clone(),
                held_at: now.into(),
            }
        }
        GovernanceCommand::RecordDisposition {
            outcome,
            rationale,
            evidence_refs,
            ..
        } => {
            let determination = target.as_ref().ok_or(RefusalCode::TargetRejected)?;
            independent(actor, determination)?;
            sources(cap, evidence_refs, EvidenceRole::HearingEvidence)?;
            if !text(rationale, 4096) {
                return Err(RefusalCode::InvalidConfiguration);
            }
            let appeal = pending(state, request)?;
            let hearing = appeal
                .hearing
                .as_ref()
                .ok_or(RefusalCode::OperationStateRejected)?;
            if hearing.reviewer_human != *actor
                || appeal
                    .schedule
                    .as_ref()
                    .is_none_or(|s| s.reviewer_actor != request.actor.declared_actor)
            {
                return Err(RefusalCode::ActorRejected);
            }
            appeal.disposition = Some(Disposition {
                operation_record_id: firing.provenance_record_id.clone(),
                recorded_at: now.into(),
                outcome: outcome.clone(),
                rationale: rationale.clone(),
                evidence_refs: evidence_refs.clone(),
            });
            appeal.continuation = ContinuationStatus::Released;
            GovernanceOutcome::DispositionRecorded {
                appeal_id: appeal.id.clone(),
                outcome: outcome.clone(),
            }
        }
        GovernanceCommand::RequestServiceTermination { rationale, .. } => {
            if !text(rationale, 4096) {
                return Err(RefusalCode::InvalidConfiguration);
            }
            let appeal = pending(state, request)?;
            if appeal.filed_by != *actor
                || !policy
                    .appeal
                    .standing_actors
                    .contains(&request.actor.declared_actor)
            {
                return Err(RefusalCode::ActorRejected);
            }
            if appeal.termination.is_some() {
                return Err(RefusalCode::OperationStateRejected);
            }
            appeal.termination = Some(HouseholdTermination {
                operation_record_id: firing.provenance_record_id.clone(),
                requested_at: now.into(),
                requested_by: actor.clone(),
                rationale: rationale.clone(),
            });
            appeal.continuation = ContinuationStatus::Released;
            GovernanceOutcome::ServiceTerminationRequested {
                appeal_id: appeal.id.clone(),
            }
        }
        GovernanceCommand::ExtendPreNoticeWindow {
            calendar_days,
            rationale,
            evidence_refs,
        } => {
            let extension = policy
                .pre_notice_extension
                .as_ref()
                .ok_or(RefusalCode::OverrideRejected)?;
            if !state.determinations.is_empty()
                || !member(
                    raw,
                    &record.process.configuration,
                    &extension.allowed_state_pointers,
                )?
                || !text(rationale, 4096)
                || !(1..=36500).contains(calendar_days)
            {
                return Err(RefusalCode::OverrideRejected);
            }
            sources(cap, evidence_refs, EvidenceRole::OverrideEvidence)
                .map_err(|_| RefusalCode::OverrideRejected)?;
            let authority = raw
                .pointer("/governance/qualityControls/overrideAuthority")
                .ok_or(RefusalCode::OverrideRejected)?;
            if [
                "requireStructuredRationale",
                "requireAuthorityVerification",
                "requireSupportingEvidence",
            ]
            .iter()
            .any(|k| authority.get(k) != Some(&Value::Bool(true)))
            {
                return Err(RefusalCode::OverrideRejected);
            }
            let prior = state
                .extensions
                .iter()
                .filter(|e| e.timer_id == extension.timer_id)
                .map(|e| e.calendar_days)
                .sum::<u32>();
            let cumulative = prior
                .checked_add(*calendar_days)
                .filter(|v| *v <= extension.max_cumulative_calendar_days)
                .ok_or(RefusalCode::OverrideRejected)?;
            let timers: Vec<_> = record
                .process
                .timers
                .iter_mut()
                .filter(|t| t.timer_id == extension.timer_id)
                .collect();
            if timers.len() != 1 {
                return Err(RefusalCode::OverrideRejected);
            }
            let timer = timers
                .into_iter()
                .next()
                .ok_or(RefusalCode::OverrideRejected)?;
            let prior_deadline = timer.deadline.clone();
            let new_deadline = add_days(&prior_deadline, *calendar_days)?;
            let mut override_prov =
                ProvenanceRecord::blank(&minter, ProvenanceKind::OverrideRecorded);
            override_prov.actor_id = Some(actor.clone());
            override_prov.timestamp = now.into();
            let override_record = OverrideRecord {
                id: minter.mint_governance_id(),
                case_id: record.process.case_ledger_id.clone(),
                rationale: OverrideRationale {
                    summary: rationale.clone(),
                },
                authority_verification: OverrideAuthorityVerification {
                    actor_id: actor.clone(),
                    method: OverrideAuthorityMethod::RoleAssignment,
                    verified_at: now.into(),
                    authority_source: Some(format!(
                        "{}#{}",
                        record.process.definition_url, request.actor.declared_actor
                    )),
                },
                supporting_evidence: evidence_refs
                    .iter()
                    .map(|e| OverrideEvidenceReference {
                        kind: OverrideEvidenceKind::Document,
                        uri: format!("urn:wos:retained:{}", e.retained_sha256),
                        summary: Some(e.section_anchor.clone()),
                    })
                    .collect(),
            };
            override_prov.data = Some(
                serde_json::to_value(&override_record)
                    .map_err(|_| RefusalCode::OverrideRejected)?,
            );
            let override_record_id = override_prov.id.clone();
            state.extensions.push(ExtensionState {
                operation_record_id: firing.provenance_record_id.clone(),
                determination_binding_ref: operation.determination_ref.clone(),
                timer_id: extension.timer_id.clone(),
                calendar_days: *calendar_days,
                cumulative_calendar_days: cumulative,
                prior_deadline,
                new_deadline: new_deadline.clone(),
                override_record,
            });
            timer.deadline = new_deadline.clone();
            timer.absolute_deadline = true;
            extra.push(override_prov);
            GovernanceOutcome::PreNoticeWindowExtended {
                override_record_id,
                new_deadline,
            }
        }
        GovernanceCommand::ApplyAdverseEffect => {
            let determination = target.as_ref().ok_or(RefusalCode::TargetRejected)?;
            let signature =
                cap.signature_verifier
                    .verify(record, kernel, determination, timestamp(now)?)?;
            if determination.signature.as_ref() != Some(&signature) {
                return Err(RefusalCode::SignatureRejected);
            }
            if determination.effect.is_some() {
                return Err(RefusalCode::OperationStateRejected);
            }
            let notice = state
                .notices
                .iter()
                .find(|n| {
                    n.determination_id == determination.id
                        && n.status == NoticeStatus::Qualified
                        && n.receipt.is_some()
                })
                .ok_or(RefusalCode::NoticePending)?;
            if timestamp(now)? < timestamp(&notice.effect_eligible_at)? {
                return Err(RefusalCode::ClockRejected);
            }
            if let Some(appeal) = state
                .appeals
                .iter()
                .find(|a| a.determination_id == determination.id)
            {
                if appeal.continuation == ContinuationStatus::Frozen {
                    return Err(RefusalCode::ContinuationFrozen);
                }
                if appeal
                    .disposition
                    .as_ref()
                    .is_some_and(|d| d.outcome == DispositionOutcome::Reversed)
                {
                    return Err(RefusalCode::OperationStateRejected);
                }
            } else if timestamp(now)? <= timestamp(&notice.appeal_deadline)? {
                return Err(RefusalCode::ContinuationFrozen);
            }
            state
                .determinations
                .iter_mut()
                .find(|d| d.id == determination.id)
                .ok_or(RefusalCode::TargetRejected)?
                .effect = Some(AdverseEffect {
                operation_record_id: firing.provenance_record_id.clone(),
                applied_at: now.into(),
            });
            GovernanceOutcome::AdverseEffectApplied {
                applied_at: now.into(),
            }
        }
    };
    let _ = runtime;
    Ok((result, extra))
}
