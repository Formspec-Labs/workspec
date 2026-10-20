// Rust guideline compliant 2026-02-21

//! Executes one native command without operational writes or immediate effects.
//!
//! Host-selected dependency and binding identifiers select qualified read-only
//! implementations. They do not verify arbitrary Rust code, supply authority,
//! or replace the host's captured-input and current-state transaction fences.

use super::process_minter;
use super::staging_store::StageStore;
use super::{
    CreateProcessRequest, DismissTaskResult, DrainOnceResult, PersistDraftResult, RuntimeError,
    TaskSubmissionResult, WosRuntime,
};
use crate::intake::{
    IntakeAcceptanceDecision, IntakeAcceptanceRequest, IntakeRecordStatus, ProcessAllocation,
};
use crate::store::{IntakeRecord, RuntimeRecord, RuntimeStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use wos_core::instance::{
    ActiveTaskStatus, FormspecTaskContext, HumanActorBinding, PendingEvent, WorkflowProcess,
};
use wos_core::{ProvenanceKind, ProvenanceRecord};

/// A qualified read-only configured dependency slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeReadOnlyDependency {
    Clock,
    Resolver,
    AccessControl,
    ContractValidator,
    IntakePolicy,
    CompanionPolicy,
    GovernancePolicy,
    PostureResolver,
    DueProcessSignatureVerifier,
}
/// Task binding operation requiring separate host qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeTaskBindingOperation {
    Prepare,
    Validate,
    Map,
    SignatureEvidence,
}
/// Intake binding operation requiring separate host qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeIntakeBindingOperation {
    Interpret,
    Finalize,
}

/// Closed selection of host-qualified preparation-safe operations.
///
/// Empty selections refuse the corresponding call before invocation. Each ID
/// selects the host-configured implementation in that slot; Core cannot prove
/// its behavior from a string. In particular, task computation qualification
/// does not qualify signature extraction, and interpretation does not qualify
/// intake finalization. Irreversible RPCs must never be selected here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreparationPolicy {
    pub read_only_dependency_ids: BTreeMap<NativeReadOnlyDependency, String>,
    pub read_only_task_binding_ids: BTreeMap<NativeTaskBindingOperation, BTreeSet<String>>,
    pub read_only_intake_binding_ids: BTreeMap<NativeIntakeBindingOperation, BTreeSet<String>>,
    pub read_only_rpc_binding_ids: BTreeSet<String>,
    /// Native deferred callbacks prepare requests without invoking providers.
    pub deferred_callback_binding_ids: BTreeSet<String>,
}
impl NativePreparationPolicy {
    pub(super) fn dependency(
        &self,
        dependency: NativeReadOnlyDependency,
    ) -> Result<(), RuntimeError> {
        if self
            .read_only_dependency_ids
            .get(&dependency)
            .is_some_and(|id| !id.trim().is_empty())
        {
            Ok(())
        } else {
            Err(NativePreparationError::UnqualifiedDependency { dependency }.into())
        }
    }
    fn task_binding(
        &self,
        operation: NativeTaskBindingOperation,
        binding: &str,
    ) -> Result<(), RuntimeError> {
        if self
            .read_only_task_binding_ids
            .get(&operation)
            .is_some_and(|ids| ids.contains(binding))
        {
            Ok(())
        } else {
            Err(NativePreparationError::UnqualifiedTaskBinding {
                operation,
                binding: binding.into(),
            }
            .into())
        }
    }
    fn intake_binding(
        &self,
        operation: NativeIntakeBindingOperation,
        binding: &str,
    ) -> Result<(), RuntimeError> {
        if self
            .read_only_intake_binding_ids
            .get(&operation)
            .is_some_and(|ids| ids.contains(binding))
        {
            Ok(())
        } else {
            Err(NativePreparationError::UnqualifiedIntakeBinding {
                operation,
                binding: binding.into(),
            }
            .into())
        }
    }
}

