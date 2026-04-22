// Rust guideline compliant 2026-02-21

//! CaseInstance serialization format (Runtime Companion S3).
//!
//! A CaseInstance captures the complete runtime state of a workflow
//! instance — enough to resume after a crash, migrate between
//! processors, or audit past behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::typeid;

/// A running workflow instance (Runtime Companion S3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseInstance {
    /// Globally unique instance identifier.
    pub instance_id: String,

    /// Canonical URL of the governing Kernel Document.
    pub definition_url: String,

    /// Version of the Kernel Document, pinned at creation.
    pub definition_version: String,

    /// Active leaf states in deterministic document order.
    pub configuration: Vec<String>,

    /// Current case file field values.
    pub case_state: serde_json::Value,

    /// Provenance log cursor.
    pub provenance_position: u64,

    /// Next task sequence number for stable task identifiers.
    #[serde(default)]
    pub next_task_sequence: u64,

    /// Pending timer state.
    pub timers: Vec<TimerState>,

    /// Active nonterminal task state.
    pub active_tasks: Vec<ActiveTask>,

    /// Saved history state configurations.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub history_store: HashMap<String, Vec<String>>,

    /// Active compensation logs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub compensation_logs: HashMap<String, CompensationLog>,

    /// Instance status.
    pub status: InstanceStatus,

    /// Events enqueued but not yet processed.
    #[serde(default)]
    pub pending_events: Vec<PendingEvent>,

    /// Governance runtime state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_state: Option<GovernanceState>,

    /// AI volume constraint counters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_counters: Option<VolumeCounters>,

    /// ISO 8601 creation timestamp.
    pub created_at: String,

    /// ISO 8601 last-modified timestamp.
    pub updated_at: String,

    /// Milestone identifiers that have already fired for this instance.
    ///
    /// Once a milestone condition first becomes true its id is recorded here,
    /// preventing it from firing again (Kernel S4.13 — once fired, stays fired).
    #[serde(default, skip_serializing_if = "std::collections::HashSet::is_empty")]
    pub fired_milestones: std::collections::HashSet<String>,

    /// Pending callback registrations awaiting inbound CloudEvents (NB.3).
    ///
    /// Keyed by the CloudEvents `subject` string used for correlation:
    /// `{instanceId}:{bindingId}:{invocationId}`. When a matching inbound
    /// event arrives its entry is removed and a `CallbackReceived` provenance
    /// record is emitted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub pending_callbacks: HashMap<String, PendingCallback>,

    /// Extension data (keys prefixed with `x-`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl CaseInstance {
    /// Mints a new case identifier.
    #[must_use]
    pub fn mint_id() -> String {
        typeid::mint_case_id()
    }

    /// Returns whether `value` already matches the reserved case TypeID shape.
    #[must_use]
    pub fn is_case_id(value: &str) -> bool {
        typeid::is_valid_type_id(value, Some(typeid::CASE_PREFIX))
    }
}

/// A callback registration that is waiting for a matching inbound CloudEvent (NB.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingCallback {
    /// Stable invocation identifier for this callback firing.
    pub invocation_id: String,

    /// Binding identifier that registered this callback.
    pub binding_id: String,

    /// ISO 8601 deadline after which this callback is considered expired, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_until: Option<String>,
}

/// Instance status (Runtime Companion S3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceStatus {
    Active,
    Suspended,
    Migrating,
    Completed,
    Terminated,
}

/// Pending timer state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    /// Timer identifier.
    pub timer_id: String,
    /// Absolute deadline (ISO 8601).
    pub deadline: String,
    /// Event to emit when fired.
    pub event: String,
    /// State that scoped this timer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_state: Option<String>,

    /// Original ISO 8601 duration string, if preserved by the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_iso: Option<String>,

    /// Original duration in milliseconds, if preserved by the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Simulated time in milliseconds when this timer was created.
    ///
    /// Preserved across serialization so `business_deadline_ms` can always
    /// compute the calendar-adjusted deadline from the original creation time,
    /// regardless of how many times the instance has been drained and persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
}

