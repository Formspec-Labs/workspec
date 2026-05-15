// Rust guideline compliant 2026-02-21

//! Generic runtime orchestration for WOS processors.
//!
//! This crate owns runtime commands, atomic persistence boundaries, task
//! lifecycle coordination, and binding dispatch. Binding-specific behavior
//! lives in separate adapter crates.

pub mod binding;
pub mod cloudevents;
pub mod companion;
mod durable;
pub mod intake;
pub mod integration;
pub mod integration_handlers;
pub mod milestones;
pub mod policy_decision;
pub mod restate_fixture_support;
pub mod runtime;
pub mod store;
pub mod studio_api;

pub use binding::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SignatureAdmissionFailure, SignatureAdmissionFailureReason, SignatureEvidence,
    SignaturePrimitiveStatus, SubmissionValidation,
};
pub use companion::ReferenceCompanionPolicy;
#[doc(inline)]
pub use durable::DurableRuntime;
pub use intake::{
    AutoCreatePublicIntakePolicy, IntakeAcceptanceAdapter, IntakeAcceptanceDecision,
    IntakeAcceptanceOutcome, IntakeAcceptancePolicy, IntakeAcceptanceRegistry,
    IntakeAcceptanceRequest, IntakeCaseDefinition, IntakeCaseDisposition, IntakeCaseIntent,
    IntakeInterpretation, IntakePolicyContext, IntakeRecordStatus, ManualReviewIntakePolicy,
    NoopIntakeAcceptancePolicy, PublicIntakeDisabledPolicy,
};
pub use integration::{
    IntegrationBinding, IntegrationBindingKind, IntegrationContractRef, IntegrationProfileDocument,
    TargetWorkflow,
};
pub use restate_fixture_support::{
    FixtureResolverError, MinimalFixtureFormspecAdapter, SharedInMemoryStore,
    SignatureFixtureResolver, restate_signature_fixture_bindings,
    restate_signature_fixture_runtime, signature_runtime_fixture_kernel,
    signature_runtime_fixture_profile,
};
pub use runtime::{
    Clock, CompanionPolicy, CompletionRequirementKind, CreateProcessRequest, DrainOnceResult,
    HttpPostureResolver, MigrationMap, MigrationOutcome, PersistDraftResult, PostureDeclaration,
    PostureResolver, ResolvedPostureDeclaration, RuntimeError, RuntimeEventContext,
    RuntimeEventDecision, SIGNATURE_PROFILE_KEY_EXTENSION, SIGNATURE_PROFILE_REF_EXTENSION,
    SIGNATURE_STEP_ID_EXTENSION, SignatureProfileDocument, StaticPostureResolver, SystemClock,
    TaskSubmissionResult, WosRuntime, populate_provenance_record_fields, stamp_provenance,
};
pub use store::{
    InMemoryStore, IntakeRecord, ReplayKey, ReplayOperation, ReplayValue, RuntimeAuxFields,
    RuntimeRecord, RuntimeStore, StoreError, TaskArtifact, TaskArtifactKind, runtime_aux_from_json,
    runtime_aux_to_json,
};
pub use wos_core::business_calendar::BusinessCalendarDocument;
#[doc(inline)]
pub use wos_events::custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendMetadata,
    CustodyAppendReceipt,
};
