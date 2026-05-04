// Rust guideline compliant 2026-05-02

//! Tier S5 — Scenario readiness (doc-local subset: SC-LINT-003,
//! SC-LINT-004, SC-LINT-006).
//!
//! `SC-LINT-001` (every-adverse-Outcome-has-scenario), `SC-LINT-002`
//! (every-AppealRight-has-scenario), `SC-LINT-005` (supersession-
//! affected-rerun), and the equity / accessibility / jurisdiction
//! cross-cuts live in `crate::workspace_rules` — they need cross-doc
//! resolution. Per the Wave 1.3 review, the previous doc-local
//! `SC-LINT-001` (warning when `lifecycleState` is absent) was a
//! different predicate sharing a rule id; that hygiene check is now
//! `SC-LINT-006`.

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::ScenarioDocument;

use super::studio_diagnostic;

pub fn check(doc: &ScenarioDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(scenarios) = doc.body.get("scenarios").and_then(Value::as_array) {
        for (i, scenario) in scenarios.iter().enumerate() {
            check_scenario(scenario, &format!("/scenarios/{i}"), diagnostics);
        }
    } else {
        // Single-scenario form: re-render through serde_json so we can apply
        // the same logic.
        if let Ok(value) = serde_json::to_value(doc) {
            check_scenario(&value, "", diagnostics);
        }
    }
}

fn check_scenario(scenario: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    sc_lint_006(scenario, base, diagnostics);
    sc_lint_003(scenario, base, diagnostics);
    sc_lint_004(scenario, base, diagnostics);
}

/// `SC-LINT-006` — Scenario SHOULD declare an explicit `lifecycleState`.
/// Hygiene check (was mis-labeled `SC-LINT-001`; that rule code is
/// reserved for the workspace-tier "every adverse Outcome has a
/// Scenario" check).
fn sc_lint_006(scenario: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if scenario.get("lifecycleState").is_none() {
        diagnostics.push(studio_diagnostic(
            "SC-LINT-006",
            LintSeverity::Warning,
            format!("{base}/lifecycleState"),
            "Scenario SHOULD declare an explicit lifecycleState (one of \
             generated / reviewed / passing / failing / acceptedAsKnownGap \
             / regression)."
                .to_string(),
        ));
    }
}

fn sc_lint_003(scenario: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let has_expected = scenario.get("expectedOutcome").is_some()
        || scenario.get("expectedTerminals").is_some()
        || scenario.get("expectedTrace").is_some();
    if !has_expected {
        diagnostics.push(studio_diagnostic(
            "SC-LINT-003",
            LintSeverity::Error,
            format!("{base}/expectedOutcome"),
            "Scenario MUST carry an expectedOutcome (or expectedTerminals / \
             expectedTrace) so the runner can compare actual vs expected."
                .to_string(),
        ));
    }
}

fn sc_lint_004(scenario: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let state = scenario
        .get("lifecycleState")
        .and_then(Value::as_str)
        .unwrap_or("");
    if state != "failing" {
        return;
    }
    let waiver = scenario.get("waiver");
    let accepted_as_known_gap = scenario
        .get("acceptedAsKnownGap")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !accepted_as_known_gap && waiver.is_none() {
        diagnostics.push(studio_diagnostic(
            "SC-LINT-004",
            LintSeverity::Error,
            format!("{base}/lifecycleState"),
            "Failing Scenario MUST be either acceptedAsKnownGap (with \
             rationale) or waived; otherwise it blocks workflow advance."
                .to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> ScenarioDocument {
        serde_json::from_value(value).expect("scenario doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    fn run(doc: ScenarioDocument) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn sc_lint_006_warns_on_missing_lifecycle() {
        let doc = parse(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-x",
            "scenarioType": "happy-path",
            "expectedOutcome": "success"
        }));
        assert_eq!(rule_count(&run(doc), "SC-LINT-006"), 1);
    }

    #[test]
    fn sc_lint_003_requires_expected_block() {
        let doc = parse(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-x",
            "lifecycleState": "passing",
            "scenarioType": "happy-path"
        }));
        assert_eq!(rule_count(&run(doc), "SC-LINT-003"), 1);
    }

    #[test]
    fn sc_lint_004_failing_needs_waiver_or_known_gap() {
        let doc = parse(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-x",
            "lifecycleState": "failing",
            "scenarioType": "happy-path",
            "expectedOutcome": "success"
        }));
        assert_eq!(rule_count(&run(doc), "SC-LINT-004"), 1);
    }

    #[test]
    fn sc_lint_004_passes_with_known_gap() {
        let doc = parse(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-x",
            "lifecycleState": "failing",
            "scenarioType": "happy-path",
            "expectedOutcome": "success",
            "acceptedAsKnownGap": true
        }));
        assert_eq!(rule_count(&run(doc), "SC-LINT-004"), 0);
    }

    #[test]
    fn collection_form_walks_each_scenario() {
        let doc = parse(json!({
            "$wosStudioScenario": "1.0",
            "scenarios": [
                {"id": "sc-1", "lifecycleState": "passing", "expectedOutcome": "success"},
                {"id": "sc-2", "scenarioType": "happy-path"}
            ]
        }));
        let diagnostics = run(doc);
        assert_eq!(rule_count(&diagnostics, "SC-LINT-006"), 1);
        assert_eq!(rule_count(&diagnostics, "SC-LINT-003"), 1);
    }
}
