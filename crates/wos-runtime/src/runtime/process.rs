// Rust guideline compliant 2026-02-21

//! Workflow process lifecycle commands for the reference runtime.
//!
//! This module owns the durable command methods that create, load, enqueue,
//! and expose append windows for workflow processes. The methods still operate
//! on `WosRuntime`, but keeping them separate from the type declarations makes
//! the adapter boundary easier to audit. Every persisted `RuntimeRecord` carries
//! `case_ledger_id` via its inner `WorkflowProcess`; the runtime keys command
//! input on `process_id` while the storage row binds the durable case-ledger
//! foreign key — see case-boundary report §4.3 and ADR-0093 §5.

use super::process_minter;
use stack_common_typeid::{self as typeid, Tenant, TypeIdMinter};
use wos_core::eval::{Evaluator, validate_migration_configuration};
use wos_core::instance::{InstanceStatus, PendingEvent, WorkflowProcess};
use wos_core::{InstanceMigratedInput, ProvenanceRecord};

use crate::store::{RuntimeRecord, StoreError};
use wos_events::custody::{CustodyAppendContext, CustodyAppendInput};

use super::timers::{
    annotate_timer_created_with_calendar_version, annotate_timer_created_with_convergence_error,
    timers_to_state,
};
use super::{
    CreateProcessRequest, MigrationMap, MigrationOutcome, RuntimeError, WosRuntime,
    format_timestamp, populate_provenance_record_fields, stamp_custody_receipt, stamp_provenance,
};

/// What an operation does to an instance, for the Kernel S11.5 status table.
#[derive(Debug, Clone, Copy)]
pub enum StatusUse {
    /// Evaluates events, fires deadlines or acts on tasks: the named operation.
    Evaluate(&'static str),
    /// Queues an event without evaluating it.
    Queue,
    /// Changes the governing definition (Kernel S11.2).
    Migrate,
    /// Suspends, resumes or terminates (Kernel S11.3).
    Lifecycle(wos_core::LifecycleControl),
}

/// Whether `status` admits `operation` (Kernel S11.3, S11.5): suspend needs `active`, resume
/// needs `suspended`, terminate needs `active` or `suspended`; a suspended instance evaluates
/// nothing but may queue; a completed or otherwise final instance admits nothing. Other
/// statuses keep every operation. Records of work already committed (custody receipts,
/// presentation outcomes) are not operations on the instance and are never gated.
#[must_use]
pub fn status_admits(status: InstanceStatus, operation: StatusUse) -> bool {
    use wos_core::LifecycleControl as Control;
    match (status, operation) {
        (InstanceStatus::Active, StatusUse::Lifecycle(control)) => control != Control::Resume,
        (InstanceStatus::Suspended, StatusUse::Lifecycle(control)) => control != Control::Suspend,
        (_, StatusUse::Lifecycle(_)) => false,
        (InstanceStatus::Suspended, use_) => matches!(use_, StatusUse::Queue),
        (
            InstanceStatus::Completed
            | InstanceStatus::Terminated
            | InstanceStatus::Declined
            | InstanceStatus::Voided
            | InstanceStatus::Expired,
            _,
        ) => false,
        (InstanceStatus::Active | InstanceStatus::Migrating | InstanceStatus::Stalled, _) => true,
    }
}

/// Refuses what the instance's status forbids ([`status_admits`]).
///
/// # Errors
/// Returns [`RuntimeError::InstanceStatusConflict`] naming the status and the operation.
pub fn require_status(process: &WorkflowProcess, operation: StatusUse) -> Result<(), RuntimeError> {
    use wos_core::LifecycleControl as Control;
    if status_admits(process.status, operation) {
        return Ok(());
    }
    let operation = match operation {
        StatusUse::Evaluate(name) => name,
        StatusUse::Queue => "queueing an event",
        StatusUse::Migrate => "migration",
        StatusUse::Lifecycle(Control::Suspend) => "suspend",
        StatusUse::Lifecycle(Control::Resume) => "resume",
        StatusUse::Lifecycle(Control::Terminate) => "terminate",
    };
    Err(RuntimeError::InstanceStatusConflict {
        status: serde_json::to_value(process.status)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
        operation,
    })
}

/// Returns the operation's non-blank reason, refusing its absence when the governing
/// workflow's `lifecycle.<operation>.reasonRequired` asks for one (Kernel S11.3; resume
/// takes no reason). The workflow, not the caller, decides.
///
/// # Errors
/// Returns [`RuntimeError::LifecycleReasonRequired`] for a required but blank or absent reason.
pub fn require_lifecycle_reason<'a>(
    kernel: &wos_core::KernelDocument,
    control: wos_core::LifecycleControl,
    reason: Option<&'a str>,
) -> Result<Option<&'a str>, RuntimeError> {
    use wos_core::LifecycleControl as Control;
    let reason = reason.filter(|reason| !reason.trim().is_empty());
    let (policy, operation) = match control {
        Control::Suspend => (kernel.lifecycle.suspend, "suspend"),
        Control::Terminate => (kernel.lifecycle.terminate, "terminate"),
        Control::Resume => (None, "resume"),
    };
    if policy.is_some_and(|policy| policy.reason_required) && reason.is_none() {
        return Err(RuntimeError::LifecycleReasonRequired(operation));
    }
    Ok(reason)
}

