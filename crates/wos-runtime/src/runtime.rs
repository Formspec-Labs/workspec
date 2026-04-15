// Rust guideline compliant 2026-02-21

//! Runtime command surface for WOS processors.

use std::error::Error as StdError;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use semver::{Version, VersionReq};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use chrono::DateTime;
use wos_core::business_calendar::{next_business_moment, BusinessCalendarDocument, BusinessCalendarError};
use wos_core::eval::{Evaluator, ObservedAction, ObservedTransition};
use wos_core::instance::{
    ActiveTask, ActiveTaskStatus, CaseInstance, FormspecTaskContext, InstanceStatus, PendingEvent,
};
use wos_core::model::governance::DelegationScope;
use wos_core::model::kernel::{ActionKind, ImpactLevel, KernelDocument};
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};
use wos_core::timer::{max_tolerance_ms, tolerance_to_iso, Timer};
use wos_core::traits::{
    AccessControl, ContractValidator, DocumentResolver, ExternalService, TaskPresenter,
};

use crate::binding::{BindingError, BindingRegistry, SubmissionValidation};
use crate::integration::{IntegrationBinding, IntegrationProfileDocument};
use crate::integration_handlers::{
    dispatch_integration_binding, load_or_invoke_service_result, InvocationContext,
};
use crate::milestones::evaluate_milestones;
use crate::store::{
    ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, RuntimeStore,
    StoreError, TaskArtifact, TaskArtifactKind,
};

const COMPLETION_EVENT_EXTENSION_KEY: &str = "x-wos-runtime-completion-event";
const FAILURE_EVENT_EXTENSION_KEY: &str = "x-wos-runtime-failure-event";

/// Request for instance creation.
#[derive(Debug, Clone)]
pub struct CreateInstanceRequest {
    /// Stable WOS instance identifier.
    pub instance_id: String,
    /// Governing kernel URL.
    pub definition_url: String,
    /// Governing kernel version.
    pub definition_version: String,
    /// Initial case-state overrides.
    pub initial_case_state: Option<serde_json::Value>,
}

/// Single-step drain result.
#[derive(Debug, Clone, Default)]
pub struct DrainOnceResult {
    /// Event processed by this drain step, if any.
    pub processed_event: Option<String>,
    /// Idempotency token from the dequeued event, if any.
    pub processed_event_token: Option<String>,
    /// State transitions observed during evaluation.
    pub transitions: Vec<ObservedTransition>,
    /// Provenance appended by this step.
    pub provenance: Vec<ProvenanceRecord>,
    /// Task identifiers created during the step.
    pub created_task_ids: Vec<String>,
    /// Event names emitted during the step.
    pub emitted_events: Vec<String>,
}

/// Draft persistence result.
#[derive(Debug, Clone)]
pub struct PersistDraftResult {
    /// Stable artifact identifier for the draft.
    pub artifact_id: String,
}

/// Task submission outcome.
#[derive(Debug, Clone)]
pub enum TaskSubmissionResult {
    /// Submission completed successfully.
    Completed {
        /// Accepted response artifact identifier.
        artifact_id: String,
        /// Whether case state changed.
        case_mutated: bool,
        /// Completion event emitted, if any.
        emitted_event: Option<String>,
    },
    /// Submission reached a terminal failure.
    Failed {
        /// Machine-readable failure code.
        code: String,
        /// Failure event emitted, if any.
        emitted_event: Option<String>,
    },
    /// Submission was rejected before terminal processing.
    Rejected {
        /// Machine-readable rejection code.
        code: String,
    },
}

/// Runtime wall-clock abstraction.
pub trait Clock {
    /// Current Unix timestamp in milliseconds.
    fn now_ms(&self) -> u64;
}

/// Context for runtime companion-policy evaluation.
#[derive(Debug, Clone)]
pub struct RuntimeEventContext {
    /// Kernel active for the instance.
    pub kernel: KernelDocument,
    /// Instance state before the event is evaluated.
    pub instance: CaseInstance,
    /// Event dequeued for processing.
    pub event: PendingEvent,
}

/// Companion-policy decision for a runtime event.
#[derive(Debug, Clone)]
pub struct RuntimeEventDecision {
    /// Effective event to process, or `None` when blocked.
    pub event: Option<PendingEvent>,
    /// Provenance emitted by companion policy evaluation.
    pub provenance: Vec<ProvenanceRecord>,
}

impl RuntimeEventDecision {
    /// Continue with the original event and no companion provenance.
    pub fn proceed(event: PendingEvent) -> Self {
        Self {
            event: Some(event),
            provenance: Vec::new(),
        }
    }
}

/// Companion-policy hook evaluated before lifecycle processing.
pub trait CompanionPolicy {
    /// Evaluate the event against companion documents and return a decision.
    fn evaluate_event(
        &mut self,
        context: RuntimeEventContext,
    ) -> Result<RuntimeEventDecision, RuntimeError>;
}

#[derive(Debug, Clone, Copy, Default)]
struct NoopCompanionPolicy;

impl CompanionPolicy for NoopCompanionPolicy {
    fn evaluate_event(
        &mut self,
        context: RuntimeEventContext,
    ) -> Result<RuntimeEventDecision, RuntimeError> {
        Ok(RuntimeEventDecision::proceed(context.event))
    }
}

/// System wall-clock implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        duration.as_millis() as u64
    }
}

/// Errors returned by runtime commands.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Runtime store failed.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// Document resolution failed.
    #[error("document resolution failed: {0}")]
    Resolver(String),

    /// Task presentation failed.
    #[error("task presentation failed: {0}")]
    Presenter(String),

    /// External service invocation failed.
    #[error("external service failed: {0}")]
    Service(String),

    /// Integration profile processing failed.
    #[error("integration failed: {0}")]
    Integration(String),

    /// Contract validation failed.
    #[error("contract validation failed: {0}")]
    ContractValidation(String),

    /// Evaluator failed to process an event.
    #[error("evaluator failed: {0}")]
    Evaluator(String),

    /// Binding adapter failed.
    #[error(transparent)]
    Binding(#[from] BindingError),

    /// Required workflow metadata is absent.
    #[error("missing workflow metadata: {0}")]
    MissingMetadata(String),

    /// The referenced task was not found.
    #[error("active task not found: {0}")]
    TaskNotFound(String),

    /// The referenced contract was not found.
    #[error("contract not found: {0}")]
    ContractNotFound(String),

    /// The binding is unsupported by the runtime (free-form string error).
    #[error("binding unsupported: {0}")]
    UnsupportedBinding(String),

    /// The integration binding kind is not yet implemented by the runtime.
    #[error("integration binding kind unsupported: {0:?}")]
    UnsupportedBindingKind(crate::integration::IntegrationBindingKind),

    /// A kernel action cannot yet be handled by the runtime.
    #[error("action unsupported: {0}")]
    UnsupportedAction(String),

    /// The response status is invalid for the operation.
    #[error("invalid response status: {0}")]
    InvalidResponseStatus(String),

    /// The actor is not authorized for the operation.
    #[error("actor unauthorized: {0}")]
    Unauthorized(String),

    /// Timestamp conversion failed.
    #[error("timestamp conversion failed: {0}")]
    Clock(String),
}

trait ResolveDocumentsDyn {
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, RuntimeError>;
}

impl<T> ResolveDocumentsDyn for T
where
    T: DocumentResolver,
    T::Error: StdError + Send + Sync + 'static,
{
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, RuntimeError> {
        DocumentResolver::resolve_kernel(self, url, version)
            .map_err(|error| RuntimeError::Resolver(error.to_string()))
    }
}

trait PresentTasksDyn {
    fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), RuntimeError>;
    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError>;
}

impl<T> PresentTasksDyn for T
where
    T: TaskPresenter,
    T::Error: StdError + Send + Sync + 'static,
{
    fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), RuntimeError> {
        TaskPresenter::present_task(self, context)
            .map_err(|error| RuntimeError::Presenter(error.to_string()))
    }

    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        TaskPresenter::dismiss_task(self, task_id, reason)
            .map_err(|error| RuntimeError::Presenter(error.to_string()))
    }
}

pub(crate) trait InvokeServicesDyn {
    fn invoke(
        &self,
        service_ref: &str,
        input: &serde_json::Value,
        idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, RuntimeError>;
}

impl<T> InvokeServicesDyn for T
where
    T: ExternalService,
    T::Error: StdError + Send + Sync + 'static,
{
    fn invoke(
        &self,
        service_ref: &str,
        input: &serde_json::Value,
        idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, RuntimeError> {
        ExternalService::invoke(self, service_ref, input, idempotency_key)
            .map_err(|error| RuntimeError::Service(error.to_string()))
    }
}

pub(crate) trait ValidateContractsDyn {
    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<wos_core::traits::ValidationResult, RuntimeError>;
}

impl<T> ValidateContractsDyn for T
where
    T: ContractValidator,
    T::Error: StdError + Send + Sync + 'static,
{
    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<wos_core::traits::ValidationResult, RuntimeError> {
        ContractValidator::validate(self, contract_ref, data)
            .map_err(|error| RuntimeError::ContractValidation(error.to_string()))
    }
}

/// Generic WOS runtime.
pub struct WosRuntime {
    store: Box<dyn RuntimeStore>,
    resolver: Box<dyn ResolveDocumentsDyn>,
    presenter: Box<dyn PresentTasksDyn>,
    access_control: Box<dyn AccessControl>,
    service: Box<dyn InvokeServicesDyn>,
    validator: Box<dyn ValidateContractsDyn>,
    clock: Box<dyn Clock>,
    companion_policy: Box<dyn CompanionPolicy>,
    integration_profile: Option<IntegrationProfileDocument>,
    /// Attached business calendar for SLA deadline computation (BC.1).
    business_calendar: Option<BusinessCalendarDocument>,
    bindings: BindingRegistry,
}

impl WosRuntime {
    /// Create a runtime from host dependencies and registered bindings.
    pub fn new<S, R, P, A, E, V, C>(
        store: S,
        resolver: R,
        presenter: P,
        access_control: A,
        service: E,
        validator: V,
        clock: C,
        bindings: BindingRegistry,
    ) -> Self
    where
        S: RuntimeStore + 'static,
        R: DocumentResolver + 'static,
        R::Error: StdError + Send + Sync + 'static,
        P: TaskPresenter + 'static,
        P::Error: StdError + Send + Sync + 'static,
        A: AccessControl + 'static,
        E: ExternalService + 'static,
        E::Error: StdError + Send + Sync + 'static,
        V: ContractValidator + 'static,
        V::Error: StdError + Send + Sync + 'static,
        C: Clock + 'static,
    {
        Self {
            store: Box::new(store),
            resolver: Box::new(resolver),
            presenter: Box::new(presenter),
            access_control: Box::new(access_control),
            service: Box::new(service),
            validator: Box::new(validator),
            clock: Box::new(clock),
            companion_policy: Box::new(NoopCompanionPolicy),
            integration_profile: None,
            business_calendar: None,
            bindings,
        }
    }

    /// Replace the default no-op companion-policy hook.
    pub fn with_companion_policy<P>(mut self, companion_policy: P) -> Self
    where
        P: CompanionPolicy + 'static,
    {
        self.companion_policy = Box::new(companion_policy);
        self
    }

    /// Attach an Integration Profile document for `invokeService` bindings.
    pub fn with_integration_profile(mut self, profile: IntegrationProfileDocument) -> Self {
        self.integration_profile = Some(profile);
        self
    }

    /// Attach a Business Calendar document for SLA deadline computation (BC.1).
    ///
    /// When a calendar is attached, timer deadlines are computed by advancing
    /// through business time rather than wall-clock time. Deadlines are
    /// computed lazily on each `drain_once` call, not at timer creation time.
    /// Replacing the calendar between events shifts future (not yet fired)
    /// timer deadlines on the next drain.
    pub fn with_business_calendar(mut self, calendar: BusinessCalendarDocument) -> Self {
        self.business_calendar = Some(calendar);
        self
    }

    /// Create and persist a new case instance.
    pub fn create_instance(
        &mut self,
        request: CreateInstanceRequest,
    ) -> Result<CaseInstance, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let CreateInstanceRequest {
            instance_id,
            definition_url,
            definition_version,
            initial_case_state,
        } = request;
        let kernel = self
            .resolver
            .resolve_kernel(&definition_url, &definition_version)?;
        let mut evaluator = Evaluator::with_time_and_case_state(
            kernel.clone(),
            now_ms,
            initial_case_state.as_ref(),
        )
        .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;

        let (timer_states, convergence_error_ids) =
            timers_to_state(evaluator.timers(), now_ms, self.business_calendar.as_ref())?;
        let instance = CaseInstance {
            instance_id,
            definition_url,
            definition_version,
            configuration: evaluator.configuration().active_states().to_vec(),
            case_state: evaluator.case_state_json(),
            provenance_position: 0,
            next_task_sequence: 0,
            timers: timer_states,
            active_tasks: Vec::new(),
            history_store: None,
            compensation_logs: None,
            status: InstanceStatus::Active,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            extensions: Default::default(),
        };

        let mut record = RuntimeRecord::new(instance);
        let mut appended_provenance = evaluator.provenance().records().to_vec();
        // Annotate any timers created during instance initialization with calendarVersion.
        if let Some(cal) = &self.business_calendar {
            annotate_timer_created_with_calendar_version(&mut appended_provenance, cal);
        }
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(&mut appended_provenance, &convergence_error_ids);
        let actions = evaluator.take_executed_actions();
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);
        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);
        record.instance.provenance_position = appended_provenance.len() as u64;
        record.provenance_log.extend(appended_provenance);
        self.store.create_record(record.clone())?;

