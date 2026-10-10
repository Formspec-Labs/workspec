// Rust guideline compliant 2026-02-21

//! Runtime command surface for WOS processors.

mod actions;
pub(crate) mod actor_bindings;
mod custody_trellis;
mod drain;
mod durable_impl;
mod intake;
mod process;
mod provenance;
mod signature;
mod staging;
mod staging_store;
mod support;
mod tasks;
mod timers;

#[doc(inline)]
pub use staging::{
    CaseProcessMembership, DeferredOutboundEvent, NativeCommandKey, NativeIntakeBindingOperation,
    NativeIntakeChange, NativePreparationError, NativePreparationFailure, NativePreparationPolicy,
    NativePresentationAcknowledgement, NativePresentationArtifactIdentity,
    NativePresentationInvocationOutcome, NativeProcessChange, NativeReadOnlyDependency,
    NativeRecoveryReference, NativeRuntimeCommand, NativeRuntimeOutcome, NativeRuntimeSnapshot,
    NativeTaskBindingOperation, PendingNativePresentationIntent, PreparedNativeCommand,
};

#[doc(inline)]
pub use tasks::task_process_id;

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error as StdError;

use serde::{Deserialize, Serialize};
#[cfg(test)]
use wos_core::ProvenanceKind;
use wos_core::ProvenanceRecord;
use wos_core::business_calendar::BusinessCalendarDocument;
#[cfg(test)]
use wos_core::eval::Evaluator;
use wos_core::eval::{GuardEvaluation, ObservedTransition};
use wos_core::instance::{FormspecTaskContext, PendingEvent, WorkflowProcess};
use wos_core::model::kernel::KernelDocument;
use wos_core::traits::{
    AccessControl, ContractValidator, DocumentResolver, ExternalService, TaskPresenter,
};

use crate::binding::{BindingError, BindingRegistry};
use crate::intake::{IntakeAcceptancePolicy, IntakeAcceptanceRegistry, NoopIntakeAcceptancePolicy};
use crate::integration::IntegrationProfileDocument;
use crate::store::{RuntimeStore, StoreError};
use wos_events::custody::CustodyAppendError;

#[doc(inline)]
pub use custody_trellis::{TrellisCustodyAppendOutcome, TrellisCustodyAppender};
pub use provenance::{
    CustodyReceiptStampError, populate_provenance_record_fields, stamp_custody_receipt,
    stamp_provenance, stamp_signature_decision_identity,
};
use provenance::{compensation_provenance, contract_validation_record};
pub use signature::{
    AdmissionOutcome, CompletionRequirementKind, HttpPostureResolver, PostureDeclaration,
    PostureResolver, ResolvedPostureDeclaration, SIGNATURE_PROFILE_KEY_EXTENSION,
    SIGNATURE_PROFILE_REF_EXTENSION, SIGNATURE_STEP_ID_EXTENSION, SignatureProfileDocument,
    StaticPostureResolver,
};
use support::{
    format_timestamp, impact_level_label, make_task_id, merge_case_state,
    normalize_semver_range_expression, parse_timestamp,
};

const COMPLETION_EVENT_EXTENSION_KEY: &str = "x-wos-runtime-completion-event";
const FAILURE_EVENT_EXTENSION_KEY: &str = "x-wos-runtime-failure-event";

/// Checks exact runtime outcome metadata against a selected task-creation action.
///
/// This consistency check grants no task read, action, or disclosure authority.
/// The host must separately qualify one unambiguous action from the actual
/// governing workflow and validate task association, assignment, scope, grants,
/// exclusions, and current policy. Unsupported selection must refuse.
///
/// Only ordinary task creation with no authored action extensions is supported.
/// The actual persisted map must contain exactly the declared completion and
/// failure event strings, without unknown keys or blank values. No metadata is
/// changed, removed, or inferred from current access authority.
pub fn task_outcome_metadata_matches(
    action: &wos_core::model::kernel::Action,
    extensions: &std::collections::HashMap<String, serde_json::Value>,
) -> bool {
    if action.action != wos_core::model::kernel::ActionKind::CreateTask
        || !action.extensions.is_empty()
        || extensions
            .keys()
            .any(|key| key != COMPLETION_EVENT_EXTENSION_KEY && key != FAILURE_EVENT_EXTENSION_KEY)
    {
        return false;
    }
    [
        (
            COMPLETION_EVENT_EXTENSION_KEY,
            action.completion_event.as_deref(),
        ),
        (FAILURE_EVENT_EXTENSION_KEY, action.failure_event.as_deref()),
    ]
    .into_iter()
    .all(|(key, expected)| match (expected, extensions.get(key)) {
        (None, None) => true,
        (Some(expected), Some(serde_json::Value::String(actual))) => {
            !expected.trim().is_empty() && actual == expected
        }
        _ => false,
    })
}

/// Request for instance creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessRequest {
    /// Stable WOS workflow-process identifier.
    pub process_id: String,
    /// Tenant this instance belongs to (ADR 0068 D-1).
    ///
    /// When `None`, the runtime extracts the tenant from the TypeID prefix
    /// (if `process_id` is a valid TypeID) or falls back to the
    /// deployment-default tenant.
    pub tenant: Option<String>,
    /// Governing kernel URL.
    pub definition_url: String,
    /// Governing kernel version.
    pub definition_version: String,
    /// Initial case-state overrides.
    pub initial_case_state: Option<serde_json::Value>,
}

/// Case-state migration map (Kernel S11.2 step 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationMap {
    #[serde(default)]
    pub field_renames: HashMap<String, String>,
    #[serde(default)]
    pub field_removals: Vec<String>,
    #[serde(default)]
    pub field_defaults: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub field_coercions: HashMap<String, String>,
}

/// Successful [`WosRuntime::migrate`] result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationOutcome {
    /// Instance that was migrated.
    pub process_id: String,
    /// Prior `definitionVersion`.
    pub previous_definition_version: String,
    /// New `definitionVersion`.
    pub new_definition_version: String,
    /// Map applied on this call (echo for idempotency and audit correlation).
    pub migration_map: MigrationMap,
}

/// Single-step drain result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistDraftResult {
    /// Stable artifact identifier for the draft.
    pub artifact_id: String,
}

/// Immutable result of a committed, resumable UI dismissal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DismissTaskResult {
    /// Identifier of the actual persisted dismissal provenance record.
    pub provenance_id: String,
    /// Original persisted RFC 3339 occurrence time, unchanged on replay.
    pub occurred_at: String,
}

/// Task submission outcome.
///
/// Serde uses externally tagged camelCase variants and fields. This native result
/// representation is separate from the maintained auxiliary replay codec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
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
    /// Submission reported a native failure; some failures keep the task active.
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
    pub instance: WorkflowProcess,
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
#[derive(Debug, Clone, thiserror::Error)]
pub enum RuntimeError {
    /// A fresh command targets a task whose explicit lifetime has expired.
    #[error("task has expired: {0}")]
    TaskExpired(String),

    /// Runtime store failed.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// Document resolution failed.
    #[error("document resolution failed: {0}")]
    Resolver(String),

    /// No kernel bundle is registered for the workflow URL (caller configuration).
    #[error("kernel workflow bundle not found for url `{url}`")]
    KernelWorkflowNotFound { url: String },

    /// Requested kernel `definitionVersion` does not match the registered bundle.
    #[error(
        "kernel `{url}` is registered with definition version `{loaded_version}`; `{requested_version}` was requested"
    )]
    KernelDefinitionVersionMismatch {
        url: String,
        loaded_version: String,
        requested_version: String,
    },

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

    /// Custody append input generation failed.
    #[error(transparent)]
    CustodyAppend(#[from] CustodyAppendError),

    /// Native preparation refused an unsupported effect or inconsistent input.
    #[error(transparent)]
    NativePreparation(#[from] NativePreparationError),

    /// Required workflow metadata is absent.
    #[error("missing workflow metadata: {0}")]
    MissingMetadata(String),

    /// Explicit human assignment resolution failed.
    #[error("actor binding failed: {0}")]
    ActorBinding(String),

    /// The referenced task was not found.
    #[error("active task not found: {0}")]
    TaskNotFound(String),

    /// The referenced contract was not found.
    #[error("contract not found: {0}")]
    ContractNotFound(String),

    /// The binding is unsupported by the runtime (free-form string error).
    #[error("binding unsupported: {0}")]
    UnsupportedBinding(String),

    /// Intake handoff id replay conflicted with a different document payload.
    #[error("intake handoff id conflict: {0}")]
    IntakeConflict(String),

    /// A dismissal key or natural task/reason identity conflicts with retained evidence.
    #[error("dismissal replay conflict: {0}")]
    DismissalConflict(String),

    /// A case ledger reference cannot identify one workflow process.
    #[error("case ledger reference conflict: {0}")]
    CaseLedgerConflict(String),

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

    /// Signature Profile runtime processing failed.
    #[error("signature profile runtime failed: {0}")]
    Signature(String),

    /// Timestamp conversion failed.
    #[error("timestamp conversion failed: {0}")]
    Clock(String),

    /// Instance migration aborted (Kernel S11.2 — no partial persistence).
    #[error("migration rejected: {0}")]
    MigrationRejected(String),

    /// The referenced provenance record was not found.
    #[error("provenance record not found: {0}")]
    ProvenanceRecordNotFound(String),

    /// Custody receipt hash disagrees with an already-stamped value on the same record.
    #[error(
        "custody receipt conflict: existing canonical_event_hash {existing} differs from attempted {attempted}"
    )]
    CustodyReceiptConflict { existing: String, attempted: String },

    /// Tenant identifier does not satisfy the ADR 0068 D-1.1 grammar.
    #[error("invalid tenant identifier: {0}")]
    TenantInvalid(String),

    /// Explicit tenant disagrees with the TypeID tenant prefix.
    #[error("tenant mismatch: explicit {explicit} differs from TypeID prefix {type_id_prefix}")]
    TenantMismatch {
        explicit: String,
        type_id_prefix: String,
    },

    /// Requested capability is not enabled in this server build.
    #[error("{0}")]
    FeatureDisabled(&'static str),
}

impl From<provenance::CustodyReceiptStampError> for RuntimeError {
    fn from(value: provenance::CustodyReceiptStampError) -> Self {
        match value {
            provenance::CustodyReceiptStampError::Conflict {
                existing,
                attempted,
            } => Self::CustodyReceiptConflict {
                existing,
                attempted,
            },
        }
    }
}

trait ResolveDocumentsDyn {
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, RuntimeError>;
}

fn map_document_resolver_error<E: StdError + Any + Send + Sync>(error: E) -> RuntimeError {
    if let Some(rt) = (&error as &dyn Any).downcast_ref::<RuntimeError>() {
        return rt.clone();
    }
    RuntimeError::Resolver(error.to_string())
}