/// Structured preparation refusal; strings never choose failure disposition.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum NativePreparationError {
    #[error("unqualified native preparation dependency: {dependency:?}")]
    UnqualifiedDependency {
        dependency: NativeReadOnlyDependency,
    },
    #[error("unqualified native task binding operation {operation:?}: {binding}")]
    UnqualifiedTaskBinding {
        operation: NativeTaskBindingOperation,
        binding: String,
    },
    #[error("unqualified native intake binding operation {operation:?}: {binding}")]
    UnqualifiedIntakeBinding {
        operation: NativeIntakeBindingOperation,
        binding: String,
    },
    #[error("immediate or unsupported preparation effect: {effect}")]
    UnsupportedEffect { effect: String },
    #[error("complete case membership was not supplied: {case_ledger_id}")]
    MissingMembership { case_ledger_id: String },
    #[error("invalid native snapshot: {reason}")]
    InvalidSnapshot { reason: String },
    #[error("presentation intent or acknowledgement conflicts: {intent_id}")]
    PresentationConflict { intent_id: String },
}

/// Host operation correlation; this key grants no authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeCommandKey(pub String);
/// Complete ordered native membership captured by the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseProcessMembership {
    pub case_ledger_id: String,
    pub process_ids: Vec<String>,
}
/// Captured native records, restored through maintained store validation.
#[derive(Debug, Default)]
pub struct NativeRuntimeSnapshot {
    pub process_records: Vec<RuntimeRecord>,
    pub intake_records: Vec<IntakeRecord>,
    pub complete_case_membership: Vec<CaseProcessMembership>,
}
/// One owning command; migration and external delivery are unsupported.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NativeRuntimeCommand {
    CreateProcess {
        request: CreateProcessRequest,
        allocation: ProcessAllocation,
        actor_bindings: Option<Vec<HumanActorBinding>>,
    },
    AcceptIntake {
        binding: String,
        request: IntakeAcceptanceRequest,
        allocation: Option<ProcessAllocation>,
    },
    EnqueueEvent {
        process_id: String,
        event: PendingEvent,
    },
    DrainOnce {
        process_id: String,
    },
    GovernanceOperation {
        process_id: String,
        caller_actor_id: String,
        request: wos_core::model::due_process::GovernanceRequest,
    },
    TaskDraft {
        task_id: String,
        response: serde_json::Value,
        actor_id: String,
        idempotency_token: String,
    },
    TaskSubmit {
        task_id: String,
        response: serde_json::Value,
        actor_id: String,
        idempotency_token: String,
    },
    TaskDismiss {
        task_id: String,
        reason: String,
        actor_id: String,
        idempotency_token: String,
    },
    ApplyCustodyReceipt {
        process_id: String,
        record_id: String,
        receipt: crate::CustodyAppendReceipt,
    },
    AcknowledgePresentation {
        intent: PendingNativePresentationIntent,
        invocation: NativePresentationInvocationOutcome,
    },
}
/// Actual native result captured inside the consuming preparation entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeRuntimeOutcome {
    ProcessCreated(WorkflowProcess),
    IntakeAccepted(IntakeAcceptanceDecision),
    EventEnqueued,
    Drained(DrainOnceResult),
    GovernanceOperation(wos_core::model::due_process::GovernanceResult),
    DraftPersisted(PersistDraftResult),
    TaskSubmitted(TaskSubmissionResult),
    TaskDismissed(DismissTaskResult),
    CustodyReceiptApplied,
    PresentationAcknowledged(NativePresentationAcknowledgement),
}
/// Coalesced writes preserve the actual captured prior record.
#[derive(Debug, Clone)]
pub enum NativeProcessChange {
    Create(RuntimeRecord),
    Replace {
        before: RuntimeRecord,
        after: RuntimeRecord,
    },
}
/// Coalesced intake writes preserve the actual acknowledged status and evidence.
#[derive(Debug, Clone)]
pub enum NativeIntakeChange {
    Create(IntakeRecord),
    Replace {
        before: IntakeRecord,
        after: IntakeRecord,
    },
}
/// Actual selected references; absent artifacts are never invented.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePresentationArtifactIdentity {
    pub binding: String,
    pub definition_url: String,
    pub definition_version: String,
    pub prefill_mapping_ref: Option<String>,
    pub response_mapping_ref: Option<String>,
    pub response_artifact_id: Option<String>,
}
/// Exact native presentation work for durable host retention and qualified dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingNativePresentationIntent {
    pub intent_id: String,
    pub command_key: NativeCommandKey,
    pub ordinal: u64,
    pub process_id: String,
    pub task_id: String,
    pub task_created_record_id: String,
    pub assigned_actor: String,
    pub task_updated_at: String,
    pub context: FormspecTaskContext,
    pub artifacts: NativePresentationArtifactIdentity,
}
/// Result from the host's qualified actual presentation attempt.
///
/// This input reports an invocation, not proof of authority, UI read or delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum NativePresentationInvocationOutcome {
    Invoked {
        attempt_id: String,
        occurred_at: String,
    },
    Failed {
        attempt_id: String,
        code: String,
    },
    Unknown {
        attempt_id: String,
    },
}
/// Truthful recording, obsolete work, or retained uncertain presentation work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum NativePresentationAcknowledgement {
    Recorded {
        provenance_id: String,
        occurred_at: String,
    },
    Obsolete {
        reason: String,
    },
    RecoveryRequired {
        intent_id: String,
        attempt_id: String,
    },
}
/// Acknowledged work that must survive a refused speculative command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum NativeRecoveryReference {
    Intake {
        binding: String,
        intake_id: String,
        status: IntakeRecordStatus,
    },
    Presentation {
        intent_id: String,
        attempt_id: String,
    },
}
/// Preparation errors discard speculative writes; acknowledged inputs remain unchanged.
#[derive(Debug)]
pub enum NativePreparationFailure {
    Refused {
        error: RuntimeError,
    },
    RecoveryRequired {
        work: Vec<NativeRecoveryReference>,
        error: RuntimeError,
    },
}