        self.deliver_pending_presentations(&pending_presentations)?;

        let _ = (created_task_ids, emitted_events);
        Ok(record.instance)
    }

    /// Load the canonical case instance state.
    pub fn load_instance(&self, instance_id: &str) -> Result<CaseInstance, RuntimeError> {
        Ok(self.store.load_record(instance_id)?.instance)
    }

    /// Append an event to the instance queue.
    pub fn enqueue_event(
        &mut self,
        instance_id: &str,
        mut event: PendingEvent,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(instance_id)?;
        if event.timestamp.is_empty() {
            event.timestamp = format_timestamp(self.clock.now_ms())?;
        }
        record.instance.pending_events.push(event);
        record.instance.updated_at = format_timestamp(self.clock.now_ms())?;
        self.store.save_record(record.clone())?;
        Ok(())
    }

    /// Drain a single event from the instance queue.
    pub fn drain_once(&mut self, instance_id: &str) -> Result<DrainOnceResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.store.load_record(instance_id)?;
        let mut appended_provenance =
            materialize_due_timers(&mut record.instance, now_ms, &now_iso)?;

        let Some(event) = record.instance.pending_events.first().cloned() else {
            if !appended_provenance.is_empty() {
                record.instance.updated_at = now_iso;
                record.instance.provenance_position += appended_provenance.len() as u64;
                record.provenance_log.extend(appended_provenance);
                self.store.save_record(record)?;
            }
            return Ok(DrainOnceResult::default());
        };

        record.instance.pending_events.remove(0);
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        let mut runtime_result = DrainOnceResult {
            processed_event: Some(event.event.clone()),
            processed_event_token: event.idempotency_token.clone(),
            transitions: Vec::new(),
            provenance: Vec::new(),
            created_task_ids: Vec::new(),
            emitted_events: Vec::new(),
        };

        let decision = self.companion_policy.evaluate_event(RuntimeEventContext {
            kernel: kernel.clone(),
            instance: record.instance.clone(),
            event,
        })?;
        appended_provenance.extend(decision.provenance);

        let Some(event) = decision.event else {
            record.instance.updated_at = now_iso;
            record.instance.provenance_position += appended_provenance.len() as u64;
            record.provenance_log.extend(appended_provenance.clone());
            self.store.save_record(record)?;
            runtime_result.provenance = appended_provenance;
            return Ok(runtime_result);
        };

        let mut evaluator = Evaluator::from_instance(kernel.clone(), &record.instance, now_ms)
            .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;
        evaluator
            .process_event(&event.event, event.actor_id.as_deref(), event.data.as_ref())
            .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;
        runtime_result.transitions = evaluator.transitions().to_vec();

        appended_provenance.extend(evaluator.provenance().records().to_vec());
        // Annotate any newly created timers with calendarVersion when a calendar
        // is attached (provenance approach a — augment data field, no new variant).
        if let Some(cal) = &self.business_calendar {
            annotate_timer_created_with_calendar_version(&mut appended_provenance, cal);
        }
        appended_provenance.extend(compensation_provenance(
            &kernel,
            &record.provenance_log,
            &appended_provenance,
        ));
        record.instance.configuration = evaluator.configuration().active_states().to_vec();
        record.instance.case_state = evaluator.case_state_json();
        let (timer_states, convergence_error_ids) =
            timers_to_state(evaluator.timers(), now_ms, self.business_calendar.as_ref())?;
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(&mut appended_provenance, &convergence_error_ids);
        record.instance.timers = timer_states;
        let history = evaluator.history_store().clone();
        record.instance.history_store = if history.is_empty() {
            None
        } else {
            Some(history)
        };
        record.instance.updated_at = now_iso.clone();

        let case_state_can_mutate_explicitly = record
            .provenance_log
            .iter()
            .chain(appended_provenance.iter())
            .any(|record| record.record_kind == ProvenanceKind::CaseStateMutation);
        if !runtime_result.transitions.is_empty() && case_state_can_mutate_explicitly {
            appended_provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::StateTransition,
                actor_id: event.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: Some(event.event.clone()),
                data: Some(serde_json::json!({ "caseStateUnchangedByTransition": true })),
            });
        }

        // Milestone firing: evaluate after durable case-state write, before reactive
        // transitions drain (Kernel S4.13).  Records are appended in lexicographic
        // milestone-id order so the provenance stream is deterministic.
        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(&kernel, &mut record.instance, &post_state);
        appended_provenance.extend(milestone_records);

        let actions = evaluator.take_executed_actions();
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);

        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);

        record.instance.provenance_position += appended_provenance.len() as u64;
        record.provenance_log.extend(appended_provenance.clone());
        self.store.save_record(record.clone())?;

        self.deliver_pending_presentations(&pending_presentations)?;

        runtime_result.provenance = appended_provenance;
        runtime_result.created_task_ids = created_task_ids;
        runtime_result.emitted_events = emitted_events;
        Ok(runtime_result)
    }

    /// Drain events until the queue is empty and no timers are due.
    pub fn drain_until_idle(
        &mut self,
        instance_id: &str,
    ) -> Result<Vec<DrainOnceResult>, RuntimeError> {
        let mut results = Vec::new();

        loop {
            let result = self.drain_once(instance_id)?;
            let should_stop = result.processed_event.is_none();
            if should_stop {
                break;
            }
            results.push(result);
        }

        Ok(results)
    }

    /// Persist a draft task response.
    pub fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.load_record_for_task_id(task_id)?;
        if let Some(token) = idempotency_token {
            let replay_key = ReplayKey {
                operation: ReplayOperation::PersistDraft,
                task_id: task_id.to_string(),
                actor_id: actor_id.to_string(),
                token: token.to_string(),
            };
            if let Some(ReplayValue::Draft(result)) = record.replay_entries.get(&replay_key) {
                return Ok(result.clone());
            }
        }
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

        let task = record.instance.active_tasks[task_index].clone();
        authorize_actor(&*self.access_control, &task, actor_id)?;
        let status = response
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RuntimeError::InvalidResponseStatus("missing draft status".to_string()))?
            .to_string();
        if !matches!(status.as_str(), "in-progress" | "amended" | "stopped") {
            return Err(RuntimeError::InvalidResponseStatus(status));
        }

        let artifact = build_artifact(
            &record,
            task_id,
            TaskArtifactKind::Draft,
            response,
            actor_id,
            &now_iso,
        );
        let result = PersistDraftResult {
            artifact_id: artifact.artifact_id.clone(),
        };
        record
            .artifacts
            .insert(artifact.artifact_id.clone(), artifact.clone());
        let provenance = ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskDraftPersisted,
            task_id,
            Some(actor_id),
            Some(serde_json::json!({
                "artifactId": artifact.artifact_id,
                "status": status,
            })),
        );
        record.instance.provenance_position += 1;
        record.provenance_log.push(provenance);
        record.instance.updated_at = now_iso;

        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::PersistDraft,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Draft(result.clone()),
            );
        }

        self.store.save_record(record)?;
        Ok(result)
    }

    /// Record a UI dismissal without advancing lifecycle state.
    pub fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        let now_iso = format_timestamp(self.clock.now_ms())?;
        let mut record = self.load_record_for_task_id(task_id)?;
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;
        let task = &record.instance.active_tasks[task_index];
        if task.context.is_some() {
            self.presenter.dismiss_task(task_id, reason)?;
        }

        record.instance.provenance_position += 1;
        record.provenance_log.push(ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskDismissed,
            task_id,
            task.assigned_actor.as_deref(),
            Some(serde_json::json!({ "reason": reason })),
        ));
        record.instance.updated_at = now_iso;
        self.store.save_record(record)?;
        Ok(())
    }

    /// Submit a completed task response.
    pub fn submit_task_response(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.load_record_for_task_id(task_id)?;
        if let Some(token) = idempotency_token {
            let replay_key = ReplayKey {
                operation: ReplayOperation::SubmitTaskResponse,
                task_id: task_id.to_string(),
                actor_id: actor_id.to_string(),
                token: token.to_string(),
            };
            if let Some(ReplayValue::Submission(result)) = record.replay_entries.get(&replay_key) {
                return Ok(result.clone());
            }
        }
        let task_index = find_task_index(&record, task_id)
            .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

        let task = record.instance.active_tasks[task_index].clone();
        authorize_actor(&*self.access_control, &task, actor_id)?;
        let status = response
            .get("status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                RuntimeError::InvalidResponseStatus("missing response status".to_string())
            })?
            .to_string();
        if status != "completed" {
            let result = TaskSubmissionResult::Rejected {
                code: "taskResponseStatusNotCompleted".to_string(),
            };
            self.record_submission_rejection(
                &mut record,
                task_id,
                actor_id,
                "taskResponseStatusNotCompleted",
                &now_iso,
                idempotency_token,
                result.clone(),
            )?;
            return Ok(result);
        }

        let binding = task
            .binding
            .as_deref()
            .ok_or_else(|| RuntimeError::UnsupportedBinding("task has no binding".to_string()))?;
        let adapter = self
            .bindings
            .get(binding)
            .ok_or_else(|| RuntimeError::UnsupportedBinding(binding.to_string()))?;
        let validation = adapter.validate_submission(&task, &response)?;
        let mut provenance = vec![ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskResponseSubmitted,
            task_id,
            Some(actor_id),
            None,
        )];
        provenance.push(contract_validation_record(
            task_id,
            actor_id,
            &response,
            &validation,
        ));

        if !validation_passed(&validation) {
            let emitted_event = remove_task_with_event(
                &mut record.instance,
                task_index,
                FAILURE_EVENT_EXTENSION_KEY,
                actor_id,
                &now_iso,
            );
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskFailed,
                task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "code": "validationFailed",
                    "validationOutcome": validation.validation_outcome,
                })),
            ));
            record.instance.provenance_position += provenance.len() as u64;
            record.provenance_log.extend(provenance);
            record.instance.updated_at = now_iso;
            let result = TaskSubmissionResult::Failed {
                code: "validationFailed".to_string(),
                emitted_event,
            };
            if let Some(token) = idempotency_token {
                record.replay_entries.insert(
                    ReplayKey {
                        operation: ReplayOperation::SubmitTaskResponse,
                        task_id: task_id.to_string(),
                        actor_id: actor_id.to_string(),
                        token: token.to_string(),
                    },
                    ReplayValue::Submission(result.clone()),
                );
            }
            self.store.save_record(record)?;
            return Ok(result);
        }

        let accepted_artifact = build_artifact(
            &record,
            task_id,
            TaskArtifactKind::Accepted,
            response.clone(),
            actor_id,
            &now_iso,
        );
        record.artifacts.insert(
            accepted_artifact.artifact_id.clone(),
            accepted_artifact.clone(),
        );
        let mutation = adapter.compute_case_mutation(&task, &response)?;
        let case_mutated = mutation
            .as_ref()
            .is_some_and(|bundle| !bundle.field_updates.is_empty());
        if let Some(bundle) = mutation {
            if !bundle.field_updates.is_empty() {
                merge_case_state(
                    &mut record.instance.case_state,
                    &serde_json::Value::Object(bundle.field_updates.clone()),
                );
                provenance.push(ProvenanceRecord::task_lifecycle(
                    ProvenanceKind::DataMapping,
                    task_id,
                    Some(actor_id),
                    Some(serde_json::json!({
                        "artifactId": accepted_artifact.artifact_id,
                        "mappingRef": task.response_mapping_ref,
                    })),
                ));
            }
        }

        // Milestone firing: evaluate after durable case-state write, before reactive
        // transitions drain (Kernel S4.13).  Records follow any DataMapping record so
        // the provenance stream reads: data changed → milestone fired.
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        let post_state = record.instance.case_state.clone();
        let milestone_records = evaluate_milestones(&kernel, &mut record.instance, &post_state);
        provenance.extend(milestone_records);

        let emitted_event = remove_task_with_event(
            &mut record.instance,
            task_index,
            COMPLETION_EVENT_EXTENSION_KEY,
            actor_id,
            &now_iso,
        );
        provenance.push(ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskCompleted,
            task_id,
            Some(actor_id),
            Some(serde_json::json!({
                "artifactId": accepted_artifact.artifact_id,
                "caseMutated": case_mutated,
            })),
        ));
        record.instance.provenance_position += provenance.len() as u64;
        record.provenance_log.extend(provenance);
        record.instance.updated_at = now_iso;

        let result = TaskSubmissionResult::Completed {
            artifact_id: accepted_artifact.artifact_id,
            case_mutated,
            emitted_event,
        };
        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::SubmitTaskResponse,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Submission(result.clone()),
            );
        }
        self.store.save_record(record)?;
        Ok(result)
    }

    /// Load a provenance window by cursor and limit.
    pub fn load_provenance_window(
        &self,
        instance_id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let record = self.store.load_record(instance_id)?;
        Ok(record
            .provenance_log
            .iter()
            .skip(cursor)
            .take(limit)
            .cloned()
            .collect())
    }

    fn load_record_for_task_id(&self, task_id: &str) -> Result<RuntimeRecord, RuntimeError> {
        let instance_id = task_instance_id(task_id)?;
        Ok(self.store.load_record(&instance_id)?)
    }

    fn apply_observed_actions(
        &mut self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        actions: &[ObservedAction],
        now_iso: &str,
    ) -> Result<(Vec<String>, Vec<String>, Vec<ProvenanceRecord>), RuntimeError> {
        let mut created_task_ids = Vec::new();
        let mut emitted_events = Vec::new();
        let mut provenance = Vec::new();

        for observed in actions {
            match observed.action.action {
                ActionKind::CreateTask => {
                    let task = self.create_active_task(kernel, record, observed, now_iso)?;
                    created_task_ids.push(task.task_id.clone());
                    provenance.push(ProvenanceRecord::task_lifecycle(
                        ProvenanceKind::TaskCreated,
                        &task.task_id,
                        observed.actor_id.as_deref(),
                        Some(serde_json::json!({
                            "taskRef": task.task_ref,
                            "binding": task.binding,
                        })),
                    ));
                    record.instance.active_tasks.push(task);
                }
                ActionKind::EmitEvent => {
                    let event_name = observed.action.event_type.clone().ok_or_else(|| {
                        RuntimeError::UnsupportedAction("emitEvent missing eventType".to_string())
                    })?;
                    record.instance.pending_events.push(PendingEvent {
                        event: event_name.clone(),
                        actor_id: observed.actor_id.clone(),
                        data: observed.action.data.clone(),
                        timestamp: now_iso.to_string(),
                        idempotency_token: None,
                    });
                    emitted_events.push(event_name);
                }
                ActionKind::InvokeService => {
                    let service_ref = observed.action.service_ref.clone().ok_or_else(|| {
                        RuntimeError::UnsupportedAction(
                            "invokeService missing serviceRef".to_string(),
                        )
                    })?;
                    let integration_binding = self
                        .integration_profile
                        .as_ref()
                        .and_then(|profile| profile.bindings.get(&service_ref))
                        .cloned();
                    if let Some(binding) = integration_binding {
                        provenance.extend(self.invoke_integration_binding(
                            record,
                            kernel,
                            observed,
                            &service_ref,
                            &binding,
                            now_iso,
                        )?);
                        continue;
                    }

                    let input = observed
                        .action
                        .data
                        .clone()
                        .unwrap_or_else(|| serde_json::json!({}));
                    let idempotency_key = observed.action.idempotency_key.as_deref();
                    let (step_result, reused_persisted_result) = load_or_invoke_service_result(
                        self.service.as_ref(),
                        record,
                        &service_ref,
                        &input,
                        idempotency_key,
                        now_iso,
                    )?;

                    if reused_persisted_result {
                        provenance.push(ProvenanceRecord {
                            record_kind: ProvenanceKind::IdempotencyDedup,
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "serviceRef": service_ref,
                                "idempotencyKey": idempotency_key,
                                "stepResultRecordedAt": step_result.recorded_at,
                            })),
                        });
                    } else {
                        provenance.push(ProvenanceRecord {
                            record_kind: ProvenanceKind::StepResultPersisted,
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "serviceRef": service_ref,
                                "idempotencyKey": idempotency_key,
                                "output": step_result.output,
                                "persistedBeforeAdvance": true,
                            })),
                        });
                    }

                    if let Some(contract_ref) = observed.action.contract_ref.as_deref() {
                        let validation_result =
                            self.validator.validate(contract_ref, &step_result.output)?;
                        provenance.push(ProvenanceRecord {
                            record_kind: ProvenanceKind::ContractValidation,
                            actor_id: observed.actor_id.clone(),
                            from_state: None,
                            to_state: None,
                            event: None,
                            data: Some(serde_json::json!({
                                "contractRef": contract_ref,
                                "structured": true,
                                "valid": validation_result.valid,
                                "errors": validation_result.errors,
                            })),
                        });
                    }
                }
                ActionKind::SetData
                | ActionKind::StartTimer
                | ActionKind::CancelTimer
                | ActionKind::Log => {}
            }
        }

        Ok((created_task_ids, emitted_events, provenance))
    }

    fn invoke_integration_binding(
        &mut self,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        self.validate_integration_profile_target(kernel, &record.instance)?;
        let ctx = InvocationContext {
            service: self.service.as_ref(),
            validator: self.validator.as_ref(),
        };
        dispatch_integration_binding(&ctx, record, kernel, observed, service_ref, binding, now_iso)
    }

    fn validate_integration_profile_target(
        &self,
        kernel: &KernelDocument,
        instance: &CaseInstance,
    ) -> Result<(), RuntimeError> {
        let Some(profile) = self.integration_profile.as_ref() else {
            return Ok(());
        };

        if profile.target_workflow.url != instance.definition_url {
            return Err(RuntimeError::Integration(format!(
                "integration profile targets '{}' but instance uses '{}'",
                profile.target_workflow.url, instance.definition_url
            )));
        }

        if let Some(compatible_versions) = profile.target_workflow.compatible_versions.as_deref() {
            let requested_version =
                Version::parse(&instance.definition_version).map_err(|error| {
                    RuntimeError::Integration(format!(
                        "instance definition version '{}' is not valid semver: {error}",
                        instance.definition_version
                    ))
                })?;
            let normalized_versions = normalize_semver_range_expression(compatible_versions);
            let version_req = VersionReq::parse(&normalized_versions).map_err(|error| {
                RuntimeError::Integration(format!(
                    "integration profile compatibleVersions '{}' is not valid semver: {error}",
                    compatible_versions
                ))
            })?;
            if !version_req.matches(&requested_version) {
                return Err(RuntimeError::Integration(format!(
                    "integration profile compatibleVersions '{}' do not include instance version '{}'",
                    compatible_versions, instance.definition_version
                )));
            }
        }

        if kernel.url.as_deref() != Some(instance.definition_url.as_str()) {
            return Err(RuntimeError::Integration(format!(
                "kernel document url '{}' does not match instance definition url '{}'",
                kernel.url.as_deref().unwrap_or_default(),
                instance.definition_url
            )));
        }

        Ok(())
    }

    fn stage_pending_tasks_for_presentation(
        &mut self,
        record: &mut RuntimeRecord,
        now_iso: &str,
    ) -> Result<(Vec<FormspecTaskContext>, Vec<ProvenanceRecord>), RuntimeError> {
        let mut pending_presentations = Vec::new();
        let mut provenance = Vec::new();

        for task in &mut record.instance.active_tasks {
            let Some(context) = task.context.as_ref() else {
                continue;
            };
            if task.binding.as_deref() != Some("formspec") {
                continue;
            }
            if task.status != ActiveTaskStatus::Created {
                continue;
            }

            task.status = ActiveTaskStatus::Assigned;
            task.updated_at = now_iso.to_string();
            pending_presentations.push(context.clone());
            provenance.push(ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskPresented,
                &task.task_id,
                task.assigned_actor.as_deref(),
                Some(serde_json::json!({
                    "definitionUrl": task.definition_url,
                    "definitionVersion": task.definition_version,
                })),
            ));
        }

        Ok((pending_presentations, provenance))
    }

    fn deliver_pending_presentations(
        &mut self,
        contexts: &[FormspecTaskContext],
    ) -> Result<(), RuntimeError> {
        for context in contexts {
            self.presenter.present_task(context)?;
        }
        Ok(())
    }

    fn create_active_task(
        &mut self,
        kernel: &KernelDocument,
        record: &mut RuntimeRecord,
        observed: &ObservedAction,
        now_iso: &str,
    ) -> Result<ActiveTask, RuntimeError> {
        let action = &observed.action;
        let task_ref = action.task_ref.clone().ok_or_else(|| {
            RuntimeError::MissingMetadata("createTask missing taskRef".to_string())
        })?;
        let task_sequence = record.instance.next_task_sequence + 1;
        record.instance.next_task_sequence = task_sequence;
        let task_id = make_task_id(&record.instance.instance_id, task_sequence, &task_ref);

        let mut task = ActiveTask {
            task_id,
            task_ref,
            status: ActiveTaskStatus::Created,
            assigned_actor: action.assign_to.clone(),
            contract_ref: action.contract_ref.clone(),
            binding: None,
            definition_url: None,
            definition_version: None,
            prefill_mapping_ref: action.prefill_mapping_ref.clone(),
            response_mapping_ref: action.response_mapping_ref.clone(),
            deadline: None,
            impact_level: kernel.impact_level.map(impact_level_label),
            context: None,
            last_validation_outcome: None,
            created_at: now_iso.to_string(),
            updated_at: now_iso.to_string(),
            extensions: Default::default(),
        };

        if let Some(completion_event) = &action.completion_event {
            task.extensions.insert(
                COMPLETION_EVENT_EXTENSION_KEY.to_string(),
                serde_json::Value::String(completion_event.clone()),
            );
        }
        if let Some(failure_event) = &action.failure_event {
            task.extensions.insert(
                FAILURE_EVENT_EXTENSION_KEY.to_string(),
                serde_json::Value::String(failure_event.clone()),
            );
        }

        if let Some(contract_key) = &task.contract_ref {
            let contract = kernel
                .contracts
                .get(contract_key)
                .ok_or_else(|| RuntimeError::ContractNotFound(contract_key.clone()))?;
            task.binding = Some(contract.binding.clone());
            task.definition_url = Some(contract.reference.clone());
            task.definition_version = Some(kernel.version.clone().ok_or_else(|| {
                RuntimeError::MissingMetadata("kernel version required".to_string())
            })?);
            if task.prefill_mapping_ref.is_none() {
                task.prefill_mapping_ref = contract.prefill_mapping_ref.clone();
            }
            if task.response_mapping_ref.is_none() {
                task.response_mapping_ref = contract.response_mapping_ref.clone();
            }

            if contract.binding == "formspec" {
                let assigned_actor = task.assigned_actor.clone().ok_or_else(|| {
                    RuntimeError::MissingMetadata(
                        "formspec task requires assigned actor".to_string(),
                    )
                })?;
                let adapter = self
                    .bindings
                    .get(&contract.binding)
                    .ok_or_else(|| RuntimeError::UnsupportedBinding(contract.binding.clone()))?;
                let prepared = adapter.prepare_task(&task, &record.instance.case_state)?;
                task.context = Some(FormspecTaskContext {
                    task_id: task.task_id.clone(),
                    instance_id: record.instance.instance_id.clone(),
                    contract_ref: contract_key.clone(),
                    definition_url: task.definition_url.clone().unwrap_or_default(),
                    definition_version: task.definition_version.clone().unwrap_or_default(),
                    binding: contract.binding.clone(),
                    assigned_actor,
                    prefill_data: prepared.prefill_data,
                    prefill_mapping_ref: task.prefill_mapping_ref.clone(),
                    response_mapping_ref: task.response_mapping_ref.clone(),
                    deadline: task.deadline.clone(),
                    impact_level: task.impact_level.clone(),
                    extensions: Default::default(),
                });
            } else {
                return Err(RuntimeError::UnsupportedBinding(contract.binding.clone()));
            }
        }

        Ok(task)
    }

    fn record_submission_rejection(
        &mut self,
        record: &mut RuntimeRecord,
        task_id: &str,
        actor_id: &str,
        code: &str,
        updated_at: &str,
        idempotency_token: Option<&str>,
        result: TaskSubmissionResult,
    ) -> Result<(), RuntimeError> {
        record.instance.provenance_position += 1;
        record.provenance_log.push(ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskResponseRejected,
            task_id,
            Some(actor_id),
            Some(serde_json::json!({ "code": code })),
        ));
        record.instance.updated_at = updated_at.to_string();
        if let Some(token) = idempotency_token {
            record.replay_entries.insert(
                ReplayKey {
                    operation: ReplayOperation::SubmitTaskResponse,
                    task_id: task_id.to_string(),
                    actor_id: actor_id.to_string(),
                    token: token.to_string(),
                },
                ReplayValue::Submission(result),
            );
        }
        self.store.save_record(record.clone())?;
        Ok(())
    }
}

