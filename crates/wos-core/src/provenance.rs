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
    /// Timer fired beyond its tolerance window (LCD S6.6, Runtime S7.2).
    ToleranceViolation,

    // ── Deontic enforcement (AI S4) ────────────────────────────────
    /// A deontic constraint was violated (AI S4.2–S4.4).
    DeonticViolation,
    /// Deontic evaluation order record (AI S4.6).
    DeonticEvaluation,
    /// Resolved effective action from multiple violations (AI S4.6).
    DeonticResolution,
    /// Deontic constraint bypass with rationale (AI S4.7).
    DeonticBypass,
    /// Rights violation not attributed to agent (AI S4.5).
    RightsViolation,
    /// Consistency check contradiction (AI S4.7).
    ConsistencyViolation,

    // ── Autonomy (AI S5) ───────────────────────────────────────────
    /// Agent attempted to override a human decision (AI S3.7).
    AutonomyViolation,
    /// Autonomy level was capped by impact level or calibration (AI S5.3).
    AutonomyCapped,
    /// Effective autonomy computed from multiple sources (AI S5.3).
    AutonomyComputed,
    /// Assistive agent required human confirmation task (AI S5.3).
    HumanTaskCreated,
    /// Tool governance violation (AdvGov S6.1).
    ToolViolation,
    /// Escalation pending human approval (AI S5.4).
    EscalationPending,
    /// Autonomy demotion applied (AI S5.5).
    AutonomyDemotion,

    // ── Confidence (AI S7) ─────────────────────────────────────────
    /// Confidence violation — missing, uncalibrated, or below floor (AI S7).
    ConfidenceViolation,
    /// Confidence decay applied (AI S7.5).
    ConfidenceDecay,
    /// Cumulative confidence below threshold (AI S7.7).
    CumulativeConfidenceViolation,
    /// Session paused due to confidence threshold (AdvGov S5.4).
    SessionPaused,
    /// Ground truth label recorded from human review (AdvGov S9.3).
    GroundTruthLabel,

    // ── Agent lifecycle (AI S3, S6) ────────────────────────────────
    AgentOutput,
    ActorTypeViolation,
    AgentProvenanceAnnotation,
    AgentVersionChange,
    NarrativeTierRecorded,
    ConstraintTamperBlocked,
    DriftReclassification,
    AgentStateTransition,
    ProxyInvocation,
    DispositiveViolation,

    // ── Fallback (AI S8) ───────────────────────────────────────────
    FallbackTriggered,
    FallbackAttempt,
    FallbackTerminal,

    // ── Due process (WG S4, S6, S7) ────────────────────────────────
    NoticeSent,
    SeparationViolation,
    AppealFiled,
    ProtocolViolation,
    IndependentFirstEnforced,
    SamplingDecision,
    OverrideViolation,
    OverrideRecorded,

    // ── Pipeline (WG S8) ───────────────────────────────────────────
    PipelineStageCompleted,
    PipelineRiskProfile,
    PipelineRejection,
    TaskCreated,
    TaskPresented,
    TaskDismissed,
    TaskDraftPersisted,
    TaskResponseSubmitted,
    TaskResponseRejected,
    DataMapping,
    TaskCompleted,
    TaskFailed,
    TaskSkipped,
    ParameterResolved,

    // ── Compensation (Kernel S9.8) ─────────────────────────────────
    CompensationLogEntry,
    CompensationExecuted,
    CompensationScopeBoundary,

    // ── Delegation (WG S9) ─────────────────────────────────────────
    DelegationViolation,

    // ── Durability (Kernel S10) ────────────────────────────────────
    InstanceResumed,
    StepResultPersisted,
    IdempotencyDedup,
    InstanceMigrated,
    ContractValidation,
    HistoryCleared,

    // ── DCR (Advanced Governance) ──────────────────────────────────
    DcrActivityExecuted,
    DcrRelationEvaluated,
    DcrResolutionError,
    ZoneSatisfied,
    EquityAlert,

    // ── Verification (Advanced Governance) ─────────────────────────
    VerificationReportProduced,
    ImmutabilityViolation,
    ActivationBlocked,

    // ── Sidecar (Business Calendar, Notification) ──────────────────
    CalendarIgnored,
    NotificationSuppressed,

    // ── Relationship provenance (Kernel S7) ────────────────────────
    RelationshipChanged,

    // ── Milestones (Kernel S4.13) ──────────────────────────────────
    /// A milestone condition became true for the first time (Kernel S4.13).
    ///
    /// `data` carries `{"milestoneId": "<id>"}`.
    MilestoneFired,

    // ── CloudEvents bindings (Integration Profile NB.3) ───────────
    /// An outbound CloudEvent was emitted by an `event-emit` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventEmitted,

    /// An inbound CloudEvent was successfully consumed by an `event-consume` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventConsumed,

    /// An inbound CloudEvent resolved a pending callback registered by a `callback` binding.
    ///
    /// `data` carries the full CloudEvent envelope and the `subject` used for correlation.
    CallbackReceived,

    /// A `callback` binding fired and is waiting for a matching inbound CloudEvent.
    ///
    /// `data` carries `{"subject": "<subject>", "bindingId": "<id>", "expectedUntil": "<iso>"}`.
    CallbackPending,

    // ── Arazzo / Tool / Policy-engine bindings (Integration Profile NB.4) ─
    /// A single step of an Arazzo multi-step sequence completed (or failed).
    ///
    /// `data` carries `{"stepId": "<id>", "outcome": "ok"|"failed", "durationMs": <n>, ...}`.
    ArazzoStep,

    /// A non-HTTP tool binding was invoked and produced a result.
    ///
    /// `data` carries `{"toolId": "<id>", "outcome": "ok"|"failed", ...}`.
    ToolInvoked,

    /// An external policy engine evaluated a request and returned a decision.
    ///
    /// `data` carries `{"decision": "allow"|"deny"|"indeterminate",
    /// "reasonsCount": <n>, "obligationsCount": <n>, ...}`.
    PolicyDecision,
}

