// Rust guideline compliant 2026-02-21

//! Generic runtime orchestration for WOS processors.
//!
//! This crate owns runtime commands, atomic persistence boundaries, task
//! lifecycle coordination, and binding dispatch. Binding-specific behavior
//! lives in separate adapter crates.

pub mod binding;
pub mod companion;
pub mod integration;
pub mod milestones;
pub mod runtime;
pub mod store;

pub use binding::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SubmissionValidation,
};
pub use companion::ReferenceCompanionPolicy;
pub use integration::{
    IntegrationBinding, IntegrationContractRef, IntegrationProfileDocument, TargetWorkflow,
};
pub use runtime::{
    Clock, CompanionPolicy, CreateInstanceRequest, DrainOnceResult, PersistDraftResult,
    RuntimeError, RuntimeEventContext, RuntimeEventDecision, SystemClock, TaskSubmissionResult,
    WosRuntime,
};
pub use wos_core::business_calendar::BusinessCalendarDocument;
pub use store::{
    InMemoryStore, ReplayKey, ReplayOperation, ReplayValue, RuntimeRecord, RuntimeStore,
    StoreError, TaskArtifact, TaskArtifactKind,
};
