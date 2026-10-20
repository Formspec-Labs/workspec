// Rust guideline compliant 2026-02-21
//! Checks selected configuration and current canonical actor admission.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::collections::HashSet;
use wos_core::model::due_process::*;
use wos_core::model::governance_checks::{GovernanceChecks, GovernanceIssueKind, LifecycleIndex};
use wos_core::model::kernel::{ActorKind, KernelDocument, Transition};
use wos_events::provenance::CaseFileSnapshot;

pub(crate) fn digest(value: &Value) -> String {
    format!("sha256:{}", CaseFileSnapshot::from_case_state(value).sha256)
}
pub(super) fn bytes_digest(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(value))
}
pub(super) fn id(value: &str) -> bool {
    value.len() <= 128 && super::super::actor_bindings::valid_local_actor(value)
}
pub(super) fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
pub(super) fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max
}
pub(super) fn pointer(value: &str) -> bool {
    if !value.starts_with('/') {
        return false;
    }
    let mut bytes = value.bytes();
    while let Some(b) = bytes.next() {
        if b == b'~' && !matches!(bytes.next(), Some(b'0' | b'1')) {
            return false;
        }
    }
    true
}
pub(super) fn unique<T: PartialEq>(items: &[T]) -> bool {
    items
        .iter()
        .enumerate()
        .all(|(i, value)| !items[..i].contains(value))
}
pub(super) fn timestamp(value: &str) -> Result<u64, RefusalCode> {
    let parsed =
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| RefusalCode::ClockRejected)?;
    let ms = u64::try_from(parsed.timestamp_millis()).map_err(|_| RefusalCode::ClockRejected)?;
    if super::super::format_timestamp(ms).map_err(|_| RefusalCode::ClockRejected)? != value {
        return Err(RefusalCode::ClockRejected);
    }
    Ok(ms)
}
pub(super) fn add_days(value: &str, days: u32) -> Result<String, RefusalCode> {
    if !(1..=36500).contains(&days) {
        return Err(RefusalCode::InvalidConfiguration);
    }
    let ms = timestamp(value)?
        .checked_add(u64::from(days) * 86_400_000)
        .ok_or(RefusalCode::ClockRejected)?;
    super::super::format_timestamp(ms).map_err(|_| RefusalCode::ClockRejected)
}
pub(super) fn duration(value: Option<&str>) -> Result<u32, RefusalCode> {
    let days = value
        .and_then(|v| v.strip_prefix('P'))
        .and_then(|v| v.strip_suffix('D'))
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| (1..=36500).contains(v))
        .ok_or(RefusalCode::UnsupportedProfile)?;
    if value != Some(format!("P{days}D").as_str()) {
        return Err(RefusalCode::UnsupportedProfile);
    }
    Ok(days)
}
pub(super) fn selected(
    cap: &ReferenceDueProcessCapabilities,
    kernel: &KernelDocument,
) -> Result<(Value, DueProcessExecution), RefusalCode> {
    let raw: Value = serde_json::from_slice(&cap.workflow_bytes)
        .map_err(|_| RefusalCode::InvalidConfiguration)?;
    let parsed: KernelDocument =
        serde_json::from_value(raw.clone()).map_err(|_| RefusalCode::InvalidConfiguration)?;
    if serde_json::to_value(parsed).ok() != serde_json::to_value(kernel).ok() {
        return Err(RefusalCode::InvalidConfiguration);
    }
    let value = raw
        .pointer("/governance/dueProcess/execution")
        .ok_or(RefusalCode::UnsupportedProfile)?;
    let policy =
        serde_json::from_value(value.clone()).map_err(|_| RefusalCode::InvalidConfiguration)?;
    Ok((raw, policy))
}
pub(super) fn sources(
    cap: &ReferenceDueProcessCapabilities,
    refs: &[SourceRef],
    role: EvidenceRole,
) -> Result<(), RefusalCode> {
    if refs.is_empty() || !unique(refs) {
        return Err(RefusalCode::InvalidConfiguration);
    }
    for source in refs {
        if !id(&source.source_version)
            || !text(&source.section_anchor, 1024)
            || !cap.retained_evidence.iter().any(|s| {
                s.id == source.source_version
                    && s.role == role
                    && s.sha256 == source.retained_sha256
            })
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
    }
    Ok(())
}
pub(super) fn transition(
    raw: &Value,
    location: &TransitionLocation,
) -> Result<Transition, RefusalCode> {
    if !pointer(&location.state_pointer) {
        return Err(RefusalCode::InvalidConfiguration);
    }
    serde_json::from_value(
        raw.pointer(&format!(
            "{}/transitions/{}",
            location.state_pointer, location.transition_index
        ))
        .ok_or(RefusalCode::InvalidConfiguration)?
        .clone(),
    )
    .map_err(|_| RefusalCode::InvalidConfiguration)
}
pub(super) fn state_id(raw: &Value, p: &str) -> Result<String, RefusalCode> {
    if !pointer(p) {
        return Err(RefusalCode::InvalidConfiguration);
    }
    let state = raw
        .pointer(p)
        .filter(|v| v.is_object())
        .ok_or(RefusalCode::InvalidConfiguration)?;
    // State IDs are map keys, except ForEach bodies which have their own authored ID.
    state
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| p.rsplit('/').next())
        .filter(|v| !v.is_empty())
        .map(|v| v.replace("~1", "/").replace("~0", "~"))
        .ok_or(RefusalCode::InvalidConfiguration)
}
pub(super) fn member(
    raw: &Value,
    configuration: &[String],
    pointers: &[String],
) -> Result<bool, RefusalCode> {
    for p in pointers {
        if !configuration.contains(&state_id(raw, p)?) {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn canonical_actor(
    kernel: &KernelDocument,
    process: &wos_core::instance::WorkflowProcess,
    declared: &str,
) -> Result<String, RefusalCode> {
    if !id(declared) {
        return Err(RefusalCode::ActorRejected);
    }
    let actors: Vec<_> = kernel.actors.iter().filter(|a| a.id == declared).collect();
    if actors.len() != 1 {
        return Err(RefusalCode::ActorRejected);
    }
    if actors[0].kind == ActorKind::Human {
        if process.actor_bindings.is_none() {
            return Err(RefusalCode::ActorRejected);
        }
        super::super::actor_bindings::resolve_assignment(
            process.actor_bindings.as_deref(),
            kernel,
            Some(declared),
        )
        .map_err(|_| RefusalCode::ActorRejected)?
        .filter(|s| s.starts_with("actor:human:"))
        .ok_or(RefusalCode::ActorRejected)
    } else {
        Ok(declared.into())
    }
}
pub(super) fn actor(
    runtime: &super::super::WosRuntime,
    kernel: &KernelDocument,
    record: &RuntimeRecord,
    transition: &Transition,
    actors: &[String],
    requested: &GovernanceActor,
    caller: &str,
) -> Result<(), RefusalCode> {
    if caller != requested.acting_actor
        || !actors.contains(&requested.declared_actor)
        || transition
            .actor
            .as_ref()
            .is_some_and(|a| a != &requested.declared_actor)
        || canonical_actor(kernel, &record.process, &requested.declared_actor)?
            != requested.acting_actor
        || !runtime
            .access_control
            .can_transition(caller, &transition.event_dispatch_label())
    {
        return Err(RefusalCode::ActorRejected);
    }
    Ok(())
}
fn validate_audit(
    raw: &Value,
    lifecycle: &LifecycleIndex,
    policy: &DueProcessExecution,
    checks: &GovernanceChecks,
) -> Result<(), RefusalCode> {
    let audit = raw
        .pointer("/governance/audit")
        .and_then(Value::as_object)
        .ok_or(RefusalCode::UnsupportedProfile)?;
    if audit
        .keys()
        .any(|key| !matches!(key.as_str(), "reasoningTier" | "counterfactualTier"))
    {
        return Err(RefusalCode::UnsupportedProfile);
    }
    for (name, flag) in [
        ("reasoningTier", "requireDecisionRequirements"),
        ("counterfactualTier", "requireProtectedCharacteristics"),
    ] {
        let tier = audit
            .get(name)
            .and_then(Value::as_object)
            .ok_or(RefusalCode::UnsupportedProfile)?;
        if tier
            .keys()
            .any(|key| key != "requiredForTags" && key != flag)
        {
            return Err(RefusalCode::UnsupportedProfile);
        }
        let tags = if name == "reasoningTier" {
            let typed = checks
                .audit
                .reasoning_tier
                .as_ref()
                .ok_or(RefusalCode::UnsupportedProfile)?;
            if typed.require_decision_requirements {
                return Err(RefusalCode::UnsupportedProfile);
            }
            &typed.required_for_tags
        } else {
            let typed = checks
                .audit
                .counterfactual_tier
                .as_ref()
                .ok_or(RefusalCode::UnsupportedProfile)?;
            &typed.required_for_tags
        };
        for tag in tags {
            let locations = lifecycle.transition_locations(tag);
            if locations.is_empty()
                || locations.iter().any(|location| {
                    !policy
                        .determinations
                        .iter()
                        .any(|binding| binding.transition == *location)
                })
            {
                return Err(RefusalCode::UnboundTransition);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate(
    cap: &ReferenceDueProcessCapabilities,
    kernel: &KernelDocument,
) -> Result<(Value, DueProcessExecution), RefusalCode> {
    let (raw, policy) = selected(cap, kernel)?;
    let lifecycle = LifecycleIndex::new(kernel);
    let governance = raw
        .get("governance")
        .ok_or(RefusalCode::InvalidConfiguration)?;
    let checks = wos_core::model::governance_checks::check_reference_governance(
        kernel, governance, &lifecycle,
    );
    if let Some(issue) = checks.issues.first() {
        return Err(match issue.kind {
            GovernanceIssueKind::UnknownAuditTag => RefusalCode::UnboundTransition,
            GovernanceIssueKind::MalformedDeclaration => RefusalCode::InvalidConfiguration,
            GovernanceIssueKind::CounterfactualInputs => RefusalCode::ExplanationRejected,
            _ => RefusalCode::UnsupportedProfile,
        });
    }
    validate_audit(&raw, &lifecycle, &policy, &checks)?;
    let due = raw
        .pointer("/governance/dueProcess")
        .ok_or(RefusalCode::InvalidConfiguration)?;
    let adverse = due
        .get("adverseDecisionPolicy")
        .ok_or(RefusalCode::InvalidConfiguration)?;
    if due.get("scope").is_some()
        || adverse.get("noticeRequired") != Some(&Value::Bool(true))
        || adverse.get("noticeTiming").and_then(Value::as_str) != Some("beforeEffective")
        || adverse.pointer("/appealMechanism/enabled") != Some(&Value::Bool(true))
        || adverse.pointer("/appealMechanism/continuationOfServices") != Some(&Value::Bool(true))
        || adverse
            .pointer("/appealMechanism/continuationScope")
            .and_then(Value::as_str)
            != Some("currentLevel")
        || adverse
            .pointer("/appealMechanism/continuationPolicyRef")
            .is_some()
    {
        return Err(RefusalCode::UnsupportedProfile);
    }
    duration(adverse.get("noticeGracePeriod").and_then(Value::as_str))?;
    duration(
        adverse
            .pointer("/appealMechanism/appealWindow")
            .and_then(Value::as_str),
    )?;
    if policy.determinations.is_empty()
        || policy.operations.is_empty()
        || policy.notice.templates.is_empty()
        || !unique(&policy.notice.templates)
        || policy.appeal.channels.is_empty()
        || !unique(&policy.appeal.channels)
        || policy.appeal.standing_actors.is_empty()
        || !unique(&policy.appeal.standing_actors)
        || policy.appeal.reviewer_actors.is_empty()
        || !unique(&policy.appeal.reviewer_actors)
        || !(1..=36500).contains(&policy.appeal.hearing_deadline.calendar_days)
        || adverse.get("noticeTemplateKey").and_then(Value::as_str)
            != Some(policy.notice.templates[0].template_key.as_str())
    {
        return Err(RefusalCode::InvalidConfiguration);
    }
    for limit in [
        policy.limits.max_determinations,
        policy.limits.max_notices,
        policy.limits.max_appeals,
        policy.limits.max_operations,
        policy.limits.max_outstanding_notice_work,
        policy.limits.max_transport_attempts,
    ] {
        if !(1..=100000).contains(&limit) {
            return Err(RefusalCode::InvalidConfiguration);
        }
    }
    if !(1..=10_485_760).contains(&policy.limits.max_notice_bytes) {
        return Err(RefusalCode::InvalidConfiguration);
    }
    let mut evidence_ids = HashSet::new();
    for e in &cap.retained_evidence {
        let bytes = STANDARD
            .decode(&e.bytes_base64)
            .map_err(|_| RefusalCode::InvalidConfiguration)?;
        if !id(&e.id)
            || !evidence_ids.insert(&e.id)
            || bytes.len() > 10_485_760
            || STANDARD.encode(&bytes) != e.bytes_base64
            || bytes_digest(&bytes) != e.sha256
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
    }
    sources(cap, &policy.notice.source_refs, EvidenceRole::Source)?;
    sources(cap, &policy.appeal.source_refs, EvidenceRole::Source)?;
    let mut locales = HashSet::new();
    for t in &policy.notice.templates {
        if !id(&t.template_key)
            || !text(&t.locale, 128)
            || !locales.insert(&t.locale)
            || !cap.retained_evidence.iter().any(|e| {
                e.id == t.review.id && e.sha256 == t.review.sha256 && e.role == EvidenceRole::Review
            })
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
        let variants: Vec<_> = cap
            .templates
            .iter()
            .flat_map(|d| d.templates.iter())
            .filter(|(key, template)| {
                *key == &t.template_key && template.locale_ref.as_deref() == Some(t.locale.as_str())
            })
            .collect();
        if variants.len() != 1 || variants[0].1.sections.iter().any(|s| s.condition.is_some()) {
            return Err(RefusalCode::UnsupportedProfile);
        }
    }
    let mut ids = HashSet::new();
    let mut locations = Vec::new();
    let mut protected: Vec<&str> = Vec::new();
    for d in &policy.determinations {
        if !id(&d.id)
            || !ids.insert(&d.id)
            || locations.contains(&d.transition)
            || d.applied_state_pointers.is_empty()
            || d.service_pointers.is_empty()
            || d.applied_state_pointers.contains(&d.pending_state_pointer)
            || !unique(&d.applied_state_pointers)
            || !unique(&d.service_pointers)
            || !id(&d.signature_task_ref)
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
        locations.push(d.transition.clone());
        let trans = transition(&raw, &d.transition)?;
        if !trans.tags.iter().any(|t| t == "determination")
            || !trans.tags.iter().any(|t| t == "adverse-decision")
            || trans.target != state_id(&raw, &d.pending_state_pointer)?
            || trans.actor.is_none()
            || trans
                .guard
                .as_ref()
                .is_none_or(|g| g.as_fel_str().is_some())
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
        for p in &d.applied_state_pointers {
            state_id(&raw, p)?;
        }
        for p in &d.service_pointers {
            if !pointer(p)
                || protected.iter().any(|q| {
                    *q == p || q.starts_with(&format!("{p}/")) || p.starts_with(&format!("{q}/"))
                })
            {
                return Err(RefusalCode::InvalidConfiguration);
            }
            protected.push(p);
        }
        sources(cap, &d.source_refs, EvidenceRole::Source)?;
        if d.explanation.reasons.is_empty() {
            return Err(RefusalCode::ExplanationRejected);
        }
        let declared = &d.explanation.protected_characteristics;
        if declared.is_empty() || !unique(declared) {
            return Err(RefusalCode::ExplanationRejected);
        }
        for (index, characteristic) in declared.iter().enumerate() {
            if !pointer(&characteristic.case_pointer)
                || declared[..index].iter().any(|other| {
                    other.case_pointer == characteristic.case_pointer
                        || other
                            .case_pointer
                            .starts_with(&format!("{}/", characteristic.case_pointer))
                        || characteristic
                            .case_pointer
                            .starts_with(&format!("{}/", other.case_pointer))
                })
            {
                return Err(RefusalCode::InvalidConfiguration);
            }
            sources(cap, &characteristic.source_refs, EvidenceRole::Source)?;
            if !d.explanation.negative.iter().any(|alternative| {
                alternative.changes.len() == 1
                    && alternative.changes[0].case_pointer == characteristic.case_pointer
            }) {
                return Err(RefusalCode::ExplanationRejected);
            }
        }
        if d.explanation.negative.iter().any(|alternative| {
            alternative.changes.len() != 1
                || !declared.iter().any(|characteristic| {
                    characteristic.case_pointer == alternative.changes[0].case_pointer
                })
        }) {
            return Err(RefusalCode::ExplanationRejected);
        }
        for reason in &d.explanation.reasons {
            if reason.table_ref != d.explanation.table_ref
                || !id(&reason.reason_code)
                || !text(&reason.row_id, 128)
                || !text(&reason.description, 4096)
            {
                return Err(RefusalCode::InvalidConfiguration);
            }
            sources(cap, &reason.source_refs, EvidenceRole::Source)?;
        }
        let mut alternative_ids = HashSet::new();
        for alternative in d.explanation.positive.iter().chain(&d.explanation.negative) {
            if !id(&alternative.id)
                || !alternative_ids.insert(&alternative.id)
                || alternative.changes.is_empty()
                || !text(&alternative.description, 4096)
            {
                return Err(RefusalCode::InvalidConfiguration);
            }
            sources(cap, &alternative.source_refs, EvidenceRole::Source)?;
            for (i, c) in alternative.changes.iter().enumerate() {
                if !pointer(&c.case_pointer)
                    || alternative.changes[..i].iter().any(|q| {
                        q.case_pointer == c.case_pointer
                            || q.case_pointer.starts_with(&format!("{}/", c.case_pointer))
                            || c.case_pointer.starts_with(&format!("{}/", q.case_pointer))
                    })
                {
                    return Err(RefusalCode::InvalidConfiguration);
                }
            }
        }
    }
    let mut operation_ids = HashSet::new();
    let mut kinds = Vec::new();
    for op in &policy.operations {
        let trans = transition(&raw, &op.transition)?;
        let pair = (&op.determination_ref, &op.kind);
        if !id(&op.id)
            || !operation_ids.insert(&op.id)
            || locations.contains(&op.transition)
            || kinds.contains(&pair)
            || !ids.contains(&op.determination_ref)
            || op.actors.is_empty()
            || !unique(&op.actors)
            || op.required_state_pointers.is_empty()
            || !unique(&op.required_state_pointers)
            || !id(&op.trigger_event)
            || trans.event_dispatch_label() != op.trigger_event
            || trans.actor.as_ref().is_some_and(|a| !op.actors.contains(a))
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
        locations.push(op.transition.clone());
        kinds.push(pair);
        for p in &op.required_state_pointers {
            state_id(&raw, p)?;
        }
        sources(cap, &op.source_refs, EvidenceRole::Source)?;
    }
    if let Some(ext) = &policy.pre_notice_extension {
        if !id(&ext.timer_id)
            || ext.allowed_state_pointers.is_empty()
            || !unique(&ext.allowed_state_pointers)
            || !(1..=36500).contains(&ext.max_cumulative_calendar_days)
        {
            return Err(RefusalCode::InvalidConfiguration);
        }
        for p in &ext.allowed_state_pointers {
            state_id(&raw, p)?;
        }
        sources(cap, &ext.source_refs, EvidenceRole::Source)?;
    }
    cap.signature_verifier
        .validate_configuration(kernel, &policy)?;
    Ok((raw, policy))
}

pub(super) fn validate_record(
    cap: &ReferenceDueProcessCapabilities,
    kernel: &KernelDocument,
    record: &RuntimeRecord,
) -> Result<(Value, DueProcessExecution), RefusalCode> {
    let (raw, policy) = validate(cap, kernel)?;
    if let Some(creation) = &record.creation {
        let original = creation
            .reference_due_process
            .as_ref()
            .ok_or(RefusalCode::StoreRejected)?;
        if original.definition_sha256 != digest(&raw) || original.execution != policy {
            return Err(RefusalCode::StoreRejected);
        }
    }
    let adverse = raw
        .pointer("/governance/dueProcess/adverseDecisionPolicy")
        .ok_or(RefusalCode::InvalidConfiguration)?;
    for notice in &operations::state(record).notices {
        if notice.appeal_deadline
            != add_days(
                &notice.issue_at,
                duration(
                    adverse
                        .pointer("/appealMechanism/appealWindow")
                        .and_then(Value::as_str),
                )?,
            )?
            || notice.effect_eligible_at
                != add_days(
                    &notice.issue_at,
                    duration(adverse.get("noticeGracePeriod").and_then(Value::as_str))?,
                )?
        {
            return Err(RefusalCode::StoreRejected);
        }
    }
    Ok((raw, policy))
}

/// Checks lexical shape independently of work equality or clock relationships.
///
/// Valid but wrong identities, digests, correlation and future times remain
/// diagnostic claims for the separate receipt-admission owner to reject.
pub(super) fn validate_transport_result(result: &TransportResult) -> Result<(), RefusalCode> {
    fn digest(value: &str) -> bool {
        value.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    }
    fn acknowledgement(value: &TransportAcknowledgement) -> bool {
        !value.notice_id.is_empty()
            && token(&value.transport_key)
            && token(&value.correlation)
            && digest(&value.artifact_sha256)
            && token(&value.adapter_result_id)
            && timestamp(&value.accepted_at).is_ok()
    }
    fn receipt(value: &QualifiedNoticeReceipt) -> bool {
        !value.notice_id.is_empty()
            && token(&value.transport_key)
            && token(&value.correlation)
            && digest(&value.artifact_sha256)
            && token(&value.adapter_result_id)
            && timestamp(&value.qualified_at).is_ok()
    }
    let valid = match result {
        TransportResult::Failed { .. } => true,
        TransportResult::AcceptedOnly {
            acknowledgement: value,
        } => acknowledgement(value),
        TransportResult::Qualified {
            acknowledgement: ack,
            receipt: value,
        } => acknowledgement(ack) && receipt(value),
    };
    if valid {
        Ok(())
    } else {
        Err(RefusalCode::InvalidConfiguration)
    }
}
