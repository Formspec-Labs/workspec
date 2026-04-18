// Rust guideline compliant 2026-02-21

//! T3 conformance-registry invariants: every reified T3 rule appears, and
//! rules promoted past `Draft` carry at least one fixture link. The Task 1
//! bootstrap test (every rule at Draft with empty fixtures) is superseded
//! by Task 2, which backfills fixture links for rules with real evidence.

use wos_conformance::rules::all_rules;
use wos_lint::Graduation;

#[test]
fn all_conformance_rules_registry_is_non_empty() {
    assert!(
        !all_rules().is_empty(),
        "wos-conformance rule registry must list every implemented T3 rule"
    );
}

#[test]
fn every_non_draft_conformance_rule_has_at_least_one_fixture() {
    let mut violations: Vec<&str> = Vec::new();
    for rule in all_rules() {
        let is_draft = matches!(rule.graduation, Graduation::Draft);
        if !is_draft && rule.fixtures.is_empty() {
            violations.push(rule.id);
        }
    }
    assert!(
        violations.is_empty(),
        "conformance rules promoted past Draft but missing fixture links: {:?}",
        violations
    );
}

#[test]
fn draft_conformance_rules_have_empty_fixtures() {
    for rule in all_rules() {
        if matches!(rule.graduation, Graduation::Draft) {
            assert!(
                rule.fixtures.is_empty(),
                "Draft conformance rule {} must not have fixture links until promoted",
                rule.id
            );
        }
    }
}