/// Refuses an instance that has queued events or a due timer or task expiry at `now_ms`: an
/// inline evaluation would take queued work's turn or fire a deadline its own wakeup owns.
///
/// # Errors
/// Returns [`RuntimeError::InstanceStatusConflict`] with status `busy`.
pub fn require_idle(process: &WorkflowProcess, now_ms: u64) -> Result<(), RuntimeError> {
    if process.pending_events.is_empty() && !deadline_due(process, now_ms)? {
        return Ok(());
    }
    Err(RuntimeError::InstanceStatusConflict {
        status: "busy".into(),
        operation: "an inline evaluation while queued work or a due deadline is pending",
    })
}

/// Whether a timer or task expiry is due at `now_ms` (what the next drain would fire).
fn deadline_due(process: &WorkflowProcess, now_ms: u64) -> Result<bool, RuntimeError> {
    for timer in &process.timers {
        if super::parse_timestamp(&timer.deadline)? <= now_ms {
            return Ok(true);
        }
    }
    for task in &process.active_tasks {
        let expiry = crate::store::validate_task_lifetime(
            task.expires_at.as_deref(),
            task.expires_with_timer.as_deref(),
        )?;
        if expiry.is_some_and(|expiry| now_ms >= expiry) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn apply_migration_map(
    case_state: &mut serde_json::Value,
    map: &MigrationMap,
) -> Result<(), RuntimeError> {
    let obj = case_state.as_object_mut().ok_or_else(|| {
        RuntimeError::MigrationRejected(
            "caseState must be a JSON object for migration map application".into(),
        )
    })?;
    for key in &map.field_removals {
        obj.remove(key);
    }
    for (old_key, new_key) in &map.field_renames {
        if let Some(value) = obj.remove(old_key) {
            obj.insert(new_key.clone(), value);
        }
    }
    for (key, default_value) in &map.field_defaults {
        obj.entry(key.clone())
            .or_insert_with(|| default_value.clone());
    }
    for (field, kind) in &map.field_coercions {
        let Some(value) = obj.get_mut(field) else {
            continue;
        };
        match kind.as_str() {
            "number" => {
                let n = match &*value {
                    serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| {
                        RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: field `{field}` is not a finite number"
                        ))
                    })?,
                    serde_json::Value::String(s) => s.parse::<f64>().map_err(|_| {
                        RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: cannot parse field `{field}`"
                        ))
                    })?,
                    _ => {
                        return Err(RuntimeError::MigrationRejected(format!(
                            "fieldCoercion number: unsupported JSON type on field `{field}`"
                        )));
                    }
                };
                if !n.is_finite() {
                    return Err(RuntimeError::MigrationRejected(format!(
                        "fieldCoercion number: field `{field}` must be finite"
                    )));
                }
                *value = serde_json::json!(n);
            }
            "string" => {
                let s = match &*value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                *value = serde_json::Value::String(s);
            }
            "boolean" => {
                let b = match &*value {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::String(s) if s.eq_ignore_ascii_case("true") => true,
                    serde_json::Value::String(s) if s.eq_ignore_ascii_case("false") => false,
                    _ => {
                        return Err(RuntimeError::MigrationRejected(format!(
                            "fieldCoercion boolean: cannot coerce field `{field}`"
                        )));
                    }
                };
                *value = serde_json::Value::Bool(b);
            }
            other => {
                return Err(RuntimeError::MigrationRejected(format!(
                    "unknown fieldCoercion `{other}` for field `{field}`"
                )));
            }
        }
    }
    Ok(())
}