fn materialize_due_timers(
    instance: &mut CaseInstance,
    now_ms: u64,
    now_iso: &str,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    let mut due = Vec::new();
    let mut remaining = Vec::new();

    for timer in instance.timers.drain(..) {
        if parse_timestamp(&timer.deadline)? <= now_ms {
            due.push(timer);
        } else {
            remaining.push(timer);
        }
    }
    instance.timers = remaining;

    let mut provenance = Vec::new();
    for timer in due {
        provenance.push(ProvenanceRecord::timer_fired(&timer.timer_id, &timer.event));
        let deadline_ms = parse_timestamp(&timer.deadline)?;
        let lateness_ms = now_ms.saturating_sub(deadline_ms);
        let max_tolerance = max_tolerance_ms(timer.duration_ms.unwrap_or(0));
        if lateness_ms > max_tolerance {
            let tolerance_iso = tolerance_to_iso(max_tolerance);
            provenance.push(ProvenanceRecord::tolerance_violation(
                &timer.timer_id,
                timer.duration_iso.as_deref().unwrap_or("P0D"),
                &tolerance_iso,
            ));
        }
        instance.pending_events.push(PendingEvent {
            event: timer.event.clone(),
            actor_id: None,
            data: Some(serde_json::json!({ "timerId": timer.timer_id })),
            timestamp: now_iso.to_string(),
            idempotency_token: None,
        });
    }

    Ok(provenance)
}