impl<T> ResolveDocumentsDyn for T
where
    T: DocumentResolver,
    T::Error: StdError + Any + Send + Sync,
{
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, RuntimeError> {
        DocumentResolver::resolve_kernel(self, url, version).map_err(map_document_resolver_error)
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
    preparation: Option<staging::PreparationState>,
    #[cfg(test)]
    preparation_fault_at_write: Option<usize>,
    store: Box<dyn RuntimeStore + Send + Sync>,
    resolver: Box<dyn ResolveDocumentsDyn + Send + Sync>,
    presenter: Box<dyn PresentTasksDyn + Send + Sync>,
    access_control: Box<dyn AccessControl + Send + Sync>,
    service: Box<dyn InvokeServicesDyn + Send + Sync>,
    validator: Box<dyn ValidateContractsDyn + Send + Sync>,
    clock: Box<dyn Clock + Send + Sync>,
    companion_policy: Box<dyn CompanionPolicy + Send + Sync>,
    integration_profile: Option<IntegrationProfileDocument>,
    deferred_callback_max_count: usize,
    deferred_callback_max_bytes: usize,
    /// Attached business calendar for SLA deadline computation (BC.1).
    business_calendar: Option<BusinessCalendarDocument>,
    bindings: BindingRegistry,
    intake_acceptors: IntakeAcceptanceRegistry,
    intake_policy: Box<dyn IntakeAcceptancePolicy + Send + Sync>,
    signature_profiles: Vec<signature::SignatureProfileRegistration>,
    /// Agent invoker registry per ADR 0064. Maps each [`InvokerKind`] to a
    /// concrete adapter; the runtime looks up adapters by an agent
    /// declaration's `invoker` discriminator. Empty by default — runtimes
    /// without agent participation never need to populate it.
    agent_invokers: wos_core::AgentInvokerRegistry,
    /// Host-owned Posture Declaration resolver.
    posture_resolver: Box<dyn signature::PostureResolver + Send + Sync>,
    /// Cached Posture Declarations keyed by resolved content digest.
    ///
    /// **Production note:** `RefCell` makes `WosRuntime` `!Send + !Sync`, which is
    /// acceptable for the in-memory adapter (single-threaded test oracle) but MUST be
    /// replaced with an async-aware cache (e.g., `tokio::sync::RwLock`) in the
    /// production Restate adapter where the runtime crosses async boundaries.
    posture_declarations: RefCell<HashMap<String, signature::PostureDeclaration>>,
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
        S: RuntimeStore + Send + Sync + 'static,
        R: DocumentResolver + Send + Sync + 'static,
        R::Error: StdError + Send + Sync + 'static,
        P: TaskPresenter + Send + Sync + 'static,
        P::Error: StdError + Send + Sync + 'static,
        A: AccessControl + Send + Sync + 'static,
        E: ExternalService + Send + Sync + 'static,
        E::Error: StdError + Send + Sync + 'static,
        V: ContractValidator + Send + Sync + 'static,
        V::Error: StdError + Send + Sync + 'static,
        C: Clock + Send + Sync + 'static,
    {
        Self {
            preparation: None,
            #[cfg(test)]
            preparation_fault_at_write: None,
            store: Box::new(store),
            resolver: Box::new(resolver),
            presenter: Box::new(presenter),
            access_control: Box::new(access_control),
            service: Box::new(service),
            validator: Box::new(validator),
            clock: Box::new(clock),
            companion_policy: Box::new(NoopCompanionPolicy),
            integration_profile: None,
            deferred_callback_max_count: 1000,
            deferred_callback_max_bytes: 1024 * 1024,
            business_calendar: None,
            bindings,
            intake_acceptors: IntakeAcceptanceRegistry::new(),
            intake_policy: Box::new(NoopIntakeAcceptancePolicy),
            signature_profiles: Vec::new(),
            agent_invokers: wos_core::AgentInvokerRegistry::new(),
            posture_resolver: Box::new(signature::HttpPostureResolver),
            posture_declarations: RefCell::new(HashMap::new()),
        }
    }

    /// Replace the default no-op companion-policy hook.
    pub fn with_companion_policy<P>(mut self, companion_policy: P) -> Self
    where
        P: CompanionPolicy + Send + Sync + 'static,
    {
        self.companion_policy = Box::new(companion_policy);
        self
    }

    /// Attach an Integration Profile document for `invokeService` bindings.
    pub fn with_integration_profile(mut self, profile: IntegrationProfileDocument) -> Self {
        self.integration_profile = Some(profile);
        self
    }

    /// Tightens deferred request count and aggregate serialized-byte limits.
    ///
    /// Defaults are 1000 requests and 1 MiB per process. Values above these
    /// bounds do not expand capacity; zero prohibits new deferred dispatch.
    pub fn with_deferred_callback_limits(mut self, count: usize, bytes: usize) -> Self {
        self.deferred_callback_max_count = count.min(1000);
        self.deferred_callback_max_bytes = bytes.min(1024 * 1024);
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

    /// Attach host-side intake-acceptance adapters.
    pub fn with_intake_acceptors(mut self, intake_acceptors: IntakeAcceptanceRegistry) -> Self {
        self.intake_acceptors = intake_acceptors;
        self
    }

    /// Replace the default no-op intake-acceptance policy.
    pub fn with_intake_policy<P>(mut self, intake_policy: P) -> Self
    where
        P: IntakeAcceptancePolicy + Send + Sync + 'static,
    {
        self.intake_policy = Box::new(intake_policy);
        self
    }

    /// Replace the agent-invoker registry wholesale (ADR 0064).
    ///
    /// Use this when wiring multiple adapter crates at deployment time:
    ///
    /// ```ignore
    /// let registry = AgentInvokerRegistry::new()
    ///     .with(InvokerKind::Anthropic, Box::new(AnthropicInvoker::new(...)))
    ///     .with(InvokerKind::Mcp, Box::new(McpInvoker::new(...)));
    /// let runtime = WosRuntime::new(/* … */).with_agent_invokers(registry);
    /// ```
    pub fn with_agent_invokers(mut self, registry: wos_core::AgentInvokerRegistry) -> Self {
        self.agent_invokers = registry;
        self
    }

    /// Register a single agent invoker for `kind` (ADR 0064). Convenience
    /// builder for deployments that wire one substrate (e.g., conformance
    /// stubs registering only [`wos_core::InvokerKind::Stub`]).
    pub fn with_agent_invoker(
        mut self,
        kind: wos_core::InvokerKind,
        invoker: Box<dyn wos_core::AgentInvoker + Send + Sync>,
    ) -> Self {
        self.agent_invokers.register(kind, invoker);
        self
    }

    /// Borrow the agent-invoker registry. Returns an empty registry when no
    /// adapters have been wired (i.e., the runtime was constructed without
    /// any `with_agent_invoker*` call).
    #[must_use]
    pub fn agent_invokers(&self) -> &wos_core::AgentInvokerRegistry {
        &self.agent_invokers
    }
}

#[cfg(test)]
mod tests {
    mod native_stage_tests {
        include!("runtime/staging_tests.rs");
    }
    use super::*;
    use crate::store::ReplayValue;

    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use crate::DurableRuntime;
    use crate::binding::{
        CaseMutationBundle, PreparedTask, SignatureAdmissionFailure,
        SignatureAdmissionFailureReason, SubmissionValidation,
    };
    use crate::intake::{
        AutoCreatePublicIntakePolicy, IntakeAcceptanceAdapter, IntakeAcceptanceDecision,
        IntakeAcceptanceOutcome, IntakeAcceptancePolicy, IntakeAcceptanceRegistry,
        IntakeAcceptanceRequest, IntakeCaseDefinition, IntakeCaseDisposition, IntakeCaseIntent,
        IntakeInterpretation, IntakePolicyContext, ManualReviewIntakePolicy,
        NoopIntakeAcceptancePolicy, PublicIntakeDisabledPolicy,
    };
    use crate::store::{InMemoryStore, IntakeRecord, RuntimeRecord, RuntimeStore, StoreError};
    use wos_core::instance::{ActiveTask, ActiveTaskStatus, ValidationOutcome};
    use wos_core::traits::{DocumentResolver, ExternalService, TaskPresenter};

    fn test_process_id(label: &str) -> &'static str {
        Box::leak(format!("{label}_process_01hw7rm71vfay8vvw14d2pf2db").into_boxed_str())
    }

    fn test_case_ledger_id(label: &str) -> &'static str {
        Box::leak(format!("{label}_case_01hw7rm71vfay8vvw14d2pf2db").into_boxed_str())
    }

    fn test_case_ledger_urn(label: &str) -> &'static str {
        Box::leak(format!("urn:wos:{}", test_case_ledger_id(label)).into_boxed_str())
    }

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
            "$wosWorkflow": "1.0",
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
                id: ProvenanceRecord::mint_id(),
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
                canonical_event_hash: None,
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-determination").to_string(),
                tenant: None,
                definition_url: "urn:test:determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(case_state.clone()),
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-determination"),
                PendingEvent {
                    event: "decide".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: "2026-04-19T00:00:00Z".to_string(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime
            .drain_once(test_process_id("case-determination"))
            .unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-join-determination").to_string(),
                tenant: None,
                definition_url: "urn:test:join-determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "reviewScore": 0 })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-join-determination"),
                PendingEvent {
                    event: "completeReview".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: "2026-04-19T00:00:00Z".to_string(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime
            .drain_once(test_process_id("case-join-determination"))
            .unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-multi-determination").to_string(),
                tenant: None,
                definition_url: "urn:test:multi-determination-snapshot".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "firstOutcome": "pending" })),
            })
            .unwrap();
        for event in ["decideFirst", "decideSecond"] {
            runtime
                .enqueue_event(
                    test_process_id("case-multi-determination"),
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
            let result = runtime
                .drain_once(test_process_id("case-multi-determination"))
                .unwrap();
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

        fn with_kernels(iter: impl IntoIterator<Item = KernelDocument>) -> Self {
            let mut kernels = HashMap::new();
            for kernel in iter {
                let url = kernel.url.clone().unwrap();
                let version = kernel.version.clone().unwrap();
                kernels.insert((url, version), kernel);
            }
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

    #[derive(Debug)]
    struct RotatingPostureResolver {
        bodies: Mutex<VecDeque<String>>,
    }

    impl RotatingPostureResolver {
        fn new(bodies: impl IntoIterator<Item = String>) -> Self {
            Self {
                bodies: Mutex::new(bodies.into_iter().collect()),
            }
        }
    }

    impl signature::PostureResolver for RotatingPostureResolver {
        fn resolve_posture_declaration(
            &self,
            posture_uri: &str,
        ) -> Result<signature::ResolvedPostureDeclaration, RuntimeError> {
            let mut bodies = self.bodies.lock().expect("posture resolver mutex poisoned");
            let body = bodies.pop_front().ok_or_else(|| {
                RuntimeError::Signature(format!(
                    "no posture declaration body queued for '{posture_uri}'"
                ))
            })?;
            Ok(signature::ResolvedPostureDeclaration::new(
                posture_uri,
                body,
            ))
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

        fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.0.lock().unwrap().load_record(process_id)
        }

        fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
            self.0.lock().unwrap().processes_for_case(case_ledger_id)
        }

        fn append_provenance_for_case(
            &mut self,
            case_ledger_id: &str,
            process_id: &str,
            record: ProvenanceRecord,
        ) -> Result<(), StoreError> {
            self.0
                .lock()
                .unwrap()
                .append_provenance_for_case(case_ledger_id, process_id, record)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.0.lock().unwrap().save_record(record)
        }

        fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.0.lock().unwrap().create_intake_record(record)
        }

        fn load_intake_record(
            &self,
            binding: &str,
            intake_id: &str,
        ) -> Result<IntakeRecord, StoreError> {
            self.0
                .lock()
                .unwrap()
                .load_intake_record(binding, intake_id)
        }

        fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.0.lock().unwrap().save_intake_record(record)
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
                .load_record(&context.process_id)
                .unwrap();
            let task = record
                .process
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

    fn test_signature_admission_failure(
        signature: &serde_json::Value,
    ) -> Option<SignatureAdmissionFailure> {
        let reason = signature
            .get("admissionFailureReason")
            .and_then(serde_json::Value::as_str)
            .map(test_signature_admission_failure_reason)?;
        Some(SignatureAdmissionFailure {
            reason,
            failure_context: signature
                .get("failureContext")
                .and_then(serde_json::Value::as_object)
                .cloned(),
        })
    }

    fn test_signature_admission_failure_reason(reason: &str) -> SignatureAdmissionFailureReason {
        match reason {
            "primitive_verification_failed" => {
                SignatureAdmissionFailureReason::PrimitiveVerificationFailed
            }
            "method_unsupported" => SignatureAdmissionFailureReason::MethodUnsupported,
            "method_unregistered" => SignatureAdmissionFailureReason::MethodUnregistered,
            "evidence_divergence" => SignatureAdmissionFailureReason::EvidenceDivergence,
            "posture_floor_unmet" => SignatureAdmissionFailureReason::PostureFloorUnmet,
            "registry_unrecognized_method" => {
                SignatureAdmissionFailureReason::RegistryUnrecognizedMethod
            }
            "adapter_unavailable" => SignatureAdmissionFailureReason::AdapterUnavailable,
            other => panic!("unsupported test admission failure reason: {other}"),
        }
    }

    fn test_signature_method(signature: &serde_json::Value) -> Option<String> {
        // Runtime-unit adapters intentionally bypass Formspec COSE parsing and
        // accept a synthetic JSON `methodUri`. Production and
        // conformance paths use `wos-formspec-binding`, which decodes
        // `signatureValue` and fails closed on unusable method_uri headers.
        signature
            .get("methodUri")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    }

    // TEST-ONLY ADAPTER — BYPASSES COSE DECODING.
    //
    // `TestAdapter` reads `methodUri` directly from a synthetic JSON field on
    // the response signature (via `test_signature_method`) instead of decoding
    // a real COSE_Sign1 envelope and reading the protected-header `method_uri`
    // (label `-65540`) per ADR 0109. This bypass is ONLY safe under
    // `#[cfg(test)]` because the surrounding test scope fully controls the
    // injected JSON. The production adapter is `wos-formspec-binding`, which
    // decodes COSE `signatureValue` and fails closed on missing or undecodable
    // `method_uri`. If this adapter is ever lifted out of `#[cfg(test)]`, the
    // ADR 0109 fail-closed admission contract collapses: attacker-supplied
    // JSON `methodUri` strings would be silently admitted as if signed.
    //
    // Shadow mechanism: `BindingRegistry::register` (binding.rs:303-305) uses
    // `HashMap::insert` and SILENTLY OVERWRITES any prior `"formspec"`
    // registration with no warning, log, panic, or test failure. The only
    // structural barrier between this test adapter and a production binary is
    // `#[cfg(test)]`. A future author migrating this adapter out of
    // `#[cfg(test)]` will collapse the ADR 0109 fail-closed contract with no
    // compile-time or runtime signal. (As of the P2.1 finding,
    // `BindingRegistry::register` now panics on duplicate registration, making
    // this shadow path visible at composition-root time.)
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

        fn signature_evidence(
            &self,
            _task: &ActiveTask,
            response: &serde_json::Value,
        ) -> Result<Option<Vec<crate::binding::SignatureEvidence>>, BindingError> {
            let Some(signature) = response.get("data").and_then(|data| data.get("signature"))
            else {
                return Ok(None);
            };
            let document_hash = signature
                .get("documentHash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let signed_at = signature
                .get("acceptedAt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let primitive_verification = match signature
                .get("primitiveVerification")
                .and_then(serde_json::Value::as_str)
            {
                Some("failed") => crate::binding::SignaturePrimitiveStatus::Failed {
                    reason: "test-primitive-failed".to_string(),
                },
                Some("verified") => crate::binding::SignaturePrimitiveStatus::Verified,
                _ => crate::binding::SignaturePrimitiveStatus::DeferredPendingHelper {
                    reason: "test-harness-no-signature-primitive".to_string(),
                },
            };
            Ok(Some(vec![crate::binding::SignatureEvidence {
                source_system: "formspec-test-harness".to_string(),
                source_signature_id: "sig013-harness".to_string(),
                signing_act_id: None,
                presentation_hash: None,
                source_response_ref: Some("urn:test:formspec-response:application".to_string()),
                document_id: "application".to_string(),
                signer_id: response
                    .get("data")
                    .and_then(|data| data.get("signerId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                signing_intent: "urn:wos:signing-intent:applicant-signature".to_string(),
                signature_method: test_signature_method(signature),
                signed_payload_digest:
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
                signed_payload_digest_algorithm: "sha-256".to_string(),
                signed_at,
                document_hash,
                document_hash_algorithm: "sha-256".to_string(),
                signature_provider: Some("formspec".to_string()),
                ceremony_id: response
                    .get("data")
                    .and_then(|data| data.get("ceremonyId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                identity_binding: response
                    .get("data")
                    .and_then(|data| data.get("identityBinding"))
                    .cloned(),
                signer_authority: None,
                primitive_verification,
                verification_receipt: signature
                    .get("verificationReceipt")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                admission_failure: test_signature_admission_failure(signature),
            }]))
        }
    }

    /// Same binding name as `TestAdapter`, but treats completed signature task
    /// payloads (affirmed ceremony) as binding-valid so `submit_task_response`
    /// reaches signature affirmation (SIG-013 policy floor). The stock
    /// `TestAdapter` only gates on `data.approved`, which would surface as
    /// generic `validationFailed` before policy checks run.
    ///
    /// TEST-ONLY ADAPTER — BYPASSES COSE DECODING. Like `TestAdapter`, this
    /// reads `methodUri` from a synthetic JSON field via
    /// `test_signature_method` instead of decoding the COSE protected-header
    /// `method_uri` (label `-65540`) per ADR 0109. Safe ONLY because
    /// `#[cfg(test)]` constrains the input. Production routing belongs to
    /// `wos-formspec-binding`, which decodes COSE `signatureValue` and fails
    /// closed on missing or undecodable `method_uri`. Lifting this adapter
    /// out of `#[cfg(test)]` would silently admit attacker-supplied JSON
    /// `methodUri` strings, defeating the ADR 0109 fail-closed admission
    /// contract.
    ///
    /// Shadow mechanism: `BindingRegistry::register` (binding.rs:303-305) uses
    /// `HashMap::insert` and SILENTLY OVERWRITES any prior `"formspec"`
    /// registration with no warning, log, panic, or test failure. The only
    /// structural barrier between this test adapter and a production binary is
    /// `#[cfg(test)]`. A future author migrating this adapter out of
    /// `#[cfg(test)]` will collapse the ADR 0109 fail-closed contract with no
    /// compile-time or runtime signal. (As of the P2.1 finding,
    /// `BindingRegistry::register` now panics on duplicate registration, making
    /// this shadow path visible at composition-root time.)
    #[derive(Debug, Default)]
    struct Sig013HarnessFormspecAdapter;

    impl crate::binding::ContractBindingAdapter for Sig013HarnessFormspecAdapter {
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
            let approved = response
                .get("data")
                .and_then(|data| data.get("approved"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let signature_completed = response
                .get("data")
                .and_then(|data| data.get("signature"))
                .and_then(|signature| signature.get("affirmed"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let definition_valid = approved || signature_completed;
            Ok(SubmissionValidation {
                validation_outcome: ValidationOutcome {
                    envelope_valid: true,
                    pin_match,
                    definition_valid,
                    errors: if definition_valid && pin_match {
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

        fn signature_evidence(
            &self,
            _task: &ActiveTask,
            response: &serde_json::Value,
        ) -> Result<Option<Vec<crate::binding::SignatureEvidence>>, BindingError> {
            let Some(signature) = response.get("data").and_then(|data| data.get("signature"))
            else {
                return Ok(None);
            };
            let document_hash = signature
                .get("documentHash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let signed_at = signature
                .get("acceptedAt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let primitive_verification = match signature
                .get("primitiveVerification")
                .and_then(serde_json::Value::as_str)
            {
                Some("failed") => crate::binding::SignaturePrimitiveStatus::Failed {
                    reason: "test-primitive-failed".to_string(),
                },
                Some("verified") => crate::binding::SignaturePrimitiveStatus::Verified,
                _ => crate::binding::SignaturePrimitiveStatus::DeferredPendingHelper {
                    reason: "test-harness-no-signature-primitive".to_string(),
                },
            };
            Ok(Some(vec![crate::binding::SignatureEvidence {
                source_system: "formspec-test-harness".to_string(),
                source_signature_id: "sig013-harness".to_string(),
                signing_act_id: None,
                presentation_hash: None,
                source_response_ref: Some("urn:test:formspec-response:application".to_string()),
                document_id: "application".to_string(),
                signer_id: response
                    .get("data")
                    .and_then(|data| data.get("signerId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                signing_intent: "urn:wos:signing-intent:applicant-signature".to_string(),
                signature_method: test_signature_method(signature),
                signed_payload_digest:
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
                signed_payload_digest_algorithm: "sha-256".to_string(),
                signed_at,
                document_hash,
                document_hash_algorithm: "sha-256".to_string(),
                signature_provider: Some("formspec".to_string()),
                ceremony_id: response
                    .get("data")
                    .and_then(|data| data.get("ceremonyId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                identity_binding: response
                    .get("data")
                    .and_then(|data| data.get("identityBinding"))
                    .cloned(),
                signer_authority: None,
                primitive_verification,
                verification_receipt: signature
                    .get("verificationReceipt")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                admission_failure: test_signature_admission_failure(signature),
            }]))
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
    struct TestIntakeAdapter;

    impl IntakeAcceptanceAdapter for TestIntakeAdapter {
        fn binding(&self) -> &'static str {
            "formspec"
        }

        fn interpret_intake_handoff(
            &self,
            request: &IntakeAcceptanceRequest,
        ) -> Result<IntakeInterpretation, BindingError> {
            let initiation_mode = request
                .document
                .get("initiationMode")
                .and_then(serde_json::Value::as_str);
            if initiation_mode == Some("publicIntake") {
                Ok(IntakeInterpretation {
                    intake_id: request
                        .document
                        .get("handoffId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("test-intake")
                        .to_string(),
                    case_intent: IntakeCaseIntent::RequestGovernedCaseCreation,
                })
            } else if let Some(case_ref) = request.governed_case_ref.clone() {
                Ok(IntakeInterpretation {
                    intake_id: request
                        .document
                        .get("handoffId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("test-intake")
                        .to_string(),
                    case_intent: IntakeCaseIntent::AttachToExistingCase { case_ref },
                })
            } else {
                Ok(IntakeInterpretation {
                    intake_id: request
                        .document
                        .get("handoffId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("test-intake")
                        .to_string(),
                    case_intent: IntakeCaseIntent::RequestGovernedCaseCreation,
                })
            }
        }

        fn finalize_intake_acceptance(
            &self,
            request: &IntakeAcceptanceRequest,
            outcome: &IntakeAcceptanceOutcome,
        ) -> Result<Vec<ProvenanceRecord>, BindingError> {
            let case_ref = match outcome {
                IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                    IntakeCaseDisposition::AttachToExistingCase { .. } => return Ok(Vec::new()),
                    IntakeCaseDisposition::CreateGovernedCase { case_ref, .. } => case_ref.clone(),
                },
                IntakeAcceptanceOutcome::Rejected { .. }
                | IntakeAcceptanceOutcome::Deferred { .. } => return Ok(Vec::new()),
            };

            Ok(vec![ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::CaseCreated,
                timestamp: String::new(),
                actor_id: request.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: Some("wos.kernel.case_created".to_string()),
                data: Some(serde_json::json!({
                    "caseRef": case_ref,
                    "source": "test-intake-adapter"
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
            }])
        }
    }

    #[derive(Debug, Clone, Copy, Default)]
    struct RejectAllIntakePolicy;

    impl IntakeAcceptancePolicy for RejectAllIntakePolicy {
        fn evaluate_intake_acceptance(
            &self,
            _context: &IntakePolicyContext,
        ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
            Ok(IntakeAcceptanceDecision::rejected("manualReviewRequired"))
        }
    }

    /// Policy that forwards to [`NoopIntakeAcceptancePolicy`] and appends a marker provenance row.
    #[derive(Debug, Clone, Copy, Default)]
    struct PolicyEmittingMarkerProvenance;

    impl IntakeAcceptancePolicy for PolicyEmittingMarkerProvenance {
        fn evaluate_intake_acceptance(
            &self,
            context: &IntakePolicyContext,
        ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
            let mut decision = NoopIntakeAcceptancePolicy.evaluate_intake_acceptance(context)?;
            decision.provenance.push(ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::NarrativeTierRecorded,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: Some("wos.kernel.intake_policy_marker".to_string()),
                data: Some(serde_json::json!({ "marker": "policyEmitted" })),
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
            Ok(decision)
        }
    }

    #[derive(Debug, Default)]
    struct FailingStore {
        inner: InMemoryStore,
        fail_on_save_call: usize,
        save_calls: usize,
        observed_save_calls: Arc<AtomicUsize>,
    }

    impl FailingStore {
        fn new(fail_on_save_call: usize) -> Self {
            Self {
                inner: InMemoryStore::new(),
                fail_on_save_call,
                save_calls: 0,
                observed_save_calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[derive(Debug, Default)]
    struct IntakeSaveFailingStore {
        inner: InMemoryStore,
        fail_on_save_call: usize,
        save_calls: usize,
    }

    impl IntakeSaveFailingStore {
        fn new(fail_on_save_call: usize) -> Self {
            Self {
                inner: InMemoryStore::new(),
                fail_on_save_call,
                save_calls: 0,
            }
        }
    }

    impl RuntimeStore for IntakeSaveFailingStore {
        fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.create_record(record)
        }

        fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(process_id)
        }

        fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
            self.inner.processes_for_case(case_ledger_id)
        }

        fn append_provenance_for_case(
            &mut self,
            case_ledger_id: &str,
            process_id: &str,
            record: ProvenanceRecord,
        ) -> Result<(), StoreError> {
            self.inner
                .append_provenance_for_case(case_ledger_id, process_id, record)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.save_record(record)
        }

        fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.create_intake_record(record)
        }

        fn load_intake_record(
            &self,
            binding: &str,
            intake_id: &str,
        ) -> Result<IntakeRecord, StoreError> {
            self.inner.load_intake_record(binding, intake_id)
        }

        fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.save_calls += 1;
            if self.save_calls == self.fail_on_save_call {
                return Err(StoreError::Failed(
                    "injected intake save failure".to_string(),
                ));
            }
            self.inner.save_intake_record(record)
        }
    }

    impl RuntimeStore for FailingStore {
        fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.create_record(record)
        }

        fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(process_id)
        }

        fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
            self.inner.processes_for_case(case_ledger_id)
        }

        fn append_provenance_for_case(
            &mut self,
            case_ledger_id: &str,
            process_id: &str,
            record: ProvenanceRecord,
        ) -> Result<(), StoreError> {
            self.inner
                .append_provenance_for_case(case_ledger_id, process_id, record)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.save_calls += 1;
            self.observed_save_calls.fetch_add(1, Ordering::SeqCst);
            if self.save_calls == self.fail_on_save_call {
                return Err(StoreError::Failed("injected save failure".to_string()));
            }
            self.inner.save_record(record)
        }

        fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.create_intake_record(record)
        }

        fn load_intake_record(
            &self,
            binding: &str,
            intake_id: &str,
        ) -> Result<IntakeRecord, StoreError> {
            self.inner.load_intake_record(binding, intake_id)
        }

        fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.save_intake_record(record)
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

        fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(process_id)
        }

        fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
            self.inner.processes_for_case(case_ledger_id)
        }

        fn append_provenance_for_case(
            &mut self,
            case_ledger_id: &str,
            process_id: &str,
            record: ProvenanceRecord,
        ) -> Result<(), StoreError> {
            self.inner
                .append_provenance_for_case(case_ledger_id, process_id, record)
        }

        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.save_record(record)
        }

        fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.create_intake_record(record)
        }

        fn load_intake_record(
            &self,
            binding: &str,
            intake_id: &str,
        ) -> Result<IntakeRecord, StoreError> {
            self.inner.load_intake_record(binding, intake_id)
        }

        fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.save_intake_record(record)
        }
    }

    fn formspec_bindings() -> BindingRegistry {
        let mut bindings = BindingRegistry::new();
        bindings.register(TestAdapter);
        bindings
    }

    fn formspec_bindings_sig013_harness() -> BindingRegistry {
        let mut bindings = BindingRegistry::new();
        bindings.register(Sig013HarnessFormspecAdapter);
        bindings
    }

    fn unavailable_bindings() -> BindingRegistry {
        let mut bindings = BindingRegistry::new();
        bindings.register(UnavailableAdapter);
        bindings
    }

    fn intake_acceptors() -> IntakeAcceptanceRegistry {
        let mut acceptors = IntakeAcceptanceRegistry::new();
        acceptors.register(TestIntakeAdapter);
        acceptors
    }

    fn runtime_with_store<S>(store: S, kernel: KernelDocument) -> WosRuntime
    where
        S: RuntimeStore + Send + Sync + 'static,
    {
        WosRuntime::new(
            store,
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
        .with_intake_acceptors(intake_acceptors())
    }

    fn runtime_with_store_kernels<S>(store: S, kernels: Vec<KernelDocument>) -> WosRuntime
    where
        S: RuntimeStore + Send + Sync + 'static,
    {
        WosRuntime::new(
            store,
            TestResolver::with_kernels(kernels),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings(),
        )
        .with_intake_acceptors(intake_acceptors())
    }

    fn runtime_with_store_sig013_harness<S>(store: S, kernel: KernelDocument) -> WosRuntime
    where
        S: RuntimeStore + Send + Sync + 'static,
    {
        WosRuntime::new(
            store,
            TestResolver::with_kernel(kernel),
            RecordingPresenter::default(),
            wos_core::traits::DefaultRuntime::new(),
            RecordingService::with_response(serde_json::Value::Null),
            wos_core::traits::DefaultRuntime::new(),
            FixedClock {
                now_ms: 1_710_000_000_000,
            },
            formspec_bindings_sig013_harness(),
        )
        .with_intake_acceptors(intake_acceptors())
    }

    fn runtime_with_kernel(kernel: KernelDocument) -> WosRuntime {
        runtime_with_store(InMemoryStore::new(), kernel)
    }

    fn runtime_with_kernels(kernels: Vec<KernelDocument>) -> WosRuntime {
        runtime_with_store_kernels(InMemoryStore::new(), kernels)
    }

    fn runtime_with_kernel_sig013_harness(kernel: KernelDocument) -> WosRuntime {
        runtime_with_store_sig013_harness(InMemoryStore::new(), kernel)
    }

    fn assert_drain_result_roundtrip(original: &DrainOnceResult) {
        let encoded = serde_json::to_value(original).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "processedEvent":original.processed_event,
                "processedEventToken":original.processed_event_token,
                "transitions":original.transitions,"provenance":original.provenance,
                "createdTaskIds":original.created_task_ids,"emittedEvents":original.emitted_events,
                "guardEvaluations":original.guard_evaluations,
            })
        );
        let decoded: DrainOnceResult = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.processed_event, original.processed_event);
        assert_eq!(
            decoded.processed_event_token,
            original.processed_event_token
        );
        assert_eq!(decoded.transitions.len(), original.transitions.len());
        for (decoded, original) in decoded.transitions.iter().zip(&original.transitions) {
            assert_eq!(decoded.from, original.from);
            assert_eq!(decoded.to, original.to);
            assert_eq!(decoded.event, original.event);
            assert_eq!(decoded.tags, original.tags);
        }
        assert_eq!(
            serde_json::to_value(&decoded.provenance).unwrap(),
            serde_json::to_value(&original.provenance).unwrap()
        );
        assert_eq!(decoded.created_task_ids, original.created_task_ids);
        assert_eq!(decoded.emitted_events, original.emitted_events);
        assert_eq!(decoded.guard_evaluations, original.guard_evaluations);
    }

    fn assert_draft_result_roundtrip(original: &PersistDraftResult) {
        let encoded = serde_json::to_value(original).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({"artifactId":original.artifact_id})
        );
        let decoded: PersistDraftResult = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.artifact_id, original.artifact_id);
    }

    fn submission_json_fields(
        original: &TaskSubmissionResult,
    ) -> (&'static str, serde_json::Value) {
        match original {
            TaskSubmissionResult::Completed {
                artifact_id,
                case_mutated,
                emitted_event,
            } => (
                "completed",
                serde_json::json!({"artifactId":artifact_id,"caseMutated":case_mutated,"emittedEvent":emitted_event}),
            ),
            TaskSubmissionResult::Failed {
                code,
                emitted_event,
            } => (
                "failed",
                serde_json::json!({"code":code,"emittedEvent":emitted_event}),
            ),
            TaskSubmissionResult::Rejected { code } => {
                ("rejected", serde_json::json!({"code":code}))
            }
        }
    }

    fn assert_submission_result_roundtrip(original: &TaskSubmissionResult) {
        let (kind, fields) = submission_json_fields(original);
        let encoded = serde_json::to_value(original).unwrap();
        let mut expected = serde_json::Map::new();
        expected.insert(kind.into(), fields);
        assert_eq!(encoded, serde_json::Value::Object(expected));
        let decoded: TaskSubmissionResult = serde_json::from_value(encoded).unwrap();
        match (&decoded, original) {
            (
                TaskSubmissionResult::Completed {
                    artifact_id: a,
                    case_mutated: b,
                    emitted_event: c,
                },
                TaskSubmissionResult::Completed {
                    artifact_id: x,
                    case_mutated: y,
                    emitted_event: z,
                },
            ) => {
                assert_eq!(a, x);
                assert_eq!(b, y);
                assert_eq!(c, z);
            }
            (
                TaskSubmissionResult::Failed {
                    code: a,
                    emitted_event: b,
                },
                TaskSubmissionResult::Failed {
                    code: x,
                    emitted_event: y,
                },
            ) => {
                assert_eq!(a, x);
                assert_eq!(b, y);
            }
            (
                TaskSubmissionResult::Rejected { code: a },
                TaskSubmissionResult::Rejected { code: x },
            ) => assert_eq!(a, x),
            other => panic!("native result variant changed: {other:?}"),
        }
    }

    fn assert_aux_submission_format(
        record: &RuntimeRecord,
        original: &TaskSubmissionResult,
        token: &str,
    ) {
        let (kind, mut fields) = submission_json_fields(original);
        fields
            .as_object_mut()
            .unwrap()
            .insert("kind".into(), kind.into());
        let encoded = crate::runtime_aux_to_json(record);
        let entry = encoded["replayEntries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["key"]["token"] == token)
            .unwrap();
        assert_eq!(entry["value"], fields);
        let decoded = crate::runtime_aux_from_json(&encoded).unwrap();
        let replay = decoded
            .replay_entries
            .iter()
            .find(|(key, _)| key.token == token)
            .unwrap()
            .1;
        let ReplayValue::Submission(decoded) = replay else {
            panic!("aux replay variant changed")
        };
        assert_eq!(
            submission_json_fields(decoded),
            submission_json_fields(original)
        );
    }

    #[test]
    fn native_outcome_serde_preserves_options_and_default_idle() {
        let idle = DrainOnceResult::default();
        assert_drain_result_roundtrip(&idle);
        assert_eq!(
            serde_json::to_value(&idle).unwrap(),
            serde_json::json!({
                "processedEvent":null,"processedEventToken":null,"transitions":[],"provenance":[],
                "createdTaskIds":[],"emittedEvents":[],"guardEvaluations":[],
            })
        );
        let options = DrainOnceResult {
            processed_event: Some("event".into()),
            processed_event_token: Some("token".into()),
            ..Default::default()
        };
        assert_drain_result_roundtrip(&options);
        for case_mutated in [false, true] {
            for emitted_event in [None, Some("completed.event".into())] {
                assert_submission_result_roundtrip(&TaskSubmissionResult::Completed {
                    artifact_id: "artifact:exact".into(),
                    case_mutated,
                    emitted_event,
                });
            }
        }
        for emitted_event in [None, Some("failed.event".into())] {
            assert_submission_result_roundtrip(&TaskSubmissionResult::Failed {
                code: "exactFailure".into(),
                emitted_event,
            });
        }
        assert_submission_result_roundtrip(&TaskSubmissionResult::Rejected {
            code: "exactRejection".into(),
        });
        assert_draft_result_roundtrip(&PersistDraftResult {
            artifact_id: "artifact:draft-exact".into(),
        });
        // Required vectors and booleans must not be fabricated from missing data.
        let mut missing = serde_json::to_value(&idle).unwrap();
        missing.as_object_mut().unwrap().remove("provenance");
        assert!(serde_json::from_value::<DrainOnceResult>(missing).is_err());
        assert!(
            serde_json::from_value::<TaskSubmissionResult>(
                serde_json::json!({"completed":{"artifactId":"a","emittedEvent":null}})
            )
            .is_err()
        );
    }

    fn workflow_intake_request(handoff_id: &str, case_ref: &str) -> IntakeAcceptanceRequest {
        IntakeAcceptanceRequest {
            actor_bindings: None,
            document: serde_json::json!({
                "$formspecIntakeHandoff": "1.0",
                "handoffId": handoff_id,
                "initiationMode": "workflowInitiated",
            }),
            actor_id: Some("intake-service".to_string()),
            governed_case_ref: Some(case_ref.to_string()),
            governed_case_definition: None,
            initial_case_state: None,
        }
    }

    fn public_intake_request(
        handoff_id: &str,
        case_ref: &str,
        definition_url: &str,
        definition_version: &str,
    ) -> IntakeAcceptanceRequest {
        IntakeAcceptanceRequest {
            actor_bindings: None,
            document: serde_json::json!({
                "$formspecIntakeHandoff": "1.0",
                "handoffId": handoff_id,
                "initiationMode": "publicIntake",
            }),
            actor_id: Some("intake-service".to_string()),
            governed_case_ref: Some(case_ref.to_string()),
            governed_case_definition: Some(IntakeCaseDefinition {
                definition_url: definition_url.to_string(),
                definition_version: definition_version.to_string(),
            }),
            initial_case_state: Some(serde_json::json!({ "source": "publicIntake" })),
        }
    }

    #[test]
    fn durable_runtime_trait_supports_instance_round_trip() {
        fn exercise_all_trait_methods(runtime: &mut impl DurableRuntime) {
            let created = runtime
                .create_process(CreateProcessRequest {
                    process_id: test_process_id("case-trait").to_string(),
                    tenant: None,
                    definition_url: "urn:test:durable-trait".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: Some(serde_json::json!({ "approved": false })),
                })
                .expect("trait create_process");
            let loaded = runtime
                .load_process(test_process_id("case-trait"))
                .expect("trait load_process");
            assert_eq!(created.process_id, loaded.process_id);
            assert_eq!(created.definition_url, loaded.definition_url);
            assert_eq!(created.definition_version, loaded.definition_version);

            runtime
                .enqueue_event(
                    test_process_id("case-trait"),
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
                .drain_until_idle(test_process_id("case-trait"))
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
                .drain_once(test_process_id("case-trait"))
                .expect("trait drain_once on idle queue");
            assert!(
                once.processed_event.is_none(),
                "queue should be idle after drain_until_idle"
            );

            let window = runtime
                .load_provenance_window(test_process_id("case-trait"), 0, 50)
                .expect("trait load_provenance_window");
            assert!(!window.is_empty());

            let custody_window = runtime
                .load_custody_append_window(
                    test_process_id("case-trait"),
                    0,
                    50,
                    crate::CustodyAppendContext {
                        event_type_prefix: "wos.kernel".to_string(),
                        case_id: None,
                        max_inline_record_bytes: None,
                        workflow_ref: Some("urn:test:durable-trait".to_string()),
                    },
                )
                .expect("trait load_custody_append_window");
            assert_eq!(custody_window.len(), window.len());
            assert_eq!(
                custody_window[0].idempotency_tuple(),
                (created.case_ledger_id.as_str(), window[0].id.as_str())
            );
            assert!(
                custody_window[0].event_type.starts_with("wos.kernel."),
                "unexpected custody event type: {}",
                custody_window[0].event_type
            );
            let mut expected_custody_record =
                serde_json::to_value(&window[0]).expect("json provenance");
            if window[0].record_kind.canonical_event_literal().is_some() {
                expected_custody_record
                    .as_object_mut()
                    .expect("provenance object")
                    .remove("recordKind");
            }
            assert_eq!(
                custody_window[0]
                    .record_json_view()
                    .expect("decoded custody json"),
                expected_custody_record
            );

            runtime
                .persist_task_draft(
                    &task_id,
                    serde_json::json!({ "status": "in-progress" }),
                    "reviewer",
                    None,
                )
                .expect("trait persist_task_draft");

            let dismissal = runtime
                .dismiss_task(
                    &task_id,
                    "trait exercise cleanup",
                    "reviewer",
                    "trait-dismiss",
                )
                .expect("trait dismiss_task");
            let dismissal_window = runtime
                .load_provenance_window(test_process_id("case-trait"), 0, 100)
                .unwrap();
            assert!(
                dismissal_window
                    .iter()
                    .any(|entry| entry.id == dismissal.provenance_id
                        && entry.actor_id.as_deref() == Some("reviewer"))
            );
            assert!(!dismissal.occurred_at.is_empty());

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

            let record_id = window[0].id.clone();
            runtime
                .apply_custody_receipt(
                    created.process_id.as_str(),
                    &record_id,
                    crate::CustodyAppendReceipt::new(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    ),
                )
                .expect("trait apply_custody_receipt");
        }

        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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

    #[test]
    fn accept_intake_handoff_attaches_provenance_to_existing_case() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-dispatch",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        );

        let created = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: test_process_id("case-2026-0042").to_string(),
                    tenant: None,
                    definition_url: "urn:test:intake-dispatch".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                test_case_ledger_urn("case-2026-0042").to_string(),
            )
            .expect("create case before attach");

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                workflow_intake_request("ih-attach", test_case_ledger_urn("case-2026-0042")),
            )
            .expect("intake accepted");

        assert_eq!(
            result.outcome,
            IntakeAcceptanceOutcome::Accepted {
                case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                    case_ref: created.case_ledger_id.clone()
                }
            }
        );
        assert!(
            result
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::IntakeAccepted)
        );
        let provenance = runtime
            .load_provenance_window(&created.process_id, 0, 20)
            .expect("load attached provenance");
        let intake_accepted = provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::IntakeAccepted)
            .expect("intake accepted provenance");
        assert_eq!(
            intake_accepted
                .data
                .as_ref()
                .and_then(|data| data.get("caseLedgerId"))
                .and_then(serde_json::Value::as_str),
            Some(created.case_ledger_id.as_str())
        );
    }

    #[test]
    fn accept_intake_handoff_rejects_ambiguous_case_ledger_ref() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-ambiguous",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        );
        let case_ledger_id = stack_common_typeid::mint_case_ledger_id();
        let case_ref = format!("urn:wos:{case_ledger_id}");
        let created_a = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: stack_common_typeid::mint_process_id(),
                    tenant: None,
                    definition_url: "urn:test:intake-ambiguous".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                case_ref.clone(),
            )
            .expect("create process a");
        let created_b = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: stack_common_typeid::mint_process_id(),
                    tenant: None,
                    definition_url: "urn:test:intake-ambiguous".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                case_ref.clone(),
            )
            .expect("create process b");

        let err = runtime
            .accept_intake_handoff(
                "formspec",
                workflow_intake_request("ih-ambiguous", &case_ref),
            )
            .expect_err("ambiguous case-ledger intake must fail closed");

        let RuntimeError::IntakeConflict(message) = err else {
            panic!("expected intake conflict for ambiguous case ledger, got {err:?}");
        };
        assert!(
            message.contains("multiple processes"),
            "unexpected conflict: {message}",
        );
        assert!(
            message.contains(&created_a.process_id) && message.contains(&created_b.process_id),
            "conflict should name the competing processes: {message}",
        );

        for process_id in [&created_a.process_id, &created_b.process_id] {
            let provenance = runtime
                .load_provenance_window(process_id, 0, 20)
                .expect("load process provenance");
            assert!(
                !provenance
                    .iter()
                    .any(|record| record.record_kind == ProvenanceKind::IntakeAccepted),
                "ambiguous intake must not append IntakeAccepted to {process_id}",
            );
        }
    }

    #[test]
    fn accept_intake_handoff_intake_policy_provenance_survives_prepare() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-policy-prov",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(PolicyEmittingMarkerProvenance);

        runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: test_process_id("case-policy-prov").to_string(),
                    tenant: None,
                    definition_url: "urn:test:intake-policy-prov".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                test_case_ledger_urn("case-policy-prov").to_string(),
            )
            .expect("create case");

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                workflow_intake_request("ih-policy-prov", test_case_ledger_urn("case-policy-prov")),
            )
            .expect("intake accepted");

        assert!(
            result.provenance.iter().any(|record| {
                record.event.as_deref() == Some("wos.kernel.intake_policy_marker")
                    && record.record_kind == ProvenanceKind::NarrativeTierRecorded
            }),
            "policy-emitted provenance must survive pending persistence and prepare"
        );
        assert!(
            result
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::IntakeAccepted),
            "runtime outcome provenance must still be present"
        );
    }

    fn outcome_metadata_fixture(
        completion: Option<&str>,
        failure: Option<&str>,
    ) -> (wos_core::model::kernel::Action, ActiveTask) {
        let mut action = serde_json::json!({
            "action": "createTask", "taskRef": "review", "assignTo": "reviewer"
        });
        if let Some(event) = completion {
            action["completionEvent"] = serde_json::json!(event);
        }
        if let Some(event) = failure {
            action["failureEvent"] = serde_json::json!(event);
        }
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:outcome-metadata", "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": {
                "type": "atomic", "onEntry": [action]
            } } }
        }))
        .unwrap();
        let selected = kernel.lifecycle.states["open"].on_entry[0].clone();
        let mut runtime = runtime_with_kernel(kernel);
        let id = test_process_id("outcome-metadata").to_string();
        runtime
            .create_process(CreateProcessRequest {
                process_id: id.clone(),
                tenant: None,
                definition_url: "urn:test:outcome-metadata".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            })
            .unwrap();
        let task = runtime.load_process(&id).unwrap().active_tasks.remove(0);
        (selected, task)
    }

    #[test]
    fn task_outcome_metadata_matches_actual_runtime_and_retained_codec_values() {
        for (completion, failure) in [
            (None, None),
            (Some("review.completed"), None),
            (None, Some("review.failed")),
            (Some("review.completed"), Some("review.failed")),
        ] {
            let (action, task) = outcome_metadata_fixture(completion, failure);
            let before = serde_json::to_value(&task).unwrap();
            assert!(crate::task_outcome_metadata_matches(
                &action,
                &task.extensions
            ));
            let retained: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_value(serde_json::to_value(&task.extensions).unwrap()).unwrap();
            assert!(crate::task_outcome_metadata_matches(&action, &retained));
            assert_eq!(retained, task.extensions);
            assert_eq!(serde_json::to_value(&task).unwrap(), before);
        }
    }

    #[test]
    fn task_outcome_metadata_refuses_mismatched_missing_unknown_and_unsupported_facts() {
        let (action, task) =
            outcome_metadata_fixture(Some("review.completed"), Some("review.failed"));
        let action_before = serde_json::to_value(&action).unwrap();
        let task_before = serde_json::to_value(&task).unwrap();
        for key in [COMPLETION_EVENT_EXTENSION_KEY, FAILURE_EVENT_EXTENSION_KEY] {
            for value in [
                serde_json::json!("different"),
                serde_json::json!(""),
                serde_json::json!(" "),
                serde_json::json!(42),
                serde_json::Value::Null,
                serde_json::json!([]),
            ] {
                let mut changed = task.extensions.clone();
                changed.insert(key.into(), value);
                assert!(!crate::task_outcome_metadata_matches(&action, &changed));
            }
            let mut missing = task.extensions.clone();
            missing.remove(key);
            assert!(!crate::task_outcome_metadata_matches(&action, &missing));
        }
        assert!(!crate::task_outcome_metadata_matches(
            &action,
            &Default::default()
        ));
        let mut unknown = task.extensions.clone();
        unknown.insert("x-unknown-access".into(), serde_json::json!(true));
        assert!(!crate::task_outcome_metadata_matches(&action, &unknown));
        let (no_event, _) = outcome_metadata_fixture(None, None);
        assert!(!crate::task_outcome_metadata_matches(
            &no_event,
            &task.extensions
        ));
        let mut unsupported = action.clone();
        unsupported
            .extensions
            .insert("x-wos-signature-step".into(), serde_json::json!("review"));
        assert!(!crate::task_outcome_metadata_matches(
            &unsupported,
            &task.extensions
        ));
        unsupported = action.clone();
        unsupported.action = wos_core::model::kernel::ActionKind::SetData;
        assert!(!crate::task_outcome_metadata_matches(
            &unsupported,
            &task.extensions
        ));
        for blank in ["", " "] {
            let mut declared = action.clone();
            declared.completion_event = Some(blank.into());
            let mut actual = task.extensions.clone();
            actual.insert(
                COMPLETION_EVENT_EXTENSION_KEY.into(),
                serde_json::json!(blank),
            );
            assert!(!crate::task_outcome_metadata_matches(&declared, &actual));
        }
        assert_eq!(serde_json::to_value(&action).unwrap(), action_before);
        assert_eq!(serde_json::to_value(&task).unwrap(), task_before);
    }

    fn creation_snapshot_retry(fail_on_save: usize) {
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:creation-snapshot", "version": "1.0.0",
            "governance": { "taskCatalog": [{ "id": "review", "pattern": "review", "title": "Original Review" }] },
            "lifecycle": { "initialState": "open", "states": {
                "open": { "type": "atomic", "onEntry": [{
                    "action": "createTask", "taskRef": "review", "assignTo": "reviewer"
                }] }
            } }
        }))
        .expect("kernel");
        let mut runtime = runtime_with_store(IntakeSaveFailingStore::new(fail_on_save), kernel)
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        let request = public_intake_request(
            "ih-creation-snapshot",
            test_case_ledger_urn("snapshot-case"),
            "urn:test:creation-snapshot",
            "1.0.0",
        );
        assert!(matches!(
            runtime.accept_intake_handoff("formspec", request.clone()),
            Err(RuntimeError::Store(StoreError::Failed(_)))
        ));
        let mut record = runtime
            .load_single_record_by_case_ledger_id(
                WorkflowProcess::extract_urn_type_id(request.governed_case_ref.as_deref().unwrap())
                    .unwrap(),
            )
            .expect("original process persisted before receipt failure");
        let original = record.creation.clone().expect("atomic creation evidence");
        assert_eq!(original.tasks.len(), 1);
        assert_eq!(
            original.tasks[0].assigned_actor.as_deref(),
            Some("reviewer")
        );
        assert_eq!(original.tasks[0].task_ref, "review");
        assert_eq!(
            original.tasks[0].catalog_title.as_deref(),
            Some("Original Review")
        );
        assert_eq!(original.tasks[0].catalog_pattern.as_deref(), Some("review"));
        assert_eq!(
            original.tasks[0].task_id,
            record.process.active_tasks[0].task_id
        );
        assert_eq!(
            original.tasks[0].created_at,
            record.process.active_tasks[0].created_at
        );
        let mut wrong_parent = original.clone();
        wrong_parent.process_id = stack_common_typeid::mint_process_id();
        assert!(matches!(
            wrong_parent.validate(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut wrong_time = original.clone();
        wrong_time.tasks[0].created_at = "invalid-time".into();
        assert!(matches!(
            wrong_time.validate(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut duplicate = original.clone();
        duplicate.tasks.push(original.tasks[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut initial_forgery = runtime.store.load_record(&original.process_id).unwrap();
        initial_forgery.creation.as_mut().unwrap().tasks[0].assigned_actor =
            Some("forged-original".into());
        assert!(matches!(
            initial_forgery.validate_initial_creation(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut changed_display = runtime.store.load_record(&original.process_id).unwrap();
        changed_display.creation.as_mut().unwrap().tasks[0].catalog_title =
            Some("Later Title".into());
        assert!(matches!(
            runtime.store.save_record(changed_display),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        // Round-trip the exact maintained storage codec, then mutate current state.
        let aux = crate::runtime_aux_from_json(&crate::runtime_aux_to_json(&record)).unwrap();
        record.creation = aux.creation;
        record.process.active_tasks[0].assigned_actor = Some("later-reviewer".into());
        record.process.active_tasks[0].task_ref = "later-title".into();
        runtime
            .store
            .save_record(record.clone())
            .expect("intervening reassignment");
        record.process.active_tasks.clear();
        runtime
            .store
            .save_record(record)
            .expect("simulate intervening task completion");
        let accepted = runtime
            .accept_intake_handoff("formspec", request.clone())
            .expect("durable retry");
        let snapshot = accepted.creation.expect("retained initial tasks");
        assert_eq!(
            serde_json::to_value(&snapshot).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        assert!(
            runtime
                .load_process(&snapshot.process_id)
                .unwrap()
                .active_tasks
                .is_empty()
        );
        let replay = runtime
            .accept_intake_handoff("formspec", request)
            .expect("applied replay");
        assert_eq!(
            serde_json::to_value(replay.creation).unwrap(),
            serde_json::to_value(Some(original)).unwrap()
        );
        let provenance = runtime
            .load_provenance_window(&snapshot.process_id, 0, 100)
            .unwrap();
        assert_eq!(
            provenance
                .iter()
                .filter(|row| row.record_kind == ProvenanceKind::IntakeAccepted)
                .count(),
            1
        );
        assert_eq!(
            provenance
                .iter()
                .filter(|row| row.record_kind == ProvenanceKind::CaseCreated)
                .count(),
            1
        );
    }

    #[test]
    fn intake_creation_snapshot_survives_prepared_write_failure_and_task_removal() {
        creation_snapshot_retry(1);
    }

    #[test]
    fn intake_creation_snapshot_survives_applied_write_failure_and_task_removal() {
        creation_snapshot_retry(2);
    }

    #[test]
    fn intake_creation_snapshot_refuses_missing_prepared_and_applied_evidence() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:required-snapshot", "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        let mut runtime =
            runtime_with_kernel(kernel.clone()).with_intake_policy(AutoCreatePublicIntakePolicy);
        let request = public_intake_request(
            "ih-required-snapshot",
            test_case_ledger_urn("required-snapshot"),
            "urn:test:required-snapshot",
            "1.0.0",
        );
        let accepted = runtime
            .accept_intake_handoff("formspec", request.clone())
            .unwrap();
        let original = runtime
            .store
            .load_intake_record("formspec", "ih-required-snapshot")
            .unwrap();
        for status in [
            crate::IntakeRecordStatus::Prepared,
            crate::IntakeRecordStatus::Applied,
        ] {
            let mut missing = original.clone();
            missing.status = status;
            missing.creation = None;
            assert!(matches!(
                missing.validate_creation(),
                Err(StoreError::InvalidCreationEvidence(_))
            ));
            let mut restored = runtime_with_kernel(kernel.clone())
                .with_intake_policy(AutoCreatePublicIntakePolicy);
            assert!(matches!(
                restored.store.create_intake_record(missing),
                Err(StoreError::InvalidCreationEvidence(_))
            ));
            assert!(matches!(
                restored
                    .store
                    .load_intake_record("formspec", "ih-required-snapshot"),
                Err(StoreError::NotFound(_))
            ));
        }
        let replay = runtime.accept_intake_handoff("formspec", request).unwrap();
        assert_eq!(replay.creation, accepted.creation);
        let mut backward = original.clone();
        backward.status = crate::IntakeRecordStatus::Pending;
        assert!(matches!(
            runtime.store.save_intake_record(backward),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let retained = runtime
            .store
            .load_intake_record("formspec", "ih-required-snapshot")
            .unwrap();
        assert_eq!(retained.creation, original.creation);
        assert_eq!(retained.status, original.status);
    }

    #[test]
    fn intake_creation_snapshot_policy_supplied_evidence_is_ignored() {
        struct SpoofPolicy;
        impl IntakeAcceptancePolicy for SpoofPolicy {
            fn evaluate_intake_acceptance(
                &self,
                context: &IntakePolicyContext,
            ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
                let mut decision =
                    AutoCreatePublicIntakePolicy.evaluate_intake_acceptance(context)?;
                decision.creation = Some(crate::ProcessCreationSnapshot {
                    actor_bindings: None,
                    process_id: "forged".into(),
                    case_ledger_id: "forged".into(),
                    tenant: "forged".into(),
                    definition_url: "forged".into(),
                    definition_version: "forged".into(),
                    created_at: "forged".into(),
                    tasks: Vec::new(),
                });
                Ok(decision)
            }
        }
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:policy-snapshot", "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        let mut runtime = runtime_with_kernel(kernel).with_intake_policy(SpoofPolicy);
        let result = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-policy-snapshot",
                    test_case_ledger_urn("policy-snapshot"),
                    "urn:test:policy-snapshot",
                    "1.0.0",
                ),
            )
            .unwrap();
        let evidence = result.creation.unwrap();
        evidence.validate().unwrap();
        assert_eq!(evidence.definition_url, "urn:test:policy-snapshot");
        assert_ne!(evidence.process_id, "forged");
    }

    #[test]
    fn intake_creation_snapshot_receipt_store_refuses_replacement_and_erasure() {
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:receipt-immutable", "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        let mut runtime =
            runtime_with_kernel(kernel).with_intake_policy(AutoCreatePublicIntakePolicy);
        runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-immutable",
                    test_case_ledger_urn("immutable"),
                    "urn:test:receipt-immutable",
                    "1.0.0",
                ),
            )
            .unwrap();
        let original = runtime
            .store
            .load_intake_record("formspec", "ih-immutable")
            .unwrap();
        let mut changed = original.clone();
        changed.creation.as_mut().unwrap().created_at = "2026-01-01T00:00:00Z".into();
        assert!(matches!(
            runtime.store.save_intake_record(changed),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut erased = original.clone();
        erased.creation = None;
        assert!(matches!(
            runtime.store.save_intake_record(erased),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        assert_eq!(
            runtime
                .store
                .load_intake_record("formspec", "ih-immutable")
                .unwrap()
                .creation,
            original.creation
        );
        let mut forged = original.clone();
        forged.creation.as_mut().unwrap().case_ledger_id =
            stack_common_typeid::mint_case_ledger_id();
        assert!(matches!(
            forged.validate_creation(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
    }

    #[test]
    fn intake_creation_snapshot_capture_refuses_duplicate_or_malformed_catalog() {
        for catalog in [
            serde_json::json!([{ "id": "review", "title": "One" }, { "id": "review", "title": "Two" }]),
            serde_json::json!([{ "id": "review", "title": 42 }]),
        ] {
            let kernel = serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0", "url": "urn:test:bad-catalog", "version": "1.0.0",
                "governance": { "taskCatalog": catalog },
                "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic", "onEntry": [{ "action": "createTask", "taskRef": "review", "assignTo": "reviewer" }] } } }
            })).unwrap();
            let mut runtime =
                runtime_with_kernel(kernel).with_intake_policy(AutoCreatePublicIntakePolicy);
            let case = test_case_ledger_urn("bad-catalog");
            assert!(matches!(
                runtime.accept_intake_handoff(
                    "formspec",
                    public_intake_request(
                        "ih-bad-catalog",
                        case.clone(),
                        "urn:test:bad-catalog",
                        "1.0.0"
                    )
                ),
                Err(RuntimeError::Store(StoreError::InvalidCreationEvidence(_)))
            ));
            assert!(
                runtime
                    .store
                    .processes_for_case(WorkflowProcess::extract_urn_type_id(&case).unwrap())
                    .is_empty()
            );
        }
    }

    fn allocation_record_snapshot(record: &RuntimeRecord) -> serde_json::Value {
        serde_json::json!({
            "process": record.process,
            "provenance": record.provenance_log,
            "aux": crate::runtime_aux_to_json(record),
        })
    }

    fn allocation_test_kernel() -> KernelDocument {
        serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:retained-allocation", "version": "1.0.0",
            "governance": { "taskCatalog": [{ "id": "review", "pattern": "review", "title": "Review" }] },
            "lifecycle": { "initialState": "open", "states": {
                "open": { "type": "atomic", "onEntry": [{
                    "action": "createTask", "taskRef": "review", "assignTo": "reviewer"
                }] }
            } }
        })).expect("allocation kernel")
    }

    fn allocation_test_identity() -> crate::intake::ProcessAllocation {
        crate::intake::ProcessAllocation {
            process_id: test_process_id("allocated").into(),
            case_ledger_id: test_case_ledger_id("allocated").into(),
            tenant: "allocated".into(),
        }
    }

    fn allocation_test_request(id: &str) -> IntakeAcceptanceRequest {
        public_intake_request(
            id,
            test_case_ledger_urn("allocated"),
            "urn:test:retained-allocation",
            "1.0.0",
        )
    }

    #[test]
    fn intake_allocation_invalid_or_conflicting_identity_has_no_receipt_or_process() {
        let valid = allocation_test_identity();
        let mut variants = vec![];
        for (process_id, case_ledger_id, tenant) in [
            ("", valid.case_ledger_id.as_str(), "allocated"),
            (
                valid.case_ledger_id.as_str(),
                valid.case_ledger_id.as_str(),
                "allocated",
            ),
            (valid.process_id.as_str(), "not-a-case", "allocated"),
            (
                valid.process_id.as_str(),
                "allocated_case_01hw7rm71vfay8vvw14d2pf2dc",
                "allocated",
            ),
            (
                valid.process_id.as_str(),
                valid.case_ledger_id.as_str(),
                "other",
            ),
            (valid.process_id.as_str(), valid.case_ledger_id.as_str(), ""),
            (
                valid.process_id.as_str(),
                test_case_ledger_id("other"),
                "allocated",
            ),
        ] {
            variants.push(crate::intake::ProcessAllocation {
                process_id: process_id.into(),
                case_ledger_id: case_ledger_id.into(),
                tenant: tenant.into(),
            });
        }
        for allocation in variants {
            let mut runtime = runtime_with_kernel(allocation_test_kernel())
                .with_intake_policy(AutoCreatePublicIntakePolicy);
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    allocation_test_request("ih-invalid-allocation"),
                    Some(allocation)
                ),
                Err(RuntimeError::IntakeConflict(_))
            ));
            assert!(matches!(
                runtime
                    .store
                    .load_intake_record("formspec", "ih-invalid-allocation"),
                Err(StoreError::NotFound(_))
            ));
            assert!(
                runtime
                    .store
                    .processes_for_case(&valid.case_ledger_id)
                    .is_empty()
            );
        }
    }

    #[test]
    fn intake_unused_allocation_refuses_rejection_defer_and_attachment() {
        for deferred in [false, true] {
            let mut runtime = runtime_with_kernel(allocation_test_kernel());
            runtime = if deferred {
                runtime.with_intake_policy(ManualReviewIntakePolicy)
            } else {
                runtime.with_intake_policy(PublicIntakeDisabledPolicy)
            };
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    allocation_test_request("ih-unused"),
                    Some(allocation_test_identity())
                ),
                Err(RuntimeError::IntakeConflict(_))
            ));
            assert!(matches!(
                runtime.store.load_intake_record("formspec", "ih-unused"),
                Err(StoreError::NotFound(_))
            ));
        }
        let mut runtime = runtime_with_kernel(allocation_test_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        runtime
            .accept_intake_handoff_with_allocation(
                "formspec",
                allocation_test_request("ih-create"),
                Some(allocation_test_identity()),
            )
            .expect("create");
        let before = allocation_record_snapshot(
            &runtime
                .store
                .load_record(test_process_id("allocated"))
                .unwrap(),
        );
        let request = workflow_intake_request("ih-attach", test_case_ledger_urn("allocated"));
        assert!(matches!(
            runtime.accept_intake_handoff_with_allocation(
                "formspec",
                request,
                Some(allocation_test_identity())
            ),
            Err(RuntimeError::IntakeConflict(_))
        ));
        assert_eq!(
            before,
            allocation_record_snapshot(
                &runtime
                    .store
                    .load_record(test_process_id("allocated"))
                    .unwrap()
            )
        );
        assert!(matches!(
            runtime.store.load_intake_record("formspec", "ih-attach"),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn intake_retained_allocation_recovery_preserves_original_and_refuses_swaps() {
        // Failure at the first receipt update retains Pending plus an actual process;
        // failure at the second retains Prepared plus original creation evidence.
        for fail_on_save in [1, 2] {
            let mut runtime = runtime_with_store(
                IntakeSaveFailingStore::new(fail_on_save),
                allocation_test_kernel(),
            )
            .with_intake_policy(AutoCreatePublicIntakePolicy);
            let request = allocation_test_request("ih-recovery-allocation");
            let allocation = allocation_test_identity();
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    request.clone(),
                    Some(allocation.clone())
                ),
                Err(RuntimeError::Store(StoreError::Failed(_)))
            ));
            let prior = runtime.store.load_record(&allocation.process_id).unwrap();
            let receipt = runtime
                .store
                .load_intake_record("formspec", "ih-recovery-allocation")
                .unwrap();
            let mut swapped = allocation.clone();
            swapped.process_id = "allocated_process_01hw7rm71vfay8vvw14d2pf2dc".into();
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    request.clone(),
                    Some(swapped)
                ),
                Err(RuntimeError::IntakeConflict(_))
            ));
            assert_eq!(
                allocation_record_snapshot(&prior),
                allocation_record_snapshot(
                    &runtime.store.load_record(&allocation.process_id).unwrap()
                )
            );
            assert_eq!(
                serde_json::to_value(&receipt).unwrap(),
                serde_json::to_value(
                    runtime
                        .store
                        .load_intake_record("formspec", "ih-recovery-allocation")
                        .unwrap()
                )
                .unwrap()
            );
            let decision = runtime
                .accept_intake_handoff_with_allocation(
                    "formspec",
                    request.clone(),
                    Some(allocation.clone()),
                )
                .expect("recovery");
            assert_eq!(decision.creation.as_ref(), prior.creation.as_ref());
            let applied = runtime
                .store
                .load_intake_record("formspec", "ih-recovery-allocation")
                .unwrap();
            assert_eq!(applied.request, request);
            let mut swapped = allocation.clone();
            swapped.process_id = "allocated_process_01hw7rm71vfay8vvw14d2pf2dc".into();
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    request.clone(),
                    Some(swapped)
                ),
                Err(RuntimeError::IntakeConflict(_))
            ));
            let mut changed = request.clone();
            changed.initial_case_state = Some(serde_json::json!({"changed":true}));
            assert!(matches!(
                runtime.accept_intake_handoff_with_allocation(
                    "formspec",
                    changed,
                    Some(allocation.clone())
                ),
                Err(RuntimeError::IntakeConflict(_))
            ));
            let replay = runtime
                .accept_intake_handoff_with_allocation("formspec", request, Some(allocation))
                .expect("applied replay");
            assert_eq!(
                serde_json::to_value(&decision).unwrap(),
                serde_json::to_value(&replay).unwrap()
            );
        }
    }

    #[test]
    fn intake_allocation_preserves_empty_binding_option_and_refuses_existing_unbound_process() {
        let mut kernel_value = serde_json::to_value(allocation_test_kernel()).unwrap();
        kernel_value["lifecycle"]["states"]["open"]["onEntry"] = serde_json::json!([]);
        let kernel: KernelDocument = serde_json::from_value(kernel_value).unwrap();
        let mut runtime =
            runtime_with_kernel(kernel.clone()).with_intake_policy(AutoCreatePublicIntakePolicy);
        let mut request = allocation_test_request("ih-bound-empty");
        request.actor_bindings = Some(vec![]);
        let decision = runtime
            .accept_intake_handoff_with_allocation(
                "formspec",
                request.clone(),
                Some(allocation_test_identity()),
            )
            .expect("explicit empty binding map");
        assert_eq!(decision.creation.unwrap().actor_bindings, Some(vec![]));
        assert_eq!(
            runtime
                .store
                .load_record(test_process_id("allocated"))
                .unwrap()
                .process
                .actor_bindings,
            Some(vec![])
        );
        let mut changed = request;
        changed.actor_bindings = None;
        assert!(matches!(
            runtime.accept_intake_handoff_with_allocation(
                "formspec",
                changed,
                Some(allocation_test_identity())
            ),
            Err(RuntimeError::IntakeConflict(_))
        ));

        let mut runtime =
            runtime_with_kernel(kernel).with_intake_policy(AutoCreatePublicIntakePolicy);
        runtime
            .accept_intake_handoff_with_allocation(
                "formspec",
                allocation_test_request("ih-unbound"),
                Some(allocation_test_identity()),
            )
            .expect("unbound creation");
        let before = allocation_record_snapshot(
            &runtime
                .store
                .load_record(test_process_id("allocated"))
                .unwrap(),
        );
        let mut request = allocation_test_request("ih-new-bound-empty");
        request.actor_bindings = Some(vec![]);
        assert!(matches!(
            runtime.accept_intake_handoff_with_allocation(
                "formspec",
                request,
                Some(allocation_test_identity())
            ),
            Err(RuntimeError::IntakeConflict(_))
        ));
        assert_eq!(
            before,
            allocation_record_snapshot(
                &runtime
                    .store
                    .load_record(test_process_id("allocated"))
                    .unwrap()
            )
        );
    }

    #[test]
    fn intake_pending_before_creation_uses_host_protected_allocation_without_native_reservation_claim()
     {
        let mut runtime = runtime_with_kernel(allocation_test_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        let request = allocation_test_request("ih-pending-no-process");
        let decision = NoopIntakeAcceptancePolicy
            .evaluate_intake_acceptance(&IntakePolicyContext {
                binding: "formspec".into(),
                request: request.clone(),
                interpretation: IntakeInterpretation {
                    intake_id: "ih-pending-no-process".into(),
                    case_intent: IntakeCaseIntent::RequestGovernedCaseCreation,
                },
            })
            .unwrap();
        runtime
            .store
            .create_intake_record(IntakeRecord {
                binding: "formspec".into(),
                intake_id: "ih-pending-no-process".into(),
                request: request.clone(),
                outcome: decision.outcome,
                creation: None,
                provenance_log: decision.provenance,
                status: crate::intake::IntakeRecordStatus::Pending,
                recorded_at: "2024-03-09T16:00:00Z".into(),
                updated_at: "2024-03-09T16:00:00Z".into(),
            })
            .unwrap();
        // No native process evidence exists yet. The host's immutable command row,
        // rather than this Pending receipt, protects the choice of valid process ID.
        let mut allocation = allocation_test_identity();
        allocation.process_id = "allocated_process_01hw7rm71vfay8vvw14d2pf2dc".into();
        let result = runtime
            .accept_intake_handoff_with_allocation(
                "formspec",
                request.clone(),
                Some(allocation.clone()),
            )
            .expect("host-protected recovery allocation");
        assert_eq!(result.creation.unwrap().process_id, allocation.process_id);
        assert_eq!(
            runtime
                .store
                .load_intake_record("formspec", "ih-pending-no-process")
                .unwrap()
                .request,
            request
        );
    }

    #[test]
    fn intake_honors_retained_owner_allocation() {
        let mut runtime = runtime_with_kernel(allocation_test_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        let reserved_process = test_process_id("allocated");
        let request = public_intake_request(
            "ih-allocation",
            test_case_ledger_urn("allocated"),
            "urn:test:retained-allocation",
            "1.0.0",
        );
        let allocation = allocation_test_identity();
        let decision = runtime
            .accept_intake_handoff_with_allocation(
                "formspec",
                request.clone(),
                Some(allocation.clone()),
            )
            .expect("accepted");
        let replay = runtime
            .accept_intake_handoff_with_allocation("formspec", request, Some(allocation))
            .expect("exact replay");
        assert_eq!(
            serde_json::to_value(&decision).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        let original = decision.creation.expect("original creation");
        assert_eq!(original.process_id, reserved_process);
        assert_eq!(original.case_ledger_id, test_case_ledger_id("allocated"));
        assert_eq!(original.tenant, "allocated");
        assert_eq!(original.tasks.len(), 1);
        assert_eq!(original.tasks[0].task_ref, "review");
        assert_eq!(
            original.tasks[0].assigned_actor.as_deref(),
            Some("reviewer")
        );
        let stored = runtime
            .store
            .load_record(reserved_process)
            .expect("exact reserved process");
        assert_eq!(stored.creation.as_ref(), Some(&original));
        assert_eq!(stored.process.actor_bindings, None);
    }

    #[test]
    fn accept_intake_handoff_creates_case_for_public_intake() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-public-create",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(AutoCreatePublicIntakePolicy);

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-public-create",
                    test_case_ledger_urn("case-2026-0050"),
                    "urn:test:intake-public-create",
                    "1.0.0",
                ),
            )
            .expect("intake decision");

        let created_case_ref = match &result.outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::CreateGovernedCase {
                    case_ref,
                    definition,
                    initial_case_state,
                } => {
                    assert_eq!(
                        definition,
                        &IntakeCaseDefinition {
                            definition_url: "urn:test:intake-public-create".to_string(),
                            definition_version: "1.0.0".to_string(),
                        }
                    );
                    assert_eq!(
                        initial_case_state,
                        &Some(serde_json::json!({ "source": "publicIntake" }))
                    );
                    assert!(
                        WorkflowProcess::is_case_id(case_ref),
                        "public intake create must return canonical case id"
                    );
                    case_ref.clone()
                }
                other => panic!("unexpected case disposition: {other:?}"),
            },
            other => panic!("unexpected intake outcome: {other:?}"),
        };
        let created = runtime
            .load_single_record_by_case_ledger_id(&created_case_ref)
            .expect("created intake case by case ledger")
            .process;
        assert_eq!(created.case_ledger_id, created_case_ref);
        assert_eq!(
            created.case_state.get("source"),
            Some(&serde_json::Value::String("publicIntake".to_string()))
        );
        let provenance = runtime
            .load_provenance_window(&created.process_id, 0, 20)
            .expect("load create provenance");
        let intake_accepted = provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::IntakeAccepted)
            .expect("intake accepted provenance");
        assert_eq!(intake_accepted.actor_id.as_deref(), Some("intake-service"));
        assert_eq!(intake_accepted.actor_type.as_deref(), Some("system"));
        assert_eq!(intake_accepted.lifecycle_state.as_deref(), Some("open"));
        assert_eq!(
            intake_accepted
                .data
                .as_ref()
                .and_then(|data| data.get("caseLedgerId"))
                .and_then(serde_json::Value::as_str),
            Some(created_case_ref.as_str())
        );

        let case_created = provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::CaseCreated)
            .expect("case created provenance");
        assert_eq!(case_created.actor_id.as_deref(), Some("intake-service"));
        assert_eq!(case_created.actor_type.as_deref(), Some("system"));
        assert_eq!(case_created.lifecycle_state.as_deref(), Some("open"));
        assert_eq!(
            case_created
                .data
                .as_ref()
                .and_then(|data| data.get("caseLedgerId"))
                .and_then(serde_json::Value::as_str),
            Some(created_case_ref.as_str())
        );
        assert_eq!(case_created.outputs, vec![created_case_ref.clone()]);
    }

    #[test]
    fn accept_intake_handoff_allows_policy_rejection() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-policy-reject",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(RejectAllIntakePolicy);

        runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: test_process_id("case-2026-0042").to_string(),
                    tenant: None,
                    definition_url: "urn:test:intake-policy-reject".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                test_case_ledger_urn("case-2026-0042").to_string(),
            )
            .expect("create case before rejection");

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                workflow_intake_request("ih-reject", test_case_ledger_urn("case-2026-0042")),
            )
            .expect("intake decision");

        assert_eq!(
            result.outcome,
            IntakeAcceptanceOutcome::Rejected {
                code: "manualReviewRequired".to_string()
            }
        );
        assert_eq!(result.provenance.len(), 1);
        assert_eq!(
            result.provenance[0].record_kind,
            ProvenanceKind::IntakeRejected
        );
    }

    #[test]
    fn public_intake_disabled_policy_rejects_public_case_creation() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-disabled",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(PublicIntakeDisabledPolicy);

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-disabled",
                    test_case_ledger_urn("case-2026-0052"),
                    "urn:test:intake-disabled",
                    "1.0.0",
                ),
            )
            .expect("disabled policy decision");

        assert_eq!(
            result.outcome,
            IntakeAcceptanceOutcome::Rejected {
                code: "publicIntakeDisabled".to_string()
            }
        );
        assert_eq!(
            result.provenance[0].record_kind,
            ProvenanceKind::IntakeRejected
        );
    }

    #[test]
    fn accept_intake_handoff_replays_existing_decision_by_intake_id() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-replay",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(ManualReviewIntakePolicy);

        let first = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-replay",
                    test_case_ledger_urn("case-2026-0051"),
                    "urn:test:intake-replay",
                    "1.0.0",
                ),
            )
            .expect("first intake decision");
        let second = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-replay",
                    test_case_ledger_urn("case-2026-0051"),
                    "urn:test:intake-replay",
                    "1.0.0",
                ),
            )
            .expect("replayed intake decision");

        assert_eq!(first.outcome, second.outcome);
        assert_eq!(first.provenance.len(), second.provenance.len());
        assert_eq!(first.provenance[0].id, second.provenance[0].id);
        assert_eq!(
            first.provenance[0].record_kind,
            ProvenanceKind::IntakeDeferred
        );
    }

    #[test]
    fn accept_intake_handoff_rejects_request_mismatch_on_replay() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-replay-conflict",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(ManualReviewIntakePolicy);

        runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-replay-conflict",
                    test_case_ledger_urn("case-2026-0053"),
                    "urn:test:intake-replay-conflict",
                    "1.0.0",
                ),
            )
            .expect("first intake decision");

        let mut actor_changed = public_intake_request(
            "ih-replay-conflict",
            test_case_ledger_urn("case-2026-0053"),
            "urn:test:intake-replay-conflict",
            "1.0.0",
        );
        actor_changed.actor_id = Some("different-intake-service".to_string());
        let actor_conflict = runtime
            .accept_intake_handoff("formspec", actor_changed)
            .expect_err("actor mismatch must conflict");
        assert!(matches!(actor_conflict, RuntimeError::IntakeConflict(_)));

        let case_ref_conflict = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-replay-conflict",
                    test_case_ledger_urn("case-2026-0054"),
                    "urn:test:intake-replay-conflict",
                    "1.0.0",
                ),
            )
            .expect_err("case metadata mismatch must conflict");
        assert!(matches!(case_ref_conflict, RuntimeError::IntakeConflict(_)));
    }

    #[test]
    fn accept_intake_handoff_replays_after_applied_receipt_save_failure_without_duplicate_provenance()
     {
        let mut runtime = runtime_with_store(
            IntakeSaveFailingStore::new(2),
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-save-failure",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(AutoCreatePublicIntakePolicy);

        let first_error = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-save-failure",
                    test_case_ledger_urn("case-save-failure"),
                    "urn:test:intake-save-failure",
                    "1.0.0",
                ),
            )
            .expect_err("applied intake receipt save should fail once");
        assert!(matches!(
            first_error,
            RuntimeError::Store(StoreError::Failed(_))
        ));

        let replayed = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-save-failure",
                    test_case_ledger_urn("case-save-failure"),
                    "urn:test:intake-save-failure",
                    "1.0.0",
                ),
            )
            .expect("retry should replay prepared create");

        let created_case_ref = match &replayed.outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::CreateGovernedCase { case_ref, .. } => case_ref.clone(),
                other => panic!("unexpected case disposition: {other:?}"),
            },
            other => panic!("unexpected intake outcome: {other:?}"),
        };

        let created = runtime
            .load_single_record_by_case_ledger_id(&created_case_ref)
            .expect("created case record")
            .process;
        let provenance = runtime
            .load_provenance_window(&created.process_id, 0, 20)
            .expect("load created case provenance");
        assert_eq!(
            provenance
                .iter()
                .filter(|record| record.record_kind == ProvenanceKind::IntakeAccepted)
                .count(),
            1
        );
        assert_eq!(
            provenance
                .iter()
                .filter(|record| record.record_kind == ProvenanceKind::CaseCreated)
                .count(),
            1
        );
    }

    #[test]
    fn accept_intake_handoff_returns_canonical_case_ref_for_public_intake_id() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-legacy-alias",
                "version": "1.0.0",
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(AutoCreatePublicIntakePolicy);

        let decision = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-canonical-case-ref",
                    test_case_ledger_urn("case-canonical-ref"),
                    "urn:test:intake-legacy-alias",
                    "1.0.0",
                ),
            )
            .expect("case creation must succeed");

        let case_ref = match &decision.outcome {
            IntakeAcceptanceOutcome::Accepted { case_disposition } => match case_disposition {
                IntakeCaseDisposition::CreateGovernedCase { case_ref, .. } => case_ref.clone(),
                other => panic!("unexpected case disposition: {other:?}"),
            },
            other => panic!("unexpected intake outcome: {other:?}"),
        };
        assert!(
            WorkflowProcess::is_case_id(&case_ref),
            "public intake create must return canonical TypeID, got {case_ref}"
        );
        assert_eq!(case_ref, test_case_ledger_id("case-canonical-ref"));

        let loaded_by_case = runtime
            .load_single_record_by_case_ledger_id(&case_ref)
            .expect("load created case by case ledger")
            .process;
        assert_eq!(loaded_by_case.case_ledger_id, case_ref);
        assert_ne!(loaded_by_case.process_id, loaded_by_case.case_ledger_id);

        let intake_record = decision
            .provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::IntakeAccepted)
            .expect("runtime intake accepted provenance");
        assert_eq!(
            intake_record
                .data
                .as_ref()
                .and_then(|data| data.get("caseRef"))
                .and_then(serde_json::Value::as_str),
            Some(case_ref.as_str())
        );
        assert_eq!(
            intake_record
                .data
                .as_ref()
                .and_then(|data| data.get("caseLedgerId"))
                .and_then(serde_json::Value::as_str),
            Some(case_ref.as_str())
        );

        let case_created = decision
            .provenance
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::CaseCreated)
            .expect("binding case created provenance");
        assert_eq!(
            case_created
                .data
                .as_ref()
                .and_then(|data| data.get("caseRef"))
                .and_then(serde_json::Value::as_str),
            Some(case_ref.as_str())
        );
        assert_eq!(
            case_created
                .data
                .as_ref()
                .and_then(|data| data.get("caseLedgerId"))
                .and_then(serde_json::Value::as_str),
            Some(case_ref.as_str())
        );
    }

    #[test]
    fn rejected_intake_provenance_is_runtime_populated() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-reject-populated",
                "version": "1.0.0",
                "actors": [{ "id": "intake-service", "type": "system" }],
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(PublicIntakeDisabledPolicy);

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-reject-populated",
                    test_case_ledger_urn("case-2026-0055"),
                    "urn:test:intake-reject-populated",
                    "1.0.0",
                ),
            )
            .expect("reject decision");

        let record = result
            .provenance
            .iter()
            .find(|entry| entry.record_kind == ProvenanceKind::IntakeRejected)
            .expect("rejected provenance");
        assert_eq!(record.actor_id.as_deref(), Some("intake-service"));
        assert_eq!(record.actor_type.as_deref(), Some("system"));
        assert_eq!(record.lifecycle_state.as_deref(), Some("open"));
        assert_eq!(record.audit_layer.as_deref(), Some("facts"));
        assert_eq!(record.definition_version.as_deref(), Some("1.0.0"));
        assert!(
            !record.timestamp.is_empty(),
            "rejected intake provenance must be stamped"
        );
    }

    #[test]
    fn deferred_intake_provenance_is_runtime_populated() {
        let mut runtime = runtime_with_kernel(
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:intake-defer-populated",
                "version": "1.0.0",
                "actors": [{ "id": "intake-service", "type": "system" }],
                "lifecycle": {
                    "initialState": "open",
                    "states": {
                        "open": { "type": "atomic" }
                    }
                }
            }))
            .expect("kernel json"),
        )
        .with_intake_policy(ManualReviewIntakePolicy);

        let result = runtime
            .accept_intake_handoff(
                "formspec",
                public_intake_request(
                    "ih-defer-populated",
                    test_case_ledger_urn("case-2026-0056"),
                    "urn:test:intake-defer-populated",
                    "1.0.0",
                ),
            )
            .expect("defer decision");

        let record = result
            .provenance
            .iter()
            .find(|entry| entry.record_kind == ProvenanceKind::IntakeDeferred)
            .expect("deferred provenance");
        assert_eq!(record.actor_id.as_deref(), Some("intake-service"));
        assert_eq!(record.actor_type.as_deref(), Some("system"));
        assert_eq!(record.lifecycle_state.as_deref(), Some("open"));
        assert_eq!(record.audit_layer.as_deref(), Some("facts"));
        assert_eq!(record.definition_version.as_deref(), Some("1.0.0"));
        assert!(
            !record.timestamp.is_empty(),
            "deferred intake provenance must be stamped"
        );
    }

    #[test]
    fn accept_intake_handoff_rejects_unsupported_binding() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:intake-unsupported",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .expect("kernel json");
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
            formspec_bindings(),
        );

        let error = runtime
            .accept_intake_handoff(
                "unknown",
                IntakeAcceptanceRequest {
                    actor_bindings: None,
                    document: serde_json::json!({}),
                    actor_id: None,
                    governed_case_ref: None,
                    governed_case_definition: None,
                    initial_case_state: None,
                },
            )
            .unwrap_err();

        assert!(matches!(error, RuntimeError::UnsupportedBinding(_)));
        assert!(
            error
                .to_string()
                .contains("intake acceptance binding unsupported")
        );
    }

    #[test]
    fn custody_receipt_stamps_canonical_event_hash_on_persisted_provenance() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:receipt",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .expect("kernel json");
        let mut runtime = runtime_with_kernel(kernel);
        let created = runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-receipt").to_string(),
                tenant: None,
                definition_url: "urn:test:receipt".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect("create instance");
        let provenance = runtime
            .load_provenance_window(test_process_id("case-receipt"), 0, 1)
            .expect("load provenance");
        let record_id = provenance[0].id.clone();

        runtime
            .apply_custody_receipt(
                test_process_id("case-receipt"),
                &record_id,
                crate::CustodyAppendReceipt::new(
                    "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
                ),
            )
            .expect("apply receipt");

        let persisted = runtime
            .load_provenance_window(created.process_id.as_str(), 0, 1)
            .expect("reload provenance");
        assert_eq!(
            persisted[0].canonical_event_hash.as_deref(),
            Some("9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c")
        );
    }

    #[test]
    fn custody_receipt_reapply_is_idempotent_when_hash_matches() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:receipt-idem",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .expect("kernel json");
        let mut runtime = runtime_with_kernel(kernel);
        let _created = runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-receipt-idem").to_string(),
                tenant: None,
                definition_url: "urn:test:receipt-idem".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect("create instance");
        let provenance = runtime
            .load_provenance_window(test_process_id("case-receipt-idem"), 0, 1)
            .expect("load provenance");
        let record_id = provenance[0].id.clone();
        let hash = "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c";
        let receipt = crate::CustodyAppendReceipt::new(hash);
        runtime
            .apply_custody_receipt(
                test_process_id("case-receipt-idem"),
                &record_id,
                receipt.clone(),
            )
            .expect("first apply");
        runtime
            .apply_custody_receipt(test_process_id("case-receipt-idem"), &record_id, receipt)
            .expect("idempotent reapply");
        let persisted = runtime
            .load_provenance_window(test_process_id("case-receipt-idem"), 0, 1)
            .expect("reload");
        assert_eq!(persisted[0].canonical_event_hash.as_deref(), Some(hash));
    }

    #[test]
    fn custody_receipt_conflict_when_hash_differs() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:receipt-conflict",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                }
            }
        }))
        .expect("kernel json");
        let mut runtime = runtime_with_kernel(kernel);
        let _created = runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-receipt-conflict").to_string(),
                tenant: None,
                definition_url: "urn:test:receipt-conflict".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect("create instance");
        let provenance = runtime
            .load_provenance_window(test_process_id("case-receipt-conflict"), 0, 1)
            .expect("load provenance");
        let record_id = provenance[0].id.clone();
        runtime
            .apply_custody_receipt(
                test_process_id("case-receipt-conflict"),
                &record_id,
                crate::CustodyAppendReceipt::new(
                    "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
                ),
            )
            .expect("first apply");
        let err = runtime
            .apply_custody_receipt(
                test_process_id("case-receipt-conflict"),
                &record_id,
                crate::CustodyAppendReceipt::new(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                ),
            )
            .expect_err("second hash must conflict");
        assert!(matches!(err, RuntimeError::CustodyReceiptConflict { .. }));
    }

    #[test]
    fn create_process_rejects_case_typeid_as_process_id() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let pre_minted = stack_common_typeid::mint_case_ledger_id();
        let err = runtime
            .create_process(CreateProcessRequest {
                process_id: pre_minted.clone(),
                tenant: None,
                definition_url: "urn:test:populator".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect_err("case TypeID must not be accepted as a process id");
        assert!(matches!(err, RuntimeError::MigrationRejected(_)));
    }

    #[test]
    fn create_process_preserves_pre_minted_process_typeid() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let pre_minted = stack_common_typeid::mint_process_id();
        let created = runtime
            .create_process(CreateProcessRequest {
                process_id: pre_minted.clone(),
                tenant: None,
                definition_url: "urn:test:populator".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect("create");

        assert_eq!(created.process_id, pre_minted);
        assert!(WorkflowProcess::is_case_id(&created.case_ledger_id));
        assert!(
            created
                .extensions
                .get("x-wos-legacy-instance-alias")
                .is_none(),
            "pre-minted process TypeID must not produce a legacy alias"
        );
    }

    #[test]
    fn create_process_can_bind_multiple_processes_to_one_case_ledger() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let case_ledger_id = stack_common_typeid::mint_case_ledger_id();
        let process_a = stack_common_typeid::mint_process_id();
        let process_b = stack_common_typeid::mint_process_id();

        let created_a = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: format!("urn:wos:{process_a}"),
                    tenant: None,
                    definition_url: "urn:test:populator".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                format!("urn:wos:{case_ledger_id}"),
            )
            .expect("create process a");
        let created_b = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: format!("urn:wos:{process_b}"),
                    tenant: None,
                    definition_url: "urn:test:populator".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                case_ledger_id.clone(),
            )
            .expect("create process b");

        assert_eq!(created_a.case_ledger_id.as_str(), case_ledger_id.as_str());
        assert_eq!(created_b.case_ledger_id.as_str(), case_ledger_id.as_str());
        assert_eq!(created_a.process_id.as_str(), process_a.as_str());
        assert_eq!(created_b.process_id.as_str(), process_b.as_str());
        assert_ne!(created_a.process_id, created_b.process_id);

        let loaded_a = runtime
            .load_process(&created_a.process_id)
            .expect("load process a");
        let loaded_b = runtime
            .load_process(&created_b.process_id)
            .expect("load process b");
        assert_eq!(loaded_a.case_ledger_id.as_str(), case_ledger_id.as_str());
        assert_eq!(loaded_b.case_ledger_id.as_str(), case_ledger_id.as_str());

        // `processes_for_case` returns both processes bound to the case ledger.
        let mut found = runtime.processes_for_case(case_ledger_id.as_str());
        found.sort();
        let mut expected = vec![created_a.process_id.clone(), created_b.process_id.clone()];
        expected.sort();
        assert_eq!(found, expected);

        // Unknown case ledger ids yield an empty vector (not a NotFound error).
        let unrelated = stack_common_typeid::mint_case_ledger_id();
        assert!(
            runtime.processes_for_case(unrelated.as_str()).is_empty(),
            "unbound case ledger should return empty Vec",
        );
    }

    #[test]
    fn process_api_rejects_ambiguous_case_ledger_ref() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let case_ledger_id = stack_common_typeid::mint_case_ledger_id();
        let created_a = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: stack_common_typeid::mint_process_id(),
                    tenant: None,
                    definition_url: "urn:test:populator".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                case_ledger_id.clone(),
            )
            .expect("create process a");
        let created_b = runtime
            .create_process_bound_to_case(
                CreateProcessRequest {
                    process_id: stack_common_typeid::mint_process_id(),
                    tenant: None,
                    definition_url: "urn:test:populator".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                },
                case_ledger_id.clone(),
            )
            .expect("create process b");

        for err in [
            runtime
                .load_process(&case_ledger_id)
                .expect_err("load_process must reject ambiguous case ledger"),
            runtime
                .load_provenance_window(&case_ledger_id, 0, 20)
                .expect_err("load_provenance_window must reject ambiguous case ledger"),
        ] {
            let RuntimeError::CaseLedgerConflict(message) = err else {
                panic!("expected case ledger conflict, got {err:?}");
            };
            assert!(
                message.contains("multiple processes"),
                "unexpected conflict: {message}",
            );
            assert!(
                message.contains(&created_a.process_id) && message.contains(&created_b.process_id),
                "conflict should name the competing processes: {message}",
            );
        }
    }

    #[test]
    fn create_process_mints_id_for_empty_string() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let created = runtime
            .create_process(CreateProcessRequest {
                process_id: "   ".to_string(),
                tenant: None,
                definition_url: "urn:test:populator".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect("create");
        assert!(
            WorkflowProcess::is_process_id(&created.process_id),
            "empty process_id must be replaced with a minted process TypeID, got {}",
            created.process_id
        );
        assert!(WorkflowProcess::is_case_id(&created.case_ledger_id));
        assert!(
            created
                .extensions
                .get("x-wos-legacy-instance-alias")
                .is_none(),
            "empty process_id must not produce a legacy alias"
        );
    }

    #[test]
    fn create_process_rejects_legacy_process_name() {
        let kernel = kernel_with_actors("1.0.0", serde_json::json!([]));
        let mut runtime = runtime_with_kernel(kernel);
        let err = runtime
            .create_process(CreateProcessRequest {
                process_id: "legacy-case-42".to_string(),
                tenant: None,
                definition_url: "urn:test:populator".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .expect_err("legacy names must not be accepted as process ids");
        assert!(matches!(err, RuntimeError::MigrationRejected(_)));
    }

    /// Kernel + Signature Profile documents shared with conformance `SIG-013`
    /// (`fixtures/kernel/signature-runtime.json` +
    /// `fixtures/profiles/signature-runtime-sequential.json`).
    fn sig013_harness_documents() -> (KernelDocument, SignatureProfileDocument) {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kernel_path = manifest.join("../../fixtures/kernel/signature-runtime.json");
        let profile_path =
            manifest.join("../../fixtures/profiles/signature-runtime-sequential.json");
        let kernel_json = fs::read_to_string(&kernel_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", kernel_path.display()));
        let kernel: KernelDocument =
            serde_json::from_str(&kernel_json).expect("signature-runtime kernel parses");
        let profile_json = fs::read_to_string(&profile_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", profile_path.display()));
        let profile_root: serde_json::Value =
            serde_json::from_str(&profile_json).expect("signature profile JSON parses");
        let sig_block = profile_root
            .get("signature")
            .cloned()
            .expect("profile fixture has top-level signature block");
        let profile: SignatureProfileDocument = serde_json::from_value(sig_block)
            .expect("signature block parses as SignatureProfileDocument");
        (kernel, profile)
    }

    fn submit_sig013_applicant_response(
        runtime: &mut WosRuntime,
        kernel: &KernelDocument,
        process_id: &str,
        token_prefix: &str,
    ) -> TaskSubmissionResult {
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some(format!("{token_prefix}-start")),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "methodUri": "urn:formspec:sig-method:ed25519-cose-sign1@1",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some(&format!("{token_prefix}-submit")),
            )
            .expect("signature admission should complete")
    }

    fn manual_formspec_task(
        process_id: &str,
        ordinal: usize,
        response_mapping_ref: Option<&str>,
    ) -> ActiveTask {
        let task_id = make_task_id(process_id, ordinal as u64, "review");
        ActiveTask {
            declared_actor: None,
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
            expires_at: None,
            expires_with_timer: None,
            impact_level: None,
            context: Some(FormspecTaskContext {
                task_id,
                process_id: process_id.to_string(),
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
    fn create_process_and_drain_create_formspec_task() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-1").to_string(),
                tenant: None,
                definition_url: "urn:test:kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": false })),
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-1"),
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once(test_process_id("case-1")).unwrap();
        assert_drain_result_roundtrip(&result);
        assert_eq!(result.processed_event.as_deref(), Some("start"));
        assert_eq!(result.created_task_ids.len(), 1);
        assert!(result.created_task_ids[0].starts_with("wos-task:"));
        assert!(
            result
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::TaskPresented)
        );

        let instance = runtime.load_process(test_process_id("case-1")).unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-token").to_string(),
                tenant: None,
                definition_url: "urn:test:event-token".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        for token in ["old-token", "target-token"] {
            runtime
                .enqueue_event(
                    test_process_id("case-token"),
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

        let first = runtime.drain_once(test_process_id("case-token")).unwrap();
        assert_drain_result_roundtrip(&first);
        assert_eq!(first.processed_event.as_deref(), Some("same"));
        assert_eq!(first.processed_event_token.as_deref(), Some("old-token"));

        let second = runtime.drain_once(test_process_id("case-token")).unwrap();
        assert_drain_result_roundtrip(&second);
        assert_eq!(second.processed_event.as_deref(), Some("same"));
        assert_eq!(
            second.processed_event_token.as_deref(),
            Some("target-token")
        );
    }

    #[test]
    fn reference_companion_policy_scopes_idempotency_by_instance() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
        for process_id in [test_process_id("case-a"), test_process_id("case-b")] {
            runtime
                .create_process(CreateProcessRequest {
                    process_id: process_id.to_string(),
                    tenant: None,
                    definition_url: "urn:test:companion-idempotency".to_string(),
                    definition_version: "1.0.0".to_string(),
                    initial_case_state: None,
                })
                .unwrap();
            runtime
                .enqueue_event(
                    process_id,
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

        let first = runtime.drain_once(test_process_id("case-a")).unwrap();
        let second = runtime.drain_once(test_process_id("case-b")).unwrap();

        assert!(
            !first
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup)
        );
        assert!(
            !second
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup)
        );
    }

    #[test]
    fn create_process_does_not_present_tasks_if_initial_commit_fails() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-1a").to_string(),
                tenant: None,
                definition_url: "urn:test:presenter-order".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": true })),
            })
            .unwrap_err();

        assert!(matches!(error, RuntimeError::Store(StoreError::Failed(_))));
        assert!(presented.lock().unwrap().is_empty());
    }

    #[test]
    fn create_process_persists_task_state_before_presentation() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-1a").to_string(),
                tenant: None,
                definition_url: "urn:test:presented-state-order".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": true })),
            })
            .unwrap();

        assert_eq!(instance.active_tasks.len(), 1);
        assert_eq!(instance.active_tasks[0].status, ActiveTaskStatus::Assigned);
    }

    #[test]
    fn create_process_and_restore_preserves_timer_duration_metadata() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-1b").to_string(),
                tenant: None,
                definition_url: "urn:test:timer-metadata".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        assert_eq!(instance.timers.len(), 1);
        assert_eq!(instance.timers[0].duration_iso.as_deref(), Some("PT2H"));
        assert_eq!(instance.timers[0].duration_ms, Some(7_200_000));

        let record = runtime
            .store
            .load_record(test_process_id("case-1b"))
            .unwrap();
        let evaluator = Evaluator::from_instance(kernel, &record.process, 1_710_000_000_000)
            .expect("restore evaluator");
        let timer = evaluator.timers().iter().next().expect("restored timer");
        assert_eq!(timer.duration_iso, "PT2H");
        assert_eq!(timer.duration_ms, 7_200_000);
    }

    #[test]
    fn migrate_bumps_definition_version_and_appends_provenance() {
        let mk = |ver: &str| -> KernelDocument {
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "url": "urn:test:migrate-kernel",
                "version": ver,
                "lifecycle": {
                    "initialState": "open",
                    "states": { "open": { "type": "atomic" } }
                },
                "actors": [{"id": "a", "type": "human"}],
                "contracts": {},
            }))
            .unwrap()
        };
        let mut runtime = runtime_with_kernels(vec![mk("1.0.0"), mk("1.1.0")]);
        runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("m-case-1").to_string(),
                tenant: None,
                definition_url: "urn:test:migrate-kernel".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: Some(serde_json::json!({ "x": 1 })),
            })
            .unwrap();
        let out = runtime
            .migrate(
                test_process_id("m-case-1"),
                "1.1.0",
                MigrationMap::default(),
                Some("supervisor-op-1"),
            )
            .unwrap();
        assert_eq!(out.new_definition_version, "1.1.0");
        let inst = runtime.load_process(test_process_id("m-case-1")).unwrap();
        assert_eq!(inst.definition_version, "1.1.0");
        let window = runtime
            .load_provenance_window(test_process_id("m-case-1"), 0, 50)
            .unwrap();
        let last = window.last().expect("provenance");
        assert_eq!(last.record_kind, ProvenanceKind::InstanceMigrated);
        assert_eq!(last.actor_id.as_deref(), Some("supervisor-op-1"));
    }

    #[test]
    fn migrate_rejects_when_target_kernel_missing_active_state() {
        let k1: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:migrate-mismatch",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "open",
                "states": { "open": { "type": "atomic" } }
            },
            "actors": [{"id": "a", "type": "human"}],
            "contracts": {},
        }))
        .unwrap();
        let k2: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:migrate-mismatch",
            "version": "1.1.0",
            "lifecycle": {
                "initialState": "done",
                "states": { "done": { "type": "final" } }
            },
            "actors": [{"id": "a", "type": "human"}],
            "contracts": {},
        }))
        .unwrap();
        let mut runtime = runtime_with_kernels(vec![k1, k2]);
        runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("m-case-2").to_string(),
                tenant: None,
                definition_url: "urn:test:migrate-mismatch".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            })
            .unwrap();
        let err = runtime
            .migrate(
                test_process_id("m-case-2"),
                "1.1.0",
                MigrationMap::default(),
                None,
            )
            .expect_err("open missing in target");
        match err {
            RuntimeError::MigrationRejected(msg) => {
                assert!(msg.contains("state not found"), "unexpected message: {msg}");
            }
            other => panic!("expected MigrationRejected, got {other:?}"),
        }
        assert_eq!(
            runtime
                .load_process(test_process_id("m-case-2"))
                .unwrap()
                .definition_version,
            "1.0.0"
        );
    }

    #[test]
    fn submit_task_response_completes_and_emits_event() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-2").to_string(),
                tenant: None,
                definition_url: "urn:test:submit-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-2"))
            .unwrap();
        let mut task =
            manual_formspec_task(test_process_id("case-2"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        task.extensions.insert(
            COMPLETION_EVENT_EXTENSION_KEY.to_string(),
            serde_json::Value::String("review.completed".to_string()),
        );
        record.process.active_tasks.push(task);
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

        assert_submission_result_roundtrip(&result);
        assert_aux_submission_format(
            &runtime
                .store
                .load_record(test_process_id("case-2"))
                .unwrap(),
            &result,
            "token-1",
        );
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

        let instance = runtime.load_process(test_process_id("case-2")).unwrap();
        assert!(instance.active_tasks.is_empty());
        assert_eq!(instance.case_state["decision"], serde_json::json!(true));
        assert_eq!(instance.pending_events.len(), 1);
        assert_eq!(instance.pending_events[0].event, "review.completed");
    }

    /// Narrow integration harness: same policy floor as conformance SIG-013,
    /// but calls `WosRuntime::submit_task_response` directly (not only the
    /// `wos_conformance::run_fixture` harness).
    #[test]
    fn submit_task_response_sig013_policy_assurance_below_floor_blocks_affirmation() {
        let (kernel, profile) = sig013_harness_documents();
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let process_id = test_process_id("case-sig013-submit-harness");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-harness-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry");

        let err = runtime
            .submit_task_response(
                &task.task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "low"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-harness-submit"),
            )
            .expect_err("below-floor assurance must fail signature affirmation");

        let msg = err.to_string();
        assert!(
            msg.contains("is below policy 'emailOtp'"),
            "unexpected error (expected policy-floor rejection like SIG-013 conformance): {msg}"
        );
    }

    #[test]
    fn submit_task_response_signature_affirmation_records_primitive_verification() {
        let (kernel, profile) = sig013_harness_documents();
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let process_id = test_process_id("case-sig013-submit-affirmation");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-affirmation-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-affirmation-submit"),
            )
            .expect("signature admission should complete");

        assert!(
            matches!(result, TaskSubmissionResult::Completed { .. }),
            "expected completed submission, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        let affirmation = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation)
            .expect("signature affirmation provenance must be appended");
        assert_eq!(
            affirmation.event.as_deref(),
            Some("wos.kernel.signature_affirmation")
        );
        let data = affirmation
            .data
            .as_ref()
            .expect("signature affirmation carries data");
        assert_eq!(
            data["caseLedgerId"],
            serde_json::json!(record.process.case_ledger_id.as_str())
        );
        assert_eq!(
            data["processId"],
            serde_json::json!(record.process.process_id.as_str())
        );
        assert_eq!(data["signerId"], serde_json::json!("applicant"));
        assert_eq!(
            data["sourceSignatureId"],
            serde_json::json!("sig013-harness")
        );
        assert_eq!(
            data["primitiveVerification"],
            serde_json::json!({
                "status": "deferredPendingHelper",
                "reason": "test-harness-no-signature-primitive"
            })
        );
    }

    #[test]
    fn submit_task_response_signature_affirmation_carries_verification_receipt() {
        let (kernel, profile) = sig013_harness_documents();
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let process_id = test_process_id("case-sig013-submit-receipt");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-receipt-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "primitiveVerification": "verified",
                            "verificationReceipt": "0oRWoQExiQEFQnNpZ25lZA==",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-receipt-submit"),
            )
            .expect("signature admission should complete");

        assert!(
            matches!(result, TaskSubmissionResult::Completed { .. }),
            "expected completed submission, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        let affirmation = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation)
            .expect("signature affirmation provenance must be appended");
        let data = affirmation
            .data
            .as_ref()
            .expect("signature affirmation carries data");
        assert_eq!(
            data["primitiveVerification"],
            serde_json::json!({ "status": "verified" })
        );
        assert_eq!(
            data["verificationReceipt"],
            serde_json::json!("0oRWoQExiQEFQnNpZ25lZA==")
        );
    }

    #[test]
    fn submit_task_response_signature_admission_failed_keeps_task_active() {
        let (kernel, profile) = sig013_harness_documents();
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let process_id = test_process_id("case-sig013-submit-admission-failed");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-admission-failed-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "primitiveVerification": "failed",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-admission-failed-submit"),
            )
            .expect("signature admission failure should be represented as a failed submission");

        assert_submission_result_roundtrip(&result);
        assert_aux_submission_format(
            &runtime.store.load_record(process_id).unwrap(),
            &result,
            "sig013-admission-failed-submit",
        );
        assert!(
            matches!(
                result,
                TaskSubmissionResult::Failed {
                    ref code,
                    emitted_event: None
                } if code == "signatureAdmissionFailed"
            ),
            "expected failed submission, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        assert!(
            record
                .process
                .active_tasks
                .iter()
                .any(|task| task.task_id == task_id),
            "admission failure must not remove the active signature task"
        );
        let admission_failed = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .expect("admission failure must emit SignatureAdmissionFailed provenance");
        assert_eq!(
            admission_failed.event.as_deref(),
            Some("wos.kernel.signature_admission_failed")
        );
        let admission_failed_data = admission_failed
            .data
            .as_ref()
            .expect("SignatureAdmissionFailed carries data");
        assert_eq!(
            admission_failed_data["caseLedgerId"],
            serde_json::json!(record.process.case_ledger_id.as_str())
        );
        assert_eq!(
            admission_failed_data["processId"],
            serde_json::json!(record.process.process_id.as_str())
        );
        assert!(
            record.provenance_log.iter().any(|record| {
                record.record_kind == wos_core::ProvenanceKind::TaskFailed
                    && record
                        .data
                        .as_ref()
                        .and_then(|data| data.get("taskId"))
                        .and_then(serde_json::Value::as_str)
                        == Some(task_id.as_str())
            }),
            "admission failure must emit TaskFailed for the attempted task"
        );
        assert!(
            !record.provenance_log.iter().any(|record| {
                record.record_kind == wos_core::ProvenanceKind::TaskCompleted
                    && record
                        .data
                        .as_ref()
                        .and_then(|data| data.get("taskId"))
                        .and_then(serde_json::Value::as_str)
                        == Some(task_id.as_str())
            }),
            "admission failure must not emit TaskCompleted for the attempted task"
        );
    }

    #[test]
    fn submit_task_response_persists_all_binding_reported_admission_failure_reasons() {
        let reasons = [
            "primitive_verification_failed",
            "method_unsupported",
            "method_unregistered",
            "evidence_divergence",
            "posture_floor_unmet",
            "registry_unrecognized_method",
            "adapter_unavailable",
        ];

        for reason in reasons {
            let (kernel, profile) = sig013_harness_documents();
            let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
                .with_signature_profile("signatureProfile", profile);
            let id_suffix = reason.replace('_', "-");
            let process_label = format!("case-sig013-binding-failed-{id_suffix}");
            let process_id = test_process_id(&process_label);
            runtime
                .create_process(CreateProcessRequest {
                    process_id: process_id.to_string(),
                    tenant: None,
                    definition_url: kernel.url.clone().unwrap(),
                    definition_version: kernel.version.clone().unwrap(),
                    initial_case_state: None,
                })
                .expect("create_process");

            runtime
                .enqueue_event(
                    &process_id,
                    PendingEvent {
                        event: "start".to_string(),
                        actor_id: Some("applicant".to_string()),
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: Some(format!("sig013-{id_suffix}-start")),
                    },
                )
                .expect("enqueue start");
            runtime
                .drain_until_idle(&process_id)
                .expect("drain after start");

            let instance = runtime.load_process(&process_id).expect("load instance");
            let task_id = instance
                .active_tasks
                .iter()
                .find(|t| t.task_ref == "applicantTask")
                .expect("applicantTask from signature onEntry")
                .task_id
                .clone();
            let submit_token = format!("sig013-{id_suffix}-submit");

            let result = runtime
                .submit_task_response(
                    &task_id,
                    serde_json::json!({
                        "status": "completed",
                        "definitionUrl": "urn:test:formspec:signature",
                        "definitionVersion": "1.0.0",
                        "data": {
                            "signerId": "applicant",
                            "signatureProvider": "formspec",
                            "ceremonyId": "ceremony-sequential",
                            "identityBinding": {
                                "method": "email-otp",
                                "assuranceLevel": "standard"
                            },
                            "signature": {
                                "acceptedAt": "2026-04-22T12:00:00Z",
                                "affirmed": true,
                                "admissionFailureReason": reason,
                                "failureContext": {
                                    "source": "binding-test",
                                    "reason": reason
                                },
                                "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                            }
                        }
                    }),
                    "applicant",
                    Some(&submit_token),
                )
                .expect("binding-reported admission failure should be persisted");

            assert!(
                matches!(
                    result,
                    TaskSubmissionResult::Failed {
                        ref code,
                        emitted_event: None
                    } if code == "signatureAdmissionFailed"
                ),
                "expected failed submission for {reason}, got {result:?}"
            );

            let record = runtime.store.load_record(&process_id).expect("load record");
            assert!(
                record
                    .process
                    .active_tasks
                    .iter()
                    .any(|task| task.task_id == task_id),
                "binding-reported admission failure must keep the task active for {reason}"
            );
            let admission_failed = record
                .provenance_log
                .iter()
                .find(|record| {
                    record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed
                })
                .unwrap_or_else(|| {
                    panic!("admission failure must emit SignatureAdmissionFailed for {reason}")
                });
            let admission_failed_data = admission_failed
                .data
                .as_ref()
                .expect("SignatureAdmissionFailed carries data");
            assert_eq!(admission_failed_data["reason"], serde_json::json!(reason));
            assert_eq!(
                admission_failed_data["failureContext"],
                serde_json::json!({
                    "source": "binding-test",
                    "reason": reason
                })
            );
            assert_eq!(
                admission_failed_data["evidenceBindings"]["responseId"],
                serde_json::json!(task_id)
            );
            assert!(
                !record.provenance_log.iter().any(|record| {
                    record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation
                }),
                "binding-reported admission failure must not emit SignatureAffirmation for {reason}"
            );
        }
    }

    #[test]
    fn submit_task_response_posture_rejects_disallowed_signature_method() {
        let (kernel, mut profile) = sig013_harness_documents();
        let posture_url = "https://example.gov/posture/signature-runtime-test.json".to_string();
        profile.posture_policy = Some(signature::PosturePolicyRef {
            url: posture_url.clone(),
            version: Some("1.0.0".to_string()),
        });
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let posture: signature::PostureDeclaration = serde_json::from_value(serde_json::json!({
            "$postureDeclaration": "1.0",
            "url": posture_url,
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ecdsa-p256-cose-sign1@1"],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": false
            }
        }))
        .expect("posture declaration parses");
        runtime = runtime.with_posture_declaration(posture);

        let process_id = test_process_id("case-sig013-posture-method-unsupported");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-posture-method-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "methodUri": "urn:formspec:sig-method:ed25519-cose-sign1@1",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-posture-method-submit"),
            )
            .expect("posture method rejection should be persisted as admission failure");

        assert!(
            matches!(
                result,
                TaskSubmissionResult::Failed {
                    ref code,
                    emitted_event: None
                } if code == "signatureAdmissionFailed"
            ),
            "expected posture method rejection, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        let admission_failed = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .expect("posture method rejection emits SignatureAdmissionFailed");
        let data = admission_failed
            .data
            .as_ref()
            .expect("SignatureAdmissionFailed carries data");
        assert_eq!(data["reason"], serde_json::json!("method_unsupported"));
        assert_eq!(
            data["evidenceBindings"]["signingIntent"],
            serde_json::json!("urn:wos:signing-intent:applicant-signature")
        );
        assert!(
            !record.provenance_log.iter().any(|record| {
                record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation
            }),
            "posture method rejection must not emit SignatureAffirmation"
        );
    }

    #[test]
    fn submit_task_response_posture_rejects_substituted_content_for_same_uri() {
        let (kernel, mut profile) = sig013_harness_documents();
        let posture_url =
            "https://example.gov/posture/signature-runtime-substitution-test.json".to_string();
        profile.posture_policy = Some(signature::PosturePolicyRef {
            url: posture_url.clone(),
            version: Some("1.0.0".to_string()),
        });
        let allowed_body = serde_json::json!({
            "$postureDeclaration": "1.0",
            "url": posture_url.clone(),
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ed25519-cose-sign1@1"],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": false
            }
        })
        .to_string();
        let substituted_body = serde_json::json!({
            "$postureDeclaration": "1.0",
            "url": posture_url,
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ecdsa-p256-cose-sign1@1"],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": false
            }
        })
        .to_string();
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile)
            .with_posture_resolver(RotatingPostureResolver::new([
                allowed_body,
                substituted_body,
            ]));

        let first = submit_sig013_applicant_response(
            &mut runtime,
            &kernel,
            test_process_id("case-sig013-posture-substitution-first"),
            "sig013-posture-substitution-first",
        );
        assert!(
            matches!(first, TaskSubmissionResult::Completed { .. }),
            "first admission should use the original posture content, got {first:?}"
        );

        let second = submit_sig013_applicant_response(
            &mut runtime,
            &kernel,
            test_process_id("case-sig013-posture-substitution-second"),
            "sig013-posture-substitution-second",
        );
        assert!(
            matches!(
                second,
                TaskSubmissionResult::Failed {
                    ref code,
                    emitted_event: None
                } if code == "signatureAdmissionFailed"
            ),
            "substituted posture content for the same URI must be re-evaluated, got {second:?}"
        );

        let record = runtime
            .store
            .load_record(test_process_id("case-sig013-posture-substitution-second"))
            .expect("load substituted-content record");
        let admission_failed = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .expect("substituted posture content emits SignatureAdmissionFailed");
        let data = admission_failed
            .data
            .as_ref()
            .expect("SignatureAdmissionFailed carries data");
        assert_eq!(data["reason"], serde_json::json!("method_unsupported"));
    }

    #[test]
    fn submit_task_response_posture_rejects_disallowed_signing_intent() {
        let (kernel, mut profile) = sig013_harness_documents();
        let posture_url =
            "https://example.gov/posture/signature-runtime-intent-test.json".to_string();
        profile.posture_policy = Some(signature::PosturePolicyRef {
            url: posture_url.clone(),
            version: Some("1.0.0".to_string()),
        });
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let posture: signature::PostureDeclaration = serde_json::from_value(serde_json::json!({
            "$postureDeclaration": "1.0",
            "url": posture_url,
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ed25519-cose-sign1@1"],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": false,
                "allowedSigningIntents": ["urn:wos:signing-intent:formal-attestation@1"]
            }
        }))
        .expect("posture declaration parses");
        runtime = runtime.with_posture_declaration(posture);

        let process_id = test_process_id("case-sig013-posture-intent-unsupported");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-posture-intent-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "methodUri": "urn:formspec:sig-method:ed25519-cose-sign1@1",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-posture-intent-submit"),
            )
            .expect("posture intent rejection should be persisted as admission failure");

        assert!(
            matches!(
                result,
                TaskSubmissionResult::Failed {
                    ref code,
                    emitted_event: None
                } if code == "signatureAdmissionFailed"
            ),
            "expected posture intent rejection, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        let admission_failed = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .expect("posture intent rejection emits SignatureAdmissionFailed");
        let data = admission_failed
            .data
            .as_ref()
            .expect("SignatureAdmissionFailed carries data");
        assert_eq!(data["reason"], serde_json::json!("method_unsupported"));
        assert_eq!(
            data["evidenceBindings"]["signingIntent"],
            serde_json::json!("urn:wos:signing-intent:applicant-signature")
        );
        assert!(
            !record.provenance_log.iter().any(|record| {
                record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation
            }),
            "posture intent rejection must not emit SignatureAffirmation"
        );
    }

    #[test]
    fn submit_task_response_posture_requires_verification_receipt_bytes() {
        let (kernel, mut profile) = sig013_harness_documents();
        let posture_url =
            "https://example.gov/posture/signature-runtime-receipt-test.json".to_string();
        profile.posture_policy = Some(signature::PosturePolicyRef {
            url: posture_url.clone(),
            version: Some("1.0.0".to_string()),
        });
        let mut runtime = runtime_with_kernel_sig013_harness(kernel.clone())
            .with_signature_profile("signatureProfile", profile);
        let posture: signature::PostureDeclaration = serde_json::from_value(serde_json::json!({
            "$postureDeclaration": "1.0",
            "url": posture_url,
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ed25519-cose-sign1@1"],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": true
            }
        }))
        .expect("posture declaration parses");
        runtime = runtime.with_posture_declaration(posture);

        let process_id = test_process_id("case-sig013-posture-receipt-required");
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.to_string(),
                tenant: None,
                definition_url: kernel.url.clone().unwrap(),
                definition_version: kernel.version.clone().unwrap(),
                initial_case_state: None,
            })
            .expect("create_process");

        runtime
            .enqueue_event(
                process_id,
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("applicant".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("sig013-posture-receipt-start".to_string()),
                },
            )
            .expect("enqueue start");
        runtime
            .drain_until_idle(process_id)
            .expect("drain after start");

        let instance = runtime.load_process(process_id).expect("load instance");
        let task_id = instance
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "applicantTask")
            .expect("applicantTask from signature onEntry")
            .task_id
            .clone();

        let result = runtime
            .submit_task_response(
                &task_id,
                serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:test:formspec:signature",
                    "definitionVersion": "1.0.0",
                    "data": {
                        "signerId": "applicant",
                        "signatureProvider": "formspec",
                        "ceremonyId": "ceremony-sequential",
                        "identityBinding": {
                            "method": "email-otp",
                            "assuranceLevel": "standard"
                        },
                        "signature": {
                            "acceptedAt": "2026-04-22T12:00:00Z",
                            "affirmed": true,
                            "methodUri": "urn:formspec:sig-method:ed25519-cose-sign1@1",
                            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        }
                    }
                }),
                "applicant",
                Some("sig013-posture-receipt-submit"),
            )
            .expect("missing signed receipt should be persisted as admission failure");

        assert!(
            matches!(
                result,
                TaskSubmissionResult::Failed {
                    ref code,
                    emitted_event: None
                } if code == "signatureAdmissionFailed"
            ),
            "expected posture receipt rejection, got {result:?}"
        );

        let record = runtime.store.load_record(process_id).expect("load record");
        let admission_failed = record
            .provenance_log
            .iter()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .expect("posture receipt rejection emits SignatureAdmissionFailed");
        let data = admission_failed
            .data
            .as_ref()
            .expect("SignatureAdmissionFailed carries data");
        assert_eq!(data["reason"], serde_json::json!("posture_floor_unmet"));
        assert_eq!(
            data["failureContext"]["field"],
            serde_json::json!("verificationReceipt")
        );
        assert!(
            !record.provenance_log.iter().any(|record| {
                record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation
            }),
            "posture receipt rejection must not emit SignatureAffirmation"
        );
    }

    #[test]
    fn persist_task_draft_stores_artifact_without_mutating_case_state() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-3").to_string(),
                tenant: None,
                definition_url: "urn:test:draft-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "decision": "pending" })),
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-3"))
            .unwrap();
        let task = manual_formspec_task(test_process_id("case-3"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
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

        let record = runtime
            .store
            .load_record(test_process_id("case-3"))
            .unwrap();
        assert_draft_result_roundtrip(&draft);
        let aux = crate::runtime_aux_to_json(&record);
        let entry = aux["replayEntries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["key"]["token"] == "draft-token")
            .unwrap();
        assert_eq!(
            entry["value"],
            serde_json::json!({"kind":"draft", "artifactId":draft.artifact_id})
        );
        let decoded_aux = crate::runtime_aux_from_json(&aux).unwrap();
        let replay = decoded_aux
            .replay_entries
            .iter()
            .find(|(key, _)| key.token == "draft-token")
            .unwrap()
            .1;
        let ReplayValue::Draft(decoded_draft) = replay else {
            panic!("aux draft replay variant changed")
        };
        assert_eq!(decoded_draft.artifact_id, draft.artifact_id);
        assert!(record.artifacts.contains_key(&draft.artifact_id));
        assert_eq!(
            record.process.case_state["decision"],
            serde_json::json!("pending")
        );
        assert_eq!(record.process.active_tasks.len(), 1);
        assert!(
            record
                .provenance_log
                .iter()
                .any(|entry| entry.record_kind == ProvenanceKind::TaskDraftPersisted)
        );
    }

    /// Regression: the persist-task-draft append site must route its record
    /// through `populate_provenance_record_fields` so it carries the same
    /// SP §5.3 shape (audit_layer, definition_version, …) as every other
    /// append path. Prior to the Finding 1 fix this site set only
    /// `timestamp` and pushed the record directly, skipping the populator.
    #[test]
    fn persist_task_draft_populates_new_fields() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-draft-pop").to_string(),
                tenant: None,
                definition_url: "urn:test:draft-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-draft-pop"))
            .unwrap();
        let task = manual_formspec_task(
            test_process_id("case-draft-pop"),
            1,
            Some("urn:mapping:response"),
        );
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
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

        let record = runtime
            .store
            .load_record(test_process_id("case-draft-pop"))
            .unwrap();
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

    #[derive(Debug, Clone, Default)]
    struct TaskDelegationProbe {
        allowed: Arc<AtomicBool>,
        checks: Arc<AtomicUsize>,
    }

    impl AccessControl for TaskDelegationProbe {
        fn can_transition(&self, _actor_id: &str, _event: &str) -> bool {
            false
        }

        fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
            false
        }

        fn can_delegate(
            &self,
            delegator_id: &str,
            delegate_id: &str,
            _scope: &wos_core::model::governance::DelegationScope,
        ) -> bool {
            self.checks.fetch_add(1, Ordering::SeqCst);
            self.allowed.load(Ordering::SeqCst)
                && delegator_id == "reviewer"
                && delegate_id == "delegate"
        }
    }

    fn dismissal_fixture() -> (WosRuntime, String, String, RuntimeRecord) {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": "urn:test:dismiss-fixture", "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        let mut runtime = runtime_with_kernel(kernel);
        let process_id = test_process_id("dismiss-fixture").to_string();
        runtime
            .create_process(CreateProcessRequest {
                process_id: process_id.clone(),
                tenant: None,
                definition_url: "urn:test:dismiss-fixture".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "approved": false })),
            })
            .unwrap();
        let initial_record = runtime.store.load_record(&process_id).unwrap();
        let mut record = initial_record.clone();
        let task = manual_formspec_task(&process_id, 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();
        (runtime, process_id, task_id, initial_record)
    }

    #[test]
    fn dismiss_task_refuses_unqualified_actor_without_write_or_presentation() {
        let (mut runtime, process_id, task_id, initial_record) = dismissal_fixture();
        runtime.access_control = Box::new(TaskDelegationProbe::default());
        let presenter = RecordingPresenter::default();
        let calls = presenter.dismissed.clone();
        runtime.presenter = Box::new(presenter);
        let before = runtime.store.load_record(&process_id).unwrap();
        let mut store = FailingStore::new(1);
        store.inner.create_record(initial_record).unwrap();
        store.inner.save_record(before.clone()).unwrap();
        let save_calls = store.observed_save_calls.clone();
        runtime.store = Box::new(store);
        assert!(matches!(
            runtime.dismiss_task(&task_id, "snoozed", "unrelated", "dismiss-1"),
            Err(RuntimeError::Unauthorized(_))
        ));
        assert_eq!(save_calls.load(Ordering::SeqCst), 0);
        let after = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(
            serde_json::to_value(&before.process).unwrap(),
            serde_json::to_value(&after.process).unwrap()
        );
        assert_eq!(before.provenance_log.len(), after.provenance_log.len());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn dismiss_task_records_actual_delegate_and_preserves_runtime_state() {
        let (mut runtime, process_id, task_id, _initial_record) = dismissal_fixture();
        let access = TaskDelegationProbe::default();
        access.allowed.store(true, Ordering::SeqCst);
        let checks = access.checks.clone();
        runtime.access_control = Box::new(access);
        let before = runtime.store.load_record(&process_id).unwrap();
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "delegate", "dismiss-1")
            .unwrap();
        let after = runtime.store.load_record(&process_id).unwrap();
        let entry = after
            .provenance_log
            .iter()
            .find(|entry| entry.id == result.provenance_id)
            .unwrap();
        assert_eq!(entry.actor_id.as_deref(), Some("delegate"));
        assert_eq!(entry.record_kind, ProvenanceKind::TaskDismissed);
        assert_eq!(checks.load(Ordering::SeqCst), 1);
        assert_eq!(
            serde_json::to_value(&before.process.active_tasks).unwrap(),
            serde_json::to_value(&after.process.active_tasks).unwrap()
        );
        assert_eq!(before.process.case_state, after.process.case_state);
        assert_eq!(
            serde_json::to_value(&before.process.pending_events).unwrap(),
            serde_json::to_value(&after.process.pending_events).unwrap()
        );
        assert_eq!(
            before.process.provenance_position + 1,
            after.process.provenance_position
        );
    }

    #[test]
    fn dismiss_task_save_failure_emits_nothing() {
        let (mut runtime, process_id, task_id, initial_record) = dismissal_fixture();
        let before = runtime.store.load_record(&process_id).unwrap();
        let mut store = FailingStore::new(1);
        store.inner.create_record(initial_record).unwrap();
        store.inner.save_record(before.clone()).unwrap();
        runtime.store = Box::new(store);
        let presenter = RecordingPresenter::default();
        let calls = presenter.dismissed.clone();
        runtime.presenter = Box::new(presenter);
        assert!(matches!(
            runtime.dismiss_task(&task_id, "snoozed", "reviewer", "dismiss-1"),
            Err(RuntimeError::Store(StoreError::Failed(_)))
        ));
        let after = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(before.provenance_log.len(), after.provenance_log.len());
        assert_eq!(
            serde_json::to_value(&before.process).unwrap(),
            serde_json::to_value(&after.process).unwrap()
        );
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn dismiss_task_returns_committed_result_without_presentation() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let presenter = RecordingPresenter::default();
        let calls = presenter.dismissed.clone();
        runtime.presenter = Box::new(presenter);
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "dismiss-1")
            .unwrap();
        let replay = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "dismiss-1")
            .unwrap();
        assert_eq!(result, replay);
        let after = runtime.store.load_record(&process_id).unwrap();
        let record = after
            .provenance_log
            .iter()
            .find(|entry| entry.id == result.provenance_id)
            .unwrap();
        assert_eq!(result.occurred_at, record.timestamp);
        assert_eq!(after.process.active_tasks.len(), 1);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn dismiss_task_without_context_records_provenance_without_presentation() {
        let (mut runtime, process_id, task_id, _initial_record) = dismissal_fixture();
        let mut record = runtime.store.load_record(&process_id).unwrap();
        record.process.active_tasks[0].context = None;
        runtime.store.save_record(record).unwrap();
        let presenter = RecordingPresenter::default();
        let calls = presenter.dismissed.clone();
        runtime.presenter = Box::new(presenter);
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "dismiss-1")
            .unwrap();
        assert!(!result.occurred_at.is_empty());
        assert!(calls.lock().unwrap().is_empty());
        assert!(
            runtime
                .store
                .load_record(&process_id)
                .unwrap()
                .provenance_log
                .iter()
                .any(|entry| entry.id == result.provenance_id)
        );
    }

    #[test]
    fn dismiss_task_records_provenance_and_leaves_task_resumable() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-4").to_string(),
                tenant: None,
                definition_url: "urn:test:dismiss-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-4"))
            .unwrap();
        let task = manual_formspec_task(test_process_id("case-4"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let result = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "dismiss-1")
            .unwrap();
        assert!(!result.occurred_at.is_empty());

        let record = runtime
            .store
            .load_record(test_process_id("case-4"))
            .unwrap();
        assert_eq!(record.process.active_tasks.len(), 1);
        assert!(
            record
                .provenance_log
                .iter()
                .any(|entry| entry.record_kind == ProvenanceKind::TaskDismissed
                    && entry.id == result.provenance_id
                    && entry.actor_id.as_deref() == Some("reviewer"))
        );
    }

    #[test]
    fn dismissal_replay_survives_task_removal_and_current_version_change() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let access = TaskDelegationProbe::default();
        access.allowed.store(true, Ordering::SeqCst);
        let allowed = access.allowed.clone();
        let checks = access.checks.clone();
        runtime.access_control = Box::new(access);
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "delegate", "replay-key")
            .unwrap();
        let mut record = runtime.store.load_record(&process_id).unwrap();
        record.process.active_tasks.clear();
        record.process.definition_version = "2.0.0".into();
        runtime.store.save_record(record).unwrap();
        allowed.store(false, Ordering::SeqCst);
        runtime.clock = Box::new(FixedClock { now_ms: u64::MAX });
        let replay = runtime
            .dismiss_task(&task_id, "snoozed", "delegate", "replay-key")
            .unwrap();
        assert_eq!(result, replay);
        assert_eq!(checks.load(Ordering::SeqCst), 1);
        let record = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(
            record
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::TaskDismissed)
                .count(),
            1
        );
        assert!(record.process.active_tasks.is_empty());
        assert_eq!(record.process.definition_version, "2.0.0");
    }

    #[test]
    fn dismissal_replay_conflicts_without_duplicate_natural_effects() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let access = TaskDelegationProbe::default();
        let checks = access.checks.clone();
        runtime.access_control = Box::new(access);
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "key-1")
            .unwrap();
        let before = runtime.store.load_record(&process_id).unwrap();
        for (reason, key) in [("changed", "key-1"), ("snoozed", "key-2")] {
            assert!(matches!(
                runtime.dismiss_task(&task_id, reason, "reviewer", key),
                Err(RuntimeError::DismissalConflict(_))
            ));
        }
        assert!(matches!(
            runtime.dismiss_task(&task_id, "snoozed", "unrelated", "key-1"),
            Err(RuntimeError::Unauthorized(_))
        ));
        assert_eq!(checks.load(Ordering::SeqCst), 1);
        let after = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(
            serde_json::to_value(&before.process).unwrap(),
            serde_json::to_value(&after.process).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&before.provenance_log).unwrap(),
            serde_json::to_value(&after.provenance_log).unwrap()
        );
        assert_eq!(
            crate::store::runtime_aux_to_json(&before),
            crate::store::runtime_aux_to_json(&after)
        );
        assert_eq!(
            runtime
                .dismiss_task(&task_id, "snoozed", "reviewer", "key-1")
                .unwrap(),
            result
        );
        let different = runtime
            .dismiss_task(&task_id, "changed", "reviewer", "key-2")
            .unwrap();
        assert_ne!(different.provenance_id, result.provenance_id);
        let record = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(
            record
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::TaskDismissed)
                .count(),
            2
        );
        assert_eq!(record.process.active_tasks.len(), 1);
    }

    #[test]
    fn dismissal_token_refuses_blank_and_preserves_exact_opaque_bytes() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let before = runtime.store.load_record(&process_id).unwrap();
        for token in ["", " \t\r\n"] {
            assert!(matches!(
                runtime.dismiss_task(&task_id, "snoozed", "reviewer", token),
                Err(RuntimeError::DismissalConflict(_))
            ));
        }
        assert_eq!(
            crate::store::runtime_aux_to_json(&before),
            crate::store::runtime_aux_to_json(&runtime.store.load_record(&process_id).unwrap())
        );
        let token = " padded-key ";
        let result = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", token)
            .unwrap();
        let record = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(record.replay_entries.keys().next().unwrap().token, token);
        let provenance = record
            .provenance_log
            .iter()
            .find(|p| p.id == result.provenance_id)
            .unwrap();
        assert_eq!(
            provenance.data.as_ref().unwrap()["details"]["operationToken"],
            token
        );
        assert!(matches!(
            runtime.dismiss_task(&task_id, "snoozed", "reviewer", "padded-key"),
            Err(RuntimeError::DismissalConflict(_))
        ));
        assert_eq!(
            runtime
                .dismiss_task(&task_id, "snoozed", "reviewer", token)
                .unwrap(),
            result
        );
    }

    struct CommitThenFailDismissalStore {
        inner: SharedStore,
    }

    impl RuntimeStore for CommitThenFailDismissalStore {
        fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.create_record(record)
        }
        fn load_record(&self, id: &str) -> Result<RuntimeRecord, StoreError> {
            self.inner.load_record(id)
        }
        fn processes_for_case(&self, id: &str) -> Vec<String> {
            self.inner.processes_for_case(id)
        }
        fn provenance_for_case(&self, id: &str) -> Vec<ProvenanceRecord> {
            self.inner.provenance_for_case(id)
        }
        fn append_provenance_for_case(
            &mut self,
            case_id: &str,
            process_id: &str,
            record: ProvenanceRecord,
        ) -> Result<(), StoreError> {
            self.inner
                .append_provenance_for_case(case_id, process_id, record)
        }
        fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.create_intake_record(record)
        }
        fn load_intake_record(
            &self,
            binding: &str,
            intake_id: &str,
        ) -> Result<IntakeRecord, StoreError> {
            self.inner.load_intake_record(binding, intake_id)
        }
        fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
            self.inner.save_intake_record(record)
        }
        fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
            self.inner.save_record(record)?;
            Err(StoreError::Failed(
                "injected lost acknowledgement after commit".into(),
            ))
        }
    }

    #[test]
    fn dismissal_reconstructed_retry_reconciles_committed_save_error() {
        let (mut runtime, process_id, task_id, initial) = dismissal_fixture();
        let mut store = SharedStore::default();
        store.create_record(initial).unwrap();
        store
            .save_record(runtime.store.load_record(&process_id).unwrap())
            .unwrap();
        runtime.store = Box::new(CommitThenFailDismissalStore {
            inner: store.clone(),
        });
        let presenter = RecordingPresenter::default();
        let effects = presenter.dismissed.clone();
        runtime.presenter = Box::new(presenter);
        assert!(matches!(
            runtime.dismiss_task(&task_id, "snoozed", "reviewer", "uncertain-key"),
            Err(RuntimeError::Store(StoreError::Failed(_)))
        ));
        let committed = store.load_record(&process_id).unwrap();
        let encoded = crate::store::runtime_aux_to_json(&committed);
        let aux = crate::store::runtime_aux_from_json(
            &serde_json::from_slice::<serde_json::Value>(&serde_json::to_vec(&encoded).unwrap())
                .unwrap(),
        )
        .unwrap();
        let reconstructed = RuntimeRecord {
            process: serde_json::from_value(serde_json::to_value(&committed.process).unwrap())
                .unwrap(),
            provenance_log: serde_json::from_value(
                serde_json::to_value(&committed.provenance_log).unwrap(),
            )
            .unwrap(),
            creation: aux.creation,
            step_results: aux.step_results,
            artifacts: aux.artifacts,
            replay_entries: aux.replay_entries,
        };
        let mut restored = InMemoryStore::new();
        restored.restore_record(reconstructed).unwrap();
        let (mut retry, _, _, _) = dismissal_fixture();
        retry.store = Box::new(restored);
        retry.clock = Box::new(FixedClock { now_ms: u64::MAX });
        let result = retry
            .dismiss_task(&task_id, "snoozed", "reviewer", "uncertain-key")
            .unwrap();
        let after = retry.store.load_record(&process_id).unwrap();
        assert_eq!(
            crate::store::runtime_aux_to_json(&committed),
            crate::store::runtime_aux_to_json(&after)
        );
        let witness = after
            .provenance_log
            .iter()
            .find(|p| p.id == result.provenance_id)
            .unwrap();
        assert_eq!(result.occurred_at, witness.timestamp);
        assert_eq!(
            serde_json::from_value::<DismissTaskResult>(serde_json::to_value(&result).unwrap())
                .unwrap(),
            result
        );
        assert_eq!(
            after
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::TaskDismissed)
                .count(),
            1
        );
        assert!(effects.lock().unwrap().is_empty());
    }

    #[test]
    fn dismissal_corrupt_or_erased_witness_refuses_actual_record_restore() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "tamper-key")
            .unwrap();
        let record = runtime.store.load_record(&process_id).unwrap();
        let key = record.replay_entries.keys().next().unwrap().clone();
        let pos = record
            .provenance_log
            .iter()
            .position(|p| p.record_kind == ProvenanceKind::TaskDismissed)
            .unwrap();
        let mut cases = Vec::new();
        for malformed_id in [
            "not-a-record".to_string(),
            process_id.clone(),
            format!("urn:wos:provenance:{}", record.provenance_log[pos].id),
        ] {
            let mut bad = record.clone();
            bad.provenance_log[pos].id = malformed_id.clone();
            let ReplayValue::Dismissal(replay) = bad.replay_entries.get_mut(&key).unwrap() else {
                panic!("actual dismissal replay missing")
            };
            replay.result.provenance_id = malformed_id;
            cases.push(bad);
        }
        let mut bad = record.clone();
        bad.replay_entries.clear();
        cases.push(bad);
        let mut bad = record.clone();
        bad.replay_entries.remove(&key);
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log.remove(pos);
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log[pos].actor_id = Some("other".into());
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log[pos].timestamp = "2030-01-01T00:00:00Z".into();
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log[pos].data.as_mut().unwrap()["details"]["reason"] =
            serde_json::json!("changed");
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log[pos].data.as_mut().unwrap()["details"]
            .as_object_mut()
            .unwrap()
            .remove("operationToken");
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log[pos].data.as_mut().unwrap()["details"]["dismissalReplayVersion"] =
            serde_json::json!(0);
        cases.push(bad);
        let mut bad = record.clone();
        bad.provenance_log.push(bad.provenance_log[pos].clone());
        cases.push(bad);
        let mut bad = record.clone();
        bad.replay_entries.insert(
            key.clone(),
            ReplayValue::Draft(PersistDraftResult {
                artifact_id: "wrong-kind".into(),
            }),
        );
        cases.push(bad);
        for bad in cases {
            let mut restored = InMemoryStore::new();
            assert!(matches!(
                restored.restore_record(bad),
                Err(StoreError::InvalidReplayEvidence(_))
            ));
            assert!(matches!(
                restored.load_record(&process_id),
                Err(StoreError::NotFound(_))
            ));
            assert!(
                restored
                    .processes_for_case(&record.process.case_ledger_id)
                    .is_empty()
            );
        }
        assert_eq!(
            crate::store::runtime_aux_to_json(&record),
            crate::store::runtime_aux_to_json(&runtime.store.load_record(&process_id).unwrap())
        );
    }

    #[test]
    fn dismissal_update_refuses_coherent_erasure_and_replacement() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "immutable-key")
            .unwrap();
        let record = runtime.store.load_record(&process_id).unwrap();
        let key = record.replay_entries.keys().next().unwrap().clone();
        let pos = record
            .provenance_log
            .iter()
            .position(|p| p.record_kind == ProvenanceKind::TaskDismissed)
            .unwrap();
        let mut erased = record.clone();
        erased.replay_entries.clear();
        erased.provenance_log.remove(pos);
        // Each standalone shape can look fresh; the actual prior record makes erasure observable.
        erased.validate_dismissal_replay().unwrap();
        assert!(matches!(
            runtime.store.save_record(erased),
            Err(StoreError::InvalidReplayEvidence(_))
        ));
        let mut replaced = record.clone();
        replaced.provenance_log[pos].timestamp = "2030-01-01T00:00:00Z".into();
        if let Some(ReplayValue::Dismissal(value)) = replaced.replay_entries.get_mut(&key) {
            value.result.occurred_at = "2030-01-01T00:00:00Z".into();
        }
        replaced.validate_dismissal_replay().unwrap();
        assert!(matches!(
            runtime.store.save_record(replaced),
            Err(StoreError::InvalidReplayEvidence(_))
        ));
        let after = runtime.store.load_record(&process_id).unwrap();
        assert_eq!(
            crate::store::runtime_aux_to_json(&record),
            crate::store::runtime_aux_to_json(&after)
        );
        assert_eq!(
            serde_json::to_value(&record.provenance_log).unwrap(),
            serde_json::to_value(&after.provenance_log).unwrap()
        );
    }

    #[test]
    fn provenance_update_returns_actual_custody_delta_and_preserves_dismissal_replay() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let outcome = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "custody-key")
            .unwrap();
        let previous = runtime.store.load_record(&process_id).unwrap();
        assert!(
            previous
                .validate_provenance_update(&previous)
                .unwrap()
                .is_empty()
        );
        let receipt = crate::CustodyAppendReceipt::new(format!("sha256:{}", "a".repeat(64)));
        runtime
            .apply_custody_receipt(&process_id, &outcome.provenance_id, receipt.clone())
            .unwrap();
        let stamped = runtime.store.load_record(&process_id).unwrap();
        let changes = stamped.validate_provenance_update(&previous).unwrap();
        assert_eq!(
            changes,
            vec![crate::store::ProvenanceCustodyUpdate {
                record_id: outcome.provenance_id.clone(),
                receipt: receipt.clone()
            }]
        );
        assert_eq!(stamped.creation, previous.creation);
        assert_eq!(
            crate::store::runtime_aux_to_json(&stamped),
            crate::store::runtime_aux_to_json(&previous)
        );
        assert_eq!(
            runtime
                .dismiss_task(&task_id, "snoozed", "reviewer", "custody-key")
                .unwrap(),
            outcome
        );
        runtime
            .apply_custody_receipt(&process_id, &outcome.provenance_id, receipt)
            .unwrap();
        let retried = runtime.store.load_record(&process_id).unwrap();
        assert!(
            retried
                .validate_provenance_update(&stamped)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(&retried.provenance_log).unwrap(),
            serde_json::to_value(&stamped.provenance_log).unwrap()
        );
        runtime
            .dismiss_task(&task_id, "different", "reviewer", "fresh-custody-key")
            .unwrap();
        let appended = runtime.store.load_record(&process_id).unwrap();
        assert!(
            appended
                .validate_provenance_update(&retried)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            appended.provenance_log.len(),
            retried.provenance_log.len() + 1
        );
    }

    #[test]
    fn provenance_update_refuses_conflicting_erased_or_simultaneously_changed_custody() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        let outcome = runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "custody-conflict")
            .unwrap();
        let unstamped = runtime.store.load_record(&process_id).unwrap();
        runtime
            .apply_custody_receipt(
                &process_id,
                &outcome.provenance_id,
                crate::CustodyAppendReceipt::new(format!("sha256:{}", "a".repeat(64))),
            )
            .unwrap();
        let previous = runtime.store.load_record(&process_id).unwrap();
        let pos = previous
            .provenance_log
            .iter()
            .position(|p| p.id == outcome.provenance_id)
            .unwrap();
        let mut erased = previous.clone();
        erased.provenance_log[pos].canonical_event_hash = None;
        assert!(matches!(
            erased.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut changed = previous.clone();
        changed.provenance_log[pos].canonical_event_hash =
            Some(format!("sha256:{}", "b".repeat(64)));
        assert!(matches!(
            changed.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut simultaneous = previous.clone();
        simultaneous.provenance_log[pos].definition_version = Some("tampered".into());
        assert!(matches!(
            simultaneous.validate_provenance_update(&unstamped),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut witness = previous.clone();
        witness.provenance_log[pos].data.as_mut().unwrap()["details"]["reason"] =
            serde_json::json!("changed");
        assert!(matches!(
            witness.validate_provenance_update(&unstamped),
            Err(StoreError::InvalidReplayEvidence(_))
        ));
        assert!(
            runtime
                .apply_custody_receipt(
                    &process_id,
                    &outcome.provenance_id,
                    crate::CustodyAppendReceipt::new(format!("sha256:{}", "b".repeat(64)))
                )
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(
                &runtime
                    .store
                    .load_record(&process_id)
                    .unwrap()
                    .provenance_log
            )
            .unwrap(),
            serde_json::to_value(&previous.provenance_log).unwrap()
        );
        assert_eq!(
            runtime
                .dismiss_task(&task_id, "snoozed", "reviewer", "custody-conflict")
                .unwrap(),
            outcome
        );
    }

    #[test]
    fn provenance_update_refuses_truncated_reordered_replaced_or_duplicate_prefix() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "prefix-key")
            .unwrap();
        let previous = runtime.store.load_record(&process_id).unwrap();
        assert!(previous.provenance_log.len() >= 2);
        let mut truncated = previous.clone();
        truncated.provenance_log.remove(0);
        assert!(matches!(
            truncated.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut reordered = previous.clone();
        reordered.provenance_log.swap(0, 1);
        assert!(matches!(
            reordered.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut replaced = previous.clone();
        replaced.provenance_log[0].id = stack_common_typeid::mint_provenance_id();
        assert!(matches!(
            replaced.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut duplicated = previous.clone();
        duplicated
            .provenance_log
            .push(previous.provenance_log[0].clone());
        assert!(matches!(
            duplicated.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        let mut missing_id = previous.clone();
        missing_id.provenance_log[0].id = "  ".into();
        assert!(matches!(
            missing_id.validate_provenance_update(&previous),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        assert_eq!(
            serde_json::to_value(
                &runtime
                    .store
                    .load_record(&process_id)
                    .unwrap()
                    .provenance_log
            )
            .unwrap(),
            serde_json::to_value(&previous.provenance_log).unwrap()
        );
    }

    #[test]
    fn dismissal_auxiliary_codec_refuses_raw_duplicates_and_malformed_entries() {
        let (mut runtime, process_id, task_id, _) = dismissal_fixture();
        runtime
            .dismiss_task(&task_id, "snoozed", "reviewer", "codec-key")
            .unwrap();
        let record = runtime.store.load_record(&process_id).unwrap();
        let encoded = crate::store::runtime_aux_to_json(&record);
        let mut cases = Vec::new();
        let mut duplicate = encoded.clone();
        let entry = duplicate["replayEntries"][0].clone();
        duplicate["replayEntries"]
            .as_array_mut()
            .unwrap()
            .push(entry);
        cases.push(duplicate);
        for path in ["reason", "result"] {
            let mut bad = encoded.clone();
            bad["replayEntries"][0]["value"][path] = serde_json::Value::Null;
            cases.push(bad);
        }
        let mut bad = encoded.clone();
        bad["replayEntries"][0]["key"]["operation"] = serde_json::json!("persistDraft");
        cases.push(bad);
        let mut bad = encoded.clone();
        bad["replayEntries"][0]["value"]["kind"] = serde_json::json!("draft");
        cases.push(bad);
        let mut bad = encoded.clone();
        bad["replayEntries"][0]["key"]["token"] = serde_json::json!("  ");
        cases.push(bad);
        let mut bad = encoded.clone();
        bad["replayEntries"][0]["value"]["result"]["occurredAt"] = serde_json::json!("not-a-time");
        cases.push(bad);
        for malformed_id in [
            "not-a-record".to_string(),
            process_id.clone(),
            format!(
                "urn:wos:provenance:{}",
                record
                    .provenance_log
                    .iter()
                    .find(|entry| entry.record_kind == ProvenanceKind::TaskDismissed)
                    .unwrap()
                    .id
            ),
        ] {
            let mut bad = encoded.clone();
            bad["replayEntries"][0]["value"]["result"]["provenanceId"] =
                serde_json::json!(malformed_id);
            cases.push(bad);
        }
        let mut bad = encoded.clone();
        bad["replayEntries"][0]["value"]["result"]["extra"] = serde_json::json!(true);
        cases.push(bad);
        for bad in cases {
            assert!(matches!(
                crate::store::runtime_aux_from_json(&bad),
                Err(StoreError::InvalidReplayEvidence(_))
            ));
        }
        let aux = crate::store::runtime_aux_from_json(&encoded).unwrap();
        let decoded = RuntimeRecord {
            process: record.process.clone(),
            provenance_log: record.provenance_log.clone(),
            creation: aux.creation,
            step_results: aux.step_results,
            artifacts: aux.artifacts,
            replay_entries: aux.replay_entries,
        };
        decoded.validate_dismissal_replay().unwrap();
        assert_eq!(crate::store::runtime_aux_to_json(&decoded), encoded);
    }

    #[test]
    fn submit_task_response_replays_same_actor_token_after_completion() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
        let access = TaskDelegationProbe::default();
        access.allowed.store(true, Ordering::SeqCst);
        let authorization_checks = access.checks.clone();
        let delegation_allowed = access.allowed.clone();
        runtime.access_control = Box::new(access);
        runtime
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-5").to_string(),
                tenant: None,
                definition_url: "urn:test:replay-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-5"))
            .unwrap();
        let mut task =
            manual_formspec_task(test_process_id("case-5"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        task.extensions.insert(
            COMPLETION_EVENT_EXTENSION_KEY.to_string(),
            serde_json::Value::String("review.completed".to_string()),
        );
        record.process.active_tasks.push(task);
        runtime.store.save_record(record).unwrap();

        let response = serde_json::json!({
            "status": "completed",
            "definitionUrl": "urn:formspec:review",
            "definitionVersion": "1.0.0",
            "data": { "approved": true }
        });
        let first = runtime
            .submit_task_response(&task_id, response.clone(), "delegate", Some("replay-1"))
            .unwrap();
        assert_eq!(authorization_checks.load(Ordering::SeqCst), 1);
        delegation_allowed.store(false, Ordering::SeqCst);
        assert!(
            runtime
                .store
                .load_record(test_process_id("case-5"))
                .unwrap()
                .process
                .active_tasks
                .is_empty()
        );
        let second = runtime
            .submit_task_response(&task_id, response, "delegate", Some("replay-1"))
            .unwrap();

        assert_eq!(
            authorization_checks.load(Ordering::SeqCst),
            1,
            "immutable replay must not repeat engine authorization"
        );

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

        let record = runtime
            .store
            .load_record(test_process_id("case-5"))
            .unwrap();
        assert_eq!(record.process.pending_events.len(), 1);
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-6").to_string(),
                tenant: None,
                definition_url: "urn:test:actor-replay-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-6"))
            .unwrap();
        let task = manual_formspec_task(test_process_id("case-6"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
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

        assert_submission_result_roundtrip(&first);
        assert_submission_result_roundtrip(&second);
        assert_aux_submission_format(
            &runtime
                .store
                .load_record(test_process_id("case-6"))
                .unwrap(),
            &first,
            "token-shared",
        );
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

        let record = runtime
            .store
            .load_record(test_process_id("case-6"))
            .unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-7").to_string(),
                tenant: None,
                definition_url: "urn:test:timer-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        assert_eq!(instance.pending_events.len(), 0);
        assert_eq!(instance.timers.len(), 1);

        let result = runtime.drain_once(test_process_id("case-7")).unwrap();
        assert_eq!(result.processed_event.as_deref(), Some("$timeout.review"));
        assert!(
            result
                .provenance
                .iter()
                .any(|entry| entry.record_kind == ProvenanceKind::TimerFired)
        );

        let instance = runtime.load_process(test_process_id("case-7")).unwrap();
        assert!(instance.configuration.contains(&"timed_out".to_string()));
        assert!(instance.pending_events.is_empty());
        assert!(instance.timers.is_empty());
    }

    #[test]
    fn drain_once_unsupported_binding_fails_deterministically() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-8").to_string(),
                tenant: None,
                definition_url: "urn:test:unsupported-binding".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-8"),
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let error = runtime.drain_once(test_process_id("case-8")).unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::UnsupportedBinding(ref binding) if binding == "json-schema"
        ));
    }

    #[test]
    fn drain_once_save_failure_leaves_store_unchanged() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-9").to_string(),
                tenant: None,
                definition_url: "urn:test:atomic-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-9"),
                PendingEvent {
                    event: "start".to_string(),
                    actor_id: Some("reviewer".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();
        let provenance_position_before_failure = runtime
            .load_process(test_process_id("case-9"))
            .unwrap()
            .provenance_position;

        let error = runtime.drain_once(test_process_id("case-9")).unwrap_err();
        assert!(matches!(error, RuntimeError::Store(StoreError::Failed(_))));

        let instance = runtime.load_process(test_process_id("case-9")).unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-10").to_string(),
                tenant: None,
                definition_url: "urn:test:unavailable-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();

        let mut record = runtime
            .store
            .load_record(test_process_id("case-10"))
            .unwrap();
        let provenance_len_before_failure = record.provenance_log.len();
        let task =
            manual_formspec_task(test_process_id("case-10"), 1, Some("urn:mapping:response"));
        let task_id = task.task_id.clone();
        record.process.active_tasks.push(task);
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

        let record = runtime
            .store
            .load_record(test_process_id("case-10"))
            .unwrap();
        assert_eq!(record.process.active_tasks.len(), 1);
        assert_eq!(record.provenance_log.len(), provenance_len_before_failure);
    }

    #[test]
    fn drain_once_invokes_service_and_persists_step_result() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-service").to_string(),
                tenant: None,
                definition_url: "urn:test:service-kernel".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            })
            .unwrap();
        runtime
            .enqueue_event(
                test_process_id("case-service"),
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("verificationSystem".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let result = runtime.drain_once(test_process_id("case-service")).unwrap();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-integration").to_string(),
                tenant: None,
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
                test_process_id("case-integration"),
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let first_result = runtime
            .drain_once(test_process_id("case-integration"))
            .unwrap();
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

        let instance = runtime
            .load_process(test_process_id("case-integration"))
            .unwrap();
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
                test_process_id("case-integration"),
                PendingEvent {
                    event: "verify".to_string(),
                    actor_id: Some("system".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: None,
                },
            )
            .unwrap();

        let second_result = runtime
            .drain_once(test_process_id("case-integration"))
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        {
            let invocations = invocations.lock().unwrap();
            assert_eq!(invocations.len(), 1);
        }
        assert!(
            second_result
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::IdempotencyDedup)
        );
        assert!(
            !second_result
                .provenance
                .iter()
                .any(|record| record.record_kind == ProvenanceKind::StepResultPersisted)
        );
    }

    #[test]
    fn drain_once_rejects_integration_profile_target_mismatch() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-target-mismatch").to_string(),
                tenant: None,
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
                test_process_id("case-target-mismatch"),
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
            .drain_once(test_process_id("case-target-mismatch"))
            .unwrap_err();
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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-tool-integration-binding").to_string(),
                tenant: None,
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
                test_process_id("case-tool-integration-binding"),
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
            .drain_once(test_process_id("case-tool-integration-binding"))
            .expect("tool binding dispatch must succeed (NB.4)");

        // The service must have been called exactly once.
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // ToolInvoked provenance must be present.
        use wos_core::ProvenanceKind;
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
    fn drain_once_rejects_invalid_integration_idempotency_expression() {
        let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-invalid-idempotency").to_string(),
                tenant: None,
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
                test_process_id("case-invalid-idempotency"),
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
            .drain_once(test_process_id("case-invalid-idempotency"))
            .unwrap_err();
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
            "$wosWorkflow": "1.0",
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
                                "guard": "caseFile.amount >= 100",
                                "tags": ["serde-roundtrip"],
                                "actions": [{"action":"emitEvent", "eventType":"serde.followup"}]
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-guards").to_string(),
                tenant: None,
                definition_url: "urn:test:drain-guard-evals".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "amount": 250 })),
            })
            .unwrap();

        runtime
            .enqueue_event(
                test_process_id("case-guards"),
                PendingEvent {
                    event: "approve".to_string(),
                    actor_id: Some("approver".to_string()),
                    data: None,
                    timestamp: String::new(),
                    idempotency_token: Some("serde-guard-token".into()),
                },
            )
            .unwrap();

        let result = runtime.drain_once(test_process_id("case-guards")).unwrap();

        assert_eq!(
            result.processed_event_token.as_deref(),
            Some("serde-guard-token")
        );
        assert_eq!(result.transitions[0].tags, vec!["serde-roundtrip"]);
        assert!(!result.provenance.is_empty());
        assert_eq!(result.emitted_events, vec!["serde.followup"]);
        assert_drain_result_roundtrip(&result);

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
            "$wosWorkflow": "1.0",
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
            .create_process(CreateProcessRequest {
                process_id: test_process_id("case-scope").to_string(),
                tenant: None,
                definition_url: "urn:test:guard-scope".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: Some(serde_json::json!({ "ok": true })),
            })
            .unwrap();
        for name in ["go", "next"] {
            runtime
                .enqueue_event(
                    test_process_id("case-scope"),
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

        let first = runtime.drain_once(test_process_id("case-scope")).unwrap();
        let second = runtime.drain_once(test_process_id("case-scope")).unwrap();

        assert_eq!(first.guard_evaluations.len(), 1);
        assert_eq!(first.guard_evaluations[0].event, "go");
        assert_eq!(second.guard_evaluations.len(), 1);
        assert_eq!(
            second.guard_evaluations[0].event, "next",
            "second drain only sees its own guard"
        );
    }

    // ── ADR 0064 / Sub-PR C-2: AgentInvoker DI wiring ───────────────────────
    //
    // Confirms `WosRuntime` accepts an `AgentInvokerRegistry` at construction
    // (or via builder) and exposes it for the runtime's eventual transition-
    // firing dispatch path. Transition-level integration with agent-typed
    // actors lands in Sub-PR D-3 alongside foreach iteration semantics.

    use wos_core::{
        AgentContext as CoreAgentContext, AgentInvocationError, AgentInvoker, AgentInvokerRegistry,
        AgentResult, AgentTask, InvokerKind, InvokerSpec,
    };

    /// Inline test invoker that records what it was asked to do. Independent
    /// of `wos-agent-stub` so this crate's tests don't pull in a dev-dep
    /// purely for a one-shot dispatch check.
    struct CountingInvoker {
        label: &'static str,
    }

    impl AgentInvoker for CountingInvoker {
        fn invoke(
            &self,
            decl: &wos_core::model::ai::AgentDeclaration,
            task: &AgentTask,
            _ctx: &CoreAgentContext<'_>,
        ) -> Result<AgentResult, AgentInvocationError> {
            Ok(AgentResult {
                output: serde_json::json!({
                    "handledBy": self.label,
                    "agent": decl.id,
                    "capability": task.capability_id,
                }),
                confidence: 1.0,
                citations: vec![],
                telemetry: None,
            })
        }
    }

    fn empty_kernel() -> KernelDocument {
        serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0",
            "url": "urn:test:c2-runtime",
            "version": "1.0.0",
            "lifecycle": {
                "initialState": "start",
                "states": {
                    "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                    "done": {"type": "final"}
                }
            }
        }))
        .expect("test kernel parses")
    }

    #[test]
    fn runtime_default_agent_invokers_is_empty() {
        let runtime = runtime_with_kernel(empty_kernel());
        assert!(
            runtime.agent_invokers().is_empty(),
            "freshly constructed runtime MUST start with an empty AgentInvokerRegistry; \
             agent participation is opt-in per deployment (ADR 0064)"
        );
    }

    #[test]
    fn runtime_with_agent_invoker_registers_single_kind() {
        let runtime = runtime_with_kernel(empty_kernel()).with_agent_invoker(
            InvokerKind::Stub,
            Box::new(CountingInvoker { label: "stub" }),
        );
        assert!(!runtime.agent_invokers().is_empty());

        let stub_spec = InvokerSpec::Stub { responses: vec![] };
        assert!(
            runtime.agent_invokers().lookup(&stub_spec).is_some(),
            "stub adapter MUST be discoverable through the registry by its InvokerSpec discriminator"
        );

        // No other kinds registered.
        let anth_spec = InvokerSpec::Anthropic {
            model: "claude-opus-4-7".into(),
            config_ref: None,
        };
        assert!(runtime.agent_invokers().lookup(&anth_spec).is_none());
    }

    #[test]
    fn runtime_with_agent_invokers_replaces_registry_wholesale() {
        let registry = AgentInvokerRegistry::new()
            .with(
                InvokerKind::Stub,
                Box::new(CountingInvoker { label: "stub" }),
            )
            .with(
                InvokerKind::Anthropic,
                Box::new(CountingInvoker { label: "anthropic" }),
            );
        let runtime = runtime_with_kernel(empty_kernel()).with_agent_invokers(registry);

        let mut kinds: Vec<&'static str> = runtime
            .agent_invokers()
            .registered_kinds()
            .map(InvokerKind::as_str)
            .collect();
        kinds.sort();
        assert_eq!(kinds, vec!["anthropic", "stub"]);
    }

    #[test]
    fn runtime_routes_invocation_to_registered_adapter_via_lookup() {
        let runtime = runtime_with_kernel(empty_kernel())
            .with_agent_invoker(
                InvokerKind::Stub,
                Box::new(CountingInvoker { label: "stub" }),
            )
            .with_agent_invoker(
                InvokerKind::Anthropic,
                Box::new(CountingInvoker { label: "anthropic" }),
            );

        // Build an agent declaration with an Anthropic invoker spec.
        use wos_core::model::ai::{AgentDeclaration, AgentType};
        let decl = AgentDeclaration {
            id: "extractor".into(),
            kind: "agent".into(),
            agent_type: AgentType::Generative,
            model_identifier: "claude-opus-4-7".into(),
            model_version: "4.7".into(),
            description: None,
            capabilities: vec![],
            model_version_policy: None,
            confidence_decay: None,
            fallback_chain: vec![],
            cascading_invocations: vec![],
            deontic_constraints: None,
            invoker: Some(InvokerSpec::Anthropic {
                model: "claude-opus-4-7".into(),
                config_ref: None,
            }),
            extensions: Default::default(),
        };

        let invoker = runtime
            .agent_invokers()
            .lookup(decl.invoker.as_ref().expect("invoker declared"))
            .expect("anthropic adapter registered");

        let task = AgentTask {
            capability_id: "extract".into(),
            prompt: None,
            inputs: Default::default(),
            correlation_key: None,
        };
        let case = serde_json::json!({});
        let ctx = CoreAgentContext {
            process_id: test_process_id("case-scope"),
            invocation_index: 0,
            case_state: &case,
        };

        let result = invoker.invoke(&decl, &task, &ctx).unwrap();
        assert_eq!(
            result.output["handledBy"], "anthropic",
            "InvokerSpec::Anthropic MUST route to the anthropic-bound adapter, not the stub"
        );
        assert_eq!(result.output["agent"], "extractor");
    }

    fn bound_actor_kernel() -> KernelDocument {
        serde_json::from_value(serde_json::json!({
            "$wosWorkflow":"1.0", "url":"urn:test:bound-actors", "version":"1.0.0",
            "actors":[{"id":"reviewer","type":"human"},{"id":"worker","type":"system"}],
            "lifecycle":{"initialState":"initial","states":{
                "initial":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"first","assignTo":"reviewer"}],
                    "transitions":[{"event":"advance","target":"later"}]},
                "later":{"type":"atomic","onEntry":[
                    {"action":"createTask","taskRef":"second","assignTo":"reviewer"},
                    {"action":"createTask","taskRef":"machine","assignTo":"worker"},
                    {"action":"createTask","taskRef":"unassigned"}
                ]}
            }}
        })).unwrap()
    }

    fn bound_actor_request() -> CreateProcessRequest {
        CreateProcessRequest {
            process_id: stack_common_typeid::mint_process_id(),
            tenant: None,
            definition_url: "urn:test:bound-actors".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        }
    }

    fn human_binding(actor: &str) -> Vec<crate::HumanActorBinding> {
        vec![crate::HumanActorBinding {
            declared_actor: "reviewer".into(),
            assigned_actor: actor.into(),
        }]
    }

    #[test]
    fn bound_actor_creation_and_later_tasks_retain_local_slots_and_actual_humans() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel());
        let first = DurableRuntime::create_process_with_actor_bindings(
            &mut runtime,
            bound_actor_request(),
            human_binding("actor:human:original"),
        )
        .unwrap();
        assert_eq!(
            first.active_tasks[0].declared_actor.as_deref(),
            Some("reviewer")
        );
        assert_eq!(
            first.active_tasks[0].assigned_actor.as_deref(),
            Some("actor:human:original")
        );
        runtime
            .enqueue_event(
                &first.process_id,
                PendingEvent {
                    event: "advance".into(),
                    actor_id: Some("actor:human:original".into()),
                    data: None,
                    timestamp: first.created_at.clone(),
                    idempotency_token: None,
                },
            )
            .unwrap();
        runtime.drain_once(&first.process_id).unwrap();
        let record = runtime.store.load_record(&first.process_id).unwrap();
        assert_eq!(record.process.active_tasks.len(), 4);
        let later = record
            .process
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "second")
            .unwrap();
        assert_eq!(later.declared_actor.as_deref(), Some("reviewer"));
        assert_eq!(
            later.assigned_actor.as_deref(),
            Some("actor:human:original")
        );
        let machine = record
            .process
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "machine")
            .unwrap();
        assert_eq!(machine.declared_actor.as_deref(), Some("worker"));
        assert_eq!(machine.assigned_actor.as_deref(), Some("worker"));
        let unassigned = record
            .process
            .active_tasks
            .iter()
            .find(|t| t.task_ref == "unassigned")
            .unwrap();
        assert_eq!(unassigned.declared_actor, None);
        assert_eq!(unassigned.assigned_actor, None);
        assert_eq!(record.creation.as_ref().unwrap().tasks.len(), 1);
        assert_eq!(
            record
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == ProvenanceKind::TaskCreated)
                .count(),
            4
        );
        record.validate_creation().unwrap();
    }

    #[test]
    fn bound_actor_resolution_refuses_missing_foreign_duplicate_and_nonhuman_bindings() {
        let candidates = vec![
            vec![],
            human_binding("actor:human:"),
            human_binding("actor:human:contains space"),
            human_binding("actor:workload:original"),
            vec![crate::HumanActorBinding {
                declared_actor: "unknown".into(),
                assigned_actor: "actor:human:original".into(),
            }],
            vec![crate::HumanActorBinding {
                declared_actor: "worker".into(),
                assigned_actor: "actor:human:original".into(),
            }],
            vec![
                human_binding("actor:human:original")[0].clone(),
                human_binding("actor:human:other")[0].clone(),
            ],
        ];
        for bindings in candidates {
            let mut runtime = runtime_with_kernel(bound_actor_kernel());
            let request = bound_actor_request();
            let id = request.process_id.clone();
            assert!(matches!(
                runtime.create_process_with_actor_bindings(request, bindings),
                Err(RuntimeError::ActorBinding(_))
            ));
            assert!(matches!(
                runtime.store.load_record(&id),
                Err(StoreError::NotFound(_))
            ));
        }
        let mut ambiguous = bound_actor_kernel();
        ambiguous.actors.push(ambiguous.actors[0].clone());
        assert!(matches!(
            runtime_with_kernel(ambiguous).create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:original")
            ),
            Err(RuntimeError::ActorBinding(_))
        ));
        assert!(serde_json::from_str::<crate::HumanActorBinding>(r#"{"declaredActor":"reviewer","assignedActor":"actor:human:original","role":"reviewer"}"#).is_err());
        assert!(serde_json::from_str::<crate::HumanActorBinding>(r#"{"declaredActor":"reviewer","declaredActor":"other","assignedActor":"actor:human:original"}"#).is_err());
    }

    #[test]
    fn bound_actor_processes_with_same_local_slot_keep_distinct_original_humans() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel());
        let first = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:first"),
            )
            .unwrap();
        let second = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:second"),
            )
            .unwrap();
        assert_ne!(first.process_id, second.process_id);
        assert_ne!(
            first.active_tasks[0].task_id,
            second.active_tasks[0].task_id
        );
        assert_eq!(
            runtime
                .load_process(&first.process_id)
                .unwrap()
                .active_tasks[0]
                .assigned_actor
                .as_deref(),
            Some("actor:human:first")
        );
        assert_eq!(
            runtime
                .load_process(&second.process_id)
                .unwrap()
                .active_tasks[0]
                .assigned_actor
                .as_deref(),
            Some("actor:human:second")
        );
    }

    #[test]
    fn bound_actor_dismissal_keeps_actual_human_type_and_replay_identity() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel());
        runtime.access_control = Box::new(TaskDelegationProbe::default());
        let process = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:original"),
            )
            .unwrap();
        let task = &process.active_tasks[0].task_id;
        assert!(matches!(
            runtime.dismiss_task(task, "later", "reviewer", "wrong-slot"),
            Err(RuntimeError::Unauthorized(_))
        ));
        assert!(matches!(
            runtime.dismiss_task(task, "later", "actor:human:other", "wrong-human"),
            Err(RuntimeError::Unauthorized(_))
        ));
        let result = runtime
            .dismiss_task(task, "later", "actor:human:original", "original-key")
            .unwrap();
        let mut record = runtime.store.load_record(&process.process_id).unwrap();
        let dismissal = record
            .provenance_log
            .iter()
            .find(|p| p.id == result.provenance_id)
            .unwrap();
        assert_eq!(dismissal.actor_id.as_deref(), Some("actor:human:original"));
        assert_eq!(dismissal.actor_type.as_deref(), Some("human"));
        assert!(
            record
                .replay_entries
                .keys()
                .all(|key| key.actor_id == "actor:human:original")
        );
        record.process.active_tasks.clear();
        runtime.store.save_record(record).unwrap();
        let replay = runtime
            .dismiss_task(task, "later", "actor:human:original", "original-key")
            .unwrap();
        assert_eq!(result, replay);
    }

    #[test]
    fn bound_actor_store_refuses_attribution_drift_and_retains_completed_witnesses() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel());
        let process = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:original"),
            )
            .unwrap();
        runtime
            .enqueue_event(
                &process.process_id,
                PendingEvent {
                    event: "advance".into(),
                    actor_id: None,
                    data: None,
                    timestamp: process.created_at.clone(),
                    idempotency_token: None,
                },
            )
            .unwrap();
        runtime.drain_once(&process.process_id).unwrap();
        let original = runtime.store.load_record(&process.process_id).unwrap();
        for id in [
            "malformed".to_string(),
            stack_common_typeid::mint_process_id(),
        ] {
            let mut malformed = original.clone();
            malformed
                .provenance_log
                .iter_mut()
                .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
                .unwrap()
                .id = id;
            assert!(malformed.validate_creation().is_err());
        }
        let mut malformed = original.clone();
        malformed
            .provenance_log
            .iter_mut()
            .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
            .unwrap()
            .timestamp = "malformed".into();
        assert!(malformed.validate_creation().is_err());
        let mut malformed = original.clone();
        malformed
            .provenance_log
            .iter_mut()
            .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
            .unwrap()
            .data
            .as_mut()
            .unwrap()["details"]
            .as_object_mut()
            .unwrap()
            .remove("binding");
        assert!(malformed.validate_creation().is_err());
        let mut changed = original.clone();
        changed.process.actor_bindings = Some(human_binding("actor:human:other"));
        assert!(changed.validate_creation().is_err());
        let mut changed = original.clone();
        changed.process.active_tasks[1].declared_actor = None;
        assert!(changed.validate_creation().is_err());
        let mut changed = original.clone();
        changed
            .provenance_log
            .retain(|p| p.record_kind != ProvenanceKind::TaskCreated);
        assert!(changed.validate_creation().is_err());
        let mut changed = original.clone();
        let witness = changed
            .provenance_log
            .iter()
            .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
            .unwrap()
            .clone();
        changed.provenance_log.push(witness);
        assert!(changed.validate_creation().is_err());
        let mut reassigned = original.clone();
        reassigned.process.active_tasks[1].assigned_actor = Some("actor:human:new-assignee".into());
        runtime.store.save_record(reassigned).unwrap();
        let mut completed = runtime.store.load_record(&process.process_id).unwrap();
        completed.process.active_tasks.clear();
        runtime.store.save_record(completed.clone()).unwrap();
        let aux = crate::runtime_aux_from_json(&crate::runtime_aux_to_json(&completed)).unwrap();
        assert_eq!(aux.creation, completed.creation);
        let mut changed = completed.clone();
        let later = changed
            .provenance_log
            .iter_mut()
            .find(|p| {
                p.record_kind == ProvenanceKind::TaskCreated
                    && p.data.as_ref().unwrap()["details"]["taskRef"] == "second"
            })
            .unwrap();
        later.data.as_mut().unwrap()["details"]["declaredActor"] = serde_json::json!("worker");
        later.data.as_mut().unwrap()["details"]["assignedActor"] = serde_json::json!("worker");
        assert!(changed.validate_creation_update(&completed).is_err());
        let mut changed = completed.clone();
        let initial_id = &completed.creation.as_ref().unwrap().tasks[0].task_id;
        changed.provenance_log.retain(|p| {
            p.record_kind != ProvenanceKind::TaskCreated
                || p.data.as_ref().unwrap()["taskId"] == *initial_id
        });
        assert!(changed.validate_creation_update(&completed).is_err());
        let mut stamped = completed.clone();
        let created = stamped
            .provenance_log
            .iter_mut()
            .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
            .unwrap();
        crate::runtime::stamp_custody_receipt(
            created,
            &wos_events::custody::CustodyAppendReceipt::new("a".repeat(64)),
        )
        .unwrap();
        stamped.validate_creation_update(&completed).unwrap();
    }

    #[test]
    fn bound_actor_intake_replay_refuses_changed_binding_and_preserves_original_creation() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        let mut request = public_intake_request(
            "ih-bound-actor",
            test_case_ledger_urn("bound-intake"),
            "urn:test:bound-actors",
            "1.0.0",
        );
        request.actor_bindings = Some(human_binding("actor:human:original"));
        let accepted = runtime
            .accept_intake_handoff("formspec", request.clone())
            .unwrap();
        let original = accepted.creation.unwrap();
        assert_eq!(original.actor_bindings, request.actor_bindings);
        assert_eq!(
            original.tasks[0].declared_actor.as_deref(),
            Some("reviewer")
        );
        assert_eq!(
            original.tasks[0].assigned_actor.as_deref(),
            Some("actor:human:original")
        );
        let replay = runtime
            .accept_intake_handoff("formspec", request.clone())
            .unwrap();
        assert_eq!(replay.creation, Some(original.clone()));
        request.actor_bindings = Some(human_binding("actor:human:other"));
        assert!(matches!(
            runtime.accept_intake_handoff("formspec", request),
            Err(RuntimeError::IntakeConflict(_))
        ));
        assert_eq!(
            runtime
                .store
                .load_record(&original.process_id)
                .unwrap()
                .creation,
            Some(original)
        );
    }

    #[test]
    fn bound_actor_migration_refuses_definition_change_and_legacy_assignment_stays_supported() {
        let mut runtime = runtime_with_kernel(bound_actor_kernel());
        let process = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:original"),
            )
            .unwrap();
        assert!(matches!(
            runtime.migrate(&process.process_id, "2.0.0", MigrationMap::default(), None),
            Err(RuntimeError::MigrationRejected(_))
        ));
        let legacy = runtime.create_process(bound_actor_request()).unwrap();
        assert_eq!(legacy.actor_bindings, None);
        assert_eq!(
            legacy.active_tasks[0].assigned_actor.as_deref(),
            Some("reviewer")
        );
        assert_eq!(legacy.active_tasks[0].declared_actor, None);
        let mut original = runtime.store.load_record(&process.process_id).unwrap();
        original.process.definition_version = "2.0.0".into();
        assert!(original.validate_creation().is_err());
    }

    #[test]
    fn bound_actor_intake_types_human_before_system_fallback_and_refuses_conflicting_type() {
        fn request(id: &str) -> IntakeAcceptanceRequest {
            let mut request = public_intake_request(
                id,
                test_case_ledger_urn(id),
                "urn:test:bound-actors",
                "1.0.0",
            );
            request.actor_id = Some("actor:human:original".into());
            request.actor_bindings = Some(human_binding("actor:human:original"));
            request
        }
        let mut runtime = runtime_with_kernel(bound_actor_kernel())
            .with_intake_policy(AutoCreatePublicIntakePolicy);
        let accepted = runtime
            .accept_intake_handoff("formspec", request("ih-bound-human-accepted"))
            .unwrap();
        for provenance in accepted.provenance.iter().filter(|p| {
            matches!(
                p.record_kind,
                ProvenanceKind::IntakeAccepted | ProvenanceKind::CaseCreated
            )
        }) {
            assert_eq!(provenance.actor_id.as_deref(), Some("actor:human:original"));
            assert_eq!(provenance.actor_type.as_deref(), Some("human"));
        }
        assert!(
            accepted
                .provenance
                .iter()
                .any(|p| p.record_kind == ProvenanceKind::IntakeAccepted)
        );
        let mut rejected = runtime_with_kernel(bound_actor_kernel())
            .with_intake_policy(PublicIntakeDisabledPolicy);
        let rejection = rejected
            .accept_intake_handoff("formspec", request("ih-bound-human-rejected"))
            .unwrap();
        let provenance = rejection
            .provenance
            .iter()
            .find(|p| p.record_kind == ProvenanceKind::IntakeRejected)
            .unwrap();
        assert_eq!(provenance.actor_type.as_deref(), Some("human"));
        let mut deferred =
            runtime_with_kernel(bound_actor_kernel()).with_intake_policy(ManualReviewIntakePolicy);
        let decision = deferred
            .accept_intake_handoff("formspec", request("ih-bound-human-deferred"))
            .unwrap();
        assert_eq!(
            decision
                .provenance
                .iter()
                .find(|p| p.record_kind == ProvenanceKind::IntakeDeferred)
                .unwrap()
                .actor_type
                .as_deref(),
            Some("human")
        );
        let process = runtime
            .load_process(&accepted.creation.unwrap().process_id)
            .unwrap();
        let mut conflicting = vec![ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskPresented,
            "test",
            Some("actor:human:original"),
            None,
        )];
        conflicting[0].actor_type = Some("system".into());
        assert!(matches!(
            super::provenance::populate_process_provenance_fields(
                &mut conflicting,
                &bound_actor_kernel(),
                &process
            ),
            Err(RuntimeError::ActorBinding(_))
        ));
        assert_eq!(conflicting[0].actor_type.as_deref(), Some("system"));
    }

    #[test]
    fn bound_actor_refuses_foreign_resolved_definition_at_initial_and_later_creation() {
        for foreign_version in [false, true] {
            let mut foreign = bound_actor_kernel();
            if foreign_version {
                foreign.version = Some("2.0.0".into());
            } else {
                foreign.url = Some("urn:test:foreign-workflow".into());
            }
            fn resolver(kernel: KernelDocument) -> TestResolver {
                TestResolver {
                    kernels: HashMap::from([(
                        ("urn:test:bound-actors".into(), "1.0.0".into()),
                        kernel,
                    )]),
                }
            }
            let mut initial = runtime_with_kernel(bound_actor_kernel());
            initial.resolver = Box::new(resolver(foreign.clone()));
            let request = bound_actor_request();
            let id = request.process_id.clone();
            assert!(matches!(
                initial.create_process_with_actor_bindings(
                    request,
                    human_binding("actor:human:original")
                ),
                Err(RuntimeError::ActorBinding(_))
            ));
            assert!(matches!(
                initial.store.load_record(&id),
                Err(StoreError::NotFound(_))
            ));
            let mut later = runtime_with_kernel(bound_actor_kernel());
            let process = later
                .create_process_with_actor_bindings(
                    bound_actor_request(),
                    human_binding("actor:human:original"),
                )
                .unwrap();
            later
                .enqueue_event(
                    &process.process_id,
                    PendingEvent {
                        event: "advance".into(),
                        actor_id: Some("actor:human:original".into()),
                        data: None,
                        timestamp: process.created_at.clone(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
            let before = later.store.load_record(&process.process_id).unwrap();
            later.resolver = Box::new(resolver(foreign));
            assert!(matches!(
                later.drain_once(&process.process_id),
                Err(RuntimeError::ActorBinding(_))
            ));
            let after = later.store.load_record(&process.process_id).unwrap();
            assert_eq!(
                serde_json::to_value(&after.process).unwrap(),
                serde_json::to_value(&before.process).unwrap()
            );
            assert_eq!(
                crate::runtime_aux_to_json(&after),
                crate::runtime_aux_to_json(&before)
            );
        }
    }

    fn bound_formspec_kernel() -> KernelDocument {
        let mut value = serde_json::to_value(bound_actor_kernel()).unwrap();
        value["contracts"] =
            serde_json::json!({"reviewForm":{"binding":"formspec","ref":"urn:formspec:review"}});
        value["lifecycle"]["states"]["initial"]["onEntry"][0]["contractRef"] =
            serde_json::json!("reviewForm");
        value["lifecycle"]["states"]["initial"]["onEntry"][0]["extensions"] = serde_json::json!({"x-wos-signature-profile-key":"bound-profile", "x-wos-signature-step-id":"applicantSigns"});
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn bound_actor_formspec_persist_present_and_authorized_reassignment_keep_original_attribution()
    {
        #[derive(Debug)]
        struct BoundPresenter {
            store: Arc<Mutex<InMemoryStore>>,
            calls: Arc<AtomicUsize>,
        }
        impl TaskPresenter for BoundPresenter {
            type Error = PresenterError;
            fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), Self::Error> {
                let store = self.store.lock().unwrap();
                let record = store.load_record(&context.process_id).unwrap();
                let task = record
                    .process
                    .active_tasks
                    .iter()
                    .find(|t| t.task_id == context.task_id)
                    .unwrap();
                assert_eq!(task.status, ActiveTaskStatus::Assigned);
                assert_eq!(task.assigned_actor.as_deref(), Some("actor:human:original"));
                assert_eq!(task.declared_actor.as_deref(), Some("reviewer"));
                assert_eq!(context.assigned_actor, "actor:human:original");
                record.validate_creation().unwrap();
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn dismiss_task(&mut self, _task: &str, _reason: &str) -> Result<(), Self::Error> {
                Ok(())
            }
        }
        let store = Arc::new(Mutex::new(InMemoryStore::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = runtime_with_store(SharedStore(store.clone()), bound_formspec_kernel());
        runtime.presenter = Box::new(BoundPresenter {
            store: store.clone(),
            calls: calls.clone(),
        });
        runtime.access_control = Box::new(TaskDelegationProbe::default());
        let profile_file: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/profiles/signature-runtime-sequential.json"
        ))
        .unwrap();
        let mut profile = profile_file["signature"].clone();
        profile["roles"][0]["actorId"] = serde_json::json!("reviewer");
        profile["reassignmentPolicy"]["authorizedActorIds"] =
            serde_json::json!(["actor:human:original"]);
        runtime = runtime
            .with_signature_profile("bound-profile", serde_json::from_value(profile).unwrap());
        let process = runtime
            .create_process_with_actor_bindings(
                bound_actor_request(),
                human_binding("actor:human:original"),
            )
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let original = runtime.store.load_record(&process.process_id).unwrap();
        let original_creation = original.creation.clone();
        let task = &process.active_tasks[0].task_id;
        let result = runtime.submit_task_response(task, serde_json::json!({"status":"reassigned","data":{"reason":"qualified reassignment","newActorId":"actor:human:successor"}}), "actor:human:original", Some("reassignment-key")).unwrap();
        assert!(matches!(result, TaskSubmissionResult::Completed { .. }));
        let current = runtime.store.load_record(&process.process_id).unwrap();
        assert_eq!(current.creation, original_creation);
        assert_eq!(
            current.process.actor_bindings,
            original.process.actor_bindings
        );
        assert_eq!(
            current.process.active_tasks[0].declared_actor.as_deref(),
            Some("reviewer")
        );
        assert_eq!(
            current.process.active_tasks[0].assigned_actor.as_deref(),
            Some("actor:human:successor")
        );
        assert_eq!(
            current.process.active_tasks[0]
                .context
                .as_ref()
                .unwrap()
                .assigned_actor,
            "actor:human:successor"
        );
        current.validate_creation().unwrap();
        assert!(matches!(
            runtime.dismiss_task(task, "later", "actor:human:original", "old-assignee"),
            Err(RuntimeError::Unauthorized(_))
        ));
    }

    #[test]
    fn bound_actor_interrupted_intake_resumes_original_allocation_after_task_removal() {
        for fail_on_save in [1, 2] {
            let mut runtime = runtime_with_store(
                IntakeSaveFailingStore::new(fail_on_save),
                bound_actor_kernel(),
            )
            .with_intake_policy(AutoCreatePublicIntakePolicy);
            let mut request = public_intake_request(
                &format!("ih-bound-retry-{fail_on_save}"),
                test_case_ledger_urn(&format!("bound-retry-{fail_on_save}")),
                "urn:test:bound-actors",
                "1.0.0",
            );
            request.actor_bindings = Some(human_binding("actor:human:original"));
            assert!(matches!(
                runtime.accept_intake_handoff("formspec", request.clone()),
                Err(RuntimeError::Store(StoreError::Failed(_)))
            ));
            let mut original = runtime
                .load_single_record_by_case_ledger_id(
                    WorkflowProcess::extract_urn_type_id(
                        request.governed_case_ref.as_deref().unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap();
            let creation = original.creation.clone().unwrap();
            let mut changed = request.clone();
            changed.actor_bindings = Some(human_binding("actor:human:other"));
            assert!(matches!(
                runtime.accept_intake_handoff("formspec", changed),
                Err(RuntimeError::IntakeConflict(_))
            ));
            original.process.active_tasks.clear();
            runtime.store.save_record(original).unwrap();
            let resumed = runtime
                .accept_intake_handoff("formspec", request.clone())
                .unwrap();
            assert_eq!(resumed.creation, Some(creation.clone()));
            assert!(
                runtime
                    .load_process(&creation.process_id)
                    .unwrap()
                    .active_tasks
                    .is_empty()
            );
            let replay = runtime.accept_intake_handoff("formspec", request).unwrap();
            assert_eq!(replay.creation, Some(creation.clone()));
            let record = runtime.store.load_record(&creation.process_id).unwrap();
            assert_eq!(
                record
                    .provenance_log
                    .iter()
                    .filter(|p| p.record_kind == ProvenanceKind::TaskCreated)
                    .count(),
                1
            );
            assert_eq!(
                record
                    .provenance_log
                    .iter()
                    .filter(|p| p.record_kind == ProvenanceKind::IntakeAccepted)
                    .count(),
                1
            );
            record.validate_creation().unwrap();
        }
    }

    #[test]
    fn bound_actor_applied_intake_replays_without_resolver_and_refuses_changed_request() {
        #[derive(Debug)]
        struct UnavailableResolver {
            calls: Arc<AtomicUsize>,
        }
        impl DocumentResolver for UnavailableResolver {
            type Error = TestResolverError;
            fn resolve_kernel(
                &self,
                _url: &str,
                _version: &str,
            ) -> Result<KernelDocument, Self::Error> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Err(TestResolverError("unavailable".into()))
            }
            fn resolve_governance(
                &self,
                _url: &str,
                _version: &str,
            ) -> Result<wos_core::GovernanceDocument, Self::Error> {
                Err(TestResolverError("unused".into()))
            }
            fn resolve_sidecar(
                &self,
                _url: &str,
                _anchor_date: Option<&str>,
            ) -> Result<serde_json::Value, Self::Error> {
                Err(TestResolverError("unused".into()))
            }
        }
        for outcome in ["accepted", "rejected", "deferred"] {
            let runtime = runtime_with_kernel(bound_actor_kernel());
            let mut runtime = match outcome {
                "accepted" => runtime.with_intake_policy(AutoCreatePublicIntakePolicy),
                "rejected" => runtime.with_intake_policy(PublicIntakeDisabledPolicy),
                _ => runtime.with_intake_policy(ManualReviewIntakePolicy),
            };
            let label = format!("ih-bound-outage-{outcome}");
            let mut request = public_intake_request(
                &label,
                test_case_ledger_urn(&label),
                "urn:test:bound-actors",
                "1.0.0",
            );
            request.actor_id = Some("actor:human:original".into());
            request.actor_bindings = Some(human_binding("actor:human:original"));
            let original = runtime
                .accept_intake_handoff("formspec", request.clone())
                .unwrap();
            let original_json = serde_json::to_value(&original).unwrap();
            let original_receipt = serde_json::to_value(
                runtime
                    .store
                    .load_intake_record("formspec", &label)
                    .unwrap(),
            )
            .unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            runtime.resolver = Box::new(UnavailableResolver {
                calls: calls.clone(),
            });
            let replay = runtime
                .accept_intake_handoff("formspec", request.clone())
                .unwrap();
            assert_eq!(serde_json::to_value(replay).unwrap(), original_json);
            for change in ["binding", "body", "url", "version"] {
                let mut changed = request.clone();
                match change {
                    "binding" => changed.actor_bindings = Some(human_binding("actor:human:other")),
                    "body" => changed.document["responseHash"] = serde_json::json!("changed-body"),
                    "url" => {
                        changed
                            .governed_case_definition
                            .as_mut()
                            .unwrap()
                            .definition_url = "urn:test:other".into()
                    }
                    _ => {
                        changed
                            .governed_case_definition
                            .as_mut()
                            .unwrap()
                            .definition_version = "2.0.0".into()
                    }
                }
                assert!(
                    matches!(
                        runtime.accept_intake_handoff("formspec", changed),
                        Err(RuntimeError::IntakeConflict(_))
                    ),
                    "{outcome}: {change}"
                );
            }
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                serde_json::to_value(
                    runtime
                        .store
                        .load_intake_record("formspec", &label)
                        .unwrap()
                )
                .unwrap(),
                original_receipt
            );
            let fresh_label = format!("ih-bound-fresh-outage-{outcome}");
            request.document["handoffId"] = serde_json::json!(fresh_label);
            request.governed_case_ref = Some(test_case_ledger_urn(&fresh_label).into());
            assert!(matches!(
                runtime.accept_intake_handoff("formspec", request),
                Err(RuntimeError::Resolver(_))
            ));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(matches!(
                runtime.store.load_intake_record("formspec", &fresh_label),
                Err(StoreError::NotFound(_))
            ));
        }
    }

    #[test]
    fn bound_actor_foreign_drain_definition_refuses_before_policy_and_service_effects() {
        #[derive(Debug)]
        struct RecordingCompanion {
            calls: Arc<AtomicUsize>,
        }
        impl CompanionPolicy for RecordingCompanion {
            fn evaluate_event(
                &mut self,
                context: RuntimeEventContext,
            ) -> Result<RuntimeEventDecision, RuntimeError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeEventDecision::proceed(context.event))
            }
        }
        for identity in ["matching", "foreign-url", "foreign-version"] {
            for service_only in [false, true] {
                for integration in [false, true] {
                    let mut foreign = serde_json::to_value(bound_actor_kernel()).unwrap();
                    if identity == "foreign-version" {
                        foreign["version"] = serde_json::json!("2.0.0");
                    } else if identity == "foreign-url" {
                        foreign["url"] = serde_json::json!("urn:test:foreign-workflow");
                    }
                    let mut actions =
                        vec![serde_json::json!({"action":"invokeService","serviceRef":"effect"})];
                    if !service_only {
                        actions.push(serde_json::json!({"action":"createTask","taskRef":"second","assignTo":"reviewer"}));
                    }
                    foreign["lifecycle"]["states"]["later"]["onEntry"] = serde_json::json!(actions);
                    let foreign: KernelDocument = serde_json::from_value(foreign).unwrap();
                    let mut runtime = runtime_with_kernel(bound_actor_kernel());
                    let service = RecordingService::with_response(serde_json::json!({"ok":true}));
                    let service_calls = service.calls.clone();
                    runtime.service = Box::new(service);
                    let policy_calls = Arc::new(AtomicUsize::new(0));
                    runtime = runtime.with_companion_policy(RecordingCompanion {
                        calls: policy_calls.clone(),
                    });
                    let process = runtime
                        .create_process_with_actor_bindings(
                            bound_actor_request(),
                            human_binding("actor:human:original"),
                        )
                        .unwrap();
                    if integration {
                        runtime = runtime.with_integration_profile(serde_json::from_value(serde_json::json!({
                            "targetWorkflow":{"url":"urn:test:bound-actors","compatibleVersions":">=1.0.0 <3.0.0"},
                            "bindings":{"effect":{"type":"request-response","interface":{"$ref":"urn:openapi:effect"},
                                "operation":"effect","requestContract":{"definitionRef":"urn:contracts:effect-request"},
                                "responseContract":{"definitionRef":"urn:contracts:effect-response"}}}
                        })).unwrap());
                    }
                    runtime
                        .enqueue_event(
                            &process.process_id,
                            PendingEvent {
                                event: "advance".into(),
                                actor_id: Some("actor:human:original".into()),
                                data: None,
                                timestamp: process.created_at.clone(),
                                idempotency_token: None,
                            },
                        )
                        .unwrap();
                    let before = runtime.store.load_record(&process.process_id).unwrap();
                    runtime.resolver = Box::new(TestResolver {
                        kernels: HashMap::from([(
                            ("urn:test:bound-actors".into(), "1.0.0".into()),
                            foreign,
                        )]),
                    });
                    if identity == "matching" {
                        runtime.drain_once(&process.process_id).unwrap();
                        assert_eq!(policy_calls.load(Ordering::SeqCst), 1);
                        assert_eq!(service_calls.load(Ordering::SeqCst), 1);
                        continue;
                    }
                    assert!(matches!(
                        runtime.drain_once(&process.process_id),
                        Err(RuntimeError::ActorBinding(_))
                    ));
                    assert_eq!(policy_calls.load(Ordering::SeqCst), 0);
                    assert_eq!(service_calls.load(Ordering::SeqCst), 0);
                    let after = runtime.store.load_record(&process.process_id).unwrap();
                    assert_eq!(
                        serde_json::to_value(&after.process).unwrap(),
                        serde_json::to_value(&before.process).unwrap()
                    );
                    assert_eq!(
                        crate::runtime_aux_to_json(&after),
                        crate::runtime_aux_to_json(&before)
                    );
                }
            }
        }
    }

    mod task_lifetime_tests {
        use super::*;

        fn kernel(anchor: Option<&str>, lifetime: bool) -> KernelDocument {
            let mut timer = serde_json::json!({"action":"startTimer", "timerId":"response", "duration":"PT1S",
                "event":{"kind":"timer","timerId":"response","source":"custom","firesAs":"elapsed"}});
            if let Some(anchor) = anchor {
                timer["startAtExpression"] = anchor.into();
            }
            let mut task = serde_json::json!({"action":"createTask","taskRef":"review","assignTo":"reviewer",
                "contractRef":"reviewForm","completionEvent":"done","failureEvent":"failed"});
            if lifetime {
                task["expiresWithTimer"] = "response".into();
            }
            serde_json::from_value(serde_json::json!({"$wosWorkflow":"1.0","url":"urn:test:lifetime","version":"1.0.0",
                "contracts":{"reviewForm":{"binding":"formspec","ref":"urn:formspec:review"}},
                "lifecycle":{"initialState":"open","states":{
                    "open":{"type":"atomic","onEntry":[timer,task],"transitions":[{"event":"advance","target":"later"}]},
                    "later":{"type":"atomic"}}}})).unwrap()
        }

        fn create(kernel: KernelDocument, now: u64) -> (WosRuntime, String) {
            let mut runtime = runtime_with_kernel(kernel);
            runtime.clock = Box::new(FixedClock { now_ms: now });
            let id = stack_common_typeid::mint_process_id();
            runtime
                .create_process(CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:lifetime".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: Some(
                        serde_json::json!({"unchanged":true,"origin":"1970-01-01T00:00:00Z"}),
                    ),
                })
                .unwrap();
            (runtime, id)
        }

        fn snapshot(runtime: &WosRuntime, id: &str) -> serde_json::Value {
            let record = runtime.store.load_record(id).unwrap();
            serde_json::json!({"process":record.process,"aux":crate::runtime_aux_to_json(&record)})
        }

        #[derive(Debug)]
        struct CountingProcessor {
            calls: Arc<AtomicUsize>,
        }

        impl crate::binding::ContractBindingAdapter for CountingProcessor {
            fn binding(&self) -> &'static str {
                "formspec"
            }
            fn prepare_task(
                &self,
                task: &ActiveTask,
                state: &serde_json::Value,
            ) -> Result<PreparedTask, BindingError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                crate::binding::ContractBindingAdapter::prepare_task(&TestAdapter, task, state)
            }
            fn validate_submission(
                &self,
                task: &ActiveTask,
                response: &serde_json::Value,
            ) -> Result<SubmissionValidation, BindingError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                crate::binding::ContractBindingAdapter::validate_submission(
                    &TestAdapter,
                    task,
                    response,
                )
            }
            fn compute_case_mutation(
                &self,
                task: &ActiveTask,
                response: &serde_json::Value,
            ) -> Result<Option<CaseMutationBundle>, BindingError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                crate::binding::ContractBindingAdapter::compute_case_mutation(
                    &TestAdapter,
                    task,
                    response,
                )
            }
        }

        #[test]
        fn overdue_task_refuses_fresh_draft_and_submit_before_response_mutation() {
            let calls = Arc::new(AtomicUsize::new(0));
            let mut bindings = BindingRegistry::new();
            bindings.register(CountingProcessor {
                calls: calls.clone(),
            });
            // A real pre-expiry submission proves this processor is installed and called.
            let (mut positive, positive_id) = create(kernel(None, true), 10_000);
            positive.bindings = bindings.clone();
            let positive_task = positive.load_process(&positive_id).unwrap().active_tasks[0]
                .task_id
                .clone();
            assert!(matches!(positive.submit_task_response(&positive_task, serde_json::json!({
                "status":"completed","definitionUrl":"urn:formspec:review","definitionVersion":"1.0.0","data":{"approved":true}
            }), "reviewer", Some("positive")), Ok(TaskSubmissionResult::Completed { .. })));
            assert!(calls.load(Ordering::SeqCst) > 0);
            calls.store(0, Ordering::SeqCst);
            let (mut runtime, id) = create(kernel(None, true), 10_000);
            runtime.bindings = bindings;
            let task = runtime.load_process(&id).unwrap().active_tasks[0]
                .task_id
                .clone();
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            let before = snapshot(&runtime, &id);
            assert!(matches!(
                runtime.persist_task_draft(
                    &task,
                    serde_json::json!({"status":"stopped","data":{"changed":true}}),
                    "reviewer",
                    Some("new-draft")
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
            assert!(matches!(
                runtime.submit_task_response(
                    &task,
                    serde_json::json!({"status":"completed","data":{"changed":true}}),
                    "reviewer",
                    Some("new-submit")
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(snapshot(&runtime, &id), before);
        }

        #[test]
        fn task_lifetime_captures_actual_business_calendar_adjusted_deadline() {
            let calendar: BusinessCalendarDocument = serde_json::from_value(serde_json::json!({
                "targetWorkflow":"urn:test:lifetime","version":"1.0.0","timezone":"UTC",
                "workWeek":["monday","tuesday","wednesday","thursday","friday"]
            }))
            .unwrap();
            let now = parse_timestamp("2026-01-02T23:59:59Z").unwrap();
            let mut runtime =
                runtime_with_kernel(kernel(None, true)).with_business_calendar(calendar);
            runtime.clock = Box::new(FixedClock { now_ms: now });
            let id = stack_common_typeid::mint_process_id();
            runtime
                .create_process(CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:lifetime".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: None,
                })
                .unwrap();
            let record = runtime.store.load_record(&id).unwrap();
            let task = &record.process.active_tasks[0];
            assert_eq!(record.process.timers[0].deadline, "2026-01-05T00:00:00Z");
            assert_eq!(task.expires_at.as_deref(), Some("2026-01-05T00:00:00Z"));
            assert_ne!(task.expires_at.as_deref(), Some("2026-01-03T00:00:00Z"));
            assert_eq!(task.expires_with_timer.as_deref(), Some("response"));
            let witness = record
                .provenance_log
                .iter()
                .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
                .unwrap();
            assert_eq!(
                witness.data.as_ref().unwrap()["details"]["expiresAt"],
                "2026-01-05T00:00:00Z"
            );
            assert_eq!(
                record.creation.as_ref().unwrap().tasks[0].expires_at,
                task.expires_at
            );
            runtime.clock = Box::new(FixedClock {
                now_ms: parse_timestamp("2026-01-03T00:00:00Z").unwrap(),
            });
            runtime
                .persist_task_draft(
                    &task.task_id,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    Some("weekend-draft"),
                )
                .unwrap();
        }

        #[test]
        fn absolute_timer_preserves_calendar_deadline_and_task_expiry_after_restore() {
            let mut selected = kernel(None, true);
            let timer = &mut selected.lifecycle.states.get_mut("open").unwrap().on_entry[0];
            timer.duration = None;
            timer.deadline = Some("2026-01-03T00:00:00.230Z".into());
            let calendar: BusinessCalendarDocument = serde_json::from_value(serde_json::json!({
                "targetWorkflow":"urn:test:lifetime", "version":"1.0.0", "timezone":"UTC",
                "workWeek":["monday","tuesday","wednesday","thursday","friday"]
            }))
            .unwrap();
            let now = parse_timestamp("2026-01-02T23:59:59Z").unwrap();
            let mut runtime =
                runtime_with_kernel(selected.clone()).with_business_calendar(calendar.clone());
            runtime.clock = Box::new(FixedClock { now_ms: now });
            let id = stack_common_typeid::mint_process_id();
            runtime
                .create_process(CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:lifetime".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: None,
                })
                .unwrap();
            let record = runtime.store.load_record(&id).unwrap();
            let expected = "2026-01-03T00:00:00.23Z";
            assert_eq!(record.process.timers[0].deadline, expected);
            assert!(record.process.timers[0].absolute_deadline);
            assert_eq!(
                record.process.active_tasks[0].expires_at.as_deref(),
                Some(expected)
            );
            let roundtrip =
                serde_json::from_value(serde_json::to_value(&record.process).unwrap()).unwrap();
            let restored =
                wos_core::Evaluator::from_instance(selected, &roundtrip, now + 500).unwrap();
            let (states, errors) =
                super::super::timers::timers_to_state(restored.timers(), Some(&calendar)).unwrap();
            assert!(errors.is_empty());
            assert_eq!(states[0].deadline, expected);
            assert!(states[0].absolute_deadline);
            let task = record.process.active_tasks[0].task_id.clone();
            runtime
                .persist_task_draft(
                    &task,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    Some("before"),
                )
                .unwrap();
            runtime.clock = Box::new(FixedClock {
                now_ms: parse_timestamp(expected).unwrap(),
            });
            assert!(matches!(
                runtime.persist_task_draft(
                    &task,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    Some("after")
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
        }

        #[test]
        fn preexpiry_draft_replays_after_expiry_and_physical_retirement() {
            let (mut runtime, id) = create(kernel(None, true), 10_000);
            let task = runtime.load_process(&id).unwrap().active_tasks[0]
                .task_id
                .clone();
            runtime.clock = Box::new(FixedClock { now_ms: 10_999 });
            let body = serde_json::json!({"status":"stopped","data":{"saved":true}});
            let original = runtime
                .persist_task_draft(&task, body.clone(), "reviewer", Some("retained-draft"))
                .unwrap();
            assert!(matches!(
                runtime
                    .submit_task_response(
                        &task,
                        serde_json::json!({"status":"stopped"}),
                        "reviewer",
                        Some("retained-submit")
                    )
                    .unwrap(),
                TaskSubmissionResult::Rejected { .. }
            ));
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            runtime.drain_once(&id).unwrap();
            let record = runtime.store.load_record(&id).unwrap();
            assert!(record.process.active_tasks.is_empty());
            assert!(record.provenance_log.iter().any(|p| p.record_kind
                == ProvenanceKind::TaskFailed
                && p.outcome.as_deref() == Some("expired")
                && p.actor_id.is_none()
                && p.actor_type.as_deref() == Some("system")));
            assert!(
                !record
                    .process
                    .pending_events
                    .iter()
                    .any(|e| e.event == "done" || e.event == "failed")
            );
            let before = snapshot(&runtime, &id);
            assert_eq!(
                runtime
                    .persist_task_draft(&task, body, "reviewer", Some("retained-draft"))
                    .unwrap()
                    .artifact_id,
                original.artifact_id
            );
            assert!(matches!(
                runtime
                    .submit_task_response(
                        &task,
                        serde_json::json!({"status":"stopped"}),
                        "reviewer",
                        Some("retained-submit")
                    )
                    .unwrap(),
                TaskSubmissionResult::Rejected { .. }
            ));
            assert!(matches!(
                runtime.submit_task_response(
                    &task,
                    serde_json::json!({"status":"completed"}),
                    "reviewer",
                    None
                ),
                Err(RuntimeError::TaskNotFound(_))
            ));
            assert_eq!(snapshot(&runtime, &id), before);
        }

        #[test]
        fn captured_calendar_fallback_belongs_to_task_not_later_same_id_timer() {
            let calendar:BusinessCalendarDocument = serde_json::from_value(serde_json::json!({
                "targetWorkflow":"urn:test:lifetime","version":"fallback-test","timezone":"UTC","workWeek":[]
            })).unwrap();
            for cancel in [false, true] {
                let mut value = serde_json::to_value(kernel(None, true)).unwrap();
                let actions = value["lifecycle"]["states"]["open"]["onEntry"]
                    .as_array_mut()
                    .unwrap();
                if cancel {
                    actions.push(serde_json::json!({"action":"cancelTimer","timerId":"response"}));
                } else {
                    let mut replacement = actions[0].clone();
                    replacement["duration"] = "PT0S".into();
                    actions.push(replacement);
                }
                let mut runtime = runtime_with_kernel(serde_json::from_value(value).unwrap())
                    .with_business_calendar(calendar.clone());
                runtime.clock = Box::new(FixedClock { now_ms: 10_000 });
                let id = stack_common_typeid::mint_process_id();
                runtime
                    .create_process(CreateProcessRequest {
                        process_id: id.clone(),
                        tenant: None,
                        definition_url: "urn:test:lifetime".into(),
                        definition_version: "1.0.0".into(),
                        initial_case_state: None,
                    })
                    .unwrap();
                let record = runtime.store.load_record(&id).unwrap();
                let task = record
                    .provenance_log
                    .iter()
                    .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
                    .unwrap();
                let details = &task.data.as_ref().unwrap()["details"];
                assert_eq!(details["expiresAt"], "1970-01-01T00:00:11Z");
                assert_eq!(details["expiresWithTimer"], "response");
                assert_eq!(details["calendarVersion"], "fallback-test");
                assert_eq!(details["calendarVersionConvergenceError"], true);
                if !cancel {
                    let later = record
                        .provenance_log
                        .iter()
                        .rev()
                        .find(|p| p.record_kind == ProvenanceKind::TimerCreated)
                        .unwrap();
                    assert_ne!(
                        later.data.as_ref().unwrap()["calendarVersionConvergenceError"],
                        true
                    );
                    assert_eq!(record.process.timers[0].deadline, "1970-01-01T00:00:10Z");
                } else {
                    assert!(record.process.timers.is_empty());
                }
                let mut restored = InMemoryStore::new();
                restored.restore_record(record.clone()).unwrap();
                let mut tampered = record;
                tampered
                    .provenance_log
                    .iter_mut()
                    .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
                    .unwrap()
                    .data
                    .as_mut()
                    .unwrap()["details"]["calendarVersionConvergenceError"] = false.into();
                assert!(restored.save_record(tampered).is_err());
            }
        }

        #[test]
        fn same_command_timer_replacement_preserves_task_action_time_expiry() {
            let mut value = serde_json::to_value(kernel(None, true)).unwrap();
            let actions = value["lifecycle"]["states"]["open"]["onEntry"]
                .as_array_mut()
                .unwrap();
            let mut replacement = actions[0].clone();
            replacement["duration"] = "PT10S".into();
            actions.push(replacement);
            let (mut runtime, id) = create(serde_json::from_value(value).unwrap(), 10_000);
            let process = runtime.load_process(&id).unwrap();
            assert_eq!(process.timers[0].deadline, "1970-01-01T00:00:20Z");
            let task = &process.active_tasks[0];
            assert_eq!(task.expires_at.as_deref(), Some("1970-01-01T00:00:11Z"));
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            assert!(matches!(
                runtime.persist_task_draft(
                    &task.task_id,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    None
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
        }

        #[test]
        fn same_command_timer_cancellation_preserves_task_action_time_expiry() {
            let mut value = serde_json::to_value(kernel(None, true)).unwrap();
            value["lifecycle"]["states"]["open"]["onEntry"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"action":"cancelTimer","timerId":"response"}));
            // Cancellation after createTask must not undo valid task creation.
            let (mut runtime, id) = create(serde_json::from_value(value).unwrap(), 10_000);
            let process = runtime.load_process(&id).unwrap();
            assert!(process.timers.is_empty());
            let task = &process.active_tasks[0];
            assert_eq!(task.expires_at.as_deref(), Some("1970-01-01T00:00:11Z"));
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            assert!(matches!(
                runtime.submit_task_response(
                    &task.task_id,
                    serde_json::json!({"status":"completed"}),
                    "reviewer",
                    None
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
        }

        #[test]
        fn timer_replacement_and_state_change_do_not_extend_original_admission() {
            let (mut runtime, id) = create(kernel(None, true), 10_000);
            let mut record = runtime.store.load_record(&id).unwrap();
            let task = record.process.active_tasks[0].clone();
            record.process.timers[0].deadline = "1970-01-01T00:01:00Z".into();
            runtime.store.save_record(record).unwrap();
            runtime
                .enqueue_event(
                    &id,
                    PendingEvent {
                        event: "advance".into(),
                        actor_id: None,
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
            runtime.drain_once(&id).unwrap();
            let retained = runtime.load_process(&id).unwrap();
            assert_eq!(retained.active_tasks[0].expires_at, task.expires_at);
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            assert!(matches!(
                runtime.persist_task_draft(
                    &task.task_id,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    None
                ),
                Err(RuntimeError::TaskExpired(_))
            ));
        }

        #[test]
        fn unbounded_task_survives_clock_expiry_and_state_change() {
            let (mut runtime, id) = create(kernel(None, false), 10_000);
            let task = runtime.load_process(&id).unwrap().active_tasks[0]
                .task_id
                .clone();
            runtime.clock = Box::new(FixedClock { now_ms: 11_000 });
            runtime
                .enqueue_event(
                    &id,
                    PendingEvent {
                        event: "advance".into(),
                        actor_id: None,
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
            runtime.drain_once(&id).unwrap();
            assert!(
                runtime
                    .load_process(&id)
                    .unwrap()
                    .active_tasks
                    .iter()
                    .any(|t| t.task_id == task)
            );
            runtime
                .persist_task_draft(
                    &task,
                    serde_json::json!({"status":"stopped"}),
                    "reviewer",
                    None,
                )
                .unwrap();
        }

        #[test]
        fn already_expired_creation_retains_created_and_failed_without_presentation() {
            let (runtime, id) = create(kernel(Some("caseFile.origin"), true), 10_000);
            let record = runtime.store.load_record(&id).unwrap();
            assert!(record.process.active_tasks.is_empty());
            assert!(
                record
                    .provenance_log
                    .iter()
                    .any(|p| p.record_kind == ProvenanceKind::TaskCreated)
            );
            assert!(
                record
                    .provenance_log
                    .iter()
                    .any(|p| p.record_kind == ProvenanceKind::TaskFailed
                        && p.outcome.as_deref() == Some("expired"))
            );
            assert!(
                !record
                    .provenance_log
                    .iter()
                    .any(|p| p.record_kind == ProvenanceKind::TaskPresented)
            );
            assert!(record.process.pending_events.is_empty());
        }

        #[test]
        fn captured_lifetime_and_completed_task_witness_are_immutable_on_save_restore() {
            let (mut runtime, id) = create(kernel(None, true), 10_000);
            let original = runtime.store.load_record(&id).unwrap();
            let mut restored = InMemoryStore::new();
            restored.restore_record(original.clone()).unwrap();
            assert_eq!(
                restored.load_record(&id).unwrap().process.active_tasks[0].expires_at,
                original.process.active_tasks[0].expires_at
            );
            for changed in ["1970-01-01T00:00:12Z", "not-a-time"] {
                let mut corrupt = original.clone();
                corrupt.process.active_tasks[0].expires_at = Some(changed.into());
                assert!(matches!(
                    InMemoryStore::new().restore_record(corrupt.clone()),
                    Err(StoreError::InvalidCreationEvidence(_))
                ));
                assert!(runtime.store.save_record(corrupt).is_err());
            }
            let mut removed = original.clone();
            removed.process.active_tasks.clear();
            runtime.store.save_record(removed.clone()).unwrap();
            removed
                .provenance_log
                .retain(|p| p.record_kind != ProvenanceKind::TaskCreated);
            assert!(runtime.store.save_record(removed).is_err());
        }

        #[test]
        fn later_created_unbound_lifetime_witness_survives_task_removal() {
            let mut value = serde_json::to_value(kernel(None, true)).unwrap();
            value["lifecycle"]["states"]["open"]["transitions"][0]["actions"] = serde_json::json!([
                {"action":"createTask","taskRef":"later","assignTo":"reviewer","expiresWithTimer":"response"}
            ]);
            let (mut runtime, id) = create(serde_json::from_value(value).unwrap(), 10_000);
            runtime
                .enqueue_event(
                    &id,
                    PendingEvent {
                        event: "advance".into(),
                        actor_id: None,
                        data: None,
                        timestamp: String::new(),
                        idempotency_token: None,
                    },
                )
                .unwrap();
            runtime.drain_once(&id).unwrap();
            let mut record = runtime.store.load_record(&id).unwrap();
            let later = record
                .process
                .active_tasks
                .iter()
                .find(|t| t.task_ref == "later")
                .unwrap()
                .task_id
                .clone();
            assert!(
                !record
                    .creation
                    .as_ref()
                    .unwrap()
                    .tasks
                    .iter()
                    .any(|t| t.task_id == later)
            );
            let mut duplicate_identity = record.clone();
            let first_id = duplicate_identity
                .provenance_log
                .iter()
                .find(|p| p.record_kind == ProvenanceKind::TaskCreated)
                .unwrap()
                .id
                .clone();
            duplicate_identity
                .provenance_log
                .iter_mut()
                .find(|p| {
                    p.record_kind == ProvenanceKind::TaskCreated
                        && p.data
                            .as_ref()
                            .and_then(|d| d.get("taskId"))
                            .and_then(serde_json::Value::as_str)
                            == Some(later.as_str())
                })
                .unwrap()
                .id = first_id;
            assert!(matches!(
                InMemoryStore::new().restore_record(duplicate_identity),
                Err(StoreError::InvalidCreationEvidence(_))
            ));
            record.process.active_tasks.retain(|t| t.task_id != later);
            runtime.store.save_record(record.clone()).unwrap();
            record.provenance_log.retain(|p| {
                p.data
                    .as_ref()
                    .and_then(|d| d.get("taskId"))
                    .and_then(serde_json::Value::as_str)
                    != Some(later.as_str())
            });
            assert!(runtime.store.save_record(record).is_err());
        }

        #[test]
        fn malformed_duplicate_and_blank_timer_lifetimes_refuse_before_task_allocation() {
            let selected = kernel(None, true);
            let (mut runtime, id) = create(selected.clone(), 10_000);
            let original = runtime.store.load_record(&id).unwrap();
            let timer = &original.process.timers[0];
            let capture = wos_core::timer::Timer {
                id: timer.timer_id.clone(),
                created_at_ms: timer.created_at_ms.unwrap(),
                deadline_ms: parse_timestamp(&timer.deadline).unwrap(),
                absolute_deadline: timer.absolute_deadline,
                fires_event: timer.event.clone(),
                created_in_state: timer.scope_state.clone().unwrap_or_default(),
                duration_iso: timer.duration_iso.clone().unwrap(),
                duration_ms: timer.duration_ms.unwrap(),
            };
            let observed = wos_core::eval::ObservedAction {
                lifecycle_state:"open".into(),actor_id:None,
                action:serde_json::from_value(serde_json::json!({
                    "action":"createTask","taskRef":"later","assignTo":"reviewer","expiresWithTimer":"response"
                })).unwrap(),event_data:None,task_expiry_timer:Some(capture),
            };
            // Establish a real valid capture control before testing each corruption.
            for mode in ["malformed", "duplicate", "blank", "missing", "foreign"] {
                let mut control = original.clone();
                assert!(
                    runtime
                        .create_active_task(
                            &selected,
                            &mut control,
                            &observed,
                            "1970-01-01T00:00:10Z"
                        )
                        .is_ok()
                );
                let mut record = original.clone();
                let mut invalid = observed.clone();
                match mode {
                    "malformed" => {
                        invalid.task_expiry_timer.as_mut().unwrap().deadline_ms = u64::MAX
                    }
                    "duplicate" => record.process.timers.push(record.process.timers[0].clone()),
                    "blank" => invalid.action.expires_with_timer = Some(" ".into()),
                    "missing" => invalid.task_expiry_timer = None,
                    _ => invalid.task_expiry_timer.as_mut().unwrap().id = "foreign".into(),
                }
                let before = serde_json::to_value(&record.process).unwrap();
                assert!(
                    runtime
                        .create_active_task(
                            &selected,
                            &mut record,
                            &invalid,
                            "1970-01-01T00:00:10Z"
                        )
                        .is_err()
                );
                assert_eq!(serde_json::to_value(&record.process).unwrap(), before);
            }
        }

        #[test]
        fn missing_timer_refuses_creation_before_any_process_is_saved() {
            let mut value = serde_json::to_value(kernel(None, true)).unwrap();
            value["lifecycle"]["states"]["open"]["onEntry"]
                .as_array_mut()
                .unwrap()
                .remove(0);
            let mut runtime = runtime_with_kernel(serde_json::from_value(value).unwrap());
            let id = stack_common_typeid::mint_process_id();
            assert!(matches!(
                runtime.create_process(CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:lifetime".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: None
                }),
                Err(RuntimeError::Evaluator(message)) if message.contains("absent timer")
            ));
            assert!(runtime.store.load_record(&id).is_err());
        }
    }
    include!("deferred_callback_tests.rs");
}
