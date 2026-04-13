// Rust guideline compliant 2026-02-21

//! Conformance test fixture format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A conformance test fixture declaring documents, events, and expectations.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConformanceFixture {
    /// Fixture identifier (e.g., "K-011-determinism").
    pub id: String,

    /// LINT-MATRIX rule this fixture tests.
    pub rule: String,

    /// Backlog batch that introduced this fixture.
    ///
    /// Profile aggregation uses batch membership so profile evidence tracks the
    /// fixture inventory directly instead of duplicating rule lists in tests.
    #[serde(default)]
    pub batch: Option<u8>,

    /// Human-readable description.
    pub description: String,

    /// Document paths keyed by role (kernel, governance, ai, etc.).
    pub documents: HashMap<String, String>,

    /// Optional initial case state to pre-seed before the event sequence.
    ///
    /// Keys are case field names (without the "caseFile." prefix).
    /// This allows fixtures to test guard-dependent transitions without
    /// requiring a `setData` action in the event sequence.
    #[serde(default)]
    pub initial_case_state: std::collections::HashMap<String, serde_json::Value>,

    /// Ordered sequence of events to feed into the workflow.
    #[serde(default)]
    pub event_sequence: Vec<EventEntry>,

    /// Expected state transitions in order.
    #[serde(default)]
    pub expected_transitions: Vec<ExpectedTransition>,

    /// Expected provenance records (partial match).
    #[serde(default)]
    pub expected_provenance: Vec<serde_json::Value>,

    /// Expected diagnostic errors (for negative tests).
    #[serde(default)]
    pub expected_errors: Vec<String>,

    /// Expected contract validation outcomes (for Formspec coprocessor tests).
    ///
    /// Each entry maps a contract reference to whether validation should
    /// pass or fail, allowing fixtures to test the `StubValidator` integration
    /// path where task submissions are validated against Formspec Definitions.
    #[serde(default)]
    pub contract_outcomes: HashMap<String, ContractOutcome>,
}

/// Expected outcome of a Formspec contract validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractOutcome {
    /// Whether validation should pass (`true`) or fail (`false`).
    pub valid: bool,

    /// Expected validation errors (when `valid` is `false`).
    #[serde(default)]
    pub errors: Vec<String>,
}

/// An event in the test sequence.
#[derive(Debug, Serialize, Deserialize)]
pub struct EventEntry {
    /// Event name.
    pub event: String,

    /// Actor performing this event.
    #[serde(default)]
    pub actor: Option<String>,

    /// Event data payload.
    #[serde(default)]
    pub data: Option<serde_json::Value>,

    /// Delay before this event (ISO 8601 duration, for timer tests).
    #[serde(default)]
    pub delay: Option<String>,
}

/// An expected state transition.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExpectedTransition {
    /// Source state.
    pub from: String,

    /// Target state.
    pub to: String,

    /// Triggering event.
    pub event: String,
}