/// Convert all timers to `TimerState`, computing calendar-adjusted deadlines lazily.
///
/// Returns `(states, convergence_error_timer_ids)`.  The second element lists
/// timer IDs whose deadline fell back to naive wall-clock time because the
/// business calendar evaluator did not converge (degenerate calendar).
/// Callers MUST annotate the corresponding `TimerCreated` provenance records
/// with `calendarVersionConvergenceError: true`.
fn timers_to_state(
    timers: &wos_core::timer::Timers,
    _now_ms: u64,
    calendar: Option<&BusinessCalendarDocument>,
) -> Result<(Vec<wos_core::instance::TimerState>, Vec<String>), RuntimeError> {
    let mut states = Vec::with_capacity(timers.len());
    let mut convergence_error_ids = Vec::new();
    for timer in timers.iter() {
        let (state, had_error) = timer_to_state(timer, calendar)?;
        if had_error {
            convergence_error_ids.push(state.timer_id.clone());
        }
        states.push(state);
    }
    Ok((states, convergence_error_ids))
}

/// Convert a `Timer` to a `TimerState`, computing the deadline lazily.
///
/// When a business calendar is attached, the deadline is re-computed using
/// [`next_business_moment`] each time this function is called (lazy evaluation).
/// This means calendar updates between events shift future deadlines on the
/// next drain.  When no calendar is attached, the raw wall-clock deadline is
/// used unchanged.
///
/// Returns `(TimerState, had_convergence_error)`.  The second field is `true`
/// when a degenerate calendar caused snap-forward to fall back to the naive
/// deadline; callers should annotate the `TimerCreated` provenance record.
fn timer_to_state(
    timer: &Timer,
    calendar: Option<&BusinessCalendarDocument>,
) -> Result<(wos_core::instance::TimerState, bool), RuntimeError> {
    let (deadline_ms, had_convergence_error) = match calendar {
        Some(cal) => business_deadline_ms(timer, cal)?,
        None => (timer.deadline_ms, false),
    };

    let state = wos_core::instance::TimerState {
        timer_id: timer.id.clone(),
        deadline: format_timestamp(deadline_ms)?,
        event: timer.fires_event.clone(),
        scope_state: if timer.created_in_state.is_empty() {
            None
        } else {
            Some(timer.created_in_state.clone())
        },
        duration_iso: Some(timer.duration_iso.clone()),
        duration_ms: Some(timer.duration_ms),
    };
    Ok((state, had_convergence_error))
}

/// Compute a business-calendar–adjusted deadline for `timer`.
///
/// Reconstructs the start time as `deadline_ms - duration_ms`, then advances
/// through business time using the attached calendar.
///
/// Returns `(deadline_ms, had_convergence_error)`.  When the calendar evaluator
/// does not converge (degenerate calendar), falls back to the naive wall-clock
/// deadline and sets the flag to `true` so the caller can annotate provenance.
///
/// # Invariant
///
/// `Timer.duration_ms` is authoritative for reconstructing the start time —
/// it is stored at timer-creation time and never mutated.
fn business_deadline_ms(
    timer: &Timer,
    calendar: &BusinessCalendarDocument,
) -> Result<(u64, bool), RuntimeError> {
    // Invariant: Timer.duration_ms is authoritative; reconstruct start from it.
    let start_ms = timer.deadline_ms.saturating_sub(timer.duration_ms);
    let start_secs = i64::try_from(start_ms / 1000)
        .map_err(|_| RuntimeError::Clock("timer start timestamp out of range".to_string()))?;
    let start_utc = DateTime::from_timestamp(start_secs, 0)
        .ok_or_else(|| RuntimeError::Clock("invalid timer start timestamp".to_string()))?;

    let duration = chrono::Duration::milliseconds(
        i64::try_from(timer.duration_ms)
            .map_err(|_| RuntimeError::Clock("timer duration out of range".to_string()))?,
    );

    match next_business_moment(start_utc, duration, calendar) {
        Ok(result) => {
            let result_ms = u64::try_from(result.timestamp_millis())
                .map_err(|_| RuntimeError::Clock("business deadline out of range".to_string()))?;
            Ok((result_ms, false))
        }
        Err(BusinessCalendarError::DidNotConverge { .. }) => {
            // Degenerate calendar: fall back to naive wall-clock deadline so the
            // timer is not lost, and signal the caller to annotate provenance.
            Ok((timer.deadline_ms, true))
        }
    }
}