/// Newly prepared maintained deferred request; transport and ingress remain host-owned.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeferredOutboundEvent {
    pub process_id: String,
    pub event: wos_core::instance::PendingOutboundEvent,
}

/// Native prepared evidence with private construction and immutable accessors.
///
/// There is no deserializer, mutable accessor or arbitrary-result finish method.
/// WOS must compare the exact command/key to its protected command row and fence
/// the supplied inputs before atomically committing the returned changes/work.
#[derive(Debug)]
pub struct PreparedNativeCommand {
    key: NativeCommandKey,
    command: NativeRuntimeCommand,
    outcome: NativeRuntimeOutcome,
    processes: Vec<NativeProcessChange>,
    intakes: Vec<NativeIntakeChange>,
    presentations: Vec<PendingNativePresentationIntent>,
    queried_cases: Vec<String>,
    deferred: Vec<DeferredOutboundEvent>,
    notice_work: Vec<wos_core::model::due_process::PendingNoticeWork>,
}
impl PreparedNativeCommand {
    pub fn command_key(&self) -> &NativeCommandKey {
        &self.key
    }
    pub fn executed_command(&self) -> &NativeRuntimeCommand {
        &self.command
    }
    pub fn outcome(&self) -> &NativeRuntimeOutcome {
        &self.outcome
    }
    pub fn process_changes(&self) -> &[NativeProcessChange] {
        &self.processes
    }
    pub fn intake_changes(&self) -> &[NativeIntakeChange] {
        &self.intakes
    }
    pub fn presentation_intents(&self) -> &[PendingNativePresentationIntent] {
        &self.presentations
    }
    pub fn deferred_outbound_events(&self) -> &[DeferredOutboundEvent] {
        &self.deferred
    }
    /// Returns immutable notice work staged without any transport invocation.
    pub fn notice_work(&self) -> &[wos_core::model::due_process::PendingNoticeWork] {
        &self.notice_work
    }
    pub fn queried_case_ledger_ids(&self) -> &[String] {
        &self.queried_cases
    }
}

pub(super) struct PreparationState {
    pub(super) policy: NativePreparationPolicy,
    pub(super) contexts: Vec<FormspecTaskContext>,
}

