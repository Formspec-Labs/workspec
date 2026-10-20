// Rust guideline compliant 2026-02-21
//! Owns bounded native due-process admission and retained transport work.
pub(super) mod admission;
mod transport;
pub use transport::{DeterministicNoticeTransport, NoticeTransport};

use crate::store::RuntimeRecord;
use serde::{Deserialize, Serialize};
use wos_core::model::due_process::{DeterminationSignature, DeterminationState, RefusalCode};
use wos_core::model::kernel::KernelDocument;
use wos_core::model::notification_template::NotificationTemplateDocument;

/// Verifies the actual accepted supervisor Response and its exact determination.
///
/// Implementations must use the maintained document signature verifier in addition
/// to the native Signature Profile and receipt admission path. A receipt verdict
/// alone does not authenticate the actual signed Response.
pub trait ReferenceDeterminationSignatureVerifier: Send + Sync {
    /// Admits selected source-signing capabilities before workflow activation.
    ///
    /// # Errors
    /// Refuses missing exact task-bound native verification capabilities.
    fn validate_configuration(
        &self,
        _kernel: &KernelDocument,
        _execution: &wos_core::model::due_process::DueProcessExecution,
    ) -> Result<(), RefusalCode> {
        Err(RefusalCode::SignatureRejected)
    }

    /// Revalidates cryptographic input association using retained original bytes.
    ///
    /// # Errors
    /// Returns a signature refusal for missing or unauthenticated exact evidence.
    fn verify(
        &self,
        record: &RuntimeRecord,
        kernel: &KernelDocument,
        determination: &DeterminationState,
        now_ms: u64,
    ) -> Result<DeterminationSignature, RefusalCode>;
}

/// Original selected evidence bytes and role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSelection {
    pub id: String,
    pub role: EvidenceRole,
    pub bytes_base64: String,
    pub sha256: String,
}
/// Closed retained evidence families.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceRole {
    Source,
    Review,
    HearingEvidence,
    OverrideEvidence,
}

/// Explicit host capabilities for the selected software fixture profile.
///
/// This value is native host configuration. Workflow JSON and generic event data
/// cannot create capabilities or select a signature verdict.
pub struct ReferenceDueProcessCapabilities {
    pub workflow_bytes: Vec<u8>,
    pub retained_evidence: Vec<EvidenceSelection>,
    pub templates: Vec<NotificationTemplateDocument>,
    pub notice_transport: Box<dyn NoticeTransport + Send + Sync>,
    pub signature_verifier: Box<dyn ReferenceDeterminationSignatureVerifier>,
}

impl ReferenceDueProcessCapabilities {
    /// Validates selected configuration and retained capability input associations.
    ///
    /// # Errors
    /// Refuses unsupported policy and invalid original evidence selections.
    pub fn validate_configuration(&self, kernel: &KernelDocument) -> Result<(), RuntimeError> {
        admission::validate(self, kernel)
            .map(|_| ())
            .map_err(RuntimeError::Governance)
    }
}

mod explanation;
mod operations;
pub(crate) mod records;

use super::{RuntimeError, WosRuntime};
use serde_json::Value;
use wos_core::ProvenanceRecord;
use wos_core::eval::{Evaluator, GuardEvaluation, ObservedAction};
use wos_core::instance::PendingEvent;
use wos_core::model::due_process::*;

impl WosRuntime {
    /// Reads the actual native record for a selected reference observation.
    ///
    /// # Errors
    /// Returns the owning store or process-resolution error.
    pub fn load_reference_record(&self, process_id: &str) -> Result<RuntimeRecord, RuntimeError> {
        let record = self.load_record_by_process_or_case_ref(process_id)?;
        record.validate_creation()?;
        if let Some(cap) = &self.reference_due_process {
            let kernel = self.resolver.resolve_kernel(
                &record.process.definition_url,
                &record.process.definition_version,
            )?;
            admission::validate_record(cap, &kernel, &record).map_err(RuntimeError::Governance)?;
            for determination in &operations::state(&record).determinations {
                if let Some(saved) = &determination.signature {
                    let verified = cap
                        .signature_verifier
                        .verify(&record, &kernel, determination, self.clock.now_ms())
                        .map_err(RuntimeError::Governance)?;
                    if &verified != saved {
                        return Err(RuntimeError::Governance(RefusalCode::SignatureRejected));
                    }
                }
            }
        }
        Ok(record)
    }

