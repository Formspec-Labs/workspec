// Rust guideline compliant 2026-05-02

//! Published Studio-facing surface of `wos-core`.
//!
//! This module is the **only** entry point Studio (Authoring) crates may
//! use to consume `wos-core` types. The boundary is enforced by one
//! workspace-wide grep-based guard test at
//! `studio/crates/wos-studio-types/tests/api_surface.rs` (Wave 0.3 of
//! the Studio decoupling, 2026-05-02).
//!
//! ## Why a published surface
//!
//! The owner anticipates extracting `studio/` to its own repo with the
//! parent consumed via submodule. To keep the eventual move mechanical —
//! `git filter-repo --subdirectory studio/` plus a path swap — every
//! Studio import goes through this module rather than reaching into
//! private internals. New Studio code that needs a wos-core type either
//! finds it re-exported here or proposes adding it here in a wos-core PR.
//!
//! ## What lives here
//!
//! - `KernelDocument` and the lifecycle types Studio's Wave-2 compiler
//!   emits as it composes a `wos-workflow.json` envelope.
//! - The polymorphic `Guard` / `DecisionTable` family — Studio's compiler
//!   normalizes Studio decision-table PolicyObjects into these.
//! - The seam types Studio's compiler must populate (`Action`, `Actor`,
//!   `Transition`, `Milestone`, `CaseFile`, etc.).
//! - Shared scalar / enum types (`ImpactLevel`, `EvaluationMode`,
//!   `ExecutionConfig`, `ContractReference`).
//!
//! ## What does NOT live here
//!
//! - The `Evaluator` / `EvalContext` / `EvalError` runtime surface.
//!   Studio's scenario simulator (Wave 3) consumes the runtime through
//!   `wos_runtime::studio_api`, not through wos-core directly.
//! - The provenance / instance / project / typeid types that are
//!   internal to wos-core's processor concerns.

pub use crate::model::decision_table::{
    DecisionTable, DecisionTableGuard, DecisionTableGuardKind, DecisionTableInput,
    DecisionTableOutput, DecisionTableRow, FelType, Guard, HitPolicy, OnNoMatch,
};
pub use crate::model::kernel::{
    Action, ActionKind, Actor, ActorKind, CaseFile, CaseRelationship, ContractReference,
    EvaluationMode, ExecutionConfig, FieldDefinition, ImpactLevel, KernelDocument, Lifecycle,
    Milestone, Region, SignalScope, State, StateKind, TimerEventSource, Transition,
    TransitionEvent,
};
