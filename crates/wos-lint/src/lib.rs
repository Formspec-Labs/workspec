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
//! All lint rules emit [`LintDiagnostic`] — a JSON-serializable struct with
//! stable `camelCase` field names, severity, verification tier, JSONPath
//! location, and an optional machine-readable [`SuggestedFix`].
//!
//! [`lint_document`], [`lint_project`], and [`lint_schema`] return
//! `Result<Vec<LintDiagnostic>, LintError>`.

mod diagnostic;
mod document;
pub mod output;
pub mod rules;

pub use diagnostic::{LintDiagnostic, LintSeverity, SourceLocation, SuggestedFix, Tier};
pub use document::{DocumentKind, WosDocument, WosProject};
pub use rules::{Graduation, RuleMetadata, all_lint_rules};

/// Lint a single WOS document (Tier 1 checks only), returning structured diagnostics.
///
/// Runs all applicable single-document rules for the detected document kind.
/// Returns diagnostics sorted by JSON path.
///
/// # Errors
///
/// Returns `LintError::Parse` if the input is not valid JSON or lacks a
/// recognized `$wos*` document type marker.
pub fn lint_document(json: &str) -> Result<Vec<LintDiagnostic>, LintError> {
    let doc = document::parse(json)?;
    let mut diagnostics = Vec::new();
    rules::tier1::check(&doc, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    Ok(diagnostics)
}

/// Lint a single JSON Schema file for documentation coverage, returning structured diagnostics.
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn lint_schema(schema_json: &str) -> Result<Vec<LintDiagnostic>, LintError> {
    let root: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| LintError::Parse(format!("invalid JSON schema: {e}")))?;
    let mut diagnostics = Vec::new();
    rules::schema_doc::check_schema(&root, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    Ok(diagnostics)
}

/// Lint a project directory (Tier 1 + Tier 2 checks), returning structured diagnostics.
///
/// # Errors
///
/// Returns `LintError::Io` if the directory cannot be read, or
/// `LintError::Parse` if any document fails to parse.
pub fn lint_project(dir: &std::path::Path) -> Result<Vec<LintDiagnostic>, LintError> {
    let project = document::load_project(dir)?;
    let mut diagnostics = Vec::new();

    for doc in project.documents() {
        rules::tier1::check(doc, &mut diagnostics);
    }
    rules::tier2::check(&project, &mut diagnostics);

    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    Ok(diagnostics)
}

/// Count the total number of SCHEMA-DOC-001 leaf properties in a schema.
///
/// Companion to `lint_schema` — same walk, but returns the count instead of
/// the diagnostics. Use this for inventory reporting and ad hoc schema-doc
/// audits when callers need denominator data rather than per-property findings.
///
/// # Errors
///
/// Returns [`LintError::Parse`] if `schema_json` is not valid JSON.
pub fn count_schema_leaves(schema_json: &str) -> Result<usize, LintError> {
    let root: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| LintError::Parse(format!("invalid JSON schema: {e}")))?;
    Ok(rules::schema_doc::count_leaves(&root))
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
