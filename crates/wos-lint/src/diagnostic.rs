// Rust guideline compliant 2026-02-21

//! Lint diagnostics with rule IDs, severity, and JSON paths.

use std::fmt;

use serde::{Deserialize, Serialize};

// ==========================================================================
// New structured diagnostic type for §5.2. Will eventually supersede the
// existing `Diagnostic` struct; migration is Task 3 of the §5.2 plan.
// ==========================================================================

/// Verification tier a rule belongs to, serialized as `"T1"` / `"T2"` / `"T3"`.
///
/// This mirrors [`crate::rules::registry::Tier`] but adds `Serialize` /
/// `Deserialize` so it can appear inside [`LintDiagnostic`]. The two types
/// will be consolidated in Task 3 once all rules emit `LintDiagnostic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// Single-document structural checks (`wos-lint`).
    T1,
    /// Cross-document resolution and FEL AST analysis (`wos-lint --project`).
    T2,
    /// Dynamic runtime conformance (`wos-conformance`).
    T3,
}

/// A structured lint diagnostic with a stable camelCase JSON serialization.
///
/// This type will eventually supersede [`Diagnostic`] once all rules have been
/// migrated (Task 3). During the migration window both types coexist.
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

/// Severity of a [`LintDiagnostic`].
///
/// Named `LintSeverity` to avoid a name collision with the existing [`Severity`]
/// enum during the Task 3 migration window. They will be consolidated once all
/// rules emit `LintDiagnostic`.
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
    Custom(String),
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

/// Severity of a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Structural error that makes the document non-conformant.
    Error,
    /// Likely mistake that should be reviewed.
    Warning,
    /// Informational suggestion for improvement.
    Info,
}

/// A single lint diagnostic.
///
/// Diagnostics reference a rule ID from the LINT-MATRIX (e.g., `K-001`),
/// a JSON path to the offending location, and a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Rule identifier from LINT-MATRIX.md (e.g., "K-001", "G-037").
    pub rule_id: &'static str,

    /// JSON pointer path to the offending location (e.g., "/lifecycle/states/submitted").
    pub path: String,

    /// Human-readable description of the problem.
    pub message: String,

    /// Diagnostic severity.
    pub severity: Severity,
}

impl Diagnostic {
    /// Create an error diagnostic.
    pub fn error(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            path: path.into(),
            message: message.into(),
            severity: Severity::Error,
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            path: path.into(),
            message: message.into(),
            severity: Severity::Warning,
        }
    }

    /// Create an informational diagnostic.
    pub fn info(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            path: path.into(),
            message: message.into(),
            severity: Severity::Info,
        }
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path
            .cmp(&other.path)
            .then(self.severity.cmp(&other.severity))
            .then(self.rule_id.cmp(&other.rule_id))
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        write!(
            f,
            "[{}] {} at {}: {}",
            self.rule_id, severity_label, self.path, self.message
        )
    }
}
