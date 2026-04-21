// Rust guideline compliant 2026-02-21

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

    assert_eq!(
        snapshot.jcs_canonical,
        r#"{"eligible":true,"income":17500}"#
    );
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
        assert!(record.outcome.is_none());
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
        ProvenanceKind::ConvergenceCapReached,
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