/// Active nonterminal task state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTask {
    /// Stable task identifier.
    pub task_id: String,

    /// Task definition reference.
    pub task_ref: String,

    /// Current nonterminal task status.
    pub status: ActiveTaskStatus,

    /// Assigned actor identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_actor: Option<String>,

    /// Kernel contract reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,

    /// Contract binding type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,

    /// Formspec Definition URL for Formspec-backed tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_url: Option<String>,

    /// Formspec Definition version for Formspec-backed tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping_ref: Option<String>,

    /// Task deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// Task impact level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<String>,

    /// Presentation context for a Formspec-backed task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<FormspecTaskContext>,

    /// Last Formspec task validation outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_validation_outcome: Option<ValidationOutcome>,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Extension data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Active task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveTaskStatus {
    Created,
    Assigned,
    Claimed,
    Delegated,
    Escalated,
}

/// Presentation context for a Formspec-backed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormspecTaskContext {
    /// Stable task identifier.
    pub task_id: String,

    /// Case instance identifier.
    pub instance_id: String,

    /// Kernel contract map key.
    pub contract_ref: String,

    /// Formspec Definition URL.
    pub definition_url: String,

    /// Formspec Definition version.
    pub definition_version: String,

    /// Task binding discriminator.
    pub binding: String,

    /// Assigned actor identifier.
    pub assigned_actor: String,

    /// Host-provided prefill data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_data: Option<serde_json::Value>,

    /// Mapping used to prefill the Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping_ref: Option<String>,

    /// Task deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// Effective task impact level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<String>,

    /// Extension data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Validation outcome for a Formspec-backed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationOutcome {
    /// Whether the Response envelope is valid.
    pub envelope_valid: bool,

    /// Whether the Response pin matches the task pin.
    pub pin_match: bool,

    /// Whether Definition validation passed.
    pub definition_valid: bool,

    /// WOS-level validation errors.
    pub errors: Vec<serde_json::Value>,

    /// Formspec-shaped validation results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_results: Option<Vec<serde_json::Value>>,
}

/// A pending event in the instance queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingEvent {
    /// Event name.
    pub event: String,
    /// Actor who submitted the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    /// Event payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Submission timestamp (ISO 8601).
    pub timestamp: String,
    /// Deduplication token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_token: Option<String>,
}

/// Governance runtime state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceState {
    /// Active delegation chains.
    #[serde(default)]
    pub active_delegations: Vec<ActiveDelegation>,
    /// Active holds.
    #[serde(default)]
    pub active_holds: Vec<ActiveHold>,
    /// Review protocol state (keyed by binding ID).
    #[serde(default)]
    pub review_state: HashMap<String, serde_json::Value>,
}

/// An active delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDelegation {
    /// Delegating actor.
    pub delegator_id: String,
    /// Receiving actor.
    pub delegate_id: String,
    /// Delegation scope.
    pub scope: String,
    /// Authority type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    /// Grant timestamp (ISO 8601).
    pub granted_at: String,
    /// Expiry timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// An active hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveHold {
    /// Hold type.
    pub hold_type: String,
    /// Start timestamp (ISO 8601).
    pub started_at: String,
    /// Expected end timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_end: Option<String>,
    /// Event that resumes the case.
    pub resume_trigger: String,
    /// State the instance was in when hold started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_state: Option<String>,
}

/// AI volume constraint counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCounters {
    /// Per-agent hourly counters.
    #[serde(default)]
    pub hourly: HashMap<String, VolumeCounter>,
    /// Per-agent daily counters.
    #[serde(default)]
    pub daily: HashMap<String, VolumeCounter>,
}

/// A single volume counter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCounter {
    /// Count in the current window.
    pub count: u64,
    /// Window start (ISO 8601).
    pub window_start: String,
}

/// Compensation log for a compensable scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompensationLog {
    /// Scope identifier.
    pub scope_id: String,
    /// Entries in forward completion order.
    pub entries: Vec<CompensationEntry>,
}

/// A compensation log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompensationEntry {
    /// Original action ID.
    pub action_id: String,
    /// Original action type.
    pub action_type: String,
    /// Compensating action.
    pub compensating_action: serde_json::Value,
    /// Completion timestamp (ISO 8601).
    pub completed_at: String,
    /// Persisted output of the original action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}
