// Rust guideline compliant 2026-02-21

//! Generic runtime orchestration for WOS processors.
//!
//! This crate owns runtime commands, atomic persistence boundaries, task
//! lifecycle coordination, and binding dispatch. Binding-specific behavior
//! lives in separate adapter crates.

pub mod binding;
pub mod cloudevents;
pub mod companion;
pub mod custody;
mod durable;
pub mod integration;
pub mod integration_handlers;
pub mod milestones;
pub mod policy_decision;
pub mod runtime;
pub mod store;

pub use binding::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SubmissionValidation,
};
pub use companion::ReferenceCompanionPolicy;
#[doc(inline)]
pub use custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendMetadata,
    CustodyLifecycleRef,
};
#[doc(inline)]
pub use durable::DurableRuntime;
pub use integration::{
    IntegrationBinding, IntegrationBindingKind, IntegrationContractRef, IntegrationProfileDocument,
    TargetWorkflow,
};
pub use runtime::{
    populate_provenance_record_fields, stamp_provenance, Clock, CompanionPolicy,
    CreateInstanceRequest, DrainOnceResult, PersistDraftResult, RuntimeError, RuntimeEventContext,
    RuntimeEventDecision, SystemClock, TaskSubmissionResult, WosRuntime,
};
pub use store::{
    InMemoryStore, ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, RuntimeStore,
    StoreError, TaskArtifact, TaskArtifactKind,
};
pub use wos_core::business_calendar::BusinessCalendarDocument;
