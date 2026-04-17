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

mod diagnostic;
mod document;
pub mod rules;

pub use diagnostic::{Diagnostic, Severity};
pub use document::{DocumentKind, WosDocument, WosProject};
pub use rules::{all_lint_rules, Graduation, RuleMetadata, Tier};

/// Lint a single WOS document (Tier 1 checks only).
///
/// Runs all applicable single-document rules for the detected document kind.
/// Returns diagnostics sorted by path, then severity.
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
    let doc = document::parse(json)?;
    let mut diagnostics = Vec::new();
    rules::tier1::check(&doc, &mut diagnostics);
    diagnostics.sort();
    Ok(diagnostics)
}

/// Lint a single JSON Schema file for documentation coverage (`SCHEMA-DOC-001`).
///
/// Unlike [`lint_document`], which lints WOS *documents* that carry a
/// `$wos*` marker, this function lints the JSON Schema files under
/// `wos-spec/schemas/` themselves. For every *leaf property* (a node with
/// a concrete `type` and no composite children), it enforces:
///
/// - **Baseline**: `description` is present and at least 60 characters,
///   and `examples` is a non-empty array.
/// - **Critical** (`x-lm.critical == true`): `description` is at least
///   140 characters and `examples` has at least 2 entries.
///
/// Diagnostic paths are JSON Pointers into the schema document (e.g.,
/// `/properties/url`, `/$defs/State/properties/kind`).
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn lint_schema(schema_json: &str) -> Result<Vec<Diagnostic>, LintError> {
    let root: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| LintError::Parse(format!("invalid JSON schema: {e}")))?;
    let mut diagnostics = Vec::new();
    rules::schema_doc::check_schema(&root, &mut diagnostics);
    diagnostics.sort();
    Ok(diagnostics)
}

/// Lint a project directory (Tier 1 + Tier 2 checks).
///
/// Loads all WOS documents from the directory, resolves cross-references,
/// and runs both single-document and cross-document rules.
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
    let project = document::load_project(dir)?;
    let mut diagnostics = Vec::new();

    for doc in project.documents() {
        rules::tier1::check(doc, &mut diagnostics);
    }
    rules::tier2::check(&project, &mut diagnostics);

    diagnostics.sort();
    Ok(diagnostics)
}

/// Errors produced by the linting pipeline.
#[derive(Debug, thiserror::Error)]
pub enum LintError {
    /// JSON parsing or document detection failed.
    #[error("parse error: {0}")]
    Parse(String),

    /// Filesystem access failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