impl WosRuntime {
    /// Execute one native command against a validated speculative store.
    ///
    /// The configured operational store and presenter are never called. Qualified
    /// read-only adapters may run. Host selection is not a sandbox for arbitrary
    /// Rust adapters; capture/current authority/SQL commit remain host-owned.
    ///
    /// # Errors
    /// Refuses invalid inputs, incomplete membership, unqualified or immediate
    /// effects and native command errors. Errors discard speculative writes and
    /// retain explicit references to matching acknowledged Pending/Prepared input.
    pub fn prepare_native_command(
        mut self,
        key: NativeCommandKey,
        command: NativeRuntimeCommand,
        snapshot: NativeRuntimeSnapshot,
        policy: NativePreparationPolicy,
    ) -> Result<PreparedNativeCommand, NativePreparationFailure> {
        let recovery = matching_recovery(&command, &snapshot.intake_records);
        let failure = |error| {
            if recovery.is_empty() {
                NativePreparationFailure::Refused { error }
            } else {
                NativePreparationFailure::RecoveryRequired {
                    work: recovery.clone(),
                    error,
                }
            }
        };
        if key.0.trim().is_empty() {
            return Err(failure(
                NativePreparationError::InvalidSnapshot {
                    reason: "command key is blank".into(),
                }
                .into(),
            ));
        }
        policy
            .dependency(NativeReadOnlyDependency::Clock)
            .map_err(&failure)?;
        let collector = StageStore::restore(snapshot).map_err(&failure)?;
        #[cfg(test)]
        collector
            .set_fault_at_write(self.preparation_fault_at_write)
            .map_err(RuntimeError::from)
            .map_err(&failure)?;
        self.store = Box::new(collector.clone());
        self.resolver = Box::new(PreparationResolver {
            inner: self.resolver,
            policy: policy.clone(),
        });
        self.validator = Box::new(PreparationValidator {
            inner: self.validator,
            policy: policy.clone(),
        });
        self.preparation = Some(PreparationState {
            policy,
            contexts: vec![],
        });
        let result = self.execute_native_command(&command);
        // RuntimeStore's list seam has no Result. Missing queried membership must
        // be detected even when native code interpreted its empty list as absent.
        collector.validate_queries().map_err(&failure)?;
        let outcome = result.map_err(&failure)?;
        let contexts = self
            .preparation
            .take()
            .ok_or_else(|| {
                failure(
                    NativePreparationError::InvalidSnapshot {
                        reason: "preparation state missing".into(),
                    }
                    .into(),
                )
            })?
            .contexts;
        let presentations = contexts
            .into_iter()
            .enumerate()
            .map(|(ordinal, context)| {
                let record = collector.load_record(&context.process_id)?;
                make_presentation_intent(&key, ordinal as u64, context, &record)
            })
            .collect::<Result<Vec<_>, RuntimeError>>()
            .map_err(&failure)?;
        let deferred = collector.deferred_outbound_events().map_err(&failure)?;
        let (processes, intakes, queried_cases) = collector.finish().map_err(&failure)?;
        let notice_work = processes
            .iter()
            .flat_map(|change| {
                let (before, after) = match change {
                    NativeProcessChange::Create(after) => (None, after),
                    NativeProcessChange::Replace { before, after } => (Some(before), after),
                };
                let previous = before
                    .map(super::governance::retained_notice_ids)
                    .unwrap_or_default();
                super::governance::pending_notice_work(after)
                    .into_iter()
                    .filter(move |work| !previous.contains(&work.notice_id))
            })
            .collect();
        Ok(PreparedNativeCommand {
            key,
            command,
            outcome,
            processes,
            intakes,
            presentations,
            queried_cases,
            deferred,
            notice_work,
        })
    }
    fn execute_native_command(
        &mut self,
        command: &NativeRuntimeCommand,
    ) -> Result<NativeRuntimeOutcome, RuntimeError> {
        match command {
            NativeRuntimeCommand::CreateProcess {
                request,
                allocation,
                actor_bindings,
            } => {
                allocation.validate()?;
                if request.process_id != allocation.process_id
                    || request.tenant.as_deref() != Some(allocation.tenant.as_str())
                {
                    return Err(RuntimeError::IntakeConflict(
                        "create request differs from retained allocation".into(),
                    ));
                }
                let process = match actor_bindings {
                    Some(bindings) => self.create_process_bound_to_case_with_actor_bindings(
                        request.clone(),
                        allocation.case_ledger_id.clone(),
                        bindings.clone(),
                    )?,
                    None => self.create_process_bound_to_case(
                        request.clone(),
                        allocation.case_ledger_id.clone(),
                    )?,
                };
                Ok(NativeRuntimeOutcome::ProcessCreated(process))
            }
            NativeRuntimeCommand::AcceptIntake {
                binding,
                request,
                allocation,
            } => Ok(NativeRuntimeOutcome::IntakeAccepted(
                self.accept_intake_handoff_with_allocation(
                    binding,
                    request.clone(),
                    allocation.clone(),
                )?,
            )),
            NativeRuntimeCommand::EnqueueEvent { process_id, event } => {
                self.enqueue_event(process_id, event.clone())?;
                Ok(NativeRuntimeOutcome::EventEnqueued)
            }
            NativeRuntimeCommand::DrainOnce { process_id } => {
                Ok(NativeRuntimeOutcome::Drained(self.drain_once(process_id)?))
            }
            NativeRuntimeCommand::GovernanceOperation {
                process_id,
                caller_actor_id,
                request,
            } => Ok(NativeRuntimeOutcome::GovernanceOperation(
                self.governance_operation(process_id, caller_actor_id, request.clone())?,
            )),
            NativeRuntimeCommand::TaskDraft {
                task_id,
                response,
                actor_id,
                idempotency_token,
            } => {
                require_token(idempotency_token)?;
                Ok(NativeRuntimeOutcome::DraftPersisted(
                    self.persist_task_draft(
                        task_id,
                        response.clone(),
                        actor_id,
                        Some(idempotency_token),
                    )?,
                ))
            }
            NativeRuntimeCommand::TaskSubmit {
                task_id,
                response,
                actor_id,
                idempotency_token,
            } => {
                require_token(idempotency_token)?;
                Ok(NativeRuntimeOutcome::TaskSubmitted(
                    self.submit_task_response(
                        task_id,
                        response.clone(),
                        actor_id,
                        Some(idempotency_token),
                    )?,
                ))
            }
            NativeRuntimeCommand::TaskDismiss {
                task_id,
                reason,
                actor_id,
                idempotency_token,
            } => {
                require_token(idempotency_token)?;
                Ok(NativeRuntimeOutcome::TaskDismissed(self.dismiss_task(
                    task_id,
                    reason,
                    actor_id,
                    idempotency_token,
                )?))
            }
            NativeRuntimeCommand::ApplyCustodyReceipt {
                process_id,
                record_id,
                receipt,
            } => {
                self.apply_custody_receipt(process_id, record_id, receipt.clone())?;
                Ok(NativeRuntimeOutcome::CustodyReceiptApplied)
            }
            NativeRuntimeCommand::AcknowledgePresentation { intent, invocation } => {
                Ok(NativeRuntimeOutcome::PresentationAcknowledged(
                    self.acknowledge_prepared_presentation(intent, invocation)?,
                ))
            }
        }
    }
    pub(super) fn require_native_creation_allocation(
        &self,
        outcome: &crate::intake::IntakeAcceptanceOutcome,
        allocation: Option<&ProcessAllocation>,
    ) -> Result<(), RuntimeError> {
        if self.preparation.is_some()
            && allocation.is_none()
            && matches!(
                outcome,
                crate::intake::IntakeAcceptanceOutcome::Accepted {
                    case_disposition: crate::intake::IntakeCaseDisposition::CreateGovernedCase { .. }
                }
            )
        {
            return Err(RuntimeError::IntakeConflict(
                "prepared creation requires host-retained allocation".into(),
            ));
        }
        Ok(())
    }
    pub(super) fn require_native_dependency(
        &self,
        dependency: NativeReadOnlyDependency,
    ) -> Result<(), RuntimeError> {
        self.preparation
            .as_ref()
            .map_or(Ok(()), |state| state.policy.dependency(dependency))
    }
    pub(super) fn require_native_task_binding(
        &self,
        operation: NativeTaskBindingOperation,
        binding: &str,
    ) -> Result<(), RuntimeError> {
        self.preparation.as_ref().map_or(Ok(()), |state| {
            state.policy.task_binding(operation, binding)
        })
    }
    pub(super) fn require_native_intake_binding(
        &self,
        operation: NativeIntakeBindingOperation,
        binding: &str,
    ) -> Result<(), RuntimeError> {
        self.preparation.as_ref().map_or(Ok(()), |state| {
            state.policy.intake_binding(operation, binding)
        })
    }
    pub(super) fn require_native_integration(
        &self,
        service_ref: &str,
        binding: &crate::integration::IntegrationBinding,
    ) -> Result<(), RuntimeError> {
        let Some(state) = &self.preparation else {
            return Ok(());
        };
        if binding.kind == crate::integration::IntegrationBindingKind::RequestResponse
            && state.policy.read_only_rpc_binding_ids.contains(service_ref)
        {
            return Ok(());
        }
        if binding.kind == crate::integration::IntegrationBindingKind::Callback
            && binding.dispatch_mode == Some(crate::integration::IntegrationDispatchMode::Deferred)
            && state
                .policy
                .deferred_callback_binding_ids
                .contains(service_ref)
        {
            return Ok(());
        }
        Err(NativePreparationError::UnsupportedEffect {
            effect: format!("integration {}:{:?}", service_ref, binding.kind),
        }
        .into())
    }
    pub(super) fn refuse_native_plain_service(
        &self,
        service_ref: &str,
    ) -> Result<(), RuntimeError> {
        if self.preparation.is_some() {
            Err(NativePreparationError::UnsupportedEffect {
                effect: format!("plain service {service_ref}"),
            }
            .into())
        } else {
            Ok(())
        }
    }

