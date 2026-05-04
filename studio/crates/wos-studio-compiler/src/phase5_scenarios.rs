// Rust guideline compliant 2026-05-02

//! Phase 5 — Emit scenario artifacts → project Scenarios into
//! `wos-tooling.scenarios[*]`.
//!
//! `SA-MUST-cmp-022`: only Scenarios in lifecycle state ≥ `reviewed`
//! are emitted. Scenarios in `failing` or `acceptedAsKnownGap` are
//! emitted with a status flag so downstream conformance does not treat
//! them as expected-passing.

use serde_json::Value;

use crate::artifact::EmittedScenario;
use wos_studio_lint::Workspace;

const EMITTABLE: &[&str] = &[
    "reviewed",
    "passing",
    "failing",
    "acceptedAsKnownGap",
    "regression",
];

pub fn run(ws: &Workspace) -> Vec<EmittedScenario> {
    let mut scenarios: Vec<EmittedScenario> = Vec::new();
    for (_doc, scenario) in ws.scenario_records() {
        let state = scenario
            .get("lifecycleState")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !EMITTABLE.contains(&state) {
            continue;
        }
        let id = scenario
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("scenario-?")
            .to_string();
        let scenario_type = scenario
            .get("scenarioType")
            .and_then(Value::as_str)
            .map(str::to_string);
        let status = match state {
            "failing" => Some("failing".to_string()),
            "acceptedAsKnownGap" => Some("known-gap".to_string()),
            "regression" => Some("regression".to_string()),
            _ => Some("expected".to_string()),
        };
        scenarios.push(EmittedScenario {
            id,
            scenario_type,
            status,
            body: scenario.clone(),
        });
    }
    // Stable sort by id.
    scenarios.sort_by(|a, b| a.id.cmp(&b.id));
    scenarios
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    #[test]
    fn skips_below_reviewed() {
        let ws = ws_from(vec![(
            "sc.json",
            json!({
                "$wosStudioScenario": "1.0",
                "scenarios": [
                    {"id": "s1", "lifecycleState": "generated"},
                    {"id": "s2", "lifecycleState": "reviewed", "scenarioType": "happy-path"}
                ]
            }),
        )]);
        let out = run(&ws);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "s2");
    }

    #[test]
    fn flags_failing_and_known_gap() {
        let ws = ws_from(vec![(
            "sc.json",
            json!({
                "$wosStudioScenario": "1.0",
                "scenarios": [
                    {"id": "s1", "lifecycleState": "failing"},
                    {"id": "s2", "lifecycleState": "acceptedAsKnownGap"}
                ]
            }),
        )]);
        let out = run(&ws);
        assert_eq!(out[0].status.as_deref(), Some("failing"));
        assert_eq!(out[1].status.as_deref(), Some("known-gap"));
    }
}
