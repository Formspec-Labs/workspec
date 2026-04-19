// Rust guideline compliant 2026-02-21

//! Provenance recording for workflow execution.
//!
//! Every action that changes lifecycle or case state produces a provenance
//! record (Kernel S8). The provenance log is append-only.

use serde::{Deserialize, Serialize};

/// Canonical case-file snapshot captured for a determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFileSnapshot {
    /// Case-file value observed at determination fire time.
    pub value: serde_json::Value,

    /// JCS-style canonical JSON representation of `value`.
    pub jcs_canonical: String,

    /// SHA-256 hex digest of `jcs_canonical`.
    pub sha256: String,
}

impl CaseFileSnapshot {
    /// Create a canonical snapshot from case state.
    pub fn from_case_state(state: &serde_json::Value) -> Self {
        let jcs_canonical = serde_json_canonicalizer::to_string(state)
            .expect("serde_json::Value serializes to JCS");
        let sha256 = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(jcs_canonical.as_bytes()))
        };
        Self {
            value: state.clone(),
            jcs_canonical,
            sha256,
        }
    }
}

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

impl ProvenanceKind {
    /// Whether this kind represents a governance / AI policy or rule that
    /// applied during event processing.
    ///
    /// Used by the runtime to decide which records should have their `event`
    /// field stamped with the drain's processed event (for records whose
    /// constructors left `event = None` — the governance layer does this
    /// uniformly today), and by the conformance trace builder to decide
    /// which records contribute a `PolicyApplication` entry on a trace step.
    ///
    /// Semantics are "applied" not "violated". Violation-shaped kinds
    /// (`DeonticViolation`, `AutonomyViolation`, `ConfidenceViolation`, ...)
    /// signal that a rule FAILED, not that one applied, so they are
    /// intentionally excluded. `DeonticBypass` and `AutonomyDemotion` are
    /// semantically "policy overridden / demoted", not accept-and-fire —
    /// they are included because downstream teaching-signal consumers want
    /// to see that an override/demotion DID happen (with its rationale)
    /// when reasoning about a workflow's actual behaviour. Consumers can
    /// filter them out if they specifically want accept-and-fire semantics.
    pub fn is_policy_application(&self) -> bool {
        matches!(
            self,
            ProvenanceKind::DeonticEvaluation
                | ProvenanceKind::DeonticResolution
                | ProvenanceKind::DeonticBypass
                | ProvenanceKind::AutonomyComputed
                | ProvenanceKind::AutonomyDemotion
                | ProvenanceKind::OverrideRecorded
                | ProvenanceKind::PolicyDecision
                | ProvenanceKind::PipelineRiskProfile
        )
    }
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

    /// Semantic tags copied from the firing transition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_tags: Vec<String>,

    /// Case-file snapshot used by a determination-tagged transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_file_snapshot: Option<CaseFileSnapshot>,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
        }
    }

    /// Create a state transition record with transition tags and an optional
    /// determination snapshot.
    pub fn tagged_state_transition(
        from: &str,
        to: &str,
        event: &str,
        actor_id: Option<&str>,
        transition_tags: &[String],
        case_file_snapshot: Option<CaseFileSnapshot>,
    ) -> Self {
        let mut record = Self::state_transition(from, to, event, actor_id);
        record.transition_tags = transition_tags.to_vec();
        record.case_file_snapshot = case_file_snapshot;
        record
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
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
            transition_tags: Vec::new(),
            case_file_snapshot: None,
        }
    }
}