/// Inject `calendarVersion` into every `TimerCreated` provenance record in `records`.
///
/// Uses provenance approach (a): extends the existing `data` JSON object with a
/// `calendarVersion` field — no new provenance variant required.  When the calendar
/// has no `version` field, the field is set to `null`.
fn annotate_timer_created_with_calendar_version(
    records: &mut [ProvenanceRecord],
    calendar: &BusinessCalendarDocument,
) {
    let version = calendar
        .version
        .as_deref()
        .map(serde_json::Value::from)
        .unwrap_or(serde_json::Value::Null);

    for record in records.iter_mut() {
        if record.record_kind != ProvenanceKind::TimerCreated {
            continue;
        }
        match &mut record.data {
            Some(serde_json::Value::Object(map)) => {
                map.insert("calendarVersion".to_string(), version.clone());
            }
            other => {
                let mut map = serde_json::Map::new();
                if let Some(existing) = other.take() {
                    if let serde_json::Value::Object(existing_map) = existing {
                        map.extend(existing_map);
                    }
                }
                map.insert("calendarVersion".to_string(), version.clone());
                *other = Some(serde_json::Value::Object(map));
            }
        }
    }
}

/// Extend `TimerCreated` records for the given timer IDs with
/// `calendarVersionConvergenceError: true`.
///
/// Uses the same payload-extension pattern as
/// `annotate_timer_created_with_calendar_version` — no new `ProvenanceKind`
/// variant is needed.
fn annotate_timer_created_with_convergence_error(
    records: &mut [ProvenanceRecord],
    timer_ids: &[String],
) {
    if timer_ids.is_empty() {
        return;
    }
    for record in records.iter_mut() {
        if record.record_kind != ProvenanceKind::TimerCreated {
            continue;
        }
        // Check whether this record's timerId is in the convergence-error set.
        let record_timer_id = record
            .data
            .as_ref()
            .and_then(|d| d.get("timerId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !timer_ids.contains(&record_timer_id) {
            continue;
        }
        match &mut record.data {
            Some(serde_json::Value::Object(map)) => {
                map.insert(
                    "calendarVersionConvergenceError".to_string(),
                    serde_json::Value::Bool(true),
                );
            }
            other => {
                let mut map = serde_json::Map::new();
                if let Some(existing) = other.take() {
                    if let serde_json::Value::Object(existing_map) = existing {
                        map.extend(existing_map);
                    }
                }
                map.insert(
                    "calendarVersionConvergenceError".to_string(),
                    serde_json::Value::Bool(true),
                );
                *other = Some(serde_json::Value::Object(map));
            }
        }
    }
}

fn format_timestamp(timestamp_ms: u64) -> Result<String, RuntimeError> {
    let nanos = i128::from(timestamp_ms) * 1_000_000;
    let nanos_i64 = i64::try_from(nanos)
        .map_err(|_| RuntimeError::Clock("timestamp exceeds supported range".to_string()))?;
    let timestamp = OffsetDateTime::from_unix_timestamp_nanos(nanos_i64.into())
        .map_err(|error| RuntimeError::Clock(error.to_string()))?;
    timestamp
        .format(&Rfc3339)
        .map_err(|error| RuntimeError::Clock(error.to_string()))
}

fn parse_timestamp(timestamp: &str) -> Result<u64, RuntimeError> {
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339)
        .map_err(|error| RuntimeError::Clock(error.to_string()))?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    u64::try_from(millis).map_err(|_| RuntimeError::Clock("negative timestamp".to_string()))
}

fn merge_case_state(target: &mut serde_json::Value, updates: &serde_json::Value) {
    if let (Some(target_object), Some(update_object)) =
        (target.as_object_mut(), updates.as_object())
    {
        for (key, value) in update_object {
            target_object.insert(key.clone(), value.clone());
        }
    }
}

fn normalize_semver_range_expression(expression: &str) -> String {
    expression
        .split("||")
        .map(|clause| {
            let clause = clause.trim();
            if clause.contains(',') {
                clause.to_string()
            } else {
                clause.split_whitespace().collect::<Vec<_>>().join(", ")
            }
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn impact_level_label(level: ImpactLevel) -> String {
    match level {
        ImpactLevel::RightsImpacting => "rights-impacting",
        ImpactLevel::SafetyImpacting => "safety-impacting",
        ImpactLevel::Operational => "operational",
        ImpactLevel::Informational => "informational",
    }
    .to_string()
}

fn task_instance_id(task_id: &str) -> Result<String, RuntimeError> {
    let Some(encoded_instance_id) = task_id
        .strip_prefix("wos-task:")
        .and_then(|rest| rest.split_once(':'))
        .map(|(encoded_instance_id, _)| encoded_instance_id)
    else {
        return Err(RuntimeError::TaskNotFound(task_id.to_string()));
    };

    let decoded = URL_SAFE_NO_PAD
        .decode(encoded_instance_id)
        .map_err(|_| RuntimeError::TaskNotFound(task_id.to_string()))?;
    std::str::from_utf8(&decoded)
        .map(str::to_owned)
        .map_err(|_| RuntimeError::TaskNotFound(task_id.to_string()))
}

fn find_task_index(record: &RuntimeRecord, task_id: &str) -> Option<usize> {
    record
        .instance
        .active_tasks
        .iter()
        .position(|task| task.task_id == task_id)
}

fn make_task_id(instance_id: &str, ordinal: u64, task_ref: &str) -> String {
    let encoded_instance_id = URL_SAFE_NO_PAD.encode(instance_id);
    format!("wos-task:{encoded_instance_id}:{ordinal}:{task_ref}")
}

fn authorize_actor(
    access_control: &dyn AccessControl,
    task: &ActiveTask,
    actor_id: &str,
) -> Result<(), RuntimeError> {
    let assigned_actor = task
        .assigned_actor
        .as_deref()
        .ok_or_else(|| RuntimeError::Unauthorized("task has no assigned actor".to_string()))?;
    if actor_id == assigned_actor {
        return Ok(());
    }

    let mut scope = DelegationScope {
        impact_levels: Vec::new(),
        case_types: Vec::new(),
        max_dollar_threshold: None,
        conditions: None,
    };
    if let Some(impact_level) = &task.impact_level {
        scope.impact_levels.push(impact_level.clone());
    }
    if access_control.can_delegate(assigned_actor, actor_id, &scope) {
        Ok(())
    } else {
        Err(RuntimeError::Unauthorized(actor_id.to_string()))
    }
}

fn validation_passed(validation: &SubmissionValidation) -> bool {
    let outcome = &validation.validation_outcome;
    outcome.envelope_valid && outcome.pin_match && outcome.definition_valid
}

fn build_artifact(
    record: &RuntimeRecord,
    task_id: &str,
    kind: TaskArtifactKind,
    response: serde_json::Value,
    actor_id: &str,
    recorded_at: &str,
) -> TaskArtifact {
    let kind_name = match kind {
        TaskArtifactKind::Draft => "draft",
        TaskArtifactKind::Accepted => "accepted",
    };
    let artifact_id = format!("{task_id}:{kind_name}:{}", record.artifacts.len() + 1);
    TaskArtifact {
        artifact_id,
        task_id: task_id.to_string(),
        kind,
        response,
        actor_id: actor_id.to_string(),
        recorded_at: recorded_at.to_string(),
    }
}

fn remove_task_with_event(
    instance: &mut CaseInstance,
    task_index: usize,
    extension_key: &str,
    actor_id: &str,
    timestamp: &str,
) -> Option<String> {
    let task = instance.active_tasks.remove(task_index);
    let emitted_event = task
        .extensions
        .get(extension_key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if let Some(event) = &emitted_event {
        instance.pending_events.push(PendingEvent {
            event: event.clone(),
            actor_id: Some(actor_id.to_string()),
            data: Some(serde_json::json!({ "taskId": task.task_id })),
            timestamp: timestamp.to_string(),
            idempotency_token: None,
        });
    }
    emitted_event
}

fn compensation_provenance(
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
            record_kind: ProvenanceKind::CompensationExecuted,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "order": reversed })),
        });
        provenance.push(ProvenanceRecord {
            record_kind: ProvenanceKind::CompensationScopeBoundary,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "innerScopeOnly": true })),
        });
    } else if visited.len() == 2 {
        if let Some((from, _)) = fail_transition {
            let compensated: Vec<&str> = visited
                .iter()
                .filter(|state| **state != *from)
                .copied()
                .collect();
            provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::CompensationExecuted,
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "pivotStep": from,
                    "compensated": compensated,
                    "excluded": [*from],
                })),
            });
        }
    }

    provenance
}

