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

    /// Provenance tier: `"facts"`, `"reasoning"`, `"counterfactual"`, or
    /// `"narrative"` (SP §5.4, §6.5). Defaults to `"facts"` at construction;
    /// populated by the runtime tier classifier before persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_layer: Option<String>,

    /// Actor type: `"human"`, `"system"`, or `"agent"` (SP §5.3, §5.5, §6.3).
    /// Populated at construction from the kernel `ActorKind` registry lookup
    /// (or from the AI Integration agent registry for `"agent"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,

    /// Canonical lifecycle state at action time, distinct from `from_state`
    /// (which carries the pre-transition label). Maps to `wos:atLifecycleState`
    /// (PROV-O §5.3) and `wos:lifecycleState` (XES §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,

    /// Version of the governing WOS Kernel Document (SP §5.3, §6.3).
    /// Populated from the workflow definition's `version` field at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<String>,

    /// Input entity references used by this activity (SP §5.3 `prov:used`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,

    /// Output entity references generated by this activity (SP §5.3
    /// `prov:wasGeneratedBy` inverse).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,

    /// Tamper-detection digest for the inputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,

    /// Tamper-detection digest for the outputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
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

    /// Confirm every constructor zero-initializes the eight new fields the
    /// runtime / exporter is expected to populate later. The push-stamped
    /// design (documented for `timestamp`) extends to these fields: the
    /// construction site leaves them unset and a downstream pass fills them
    /// in before persistence.
    #[test]
    fn new_fields_default_to_none_or_empty_vec() {
        fn assert_zero_defaults(record: &ProvenanceRecord) {
            assert!(record.audit_layer.is_none());
            assert!(record.actor_type.is_none());
            assert!(record.lifecycle_state.is_none());
            assert!(record.definition_version.is_none());
            assert!(record.inputs.is_empty());
            assert!(record.outputs.is_empty());
            assert!(record.input_digest.is_none());
            assert!(record.output_digest.is_none());
        }

        assert_zero_defaults(&ProvenanceRecord::state_transition("a", "b", "ev", None));
        assert_zero_defaults(&ProvenanceRecord::unmatched_event("ev", None));
        assert_zero_defaults(&ProvenanceRecord::case_state_mutation(
            "/p",
            &serde_json::json!(1),
            None,
            "active",
        ));
        assert_zero_defaults(&ProvenanceRecord::timer_created("t", "PT1S", "fire"));
        assert_zero_defaults(&ProvenanceRecord::timer_fired("t", "fire"));
        assert_zero_defaults(&ProvenanceRecord::timer_cancelled("t", "reason"));
        assert_zero_defaults(&ProvenanceRecord::state_entered("s"));
        assert_zero_defaults(&ProvenanceRecord::on_entry("s", "action"));
        assert_zero_defaults(&ProvenanceRecord::on_exit("s", "action"));
        assert_zero_defaults(&ProvenanceRecord::action_executed("s", "action"));
        assert_zero_defaults(&ProvenanceRecord::tolerance_violation("t", "PT1S", "PT2S"));
        assert_zero_defaults(&ProvenanceRecord::history_cleared("s", "reason"));
        assert_zero_defaults(&ProvenanceRecord::invalid_duration("raw", "t"));
        assert_zero_defaults(&ProvenanceRecord::task_lifecycle(
            ProvenanceKind::TaskCreated,
            "task-1",
            None,
            None,
        ));
        assert_zero_defaults(&ProvenanceRecord::contract_validation(
            "task-1",
            None,
            serde_json::json!({}),
        ));
    }

    /// All eight new fields round-trip through serde when populated.
    #[test]
    fn round_trips_all_eight_new_fields() {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("actor"));
        record.audit_layer = Some("reasoning".to_string());
        record.actor_type = Some("agent".to_string());
        record.lifecycle_state = Some("under-review".to_string());
        record.definition_version = Some("1.2.3".to_string());
        record.inputs = vec!["entity:application".to_string(), "entity:evidence".to_string()];
        record.outputs = vec!["entity:decision".to_string()];
        record.input_digest = Some("sha256:deadbeef".to_string());
        record.output_digest = Some("sha256:cafebabe".to_string());

        let json = serde_json::to_string(&record).expect("serialize");
        let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.audit_layer.as_deref(), Some("reasoning"));
        assert_eq!(restored.actor_type.as_deref(), Some("agent"));
        assert_eq!(restored.lifecycle_state.as_deref(), Some("under-review"));
        assert_eq!(restored.definition_version.as_deref(), Some("1.2.3"));
        assert_eq!(
            restored.inputs,
            vec![
                "entity:application".to_string(),
                "entity:evidence".to_string(),
            ]
        );
        assert_eq!(restored.outputs, vec!["entity:decision".to_string()]);
        assert_eq!(restored.input_digest.as_deref(), Some("sha256:deadbeef"));
        assert_eq!(restored.output_digest.as_deref(), Some("sha256:cafebabe"));
    }

    /// Legacy records that predate these fields MUST still deserialize,
    /// defaulting each new field to its zero value.
    #[test]
    fn deserializes_legacy_record_missing_new_fields() {
        let legacy = serde_json::json!({
            "recordKind": "stateTransition",
            "timestamp": "2026-04-15T12:00:00Z",
            "fromState": "a",
            "toState": "b",
            "event": "ev"
        });
        let record: ProvenanceRecord = serde_json::from_value(legacy).expect("deserialize");
        assert!(record.audit_layer.is_none());
        assert!(record.actor_type.is_none());
        assert!(record.lifecycle_state.is_none());
        assert!(record.definition_version.is_none());
        assert!(record.inputs.is_empty());
        assert!(record.outputs.is_empty());
        assert!(record.input_digest.is_none());
        assert!(record.output_digest.is_none());
    }
}
