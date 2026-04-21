// Rust guideline compliant 2026-02-21

//! Runtime command surface for WOS processors.

mod actions;
mod tasks;
mod timers;

use std::error::Error as StdError;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use wos_core::business_calendar::BusinessCalendarDocument;
use wos_core::eval::{Evaluator, GuardEvaluation, ObservedTransition};
use wos_core::instance::{
    CaseInstance, FormspecTaskContext, InstanceStatus, PendingEvent,
};
use wos_core::model::kernel::{ActorKind, ImpactLevel, KernelDocument};
use wos_core::provenance::{ProvenanceAuditTier, ProvenanceKind, ProvenanceRecord};
use wos_core::traits::{
    AccessControl, ContractValidator, DocumentResolver, ExternalService, TaskPresenter,
};

use self::timers::{
    annotate_timer_created_with_calendar_version, annotate_timer_created_with_convergence_error,
    materialize_due_timers, timers_to_state,
};
use crate::binding::{BindingError, BindingRegistry, SubmissionValidation};
use crate::durable::DurableRuntime;
use crate::integration::IntegrationProfileDocument;
use crate::milestones::evaluate_milestones;
use crate::store::{RuntimeRecord, RuntimeStore, StoreError};

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
    /// Guard expressions evaluated during this step (teaching signal, §5.3).
    ///
    /// Every transition whose `guard` was tested — pass or fail — contributes
    /// one entry. Short-circuited false guards on transitions that did not
    /// fire are included so conformance traces can surface "this guard
    /// evaluated false" as the reason an expected transition didn't happen.
    pub guard_evaluations: Vec<GuardEvaluation>,
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
    /// Runtime clock at the start of event processing.
    pub now_ms: u64,
    /// RFC 3339 rendering of `now_ms`.
    pub now_iso: String,
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
            timers_to_state(evaluator.timers(), self.business_calendar.as_ref())?;
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
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: InstanceStatus::Active,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
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
        annotate_timer_created_with_convergence_error(
            &mut appended_provenance,
            &convergence_error_ids,
        );
        let actions = evaluator.take_executed_actions();
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);
        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);
        populate_provenance_record_fields(
            &mut appended_provenance,
            &kernel,
            &record.instance.definition_version,
        );
        stamp_provenance(&mut appended_provenance, &now_iso);
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
                // Resolve kernel for SP §5.3/§5.4 field population (due-timer
                // materialization path). The kernel is always resolvable here
                // because the instance is persisted.
                let kernel = self.resolver.resolve_kernel(
                    &record.instance.definition_url,
                    &record.instance.definition_version,
                )?;
                populate_provenance_record_fields(
                    &mut appended_provenance,
                    &kernel,
                    &record.instance.definition_version,
                );
                stamp_provenance(&mut appended_provenance, &now_iso);
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
            guard_evaluations: Vec::new(),
        };

        let drained_event_name = event.event.clone();
        let decision = self.companion_policy.evaluate_event(RuntimeEventContext {
            kernel: kernel.clone(),
            instance: record.instance.clone(),
            event,
            now_ms,
            now_iso: now_iso.clone(),
        })?;
        appended_provenance.extend(decision.provenance);

        let Some(event) = decision.event else {
            populate_provenance_record_fields(
                &mut appended_provenance,
                &kernel,
                &record.instance.definition_version,
            );
            stamp_provenance(&mut appended_provenance, &now_iso);
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
        runtime_result.guard_evaluations = evaluator.take_guard_evaluations();

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
            timers_to_state(evaluator.timers(), self.business_calendar.as_ref())?;
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(
            &mut appended_provenance,
            &convergence_error_ids,
        );
        record.instance.timers = timer_states;
        record.instance.history_store = evaluator.history_store().clone();
        record.instance.updated_at = now_iso.clone();

        let case_state_can_mutate_explicitly = record
            .provenance_log
            .iter()
            .chain(appended_provenance.iter())
            .any(|record| record.record_kind == ProvenanceKind::CaseStateMutation);
        if !runtime_result.transitions.is_empty() && case_state_can_mutate_explicitly {
            appended_provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::StateTransition,
                timestamp: String::new(),
                actor_id: event.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: Some(event.event.clone()),
                data: Some(serde_json::json!({ "caseStateUnchangedByTransition": true })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
                outcome: None,
            });
        }

        // Milestone firing: evaluate after the event's transition tree completes
        // (including all onEntry/onExit setData), before side-effect realization
        // (createTask/emitEvent) that would enqueue follow-on events (Kernel S4.13).
        // Records are appended in lexicographic milestone-id order so the provenance
        // stream is deterministic.
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

        // Stamp the drain's event onto policy-application provenance
        // records that left `event = None`. The governance / AI / autonomy
        // / confidence constructors all set `event: None` because they
        // don't carry the triggering event in their construction context,
        // but the trace teaching-signal (§5.3) needs this association so
        // conformance traces can scope `policies_applied` to the right
        // trace step. Scoped strictly to `is_policy_application()` kinds
        // — kernel-layer records (state transitions, action executions)
        // already set `event` correctly in their constructors and the
        // field is load-bearing there (see `ProvenanceRecord::state_transition`).
        for prov_record in &mut appended_provenance {
            if prov_record.event.is_none() && prov_record.record_kind.is_policy_application() {
                prov_record.event = Some(drained_event_name.clone());
            }
        }
        populate_provenance_record_fields(
            &mut appended_provenance,
            &kernel,
            &record.instance.definition_version,
        );
        stamp_provenance(&mut appended_provenance, &now_iso);
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
}