    /// Installs explicit reference capabilities after validating selected documents.
    ///
    /// # Errors
    /// Refuses unsupported configuration before a workflow can activate.
    pub fn configure_reference_due_process(
        &mut self,
        capabilities: ReferenceDueProcessCapabilities,
    ) -> Result<(), RuntimeError> {
        if self.preparation.is_some() || self.generic_governance.is_some() {
            return Err(RuntimeError::Governance(RefusalCode::UnsupportedProfile));
        }
        let parsed: KernelDocument = serde_json::from_slice(&capabilities.workflow_bytes)
            .map_err(|_| RuntimeError::Governance(RefusalCode::InvalidConfiguration))?;
        admission::validate(&capabilities, &parsed).map_err(RuntimeError::Governance)?;
        self.reference_due_process = Some(capabilities);
        Ok(())
    }

    /// Dispatches one trusted native governance request with independent caller identity.
    ///
    /// # Errors
    /// Persistence failures return errors; policy refusals retain a typed result.
    pub fn governance_operation(
        &mut self,
        process_id: &str,
        caller: &str,
        request: GovernanceRequest,
    ) -> Result<GovernanceResult, RuntimeError> {
        self.governance_operation_with_observation(process_id, caller, request)
            .map(|(result, _)| result)
    }

    /// Dispatches once and retains the actual native drain result, if newly executed.
    ///
    /// # Errors
    /// Returns native persistence or execution errors without inventing observations.
    pub fn governance_operation_with_observation(
        &mut self,
        process_id: &str,
        caller: &str,
        request: GovernanceRequest,
    ) -> Result<(GovernanceResult, Option<super::DrainOnceResult>), RuntimeError> {
        match self.governance_operation_inner(process_id, caller, &request) {
            Ok(result) => Ok(result),
            Err(RuntimeError::Governance(code)) => Ok((GovernanceResult::Refused { code }, None)),
            Err(error) => Err(error),
        }
    }
    fn governance_operation_inner(
        &mut self,
        process_id: &str,
        caller: &str,
        request: &GovernanceRequest,
    ) -> Result<(GovernanceResult, Option<super::DrainOnceResult>), RuntimeError> {
        let record = self.load_reference_record(process_id)?;
        let kernel = self.resolver.resolve_kernel(
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        let cap = self
            .reference_due_process
            .as_ref()
            .ok_or(RuntimeError::Governance(RefusalCode::UnsupportedProfile))?;
        let (raw, policy) =
            admission::validate_record(cap, &kernel, &record).map_err(RuntimeError::Governance)?;
        let op = policy
            .operations
            .iter()
            .find(|op| op.id == request.operation_ref)
            .ok_or(RuntimeError::Governance(RefusalCode::TargetRejected))?;
        admission::actor(
            self,
            &kernel,
            &record,
            &admission::transition(&raw, &op.transition).map_err(RuntimeError::Governance)?,
            &op.actors,
            &request.actor,
            caller,
        )
        .map_err(RuntimeError::Governance)?;
        if !admission::token(&request.idempotency_token) {
            return Err(RuntimeError::Governance(RefusalCode::InvalidConfiguration));
        }
        let saved = operations::state(&record);
        if let Some(prior) = saved
            .replays
            .iter()
            .find(|p| p.request.idempotency_token == request.idempotency_token)
        {
            if prior.request == *request {
                return Ok((prior.result.clone(), None));
            }
            return Err(RuntimeError::Governance(RefusalCode::ReplayConflict));
        }
        if op.kind != request.command.operation_kind() {
            return Err(RuntimeError::Governance(RefusalCode::InvalidConfiguration));
        }
        let observation = self.drain_once_governance(process_id, Some(request))?;
        let saved = operations::state(&self.store.load_record(process_id)?);
        saved
            .replays
            .iter()
            .find(|r| r.request == *request)
            .map(|r| (r.result.clone(), Some(observation)))
            .ok_or(RuntimeError::Governance(RefusalCode::SelectionRejected))
    }

    pub(super) fn reference_governance_event(
        &self,
        kernel: &KernelDocument,
        request: &GovernanceRequest,
        now: &str,
    ) -> Result<PendingEvent, RuntimeError> {
        let cap = self
            .reference_due_process
            .as_ref()
            .ok_or(RuntimeError::Governance(RefusalCode::UnsupportedProfile))?;
        let (_, policy) = admission::validate(cap, kernel).map_err(RuntimeError::Governance)?;
        let op = policy
            .operations
            .iter()
            .find(|o| o.id == request.operation_ref)
            .ok_or(RuntimeError::Governance(RefusalCode::TargetRejected))?;
        let mut governance =
            serde_json::json!({"operationRef":request.operation_ref,"command":request.command});
        if let Some(id) = &request.determination_id {
            governance["determinationId"] = Value::String(id.clone());
        }
        Ok(PendingEvent {
            event: op.trigger_event.clone(),
            actor_id: Some(request.actor.acting_actor.clone()),
            data: Some(serde_json::json!({"governance":governance})),
            timestamp: now.into(),
            origin: None,
            idempotency_token: None,
        })
    }

    pub(super) fn reference_preflight(
        &self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        evaluator: &Evaluator,
        actions: &[ObservedAction],
        guards: &[GuardEvaluation],
        request: Option<&GovernanceRequest>,
        now: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        self.reference_preflight_inner(kernel, record, evaluator, actions, guards, request, now)
            .map_err(RuntimeError::Governance)
    }
    fn reference_preflight_inner(
        &self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        evaluator: &Evaluator,
        actions: &[ObservedAction],
        guards: &[GuardEvaluation],
        request: Option<&GovernanceRequest>,
        now: &str,
    ) -> Result<Vec<ProvenanceRecord>, RefusalCode> {
        if self.preparation.is_some() && self.reference_due_process.is_some() {
            self.require_native_dependency(
                super::NativeReadOnlyDependency::DueProcessSignatureVerifier,
            )
            .map_err(|_| RefusalCode::UnsupportedProfile)?;
        }
        let declared = kernel
            .governance
            .as_ref()
            .and_then(|g| g.pointer("/dueProcess/execution"))
            .is_some();
        if !declared {
            return Ok(Vec::new());
        }
        let cap = self
            .reference_due_process
            .as_ref()
            .ok_or(RefusalCode::UnsupportedProfile)?;
        let (raw, policy) = admission::validate_record(cap, kernel, record)?;
        for declared in policy
            .appeal
            .standing_actors
            .iter()
            .chain(&policy.appeal.reviewer_actors)
        {
            let human = admission::canonical_actor(kernel, &record.process, declared)?;
            if !human.starts_with("actor:human:") {
                return Err(RefusalCode::ActorRejected);
            }
        }
        let mut state = operations::state(record);
        let mut extra = Vec::new();
        let transitions = evaluator.transitions();
        let prospective = evaluator.provenance().records();
        let mut allowed_effects = Vec::new();
        let mut selected_request = false;
        for transition in transitions {
            if let Some(binding) = policy
                .determinations
                .iter()
                .find(|d| d.transition == transition.location)
            {
                if state.determinations.len() >= policy.limits.max_determinations {
                    return Err(RefusalCode::LimitReached);
                }
                let trans = admission::transition(&raw, &transition.location)?;
                let declared_actor = trans.actor.as_ref().ok_or(RefusalCode::ActorRejected)?;
                let canonical =
                    admission::canonical_actor(kernel, &record.process, declared_actor)?;
                let actual = prospective
                    .iter()
                    .find(|p| p.id == transition.provenance_record_id)
                    .ok_or(RefusalCode::TargetRejected)?;
                if !canonical.starts_with("actor:human:")
                    || actual.actor_id.as_deref() != Some(canonical.as_str())
                {
                    return Err(RefusalCode::ActorRejected);
                }
                admission::actor(
                    self,
                    kernel,
                    record,
                    &trans,
                    &[declared_actor.clone()],
                    &GovernanceActor {
                        declared_actor: declared_actor.clone(),
                        acting_actor: canonical.clone(),
                    },
                    &canonical,
                )?;
                let snapshot = actual
                    .case_file_snapshot
                    .as_ref()
                    .ok_or(RefusalCode::ExplanationRejected)?;
                for p in &binding.service_pointers {
                    if snapshot.value.pointer(p).is_none() {
                        return Err(RefusalCode::InvalidConfiguration);
                    }
                }
                let determination = DeterminationState {
                    id: actual.id.clone(),
                    binding_ref: binding.id.clone(),
                    firing: FiringRef {
                        definition_sha256: admission::digest(&raw),
                        location: transition.location.clone(),
                        provenance_record_id: actual.id.clone(),
                    },
                    determined_at: now.into(),
                    snapshot: SnapshotRef {
                        record_id: actual.id.clone(),
                        sha256: snapshot.sha256.clone(),
                    },
                    determining_human: canonical,
                    service_pointers: binding.service_pointers.clone(),
                    signature: None,
                    effect: None,
                };
                let matching: Vec<_> = guards
                    .iter()
                    .filter(|g| {
                        g.firing_record_id.as_deref() == Some(actual.id.as_str())
                            && g.location.as_ref() == Some(&binding.transition)
                    })
                    .collect();
                if matching.len() != 1 {
                    return Err(RefusalCode::ExplanationRejected);
                }
                extra.extend(explanation::records(
                    &super::process_minter(&record.process)
                        .map_err(|_| RefusalCode::StoreRejected)?,
                    kernel,
                    binding,
                    &determination,
                    &snapshot.value,
                    matching[0],
                    now,
                )?);
                state.determinations.push(determination);
            } else if transition.tags.iter().any(|t| t == "adverse-decision") {
                return Err(RefusalCode::UnboundTransition);
            }
            if let Some(operation) = policy
                .operations
                .iter()
                .find(|o| o.transition == transition.location)
            {
                if !admission::member(
                    &raw,
                    &record.process.configuration,
                    &operation.required_state_pointers,
                )? {
                    return Err(RefusalCode::SelectionRejected);
                }
                let trans = admission::transition(&raw, &transition.location)?;
                let actual = prospective
                    .iter()
                    .find(|p| p.id == transition.provenance_record_id)
                    .ok_or(RefusalCode::TargetRejected)?;
                let internal;
                let invocation = if let Some(request) = request {
                    if request.operation_ref != operation.id
                        || request.command.operation_kind() != operation.kind
                        || selected_request
                    {
                        return Err(RefusalCode::SelectionRejected);
                    }
                    selected_request = true;
                    request
                } else {
                    if !matches!(
                        operation.kind,
                        OperationKind::IssueNotice | OperationKind::ApplyAdverseEffect
                    ) {
                        return Err(RefusalCode::InvocationRequired);
                    }
                    let declared = trans.actor.as_ref().ok_or(RefusalCode::ActorRejected)?;
                    let canonical = admission::canonical_actor(kernel, &record.process, declared)?;
                    let targets: Vec<_> = state
                        .determinations
                        .iter()
                        .filter(|d| {
                            d.binding_ref == operation.determination_ref && d.effect.is_none()
                        })
                        .collect();
                    if targets.len() != 1 {
                        return Err(RefusalCode::TargetRejected);
                    }
                    internal = GovernanceRequest {
                        operation_ref: operation.id.clone(),
                        determination_id: Some(targets[0].id.clone()),
                        idempotency_token: format!("internal.{}", actual.id),
                        actor: GovernanceActor {
                            declared_actor: declared.clone(),
                            acting_actor: canonical,
                        },
                        command: if operation.kind == OperationKind::IssueNotice {
                            GovernanceCommand::IssueNotice
                        } else {
                            GovernanceCommand::ApplyAdverseEffect
                        },
                    };
                    &internal
                };
                admission::actor(
                    self,
                    kernel,
                    record,
                    &trans,
                    &operation.actors,
                    &invocation.actor,
                    actual
                        .actor_id
                        .as_deref()
                        .ok_or(RefusalCode::ActorRejected)?,
                )?;
                let firing = FiringRef {
                    definition_sha256: admission::digest(&raw),
                    location: transition.location.clone(),
                    provenance_record_id: actual.id.clone(),
                };
                let (outcome, provenance) = operations::apply(
                    self,
                    cap,
                    kernel,
                    &raw,
                    &policy,
                    record,
                    &mut state,
                    operation,
                    invocation,
                    &firing,
                    prospective,
                    now,
                )?;
                extra.extend(provenance);
                if operation.kind == OperationKind::ApplyAdverseEffect {
                    // A chain could hide an earlier unbound entry/write behind a later apply.
                    if transitions.len() != 1 {
                        return Err(RefusalCode::UnsupportedProfile);
                    }
                    allowed_effects.push(actual.id.clone());
                }
                state.operations.push(OperationState {
                    record_id: actual.id.clone(),
                    operation_ref: operation.id.clone(),
                    determination_id: invocation.determination_id.clone(),
                    kind: operation.kind.clone(),
                    performed_at: now.into(),
                    actor: invocation.actor.clone(),
                    firing,
                    outcome: outcome.clone(),
                });
                if request.is_some() {
                    state.replays.push(ReplayState {
                        request: invocation.clone(),
                        result: GovernanceResult::Committed {
                            operation_record_id: actual.id.clone(),
                            outcome,
                        },
                    });
                }
            }
        }
        if request.is_some() && !selected_request {
            return Err(RefusalCode::SelectionRejected);
        }
        let snapshots = evaluator.action_case_states();
        if snapshots.len() != actions.len() {
            return Err(RefusalCode::UnsupportedProfile);
        }
        for binding in &policy.determinations {
            let original = state
                .determinations
                .iter()
                .rev()
                .find(|d| d.binding_ref == binding.id && d.effect.is_none())
                .and_then(|d| {
                    record
                        .provenance_log
                        .iter()
                        .chain(prospective)
                        .find(|p| p.id == d.snapshot.record_id)
                })
                .and_then(|p| p.case_file_snapshot.as_ref())
                .map(|s| &s.value)
                .unwrap_or(evaluator.initial_case_state());
            for pointer in &binding.service_pointers {
                let original = original
                    .pointer(pointer)
                    .ok_or(RefusalCode::InvalidConfiguration)?;
                for (action, snapshot) in actions.iter().zip(snapshots) {
                    if snapshot.pointer(pointer) != Some(original)
                        && !action
                            .transition_record_id
                            .as_ref()
                            .is_some_and(|id| allowed_effects.contains(id))
                    {
                        return Err(RefusalCode::ContinuationFrozen);
                    }
                }
                if evaluator.case_state_json().pointer(pointer) != Some(original)
                    && allowed_effects.is_empty()
                {
                    return Err(RefusalCode::UnboundTransition);
                }
            }
            for pointer in &binding.applied_state_pointers {
                if evaluator
                    .entered_state_pointers()
                    .iter()
                    .any(|p| p == pointer || p.starts_with(&format!("{pointer}/")))
                    && allowed_effects.is_empty()
                {
                    return Err(RefusalCode::UnboundTransition);
                }
            }
        }
        for observed in actions
            .iter()
            .filter(|a| a.action.action == wos_core::model::kernel::ActionKind::InvokeService)
        {
            if !self
                .qualified_reference_invocation(
                    kernel,
                    record,
                    observed,
                    &evaluator.case_state_json(),
                )
                .map_err(|_| RefusalCode::UnsupportedProfile)?
            {
                return Err(RefusalCode::UnsupportedProfile);
            }
        }
        operations::retain(record, state);
        Ok(extra)
    }
}

impl WosRuntime {
    pub(super) fn reference_apply_timer_extensions(
        &self,
        record: &mut RuntimeRecord,
        transitions: &[wos_core::eval::ObservedTransition],
    ) -> Result<(), RuntimeError> {
        let state = operations::state(record);
        for extension in state.extensions.iter().filter(|e| {
            transitions
                .iter()
                .any(|t| t.provenance_record_id == e.operation_record_id)
        }) {
            let timers: Vec<_> = record
                .process
                .timers
                .iter_mut()
                .filter(|t| t.timer_id == extension.timer_id)
                .collect();
            if timers.len() != 1 {
                return Err(RuntimeError::Governance(RefusalCode::OverrideRejected));
            }
            let timer = timers
                .into_iter()
                .next()
                .ok_or(RuntimeError::Governance(RefusalCode::OverrideRejected))?;
            if timer.deadline != extension.prior_deadline {
                return Err(RuntimeError::Governance(RefusalCode::OverrideRejected));
            }
            timer.deadline = extension.new_deadline.clone();
            timer.absolute_deadline = true;
        }
        Ok(())
    }
    pub(super) fn reference_validate_case_change(
        &self,
        kernel: &KernelDocument,
        record: &RuntimeRecord,
        next: &Value,
    ) -> Result<(), RuntimeError> {
        if kernel
            .governance
            .as_ref()
            .and_then(|g| g.pointer("/dueProcess/execution"))
            .is_none()
        {
            return Ok(());
        }
        let cap = self
            .reference_due_process
            .as_ref()
            .ok_or(RuntimeError::Governance(RefusalCode::UnsupportedProfile))?;
        let (_, policy) =
            admission::validate_record(cap, kernel, record).map_err(RuntimeError::Governance)?;
        for pointer in policy
            .determinations
            .iter()
            .flat_map(|d| &d.service_pointers)
        {
            if record.process.case_state.pointer(pointer) != next.pointer(pointer) {
                return Err(RuntimeError::Governance(RefusalCode::UnboundTransition));
            }
        }
        Ok(())
    }
}

pub(super) fn retained_notice_ids(record: &RuntimeRecord) -> Vec<String> {
    operations::state(record)
        .notices
        .into_iter()
        .map(|n| n.id)
        .collect()
}
pub(super) fn pending_notice_work(record: &RuntimeRecord) -> Vec<PendingNoticeWork> {
    operations::state(record)
        .notices
        .iter()
        .filter(|n| n.receipt.is_none())
        .map(|n| transport::work(record, n))
        .collect()
}
