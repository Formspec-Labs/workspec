// Rust guideline compliant 2026-05-02

//! Compiler failure types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `failureKind` values per `SA-MUST-cmp-040..043`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureKind {
    /// Phase 1: missing required input documents (e.g., no
    /// `$wosStudioWorkflowIntent` document; or a `policyObjectRefs`
    /// entry references a PolicyObject id that doesn't exist anywhere
    /// in the workspace).
    MissingInput,
    /// Phase 1: PolicyObject IS present in the workspace but not in
    /// `lifecycleState` ≥ `approved`. Distinct from `MissingInput`
    /// (the doc exists; it's just not yet promoted).
    UnapprovedInput,
    /// Phase 2: PolicyObject referenced has no Mapping (or has multiple).
    UnmappedInput,
    /// Phase 3: WorkflowElement carries a malformed bridge for its kind.
    MalformedBridge,
    /// Phase 4: WorkflowIntent references an event with no EventBinding.
    UnresolvedEventReference,
    /// Phase 4: ServiceBinding does not cover required inputs.
    IncompleteServiceBinding,
    /// Phase 4: two PolicyObjects produce equivalent artifact content.
    ArtifactCollision,
    /// Phase 6: Studio readiness rule with severity ≥ error.
    StudioReadinessFailure,
    /// Phase 7: schema-pass external gate failed.
    SchemaPassFailed,
    /// Phase 7: lint-pass external gate failed.
    LintPassFailed,
    /// Phase 7: conformance-pass external gate failed.
    ConformancePassFailed,
    /// Phase 1: WorkflowIntent's `wosVersionPin` is incompatible with
    /// the compiler's loaded schema version (`SA-MUST-cmp-052`).
    PinMismatch,
}

impl FailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingInput => "missing-input",
            Self::UnapprovedInput => "unapproved-input",
            Self::UnmappedInput => "unmapped-input",
            Self::MalformedBridge => "malformed-bridge",
            Self::UnresolvedEventReference => "unresolved-event-reference",
            Self::IncompleteServiceBinding => "incomplete-service-binding",
            Self::ArtifactCollision => "artifact-collision",
            Self::StudioReadinessFailure => "studio-readiness-failure",
            Self::SchemaPassFailed => "schema-pass-failed",
            Self::LintPassFailed => "lint-pass-failed",
            Self::ConformancePassFailed => "conformance-pass-failed",
            Self::PinMismatch => "pin-mismatch",
        }
    }
}

/// Compile dispositions per `SA-MUST-cmp-040..043`. The compiler can
/// produce three flavors of artifact:
///
/// - `Compiled` — clean compile; artifact is publishable.
/// - `EmitWithWarnings` — soft warnings (`unmappedButApproved`
///   mappings, accepted-as-known-gap scenarios). Artifact is still
///   publishable; warnings flow into release notes.
/// - `EmitWithBlockers` — lint failures, unresolved tier-S6 findings.
///   Artifact is produced for inspection but MUST NOT be published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    Compiled,
    EmitWithWarnings,
    EmitWithBlockers,
}

/// Top-level compile error returned to callers.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("phase {phase}: {} — {message}", kind.as_str())]
    Halt {
        phase: u8,
        kind: FailureKind,
        message: String,
        details: Vec<String>,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CompileError {
    pub fn halt(
        phase: u8,
        kind: FailureKind,
        message: impl Into<String>,
    ) -> Self {
        Self::Halt {
            phase,
            kind,
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn halt_with(
        phase: u8,
        kind: FailureKind,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self::Halt {
            phase,
            kind,
            message: message.into(),
            details,
        }
    }
}