impl DurableRuntime for WosRuntime {
    fn create_instance(
        &mut self,
        request: CreateInstanceRequest,
    ) -> Result<CaseInstance, RuntimeError> {
        WosRuntime::create_instance(self, request)
    }

    fn load_instance(&self, instance_id: &str) -> Result<CaseInstance, RuntimeError> {
        WosRuntime::load_instance(self, instance_id)
    }

    fn enqueue_event(
        &mut self,
        instance_id: &str,
        event: PendingEvent,
    ) -> Result<(), RuntimeError> {
        WosRuntime::enqueue_event(self, instance_id, event)
    }

    fn drain_once(&mut self, instance_id: &str) -> Result<DrainOnceResult, RuntimeError> {
        WosRuntime::drain_once(self, instance_id)
    }

    fn drain_until_idle(
        &mut self,
        instance_id: &str,
    ) -> Result<Vec<DrainOnceResult>, RuntimeError> {
        WosRuntime::drain_until_idle(self, instance_id)
    }

    fn persist_task_draft(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<PersistDraftResult, RuntimeError> {
        WosRuntime::persist_task_draft(self, task_id, response, actor_id, idempotency_token)
    }

    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), RuntimeError> {
        WosRuntime::dismiss_task(self, task_id, reason)
    }

    fn submit_task_response(
        &mut self,
        task_id: &str,
        response: serde_json::Value,
        actor_id: &str,
        idempotency_token: Option<&str>,
    ) -> Result<TaskSubmissionResult, RuntimeError> {
        WosRuntime::submit_task_response(self, task_id, response, actor_id, idempotency_token)
    }

