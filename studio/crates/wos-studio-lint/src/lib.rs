// Rust guideline compliant 2026-05-02

//! Stage-4 readiness lint engine for Studio (Authoring) documents.
//!
//! Catalogues 70 readiness rules across six Studio tiers (S1–S6) per
//! `studio/specs/readiness-validation.md`. The crate is mirror-shaped
//! against the parent `wos-lint`:
//!
//! - [`StudioRule`] is the per-rule metadata.
//! - [`lint_document`] takes a [`wos_studio_model::StudioDocument`] and
//!   returns the parent's [`LintDiagnostic`] (consumed via
//!   [`wos_lint::studio_api`]) — for **doc-local** rules.
//! - [`lint_workspace`] takes a [`Workspace`] (collection of documents)
//!   and runs **cross-document** rules in addition to doc-local ones.
//! - Rule-family modules under [`rules`] hold per-rule check functions.
//!
//! ## Boundary
//!
//! Studio crates consume `wos-lint` ONLY through `wos_lint::studio_api`.
//! This crate's `LintDiagnostic` is the parent's `LintDiagnostic` — the
//! shape is shared so Studio diagnostics interoperate with parent
//! tooling without conversion.

#[cfg(test)]
pub(crate) mod date_util;
pub mod registry;
pub mod rules;
pub mod workspace;
pub mod workspace_rules;

pub use registry::{StudioGraduation, StudioRule, StudioTier, all_studio_rules};
pub use workspace::{Workspace, WorkspaceDocument};
pub use workspace_rules::lint_workspace;
pub use wos_lint::studio_api::{LintDiagnostic, LintSeverity, SourceLocation, SuggestedFix, Tier};

use wos_studio_model::StudioDocument;

/// Lint a single Studio document, running every applicable doc-local rule.
///
/// Returns diagnostics sorted by JSON path then rule id so output is
/// deterministic across runs. For cross-document rules, build a
/// [`Workspace`] and call [`lint_workspace`] instead.
pub fn lint_document(doc: &StudioDocument) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    rules::run_all(doc, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    diagnostics
}

/// Lint a Studio document supplied as raw JSON. Parses through
/// [`StudioDocument`] before delegating to [`lint_document`].
///
/// # Errors
///
/// Returns `serde_json::Error` if the JSON cannot be parsed or does not
/// carry a recognised `$wosStudio*` marker.
pub fn lint_json(json: &str) -> Result<Vec<LintDiagnostic>, serde_json::Error> {
    let doc: StudioDocument = serde_json::from_str(json)?;
    Ok(lint_document(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_lists_at_least_one_rule_per_active_tier() {
        let rules = all_studio_rules();
        let tiers: std::collections::HashSet<_> =
            rules.iter().map(|r| r.studio_tier).collect();
        for t in [
            registry::StudioTier::S1,
            registry::StudioTier::S2,
            registry::StudioTier::S3,
            registry::StudioTier::S4,
            registry::StudioTier::S5,
            registry::StudioTier::S6,
        ] {
            assert!(tiers.contains(&t), "missing tier: {t:?}");
        }
    }

    #[test]
    fn workspace_lint_walks_cross_document_rules() {
        let ws = Workspace::from_iter(vec![(
            "wf.json".to_string(),
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-1",
                "reviewerRoles": [{"id": "compliance", "requiredForPublication": true}]
            })
            .to_string(),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-002"),
            "expected PUB-LINT-002 to fire: {diagnostics:?}",
        );
    }
}
