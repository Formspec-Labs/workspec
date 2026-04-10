// Rust guideline compliant 2026-02-21

//! CaseInstance serialization format (Runtime Companion S3).
//!
//! A CaseInstance captures the complete runtime state of a workflow
//! instance — enough to resume after a crash, migrate between
//! processors, or audit past behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    /// Pending timer state.
    pub timers: Vec<TimerState>,

    /// Saved history state configurations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_store: Option<HashMap<String, Vec<String>>>,

    /// Active compensation logs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation_logs: Option<HashMap<String, CompensationLog>>,

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

    /// Extension data (keys prefixed with `x-`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
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