fn contract_validation_record(
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use crate::binding::{CaseMutationBundle, PreparedTask};
    use crate::store::{InMemoryStore, RuntimeStore, StoreError};
    use wos_core::instance::ValidationOutcome;
    use wos_core::traits::{DocumentResolver, ExternalService, TaskPresenter};

    #[derive(Debug, Clone)]
    struct FixedClock {
        now_ms: u64,
    }

    impl Clock for FixedClock {
        fn now_ms(&self) -> u64 {
            self.now_ms
        }
    }

    #[derive(Debug, Clone)]
    struct TestResolver {
        kernels: HashMap<(String, String), KernelDocument>,
    }

    impl TestResolver {
        fn with_kernel(kernel: KernelDocument) -> Self {
            let url = kernel.url.clone().unwrap();
            let version = kernel.version.clone().unwrap();
            let mut kernels = HashMap::new();
            kernels.insert((url, version), kernel);
            Self { kernels }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("resolver error: {0}")]
    struct TestResolverError(String);

    impl DocumentResolver for TestResolver {
        type Error = TestResolverError;

        fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
            self.kernels
                .get(&(url.to_string(), version.to_string()))
                .cloned()
                .ok_or_else(|| TestResolverError(format!("{url}@{version}")))
        }

        fn resolve_governance(
            &self,
            _url: &str,
            _version: &str,
        ) -> Result<wos_core::GovernanceDocument, Self::Error> {
            Err(TestResolverError("unused".to_string()))
        }

        fn resolve_sidecar(
            &self,
            _url: &str,
            _anchor_date: Option<&str>,
        ) -> Result<serde_json::Value, Self::Error> {
            Err(TestResolverError("unused".to_string()))
        }
    }

    #[derive(Debug, Clone, Default)]
    struct RecordingPresenter {
        presented: Arc<Mutex<Vec<FormspecTaskContext>>>,
        dismissed: Arc<Mutex<Vec<(String, String)>>>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("presenter error: {0}")]
    struct PresenterError(String);

    impl TaskPresenter for RecordingPresenter {
        type Error = PresenterError;

        fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), Self::Error> {
            self.presented.lock().unwrap().push(context.clone());
            Ok(())
        }

        fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), Self::Error> {
            self.dismissed
                .lock()
                .unwrap()
                .push((task_id.to_string(), reason.to_string()));
            Ok(())
        }
    }

    #[derive(Debug, Clone, Default)]
    struct SharedStore(Arc<Mutex<InMemoryStore>>);

    impl RuntimeStore for SharedStore {
        fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.0.lock().unwrap().create_record(record)
        }

        fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.0.lock().unwrap().load_record(instance_id)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.0.lock().unwrap().save_record(record)
        }
    }

    #[derive(Debug, Clone)]
    struct RecordingService {
        response: serde_json::Value,
        calls: Arc<AtomicUsize>,
        invocations: Arc<Mutex<Vec<RecordedInvocation>>>,
    }

    #[derive(Debug, Clone)]
    struct RecordedInvocation {
        service_ref: String,
        input: serde_json::Value,
        idempotency_key: Option<String>,
    }

    impl RecordingService {
        fn with_response(response: serde_json::Value) -> Self {
            Self {
                response,
                calls: Arc::new(AtomicUsize::new(0)),
                invocations: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("service error: {0}")]
    struct ServiceError(String);

    impl ExternalService for RecordingService {
        type Error = ServiceError;

        fn invoke(
            &self,
            service_ref: &str,
            input: &serde_json::Value,
            idempotency_key: Option<&str>,
        ) -> Result<serde_json::Value, Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.invocations.lock().unwrap().push(RecordedInvocation {
                service_ref: service_ref.to_string(),
                input: input.clone(),
                idempotency_key: idempotency_key.map(str::to_string),
            });
            Ok(self.response.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct StatusCheckingPresenter {
        store: Arc<Mutex<InMemoryStore>>,
    }

    impl TaskPresenter for StatusCheckingPresenter {
        type Error = PresenterError;

        fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), Self::Error> {
            let record = self
                .store
                .lock()
                .unwrap()
                .load_record(&context.instance_id)
                .unwrap();
            let task = record
                .instance
                .active_tasks
                .iter()
                .find(|task| task.task_id == context.task_id)
                .expect("task should exist when presented");
            assert_eq!(task.status, ActiveTaskStatus::Assigned);
            Ok(())
        }

        fn dismiss_task(&mut self, _task_id: &str, _reason: &str) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct TestAdapter;

    impl crate::binding::ContractBindingAdapter for TestAdapter {
        fn binding(&self) -> &'static str {
            "formspec"
        }

        fn prepare_task(
            &self,
            _task: &ActiveTask,
            case_state: &serde_json::Value,
        ) -> Result<PreparedTask, BindingError> {
            Ok(PreparedTask {
                prefill_data: Some(serde_json::json!({
                    "approved": case_state
                        .get("approved")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                })),
            })
        }

        fn validate_submission(
            &self,
            task: &ActiveTask,
            response: &serde_json::Value,
        ) -> Result<SubmissionValidation, BindingError> {
            let pin_match = response
                .get("definitionUrl")
                .and_then(serde_json::Value::as_str)
                == task.definition_url.as_deref()
                && response
                    .get("definitionVersion")
                    .and_then(serde_json::Value::as_str)
                    == task.definition_version.as_deref();
            let valid = response
                .get("data")
                .and_then(|data| data.get("approved"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            Ok(SubmissionValidation {
                validation_outcome: ValidationOutcome {
                    envelope_valid: true,
                    pin_match,
                    definition_valid: valid,
                    errors: if valid && pin_match {
                        Vec::new()
                    } else {
                        vec![serde_json::json!({ "code": "invalid" })]
                    },
                    validation_results: None,
                },
            })
        }

        fn compute_case_mutation(
            &self,
            task: &ActiveTask,
            response: &serde_json::Value,
        ) -> Result<Option<CaseMutationBundle>, BindingError> {
            if task.response_mapping_ref.is_none() {
                return Ok(None);
            }
            let mut field_updates = serde_json::Map::new();
            field_updates.insert("decision".to_string(), response["data"]["approved"].clone());
            Ok(Some(CaseMutationBundle { field_updates }))
        }
    }

    #[derive(Debug, Default)]
    struct UnavailableAdapter;

    impl crate::binding::ContractBindingAdapter for UnavailableAdapter {
        fn binding(&self) -> &'static str {
            "formspec"
        }

        fn prepare_task(
            &self,
            _task: &ActiveTask,
            _case_state: &serde_json::Value,
        ) -> Result<PreparedTask, BindingError> {
            Ok(PreparedTask::default())
        }

        fn validate_submission(
            &self,
            _task: &ActiveTask,
            _response: &serde_json::Value,
        ) -> Result<SubmissionValidation, BindingError> {
            Err(BindingError::ProcessorUnavailable(
                "formspec processor offline".to_string(),
            ))
        }

        fn compute_case_mutation(
            &self,
            _task: &ActiveTask,
            _response: &serde_json::Value,
        ) -> Result<Option<CaseMutationBundle>, BindingError> {
            Ok(None)
        }
    }

    #[derive(Debug, Default)]
    struct FailingStore {
        inner: InMemoryStore,
        fail_on_save_call: usize,
        save_calls: usize,
    }

    impl FailingStore {
        fn new(fail_on_save_call: usize) -> Self {
            Self {
                inner: InMemoryStore::new(),
                fail_on_save_call,
                save_calls: 0,
            }
        }
    }

    impl RuntimeStore for FailingStore {
        fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.create_record(record)
        }

        fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(instance_id)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.save_calls += 1;
            if self.save_calls == self.fail_on_save_call {
                return Err(StoreError::Failed("injected save failure".to_string()));
            }
            self.inner.save_record(record)
        }
    }

    #[derive(Debug, Default)]
    struct CreateFailingStore {
        inner: InMemoryStore,
    }

    impl RuntimeStore for CreateFailingStore {
        fn create_record(&mut self, _record: RuntimeRecord) -> Result<(), StoreError> {
            Err(StoreError::Failed("injected create failure".to_string()))
        }

        fn load_record(&self, instance_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(instance_id)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.save_record(record)
        }
    }

    fn formspec_bindings() -> BindingRegistry {
        let mut bindings = BindingRegistry::new();
        bindings.register(TestAdapter);
        bindings
    }

    fn unavailable_bindings() -> BindingRegistry {
        let mut bindings = BindingRegistry::new();
        bindings.register(UnavailableAdapter);
        bindings
    }

    fn runtime_with_kernel(kernel: KernelDocument) -> WosRuntime {
        WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
    }

    fn manual_formspec_task(
        instance_id: &str,
        ordinal: usize,
        response_mapping_ref: Option<&str>,
    ) -> ActiveTask {
        let task_id = make_task_id(instance_id, ordinal as u64, "review");
        ActiveTask {
            task_id: task_id.clone(),
            task_ref: "review".to_string(),
            status: ActiveTaskStatus::Assigned,
            assigned_actor: Some("reviewer".to_string()),
            contract_ref: Some("reviewForm".to_string()),
            binding: Some("formspec".to_string()),
            definition_url: Some("urn:formspec:review".to_string()),
            definition_version: Some("1.0.0".to_string()),
            prefill_mapping_ref: None,
            response_mapping_ref: response_mapping_ref.map(str::to_string),
            deadline: None,
            impact_level: None,
            context: Some(FormspecTaskContext {
                task_id,
                instance_id: instance_id.to_string(),
                contract_ref: "reviewForm".to_string(),
                definition_url: "urn:formspec:review".to_string(),
                definition_version: "1.0.0".to_string(),
                binding: "formspec".to_string(),
                assigned_actor: "reviewer".to_string(),
                prefill_data: None,
                prefill_mapping_ref: None,
                response_mapping_ref: response_mapping_ref.map(str::to_string),
                deadline: None,
                impact_level: None,
                extensions: Default::default(),
            }),
            last_validation_outcome: None,
            created_at: "2024-03-09T00:00:00Z".to_string(),
            updated_at: "2024-03-09T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn create_instance_and_drain_create_formspec_task() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "start",
                            "target": "open",
                            "actions": [{
                                "action": "createTask",
                                "taskRef": "review",
                                "assignTo": "reviewer",
                                "contractRef": "reviewForm",
                                "responseMappingRef": "urn:mapping:response",
                                "completionEvent": "review.completed",
                                "failureEvent": "review.failed"
                            }]
                        }]
                    }
                }
            },
            "contracts": {
                "reviewForm": {
                    "binding": "formspec",
                    "ref": "urn:formspec:review"
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-1".to_string(),
                definition_url: "urn:test:kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": false })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-1",
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once("case-1").unwrap();
        assert_eq!(result.processed_event.as_deref(), Some("start"));
        assert_eq!(result.created_task_ids.len(), 1);
        assert!(result.created_task_ids[0].starts_with("wos-task:"));
        assert!(result
            .provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::TaskPresented));

        let instance = runtime.load_instance("case-1").unwrap();
        assert_eq!(instance.active_tasks.len(), 1);
        assert_eq!(instance.active_tasks[0].status, ActiveTaskStatus::Assigned);
        assert_eq!(
            instance.active_tasks[0]
                .context
                .as_ref()
                .and_then(|context| context.prefill_data.clone()),
            Some(serde_json::json!({ "approved": false }))
        );
    }

    #[test]
    fn drain_once_reports_event_token_for_same_named_events() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:event-token",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "same",
                            "target": "open"
                        }]
                    }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-token".to_string(),
                definition_url: "urn:test:event-token".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        for token in ["old-token", "target-token"] {
            runtime
                .enqueue_event(
                    "case-token",
                    PendingEvent {
                        event: "same".to_string(),
                        actor_id: Some("reviewer".to_string()),
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: Some(token.to_string()),
                    },
                )
                .unwrap();
        }

        let first = runtime.drain_once("case-token").unwrap();
        assert_eq!(first.processed_event.as_deref(), Some("same"));
        assert_eq!(first.processed_event_token.as_deref(), Some("old-token"));

        let second = runtime.drain_once("case-token").unwrap();
        assert_eq!(second.processed_event.as_deref(), Some("same"));
        assert_eq!(
            second.processed_event_token.as_deref(),
            Some("target-token")
        );
    }

    #[test]
    fn reference_companion_policy_scopes_idempotency_by_instance() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:companion-idempotency",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic"
                    }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel)
            .with_companion_policy(crate::ReferenceCompanionPolicy::default());
        for instance_id in ["case-a", "case-b"] {
            runtime
                .create_instance(CreateInstanceRequest {
                    instance_id: instance_id.to_string(),
                    definition_url: "urn:test:companion-idempotency".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                })
                .unwrap();
            runtime
                .enqueue_event(
                    instance_id,
                    PendingEvent {
                        event: "submit".to_string(),
                        actor_id: Some("reviewer".to_string()),
                        data: Some(serde_json::json!({ "idempotencyKey": "shared-key" })),
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
        }

        let first = runtime.drain_once("case-a").unwrap();
        let second = runtime.drain_once("case-b").unwrap();

        assert!(!first
            .provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup));
        assert!(!second
            .provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup));
    }

    #[test]
    fn create_instance_does_not_present_tasks_if_initial_commit_fails() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:presenter-order",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "onEntry": [{
                            "action": "createTask",
                            "taskRef": "review",
                            "assignTo": "reviewer",
                            "contractRef": "reviewForm",
                            "responseMappingRef": "urn:mapping:response"
                        }]
                    }
                }
            },
            "contracts": {
                "reviewForm": {
                    "binding": "formspec",
                    "ref": "urn:formspec:review"
                }
            }
        }))
        .unwrap();

        let presented = Arc::new(Mutex::new(Vec::<FormspecTaskContext>::new()));
        let presenter = RecordingPresenter {
            presented: presented.clone(),
            dismissed: Arc::new(Mutex::new(Vec::new())),
        };
        let mut runtime = WosRuntime::new(
            CreateFailingStore::default(),
            TestResolver::with_kernel(kernel),
            presenter,
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        );

        let error = runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-1a".to_string(),
                definition_url: "urn:test:presenter-order".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": true })),
            })
            .unwrap_err();

        assert!(matches!(error, RuntimeError::Store(StoreError::Failed(_))));
        assert!(presented.lock().unwrap().is_empty());
    }

    #[test]
    fn create_instance_persists_task_state_before_presentation() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:presented-state-order",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "onEntry": [{
                            "action": "createTask",
                            "taskRef": "review",
                            "assignTo": "reviewer",
                            "contractRef": "reviewForm",
                            "responseMappingRef": "urn:mapping:response"
                        }]
                    }
                }
            },
            "contracts": {
                "reviewForm": {
                    "binding": "formspec",
                    "ref": "urn:formspec:review"
                }
            }
        }))
        .unwrap();

        let store = Arc::new(Mutex::new(InMemoryStore::new()));
        let mut runtime = WosRuntime::new(
            SharedStore(store.clone()),
            TestResolver::with_kernel(kernel),
            StatusCheckingPresenter {
                store: store.clone(),
            },
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        );

        let instance = runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-1a".to_string(),
                definition_url: "urn:test:presented-state-order".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": true })),
            })
            .unwrap();

        assert_eq!(instance.active_tasks.len(), 1);
        assert_eq!(instance.active_tasks[0].status, ActiveTaskStatus::Assigned);
    }

    #[test]
    fn create_instance_and_restore_preserves_timer_duration_metadata() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:timer-metadata",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "waiting",
                "states": {
                    "waiting": {
                        "type": "atomic",
                        "onEntry": [{
                            "action": "startTimer",
                            "timerId": "t1",
                            "duration": "PT2H",
                            "event": "$timeout.review"
                        }]
                    }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel.clone());
        let instance = runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-1b".to_string(),
                definition_url: "urn:test:timer-metadata".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        assert_eq!(instance.timers.len(), 1);
        assert_eq!(instance.timers[0].duration_iso.as_deref(), Some("PT2H"));
        assert_eq!(instance.timers[0].duration_ms, Some(7_200_000));

        let record = runtime.store.load_record("case-1b").unwrap();
        let evaluator = Evaluator::from_instance(kernel, &record.instance, 1_710_000_000_000)
            .expect("restore evaluator");
        let timer = evaluator.timers().iter().next().expect("restored timer");
        assert_eq!(timer.duration_iso, "PT2H");
        assert_eq!(timer.duration_ms, 7_200_000);
    }

    #[test]
    fn submit_task_response_completes_and_emits_event() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:submit-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic"
                    }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-2".to_string(),
                definition_url: "urn:test:submit-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-2").unwrap();
        let mut task = manual_formspec_task("case-2", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        task.extensions.insert(
            COMPLETION_EVENT_EXTENSION_KEY.to_string(),
            serde_json::Value::String("review.completed".to_string()),
        );
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
                "reviewer",
                Some("token-1"),
            )
            .unwrap();

        match result {
            TaskSubmissionResult::Completed {
                case_mutated,
                emitted_event,
                ..
            } => {
                assert!(case_mutated);
                assert_eq!(emitted_event.as_deref(), Some("review.completed"));
            }
            other => panic!("expected completed result, got {other:?}"),
        }

        let instance = runtime.load_instance("case-2").unwrap();
        assert!(instance.active_tasks.is_empty());
        assert_eq!(instance.case_state["decision"], serde_json::json!(true));
        assert_eq!(instance.pending_events.len(), 1);
        assert_eq!(instance.pending_events[0].event, "review.completed");
    }

    #[test]
    fn persist_task_draft_stores_artifact_without_mutating_case_state() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:draft-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-3".to_string(),
                definition_url: "urn:test:draft-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "decision": "pending" })),
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-3").unwrap();
        let task = manual_formspec_task("case-3", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let draft = runtime
            .persist_task_draft(
                &task_id,
                serde_json::json!({
                    "status": "stopped",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": false }
                }),
                "reviewer",
                Some("draft-token"),
            )
            .unwrap();

        let record = runtime.store.load_record("case-3").unwrap();
        assert!(record.artifacts.contains_key(&draft.artifact_id));
        assert_eq!(
            record.instance.case_state["decision"],
            serde_json::json!("pending")
        );
        assert_eq!(record.instance.active_tasks.len(), 1);
        assert!(record
            .provenance_log
            .iter()
            .any(|entry| entry.record_kind == ProvenanceKind::TaskDraftPersisted));
    }

    #[test]
    fn dismiss_task_records_provenance_and_leaves_task_resumable() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:dismiss-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-4".to_string(),
                definition_url: "urn:test:dismiss-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-4").unwrap();
        let task = manual_formspec_task("case-4", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        runtime.dismiss_task(&task_id, "snoozed").unwrap();

        let record = runtime.store.load_record("case-4").unwrap();
        assert_eq!(record.instance.active_tasks.len(), 1);
        assert!(record
            .provenance_log
            .iter()
            .any(|entry| entry.record_kind == ProvenanceKind::TaskDismissed));
    }

    #[test]
    fn submit_task_response_replays_same_actor_token_after_completion() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:replay-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-5".to_string(),
                definition_url: "urn:test:replay-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-5").unwrap();
        let mut task = manual_formspec_task("case-5", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        task.extensions.insert(
            COMPLETION_EVENT_EXTENSION_KEY.to_string(),
            serde_json::Value::String("review.completed".to_string()),
        );
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let response = serde_json::json!({
            "status": "completed",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true }
        });
        let first = runtime
            .submit_task_response(&task_id, response.clone(), "reviewer", Some("replay-1"))
            .unwrap();
        let second = runtime
            .submit_task_response(&task_id, response, "reviewer", Some("replay-1"))
            .unwrap();

        match (&first, &second) {
            (
                TaskSubmissionResult::Completed {
                    artifact_id: first_artifact_id,
                    ..
                },
                TaskSubmissionResult::Completed {
                    artifact_id: second_artifact_id,
                    ..
                },
            ) => assert_eq!(first_artifact_id, second_artifact_id),
            other => panic!("expected replayed completed results, got {other:?}"),
        }

        let record = runtime.store.load_record("case-5").unwrap();
        assert_eq!(record.instance.pending_events.len(), 1);
        assert_eq!(
            record
                .provenance_log
                .iter()
                .filter(|entry| entry.record_kind == ProvenanceKind::TaskCompleted)
                .count(),
            1
        );
    }

    #[test]
    fn submit_task_response_same_token_different_actor_does_not_replay() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:actor-replay-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-6".to_string(),
                definition_url: "urn:test:actor-replay-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-6").unwrap();
        let task = manual_formspec_task("case-6", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let response = serde_json::json!({
            "status": "in-progress",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true }
        });
        let first = runtime
            .submit_task_response(&task_id, response.clone(), "reviewer", Some("token-shared"))
            .unwrap();
        let second = runtime
            .submit_task_response(&task_id, response, "delegate", Some("token-shared"))
            .unwrap();

        assert!(matches!(
            first,
            TaskSubmissionResult::Rejected { ref code }
            if code == "taskResponseStatusNotCompleted"
        ));
        assert!(matches!(
            second,
            TaskSubmissionResult::Rejected { ref code }
            if code == "taskResponseStatusNotCompleted"
        ));

        let record = runtime.store.load_record("case-6").unwrap();
        assert_eq!(
            record
                .provenance_log
                .iter()
                .filter(|entry| entry.record_kind == ProvenanceKind::TaskResponseRejected)
                .count(),
            2
        );
    }

    #[test]
    fn drain_once_processes_due_timer_via_queued_timeout_event() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:timer-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "waiting",
                "states": {
                    "waiting": {
                        "type": "atomic",
                        "onEntry": [{
                            "action": "startTimer",
                            "timerId": "t1",
                            "duration": "PT0S",
                            "event": "$timeout.review"
                        }],
                        "transitions": [{
                            "event": "$timeout.review",
                            "target": "timed_out"
                        }]
                    },
                    "timed_out": {
                        "type": "final"
                    }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        let instance = runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-7".to_string(),
                definition_url: "urn:test:timer-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        assert_eq!(instance.pending_events.len(), 0);
        assert_eq!(instance.timers.len(), 1);

        let result = runtime.drain_once("case-7").unwrap();
        assert_eq!(result.processed_event.as_deref(), Some("$timeout.review"));
        assert!(result
            .provenance
            .iter()
            .any(|entry| entry.record_kind == ProvenanceKind::TimerFired));

        let instance = runtime.load_instance("case-7").unwrap();
        assert!(instance.configuration.contains(&"timed_out".to_string()));
        assert!(instance.pending_events.is_empty());
        assert!(instance.timers.is_empty());
    }

    #[test]
    fn drain_once_unsupported_binding_fails_deterministically() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:unsupported-binding",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "start",
                            "target": "open",
                            "actions": [{
                                "action": "createTask",
                                "taskRef": "review",
                                "assignTo": "reviewer",
                                "contractRef": "reviewForm"
                            }]
                        }]
                    }
                }
            },
            "contracts": {
                "reviewForm": {
                    "binding": "json-schema",
                    "ref": "urn:contracts:review"
                }
            }
        }))
        .unwrap();

        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            BindingRegistry::new(),
        );
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-8".to_string(),
                definition_url: "urn:test:unsupported-binding".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-8",
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let error = runtime.drain_once("case-8").unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::UnsupportedBinding(ref binding) if binding == "json-schema"
        ));
    }

    #[test]
    fn drain_once_save_failure_leaves_store_unchanged() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:atomic-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "start",
                            "target": "open",
                            "actions": [{
                                "action": "createTask",
                                "taskRef": "review",
                                "assignTo": "reviewer",
                                "contractRef": "reviewForm"
                            }]
                        }]
                    }
                }
            },
            "contracts": {
                "reviewForm": {
                    "binding": "formspec",
                    "ref": "urn:formspec:review"
                }
            }
        }))
        .unwrap();

        let mut runtime = WosRuntime::new(
            FailingStore::new(2),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        );
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-9".to_string(),
                definition_url: "urn:test:atomic-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-9",
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();
        let provenance_position_before_failure =
            runtime.load_instance("case-9").unwrap().provenance_position;

        let error = runtime.drain_once("case-9").unwrap_err();
        assert!(matches!(error, RuntimeError::Store(StoreError::Failed(_))));

        let instance = runtime.load_instance("case-9").unwrap();
        assert_eq!(instance.pending_events.len(), 1);
        assert!(instance.active_tasks.is_empty());
        assert_eq!(
            instance.provenance_position,
            provenance_position_before_failure
        );
    }

    #[test]
    fn submit_task_response_returns_retryable_error_when_processor_unavailable() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:unavailable-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            unavailable_bindings(),
        );
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-10".to_string(),
                definition_url: "urn:test:unavailable-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-10").unwrap();
        let provenance_len_before_failure = record.provenance_log.len();
        let task = manual_formspec_task("case-10", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let error = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
                "reviewer",
                Some("retry-token"),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::Binding(BindingError::ProcessorUnavailable(_))
        ));

        let record = runtime.store.load_record("case-10").unwrap();
        assert_eq!(record.instance.active_tasks.len(), 1);
        assert_eq!(record.provenance_log.len(), provenance_len_before_failure);
    }

    #[test]
    fn drain_once_invokes_service_and_persists_step_result() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:service-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "verify",
                            "target": "open",
                            "actions": [{
                                "action": "invokeService",
                                "serviceRef": "verificationSystem",
                                "idempotencyKey": "verify-1",
                                "contractRef": "verificationContract"
                            }]
                        }]
                    }
                }
            },
            "contracts": {
                "verificationContract": {
                    "binding": "formspec",
                    "ref": "urn:formspec:verify"
                }
            }
        }))
        .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "result": "pass",
            "score": 92
        }));
        let calls = service.calls.clone();
        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            service,
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        );

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-service".to_string(),
                definition_url: "urn:test:service-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-service",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("verificationSystem".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once("case-service").unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::StepResultPersisted
                && record.data.as_ref().and_then(|data| data.get("serviceRef"))
                    == Some(&serde_json::json!("verificationSystem"))
        }));
        assert!(result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ContractValidation
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("contractRef"))
                    == Some(&serde_json::json!("verificationContract"))
                && record.data.as_ref().and_then(|data| data.get("valid"))
                    == Some(&serde_json::json!(true))
        }));
    }

    #[test]
    fn drain_once_consumes_integration_profile_binding_and_replays_persisted_result() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:integration-profile-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "verify",
                            "target": "open",
                            "actions": [{
                                "action": "invokeService",
                                "serviceRef": "eligibilityCheck"
                            }]
                        }]
                    }
                }
            }
        }))
        .unwrap();
        let profile: crate::IntegrationProfileDocument =
            serde_json::from_value(serde_json::json!({
                "$wosIntegrationProfile": "1.0",
                "targetWorkflow": {
                    "url": "urn:test:integration-profile-kernel",
                    "compatibleVersions": ">=1.0.0 <2.0.0"
                },
                "bindings": {
                    "eligibilityCheck": {
                        "type": "request-response",
                        "interface": { "$ref": "urn:openapi:eligibility" },
                        "operation": "checkEligibility",
                        "requestContract": {
                            "definitionRef": "urn:contracts:eligibility-request"
                        },
                        "responseContract": {
                            "definitionRef": "urn:contracts:eligibility-response"
                        },
                        "inputMapping": {
                            "applicantSSN": "caseFile.application.ssn",
                            "householdSize": "caseFile.application.householdSize",
                            "checkType": "if caseFile.application.householdSize > 2 then 'large' else 'small'",
                            "submittedBy": "event.actorId"
                        },
                        "outputBinding": {
                            "caseFile.eligibility.result": "$.decisions[0].result",
                            "caseFile.eligibility.checkedAt": "$.decisions[0].checkedAt"
                        },
                        "idempotencyKeyExpression": "caseFile.application.id & '-' & event.actorId"
                    }
                }
            }))
            .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "decisions": [{
                "result": "eligible",
                "checkedAt": "2026-04-14T10:00:00Z"
            }]
        }));
        let calls = service.calls.clone();
        let invocations = service.invocations.clone();
        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            service,
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
        .with_integration_profile(profile);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-integration".to_string(),
                definition_url: "urn:test:integration-profile-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({
                    "application": {
                        "id": "app-123",
                        "ssn": "123-45-6789",
                        "householdSize": 3
                    }
                })),
            })
            .unwrap();

        runtime
            .enqueue_event(
                "case-integration",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let first_result = runtime.drain_once("case-integration").unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        {
            let invocations = invocations.lock().unwrap();
            assert_eq!(invocations.len(), 1);
            assert_eq!(invocations[0].service_ref, "eligibilityCheck");
            assert_eq!(
                invocations[0].input,
                serde_json::json!({
                    "applicantSSN": "123-45-6789",
                    "householdSize": 3,
                    "checkType": "large",
                    "submittedBy": "system"
                })
            );
            assert_eq!(
                invocations[0].idempotency_key.as_deref(),
                Some("app-123-system")
            );
        }

        let instance = runtime.load_instance("case-integration").unwrap();
        assert_eq!(
            instance.case_state["eligibility"]["result"],
            serde_json::json!("eligible")
        );
        assert_eq!(
            instance.case_state["eligibility"]["checkedAt"],
            serde_json::json!("2026-04-14T10:00:00Z")
        );
        assert!(first_result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ContractValidation
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("contractRef"))
                    == Some(&serde_json::json!("urn:contracts:eligibility-request"))
        }));
        assert!(first_result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ContractValidation
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("contractRef"))
                    == Some(&serde_json::json!("urn:contracts:eligibility-response"))
        }));
        assert!(first_result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::DataMapping
                && record.data.as_ref().and_then(|data| data.get("serviceRef"))
                    == Some(&serde_json::json!("eligibilityCheck"))
        }));

        runtime
            .enqueue_event(
                "case-integration",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let second_result = runtime.drain_once("case-integration").unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        {
            let invocations = invocations.lock().unwrap();
            assert_eq!(invocations.len(), 1);
        }
        assert!(second_result
            .provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup));
        assert!(!second_result
            .provenance
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::StepResultPersisted));
    }

    #[test]
    fn drain_once_rejects_integration_profile_target_mismatch() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:integration-profile-kernel",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "verify",
                            "target": "open",
                            "actions": [{
                                "action": "invokeService",
                                "serviceRef": "eligibilityCheck"
                            }]
                        }]
                    }
                }
            }
        }))
        .unwrap();
        let profile: crate::IntegrationProfileDocument =
            serde_json::from_value(serde_json::json!({
                "$wosIntegrationProfile": "1.0",
                "targetWorkflow": {
                    "url": "urn:test:different-kernel",
                    "compatibleVersions": ">=1.0.0 <2.0.0"
                },
                "bindings": {
                    "eligibilityCheck": {
                        "type": "request-response",
                        "interface": { "$ref": "urn:openapi:eligibility" },
                        "operation": "checkEligibility",
                        "inputMapping": {
                            "applicantSSN": "caseFile.application.ssn"
                        }
                    }
                }
            }))
            .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "result": "eligible"
        }));
        let calls = service.calls.clone();
        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            service,
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
        .with_integration_profile(profile);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-target-mismatch".to_string(),
                definition_url: "urn:test:integration-profile-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({
                    "application": {
                        "id": "app-123",
                        "ssn": "123-45-6789"
                    }
                })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-target-mismatch",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let error = runtime.drain_once("case-target-mismatch").unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::Integration(ref message) if message.contains("targets")
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn drain_once_rejects_unsupported_integration_profile_binding_kind() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:unsupported-integration-binding",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "verify",
                            "target": "open",
                            "actions": [{
                                "action": "invokeService",
                                "serviceRef": "legacyTool"
                            }]
                        }]
                    }
                }
            }
        }))
        .unwrap();
        let profile: crate::IntegrationProfileDocument =
            serde_json::from_value(serde_json::json!({
                "$wosIntegrationProfile": "1.0",
                "targetWorkflow": {
                    "url": "urn:test:unsupported-integration-binding",
                    "compatibleVersions": ">=1.0.0 <2.0.0"
                },
                "bindings": {
                    "legacyTool": {
                        "type": "tool",
                        "invocation": {
                            "method": "command-line",
                            "command": "/usr/bin/legacy-tool"
                        },
                        "inputMapping": {
                            "payload": "caseFile.application.id"
                        }
                    }
                }
            }))
            .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "result": "unused"
        }));
        let calls = service.calls.clone();
        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            service,
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
        .with_integration_profile(profile);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-unsupported-integration-binding".to_string(),
                definition_url: "urn:test:unsupported-integration-binding".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({
                    "application": {
                        "id": "app-123"
                    }
                })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-unsupported-integration-binding",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let error = runtime
            .drain_once("case-unsupported-integration-binding")
            .unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::UnsupportedBindingKind(crate::integration::IntegrationBindingKind::Tool)
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn drain_once_rejects_invalid_integration_profile_idempotency_expression() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:invalid-idempotency-expression",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "verify",
                            "target": "open",
                            "actions": [{
                                "action": "invokeService",
                                "serviceRef": "eligibilityCheck"
                            }]
                        }]
                    }
                }
            }
        }))
        .unwrap();
        let profile: crate::IntegrationProfileDocument =
            serde_json::from_value(serde_json::json!({
                "$wosIntegrationProfile": "1.0",
                "targetWorkflow": {
                    "url": "urn:test:invalid-idempotency-expression",
                    "compatibleVersions": ">=1.0.0 <2.0.0"
                },
                "bindings": {
                    "eligibilityCheck": {
                        "type": "request-response",
                        "interface": { "$ref": "urn:openapi:eligibility" },
                        "operation": "checkEligibility",
                        "inputMapping": {
                            "applicantSSN": "caseFile.application.ssn"
                        },
                        "idempotencyKeyExpression": "caseFile.application.missing"
                    }
                }
            }))
            .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "result": "eligible"
        }));
        let calls = service.calls.clone();
        let mut runtime = WosRuntime::new(
            InMemoryStore::new(),
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            service,
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
        .with_integration_profile(profile);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-invalid-idempotency".to_string(),
                definition_url: "urn:test:invalid-idempotency-expression".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({
                    "application": {
                        "id": "app-123",
                        "ssn": "123-45-6789"
                    }
                })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-invalid-idempotency",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let error = runtime.drain_once("case-invalid-idempotency").unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::Integration(ref message)
                if message.contains("resolved to no value")
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
