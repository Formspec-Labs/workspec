// Rust guideline compliant 2026-02-21

//! Lint diagnostics with rule IDs, severity, and JSON paths.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    /// Structural error that makes the document non-conformant.
    Error,
    /// Likely mistake that should be reviewed.
    Warning,
    /// Informational suggestion for improvement.
    Info,
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

    /// JSONPath to the offending location (e.g., `"$.states.approved"`).
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
