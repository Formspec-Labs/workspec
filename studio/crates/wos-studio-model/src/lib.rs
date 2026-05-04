// Rust guideline compliant 2026-05-02

//! Typed serde model for the Studio (Authoring) document schemas.
//!
//! Each `$wosStudio*` marker maps to a top-level [`StudioDocument`] variant
//! plus a per-marker module under [`mod@docs`] holding the envelope + body
//! types. Bodies are intentionally permissive (`serde_json::Value` for
//! polymorphic per-kind blocks) so downstream consumers — Stage-4
//! readiness lint, Stage-5 compiler, Stage-6 scenario simulator — can grow
//! tighter typing where they need it without forcing exhaustive struct
//! authoring up front.
//!
//! ## Boundary
//!
//! This crate is a leaf of the Studio dependency graph. It depends only on
//! `wos-studio-types` (Studio-local shared vocab) and serde. It does NOT
//! depend on `wos-core`, `wos-lint`, or `wos-runtime` — Studio-tier
//! authoring documents are independent of the kernel they eventually
//! compile to. That dependency arrow becomes load-bearing once
//! `wos-studio-compiler` (Wave 2 of the comprehensive plan) takes a
//! `StudioDocument` and emits a kernel `KernelDocument`.

pub mod common;
pub mod docs;
pub mod marker;
pub mod policy;

pub use common::{
    AuthorityGrantApplied, ExtractedClaimReviewState, Iri, MappingState, OriginClass,
    PolicyObjectLifecycleState, ScenarioLifecycleState, SourceVersionLifecycleState,
    WorkflowIntentLifecycleState,
};
pub use docs::{
    ApprovalDocument, BindingDocument, EffectivenessDocument, IdentitySubjectDocument,
    MappingDocument, MigrationPathDocument, PolicyObjectDocument, ProvenanceDocument,
    ReadinessDocument, ScenarioDocument, SourceDocument, StudioDocument,
    TerminologyMapDocument, WorkflowIntentDocument, WorkspaceDocument,
};
pub use marker::{StudioMarker, classify};
pub use policy::{DisposalAction, RetentionMode, RetentionPolicy, TriggerEvent};

// Common types from `common` are also re-exported via [`crate::common::*`]; the
// list above pins the most-commonly-imported ones at the crate root for
// ergonomics.
