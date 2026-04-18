// Rust guideline compliant 2026-02-21

//! Diagnostic types for authoring operations.

use serde::{Deserialize, Serialize};

/// Severity level for an authoring diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// The operation cannot proceed; the document was not modified.
    Error,
    /// The operation succeeded but with a notable side effect.
    Warning,
}

/// A diagnostic emitted during a command dispatch.
///
/// Errors are returned directly from `dispatch`; warnings are accumulated
/// in `RawWosProject::diagnostics` after a successful operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoringDiagnostic {
    /// Index of the command that produced this diagnostic, if applicable.
    pub command_index: Option<usize>,

    /// Whether this diagnostic represents a fatal error or an advisory warning.
    pub severity: Severity,

    /// JSON-pointer-style path to the element involved (e.g., `"/lifecycle/states/draft"`).
    pub path: String,

    /// Human-readable description of the issue.
    pub message: String,
}

impl AuthoringDiagnostic {
    /// Create an error-severity diagnostic.
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_index: None,
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a warning-severity diagnostic.
    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_index: None,
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AuthoringDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}] {}: {}",
            self.severity, self.path, self.message
        )
    }
}

impl std::error::Error for AuthoringDiagnostic {}
