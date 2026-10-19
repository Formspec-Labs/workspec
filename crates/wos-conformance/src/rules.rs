// Rust guideline compliant 2026-02-21

//! Tier 3 rule metadata registry for dynamic conformance checks.
//!
//! `wos-conformance` exposes a small T3 rule surface today — primarily the
//! Batch 16 processor-claim meta-rules verified in
//! [`crate::meta`]. This registry names those rules explicitly so
//! rule-coverage tooling (planned follow-up) can treat T1/T2 lint rules
//! and T3 runtime rules uniformly.
//!
//! Metadata types (`RuleMetadata`, `Tier`, `Graduation`, `LintSeverity`) are
//! re-exported from `wos-lint` to keep the ladder definition singular.

// Use the registry Tier (wos_lint::rules::Tier) explicitly so this module
// stays decoupled from the diagnostic::Tier added in §5.2, which wos_lint
// now re-exports under the same short name.
pub use wos_lint::rules::{Graduation, Tier};
pub use wos_lint::{LintSeverity, RuleMetadata};

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
    // Full native registration remains incomplete. Local policy-stage checks
    // and rejection of retired inputs do not prove required operation owners.
    RuleMetadata {
        id: "AI-001",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Processor MUST implement agent registration (AI S3).",
        // Required invocation/operational owners remain unsupported; retired
        // synthetic fixtures are historical and confer no positive coverage.
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // Report parsing/floors work locally; calibration/history/review owners
    // remain required for a complete confidence-framework claim.
    RuleMetadata {
        id: "AI-002",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Processor MUST implement the confidence framework (AI S7).",
        // Required invocation/operational owners remain unsupported; retired
        // synthetic fixtures are historical and confer no positive coverage.
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-004: verified by inline evidence construction in processor_conformance tests only —
    // no standalone JSON fixture file exists. Remains Draft until a fixture file is authored.
    RuleMetadata {
        id: "AI-004",
        tier: Tier::T3,
        severity: LintSeverity::Error,
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
        severity: LintSeverity::Error,
        summary: "Assist Governance Proxy MUST NOT modify conformance requirements.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // Historical event-hint autonomy records are retired; authenticated
    // operational-state provenance remains unsupported.
    RuleMetadata {
        id: "AI-AUTO-001",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Escalation expiry MUST revoke elevated autonomy and emit an autonomyDemotion record (AI S5.5).",
        // Required invocation/operational owners remain unsupported; retired
        // synthetic fixtures are historical and confer no positive coverage.
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // Drift declarations admit structurally; actual monitoring/demotion
    // operations are not implemented by supplied-output policy exercise.
    RuleMetadata {
        id: "AI-AUTO-002",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Drift-alert thresholds with action=demoteToAssistive MUST emit autonomyDemotion + driftReclassification and reroute the event through escalation (AI S5.5).",
        // Required invocation/operational owners remain unsupported; retired
        // synthetic fixtures are historical and confer no positive coverage.
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // Governance §3–§4 and Complete's additional sections require native
    // operation owners absent from this reference route. Refusal regression
    // tests and historical event-hint records confer no positive support.
    RuleMetadata {
        id: "G-051",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Governance Basic processor MUST enforce due process and review protocols.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-052",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Governance Complete processor MUST enforce all normative sections.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-DET-001",
        tier: Tier::T3,
        severity: LintSeverity::Error,
        summary: "Determination-tagged transitions MUST emit the pre-transition case-file snapshot in Facts-tier provenance.",
        fixtures: &["crates/wos-conformance/tests/fixtures/native-determination-snapshot.json"],
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
