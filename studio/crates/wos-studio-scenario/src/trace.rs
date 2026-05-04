// Rust guideline compliant 2026-05-02

//! Trace types — expected, actual, and the diff between them.
//!
//! Internal trace shape (`TraceStep`) carries `state_before`,
//! `state_after`, `event`, optional `actor`, and an optional
//! `data_delta` map. The internal shape is convenient for
//! Studio-tier diffing — it does NOT match the conformance-trace
//! schema (`wos-tooling.schema.json#/$defs/conformanceTrace__Root`)
//! one-to-one. Use [`ActualTrace::to_conformance_trace`] to project
//! into the schema-compatible shape.
//!
//! ## Conformance-trace shape contract
//!
//! Per `schemas/wos-tooling.schema.json#/$defs/conformanceTrace__Root`,
//! the projected document MUST have exactly:
//! - `fixtureId: string` (matches the fixture filename stem)
//! - `kernelVersion: string` (matches the `$wosWorkflow` marker)
//! - `steps: array` (each step matches `conformanceTrace__TraceStep`)
//! - `outcome: "pass" | "fail" | "error"`
//! - `additionalProperties: false`
//!
//! And each step MUST have:
//! - `stepIndex: integer >= 0`
//! - `event: { name: string, sourceActor?: string }`
//! - `stateBefore: string`
//! - `stateAfter: string`
//! - `additionalProperties: false`

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceStep {
    pub state_before: String,
    pub state_after: String,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub data_delta: IndexMap<String, Value>,
}

/// Expected trace declared by the Scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedTrace {
    pub initial_state: String,
    pub steps: Vec<TraceStep>,
    pub expected_terminals: Vec<String>,
}

/// Actual trace computed by the simulator.
///
/// Note: an `actor_summary` field existed on earlier shapes but had no
/// defined contract or consumer (Wave 3 review MAJOR-9). Removed; if a
/// real consumer ever needs per-actor analytics, surface them through
/// a dedicated `ActorBreakdown` type tied to a specific spec section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActualTrace {
    pub initial_state: String,
    pub steps: Vec<TraceStep>,
    pub final_state: Option<String>,
}

/// Outcome of a conformance run, matching
/// `wos-tooling.schema.json#/$defs/conformanceTrace__Root.outcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConformanceOutcome {
    Pass,
    Fail,
    Error,
}

impl ConformanceOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Error => "error",
        }
    }
}

impl ActualTrace {
    /// Project to the conformance-trace shape that
    /// `wos-tooling.schema.json#/$defs/conformanceTrace__Root` admits.
    ///
    /// `fixture_id` and `kernel_version` are required per the schema —
    /// callers MUST supply them rather than guess. `outcome` reflects
    /// the simulator's pass/fail/error verdict.
    ///
    /// The output's top-level keys are exactly `fixtureId`,
    /// `kernelVersion`, `steps`, `outcome` (no `scenarioRef` /
    /// `initialState` / `finalState` — the schema rejects those via
    /// `additionalProperties: false`).
    pub fn to_conformance_trace(
        &self,
        fixture_id: &str,
        kernel_version: &str,
        outcome: ConformanceOutcome,
    ) -> Value {
        let steps: Vec<Value> = self
            .steps
            .iter()
            .enumerate()
            .map(|(idx, s)| step_to_conformance_step(idx, s))
            .collect();
        json!({
            "fixtureId": fixture_id,
            "kernelVersion": kernel_version,
            "steps": steps,
            "outcome": outcome.as_str(),
        })
    }
}