fn requested_type_id(value: &str) -> Option<&str> {
    if typeid::is_valid_type_id(value, None) {
        Some(value)
    } else {
        WorkflowProcess::extract_urn_type_id(value)
    }
}

impl WosRuntime {
    /// Create and persist a new workflow process.
    ///
    /// # Errors
    /// Returns an error when kernel resolution, evaluation, task staging, or
    /// persistence fails.
    pub fn create_process(
        &mut self,
        request: CreateProcessRequest,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_process_inner(request, None, None, None)
    }

    /// Create and persist a new workflow process bound to an existing case ledger.
    ///
    /// # Errors
    /// Returns an error when the case ledger id is invalid, its tenant disagrees
    /// with the process id, or normal process creation fails.
    pub fn create_process_bound_to_case(
        &mut self,
        request: CreateProcessRequest,
        case_ledger_id: String,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_process_inner(request, Some(case_ledger_id), None, None)
    }

    /// Create a process with explicit host-resolved human assignments.
    ///
    /// Bindings retain attribution, never current authority. Missing human
    /// resolution refuses instead of falling back to an authored local ID.
    ///
    /// # Errors
    /// Returns an error for invalid bindings or normal process creation failures.
    pub fn create_process_with_actor_bindings(
        &mut self,
        request: CreateProcessRequest,
        bindings: Vec<wos_core::instance::HumanActorBinding>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_process_inner(request, None, Some(bindings), None)
    }

    /// Create a case-bound process with explicit human assignments.
    ///
    /// # Errors
    /// Returns an error for invalid bindings, case identity or process creation.
    pub fn create_process_bound_to_case_with_actor_bindings(
        &mut self,
        request: CreateProcessRequest,
        case_ledger_id: String,
        bindings: Vec<wos_core::instance::HumanActorBinding>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_process_inner(request, Some(case_ledger_id), Some(bindings), None)
    }

