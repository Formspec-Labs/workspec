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

pub mod model;
pub mod eval;
pub mod provenance;
pub mod context;
pub mod timer;
pub mod traits;
pub mod instance;
pub mod explain;
pub mod project;

pub use model::kernel::{KernelDocument, State, StateKind, Transition, Action, ActionKind};
pub use model::kernel::{Actor, ActorKind, CaseFile, FieldDefinition, CaseRelationship};
pub use model::kernel::{Lifecycle, Region, Milestone, ImpactLevel};
pub use model::kernel::{ContractReference, ExecutionConfig, EvaluationMode};
pub use model::governance::GovernanceDocument;
pub use model::ai::AIIntegrationDocument;
pub use model::business_calendar::BusinessCalendarDocument;
pub use model::notification_template::NotificationTemplateDocument;
pub use eval::{Evaluator, Configuration, EvalError, ObservedTransition, IndexedState, parse_iso_duration_to_ms};
pub use provenance::{ProvenanceLog, ProvenanceRecord, ProvenanceKind};
pub use context::EvalContext;
pub use timer::Timers;
pub use project::Project;
