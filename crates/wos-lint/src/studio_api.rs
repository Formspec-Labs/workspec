// Rust guideline compliant 2026-05-02

//! Published Studio-facing surface of `wos-lint`.
//!
//! Mirrors the contract laid out in `wos_core::studio_api`: Studio
//! (Authoring) crates may consume `wos-lint` ONLY via this module.
//! Lint-rule internals (`crate::rules::*`, `crate::document::*` private
//! items) are off-limits to Studio code; the boundary is enforced by
//! one workspace-wide grep-based guard test at
//! `studio/crates/wos-studio-types/tests/api_surface.rs`.
//!
//! ## What lives here
//!
//! - The diagnostic shape: [`LintDiagnostic`], [`LintSeverity`],
//!   [`SuggestedFix`], [`Tier`], [`SourceLocation`]. Studio rule modules
//!   reuse this exact shape so Studio diagnostics interoperate with the
//!   parent's tooling.
//! - [`lint_workflow`] — the compiler's external lint-pass gate (Wave
//!   2.3 of the Studio decoupling plan). Returns the same diagnostic
//!   stream as `lint_document` for the Workflow document kind.
//!
//! ## What does NOT live here
//!
//! - The 113 individual lint rules (T1/T2/T3) and their helpers. Studio
//!   does not extend the parent's rule registry; Studio rules live in
//!   `studio/crates/wos-studio-lint` against Studio document types.

pub use crate::{LintDiagnostic, LintSeverity, SourceLocation, SuggestedFix, Tier};

/// External lint-pass gate for the Studio compiler — **T1-only**.
///
/// Equivalent to [`lint_document`](crate::lint_document) on a Workflow
/// JSON. Returns zero diagnostics when the compiled envelope passes
/// every applicable Tier-1 (single-document structural) rule.
///
/// **Cross-document T2 rules are NOT run by this entrypoint.** Callers
/// that need T2 cross-document analysis (FEL AST resolution against
/// referenced contracts, agent-actor xref, signature coverage, etc.)
/// MUST use [`lint_workflow_with_project`] instead — it builds a
/// single-document `WosProject` in-memory and runs T1 + T2.
///
/// The Studio compiler's `lint-pass` external gate (F4.2, 2026-05-02)
/// uses [`lint_workflow_with_project`] as the default; this T1-only
/// surface is preserved for backward compatibility.
///
/// # Errors
///
/// Returns `LintError` if the JSON is malformed or fails the lint pipeline
/// for non-rule reasons (deserialization failure, unknown document kind).
pub fn lint_workflow(workflow_json: &str) -> Result<Vec<LintDiagnostic>, crate::LintError> {
    crate::lint_document(workflow_json)
}

/// External lint-pass gate for the Studio compiler — **T1 + T2**.
///
/// Wraps the input workflow JSON in an in-memory `WosProject` and
/// runs both Tier-1 (single-document structural) and Tier-2
/// (cross-document resolution / FEL AST analysis) rules. Returns zero
/// diagnostics when the compiled envelope passes every applicable
/// rule. The Studio compiler MUST refuse to emit if this returns any
/// diagnostic with severity `>= LintSeverity::Error`.
///
/// Note: T2 rules that require sibling documents (e.g., a Delivery
/// sidecar joined by `targetWorkflow`) will see only the workflow in
/// the project. The Studio compiler today emits a single-document
/// envelope; sidecar-tier validation lives at the workspace boundary,
/// not inside the lint-pass gate.
///
/// # Errors
///
/// Returns `LintError` if the JSON is malformed or fails the lint pipeline
/// for non-rule reasons (deserialization failure, unknown document kind).
pub fn lint_workflow_with_project(
    workflow_json: &str,
) -> Result<Vec<LintDiagnostic>, crate::LintError> {
    crate::lint_workflow_with_project(workflow_json)
}
