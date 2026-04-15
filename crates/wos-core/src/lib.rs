// Rust guideline compliant 2026-02-21

//! Domain model and evaluation algorithm for WOS workflows.
//!
//! This crate owns the typed representation of WOS documents and the
//! deterministic lifecycle evaluation algorithm. It is format-independent:
//! consumers deserialize from JSON (or any other format) into these types,
//! and the evaluation logic operates on the typed model.
//!
//! # Architecture
//!
//! ```text
//!   wos-core (this crate)
//!      ↑           ↑           ↑
//!   wos-lint    wos-conformance  wos-runtime (future)
//! ```
//!
//! - `wos-lint` deserializes documents and runs static checks on the typed model.
//! - `wos-conformance` feeds events through the evaluation algorithm.
//! - A future `wos-runtime` adapts the algorithm to Temporal, Step Functions, etc.

pub mod autonomy;
pub mod business_calendar;
pub mod confidence;
pub mod context;
pub mod deontic;
pub mod eval;
pub mod eval_mode;
pub mod event_handler;
pub mod explain;
pub mod instance;
pub mod model;
pub mod project;
pub mod provenance;
pub mod proxy;
pub mod timer;
pub mod traits;

pub use context::EvalContext;
pub use eval::{
    Configuration, EvalError, Evaluator, IndexedState, ObservedAction, ObservedTransition,
    parse_iso_duration_to_ms,
};
pub use model::ai::AIIntegrationDocument;
pub use model::business_calendar::BusinessCalendarDocument;
pub use model::governance::GovernanceDocument;
pub use model::kernel::{Action, ActionKind, KernelDocument, State, StateKind, Transition};
pub use model::kernel::{Actor, ActorKind, CaseFile, CaseRelationship, FieldDefinition};
pub use model::kernel::{ContractReference, EvaluationMode, ExecutionConfig};
pub use model::kernel::{ImpactLevel, Lifecycle, Milestone, Region};
pub use model::notification_template::NotificationTemplateDocument;
pub use project::Project;
pub use provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};
pub use proxy::{AssistGovernanceProxyEvidence, observe_assist_governance_proxy};
pub use timer::Timers;
