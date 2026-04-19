// Rust guideline compliant 2026-02-21

//! Tier 3 rule metadata registry for dynamic conformance checks.
//!
//! `wos-conformance` exposes a small T3 rule surface today — primarily the
//! Batch 16 processor-claim meta-rules verified in
//! [`crate::meta`]. This registry names those rules explicitly so
//! rule-coverage tooling (planned follow-up) can treat T1/T2 lint rules
//! and T3 runtime rules uniformly.
//!
//! Metadata types (`RuleMetadata`, `Tier`, `Graduation`, `Severity`) are
//! re-exported from `wos-lint` to keep the ladder definition singular.

// Use the registry Tier (wos_lint::rules::Tier) explicitly so this module
// stays decoupled from the diagnostic::Tier added in §5.2, which wos_lint
// now re-exports under the same short name.
pub use wos_lint::rules::{Graduation, Tier};
pub use wos_lint::{RuleMetadata, Severity};

/// Return the full static registry of currently-implemented T3 conformance rules.
///
/// Ordering is by rule id to keep diffs readable; downstream code should
/// not rely on the ordering.
pub fn all_rules() -> &'static [RuleMetadata] {
    ALL_CONFORMANCE_RULES
}

/// T3 rules currently implemented by `wos-conformance`.
///
/// Today this is the Batch 16 processor-claim set verified by
/// [`crate::meta::verify_processor_manifest`]. Additional T3 rules are
/// exercised indirectly through fixture-driven runtime assertions and are
/// not yet reified as metadata entries — that tracking is part of the
/// rule-coverage plan follow-up.
static ALL_CONFORMANCE_RULES: &[RuleMetadata] = &[
    // AI-001: Fixture links are indirect — the linked files carry rule IDs
    // AI-005, AI-009, and AI-034, not AI-001 directly. The conformance verifier
    // runs by batch number (see AI_REGISTRATION_BATCHES) so the structural
    // coverage holds. Mirror of the AI-004 / AI-050 inline-evidence pattern.
    // (See 2026-04-18 review.)
    // Verified by processor manifest test against fixtures with batches 3, 4, 5, 10.
    RuleMetadata {
        id: "AI-001",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Processor MUST implement agent registration (AI S3).",
        fixtures: &[
            "crates/wos-conformance/tests/fixtures/ai-005-no-override-human.json",
            "crates/wos-conformance/tests/fixtures/ai-009-permission-bounds.json",
            "crates/wos-conformance/tests/fixtures/ai-034-confidence-report-required.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-002: Fixture links are indirect — the linked files carry rule IDs
    // AI-034, AI-035, and AI-036, not AI-002 directly. The conformance verifier
    // runs by batch number (see AI_CONFIDENCE_BATCHES) so the structural
    // coverage holds. Mirror of the AI-004 / AI-050 inline-evidence pattern.
    // (See 2026-04-18 review.)
    // Verified by processor manifest test against batch 5 (confidence framework) fixtures.
    RuleMetadata {
        id: "AI-002",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Processor MUST implement the confidence framework (AI S7).",
        fixtures: &[
            "crates/wos-conformance/tests/fixtures/ai-034-confidence-report-required.json",
            "crates/wos-conformance/tests/fixtures/ai-035-calibrated-confidence.json",
            "crates/wos-conformance/tests/fixtures/ai-036-confidence-below-floor.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-004: verified by inline evidence construction in processor_conformance tests only —
    // no standalone JSON fixture file exists. Remains Draft until a fixture file is authored.
    RuleMetadata {
        id: "AI-004",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Processor MUST delegate Formspec evaluation to a conformant processor.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-050: verified by inline evidence construction in processor_conformance tests only —
    // no standalone JSON fixture file exists. Remains Draft until a fixture file is authored.
    RuleMetadata {
        id: "AI-050",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Assist Governance Proxy MUST NOT modify conformance requirements.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // G-051: verified by governance-basic profile execution over G-002, G-006, G-007, G-010,
    // G-016, G-017, and G-018 fixtures in the conformance test suite.
    RuleMetadata {
        id: "G-051",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Governance Basic processor MUST enforce due process and review protocols.",
        fixtures: &[
            "crates/wos-conformance/tests/fixtures/g-002-notice-before-adverse.json",
            "crates/wos-conformance/tests/fixtures/g-006-appeal-independent-reviewer.json",
            "crates/wos-conformance/tests/fixtures/g-007-appeal-provenance.json",
            "crates/wos-conformance/tests/fixtures/g-010-independent-first.json",
            "crates/wos-conformance/tests/fixtures/g-016-review-sampling.json",
            "crates/wos-conformance/tests/fixtures/g-017-reviewer-separation.json",
            "crates/wos-conformance/tests/fixtures/g-018-override-rationale.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // G-052: verified by governance-complete profile execution over all G-*
    // fixtures. The runtime selects fixtures via `rule.starts_with("G-")` in
    // `meta::evaluate_governance_complete`, so every `g-*.json` fixture in
    // `crates/wos-conformance/tests/fixtures/` participates. Listed exhaustively
    // below so the registry reflects the full coverage set rather than a
    // representative sample. (See 2026-04-18 review.)
    RuleMetadata {
        id: "G-052",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Governance Complete processor MUST enforce all normative sections.",
        fixtures: &[
            "crates/wos-conformance/tests/fixtures/g-002-notice-before-adverse.json",
            "crates/wos-conformance/tests/fixtures/g-006-appeal-independent-reviewer.json",
            "crates/wos-conformance/tests/fixtures/g-007-appeal-provenance.json",
            "crates/wos-conformance/tests/fixtures/g-010-independent-first.json",
            "crates/wos-conformance/tests/fixtures/g-012-pipeline-stage-provenance.json",
            "crates/wos-conformance/tests/fixtures/g-013-weakest-link-risk.json",
            "crates/wos-conformance/tests/fixtures/g-016-review-sampling.json",
            "crates/wos-conformance/tests/fixtures/g-017-reviewer-separation.json",
            "crates/wos-conformance/tests/fixtures/g-018-override-rationale.json",
            "crates/wos-conformance/tests/fixtures/g-019-override-immutable.json",
            "crates/wos-conformance/tests/fixtures/g-020-rejection-detail.json",
            "crates/wos-conformance/tests/fixtures/g-021-task-provenance.json",
            "crates/wos-conformance/tests/fixtures/g-025-delegation-required.json",
            "crates/wos-conformance/tests/fixtures/g-026-delegation-in-provenance.json",
            "crates/wos-conformance/tests/fixtures/g-030-hold-timer-start.json",
            "crates/wos-conformance/tests/fixtures/g-032-temporal-resolution.json",
            "crates/wos-conformance/tests/fixtures/g-049-binding-type-neutral.json",
            "crates/wos-conformance/tests/fixtures/g-054-resume-cancels-hold-timer.json",
            "crates/wos-conformance/tests/fixtures/g-061-expired-calendar-ignored.json",
            "crates/wos-conformance/tests/fixtures/g-064-notification-missing-variables.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-DET-001",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Determination-tagged transitions MUST emit the pre-transition case-file snapshot in Facts-tier provenance.",
        fixtures: &["crates/wos-conformance/tests/fixtures/k-det-001-determination-snapshot.json"],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in ALL_CONFORMANCE_RULES {
            assert!(
                seen.insert(rule.id),
                "duplicate rule id in registry: {}",
                rule.id
            );
        }
    }

    #[test]
    fn registry_ids_are_sorted() {
        let ids: Vec<&str> = ALL_CONFORMANCE_RULES.iter().map(|r| r.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "registry entries must be sorted by id");
    }

    #[test]
    fn every_conformance_rule_is_tier_three() {
        for rule in ALL_CONFORMANCE_RULES {
            assert_eq!(
                rule.tier,
                Tier::T3,
                "conformance rule {} must be tagged T3",
                rule.id
            );
        }
    }
}
