// Rust guideline compliant 2026-02-21

//! Registry invariants: every implemented lint rule appears, and rules
//! promoted past `Draft` carry at least one fixture link. The Task 1
//! bootstrap test (every rule at Draft with empty fixtures) is superseded
//! by Task 2, which backfills fixture links for rules with real evidence.

use wos_lint::{all_lint_rules, Graduation};

#[test]
fn all_lint_rules_registry_is_non_empty() {
    assert!(
        !all_lint_rules().is_empty(),
        "wos-lint rule registry must list every implemented rule"
    );
}

#[test]
fn every_non_draft_rule_has_at_least_one_fixture() {
    let mut violations: Vec<&str> = Vec::new();
    for rule in all_lint_rules() {
        let is_draft = matches!(rule.graduation, Graduation::Draft);
        if !is_draft && rule.fixtures.is_empty() {
            violations.push(rule.id);
        }
    }
    assert!(
        violations.is_empty(),
        "rules promoted past Draft but missing fixture links: {:?}",
        violations
    );
}

#[test]
fn draft_rules_have_empty_fixtures() {
    for rule in all_lint_rules() {
        if matches!(rule.graduation, Graduation::Draft) {
            assert!(
                rule.fixtures.is_empty(),
                "Draft rule {} must not have fixture links until promoted",
                rule.id
            );
        }
    }
}