    fn acknowledge_prepared_presentation(
        &mut self,
        intent: &PendingNativePresentationIntent,
        invocation: &NativePresentationInvocationOutcome,
    ) -> Result<NativePresentationAcknowledgement, RuntimeError> {
        validate_intent_id(intent)?;
        let attempt_id = match invocation {
            NativePresentationInvocationOutcome::Invoked { attempt_id, .. }
            | NativePresentationInvocationOutcome::Failed { attempt_id, .. }
            | NativePresentationInvocationOutcome::Unknown { attempt_id } => attempt_id,
        };
        require_token(attempt_id)?;
        if let NativePresentationInvocationOutcome::Failed { code, .. } = invocation {
            require_token(code)?;
        }
        let mut record = self.store.load_record(&intent.process_id)?;
        let minter = process_minter(&record.process)?;
        record.validate_creation()?;
        let fingerprint = stable_json_digest(&serde_json::to_value(intent).map_err(json_error)?);
        let invocation_value = serde_json::to_value(invocation).map_err(json_error)?;
        let existing: Vec<_> = record
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == ProvenanceKind::TaskPresented)
            .filter(|p| {
                p.data
                    .as_ref()
                    .and_then(|d| d.pointer("/details/nativePresentationIntentId"))
                    .and_then(serde_json::Value::as_str)
                    == Some(intent.intent_id.as_str())
            })
            .collect();
        if existing.len() > 1 {
            return Err(presentation_conflict(intent));
        }
        if let Some(prior) = existing.first() {
            if prior
                .data
                .as_ref()
                .and_then(|d| d.pointer("/details/nativePresentationIntentDigest"))
                .and_then(serde_json::Value::as_str)
                != Some(fingerprint.as_str())
                || prior
                    .data
                    .as_ref()
                    .and_then(|d| d.pointer("/details/nativePresentationInvocation"))
                    != Some(&invocation_value)
            {
                return Err(presentation_conflict(intent));
            }
            return Ok(NativePresentationAcknowledgement::Recorded {
                provenance_id: prior.id.clone(),
                occurred_at: prior.timestamp.clone(),
            });
        }
        let Some(task) = record
            .process
            .active_tasks
            .iter()
            .find(|task| task.task_id == intent.task_id)
        else {
            return Ok(NativePresentationAcknowledgement::Obsolete {
                reason: "task no longer active".into(),
            });
        };
        if task.status != ActiveTaskStatus::Assigned
            || task.assigned_actor.as_deref() != Some(intent.assigned_actor.as_str())
            || task.updated_at != intent.task_updated_at
        {
            return Ok(NativePresentationAcknowledgement::Obsolete {
                reason: "task assignment or generation changed".into(),
            });
        }
        let expiry = crate::store::validate_task_lifetime(
            task.expires_at.as_deref(),
            task.expires_with_timer.as_deref(),
        )?;
        if expiry.is_some_and(|expiry| self.clock.now_ms() >= expiry) {
            return Ok(NativePresentationAcknowledgement::Obsolete {
                reason: "task expired".into(),
            });
        }
        self.require_native_dependency(NativeReadOnlyDependency::AccessControl)?;
        super::tasks::authorize_actor(self.access_control.as_ref(), task, &intent.assigned_actor)?;
        if serde_json::to_value(task.context.as_ref()).map_err(json_error)?
            != serde_json::to_value(Some(&intent.context)).map_err(json_error)?
            || creation_witness(&record, &intent.task_id)?.id != intent.task_created_record_id
            || serde_json::to_value(artifact_identity(&intent.context)).map_err(json_error)?
                != serde_json::to_value(&intent.artifacts).map_err(json_error)?
        {
            return Err(presentation_conflict(intent));
        }
        let NativePresentationInvocationOutcome::Invoked { occurred_at, .. } = invocation else {
            return Ok(NativePresentationAcknowledgement::RecoveryRequired {
                intent_id: intent.intent_id.clone(),
                attempt_id: attempt_id.clone(),
            });
        };
        let occurred_ms = super::parse_timestamp(occurred_at)?;
        if occurred_ms > self.clock.now_ms()
            || occurred_ms < super::parse_timestamp(&intent.task_updated_at)?
        {
            return Err(presentation_conflict(intent));
        }
        let mut provenance = ProvenanceRecord::task_lifecycle(
            &minter,
            ProvenanceKind::TaskPresented,
            &intent.task_id,
            Some(&intent.assigned_actor),
            Some(serde_json::json!({
                "definitionUrl":intent.context.definition_url,"definitionVersion":intent.context.definition_version,
                "nativePresentationIntentId":intent.intent_id,"nativePresentationIntentDigest":fingerprint,
                "taskCreatedRecordId":intent.task_created_record_id,"nativePresentationInvocation":invocation_value,
            })),
        );
        provenance.timestamp = occurred_at.clone();
        let kernel = self.resolver.resolve_kernel(
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        super::provenance::populate_process_provenance_fields(
            std::slice::from_mut(&mut provenance),
            &kernel,
            &record.process,
        )?;
        let acknowledgement = NativePresentationAcknowledgement::Recorded {
            provenance_id: provenance.id.clone(),
            occurred_at: provenance.timestamp.clone(),
        };
        record.process.provenance_position += 1;
        record.provenance_log.push(provenance);
        record.process.updated_at = super::format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record)?;
        Ok(acknowledgement)
    }
}

