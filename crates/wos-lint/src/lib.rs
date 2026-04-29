// Rust guideline compliant 2026-02-21

//! Static linter for WOS documents.
//!
//! Checks normative constraints that JSON Schema cannot enforce, organized
//! into two tiers:
//!
//! - **Tier 1** — single-document structural checks (30 rules)
//! - **Tier 2** — cross-document resolution + FEL AST analysis (47 rules)
//!
//! See `LINT-MATRIX.md` for the complete constraint catalog.
//!
//! # Structured output (§5.2)
//!
//! All lint rules now emit [`LintDiagnostic`] — a JSON-serializable struct
//! with stable `camelCase` field names, severity, verification tier, JSONPath
//! location, and an optional machine-readable [`SuggestedFix`].
//!
//! The [`lint_document_structured`], [`lint_project_structured`], and
//! [`lint_schema_structured`] functions return `Vec<LintDiagnostic>` directly.
//! The legacy [`lint_document`], [`lint_project`], and [`lint_schema`] functions
//! remain for callers that depend on the older [`Diagnostic`] type.

mod diagnostic;
mod document;
pub mod output;
pub mod rules;

pub use diagnostic::{
    Diagnostic, LintDiagnostic, LintSeverity, Severity, SourceLocation, SuggestedFix, Tier,
};
pub use document::{DocumentKind, WosDocument, WosProject};
pub use rules::{Graduation, RuleMetadata, all_lint_rules};

// ==========================================================================
// Structured API — primary interface returning `Vec<LintDiagnostic>`
// ==========================================================================

/// Lint a single WOS document (Tier 1 checks only), returning structured diagnostics.
///
/// Runs all applicable single-document rules for the detected document kind.
/// Returns diagnostics sorted by JSON path.
///
/// # Errors
///
/// Returns `LintError::Parse` if the input is not valid JSON or lacks a
/// recognized `$wos*` document type marker.
pub fn lint_document_structured(json: &str) -> Result<Vec<LintDiagnostic>, LintError> {
    let doc = document::parse(json)?;
    let mut diagnostics = Vec::new();
    rules::tier1::check(&doc, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(b.rule_id)));
    Ok(diagnostics)
}

/// Lint a single JSON Schema file for documentation coverage, returning structured diagnostics.
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn lint_schema_structured(schema_json: &str) -> Result<Vec<LintDiagnostic>, LintError> {
    let root: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| LintError::Parse(format!("invalid JSON schema: {e}")))?;
    let mut diagnostics = Vec::new();
    rules::schema_doc::check_schema(&root, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(b.rule_id)));
    Ok(diagnostics)
}

/// Lint a project directory (Tier 1 + Tier 2 checks), returning structured diagnostics.
///
/// # Errors
///
/// Returns `LintError::Io` if the directory cannot be read, or
/// `LintError::Parse` if any document fails to parse.
pub fn lint_project_structured(dir: &std::path::Path) -> Result<Vec<LintDiagnostic>, LintError> {
    let project = document::load_project(dir)?;
    let mut diagnostics = Vec::new();

    for doc in project.documents() {
        rules::tier1::check(doc, &mut diagnostics);
    }
    rules::tier2::check(&project, &mut diagnostics);

    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(b.rule_id)));
    Ok(diagnostics)
}

// ==========================================================================
// Legacy API — backward-compatible functions returning `Vec<Diagnostic>`
// ==========================================================================

/// Lint a single WOS document (Tier 1 checks only).
///
/// Runs all applicable single-document rules for the detected document kind.
/// Returns diagnostics sorted by path, then severity.
///
/// **Prefer [`lint_document_structured`]** for new callers — it returns the
/// richer [`LintDiagnostic`] type with tier, JSON path, and optional fixes.
///
/// # Examples
///
/// ```no_run
/// use wos_lint::lint_document;
///
/// let json = std::fs::read_to_string("kernel.json").unwrap();
/// let diagnostics = lint_document(&json).unwrap();
/// for d in &diagnostics {
///     eprintln!("{}", d);
/// }
/// ```
///
/// # Errors
///
/// Returns `LintError::Parse` if the input is not valid JSON or lacks a
/// recognized `$wos*` document type marker.
pub fn lint_document(json: &str) -> Result<Vec<Diagnostic>, LintError> {
    let structured = lint_document_structured(json)?;
    let mut diagnostics: Vec<Diagnostic> = structured.into_iter().map(Diagnostic::from).collect();
    diagnostics.sort();
    Ok(diagnostics)
}

/// Lint a single JSON Schema file for documentation coverage (`SCHEMA-DOC-001`).
///
/// Unlike [`lint_document`], which lints WOS *documents* that carry a
/// `$wos*` marker, this function lints the JSON Schema files under
/// `wos-spec/schemas/` themselves.
///
/// **Prefer [`lint_schema_structured`]** for new callers.
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn lint_schema(schema_json: &str) -> Result<Vec<Diagnostic>, LintError> {
    let structured = lint_schema_structured(schema_json)?;
    let mut diagnostics: Vec<Diagnostic> = structured.into_iter().map(Diagnostic::from).collect();
    diagnostics.sort();
    Ok(diagnostics)
}

/// Count the total number of SCHEMA-DOC-001 leaf properties in a schema.
///
/// Companion to `lint_schema` — same walk, but returns the count instead of
/// the diagnostics. Used by the leaf-count companion ratchet in
/// `schema_doc_zero_regression.rs` to detect "fill 1, sketch 1" gaming where
/// violation count stays flat but total leaf count grows.
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn count_schema_leaves(schema_json: &str) -> Result<usize, LintError> {
    let root: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| LintError::Parse(format!("invalid JSON schema: {e}")))?;
    Ok(rules::schema_doc::count_leaves(&root))
}

/// Lint a project directory (Tier 1 + Tier 2 checks).
///
/// Loads all WOS documents from the directory, resolves cross-references,
/// and runs both single-document and cross-document rules.
///
/// **Prefer [`lint_project_structured`]** for new callers.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use wos_lint::lint_project;
///
/// let diagnostics = lint_project(Path::new("my-workflow/")).unwrap();
/// for d in &diagnostics {
///     eprintln!("{}", d);
/// }
/// ```
///
/// # Errors
///
/// Returns `LintError::Io` if the directory cannot be read, or
/// `LintError::Parse` if any document fails to parse.
pub fn lint_project(dir: &std::path::Path) -> Result<Vec<Diagnostic>, LintError> {
    let structured = lint_project_structured(dir)?;
    let mut diagnostics: Vec<Diagnostic> = structured.into_iter().map(Diagnostic::from).collect();
    diagnostics.sort();
    Ok(diagnostics)
}

/// Errors produced by the linting pipeline.
#[derive(Debug, thiserror::Error)]
pub enum LintError {
    /// JSON parsing or document-shape detection failed (root not an object,
    /// invalid JSON, etc.). Marker absence is a separate variant —
    /// see [`LintError::MissingMarker`].
    #[error("parse error: {0}")]
    Parse(String),

    /// JSON was well-formed but carried no `$wos*` document-type marker.
    /// Promoted from a `LintError::Parse` substring sentinel
    /// (`"no recognized $wos* document type marker found"`) so downstream
    /// crates (`wos-synth-spike`, future synth providers) can match on
    /// the variant instead of substring-matching the message.
    #[error("no recognized $wos* document type marker found")]
    MissingMarker,

    /// Filesystem access failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
