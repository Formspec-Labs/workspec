// Rust guideline compliant 2026-05-02

//! Compile output artifact types.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Disposition;
use crate::events::EventBuffer;
use crate::manifest::CompileManifest;
use crate::phase9_export::WorkspaceExportBundle;

/// What the compiler returns on a successful run.
///
/// `wos_workflow` is the compiled `$wosWorkflow` document; downstream
/// callers serialize it for schema-pass validation. `scenarios`,
/// `approval_package`, `release_notes`, and `manifest` are the
/// remaining phase-8 outputs.
#[derive(Debug, Clone, Serialize)]
pub struct CompileArtifact {
    pub wos_workflow: Value,
    pub scenarios: Vec<EmittedScenario>,
    pub approval_package: ApprovalPackage,
    pub release_notes: Option<String>,
    pub manifest: CompileManifest,
    /// Three-valued disposition per `SA-MUST-cmp-040..043`. Defaults to
    /// `Compiled`. Pipeline raises to `EmitWithWarnings` when soft
    /// findings (e.g., `unmappedButApproved`) are present, and to
    /// `EmitWithBlockers` when lint-pass produces blocker-severity
    /// diagnostics — the artifact still flows to the caller for
    /// inspection, but MUST NOT be published.
    pub disposition: Disposition,
    /// Studio-tier readiness diagnostics produced at phase 6 (warnings
    /// / blockers that did not halt the compile). Skipped when
    /// serializing the published artifact bundle — they live in the
    /// workspace, not the published shape — but available in-memory
    /// for the CLI / dry-run paths.
    #[serde(skip_serializing)]
    pub readiness_findings: Vec<wos_studio_lint::LintDiagnostic>,
    /// Compiler lifecycle events per `SA-MUST-cmp-070..073`. One
    /// `phase-started` + `phase-completed` per phase, plus
    /// `gate-passed` / `gate-failed` per external gate, plus a final
    /// `compile-succeeded` or `compile-failed`. Sequence-numbered, not
    /// wall-clock-stamped, so output stays deterministic. JSON-Lines
    /// serializable via [`EventBuffer::to_jsonl`].
    #[serde(skip_serializing)]
    pub events: EventBuffer,
    /// Self-contained workspace export bundle per
    /// `SA-MUST-cmp-060..063`. Deterministic; reproducible from the
    /// embedded manifest alone; carries sources + PolicyObjects +
    /// mappings + scenarios + provenance log + custody receipts.
    #[serde(skip_serializing)]
    pub export_bundle: WorkspaceExportBundle,
}

/// One emitted Scenario per `wos-tooling.scenarios[*]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmittedScenario {
    pub id: String,
    pub scenario_type: Option<String>,
    pub status: Option<String>,
    /// Original Studio Scenario body, projected verbatim. Tightening
    /// happens as Wave 3 (scenario simulator) shapes it.
    pub body: Value,
}

/// Phase-8 ApprovalPackage. Deliberately permissive shape; the full
/// schema lives in `studio/specs/review-and-approval.md`.
///
/// `bound_manifest_hash` ties the package cryptographically to a
/// specific compile per `SA-MUST-cmp-073`: a verifier given a published
/// artifact + this ApprovalPackage can re-hash the manifest and confirm
/// approval was for this exact compile, not a sibling.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPackage {
    pub workflow_intent_id: String,
    pub workflow_intent_version: String,
    pub approvals: Vec<Value>,
    pub compliance_attestations: Vec<Value>,
    /// Sha256 of the JCS-canonicalized manifest minus its own
    /// `manifestHash` and `compiledAt`. Matches
    /// `CompileManifest.manifest_hash`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bound_manifest_hash: String,
    /// Open extension surface.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}