/// A single provenance record.
///
/// Records carry an RFC 3339 / ISO 8601 `timestamp` populated by the runtime
/// (or test harness) at the moment the record is appended to the instance log.
/// Constructors leave the field empty; the runtime stamps any empty timestamp
/// with the active clock before persisting the record (see
/// `wos_runtime::stamp_provenance`). Records produced in unit tests that never
/// reach the runtime may carry an empty `timestamp` — exporters and
/// downstream consumers (PROV-O, XES, OCEL) MUST treat an empty value as
/// "unknown" rather than emitting it verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecord {
    /// Record type.
    pub record_kind: ProvenanceKind,

    /// RFC 3339 / ISO 8601 timestamp set by the runtime when the record is
    /// appended to a log. Empty until stamped.
    #[serde(default)]
    pub timestamp: String,

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
    pub fn state_transition(from: &str, to: &str, event: &str, actor_id: Option<&str>) -> Self {
        Self {
            record_kind: ProvenanceKind::StateTransition,
            timestamp: String::new(),
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
            timestamp: String::new(),
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
            timestamp: String::new(),
            actor_id: actor_id.map(String::from),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "path": path,
                "newValue": new_value,
                "lifecycleState": lifecycle_state,
                "viaExplicitAction": true,
            })),
        }
    }

    /// Create a timer created record (Lifecycle Detail S6.7).
    pub fn timer_created(timer_id: &str, duration: &str, fires_event: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::TimerCreated,
            timestamp: String::new(),
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
            timestamp: String::new(),
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
            timestamp: String::new(),
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

    /// Create a state-entry record.
    pub fn state_entered(state: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::OnEntry,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: Some(state.to_string()),
            event: None,
            data: Some(serde_json::json!({ "state": state })),
        }
    }

    /// Create an onEntry action record.
    pub fn on_entry(state: &str, action_type: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::OnEntry,
            timestamp: String::new(),
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
            timestamp: String::new(),
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
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: Some(state.to_string()),
            event: None,
            data: Some(serde_json::json!({ "actionType": action_type })),
        }
    }

    /// Create a timer tolerance violation record (LCD S6.6, Runtime S7.2).
    pub fn tolerance_violation(
        timer_id: &str,
        duration_iso: &str,
        max_tolerance_iso: &str,
    ) -> Self {
        Self {
            record_kind: ProvenanceKind::ToleranceViolation,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "timerId": timer_id,
                "duration": duration_iso,
                "maxTolerance": max_tolerance_iso,
            })),
        }
    }

    /// Create a history-cleared record.
    pub fn history_cleared(state: &str, reason: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::HistoryCleared,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "state": state,
                "reason": reason,
            })),
        }
    }

    /// Create an invalid-duration warning record.
    pub fn invalid_duration(raw_duration: &str, timer_id: &str) -> Self {
        Self {
            record_kind: ProvenanceKind::InvalidDuration,
            timestamp: String::new(),
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

    /// Create a task lifecycle record emitted by the runtime layer.
    pub fn task_lifecycle(
        record_kind: ProvenanceKind,
        task_id: &str,
        actor_id: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            record_kind,
            timestamp: String::new(),
            actor_id: actor_id.map(String::from),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(match data {
                Some(extra) => {
                    let mut object = serde_json::Map::new();
                    object.insert(
                        "taskId".to_string(),
                        serde_json::Value::String(task_id.to_string()),
                    );
                    object.insert("details".to_string(), extra);
                    serde_json::Value::Object(object)
                }
                None => serde_json::json!({ "taskId": task_id }),
            }),
        }
    }

    /// Create a contract validation record emitted by runtime task flows.
    pub fn contract_validation(
        task_id: &str,
        actor_id: Option<&str>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            record_kind: ProvenanceKind::ContractValidation,
            timestamp: String::new(),
            actor_id: actor_id.map(String::from),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "taskId": task_id,
                "details": data,
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
        if !self.timestamp.is_empty() {
            write!(f, " at={}", self.timestamp)?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_leave_timestamp_empty_for_runtime_to_stamp() {
        let record = ProvenanceRecord::state_transition("a", "b", "ev", Some("actor"));
        assert!(record.timestamp.is_empty());
    }

    #[test]
    fn serializes_timestamp_field_when_populated() {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
        record.timestamp = "1970-01-01T00:00:00Z".to_string();

        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["timestamp"], "1970-01-01T00:00:00Z");
        assert_eq!(json["recordKind"], "stateTransition");
        assert_eq!(json["fromState"], "a");
        assert_eq!(json["toState"], "b");
    }

    #[test]
    fn serializes_empty_timestamp_explicitly() {
        // An empty timestamp surfaces a missed stamping site to consumers
        // rather than vanishing silently.
        let record = ProvenanceRecord::state_transition("a", "b", "ev", None);
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["timestamp"], "");
    }

    #[test]
    fn round_trip_preserves_timestamp() {
        let mut original = ProvenanceRecord::case_state_mutation(
            "/path",
            &serde_json::json!(42),
            Some("actor"),
            "active",
        );
        original.timestamp = "2026-04-15T12:34:56Z".to_string();

        let json = serde_json::to_string(&original).expect("serialize");
        let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.timestamp, "2026-04-15T12:34:56Z");
        assert_eq!(restored.record_kind, ProvenanceKind::CaseStateMutation);
        assert_eq!(restored.actor_id.as_deref(), Some("actor"));
    }

    #[test]
    fn deserializes_legacy_records_without_timestamp_field() {
        // Older fixtures predate the timestamp field; they must still load
        // and yield an empty timestamp the runtime can stamp on replay.
        let legacy = serde_json::json!({
            "recordKind": "stateTransition",
            "fromState": "a",
            "toState": "b",
            "event": "ev"
        });
        let record: ProvenanceRecord = serde_json::from_value(legacy).expect("deserialize");
        assert!(record.timestamp.is_empty());
    }
}
