//! Structured execution traces for T3 conformance runs.
//!
//! Emitted by the runner per fixture; consumed by `explain` / `diff` CLI
//! subcommands (future tasks). See Q5.3 plan for design.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConformanceTrace {
    pub fixture_id: String,
    pub kernel_version: String,
    pub steps: Vec<TraceStep>,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraceStep {
    pub step_index: u32,
    pub event: Event,
    pub state_before: String,
    pub state_after: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_state_after: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guards_evaluated: Vec<GuardEvaluation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies_applied: Vec<PolicyApplication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

// `GuardEvaluation` is the canonical runtime-observation type defined in
// `wos-core`. Re-exported here so the trace JSON schema uses one source-of-
// truth type across the runtime → conformance boundary. Adds source_state /
// target_state / event fields beyond the original 4-field sketch; these are
// load-bearing for the teaching signal (§5.3) so an LLM reading a failing
// trace can reason about which transition's guard blocked its expected path.
pub use wos_core::eval::GuardEvaluation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PolicyApplication {
    pub policy_id: String,
    pub parameter_bindings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum Delta {
    /// Actual state differs from expected.
    StateMismatch {
        expected: String,
        actual: String,
        cause: Option<String>,
    },
    /// Guard evaluated false unexpectedly.
    GuardFalse {
        guard_id: String,
        inputs: serde_json::Value,
    },
    /// Policy application changed the outcome.
    PolicyOverride {
        policy_id: String,
        expected_without_policy: String,
        actual_with_policy: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Pass,
    Fail,
    Error,
}

impl ConformanceTrace {
    pub fn new(fixture_id: impl Into<String>, kernel_version: impl Into<String>) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            kernel_version: kernel_version.into(),
            steps: Vec::new(),
            outcome: Outcome::Pass,
        }
    }

    pub fn push_step(&mut self, step: TraceStep) {
        self.steps.push(step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_step(index: u32) -> TraceStep {
        TraceStep {
            step_index: index,
            event: Event {
                name: format!("event.{index}"),
                source_actor: Some("actor-1".into()),
                payload: Some(json!({ "foo": "bar" })),
            },
            state_before: "initial".into(),
            state_after: "next".into(),
            expected_state_after: None,
            guards_evaluated: Vec::new(),
            policies_applied: Vec::new(),
            delta: None,
        }
    }

    #[test]
    fn new_trace_has_empty_steps_and_pass_outcome() {
        let trace = ConformanceTrace::new("fixture-a", "1.0");
        assert_eq!(trace.fixture_id, "fixture-a");
        assert_eq!(trace.kernel_version, "1.0");
        assert!(trace.steps.is_empty());
        assert_eq!(trace.outcome, Outcome::Pass);
    }

    #[test]
    fn trace_round_trips_through_json() {
        let mut trace = ConformanceTrace::new("fixture-b", "1.2");
        trace.push_step(sample_step(1));
        trace.push_step(sample_step(2));

        let json_str = serde_json::to_string(&trace).expect("serialize");
        let deserialized: ConformanceTrace =
            serde_json::from_str(&json_str).expect("deserialize");

        assert_eq!(trace, deserialized);
    }

    #[test]
    fn trace_json_uses_camel_case_keys() {
        let mut trace = ConformanceTrace::new("fixture-c", "1.0");
        let step = TraceStep {
            step_index: 7,
            event: Event {
                name: "application.submitted".into(),
                source_actor: Some("applicant".into()),
                payload: None,
            },
            state_before: "initial".into(),
            state_after: "review".into(),
            expected_state_after: Some("review".into()),
            guards_evaluated: vec![GuardEvaluation {
                guard_id: "initial->review:application.submitted".into(),
                source_state: "initial".into(),
                target_state: "review".into(),
                event: "application.submitted".into(),
                expression: "amount > 0".into(),
                result: true,
                inputs: json!({ "caseFile": { "amount": 5 } }),
            }],
            policies_applied: vec![PolicyApplication {
                policy_id: "P-01".into(),
                parameter_bindings: json!({ "limit": 10 }),
            }],
            delta: None,
        };
        trace.push_step(step);

        let json_str = serde_json::to_string(&trace).expect("serialize");

        assert!(json_str.contains("\"fixtureId\""));
        assert!(json_str.contains("\"kernelVersion\""));
        assert!(json_str.contains("\"stepIndex\""));
        assert!(json_str.contains("\"stateBefore\""));
        assert!(json_str.contains("\"stateAfter\""));
        assert!(json_str.contains("\"expectedStateAfter\""));
        assert!(json_str.contains("\"guardsEvaluated\""));
        assert!(json_str.contains("\"policiesApplied\""));
        assert!(json_str.contains("\"sourceActor\""));
        assert!(json_str.contains("\"guardId\""));
        assert!(json_str.contains("\"policyId\""));
        assert!(json_str.contains("\"parameterBindings\""));
        // No snake_case leakage.
        assert!(!json_str.contains("\"fixture_id\""));
        assert!(!json_str.contains("\"step_index\""));
        assert!(!json_str.contains("\"state_before\""));
    }

    #[test]
    fn optional_fields_are_omitted_when_empty() {
        let mut trace = ConformanceTrace::new("fixture-d", "1.0");
        trace.push_step(TraceStep {
            step_index: 0,
            event: Event {
                name: "tick".into(),
                source_actor: None,
                payload: None,
            },
            state_before: "a".into(),
            state_after: "b".into(),
            expected_state_after: None,
            guards_evaluated: Vec::new(),
            policies_applied: Vec::new(),
            delta: None,
        });

        let json_str = serde_json::to_string(&trace).expect("serialize");

        assert!(!json_str.contains("\"delta\""));
        assert!(!json_str.contains("\"expectedStateAfter\""));
        assert!(!json_str.contains("\"guardsEvaluated\""));
        assert!(!json_str.contains("\"policiesApplied\""));
        assert!(!json_str.contains("\"sourceActor\""));
        assert!(!json_str.contains("\"payload\""));
    }

    #[test]
    fn delta_state_mismatch_round_trips_with_kind_tag() {
        let delta = Delta::StateMismatch {
            expected: "approved".into(),
            actual: "rejected".into(),
            cause: Some("guard false".into()),
        };

        let json_str = serde_json::to_string(&delta).expect("serialize");
        assert!(json_str.contains("\"kind\":\"stateMismatch\""));
        assert!(json_str.contains("\"expected\":\"approved\""));
        assert!(json_str.contains("\"actual\":\"rejected\""));

        let parsed: Delta = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(parsed, delta);
    }

    #[test]
    fn delta_guard_false_round_trips_with_kind_tag() {
        let delta = Delta::GuardFalse {
            guard_id: "G-02".into(),
            inputs: json!({ "benefit_amount": 520, "income_limit": 500 }),
        };

        let json_str = serde_json::to_string(&delta).expect("serialize");
        assert!(json_str.contains("\"kind\":\"guardFalse\""));
        assert!(json_str.contains("\"guardId\":\"G-02\""));

        let parsed: Delta = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(parsed, delta);
    }

    #[test]
    fn delta_policy_override_round_trips_with_kind_tag() {
        let delta = Delta::PolicyOverride {
            policy_id: "P-11".into(),
            expected_without_policy: "approved".into(),
            actual_with_policy: "rejected".into(),
        };

        let json_str = serde_json::to_string(&delta).expect("serialize");
        assert!(json_str.contains("\"kind\":\"policyOverride\""));
        assert!(json_str.contains("\"policyId\":\"P-11\""));
        assert!(json_str.contains("\"expectedWithoutPolicy\":\"approved\""));
        assert!(json_str.contains("\"actualWithPolicy\":\"rejected\""));

        let parsed: Delta = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(parsed, delta);
    }

    #[test]
    fn outcome_serializes_as_lowercase() {
        assert_eq!(serde_json::to_string(&Outcome::Pass).unwrap(), "\"pass\"");
        assert_eq!(serde_json::to_string(&Outcome::Fail).unwrap(), "\"fail\"");
        assert_eq!(serde_json::to_string(&Outcome::Error).unwrap(), "\"error\"");

        let parsed: Outcome = serde_json::from_str("\"pass\"").unwrap();
        assert_eq!(parsed, Outcome::Pass);
        let parsed: Outcome = serde_json::from_str("\"fail\"").unwrap();
        assert_eq!(parsed, Outcome::Fail);
        let parsed: Outcome = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(parsed, Outcome::Error);
    }
}