    fn load_provenance_window(
        &self,
        instance_id: &str,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        WosRuntime::load_provenance_window(self, instance_id, cursor, limit)
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

/// Stamp every record whose `timestamp` is empty with `now_iso`.
///
/// Records that already carry a timestamp (e.g. stamped earlier in the pipeline
/// or restored from persistence) are left untouched. This is the single
/// authoritative point where an empty `ProvenanceRecord::timestamp` is filled
/// in on the append path; exporters (PROV-O, XES, OCEL) downstream can treat
/// any record surfaced from the runtime as having a non-empty timestamp.
pub fn stamp_provenance(records: &mut [ProvenanceRecord], now_iso: &str) {
    for record in records {
        if record.timestamp.is_empty() {
            record.timestamp = now_iso.to_string();
        }
    }
}

/// Populate the eight push-stamped Semantic Profile fields on `records`
/// immediately before persistence (SP §5.3, §5.4, §5.5, §6.3, §6.5).
///
/// This is the sole append-path site where these fields are filled in, so
/// every record handed to the store downstream carries the full SP-required
/// shape. Each field is set only when it is currently `None` / empty — the
/// same push-stamped discipline as [`stamp_provenance`]. Callers MUST invoke
/// this before `stamp_provenance` (timestamp is independent and stamped last).
///
/// The populator intentionally does NOT set `timestamp`. Callers either call
/// [`stamp_provenance`] on the same records immediately after the populate
/// pass (the common path), OR pre-assign `timestamp` themselves before the
/// record enters the populator (e.g. `persist_task_draft`, `dismiss_task`,
/// `record_submission_rejection`, which stamp from their ambient `now_iso`).
/// Both patterns satisfy the "every stored record has a timestamp" invariant.
///
/// The populator mirrors the AI Integration-aware actor lookup used by
/// `integration_handlers::request_response` (see its `actor_kind_to_string`
/// site): we resolve `actor_id` against the kernel's declared `actors` and
/// map `ActorKind::Human → "human"`, `ActorKind::System → "system"`. Records
/// whose `actor_id` is not in the kernel registry keep `actor_type = None`
/// (SP §5.3 "omit, do not default"). The `"agent"` variant is reserved for
/// AI Integration agent-registry resolution (out of scope here —
// TODO(spec-upstream) below).
pub fn populate_provenance_record_fields(
    records: &mut [ProvenanceRecord],
    kernel: &KernelDocument,
    definition_version: &str,
) {
    for record in records {
        // Tier classification (SP §5.4, §6.5).
        if record.audit_layer.is_none() {
            record.audit_layer = Some(
                ProvenanceAuditTier::from(record.record_kind)
                    .as_str()
                    .to_string(),
            );
        }

        // Actor type (SP §5.3, §5.5, §6.3).
        if record.actor_type.is_none()
            && let Some(actor_id) = record.actor_id.as_deref()
            && let Some(actor) = kernel.actors.iter().find(|a| a.id == actor_id)
        {
            record.actor_type = Some(match actor.kind {
                ActorKind::Human => "human".to_string(),
                ActorKind::System => "system".to_string(),
                // TODO(spec-upstream): map ActorKind::Agent → "agent" once
                // the AI Integration agent registry lookup is threaded into
                // the runtime context. ActorKind today is Human | System only.
            });
        }

        // Definition version (SP §5.3, §6.3).
        if record.definition_version.is_none() && !definition_version.is_empty() {
            record.definition_version = Some(definition_version.to_string());
        }

        // Lifecycle state (SP §5.3, §6.3): promote from record-specific sources.
        if record.lifecycle_state.is_none() {
            match record.record_kind {
                ProvenanceKind::StateTransition => {
                    // The pre-transition state IS the lifecycle state at
                    // action time (the event fired while the instance
                    // occupied `from_state`).
                    if let Some(from) = record.from_state.as_deref() {
                        record.lifecycle_state = Some(from.to_string());
                    }
                }
                ProvenanceKind::CaseStateMutation => {
                    // `case_state_mutation` embeds the lifecycle state in `data`.
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

        // Inputs / outputs (SP §5.3, §6.3) — only for record kinds that have
        // identifiable entity relationships. Other kinds stay empty.
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
                    // SP §5.3 outputs are scalar entity references. JSON string
                    // scalars must appear unquoted (so `"approved"` → `approved`,
                    // matching the unquoted form of numbers/bools). Other JSON
                    // shapes (number, bool, null, object, array) fall back to
                    // the JSON serialization, which already lacks surrounding
                    // quotes for primitives.
                    record.outputs = vec![stringify_scalar(new_value)];
                }
            }
            _ => {}
        }

        // Digests (SP §5.3, §6.3) — computed last, from the final inputs/outputs.
        if record.input_digest.is_none() {
            record.input_digest = digest_of(&record.inputs);
        }
        if record.output_digest.is_none() {
            record.output_digest = digest_of(&record.outputs);
        }
    }
}

/// Stringify a JSON scalar for inclusion in `inputs`/`outputs` (SP §5.3).
///
/// JSON strings are emitted as their raw value (without surrounding quotes);
/// everything else falls back to the JSON serialization — which for numbers,
/// bools, and null is already unquoted, and for composite shapes (objects,
/// arrays) preserves the embedded structure for downstream exporters.
///
/// This keeps the stringified form consistent across scalar JSON types:
/// `Value::String("x")` → `"x"`, `Value::Number(42)` → `"42"`.
fn stringify_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// SHA-256 hex digest of the JSON-serialized `items` vector.
///
/// Returns `None` when the vector is empty (per SP §5.3, digests are only
/// emitted when the corresponding inputs/outputs are present).
fn digest_of(items: &[String]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    use sha2::{Digest, Sha256};
    let payload = serde_json::to_string(items).unwrap_or_default();
    Some(format!("{:x}", Sha256::digest(payload.as_bytes())))
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

fn make_task_id(instance_id: &str, ordinal: u64, task_ref: &str) -> String {
    let encoded_instance_id = URL_SAFE_NO_PAD.encode(instance_id);
    format!("wos-task:{encoded_instance_id}:{ordinal}:{task_ref}")
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        });
        provenance.push(ProvenanceRecord {
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
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
                transition_tags: Vec::new(),
                case_file_snapshot: None,
                outcome: None,
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
    use crate::DurableRuntime;
    use wos_core::instance::{ActiveTask, ActiveTaskStatus, ValidationOutcome};
    use wos_core::traits::{DocumentResolver, ExternalService, TaskPresenter};

    #[test]
    fn stamp_provenance_fills_empty_timestamps_only() {
        let mut records = vec![
            ProvenanceRecord::state_transition("a", "b", "ev", None),
            ProvenanceRecord::state_transition("b", "c", "ev", None),
        ];
        records[0].timestamp = "2020-01-01T00:00:00Z".to_string();

        stamp_provenance(&mut records, "2026-04-15T12:00:00Z");

        assert_eq!(records[0].timestamp, "2020-01-01T00:00:00Z");
        assert_eq!(records[1].timestamp, "2026-04-15T12:00:00Z");
    }

    #[test]
    fn stamp_provenance_noop_on_empty_slice() {
        let mut records: Vec<ProvenanceRecord> = Vec::new();
        stamp_provenance(&mut records, "2026-04-15T12:00:00Z");
        assert!(records.is_empty());
    }

    /// Build a minimal kernel with configurable actors for populator tests.
    /// Keeps each test self-contained without dragging in the full DSL fixtures.
    fn kernel_with_actors(version: &str, actors: serde_json::Value) -> KernelDocument {
        serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:populator",
            "version": version,
            "actors": actors,
            "lifecycle": {
                "initialState": "Draft",
                "states": {
                    "Draft": { "type": "atomic" }
                }
            }
        }))
        .unwrap()
    }