    pub(super) fn create_process_inner(
        &mut self,
        request: CreateProcessRequest,
        bound_case_ledger_id: Option<String>,
        actor_bindings: Option<Vec<wos_core::instance::HumanActorBinding>>,
        initial_initiating_actor_id: Option<&str>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        self.create_process_with_intake_admission(
            request,
            bound_case_ledger_id,
            actor_bindings,
            initial_initiating_actor_id,
            None,
        )
    }
    pub(super) fn create_process_with_intake_admission(
        &mut self,
        request: CreateProcessRequest,
        bound_case_ledger_id: Option<String>,
        actor_bindings: Option<Vec<wos_core::instance::HumanActorBinding>>,
        initial_initiating_actor_id: Option<&str>,
        intake: Option<&crate::intake::IntakeAcceptanceRequest>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let CreateProcessRequest {
            process_id,
            tenant: requested_tenant,
            definition_url,
            definition_version,
            initial_case_state,
        } = request;

        // ADR 0068 D-1.2: resolve the authoritative tenant.
        // 1. If the caller supplied an explicit tenant, validate it.
        // 2. If process_id is a valid TypeID, extract the prefix.
        // 3. If both are present, they must match.
        // 4. Otherwise refuse: a tenant is never defaulted here.
        let type_id_tenant = Tenant::of(&process_id).map(|tenant| tenant.to_string());
        let bound_case_ledger_id = bound_case_ledger_id
            .map(|value| {
                let normalized = requested_type_id(&value)
                    .map(str::to_string)
                    .unwrap_or_else(|| value.clone());
                if !typeid::is_valid_type_id(&normalized, Some(typeid::CASE_PREFIX)) {
                    return Err(RuntimeError::MigrationRejected(format!(
                        "case ledger id `{value}` is not a case TypeID"
                    )));
                }
                Ok(normalized)
            })
            .transpose()?;
        let bound_case_tenant = bound_case_ledger_id
            .as_deref()
            .and_then(Tenant::of)
            .map(|tenant| tenant.to_string());
        let inferred_tenant = type_id_tenant.as_ref().or_else(|| {
            if process_id.trim().is_empty() {
                bound_case_tenant.as_ref()
            } else {
                None
            }
        });
        let tenant = match (requested_tenant, inferred_tenant) {
            (Some(explicit), Some(prefix)) => {
                let tenant = Tenant::parse(&explicit)
                    .map_err(|_| RuntimeError::TenantInvalid(explicit.clone()))?;
                if tenant.as_str() != prefix {
                    return Err(RuntimeError::TenantMismatch {
                        explicit,
                        type_id_prefix: prefix.clone(),
                    });
                }
                tenant
            }
            (Some(explicit), None) => Tenant::parse(&explicit)
                .map_err(|_| RuntimeError::TenantInvalid(explicit.clone()))?,
            (None, Some(prefix)) => {
                Tenant::parse(prefix).map_err(|_| RuntimeError::TenantInvalid(prefix.clone()))?
            }
            (None, None) => return Err(RuntimeError::TenantUnresolved),
        };
        let minter = TypeIdMinter::new(tenant.clone());
        let bound_case_ledger_id = bound_case_ledger_id
            .map(|value| {
                if let Some(prefix) = Tenant::of(&value)
                    && prefix != tenant
                {
                    return Err(RuntimeError::TenantMismatch {
                        explicit: tenant.to_string(),
                        type_id_prefix: prefix.to_string(),
                    });
                }
                Ok(value)
            })
            .transpose()?;

        let process_id = if process_id.trim().is_empty() {
            minter.mint_process_id()
        } else if let Some(input_type_id) = requested_type_id(&process_id) {
            if !typeid::is_valid_type_id(input_type_id, Some(typeid::PROCESS_PREFIX)) {
                return Err(RuntimeError::MigrationRejected(format!(
                    "process id `{process_id}` is not a process TypeID"
                )));
            }
            input_type_id.to_string()
        } else {
            return Err(RuntimeError::MigrationRejected(format!(
                "process id `{process_id}` is not a WOS TypeID"
            )));
        };
        let case_ledger_id = bound_case_ledger_id.unwrap_or_else(|| minter.mint_case_ledger_id());
        let kernel = self
            .resolver
            .resolve_kernel(&definition_url, &definition_version)?;
        // The immutable selected workflow fixes every task form's own version,
        // including tasks created by later transitions.
        super::actions::validate_task_contract_pins(&kernel)?;
        super::actor_bindings::validate_binding_definition(
            actor_bindings.as_deref(),
            &kernel,
            &definition_url,
            &definition_version,
        )?;
        self.validate_governance_selection(&kernel)?;
        let mut evaluator = Evaluator::with_time_and_case_state_and_actor(
            kernel.clone(),
            &minter,
            now_ms,
            initial_case_state.as_ref(),
            initial_initiating_actor_id,
        )
        .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;

        let (timer_states, convergence_error_ids) =
            timers_to_state(evaluator.timers(), self.business_calendar.as_ref())?;
        let instance = WorkflowProcess {
            process_id,
            case_ledger_id,
            tenant: tenant.to_string(),
            definition_url,
            definition_version,
            actor_bindings,
            configuration: evaluator.configuration().active_states().to_vec(),
            case_state: evaluator.case_state_json(),
            provenance_position: 0,
            next_task_sequence: 0,
            timers: timer_states,
            active_tasks: Vec::new(),
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: InstanceStatus::Active,
            stalled_since: None,
            resumed_at: None,
            decline_reason: None,
            voided_by: None,
            voided_at: None,
            expired_at: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            pending_outbound_events: Default::default(),
            next_deferred_callback_sequence: 0,
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            extensions: Default::default(),
        };
        let mut record = RuntimeRecord::new(instance);
        let mut appended_provenance = evaluator.provenance().records().to_vec();
        // Qualify the original applicant act before actions, presentation or storage.
        if let Some(intake) = intake
            && let Some(affirmation) =
                self.admit_original_intake_signature(intake, &mut record, false)?
        {
            appended_provenance.push(affirmation);
        }
        // Annotate any timers created during instance initialization with calendarVersion.
        if let Some(cal) = &self.business_calendar {
            annotate_timer_created_with_calendar_version(&mut appended_provenance, cal);
        }
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(
            &mut appended_provenance,
            &convergence_error_ids,
        );
        let actions = evaluator.take_executed_actions();
        let guards = evaluator.take_guard_evaluations();
        appended_provenance.extend(self.reference_preflight(
            &kernel,
            &mut record,
            &evaluator,
            &actions,
            &guards,
            None,
            &now_iso,
        )?);
        let initial_event = wos_core::instance::PendingEvent {
            event: "wos.initial".into(),
            actor_id: initial_initiating_actor_id.map(str::to_owned),
            data: None,
            idempotency_token: None,
            timestamp: now_iso.clone(),
            origin: None,
        };
        let initial_provenance =
            self.check_governance_result(crate::governance::RuntimeGovernanceResultContext {
                kernel: &kernel,
                prior: &record,
                event: &initial_event,
                proposed_process: &record.process,
                transitions: evaluator.transitions(),
                provenance: &appended_provenance,
                now_ms,
                now_iso: &now_iso,
            })?;
        appended_provenance.extend(initial_provenance);
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);
        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);
        super::provenance::populate_process_provenance_fields(
            &mut appended_provenance,
            &kernel,
            &record.process,
        )?;
        stamp_provenance(&mut appended_provenance, &now_iso);
        record.process.provenance_position = appended_provenance.len() as u64;
        record.provenance_log.extend(appended_provenance);
        // Capture once in the same atomic write as the created process and tasks.
        record.creation = Some(crate::intake::ProcessCreationSnapshot::capture(
            &record.process,
            &kernel,
        )?);
        if let Some(cap) = self.reference_due_process.as_ref() {
            let (raw, execution) = super::governance::admission::validate(cap, &kernel)
                .map_err(RuntimeError::Governance)?;
            record
                .creation
                .as_mut()
                .ok_or(RuntimeError::Governance(
                    wos_core::model::due_process::RefusalCode::StoreRejected,
                ))?
                .reference_due_process = Some(crate::intake::DueProcessCreationWitness {
                definition_sha256: super::governance::admission::digest(&raw),
                execution,
            });
        }
        self.store.create_record(record.clone())?;

        self.deliver_pending_presentations(&pending_presentations)?;

        let _ = (created_task_ids, emitted_events);
        Ok(record.process)
    }

    /// Load the canonical workflow process state.
    ///
    /// # Errors
    /// Returns an error when the process cannot be found or loaded.
    pub fn load_process(&self, process_id: &str) -> Result<WorkflowProcess, RuntimeError> {
        Ok(self.load_record_by_process_or_case_ref(process_id)?.process)
    }

    /// Read the service outputs saved by completed native operations.
    ///
    /// # Errors
    /// Returns an error when the process cannot be resolved or its record cannot be read.
    pub fn load_step_results(
        &self,
        process_id: &str,
    ) -> Result<Vec<crate::store::StepResultRecord>, RuntimeError> {
        Ok(self
            .load_record_by_process_or_case_ref(process_id)?
            .step_results)
    }

    /// Return the `process_id` of every workflow process bound to the given
    /// case ledger.
    ///
    /// Case-ledger N:1 traversal delegated to the configured store. Returns an
    /// empty vector when no processes are bound, including when the case ledger
    /// does not exist. `InMemoryStore` uses its case-ledger index, so traversal
    /// is O(processes-per-case); callers MUST NOT depend on ordering.
    pub fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
        self.store.processes_for_case(case_ledger_id)
    }

    pub(super) fn load_single_record_by_case_ledger_id(
        &self,
        case_ledger_id: &str,
    ) -> Result<RuntimeRecord, RuntimeError> {
        let process_ids = self.store.processes_for_case(case_ledger_id);
        match process_ids.as_slice() {
            [] => Err(RuntimeError::from(StoreError::NotFound(
                case_ledger_id.to_string(),
            ))),
            [process_id] => self
                .store
                .load_record(process_id)
                .map_err(RuntimeError::from),
            _ => Err(RuntimeError::CaseLedgerConflict(
                multiple_processes_for_case_message(case_ledger_id, &process_ids),
            )),
        }
    }

    pub(super) fn load_record_by_process_or_case_ref(
        &self,
        id: &str,
    ) -> Result<RuntimeRecord, RuntimeError> {
        match self.store.load_record(id) {
            Ok(record) => return Ok(record),
            Err(crate::store::StoreError::NotFound(_)) => {}
            Err(error) => return Err(RuntimeError::from(error)),
        }

        let normalized = WorkflowProcess::extract_urn_type_id(id).unwrap_or(id);
        if normalized != id {
            match self.store.load_record(normalized) {
                Ok(record) => return Ok(record),
                Err(crate::store::StoreError::NotFound(_)) => {}
                Err(error) => return Err(RuntimeError::from(error)),
            }
        }

        self.load_single_record_by_case_ledger_id(normalized)
    }

    /// Append an event to the instance queue.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found, timestamping fails,
    /// or persistence fails.
    pub fn enqueue_event(
        &mut self,
        process_id: &str,
        mut event: PendingEvent,
    ) -> Result<(), RuntimeError> {
        if event.origin.is_some() {
            return Err(RuntimeError::UnsupportedAction(
                "external enqueue cannot supply native event origin".into(),
            ));
        }
        let mut record = self.store.load_record(process_id)?;
        if event.timestamp.is_empty() {
            event.timestamp = format_timestamp(self.clock.now_ms())?;
        }
        require_status(&record.process, StatusUse::Queue)?;
        record.process.pending_events.push(event);
        record.process.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record.clone())?;
        Ok(())
    }

    /// Evaluates one external event now (Kernel S11.3 `processEvent`) and returns that step.
    ///
    /// The instance must be `active`, with no queued event and no due timer or task expiry:
    /// the submitted event is then the one evaluated, earlier queued work keeps its own
    /// durable continuation, and deadlines keep their own owner.
    ///
    /// # Errors
    /// Returns [`RuntimeError::InstanceStatusConflict`] for a non-active, busy or overdue
    /// instance, and the enqueue and drain errors otherwise.
    pub fn process_event(
        &mut self,
        process_id: &str,
        event: PendingEvent,
    ) -> Result<super::DrainOnceResult, RuntimeError> {
        let record = self.store.load_record(process_id)?;
        require_status(&record.process, StatusUse::Evaluate("event processing"))?;
        require_idle(&record.process, self.clock.now_ms())?;
        self.enqueue_event(process_id, event)?;
        self.drain_once(process_id)
    }

    /// Suspends, resumes or terminates an instance (Kernel S11.3) under the S11.5 table, and
    /// records the operation's Facts-tier record with its actor and reason.
    ///
    /// # Errors
    /// Returns [`RuntimeError::InstanceStatusConflict`] when the status forbids the
    /// operation, [`RuntimeError::LifecycleReasonRequired`] when the governing workflow
    /// requires a reason the request lacks, and resolution or persistence errors.
    pub fn control_lifecycle(
        &mut self,
        process_id: &str,
        control: wos_core::LifecycleControl,
        actor_id: &str,
        reason: Option<&str>,
    ) -> Result<WorkflowProcess, RuntimeError> {
        use wos_core::LifecycleControl as Control;
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let mut record = self.store.load_record(process_id)?;
        require_status(&record.process, StatusUse::Lifecycle(control))?;
        let kernel = self.resolver.resolve_kernel(
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        let reason = require_lifecycle_reason(&kernel, control, reason)?;
        let minter = process_minter(&record.process)?;
        let mut appended = vec![ProvenanceRecord::instance_lifecycle(
            &minter,
            wos_core::InstanceLifecycleInput {
                control,
                actor_id,
                reason,
            },
        )];
        record.process.status = match control {
            Control::Suspend => InstanceStatus::Suspended,
            Control::Resume => InstanceStatus::Active,
            Control::Terminate => InstanceStatus::Terminated,
        };
        if control == Control::Resume {
            record.process.resumed_at = Some(now_iso.clone());
        }
        super::provenance::populate_process_provenance_fields(
            &mut appended,
            &kernel,
            &record.process,
        )?;
        stamp_provenance(&mut appended, &now_iso);
        record.process.updated_at = now_iso;
        record.provenance_log.extend(appended);
        record.process.provenance_position = record.provenance_log.len() as u64;
        let process = record.process.clone();
        self.store.save_record(record)?;
        Ok(process)
    }

    /// Load ADR-0061 custody append inputs by provenance-log window.
    ///
    /// # Errors
    /// Returns an error when the instance cannot be found or when a provenance
    /// record cannot be canonicalized into custody append input form.
    pub fn load_custody_append_window(
        &self,
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<CustodyAppendInput>, RuntimeError> {
        let record = self.load_record_by_process_or_case_ref(process_id)?;
        record
            .provenance_log
            .iter()
            .enumerate()
            .skip(cursor)
            .take(limit)
            .map(|(position, provenance)| {
                let metadata = context.metadata_for_provenance_record(
                    &record.process.case_ledger_id,
                    position,
                    provenance,
                )?;
                CustodyAppendInput::from_provenance_record(provenance, &context, metadata)
                    .map_err(RuntimeError::from)
            })
            .collect()
    }

    /// Stamps the Trellis custody receipt onto the matching provenance record.
    ///
    /// # Errors
    /// Returns an error when the instance or provenance record cannot be
    /// located, or when persistence fails.
    pub fn apply_custody_receipt(
        &mut self,
        process_id: &str,
        record_id: &str,
        receipt: wos_events::custody::CustodyAppendReceipt,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(process_id)?;
        let Some(provenance) = record
            .provenance_log
            .iter_mut()
            .find(|entry| entry.id == record_id)
        else {
            return Err(RuntimeError::ProvenanceRecordNotFound(
                record_id.to_string(),
            ));
        };
        stamp_custody_receipt(provenance, &receipt)?;
        record.process.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record)?;
        Ok(())
    }

    /// Migrate a running instance to a new kernel `definitionVersion` for the
    /// same `definitionUrl` (Kernel S11.2, ADR 0083 D1).
    ///
    /// Validates active states against the target kernel, applies
    /// [`MigrationMap`] to `caseState`, appends one `instanceMigrated`
    /// provenance record, and persists atomically via [`RuntimeStore::save_record`].
    ///
    /// **Idempotency:** When `target_definition_version` already matches the
    /// instance's current version, this returns [`MigrationOutcome`] immediately
    /// with no store write and no new provenance. HTTP `Idempotency-Key` replay
    /// for successful version bumps is handled in `wos-server` (ADR 0083 D5).
    ///
    /// # Errors
    /// Returns [`RuntimeError::MigrationRejected`] when validation or map
    /// application fails; the instance is not modified.
    pub fn migrate(
        &mut self,
        process_id: &str,
        target_definition_version: &str,
        migration_map: MigrationMap,
        operator_actor_id: Option<&str>,
    ) -> Result<MigrationOutcome, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.store.load_record(process_id)?;
        require_status(&record.process, StatusUse::Migrate)?;
        let minter = process_minter(&record.process)?;
        if record.process.definition_version == target_definition_version {
            return Ok(MigrationOutcome {
                process_id: process_id.to_string(),
                previous_definition_version: record.process.definition_version.clone(),
                new_definition_version: target_definition_version.to_string(),
                migration_map,
            });
        }
        // Queued events were accepted under the source version; their continuations stay
        // pinned to it, so a version change waits for the queue to drain.
        if !record.process.pending_events.is_empty() {
            return Err(RuntimeError::InstanceStatusConflict {
                status: "busy".into(),
                operation: "migration while queued events are pending",
            });
        }

        if self.generic_governance.is_some() {
            return Err(RuntimeError::MigrationRejected(
                "generic governance requires explicit version requalification".into(),
            ));
        }

        if !record.process.pending_outbound_events.is_empty() {
            return Err(RuntimeError::MigrationRejected(
                "unresolved deferred callbacks require original definition interpretation".into(),
            ));
        }
        if record.process.actor_bindings.is_some() {
            return Err(RuntimeError::MigrationRejected(
                "bound human assignments require explicit definition requalification".into(),
            ));
        }
        let url = record.process.definition_url.clone();
        let previous_definition_version = record.process.definition_version.clone();
        let target_kernel = self
            .resolver
            .resolve_kernel(&url, target_definition_version)?;
        super::actions::validate_task_contract_pins(&target_kernel)
            .map_err(|error| RuntimeError::MigrationRejected(error.to_string()))?;
        validate_migration_configuration(&target_kernel, &record.process.configuration)
            .map_err(|e| RuntimeError::MigrationRejected(e.to_string()))?;

        let mut new_case_state = record.process.case_state.clone();
        apply_migration_map(&mut new_case_state, &migration_map)?;

        let migration_map_json = serde_json::to_value(&migration_map)
            .map_err(|e| RuntimeError::MigrationRejected(e.to_string()))?;
        let mut appended = vec![ProvenanceRecord::instance_migrated(
            &minter,
            InstanceMigratedInput {
                from_version: previous_definition_version.as_str(),
                to_version: target_definition_version,
                migration_map: migration_map_json,
                actor_id: operator_actor_id,
                context: None,
            },
        )];
        populate_provenance_record_fields(&mut appended, &target_kernel, target_definition_version);
        stamp_provenance(&mut appended, &now_iso);

        record.process.case_state = new_case_state;
        record.process.definition_version = target_definition_version.to_string();
        record.process.updated_at = now_iso;
        record.provenance_log.extend(appended);
        record.process.provenance_position = record.provenance_log.len() as u64;
        self.store.save_record(record)?;

        Ok(MigrationOutcome {
            process_id: process_id.to_string(),
            previous_definition_version,
            new_definition_version: target_definition_version.to_string(),
            migration_map,
        })
    }
}

fn multiple_processes_for_case_message(case_ledger_id: &str, process_ids: &[String]) -> String {
    format!(
        "case ledger `{case_ledger_id}` is bound to multiple processes ({}); use a process id or explicit disambiguation",
        process_ids.join(", ")
    )
}
