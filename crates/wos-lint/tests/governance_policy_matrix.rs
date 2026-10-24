// Rust guideline compliant 2026-02-21
//! The schema/model fixture matrix exercises the actual Tier1 and Tier2 dispatchers.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use wos_lint::{DocumentKind, WosDocument, WosProject};

const GOVERNANCE_RULES: &[&str] = &[
    "G-001",
    "G-003",
    "G-004",
    "G-005",
    "G-008",
    "G-009",
    "G-014",
    "G-015",
    "G-062",
    "G-063",
    "WOS-GOVERNANCE-SHAPE-001",
];

fn matrix() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/validation/governance-policy-matrix.json"
    ))
    .unwrap()
}

fn diagnostics(case: &Value, delivery: &Value) -> Vec<wos_lint::LintDiagnostic> {
    let workflow = json!({
        "$wosWorkflow": "1.0", "url": "https://example.test/governance-matrix",
        "version": "1.0.0", "title": "Governance owner control", "impactLevel": "rights-impacting",
        "actors": [{"id": "reviewer", "type": "human"}],
        "lifecycle": {"initialState": "review", "states": {
            "review": {"type": "atomic", "tags": if case["hold"] == true {json!(["hold"])} else {json!([])},
                "transitions": [{"event": {"name": "decide", "kind": "message"}, "target": "done", "tags": ["determination", "adverse-decision"]}]},
            "done": {"type": "final"}
        }},
        "governance": case["governance"]
    });
    let mut project = WosProject::default();
    project.push(WosDocument {
        kind: DocumentKind::Workflow,
        value: workflow,
        source: None,
    });
    if !delivery.is_null() {
        project.push(WosDocument {
            kind: DocumentKind::Delivery,
            value: delivery.clone(),
            source: None,
        });
    }
    let mut diagnostics = Vec::new();
    for doc in project.documents() {
        wos_lint::rules::tier1::check(doc, &mut diagnostics);
    }
    wos_lint::rules::tier2::check(&project, &mut diagnostics);
    diagnostics
}

#[test]
fn canonical_policy_rules_and_structural_refusals_agree() {
    let matrix = matrix();
    for case in matrix["cases"].as_array().unwrap() {
        let delivery = &matrix["deliveries"][case["delivery"].as_str().unwrap()]["document"];
        let result = diagnostics(case, delivery);
        let observed: BTreeSet<_> = result
            .iter()
            .filter(|d| GOVERNANCE_RULES.contains(&d.rule_id))
            .map(|d| d.rule_id)
            .collect();
        if case["structurallyValid"] == false {
            assert!(
                observed.contains("WOS-GOVERNANCE-SHAPE-001"),
                "{}: {result:?}",
                case["id"]
            );
        } else {
            let expected: BTreeSet<_> = case["expectedRules"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap())
                .collect();
            assert_eq!(observed, expected, "{}: {result:?}", case["id"]);
        }
        for diagnostic in result
            .iter()
            .filter(|d| GOVERNANCE_RULES.contains(&d.rule_id))
        {
            let expected_prefix = if diagnostic.rule_id == "G-062" {
                "/notifications/templates/"
            } else {
                "/governance"
            };
            assert!(
                diagnostic.path.starts_with(expected_prefix),
                "{}: {diagnostic:?}",
                case["id"]
            );
        }
    }
}
