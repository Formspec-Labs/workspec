// Rust guideline compliant 2026-02-21

//! Provenance recording for workflow execution.
//!
//! Every action that changes lifecycle or case state produces a provenance
//! record (Kernel S8). The provenance log is append-only.

use serde::{Deserialize, Serialize};

/// Provenance record type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProvenanceKind {
    /// Lifecycle state transition.
    StateTransition,
    /// Event that matched no transition (Kernel S4.9).
    UnmatchedEvent,
    /// Case state field mutation (Kernel S5.4).
    CaseStateMutation,
    /// Timer created (Lifecycle Detail S6.7).
    TimerCreated,
    /// Timer fired (Lifecycle Detail S6.7).
    TimerFired,
    /// Timer cancelled (Lifecycle Detail S6.7).
    TimerCancelled,
    /// An `onEntry` lifecycle hook executed.
    OnEntry,
    /// An `onExit` lifecycle hook executed.
    OnExit,
    /// Action executed during onEntry, onExit, or transition.
    ActionExecuted,
    /// Duration string could not be parsed; timer deadline set to zero.
    InvalidDuration,
}

/// A single provenance record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecord {
    /// Record type.
    pub record_kind: ProvenanceKind,

    /// Actor who triggered the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    /// Source state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_state: Option<String>,

    /// Target state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_state: Option<String>,

    /// Triggering event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,

    /// Additional context data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ProvenanceRecord {
    /// Create a state transition record.
    pub fn state_transition(
        from: &str,
        to: &str,
        event: &str,
        actor_id: Option<&str>,
    ) -> Self {
        Self {
            record_kind: ProvenanceKind::StateTransition,
            actor_id: actor_id.map(String::from),
            from_state: Some(from.to_string()),
            to_state: Some(to.to_string()),
            event: Some(event.to_string()),
            data: None,
        }
    }

    /// Create an unmatched event record (Kernel S4.9).
    pub fn unmatched_event(event: &str, actor_id: Option<&str>) -> Self {
        Self {
            record_kind: ProvenanceKind::UnmatchedEvent,
            actor_id: actor_id.map(String::from),
            from_state: None,
            to_state: None,
            event: Some(event.to_string()),
            data: None,
        }
    }

    /// Create a case state mutation record (Kernel S5.4).
    pub fn case_state_mutation(
        path: &str,
        new_value: &serde_json::Value,
        actor_id: Option<&str>,
        lifecycle_state: &str,
    ) -> Self {
        Self {
            record_kind: ProvenanceKind::CaseStateMutation,
            actor_id: actor_id.map(String::from),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "path": path,
                "newValue": new_value,
                "lifecycleState": lifecycle_state,
            })),
        }
    }

    /// Create a timer created record (Lifecycle Detail S6.7).
    pub fn timer_created(timer_id: &str, duration: &str, fires_event: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::TimerCreated,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "timerId": timer_id,
                "duration": duration,
                "firesEvent": fires_event,
            })),
        }
    }

    /// Create a timer fired record (Lifecycle Detail S6.7).
    pub fn timer_fired(timer_id: &str, fires_event: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::TimerFired,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "timerId": timer_id,
                "firesEvent": fires_event,
            })),
        }
    }

    /// Create a timer cancelled record (Lifecycle Detail S6.7).
    pub fn timer_cancelled(timer_id: &str, reason: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::TimerCancelled,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "timerId": timer_id,
                "reason": reason,
            })),
        }
    }

    /// Create an onEntry action record.
    pub fn on_entry(state: &str, action_type: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::OnEntry,
            actor_id: None,
            from_state: None,
            to_state: Some(state.to_string()),
            event: None,
            data: Some(serde_json::json!({ "actionType": action_type })),
        }
    }

    /// Create an onExit action record.
    pub fn on_exit(state: &str, action_type: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::OnExit,
            actor_id: None,
            from_state: Some(state.to_string()),
            to_state: None,
            event: None,
            data: Some(serde_json::json!({ "actionType": action_type })),
        }
    }

    /// Create a generic action-executed record.
    pub fn action_executed(state: &str, action_type: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::ActionExecuted,
            actor_id: None,
            from_state: None,
            to_state: Some(state.to_string()),
            event: None,
            data: Some(serde_json::json!({ "actionType": action_type })),
        }
    }

    /// Create an invalid-duration warning record.
    pub fn invalid_duration(raw_duration: &str, timer_id: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::InvalidDuration,
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "rawDuration": raw_duration,
                "timerId": timer_id,
                "note": "unrecognized ISO 8601 duration; deadline set to zero (fires immediately)",
            })),
        }
    }
}

/// Append-only provenance log.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceLog {
    records: Vec<ProvenanceRecord>,
}

impl ProvenanceLog {
    /// Append a record.
    pub fn push(&mut self, record: ProvenanceRecord) {
        self.records.push(record);
    }

    /// All records in order.
    pub fn records(&self) -> &[ProvenanceRecord] {
        &self.records
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl std::fmt::Display for ProvenanceRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.record_kind)?;
        if let Some(from) = &self.from_state {
            write!(f, " from={from}")?;
        }
        if let Some(to) = &self.to_state {
            write!(f, " to={to}")?;
        }
        if let Some(event) = &self.event {
            write!(f, " event={event}")?;
        }
        Ok(())
    }
}
