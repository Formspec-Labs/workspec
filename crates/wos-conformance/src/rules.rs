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

pub use wos_lint::{Graduation, RuleMetadata, Severity, Tier};

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
    RuleMetadata {
        id: "AI-001",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Processor MUST implement agent registration (AI S3).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-002",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Processor MUST implement the confidence framework (AI S7).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
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
    RuleMetadata {
        id: "G-051",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Governance Basic processor MUST enforce due process and review protocols.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-052",
        tier: Tier::T3,
        severity: Severity::Error,
        summary: "Governance Complete processor MUST enforce all normative sections.",
        fixtures: &[],
        graduation: Graduation::Draft,
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