    #[test]
    fn audit_layer_stamped_by_runtime_pass() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![
            ProvenanceRecord::state_transition("Draft", "Submitted", "submit", None),
            ProvenanceRecord {
                record_kind: ProvenanceKind::NarrativeTierRecorded,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: None,
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
                outcome: None,
            },
        ];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].audit_layer.as_deref(), Some("facts"));
        assert_eq!(records[1].audit_layer.as_deref(), Some("narrative"));
    }

    #[test]
    fn actor_type_human_from_registry() {
        let kernel = kernel_with_actors(
            "1.0.0",
            serde_json::json!([{ "id": "reviewer", "type": "human" }]),
        );
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            Some("reviewer"),
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].actor_type.as_deref(), Some("human"));
    }

    #[test]
    fn actor_type_system_from_registry() {
        let kernel = kernel_with_actors(
            "1.0.0",
            serde_json::json!([{ "id": "scheduler", "type": "system" }]),
        );
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "tick",
            Some("scheduler"),
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].actor_type.as_deref(), Some("system"));
    }

    #[test]
    fn actor_type_absent_when_no_actor_id() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::timer_fired("timer-1", "deadline")];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        // No actor_id on timer_fired — actor_type stays None, not defaulted to "system".
        assert!(records[0].actor_type.is_none());
    }

    #[test]
    fn actor_type_absent_when_actor_not_in_registry() {
        // Unknown actor ids are NOT defaulted — the spec says omit, not default.
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            Some("unknown-actor"),
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert!(records[0].actor_type.is_none());
    }

    #[test]
    fn lifecycle_state_set_to_from_state_on_transition() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            None,
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].lifecycle_state.as_deref(), Some("Draft"));
    }

    #[test]
    fn lifecycle_state_promoted_from_case_state_mutation_data() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::case_state_mutation(
            "/amount",
            &serde_json::json!(42),
            None,
            "UnderReview",
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].lifecycle_state.as_deref(), Some("UnderReview"));
    }

    #[test]
    fn definition_version_propagated_from_kernel_document() {
        let kernel = kernel_with_actors("2.7.3", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            None,
        )];

        populate_provenance_record_fields(&mut records, &kernel, "2.7.3");

        assert_eq!(records[0].definition_version.as_deref(), Some("2.7.3"));
    }

    #[test]
    fn inputs_outputs_set_for_state_transition() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            None,
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].inputs, vec!["submit".to_string()]);
        assert_eq!(records[0].outputs, vec!["Submitted".to_string()]);
    }

    #[test]
    fn determination_transition_emits_case_file_snapshot() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:determination-snapshot",
            "version": "1.0.0",
            "actors": [{ "id": "reviewer", "type": "human" }],
            "lifecycle": {
                "initialState": "review",
                "states": {
                    "review": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "decide",
                            "target": "decided",
                            "tags": ["determination"]
                        }]
                    },
                    "decided": { "type": "final" }
                }
            }
        }))
        .unwrap();
        let mut runtime = runtime_with_kernel(kernel);
        let case_state = serde_json::json!({
            "applicantId": "A-123",
            "income": 17500,
            "eligible": true
        });

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-determination".to_string(),
                definition_url: "urn:test:determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(case_state.clone()),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-determination",
                PendingEvent {
                    event: "decide".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: "2026-04-19T00:00:00Z".to_string(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once("case-determination").unwrap();
        let transition = result
            .provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::StateTransition)
            .expect("state transition provenance");
        let snapshot = transition
            .case_file_snapshot
            .as_ref()
            .expect("determination transition captures case state");

        assert_eq!(snapshot.value, case_state);
        assert_eq!(
            snapshot.jcs_canonical,
            r#"{"applicantId":"A-123","eligible":true,"income":17500}"#
        );
        assert_eq!(snapshot.sha256.len(), 64);
    }

    #[test]
    fn recursive_join_determination_uses_current_transition_case_state() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:join-determination-snapshot",
            "version": "1.0.0",
            "actors": [{ "id": "reviewer", "type": "human" }],
            "lifecycle": {
                "initialState": "parallelReview",
                "states": {
                    "parallelReview": {
                        "type": "parallel",
                        "regions": {
                            "human": {
                                "initialState": "humanReview",
                                "states": {
                                    "humanReview": {
                                        "type": "atomic",
                                        "transitions": [{
                                            "event": "completeReview",
                                            "target": "humanDone",
                                            "actions": [{
                                                "action": "setData",
                                                "path": "caseFile.reviewScore",
                                                "value": 100
                                            }]
                                        }]
                                    },
                                    "humanDone": { "type": "final" }
                                }
                            },
                            "system": {
                                "initialState": "systemDone",
                                "states": {
                                    "systemDone": { "type": "final" }
                                }
                            }
                        },
                        "transitions": [{
                            "event": "$join",
                            "target": "decided",
                            "tags": ["determination"]
                        }]
                    },
                    "decided": { "type": "final" }
                }
            }
        }))
        .unwrap();
        let mut runtime = runtime_with_kernel(kernel);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-join-determination".to_string(),
                definition_url: "urn:test:join-determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "reviewScore": 0 })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                "case-join-determination",
                PendingEvent {
                    event: "completeReview".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: "2026-04-19T00:00:00Z".to_string(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once("case-join-determination").unwrap();
        let join_transition = result
            .provenance
            .iter()
            .find(|record| {
                record.record_kind == ProvenanceKind::StateTransition
                    && record.from_state.as_deref() == Some("parallelReview")
                    && record.to_state.as_deref() == Some("decided")
            })
            .expect("join determination transition provenance");
        let snapshot = join_transition
            .case_file_snapshot
            .as_ref()
            .expect("join determination captures case state");

        assert_eq!(snapshot.value, serde_json::json!({ "reviewScore": 100 }));
        assert_eq!(snapshot.jcs_canonical, r#"{"reviewScore":100}"#);
    }

    /// Finding 3 regression: each determination-tagged transition in a
    /// single drain MUST capture its own pre-transition case-state snapshot.
    /// An earlier design draft hoisted the snapshot to a single pre-drain
    /// capture reused for every record — this test fails that shape and
    /// forces the per-transition capture wired through `Evaluator`.
    #[test]
    fn each_determination_transition_captures_its_own_snapshot() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:multi-determination-snapshot",
            "version": "1.0.0",
            "actors": [{ "id": "reviewer", "type": "human" }],
            "lifecycle": {
                "initialState": "firstReview",
                "states": {
                    "firstReview": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "decideFirst",
                            "target": "secondReview",
                            "tags": ["determination"],
                            "actions": [{
                                "action": "setData",
                                "path": "caseFile.firstOutcome",
                                "value": "approved"
                            }]
                        }]
                    },
                    "secondReview": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "decideSecond",
                            "target": "decided",
                            "tags": ["determination"]
                        }]
                    },
                    "decided": { "type": "final" }
                }
            }
        }))
        .unwrap();
        let mut runtime = runtime_with_kernel(kernel);

        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-multi-determination".to_string(),
                definition_url: "urn:test:multi-determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "firstOutcome": "pending" })),
            })
            .unwrap();
        for event in ["decideFirst", "decideSecond"] {
            runtime
                .enqueue_event(
                    "case-multi-determination",
                    PendingEvent {
                        event: event.to_string(),
                        actor_id: Some("reviewer".to_string()),
                        data: None,
                        timestamp: "2026-04-19T00:00:00Z".to_string(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
        }

        let mut snapshots = Vec::new();
        loop {
            let result = runtime.drain_once("case-multi-determination").unwrap();
            for record in &result.provenance {
                if record.record_kind == ProvenanceKind::StateTransition {
                    if let Some(snapshot) = record.case_file_snapshot.as_ref() {
                        snapshots.push((
                            record.from_state.clone().unwrap_or_default(),
                            snapshot.jcs_canonical.clone(),
                        ));
                    }
                }
            }
            if result.provenance.is_empty() {
                break;
            }
        }

        assert_eq!(
            snapshots.len(),
            2,
            "two determination transitions must each emit a snapshot"
        );
        assert_eq!(
            snapshots[0],
            (
                "firstReview".to_string(),
                r#"{"firstOutcome":"pending"}"#.to_string()
            ),
            "first determination snapshot is pre-mutation"
        );
        assert_eq!(
            snapshots[1],
            (
                "secondReview".to_string(),
                r#"{"firstOutcome":"approved"}"#.to_string()
            ),
            "second determination snapshot reflects the mutation from the first transition"
        );
    }

    #[test]
    fn inputs_outputs_set_for_case_state_mutation() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::case_state_mutation(
            "/amount",
            &serde_json::json!(42),
            None,
            "UnderReview",
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].inputs, vec!["/amount".to_string()]);
        assert_eq!(records[0].outputs, vec!["42".to_string()]);
    }

    /// Finding 2 regression: JSON string newValue must stringify as the bare
    /// string, not the JSON-quoted form. Previously `new_value.to_string()`
    /// emitted `"\"approved\""` for `Value::String("approved")`, inconsistent
    /// with the unquoted `"42"` emitted for `Value::Number(42)`.
    #[test]
    fn outputs_unquoted_for_case_state_mutation_string_value() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::case_state_mutation(
            "/status",
            &serde_json::json!("approved"),
            None,
            "UnderReview",
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].inputs, vec!["/status".to_string()]);
        assert_eq!(
            records[0].outputs,
            vec!["approved".to_string()],
            "JSON string newValue must appear unquoted in outputs"
        );
    }

    /// `stringify_scalar` handles every `serde_json::Value` variant. The
    /// string-unquoted case is covered above (finding-2 regression); this
    /// test exercises the remaining branches so a future refactor of the
    /// `other => other.to_string()` fall-through cannot silently change
    /// behavior for bools, null, objects, or arrays. Numbers are covered
    /// indirectly by `inputs_outputs_set_for_case_state_mutation` above.
    #[test]
    fn outputs_stringification_handles_all_value_types() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));

        // Bool → unquoted `"true"` / `"false"`.
        let mut bool_records = vec![ProvenanceRecord::case_state_mutation(
            "/approved",
            &serde_json::json!(true),
            None,
            "UnderReview",
        )];
        populate_provenance_record_fields(&mut bool_records, &kernel, "1.0.0");
        assert_eq!(bool_records[0].outputs, vec!["true".to_string()]);

        // Null → `"null"`.
        let mut null_records = vec![ProvenanceRecord::case_state_mutation(
            "/cleared",
            &serde_json::Value::Null,
            None,
            "UnderReview",
        )];
        populate_provenance_record_fields(&mut null_records, &kernel, "1.0.0");
        assert_eq!(null_records[0].outputs, vec!["null".to_string()]);

        // Object → valid JSON serialization that round-trips to the original.
        // We don't assert exact bytes — `serde_json` key order is deterministic
        // today but tying the test to that is brittle, and the contract is
        // "some valid JSON representation", not "these exact bytes".
        let object_value = serde_json::json!({ "k": 1, "nested": { "x": true } });
        let mut object_records = vec![ProvenanceRecord::case_state_mutation(
            "/payload",
            &object_value,
            None,
            "UnderReview",
        )];
        populate_provenance_record_fields(&mut object_records, &kernel, "1.0.0");
        let object_output = &object_records[0].outputs[0];
        let round_trip: serde_json::Value =
            serde_json::from_str(object_output).expect("object output must be valid JSON");
        assert_eq!(round_trip, object_value, "object output must round-trip");

        // Array → same round-trip contract.
        let array_value = serde_json::json!([1, "two", false, null]);
        let mut array_records = vec![ProvenanceRecord::case_state_mutation(
            "/history",
            &array_value,
            None,
            "UnderReview",
        )];
        populate_provenance_record_fields(&mut array_records, &kernel, "1.0.0");
        let array_output = &array_records[0].outputs[0];
        let round_trip: serde_json::Value =
            serde_json::from_str(array_output).expect("array output must be valid JSON");
        assert_eq!(round_trip, array_value, "array output must round-trip");
    }

    #[test]
    fn digests_computed_and_non_empty_when_inputs_present() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut records = vec![ProvenanceRecord::state_transition(
            "Draft",
            "Submitted",
            "submit",
            None,
        )];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        let input_digest = records[0]
            .input_digest
            .as_ref()
            .expect("input_digest populated when inputs are present");
        let output_digest = records[0]
            .output_digest
            .as_ref()
            .expect("output_digest populated when outputs are present");
        assert_eq!(input_digest.len(), 64, "SHA-256 hex is 64 chars");
        assert_eq!(output_digest.len(), 64, "SHA-256 hex is 64 chars");
        assert!(input_digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(output_digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn digests_absent_when_inputs_empty() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        // A timer_fired record has no inputs/outputs — digests must stay None.
        let mut records = vec![ProvenanceRecord::timer_fired("timer-1", "deadline")];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert!(records[0].inputs.is_empty());
        assert!(records[0].outputs.is_empty());
        assert!(records[0].input_digest.is_none());
        assert!(records[0].output_digest.is_none());
    }

    #[test]
    fn populate_is_idempotent_preserves_preset_values() {
        // Push-stamped discipline: if a field is already set, do not overwrite.
        let kernel = kernel_with_actors(
            "1.0.0",
            serde_json::json!([{ "id": "reviewer", "type": "human" }]),
        );
        let mut record =
            ProvenanceRecord::state_transition("Draft", "Submitted", "submit", Some("reviewer"));
        record.audit_layer = Some("reasoning".to_string());
        record.actor_type = Some("agent".to_string());
        record.lifecycle_state = Some("Preset".to_string());
        record.definition_version = Some("99.99.99".to_string());
        let mut records = vec![record];

        populate_provenance_record_fields(&mut records, &kernel, "1.0.0");

        assert_eq!(records[0].audit_layer.as_deref(), Some("reasoning"));
        assert_eq!(records[0].actor_type.as_deref(), Some("agent"));
        assert_eq!(records[0].lifecycle_state.as_deref(), Some("Preset"));
        assert_eq!(records[0].definition_version.as_deref(), Some("99.99.99"));
    }

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

    #[test]
    fn durable_runtime_trait_supports_instance_round_trip() {
        fn exercise_all_trait_methods(runtime: &mut impl DurableRuntime) {
            let created = runtime
                .create_instance(CreateInstanceRequest {
                    instance_id: "case-trait".to_string(),
                    definition_url: "urn:test:durable-trait".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: Some(serde_json::json!({ "approved": false })),
                })
                .expect("trait create_instance");
            let loaded = runtime
                .load_instance("case-trait")
                .expect("trait load_instance");
            assert_eq!(created.instance_id, loaded.instance_id);
            assert_eq!(created.definition_url, loaded.definition_url);
            assert_eq!(created.definition_version, loaded.definition_version);

            runtime
                .enqueue_event(
                    "case-trait",
                    PendingEvent {
                        event: "start".to_string(),
                        actor_id: Some("reviewer".to_string()),
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .expect("trait enqueue_event");

            let idle = runtime
                .drain_until_idle("case-trait")
                .expect("trait drain_until_idle");
            assert!(
                idle.iter().any(|step| step.processed_event.is_some()),
                "expected at least one drain step that processed an event: {idle:?}"
            );
            let task_id = idle
                .iter()
                .flat_map(|step| step.created_task_ids.iter())
                .next()
                .cloned()
                .expect("task id from drain");

            let once = runtime
                .drain_once("case-trait")
                .expect("trait drain_once on idle queue");
            assert!(
                once.processed_event.is_none(),
                "queue should be idle after drain_until_idle"
            );

            let window = runtime
                .load_provenance_window("case-trait", 0, 50)
                .expect("trait load_provenance_window");
            assert!(!window.is_empty());

            runtime
                .persist_task_draft(
                    &task_id,
                    serde_json::json!({ "status": "in-progress" }),
                    "reviewer",
                    None,
                )
                .expect("trait persist_task_draft");

            runtime
                .dismiss_task(&task_id, "trait exercise cleanup")
                .expect("trait dismiss_task");

            let submission = runtime
                .submit_task_response(
                    &task_id,
                    serde_json::json!({
                        "status": "completed",
                        "definitionUrl": "urn:formspec:review",
                        "definitionVersion": "1.0.0",
                        "data": { "approved": true }
                    }),
                    "reviewer",
                    None,
                )
                .expect("trait submit_task_response");
            assert!(
                matches!(submission, TaskSubmissionResult::Completed { .. }),
                "expected completed submission, got {submission:?}"
            );
        }

        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:durable-trait",
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
        .expect("kernel json");
        let mut runtime = runtime_with_kernel(kernel);
        exercise_all_trait_methods(&mut runtime);
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

    /// Regression: the persist-task-draft append site must route its record
    /// through `populate_provenance_record_fields` so it carries the same
    /// SP §5.3 shape (audit_layer, definition_version, …) as every other
    /// append path. Prior to the Finding 1 fix this site set only
    /// `timestamp` and pushed the record directly, skipping the populator.
    #[test]
    fn persist_task_draft_populates_new_fields() {
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
                instance_id: "case-draft-pop".to_string(),
                definition_url: "urn:test:draft-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime.store.load_record("case-draft-pop").unwrap();
        let task = manual_formspec_task("case-draft-pop", 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.instance.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        runtime
            .persist_task_draft(
                &task_id,
                serde_json::json!({
                    "status": "stopped",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": false }
                }),
                "reviewer",
                None,
            )
            .unwrap();

        let record = runtime.store.load_record("case-draft-pop").unwrap();
        let draft_entry = record
            .provenance_log
            .iter()
            .find(|entry| entry.record_kind == ProvenanceKind::TaskDraftPersisted)
            .expect("TaskDraftPersisted record appended");
        assert_eq!(
            draft_entry.audit_layer.as_deref(),
            Some("facts"),
            "populator must stamp audit_layer on persist_task_draft path"
        );
        assert_eq!(
            draft_entry.definition_version.as_deref(),
            Some("1.0.0"),
            "populator must stamp definition_version on persist_task_draft path"
        );
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

    // NB.4: all seven binding kinds are now dispatched to handlers.
    // The old "unsupported tool binding kind" test is superseded — `tool` bindings
    // now succeed. This test verifies the tool handler executes and emits ToolInvoked
    // provenance (replacing the prior UnsupportedBindingKind assertion).
    #[test]
    fn drain_once_dispatches_tool_integration_profile_binding() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:tool-integration-binding",
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
                    "url": "urn:test:tool-integration-binding",
                    "compatibleVersions": ">=1.0.0 <2.0.0"
                },
                "bindings": {
                    "legacyTool": {
                        "type": "tool",
                        "toolId": "legacy-analysis-tool",
                        "inputMapping": {
                            "payload": "caseFile.application.id"
                        }
                    }
                }
            }))
            .unwrap();

        let service = RecordingService::with_response(serde_json::json!({
            "result": "ok"
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
                instance_id: "case-tool-integration-binding".to_string(),
                definition_url: "urn:test:tool-integration-binding".to_string(),
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
                "case-tool-integration-binding",
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        // Tool bindings now succeed — expect Ok, not Err.
        let result = runtime
            .drain_once("case-tool-integration-binding")
            .expect("tool binding dispatch must succeed (NB.4)");

        // The service must have been called exactly once.
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // ToolInvoked provenance must be present.
        use wos_core::provenance::ProvenanceKind;
        let tool_invoked = result
            .provenance
            .iter()
            .find(|p| p.record_kind == ProvenanceKind::ToolInvoked)
            .expect("ToolInvoked provenance must be emitted by the tool handler");
        assert_eq!(
            tool_invoked
                .data
                .as_ref()
                .and_then(|d| d.get("toolId"))
                .and_then(|v| v.as_str()),
            Some("legacy-analysis-tool")
        );
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

    // ── §5.3 teaching-signal: guard evaluations surface through DrainOnceResult ──

    #[test]
    fn drain_once_exposes_guard_evaluations() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:drain-guard-evals",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "submitted",
                "states": {
                    "submitted": {
                        "type": "atomic",
                        "transitions": [
                            {
                                "event": "approve",
                                "target": "approved",
                                "guard": "caseFile.amount < 100"
                            },
                            {
                                "event": "approve",
                                "target": "escalated",
                                "guard": "caseFile.amount >= 100"
                            }
                        ]
                    },
                    "approved": { "type": "final" },
                    "escalated": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-guards".to_string(),
                definition_url: "urn:test:drain-guard-evals".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "amount": 250 })),
            })
            .unwrap();

        runtime
            .enqueue_event(
                "case-guards",
                PendingEvent {
                    event: "approve".to_string(),
                    actor_id: Some("approver".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once("case-guards").unwrap();

        // DrainOnceResult must carry both guard evaluations for this event:
        // the first one blocked (amount < 100 = false), the second fired
        // (amount >= 100 = true). Without both, §5.4's teaching signal has
        // no way to show which guard the LLM's workflow expected to fire.
        assert_eq!(
            result.guard_evaluations.len(),
            2,
            "both guards evaluated on the `approve` event"
        );
        assert_eq!(result.guard_evaluations[0].target_state, "approved");
        assert!(!result.guard_evaluations[0].result);
        assert_eq!(result.guard_evaluations[1].target_state, "escalated");
        assert!(result.guard_evaluations[1].result);
        assert_eq!(
            result.guard_evaluations[0].inputs,
            serde_json::json!({ "caseFile": { "amount": 250 } })
        );
    }

    #[test]
    fn drain_once_guard_evaluations_scope_to_one_event() {
        // Each drain_once must return only the guard evaluations observed
        // during THAT event. A later drain on a second event must not leak
        // the first event's records.
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosKernel": "1.0",
            "url": "urn:test:guard-scope",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "s1",
                "states": {
                    "s1": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "go",
                            "target": "s2",
                            "guard": "caseFile.ok = true"
                        }]
                    },
                    "s2": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "next",
                            "target": "s3",
                            "guard": "caseFile.ok = true"
                        }]
                    },
                    "s3": { "type": "atomic" }
                }
            }
        }))
        .unwrap();

        let mut runtime = runtime_with_kernel(kernel);
        runtime
            .create_instance(CreateInstanceRequest {
                instance_id: "case-scope".to_string(),
                definition_url: "urn:test:guard-scope".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "ok": true })),
            })
            .unwrap();
        for name in ["go", "next"] {
            runtime
                .enqueue_event(
                    "case-scope",
                    PendingEvent {
                        event: name.to_string(),
                        actor_id: None,
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
        }

        let first = runtime.drain_once("case-scope").unwrap();
        let second = runtime.drain_once("case-scope").unwrap();

        assert_eq!(first.guard_evaluations.len(), 1);
        assert_eq!(first.guard_evaluations[0].event, "go");
        assert_eq!(second.guard_evaluations.len(), 1);
        assert_eq!(
            second.guard_evaluations[0].event, "next",
            "second drain only sees its own guard"
        );
    }
}