/// Project an internal `TraceStep` to the conformance-trace step shape.
/// Drops `data_delta` and `actor` from the top-level step (the schema
/// rejects them via `additionalProperties: false`); `actor` flows into
/// `event.sourceActor` per the schema's intent.
fn step_to_conformance_step(idx: usize, step: &TraceStep) -> Value {
    let mut event = Map::new();
    event.insert("name".into(), Value::String(step.event.clone()));
    if let Some(actor) = &step.actor {
        event.insert("sourceActor".into(), Value::String(actor.clone()));
    }
    json!({
        "stepIndex": idx,
        "event": Value::Object(event),
        "stateBefore": step.state_before,
        "stateAfter": step.state_after,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraceDelta {
    pub matching_steps: usize,
    pub missing_steps: Vec<TraceStep>,
    pub extra_steps: Vec<TraceStep>,
    pub mismatched_steps: Vec<MismatchedStep>,
    pub terminal_match: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MismatchedStep {
    pub step_index: usize,
    pub expected: TraceStep,
    pub actual: TraceStep,
}

impl TraceDelta {
    pub fn ok(&self) -> bool {
        self.missing_steps.is_empty()
            && self.extra_steps.is_empty()
            && self.mismatched_steps.is_empty()
            && self.terminal_match
    }
}

/// Diff `expected` vs `actual`. Step-wise comparison up to the shorter
/// length; remaining expected steps are `missing_steps`; remaining
/// actual steps are `extra_steps`. Terminal match is true if
/// `actual.final_state` is in `expected.expected_terminals`.
pub fn diff(expected: &ExpectedTrace, actual: &ActualTrace) -> TraceDelta {
    let mut delta = TraceDelta::default();
    let pair_len = expected.steps.len().min(actual.steps.len());
    for i in 0..pair_len {
        if expected.steps[i] == actual.steps[i] {
            delta.matching_steps += 1;
        } else {
            delta.mismatched_steps.push(MismatchedStep {
                step_index: i,
                expected: expected.steps[i].clone(),
                actual: actual.steps[i].clone(),
            });
        }
    }
    if expected.steps.len() > actual.steps.len() {
        delta
            .missing_steps
            .extend(expected.steps[pair_len..].iter().cloned());
    } else if actual.steps.len() > expected.steps.len() {
        delta
            .extra_steps
            .extend(actual.steps[pair_len..].iter().cloned());
    }
    // Terminal-match logic. Empty `expected_terminals` is *not* an
    // automatic match — per SA-MUST-scn-002, every Scenario MUST carry
    // expectedTerminalOutcome. An empty list either means "scenario
    // declared none and still passed" (incorrect; a workflow that
    // ends in any state is not what was claimed) OR "scenario declared
    // none and produced no events" (vacuous). Both cases produce
    // `terminal_match = true` only when actual has no final_state.
    delta.terminal_match = match (&actual.final_state, expected.expected_terminals.is_empty()) {
        (None, true) => true,                  // both empty — vacuous match
        (None, false) => false,                // expected something, got nothing
        (Some(_), true) => false,              // expected nothing declared, got something — fail
        (Some(state), false) => {
            expected.expected_terminals.iter().any(|t| t == state)
        }
    };
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(before: &str, event: &str, after: &str) -> TraceStep {
        TraceStep {
            state_before: before.to_string(),
            state_after: after.to_string(),
            event: event.to_string(),
            actor: None,
            data_delta: IndexMap::new(),
        }
    }

    #[test]
    fn diff_empty_traces_match() {
        let expected = ExpectedTrace {
            initial_state: "intake".to_string(),
            steps: vec![],
            expected_terminals: vec![],
        };
        let actual = ActualTrace {
            initial_state: "intake".to_string(),
            steps: vec![],
            final_state: None,
        };
        let delta = diff(&expected, &actual);
        assert!(delta.ok());
    }

    #[test]
    fn diff_detects_missing_step() {
        let expected = ExpectedTrace {
            initial_state: "intake".to_string(),
            steps: vec![step("intake", "submit", "review")],
            expected_terminals: vec!["review".to_string()],
        };
        let actual = ActualTrace::default();
        let delta = diff(&expected, &actual);
        assert_eq!(delta.missing_steps.len(), 1);
        assert!(!delta.ok());
    }

    #[test]
    fn diff_detects_mismatch() {
        let expected = ExpectedTrace {
            initial_state: "intake".to_string(),
            steps: vec![step("intake", "submit", "review")],
            expected_terminals: vec!["review".to_string()],
        };
        let actual = ActualTrace {
            initial_state: "intake".to_string(),
            steps: vec![step("intake", "abandon", "withdrawn")],
            final_state: Some("withdrawn".to_string()),
        };
        let delta = diff(&expected, &actual);
        assert_eq!(delta.mismatched_steps.len(), 1);
        assert!(!delta.terminal_match);
    }

    #[test]
    fn conformance_trace_shape_matches_schema_required_fields() {
        // schemas/wos-tooling.schema.json#/$defs/conformanceTrace__Root
        // requires: fixtureId, kernelVersion, steps, outcome.
        // Top-level additionalProperties: false.
        let mut step1 = step("intake", "submit", "approved");
        step1.actor = Some("caseworker-1".to_string());
        let actual = ActualTrace {
            initial_state: "intake".to_string(),
            steps: vec![step1],
            final_state: Some("approved".to_string()),
        };
        let trace = actual.to_conformance_trace(
            "K-001-fixture",
            "1.0",
            ConformanceOutcome::Pass,
        );

        // Schema-required top-level fields.
        assert_eq!(trace["fixtureId"], "K-001-fixture");
        assert_eq!(trace["kernelVersion"], "1.0");
        assert_eq!(trace["outcome"], "pass");
        assert!(trace["steps"].is_array());

        // additionalProperties: false at top level — no forbidden keys.
        let obj = trace.as_object().unwrap();
        for key in obj.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "fixtureId" | "kernelVersion" | "steps" | "outcome"
                ),
                "forbidden top-level key in conformance trace: {key}"
            );
        }

        // Step shape: schema requires stepIndex, event{name}, stateBefore,
        // stateAfter. additionalProperties: false.
        let step = &trace["steps"][0];
        assert_eq!(step["stepIndex"], 0);
        assert_eq!(step["event"]["name"], "submit");
        assert_eq!(step["event"]["sourceActor"], "caseworker-1");
        assert_eq!(step["stateBefore"], "intake");
        assert_eq!(step["stateAfter"], "approved");
        let step_obj = step.as_object().unwrap();
        for key in step_obj.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "stepIndex" | "event" | "stateBefore" | "stateAfter"
                ),
                "forbidden step key: {key}"
            );
        }
    }
}
