// Rust guideline compliant 2026-05-02

//! Studio readiness rule implementations — doc-local dispatch.
//!
//! Workspace-tier rules (those that need cross-document analysis) live
//! in `crate::workspace_rules`.

pub mod mapping_readiness;
pub mod policy_object_readiness;
pub mod publication_readiness;
pub mod scenario_readiness;
pub mod source_vault;
pub mod workflow_readiness;

use crate::LintDiagnostic;
use wos_studio_model::StudioDocument;

/// Dispatch every doc-local rule against `doc`, appending diagnostics.
pub fn run_all(doc: &StudioDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    match doc {
        StudioDocument::Source(d) => source_vault::check(d, diagnostics),
        StudioDocument::PolicyObject(d) => {
            policy_object_readiness::check(d, diagnostics);
        }
        StudioDocument::Mapping(d) => mapping_readiness::check(d, diagnostics),
        StudioDocument::WorkflowIntent(d) => {
            workflow_readiness::check(d, diagnostics);
        }
        StudioDocument::Scenario(d) => scenario_readiness::check(d, diagnostics),
        StudioDocument::IdentitySubject(d) => {
            publication_readiness::check_identity(d, diagnostics);
        }
        StudioDocument::Provenance(d) => {
            publication_readiness::check_provenance(d, diagnostics);
        }
        // Other markers run no doc-local rules; workspace-tier rules
        // pick them up via `crate::workspace_rules`.
        _ => {}
    }
}

/// Construct a Studio diagnostic. Studio rules emit at the parent's T1
/// tier — the diagnostic shape is shared via `wos_lint::studio_api`; the
/// `rule_id` (e.g., `"SV-LINT-001"`) discriminates Studio rules from
/// parent rules.
pub(crate) fn studio_diagnostic(
    rule_id: &'static str,
    severity: crate::LintSeverity,
    path: impl Into<String>,
    message: impl Into<String>,
) -> LintDiagnostic {
    LintDiagnostic {
        rule_id,
        severity,
        tier: crate::Tier::T1,
        path: path.into(),
        message: message.into(),
        suggested_fix: None,
        related_docs: Vec::new(),
        source: None,
    }
}
