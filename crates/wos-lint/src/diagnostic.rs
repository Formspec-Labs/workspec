// Rust guideline compliant 2026-02-21

//! Lint diagnostics with rule IDs, severity, and JSON paths.

use std::fmt;

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
