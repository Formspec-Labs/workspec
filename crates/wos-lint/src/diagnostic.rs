// Rust guideline compliant 2026-02-21

//! Lint diagnostics with rule IDs, severity, and JSON paths.
//!
//! ## JSONPath shape (rule-author convention, ratified 2026-05-02)
//!
//! Every [`LintDiagnostic::path`] is **slash-separated, leading-slash-
//! prefixed, parallel to RFC 6901 JSON Pointer** — e.g.,
//! `"/lifecycle/states/approved/transitions/0"`. New rules MUST follow
//! this convention so:
//!
//! - All conformance fixtures' `expected_errors` substrings match
//!   against a single canonical path shape (the K-049 / K-051..053
//!   fixtures established this; older `$.`-prefixed JSONPath drafts are
//!   rejected).
//! - Path strings round-trip through [`serde_json::Value::pointer`] for
//!   zero-copy traversal of in-memory trees.
//! - Tooling (case-portal, studio compiler at
//!   `studio/crates/wos-studio-compiler`) can consume
//!   diagnostic streams without parsing two competing path syntaxes.
//!
//! Implementation reference: `rules/continuous_mode.rs:206-209,274`
//! (K-049). Stage 4 Wave 4 review locked the decision.

use std::fmt;

use serde::{Deserialize, Serialize};

// ==========================================================================
// Structured diagnostic type — primary output of all lint rules (§5.2 Task 3).
// ==========================================================================

/// Verification tier a rule belongs to, serialized as `"T1"` / `"T2"` / `"T3"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// Single-document structural checks (`wos-lint`).
    T1,
    /// Cross-document resolution and FEL AST analysis (`wos-lint --project`).
    T2,
    /// Dynamic runtime conformance (`wos-conformance`).
    T3,
}

/// Severity of a [`LintDiagnostic`].
///
/// Ordered low → high: `Info < Warning < Error < Block`. The `Block`
/// rung was added 2026-05-02 to express "publication-blocker" findings
/// per Studio readiness-validation §6 — diagnostics that MUST halt a
/// publication advance even when authors waive lower-severity issues.
/// Wire form is kebab-case (e.g., `"block"`); JSON-strict consumers
/// that previously matched only `error|warning|info` need updating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    /// Informational suggestion for improvement.
    Info,
    /// Likely mistake that should be reviewed.
    Warning,
    /// Structural error that makes the document non-conformant.
    Error,
    /// Publication-blocker: a finding that MUST halt advancement to a
    /// gated lifecycle state (e.g., approved → published) regardless
    /// of waivers applied to lower severities. Used by Studio S6
    /// publication gate rules (`PUB-LINT-001/003/004/005/007`).
    Block,
}

/// A machine-readable remediation proposal attached to a [`LintDiagnostic`].
///
/// Serialized with a `"kind"` discriminant field in kebab-case so downstream
/// consumers can switch on the string without numeric enum variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SuggestedFix {
    /// Add a property at `path` with `value`.
    AddProperty {
        path: String,
        value: serde_json::Value,
    },
    /// Remove the property at `path`.
    RemoveProperty { path: String },
    /// Replace the value at `path` with `value`.
    ReplaceValue {
        path: String,
        value: serde_json::Value,
    },
    /// Rename a key or identifier from `from` to `to`.
    Rename { from: String, to: String },
    /// Free-form remediation hint when none of the above variants fit.
    ///
    /// Struct variant (not tuple) because serde's internally-tagged enum
    /// representation (`#[serde(tag = "kind")]`) does not support newtype or
    /// tuple variants — `serde_json::to_value` would fail at runtime.
    Custom { hint: String },
}

/// File location of the offending JSON node.
///
/// Absent from a [`LintDiagnostic`] when linting runs against an in-memory
/// tree rather than a file on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    /// Path or URL of the source document.
    pub document: String,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub column: u32,
}

/// A structured lint diagnostic with a stable camelCase JSON serialization.
///
/// This is the canonical output type for all lint rules in `wos-lint`. Every
/// rule emits `LintDiagnostic` instances; the public API surfaces
/// `Vec<LintDiagnostic>`. Downstream consumers may format these as plain text,
/// JSON, or SARIF through the helpers in [`crate::output`].
///
/// # Constructing diagnostics
///
/// Prefer the tier-specific helper constructors (`t1_error`, `t2_error`, etc.)
/// over populating the struct directly — they fill the common fields and accept
/// the `rule_id`, `path`, and `message` that every diagnostic requires.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintDiagnostic {
    /// Rule identifier from LINT-MATRIX.md (e.g., `"K-001"`).
    pub rule_id: &'static str,

    /// Diagnostic severity.
    pub severity: LintSeverity,

    /// Verification tier the rule belongs to.
    pub tier: Tier,

    /// JSON-pointer-shaped path to the offending location (e.g.,
    /// `"/lifecycle/states/approved/transitions/0"`).
    ///
    /// **Format decision (2026-05-02, Stage 4 Wave 4 review remediation):**
    /// Slash-separated, leading-slash-prefixed, parallel to RFC-6901 JSON
    /// Pointer. This matches the K-049 implementation in
    /// `rules/continuous_mode.rs:206-209,274` and is what the K-051/K-052/K-053
    /// fixtures' `expected_errors` substrings will match against. Earlier
    /// drafts of this doc said `$.`-prefixed JSONPath; that form is
    /// **rejected** to avoid the three-way conflict surfaced in code review.
    pub path: String,

    /// Human-readable description of the problem.
    pub message: String,

    /// Machine-readable remediation proposal, if the rule can make one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<SuggestedFix>,

    /// Spec sections or matrix entries related to this diagnostic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_docs: Vec<String>,

    /// File location of the offending node, when lint runs against a file.
    /// Absent when linting an in-memory tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLocation>,
}

impl LintDiagnostic {
    /// Create a Tier 1 error diagnostic.
    pub fn t1_error(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Error,
            tier: Tier::T1,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }

    /// Create a Tier 1 warning diagnostic.
    pub fn t1_warning(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Warning,
            tier: Tier::T1,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }

    /// Create a Tier 1 info diagnostic.
    pub fn t1_info(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Info,
            tier: Tier::T1,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }

    /// Create a Tier 2 error diagnostic.
    pub fn t2_error(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Error,
            tier: Tier::T2,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }

    /// Create a Tier 2 warning diagnostic.
    pub fn t2_warning(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Warning,
            tier: Tier::T2,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }

    /// Create a Tier 2 info diagnostic.
    pub fn t2_info(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity: LintSeverity::Info,
            tier: Tier::T2,
            path: path.into(),
            message: message.into(),
            suggested_fix: None,
            related_docs: Vec::new(),
            source: None,
        }
    }
}

impl fmt::Display for LintDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_label = match self.severity {
            LintSeverity::Block => "block",
            LintSeverity::Error => "error",
            LintSeverity::Warning => "warning",
            LintSeverity::Info => "info",
        };
        write!(
            f,
            "[{}] {} at {}: {}",
            self.rule_id, severity_label, self.path, self.message
        )
    }
}