fn matching_recovery(
    command: &NativeRuntimeCommand,
    records: &[IntakeRecord],
) -> Vec<NativeRecoveryReference> {
    let NativeRuntimeCommand::AcceptIntake {
        binding, request, ..
    } = command
    else {
        return vec![];
    };
    records
        .iter()
        .filter(|record| {
            record.binding == *binding
                && record.request == *request
                && matches!(
                    record.status,
                    IntakeRecordStatus::Pending | IntakeRecordStatus::Prepared
                )
        })
        .map(|record| NativeRecoveryReference::Intake {
            binding: record.binding.clone(),
            intake_id: record.intake_id.clone(),
            status: record.status,
        })
        .collect()
}
fn require_token(token: &str) -> Result<(), RuntimeError> {
    if token.trim().is_empty() {
        Err(NativePreparationError::InvalidSnapshot {
            reason: "required stable key is blank".into(),
        }
        .into())
    } else {
        Ok(())
    }
}
fn json_error(error: serde_json::Error) -> RuntimeError {
    NativePreparationError::InvalidSnapshot {
        reason: error.to_string(),
    }
    .into()
}
fn presentation_conflict(intent: &PendingNativePresentationIntent) -> RuntimeError {
    NativePreparationError::PresentationConflict {
        intent_id: intent.intent_id.clone(),
    }
    .into()
}
fn creation_witness<'a>(
    record: &'a RuntimeRecord,
    task_id: &str,
) -> Result<&'a ProvenanceRecord, RuntimeError> {
    let mut witnesses = record.provenance_log.iter().filter(|p| {
        p.record_kind == ProvenanceKind::TaskCreated
            && p.data
                .as_ref()
                .and_then(|d| d.get("taskId"))
                .and_then(serde_json::Value::as_str)
                == Some(task_id)
    });
    let witness = witnesses
        .next()
        .ok_or_else(|| NativePreparationError::InvalidSnapshot {
            reason: "presentation task lacks original TaskCreated witness".into(),
        })?;
    if witnesses.next().is_some()
        || !stack_common_typeid::is_valid_type_id(
            &witness.id,
            Some(stack_common_typeid::PROVENANCE_PREFIX),
        )
    {
        return Err(NativePreparationError::InvalidSnapshot {
            reason: "presentation task has invalid or ambiguous TaskCreated witness".into(),
        }
        .into());
    }
    Ok(witness)
}
fn artifact_identity(context: &FormspecTaskContext) -> NativePresentationArtifactIdentity {
    NativePresentationArtifactIdentity {
        binding: context.binding.clone(),
        definition_url: context.definition_url.clone(),
        definition_version: context.definition_version.clone(),
        prefill_mapping_ref: context.prefill_mapping_ref.clone(),
        response_mapping_ref: context.response_mapping_ref.clone(),
        response_artifact_id: None,
    }
}
fn make_presentation_intent(
    key: &NativeCommandKey,
    ordinal: u64,
    context: FormspecTaskContext,
    record: &RuntimeRecord,
) -> Result<PendingNativePresentationIntent, RuntimeError> {
    let task = record
        .process
        .active_tasks
        .iter()
        .find(|task| task.task_id == context.task_id)
        .ok_or_else(|| NativePreparationError::InvalidSnapshot {
            reason: "prepared presentation task missing".into(),
        })?;
    let intent_id = presentation_id(key, &context.process_id, &context.task_id, ordinal);
    Ok(PendingNativePresentationIntent {
        intent_id,
        command_key: key.clone(),
        ordinal,
        process_id: context.process_id.clone(),
        task_id: context.task_id.clone(),
        task_created_record_id: creation_witness(record, &context.task_id)?.id.clone(),
        assigned_actor: context.assigned_actor.clone(),
        task_updated_at: task.updated_at.clone(),
        artifacts: artifact_identity(&context),
        context,
    })
}
fn validate_intent_id(intent: &PendingNativePresentationIntent) -> Result<(), RuntimeError> {
    require_token(&intent.command_key.0)?;
    if intent.intent_id
        != presentation_id(
            &intent.command_key,
            &intent.process_id,
            &intent.task_id,
            intent.ordinal,
        )
        || intent.context.process_id != intent.process_id
        || intent.context.task_id != intent.task_id
        || intent.context.assigned_actor != intent.assigned_actor
    {
        return Err(presentation_conflict(intent));
    }
    Ok(())
}
/// SHA-256 of domain plus length-prefixed UTF-8 tuple fields and a big-endian ordinal.
fn presentation_id(
    key: &NativeCommandKey,
    process_id: &str,
    task_id: &str,
    ordinal: u64,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"wos.native-presentation.v1\0");
    for value in [key.0.as_str(), process_id, task_id, "presentTask"] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    hash.update(ordinal.to_be_bytes());
    format!("sha256:{:x}", hash.finalize())
}
fn stable_json_digest(value: &serde_json::Value) -> String {
    fn sorted(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => serde_json::Value::Object(
                object
                    .iter()
                    .map(|(k, v)| (k.clone(), sorted(v)))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            serde_json::Value::Array(array) => {
                serde_json::Value::Array(array.iter().map(sorted).collect())
            }
            other => other.clone(),
        }
    }
    // Value serialization is infallible: no map-key coercion or non-JSON numbers.
    let bytes = sorted(value).to_string();
    format!("sha256:{:x}", Sha256::digest(bytes.as_bytes()))
}

struct PreparationResolver {
    inner: Box<dyn super::ResolveDocumentsDyn + Send + Sync>,
    policy: NativePreparationPolicy,
}
impl super::ResolveDocumentsDyn for PreparationResolver {
    fn resolve_kernel(
        &self,
        url: &str,
        version: &str,
    ) -> Result<wos_core::model::kernel::KernelDocument, RuntimeError> {
        self.policy.dependency(NativeReadOnlyDependency::Resolver)?;
        let kernel = self.inner.resolve_kernel(url, version)?;
        if kernel
            .actors
            .iter()
            .any(|actor| actor.kind == wos_core::model::kernel::ActorKind::Agent)
        {
            return Err(NativePreparationError::UnsupportedEffect {
                effect: "agent participation".into(),
            }
            .into());
        }
        Ok(kernel)
    }
}
struct PreparationValidator {
    inner: Box<dyn super::ValidateContractsDyn + Send + Sync>,
    policy: NativePreparationPolicy,
}
impl super::ValidateContractsDyn for PreparationValidator {
    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<wos_core::traits::ValidationResult, RuntimeError> {
        self.policy
            .dependency(NativeReadOnlyDependency::ContractValidator)?;
        self.inner.validate(contract_ref, data)
    }
}
