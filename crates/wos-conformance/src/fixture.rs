// Rust guideline compliant 2026-02-21

//! Conformance test fixture format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_binding() -> Option<String> {
    Some("formspec".to_string())
}

/// A conformance test fixture declaring documents, events, and expectations.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConformanceFixture {
    /// Contract binding to use for this fixture.
    ///
    /// `"formspec"` (default) routes through the `FormspecBinding` adapter
    /// with `FixtureFormspecProcessor`; `"conformance"` is accepted as an
    /// alias for backward compatibility.
    #[serde(default = "default_binding")]
    pub binding: Option<String>,

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

    /// Inline document contents keyed by role.
    ///
    /// When a document path in `documents` is `"inline"`, the engine
    /// reads the content from this map instead of the filesystem.
    #[serde(default)]
    pub inline_documents: HashMap<String, serde_json::Value>,

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

    /// Canned definition-validation errors for the `FixtureFormspecProcessor`.
    ///
    /// When `binding` is `"formspec"` and this field is non-empty, the
    /// processor's `validate_definition` method returns these errors instead
    /// of `None`, simulating a definition that fails validation.
    #[serde(default)]
    pub definition_errors: Vec<serde_json::Value>,

    /// Canned response body returned by the stub external service for all
    /// `invokeService` calls.
    ///
    /// When absent the stub returns `null`.  Use this field when a fixture
    /// needs an integration output binding to populate case-state fields
    /// (e.g. to trigger a milestone whose condition depends on the response).
    #[serde(default)]
    pub service_response: Option<serde_json::Value>,
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

    /// When present, this entry submits a task response instead of dispatching
    /// a lifecycle event. The `task_ref` is resolved to the active task's
    /// generated `task_id` at runtime.
    #[serde(default)]
    pub task_submission: Option<TaskSubmission>,
}

/// A task response submission within a fixture event sequence.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskSubmission {
    /// Logical task reference (matches `taskRef` in the kernel's createTask action).
    pub task_ref: String,

    /// Response envelope to submit.
    pub response: serde_json::Value,

    /// Idempotency token for replay protection.
    #[serde(default)]
    pub idempotency_token: Option<String>,
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
