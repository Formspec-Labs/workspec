// Rust guideline compliant 2026-02-21

//! Registry invariants: every implemented lint rule appears, and every
//! rule starts at `Draft` with an empty `fixtures` slice. Promotions are
//! the job of a follow-up plan — Task 1 of the rule-coverage plan is
//! bootstrap only.

use wos_lint::{all_lint_rules, Graduation};

#[test]
fn all_lint_rules_registry_is_non_empty() {
    assert!(
        !all_lint_rules().is_empty(),
        "wos-lint rule registry must list every implemented rule"
    );
}

#[test]
fn every_rule_starts_at_draft_with_empty_fixtures() {
    for rule in all_lint_rules() {
        assert_eq!(
            rule.graduation,
            Graduation::Draft,
            "rule {} started at non-Draft graduation in Task 1 bootstrap",
            rule.id
        );
        assert!(
            rule.fixtures.is_empty(),
            "rule {} has fixtures populated in Task 1 bootstrap",
            rule.id
        );
        assert!(
            rule.spec_ref.is_none(),
            "rule {} has spec_ref populated in Task 1 bootstrap",
            rule.id
        );
        assert!(
            rule.suggested_fix.is_none(),
            "rule {} has suggested_fix populated in Task 1 bootstrap",
            rule.id
        );
    }
}