/// Classify a provenance record kind into its tier (SP §5.4, §6.5).
///
/// The tier for a record is deterministic from its `ProvenanceKind`. Only
/// `NarrativeTierRecorded` maps to `"narrative"` today — every other variant
/// is a factual observation (`"facts"`). The `"reasoning"` and
/// `"counterfactual"` tiers (SP §5.4) are reserved for Layer 1 injection
/// paths not yet wired to a dedicated `ProvenanceKind` variant.
///
/// The match is written exhaustively (no wildcard arm) so that adding a new
/// `ProvenanceKind` variant upstream forces the author to consciously decide
/// its tier — silent mis-classification via a wildcard fallback is the exact
/// failure mode this indirection is here to prevent.
pub fn audit_layer_for_kind(kind: ProvenanceKind) -> &'static str {
    match kind {
        // Narrative tier (SP §5.4): the sole variant carrying narrative-layer
        // annotations today.
        ProvenanceKind::NarrativeTierRecorded => "narrative",

        // Facts tier (SP §5.4): every other variant records an observable
        // fact about workflow execution. Deliberately enumerated rather than
        // collapsed into `_` so a new variant triggers a compile error until
        // its tier is assigned.
        ProvenanceKind::StateTransition
        | ProvenanceKind::UnmatchedEvent
        | ProvenanceKind::CaseStateMutation
        | ProvenanceKind::TimerCreated
        | ProvenanceKind::TimerFired
        | ProvenanceKind::TimerCancelled
        | ProvenanceKind::OnEntry
        | ProvenanceKind::OnExit
        | ProvenanceKind::ActionExecuted
        | ProvenanceKind::InvalidDuration
        | ProvenanceKind::ToleranceViolation
        | ProvenanceKind::DeonticViolation
        | ProvenanceKind::DeonticEvaluation
        | ProvenanceKind::DeonticResolution
        | ProvenanceKind::DeonticBypass
        | ProvenanceKind::RightsViolation
        | ProvenanceKind::ConsistencyViolation
        | ProvenanceKind::AutonomyViolation
        | ProvenanceKind::AutonomyCapped
        | ProvenanceKind::AutonomyComputed
        | ProvenanceKind::HumanTaskCreated
        | ProvenanceKind::ToolViolation
        | ProvenanceKind::EscalationPending
        | ProvenanceKind::AutonomyDemotion
        | ProvenanceKind::ConfidenceViolation
        | ProvenanceKind::ConfidenceDecay
        | ProvenanceKind::CumulativeConfidenceViolation
        | ProvenanceKind::SessionPaused
        | ProvenanceKind::GroundTruthLabel
        | ProvenanceKind::AgentOutput
        | ProvenanceKind::ActorTypeViolation
        | ProvenanceKind::AgentProvenanceAnnotation
        | ProvenanceKind::AgentVersionChange
        | ProvenanceKind::ConstraintTamperBlocked
        | ProvenanceKind::DriftReclassification
        | ProvenanceKind::AgentStateTransition
        | ProvenanceKind::ProxyInvocation
        | ProvenanceKind::DispositiveViolation
        | ProvenanceKind::FallbackTriggered
        | ProvenanceKind::FallbackAttempt
        | ProvenanceKind::FallbackTerminal
        | ProvenanceKind::NoticeSent
        | ProvenanceKind::SeparationViolation
        | ProvenanceKind::AppealFiled
        | ProvenanceKind::ProtocolViolation
        | ProvenanceKind::IndependentFirstEnforced
        | ProvenanceKind::SamplingDecision
        | ProvenanceKind::OverrideViolation
        | ProvenanceKind::OverrideRecorded
        | ProvenanceKind::PipelineStageCompleted
        | ProvenanceKind::PipelineRiskProfile
        | ProvenanceKind::PipelineRejection
        | ProvenanceKind::TaskCreated
        | ProvenanceKind::TaskPresented
        | ProvenanceKind::TaskDismissed
        | ProvenanceKind::TaskDraftPersisted
        | ProvenanceKind::TaskResponseSubmitted
        | ProvenanceKind::TaskResponseRejected
        | ProvenanceKind::DataMapping
        | ProvenanceKind::TaskCompleted
        | ProvenanceKind::TaskFailed
        | ProvenanceKind::TaskSkipped
        | ProvenanceKind::ParameterResolved
        | ProvenanceKind::CompensationLogEntry
        | ProvenanceKind::CompensationExecuted
        | ProvenanceKind::CompensationScopeBoundary
        | ProvenanceKind::DelegationViolation
        | ProvenanceKind::InstanceResumed
        | ProvenanceKind::StepResultPersisted
        | ProvenanceKind::IdempotencyDedup
        | ProvenanceKind::InstanceMigrated
        | ProvenanceKind::ContractValidation
        | ProvenanceKind::HistoryCleared
        | ProvenanceKind::DcrActivityExecuted
        | ProvenanceKind::DcrRelationEvaluated
        | ProvenanceKind::DcrResolutionError
        | ProvenanceKind::ZoneSatisfied
        | ProvenanceKind::EquityAlert
        | ProvenanceKind::VerificationReportProduced
        | ProvenanceKind::ImmutabilityViolation
        | ProvenanceKind::ActivationBlocked
        | ProvenanceKind::CalendarIgnored
        | ProvenanceKind::NotificationSuppressed
        | ProvenanceKind::RelationshipChanged
        | ProvenanceKind::MilestoneFired
        | ProvenanceKind::EventEmitted
        | ProvenanceKind::EventConsumed
        | ProvenanceKind::CallbackReceived
        | ProvenanceKind::CallbackPending
        | ProvenanceKind::ArazzoStep
        | ProvenanceKind::ToolInvoked
        | ProvenanceKind::PolicyDecision => "facts",
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
    fn case_file_snapshot_is_canonical_and_tamper_evident() {
        let first = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "b": 2,
            "a": {
                "z": true,
                "m": "stable"
            }
        }));
        let second = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "a": {
                "m": "stable",
                "z": true
            },
            "b": 2
        }));

        assert_eq!(first.jcs_canonical, second.jcs_canonical);
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(
            first.jcs_canonical,
            r#"{"a":{"m":"stable","z":true},"b":2}"#
        );
        assert_eq!(first.sha256.len(), 64);
        assert!(first.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn case_file_snapshot_uses_rfc8785_number_canonicalization() {
        let snapshot = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "b": 12e1,
            "a": 1.0
        }));

        assert_eq!(snapshot.jcs_canonical, r#"{"a":1,"b":120}"#);
    }

    /// RFC 8785 §3.2.3 requires key ordering by UTF-16 code-unit values, not
    /// by UTF-8 bytes. A supplementary-plane character like 🦀 (U+1F980)
    /// encodes to UTF-16 as a surrogate pair starting at 0xD83E; a BMP char
    /// in the private-use area like U+E000 encodes as a single unit 0xE000.
    /// UTF-16 sort: 0xD83E < 0xE000 so 🦀 MUST sort BEFORE "\uE000".
    /// UTF-8 byte sort: "\uE000" first byte 0xEE < 🦀 first byte 0xF0 would
    /// reverse the order — if this test ever flips, the underlying
    /// canonicalizer has silently drifted off RFC 8785.
    #[test]
    fn case_file_snapshot_sorts_keys_by_utf16_code_unit() {
        let snapshot = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "\u{E000}": "private-use",
            "\u{1F980}": "crab"
        }));

        let canonical = &snapshot.jcs_canonical;
        let crab_pos = canonical.find("crab").expect("crab value present");
        let private_pos = canonical.find("private-use").expect("pua value present");
        assert!(
            crab_pos < private_pos,
            "RFC 8785 UTF-16 code-unit order requires 🦀 (U+1F980) to sort \
             before U+E000, but got: {canonical}"
        );
    }

    /// Control characters inside string values MUST use the JSON minimal
    /// escape forms (`\n`, `\t`, `\r`, `\"`, `\\`) and `\u00XX` only when
    /// no short form applies (RFC 8785 §3.2.2).
    #[test]
    fn case_file_snapshot_escapes_control_characters_minimally() {
        let snapshot = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "k": "line1\nline2\ttab\u{0001}end"
        }));

        assert_eq!(
            snapshot.jcs_canonical, r#"{"k":"line1\nline2\ttab\u0001end"}"#,
            "control chars must use short forms where defined and \\u00XX \
             otherwise"
        );
    }

    /// RFC 8785 ES6 `ToString(Number)` canonicalization: exponents and
    /// trailing zeros normalise to the shortest round-trip form.
    #[test]
    fn case_file_snapshot_canonicalises_floats_and_exponents() {
        let snapshot = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "a": 1.0,
            "b": 1.5e2,
            "c": 0.1,
        }));

        assert_eq!(snapshot.jcs_canonical, r#"{"a":1,"b":150,"c":0.1}"#);
    }

    /// Belt-and-braces: the schema example `sha256` digest MUST match what
    /// the Rust JCS path actually computes for `{"eligible":true,"income":17500}`.
    /// A drift here means the schema example and the runtime disagree on
    /// canonical output — a tamper-evidence break.
    #[test]
    fn schema_example_snapshot_digest_matches_runtime_output() {
        let snapshot = CaseFileSnapshot::from_case_state(&serde_json::json!({
            "eligible": true,
            "income": 17500
        }));

        assert_eq!(snapshot.jcs_canonical, r#"{"eligible":true,"income":17500}"#);
        assert_eq!(
            snapshot.sha256,
            "b19f000c0cd497b52c4a78e50641651e4b1e96931a1b1558984d69e722f73f5e"
        );
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

    /// Confirm every constructor zero-initializes the enrichment fields the
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
            assert!(record.transition_tags.is_empty());
            assert!(record.case_file_snapshot.is_none());
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

    /// All provenance enrichment fields round-trip through serde when populated.
    #[test]
    fn round_trips_provenance_enrichment_fields() {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("actor"));
        record.audit_layer = Some("reasoning".to_string());
        record.actor_type = Some("agent".to_string());
        record.lifecycle_state = Some("under-review".to_string());
        record.definition_version = Some("1.2.3".to_string());
        record.inputs = vec![
            "entity:application".to_string(),
            "entity:evidence".to_string(),
        ];
        record.outputs = vec!["entity:decision".to_string()];
        record.input_digest = Some("sha256:deadbeef".to_string());
        record.output_digest = Some("sha256:cafebabe".to_string());
        record.transition_tags = vec!["determination".to_string()];
        record.case_file_snapshot = Some(CaseFileSnapshot::from_case_state(
            &serde_json::json!({ "decision": "denied" }),
        ));

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
        assert_eq!(restored.transition_tags, vec!["determination".to_string()]);
        assert_eq!(
            restored
                .case_file_snapshot
                .as_ref()
                .map(|snapshot| snapshot.jcs_canonical.as_str()),
            Some(r#"{"decision":"denied"}"#)
        );
    }

    #[test]
    fn audit_layer_for_kind_maps_narrative_only() {
        assert_eq!(
            audit_layer_for_kind(ProvenanceKind::NarrativeTierRecorded),
            "narrative"
        );
        assert_eq!(
            audit_layer_for_kind(ProvenanceKind::StateTransition),
            "facts"
        );
        assert_eq!(
            audit_layer_for_kind(ProvenanceKind::CaseStateMutation),
            "facts"
        );
        assert_eq!(audit_layer_for_kind(ProvenanceKind::TaskCompleted), "facts");
        assert_eq!(audit_layer_for_kind(ProvenanceKind::EventEmitted), "facts");
    }

    /// Finding 3 regression: every `ProvenanceKind` variant must map to a
    /// tier via an explicit match arm — no wildcard fallback. The hand-list
    /// below mirrors the enum; adding a new variant upstream fails this
    /// test (exhaustive match in the helper) AND this list (missing entry),
    /// forcing the author to consciously assign its tier.
    #[test]
    fn audit_layer_for_kind_covers_every_variant() {
        let all: &[ProvenanceKind] = &[
            ProvenanceKind::StateTransition,
            ProvenanceKind::UnmatchedEvent,
            ProvenanceKind::CaseStateMutation,
            ProvenanceKind::TimerCreated,
            ProvenanceKind::TimerFired,
            ProvenanceKind::TimerCancelled,
            ProvenanceKind::OnEntry,
            ProvenanceKind::OnExit,
            ProvenanceKind::ActionExecuted,
            ProvenanceKind::InvalidDuration,
            ProvenanceKind::ToleranceViolation,
            ProvenanceKind::DeonticViolation,
            ProvenanceKind::DeonticEvaluation,
            ProvenanceKind::DeonticResolution,
            ProvenanceKind::DeonticBypass,
            ProvenanceKind::RightsViolation,
            ProvenanceKind::ConsistencyViolation,
            ProvenanceKind::AutonomyViolation,
            ProvenanceKind::AutonomyCapped,
            ProvenanceKind::AutonomyComputed,
            ProvenanceKind::HumanTaskCreated,
            ProvenanceKind::ToolViolation,
            ProvenanceKind::EscalationPending,
            ProvenanceKind::AutonomyDemotion,
            ProvenanceKind::ConfidenceViolation,
            ProvenanceKind::ConfidenceDecay,
            ProvenanceKind::CumulativeConfidenceViolation,
            ProvenanceKind::SessionPaused,
            ProvenanceKind::GroundTruthLabel,
            ProvenanceKind::AgentOutput,
            ProvenanceKind::ActorTypeViolation,
            ProvenanceKind::AgentProvenanceAnnotation,
            ProvenanceKind::AgentVersionChange,
            ProvenanceKind::NarrativeTierRecorded,
            ProvenanceKind::ConstraintTamperBlocked,
            ProvenanceKind::DriftReclassification,
            ProvenanceKind::AgentStateTransition,
            ProvenanceKind::ProxyInvocation,
            ProvenanceKind::DispositiveViolation,
            ProvenanceKind::FallbackTriggered,
            ProvenanceKind::FallbackAttempt,
            ProvenanceKind::FallbackTerminal,
            ProvenanceKind::NoticeSent,
            ProvenanceKind::SeparationViolation,
            ProvenanceKind::AppealFiled,
            ProvenanceKind::ProtocolViolation,
            ProvenanceKind::IndependentFirstEnforced,
            ProvenanceKind::SamplingDecision,
            ProvenanceKind::OverrideViolation,
            ProvenanceKind::OverrideRecorded,
            ProvenanceKind::PipelineStageCompleted,
            ProvenanceKind::PipelineRiskProfile,
            ProvenanceKind::PipelineRejection,
            ProvenanceKind::TaskCreated,
            ProvenanceKind::TaskPresented,
            ProvenanceKind::TaskDismissed,
            ProvenanceKind::TaskDraftPersisted,
            ProvenanceKind::TaskResponseSubmitted,
            ProvenanceKind::TaskResponseRejected,
            ProvenanceKind::DataMapping,
            ProvenanceKind::TaskCompleted,
            ProvenanceKind::TaskFailed,
            ProvenanceKind::TaskSkipped,
            ProvenanceKind::ParameterResolved,
            ProvenanceKind::CompensationLogEntry,
            ProvenanceKind::CompensationExecuted,
            ProvenanceKind::CompensationScopeBoundary,
            ProvenanceKind::DelegationViolation,
            ProvenanceKind::InstanceResumed,
            ProvenanceKind::StepResultPersisted,
            ProvenanceKind::IdempotencyDedup,
            ProvenanceKind::InstanceMigrated,
            ProvenanceKind::ContractValidation,
            ProvenanceKind::HistoryCleared,
            ProvenanceKind::DcrActivityExecuted,
            ProvenanceKind::DcrRelationEvaluated,
            ProvenanceKind::DcrResolutionError,
            ProvenanceKind::ZoneSatisfied,
            ProvenanceKind::EquityAlert,
            ProvenanceKind::VerificationReportProduced,
            ProvenanceKind::ImmutabilityViolation,
            ProvenanceKind::ActivationBlocked,
            ProvenanceKind::CalendarIgnored,
            ProvenanceKind::NotificationSuppressed,
            ProvenanceKind::RelationshipChanged,
            ProvenanceKind::MilestoneFired,
            ProvenanceKind::EventEmitted,
            ProvenanceKind::EventConsumed,
            ProvenanceKind::CallbackReceived,
            ProvenanceKind::CallbackPending,
            ProvenanceKind::ArazzoStep,
            ProvenanceKind::ToolInvoked,
            ProvenanceKind::PolicyDecision,
        ];

        for kind in all {
            let tier = audit_layer_for_kind(*kind);
            assert!(
                matches!(tier, "facts" | "narrative" | "reasoning" | "counterfactual"),
                "{kind:?} classified as unknown tier {tier:?}"
            );
        }

        // Exactly one variant is narrative today.
        let narrative_count = all
            .iter()
            .filter(|k| audit_layer_for_kind(**k) == "narrative")
            .count();
        assert_eq!(
            narrative_count, 1,
            "only NarrativeTierRecorded should classify as narrative today"
        );
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
