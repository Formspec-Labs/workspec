// Rust guideline compliant 2026-02-21

use super::*;
use crate::instance::{ActiveTask, ActiveTaskStatus};
use crate::model::kernel::{
    ActorKind, AuditLayer, ImpactLevel, MutationSource, PublicationStatus, VerificationLevel,
    WorkflowDocument,
};

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
fn workflow_document_status_round_trips_canonical_string() {
    let document: WorkflowDocument = serde_json::from_value(serde_json::json!({
        "$wosWorkflow": "1.0",
        "status": "deprecated",
        "lifecycle": {
            "initialState": "draft",
            "states": {
                "draft": {
                    "type": "final"
                }
            }
        }
    }))
    .expect("deserialize");

    assert_eq!(document.status, Some(PublicationStatus::Deprecated));

    let json = serde_json::to_value(&document).expect("serialize");
    assert_eq!(json["status"], "deprecated");
}

#[test]
fn active_task_impact_level_round_trips_canonical_string() {
    let task = ActiveTask {
        task_id: "task-1".to_string(),
        task_ref: "task-ref".to_string(),
        status: ActiveTaskStatus::Created,
        assigned_actor: None,
        contract_ref: None,
        binding: None,
        definition_url: None,
        definition_version: None,
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        deadline: None,
        impact_level: Some(ImpactLevel::Operational),
        context: None,
        last_validation_outcome: None,
        created_at: "2026-05-01T00:00:00Z".to_string(),
        updated_at: "2026-05-01T00:00:00Z".to_string(),
        extensions: std::collections::HashMap::new(),
    };

    let json = serde_json::to_value(&task).expect("serialize");
    assert_eq!(json["impactLevel"], "operational");

    let restored: ActiveTask = serde_json::from_value(json).expect("deserialize");
    assert_eq!(restored.impact_level, Some(ImpactLevel::Operational));
}

#[test]
fn provenance_typed_accessors_round_trip_canonical_strings() {
    let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("actor"));
    record.set_audit_layer_kind(AuditLayer::Reasoning);
    record.set_actor_kind(ActorKind::Agent);

    let json = serde_json::to_value(&record).expect("serialize");
    assert_eq!(json["auditLayer"], "reasoning");
    assert_eq!(json["actorType"], "agent");

    let restored: ProvenanceRecord = serde_json::from_value(json).expect("deserialize");
    assert_eq!(restored.audit_layer_kind(), Some(AuditLayer::Reasoning));
    assert_eq!(restored.actor_kind(), Some(ActorKind::Agent));
}

#[test]
fn case_state_mutation_with_source_round_trips_canonical_strings() {
    let mut record = ProvenanceRecord::case_state_mutation_with_source(
        "/path",
        &serde_json::json!(42),
        Some("actor"),
        "active",
        Some(MutationSource::AgentExtracted),
        Some(VerificationLevel::Authoritative),
    );
    record.timestamp = "2026-05-01T00:00:00Z".to_string();

    let json = serde_json::to_value(&record).expect("serialize");
    assert_eq!(json["data"]["mutationSource"], "agent-extracted");
    assert_eq!(json["data"]["verificationLevel"], "authoritative");

    let restored: ProvenanceRecord = serde_json::from_value(json).expect("deserialize");
    assert_eq!(restored.timestamp, "2026-05-01T00:00:00Z");
    assert_eq!(
        restored
            .data
            .as_ref()
            .and_then(|data| data.get("mutationSource"))
            .and_then(serde_json::Value::as_str),
        Some("agent-extracted")
    );
    assert_eq!(
        restored
            .data
            .as_ref()
            .and_then(|data| data.get("verificationLevel"))
            .and_then(serde_json::Value::as_str),
        Some("authoritative")
    );
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
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
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
    let mutation =
        ProvenanceRecord::case_state_mutation("/p", &serde_json::json!(1), None, "active");
    assert!(mutation.audit_layer.is_none());
    assert!(mutation.actor_type.is_none());
    assert_eq!(mutation.lifecycle_state.as_deref(), Some("active"));
    assert_eq!(mutation.to_state.as_deref(), Some("active"));
    assert!(mutation.definition_version.is_none());
    assert!(mutation.inputs.is_empty());
    assert!(mutation.outputs.is_empty());
    assert!(mutation.input_digest.is_none());
    assert!(mutation.output_digest.is_none());
    assert!(mutation.transition_tags.is_empty());
    assert!(mutation.case_file_snapshot.is_none());
    assert!(mutation.outcome.is_none());
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
    assert_zero_defaults(&ProvenanceRecord::signature_affirmation(
        signature_affirmation_input(),
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
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::IntakeAccepted),
        "facts"
    );
    assert_eq!(audit_layer_for_kind(ProvenanceKind::TaskCompleted), "facts");
    assert_eq!(audit_layer_for_kind(ProvenanceKind::EventEmitted), "facts");
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::SignatureAffirmation),
        "facts"
    );
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::InstanceSuspended),
        "facts"
    );
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::ReportTimedOut),
        "facts"
    );
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
        ProvenanceKind::CaseCreated,
        ProvenanceKind::IntakeAccepted,
        ProvenanceKind::IntakeRejected,
        ProvenanceKind::IntakeDeferred,
        ProvenanceKind::TimerCreated,
        ProvenanceKind::TimerFired,
        ProvenanceKind::ForEachIterationStarted,
        ProvenanceKind::ForEachIterationCompleted,
        ProvenanceKind::ForEachCompleted,
        ProvenanceKind::TimerCancelled,
        ProvenanceKind::OnEntry,
        ProvenanceKind::OnExit,
        ProvenanceKind::ActionExecuted,
        ProvenanceKind::InvalidDuration,
        ProvenanceKind::ToleranceViolation,
        ProvenanceKind::ConvergenceCapReached,
        ProvenanceKind::CapabilityInvocation,
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
        ProvenanceKind::AutonomyEscalation,
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
        ProvenanceKind::LegalHoldPlaced,
        ProvenanceKind::LegalHoldReleased,
        ProvenanceKind::LegalHoldDestructionRejected,
        ProvenanceKind::ContinuationOfServicesActivated,
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
        ProvenanceKind::InstanceSuspended,
        ProvenanceKind::InstanceResumed,
        ProvenanceKind::InstanceTerminated,
        ProvenanceKind::StepResultPersisted,
        ProvenanceKind::IdempotencyDedup,
        ProvenanceKind::InstanceMigrated,
        ProvenanceKind::ContractValidation,
        ProvenanceKind::HistoryCleared,
        ProvenanceKind::DcrActivityExecuted,
        ProvenanceKind::DcrRelationEvaluated,
        ProvenanceKind::DcrResolutionError,
        ProvenanceKind::ZoneSatisfied,
        ProvenanceKind::DcrZoneViolation,
        ProvenanceKind::EquityAlert,
        ProvenanceKind::CircuitBreakerTripped,
        ProvenanceKind::CircuitBreakerReset,
        ProvenanceKind::ShadowModeDivergence,
        ProvenanceKind::DriftAlert,
        ProvenanceKind::VerificationReportProduced,
        ProvenanceKind::ImmutabilityViolation,
        ProvenanceKind::ActivationBlocked,
        ProvenanceKind::CalendarIgnored,
        ProvenanceKind::NotificationSuppressed,
        ProvenanceKind::ReportTimedOut,
        ProvenanceKind::ConfigurationWarning,
        ProvenanceKind::RelationshipChanged,
        ProvenanceKind::MilestoneFired,
        ProvenanceKind::EventEmitted,
        ProvenanceKind::EventConsumed,
        ProvenanceKind::CallbackReceived,
        ProvenanceKind::CallbackPending,
        ProvenanceKind::ArazzoStep,
        ProvenanceKind::ToolInvoked,
        ProvenanceKind::PolicyDecision,
        ProvenanceKind::SignatureAffirmation,
        ProvenanceKind::CorrectionAuthorized,
        ProvenanceKind::AmendmentAuthorized,
        ProvenanceKind::DeterminationAmended,
        ProvenanceKind::RescissionAuthorized,
        ProvenanceKind::DeterminationRescinded,
        ProvenanceKind::Reinstated,
        ProvenanceKind::AuthorizationAttestation,
        ProvenanceKind::ClockStarted,
        ProvenanceKind::ClockResolved,
        ProvenanceKind::IdentityAttestation,
        ProvenanceKind::ClockSkewObserved,
        ProvenanceKind::CommitAttemptFailure,
        ProvenanceKind::AuthorizationRejected,
        ProvenanceKind::MigrationPinChanged,
    ];

    assert_eq!(
        all.len(),
        131,
        "ProvenanceKind has 131 variants at HEAD; a new variant upstream MUST add an entry here"
    );

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
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
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

fn signature_affirmation_input() -> SignatureAffirmationInput<'static> {
    SignatureAffirmationInput {
        signer_id: "applicant",
        role_id: "applicantSigner",
        role: "signer",
        document_id: "benefitsApplication",
        document_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        document_hash_algorithm: "sha-256",
        source_signature_system: "formspec",
        source_signature_id: "sig-2026-0001",
        signed_payload_digest: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        signed_payload_digest_algorithm: "sha-256",
        signing_intent: "urn:wos:signing-intent:applicant-signature",
        signed_at: "2026-04-22T14:30:00Z",
        identity_binding: serde_json::json!({
            "method": "email-otp",
            "assuranceLevel": "standard",
            "providerRef": "urn:agency.gov:identity:providers:email-otp"
        }),
        consent_reference: serde_json::json!({
            "consentTextRef": "urn:agency.gov:consent:esign-benefits:v1",
            "consentVersion": "1.0.0",
            "acceptedAtPath": "response.signature.acceptedAt",
            "affirmationPath": "response.signature.affirmed"
        }),
        signature_provider: "urn:agency.gov:signature:providers:formspec",
        ceremony_id: "ceremony-2026-0001",
        profile_ref: Some("urn:agency.gov:wos:signature-profile:benefits:v1"),
        profile_key: None,
        source_response_ref: "urn:agency.gov:formspec:responses:benefits:case-2026-0001",
        signer_authority: None,
        custody_hook_eligible: true,
        primitive_verification: serde_json::json!({
            "status": "deferredPendingHelper",
            "reason": "formspec-signing-helper-pending",
        }),
    }
}

#[test]
fn signature_affirmation_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::signature_affirmation(signature_affirmation_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "signatureAffirmation");
    assert_eq!(json["actorId"], "applicant");
    assert_eq!(json["data"]["signerId"], "applicant");
    assert_eq!(json["data"]["roleId"], "applicantSigner");
    assert_eq!(json["data"]["role"], "signer");
    assert_eq!(json["data"]["documentId"], "benefitsApplication");
    assert_eq!(
        json["data"]["documentHash"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(json["data"]["documentHashAlgorithm"], "sha-256");
    assert_eq!(json["data"]["sourceSignatureSystem"], "formspec");
    assert_eq!(json["data"]["sourceSignatureId"], "sig-2026-0001");
    assert_eq!(
        json["data"]["signedPayloadDigest"],
        "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    assert_eq!(json["data"]["signedPayloadDigestAlgorithm"], "sha-256");
    assert_eq!(
        json["data"]["signingIntent"],
        "urn:wos:signing-intent:applicant-signature"
    );
    assert_eq!(json["data"]["signedAt"], "2026-04-22T14:30:00Z");
    assert_eq!(
        json["data"]["signatureProvider"],
        "urn:agency.gov:signature:providers:formspec"
    );
    assert_eq!(json["data"]["ceremonyId"], "ceremony-2026-0001");
    assert_eq!(
        json["data"]["profileRef"],
        "urn:agency.gov:wos:signature-profile:benefits:v1"
    );
    assert_eq!(
        json["data"]["sourceResponseRef"],
        "urn:agency.gov:formspec:responses:benefits:case-2026-0001"
    );
    assert_eq!(json["data"]["custodyHookEligible"], true);
}

#[test]
fn signature_affirmation_carries_primitive_verification_deferred() {
    // The reference Formspec binding emits
    // SignaturePrimitiveStatus::DeferredPendingHelper while
    // FORMSPEC-SIGN-HELPER-001 is unshipped. The SignatureAffirmation
    // provenance record must carry that status forward verbatim into
    // data.primitiveVerification so downstream verifiers can see the
    // verification gap rather than be misled by a falsely-confident
    // affirmation.
    let mut input = signature_affirmation_input();
    input.primitive_verification = serde_json::json!({
        "status": "deferredPendingHelper",
        "reason": "formspec-signing-helper-pending",
    });
    let record = ProvenanceRecord::signature_affirmation(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(
        json["data"]["primitiveVerification"]["status"],
        "deferredPendingHelper"
    );
    assert_eq!(
        json["data"]["primitiveVerification"]["reason"],
        "formspec-signing-helper-pending"
    );
}

#[test]
fn capability_invocation_blocked_sets_precondition_outcome() {
    let mut context = serde_json::Map::new();
    context.insert(
        "failedPrecondition".to_string(),
        serde_json::Value::String("caseFile.applicantConsent == true".to_string()),
    );
    let record = ProvenanceRecord::capability_invocation(CapabilityInvocationInput {
        capability_id: "documentExtraction",
        agent_id: "intake-classifier",
        invocation_blocked: true,
        context: Some(context),
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "capabilityInvocation");
    assert_eq!(json["actorId"], "intake-classifier");
    assert_eq!(json["data"]["capabilityId"], "documentExtraction");
    assert_eq!(json["data"]["invocationBlocked"], true);
    assert_eq!(
        json["data"]["failedPrecondition"],
        "caseFile.applicantConsent == true"
    );
    assert_eq!(json["outcome"], "preconditionNotSatisfied");
}

#[test]
fn capability_invocation_permitted_omits_outcome() {
    let record = ProvenanceRecord::capability_invocation(CapabilityInvocationInput {
        capability_id: "documentExtraction",
        agent_id: "intake-classifier",
        invocation_blocked: false,
        context: None,
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "capabilityInvocation");
    assert_eq!(json["data"]["capabilityId"], "documentExtraction");
    assert_eq!(json["data"]["invocationBlocked"], false);
    assert!(
        json.get("outcome").is_none(),
        "permitted invocations MUST omit the outcome field; AI §3.3.1 reserves the literal for blocked records"
    );
}

#[test]
fn capability_invocation_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "capabilityId".to_string(),
        serde_json::Value::String("attacker-overrides-this".to_string()),
    );
    context.insert(
        "invocationBlocked".to_string(),
        serde_json::Value::Bool(true),
    );
    context.insert(
        "fallbackChainRef".to_string(),
        serde_json::Value::String("urn:agency.gov:fallback:human-review".to_string()),
    );
    let record = ProvenanceRecord::capability_invocation(CapabilityInvocationInput {
        capability_id: "trueCapability",
        agent_id: "intake-classifier",
        invocation_blocked: false,
        context: Some(context),
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(
        json["data"]["capabilityId"], "trueCapability",
        "constructor's capability_id MUST win over context-supplied capabilityId"
    );
    assert_eq!(
        json["data"]["invocationBlocked"], false,
        "constructor's invocation_blocked MUST win over context-supplied invocationBlocked"
    );
    assert_eq!(
        json["data"]["fallbackChainRef"],
        "urn:agency.gov:fallback:human-review"
    );
}

#[test]
fn capability_invocation_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::CapabilityInvocation),
        "facts"
    );
}

#[test]
fn configuration_warning_unresolved_ref_subject_serializes_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "workflowUri".to_string(),
        serde_json::Value::String("urn:agency.gov:wos:benefits:v1".to_string()),
    );
    let record = ProvenanceRecord::configuration_warning(ConfigurationWarningInput {
        subject: "drift-monitor.policyRef",
        unresolved_ref: Some("DemotionRule.id::nonexistent"),
        context: Some(context),
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "configurationWarning");
    assert_eq!(json["data"]["subject"], "drift-monitor.policyRef");
    assert_eq!(
        json["data"]["unresolvedRef"],
        "DemotionRule.id::nonexistent"
    );
    assert_eq!(
        json["data"]["workflowUri"],
        "urn:agency.gov:wos:benefits:v1"
    );
}

#[test]
fn configuration_warning_render_failure_subject_omits_unresolved_ref() {
    let mut context = serde_json::Map::new();
    context.insert(
        "templateKey".to_string(),
        serde_json::Value::String("benefits-denial-notice".to_string()),
    );
    context.insert(
        "failureReason".to_string(),
        serde_json::Value::String("template field {{caseId}} unresolved".to_string()),
    );
    let record = ProvenanceRecord::configuration_warning(ConfigurationWarningInput {
        subject: "notification-template.render",
        unresolved_ref: None,
        context: Some(context),
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["subject"], "notification-template.render");
    assert!(
        json["data"].get("unresolvedRef").is_none(),
        "render-failure subjects MUST omit unresolvedRef when input.unresolved_ref is None"
    );
    assert_eq!(json["data"]["templateKey"], "benefits-denial-notice");
}

#[test]
fn configuration_warning_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "subject".to_string(),
        serde_json::Value::String("attacker-overrides-this".to_string()),
    );
    context.insert(
        "unresolvedRef".to_string(),
        serde_json::Value::String("attacker-overrides-this-too".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("preserved".to_string()),
    );
    let record = ProvenanceRecord::configuration_warning(ConfigurationWarningInput {
        subject: "governance.continuationPolicyRef",
        unresolved_ref: Some("ContinuationPolicy.id::missing"),
        context: Some(context),
    });
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(
        json["data"]["subject"], "governance.continuationPolicyRef",
        "constructor's subject MUST win over context-supplied subject"
    );
    assert_eq!(
        json["data"]["unresolvedRef"], "ContinuationPolicy.id::missing",
        "constructor's unresolved_ref MUST win over context-supplied unresolvedRef"
    );
    assert_eq!(json["data"]["auxNote"], "preserved");
}

#[test]
fn configuration_warning_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::ConfigurationWarning),
        "facts"
    );
}

#[test]
fn capability_invocation_round_trips_through_serde() {
    let blocked = ProvenanceRecord::capability_invocation(CapabilityInvocationInput {
        capability_id: "documentExtraction",
        agent_id: "intake-classifier",
        invocation_blocked: true,
        context: None,
    });
    let json = serde_json::to_string(&blocked).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(
        restored.record_kind,
        ProvenanceKind::CapabilityInvocation
    ));
    assert_eq!(
        restored.outcome.as_deref(),
        Some("preconditionNotSatisfied")
    );
    assert_eq!(restored.actor_id.as_deref(), Some("intake-classifier"));
}

// ── Amendment & supersession (ADR 0066) ─────────────────────────────────

fn correction_authorized_input() -> CorrectionAuthorizedInput<'static> {
    CorrectionAuthorizedInput {
        correction_target_event_hash: "sha256:event-1",
        corrected_field_set: vec!["/applicant/email", "/applicant/phone"],
        reason: "transcription typo",
        authorizing_actor_id: "case-worker-7",
        authority_basis: serde_json::json!({"kind": "actorPolicyRef", "value": "policy:line-corrections"}),
        context: None,
    }
}

#[test]
fn correction_authorized_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::correction_authorized(correction_authorized_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "correctionAuthorized");
    assert_eq!(json["actorId"], "case-worker-7");
    assert_eq!(json["data"]["correctionTargetEventHash"], "sha256:event-1");
    assert_eq!(
        json["data"]["correctedFieldSet"],
        serde_json::json!(["/applicant/email", "/applicant/phone"])
    );
    assert_eq!(json["data"]["reason"], "transcription typo");
    assert_eq!(json["data"]["authorizingActorId"], "case-worker-7");
    assert_eq!(json["data"]["authorityBasis"]["kind"], "actorPolicyRef");
    assert_eq!(
        json["data"]["authorityBasis"]["value"],
        "policy:line-corrections"
    );
}

#[test]
fn correction_authorized_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "correctionTargetEventHash".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "reason".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("preserved".to_string()),
    );
    let mut input = correction_authorized_input();
    input.context = Some(context);
    let record = ProvenanceRecord::correction_authorized(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["correctionTargetEventHash"], "sha256:event-1");
    assert_eq!(json["data"]["reason"], "transcription typo");
    assert_eq!(json["data"]["auxNote"], "preserved");
}

#[test]
fn correction_authorized_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::CorrectionAuthorized),
        "facts"
    );
}

#[test]
fn correction_authorized_round_trips_through_serde() {
    let record = ProvenanceRecord::correction_authorized(correction_authorized_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::CorrectionAuthorized);
    assert_eq!(restored.actor_id.as_deref(), Some("case-worker-7"));
}

#[test]
fn correction_authorized_field_set_is_array_of_pointer_strings() {
    let record = ProvenanceRecord::correction_authorized(correction_authorized_input());
    let json = serde_json::to_value(&record).expect("serialize");
    let arr = json["data"]["correctedFieldSet"].as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert!(arr.iter().all(|v| v.is_string()));
}

fn amendment_authorized_input() -> AmendmentAuthorizedInput<'static> {
    AmendmentAuthorizedInput {
        amendment_target_event_hash: "sha256:event-2",
        prior_determination_hash: "sha256:det-1",
        reason: "new evidence received",
        authorizing_actor_id: "supervisor-3",
        authority_basis: serde_json::json!({"kind": "uri", "value": "https://agency.gov/auth/amend"}),
        context: None,
    }
}

#[test]
fn amendment_authorized_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::amendment_authorized(amendment_authorized_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "amendmentAuthorized");
    assert_eq!(json["actorId"], "supervisor-3");
    assert_eq!(json["data"]["amendmentTargetEventHash"], "sha256:event-2");
    assert_eq!(json["data"]["priorDeterminationHash"], "sha256:det-1");
    assert_eq!(json["data"]["reason"], "new evidence received");
    assert_eq!(json["data"]["authorityBasis"]["kind"], "uri");
}

#[test]
fn amendment_authorized_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "amendmentTargetEventHash".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = amendment_authorized_input();
    input.context = Some(context);
    let record = ProvenanceRecord::amendment_authorized(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["amendmentTargetEventHash"], "sha256:event-2");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn amendment_authorized_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::AmendmentAuthorized),
        "facts"
    );
}

#[test]
fn amendment_authorized_round_trips_through_serde() {
    let record = ProvenanceRecord::amendment_authorized(amendment_authorized_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::AmendmentAuthorized);
}

fn determination_amended_input() -> DeterminationAmendedInput<'static> {
    DeterminationAmendedInput {
        prior_determination_hash: "sha256:det-1",
        new_determination_value: serde_json::json!({"eligibility": "eligible", "amount": 1200}),
        amendment_authorization_event_hash: "sha256:event-2",
        context: None,
    }
}

#[test]
fn determination_amended_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::determination_amended(determination_amended_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "determinationAmended");
    assert_eq!(json["data"]["priorDeterminationHash"], "sha256:det-1");
    assert_eq!(
        json["data"]["newDeterminationValue"]["eligibility"],
        "eligible"
    );
    assert_eq!(json["data"]["newDeterminationValue"]["amount"], 1200);
    assert_eq!(
        json["data"]["amendmentAuthorizationEventHash"],
        "sha256:event-2"
    );
}

#[test]
fn determination_amended_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "newDeterminationValue".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = determination_amended_input();
    input.context = Some(context);
    let record = ProvenanceRecord::determination_amended(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert!(
        json["data"]["newDeterminationValue"].is_object(),
        "constructor value MUST win over context override"
    );
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn determination_amended_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::DeterminationAmended),
        "facts"
    );
}

#[test]
fn determination_amended_round_trips_through_serde() {
    let record = ProvenanceRecord::determination_amended(determination_amended_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::DeterminationAmended);
}

fn rescission_authorized_input() -> RescissionAuthorizedInput<'static> {
    RescissionAuthorizedInput {
        rescission_target_event_hash: "sha256:event-3",
        prior_determination_hash: "sha256:det-1",
        reason: "fraud finding",
        authorizing_actor_id: "fraud-officer-2",
        authority_basis: serde_json::json!({"kind": "uri", "value": "https://agency.gov/auth/rescind"}),
        migration_pin_change: None,
        context: None,
    }
}

#[test]
fn rescission_authorized_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::rescission_authorized(rescission_authorized_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "rescissionAuthorized");
    assert_eq!(json["actorId"], "fraud-officer-2");
    assert_eq!(json["data"]["rescissionTargetEventHash"], "sha256:event-3");
    assert_eq!(json["data"]["priorDeterminationHash"], "sha256:det-1");
    assert_eq!(json["data"]["reason"], "fraud finding");
    assert!(
        json["data"].get("migrationPinChange").is_none(),
        "omits migrationPinChange when input has None"
    );
}

#[test]
fn rescission_authorized_carries_optional_migration_pin_change() {
    let mut pin = serde_json::Map::new();
    pin.insert(
        "newChainPinEventHash".to_string(),
        serde_json::Value::String("sha256:pin-2".to_string()),
    );
    pin.insert(
        "priorPinSet".to_string(),
        serde_json::json!({"formspec.definitionVersion": "1.0.0"}),
    );
    pin.insert(
        "newPinSet".to_string(),
        serde_json::json!({"formspec.definitionVersion": "1.1.0"}),
    );
    let mut input = rescission_authorized_input();
    input.migration_pin_change = Some(pin);
    let record = ProvenanceRecord::rescission_authorized(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(
        json["data"]["migrationPinChange"]["newChainPinEventHash"],
        "sha256:pin-2"
    );
    assert_eq!(
        json["data"]["migrationPinChange"]["newPinSet"]["formspec.definitionVersion"],
        "1.1.0"
    );
}

#[test]
fn rescission_authorized_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "rescissionTargetEventHash".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "migrationPinChange".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = rescission_authorized_input();
    input.context = Some(context);
    let record = ProvenanceRecord::rescission_authorized(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["rescissionTargetEventHash"], "sha256:event-3");
    assert!(
        json["data"].get("migrationPinChange").is_none(),
        "context-supplied migrationPinChange dropped; constructor's None wins"
    );
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn rescission_authorized_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::RescissionAuthorized),
        "facts"
    );
}

#[test]
fn rescission_authorized_round_trips_through_serde() {
    let record = ProvenanceRecord::rescission_authorized(rescission_authorized_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::RescissionAuthorized);
}

fn determination_rescinded_input() -> DeterminationRescindedInput<'static> {
    DeterminationRescindedInput {
        prior_determination_hash: "sha256:det-1",
        rescission_authorization_event_hash: "sha256:event-3",
        context: None,
    }
}

#[test]
fn determination_rescinded_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::determination_rescinded(determination_rescinded_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "determinationRescinded");
    assert_eq!(json["data"]["priorDeterminationHash"], "sha256:det-1");
    assert_eq!(
        json["data"]["rescissionAuthorizationEventHash"],
        "sha256:event-3"
    );
}

#[test]
fn determination_rescinded_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "priorDeterminationHash".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = determination_rescinded_input();
    input.context = Some(context);
    let record = ProvenanceRecord::determination_rescinded(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["priorDeterminationHash"], "sha256:det-1");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn determination_rescinded_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::DeterminationRescinded),
        "facts"
    );
}

#[test]
fn determination_rescinded_round_trips_through_serde() {
    let record = ProvenanceRecord::determination_rescinded(determination_rescinded_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::DeterminationRescinded);
}

fn reinstated_input() -> ReinstatedInput<'static> {
    ReinstatedInput {
        prior_rescission_event_hash: "sha256:event-4",
        reactivation_authorization_event_hash: "sha256:event-5",
        reason: "rescission overturned on appeal",
        context: None,
    }
}

#[test]
fn reinstated_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::reinstated(reinstated_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "reinstated");
    assert_eq!(json["data"]["priorRescissionEventHash"], "sha256:event-4");
    assert_eq!(
        json["data"]["reactivationAuthorizationEventHash"],
        "sha256:event-5"
    );
    assert_eq!(json["data"]["reason"], "rescission overturned on appeal");
}

#[test]
fn reinstated_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "reason".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = reinstated_input();
    input.context = Some(context);
    let record = ProvenanceRecord::reinstated(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["reason"], "rescission overturned on appeal");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn reinstated_classifies_as_facts() {
    assert_eq!(audit_layer_for_kind(ProvenanceKind::Reinstated), "facts");
}

#[test]
fn reinstated_round_trips_through_serde() {
    let record = ProvenanceRecord::reinstated(reinstated_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::Reinstated);
}

#[test]
fn reinstated_is_distinct_from_amendment_kind() {
    let r = ProvenanceRecord::reinstated(reinstated_input());
    let a = ProvenanceRecord::amendment_authorized(amendment_authorized_input());
    assert_ne!(r.record_kind, a.record_kind);
    let r_json = serde_json::to_value(&r).expect("serialize");
    assert_eq!(r_json["recordKind"], "reinstated");
}

fn authorization_attestation_input() -> AuthorizationAttestationInput<'static> {
    AuthorizationAttestationInput {
        authorizing_actor_id: "supervisor-3",
        authority_basis: serde_json::json!({"kind": "actorPolicyRef", "value": "policy:amendment"}),
        policy_predicate: "amendment-authority",
        assurance_level: Some("high"),
        context: None,
    }
}

#[test]
fn authorization_attestation_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::authorization_attestation(authorization_attestation_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "authorizationAttestation");
    assert_eq!(json["actorId"], "supervisor-3");
    assert_eq!(json["data"]["authorizingActorId"], "supervisor-3");
    assert_eq!(json["data"]["policyPredicate"], "amendment-authority");
    assert_eq!(json["data"]["assuranceLevel"], "high");
    assert_eq!(json["data"]["authorityBasis"]["kind"], "actorPolicyRef");
}

#[test]
fn authorization_attestation_omits_optional_assurance_level_when_none() {
    let mut input = authorization_attestation_input();
    input.assurance_level = None;
    let record = ProvenanceRecord::authorization_attestation(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert!(json["data"].get("assuranceLevel").is_none());
}

#[test]
fn authorization_attestation_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "policyPredicate".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = authorization_attestation_input();
    input.context = Some(context);
    let record = ProvenanceRecord::authorization_attestation(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["policyPredicate"], "amendment-authority");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn authorization_attestation_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::AuthorizationAttestation),
        "facts"
    );
}

#[test]
fn authorization_attestation_round_trips_through_serde() {
    let record = ProvenanceRecord::authorization_attestation(authorization_attestation_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        restored.record_kind,
        ProvenanceKind::AuthorizationAttestation
    );
}

// ── Statutory clocks (ADR 0067) ─────────────────────────────────────────

fn clock_started_input() -> ClockStartedInput<'static> {
    ClockStartedInput {
        clock_id: "clock-appeal-001",
        clock_kind: "AppealClock",
        origin_event_hash: "sha256:origin-1",
        duration: "P30D",
        computed_deadline: "2026-05-28T00:00:00Z",
        calendar_ref: Some("urn:agency.gov:calendar:business"),
        statute_reference: Some("https://agency.gov/statute/appeal-30d"),
        context: None,
    }
}

#[test]
fn clock_started_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::clock_started(clock_started_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "clockStarted");
    assert_eq!(json["data"]["clockId"], "clock-appeal-001");
    assert_eq!(json["data"]["clockKind"], "AppealClock");
    assert_eq!(json["data"]["originEventHash"], "sha256:origin-1");
    assert_eq!(json["data"]["duration"], "P30D");
    assert_eq!(json["data"]["computedDeadline"], "2026-05-28T00:00:00Z");
    assert_eq!(
        json["data"]["calendarRef"],
        "urn:agency.gov:calendar:business"
    );
    assert_eq!(
        json["data"]["statuteReference"],
        "https://agency.gov/statute/appeal-30d"
    );
}

#[test]
fn clock_started_omits_optional_fields_when_none() {
    let mut input = clock_started_input();
    input.calendar_ref = None;
    input.statute_reference = None;
    let record = ProvenanceRecord::clock_started(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert!(json["data"].get("calendarRef").is_none());
    assert!(json["data"].get("statuteReference").is_none());
}

#[test]
fn clock_started_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "clockId".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = clock_started_input();
    input.context = Some(context);
    let record = ProvenanceRecord::clock_started(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["clockId"], "clock-appeal-001");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn clock_started_classifies_as_facts() {
    assert_eq!(audit_layer_for_kind(ProvenanceKind::ClockStarted), "facts");
}

#[test]
fn clock_started_round_trips_through_serde() {
    let record = ProvenanceRecord::clock_started(clock_started_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::ClockStarted);
}

fn clock_resolved_input() -> ClockResolvedInput<'static> {
    ClockResolvedInput {
        clock_id: "clock-appeal-001",
        origin_clock_hash: "sha256:clock-origin-1",
        resolution: ClockResolvedResolution::Satisfied {
            resolving_event_hash: Some("sha256:resolve-1"),
        },
        resolved_at: "2026-05-15T12:00:00Z",
        context: None,
    }
}

#[test]
fn clock_resolved_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::clock_resolved(clock_resolved_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "clockResolved");
    assert_eq!(json["data"]["clockId"], "clock-appeal-001");
    assert_eq!(json["data"]["originClockHash"], "sha256:clock-origin-1");
    assert_eq!(json["data"]["resolution"], "satisfied");
    assert_eq!(json["data"]["resolvedAt"], "2026-05-15T12:00:00Z");
    assert_eq!(json["data"]["resolvingEventHash"], "sha256:resolve-1");
}

#[test]
fn clock_resolved_paused_requires_resolving_event_hash() {
    let input = ClockResolvedInput {
        clock_id: "clock-appeal-001",
        origin_clock_hash: "sha256:clock-origin-1",
        resolution: ClockResolvedResolution::Paused {
            resolving_event_hash: "sha256:pause-event-1",
        },
        resolved_at: "2026-05-01T14:30:00Z",
        context: None,
    };
    let record = ProvenanceRecord::clock_resolved(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert_eq!(json["data"]["resolution"], "paused");
    assert_eq!(json["data"]["resolvingEventHash"], "sha256:pause-event-1");
}

#[test]
fn clock_resolved_all_typed_resolutions_serialize_camelcase() {
    for (resolution, expected) in [
        (
            ClockResolvedResolution::Satisfied {
                resolving_event_hash: None,
            },
            "satisfied",
        ),
        (
            ClockResolvedResolution::Elapsed {
                resolving_event_hash: None,
            },
            "elapsed",
        ),
        (
            ClockResolvedResolution::Paused {
                resolving_event_hash: "sha256:pause-by-kind",
            },
            "paused",
        ),
        (
            ClockResolvedResolution::Cancelled {
                resolving_event_hash: None,
            },
            "cancelled",
        ),
    ] {
        let mut input = clock_resolved_input();
        input.resolution = resolution;
        let record = ProvenanceRecord::clock_resolved(input);
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["data"]["resolution"], expected);
    }
}

#[test]
fn clock_resolved_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "resolution".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = clock_resolved_input();
    input.context = Some(context);
    let record = ProvenanceRecord::clock_resolved(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["resolution"], "satisfied");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn clock_resolved_classifies_as_facts() {
    assert_eq!(audit_layer_for_kind(ProvenanceKind::ClockResolved), "facts");
}

#[test]
fn clock_resolved_round_trips_through_serde() {
    let record = ProvenanceRecord::clock_resolved(clock_resolved_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::ClockResolved);
}

// ── Identity attestation (ADR 0068) ─────────────────────────────────────

fn identity_attestation_input() -> IdentityAttestationInput<'static> {
    IdentityAttestationInput {
        subject_global_id: "did:example:applicant-001",
        assurance_level: "high",
        attestation_provider: "urn:agency.gov:identity:provider:idme",
        provider_attestation_id: "att-2026-0001",
        attested_at: "2026-04-01T10:00:00Z",
        valid_until: Some("2027-04-01T10:00:00Z"),
        attested_predicates: vec!["legal-name-verified", "age-of-majority"],
        context: None,
    }
}

#[test]
fn identity_attestation_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::identity_attestation(identity_attestation_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "identityAttestation");
    assert_eq!(json["data"]["subjectGlobalId"], "did:example:applicant-001");
    assert_eq!(json["data"]["assuranceLevel"], "high");
    assert_eq!(
        json["data"]["attestationProvider"],
        "urn:agency.gov:identity:provider:idme"
    );
    assert_eq!(json["data"]["providerAttestationId"], "att-2026-0001");
    assert_eq!(json["data"]["attestedAt"], "2026-04-01T10:00:00Z");
    assert_eq!(json["data"]["validUntil"], "2027-04-01T10:00:00Z");
    assert_eq!(
        json["data"]["attestedPredicates"],
        serde_json::json!(["legal-name-verified", "age-of-majority"])
    );
}

#[test]
fn identity_attestation_omits_valid_until_when_none() {
    let mut input = identity_attestation_input();
    input.valid_until = None;
    let record = ProvenanceRecord::identity_attestation(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert!(json["data"].get("validUntil").is_none());
}

#[test]
fn identity_attestation_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "subjectGlobalId".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = identity_attestation_input();
    input.context = Some(context);
    let record = ProvenanceRecord::identity_attestation(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["subjectGlobalId"], "did:example:applicant-001");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn identity_attestation_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::IdentityAttestation),
        "facts"
    );
}

#[test]
fn identity_attestation_round_trips_through_serde() {
    let record = ProvenanceRecord::identity_attestation(identity_attestation_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::IdentityAttestation);
}

// ── Clock skew (ADR 0069) ───────────────────────────────────────────────

fn clock_skew_observed_input() -> ClockSkewObservedInput<'static> {
    ClockSkewObservedInput {
        processor_authored_at: "2026-04-22T14:30:00.500Z",
        substrate_created_at: "2026-04-22T14:30:01.700Z",
        skew_milliseconds: -1200,
        threshold_milliseconds: 1000,
        event_hash: "sha256:event-skew-1",
        context: None,
    }
}

#[test]
fn clock_skew_observed_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::clock_skew_observed(clock_skew_observed_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "clockSkewObserved");
    assert_eq!(
        json["data"]["processorAuthoredAt"],
        "2026-04-22T14:30:00.500Z"
    );
    assert_eq!(
        json["data"]["substrateCreatedAt"],
        "2026-04-22T14:30:01.700Z"
    );
    assert_eq!(json["data"]["skewMilliseconds"], -1200);
    assert_eq!(json["data"]["thresholdMilliseconds"], 1000);
    assert_eq!(json["data"]["eventHash"], "sha256:event-skew-1");
}

#[test]
fn clock_skew_observed_skew_can_be_negative() {
    let record = ProvenanceRecord::clock_skew_observed(clock_skew_observed_input());
    let json = serde_json::to_value(&record).expect("serialize");
    let skew = json["data"]["skewMilliseconds"]
        .as_i64()
        .expect("signed integer");
    assert!(skew < 0, "negative skew (substrate ahead) MUST round-trip");
}

#[test]
fn clock_skew_observed_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "skewMilliseconds".to_string(),
        serde_json::Value::Number(serde_json::Number::from(99999)),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = clock_skew_observed_input();
    input.context = Some(context);
    let record = ProvenanceRecord::clock_skew_observed(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["skewMilliseconds"], -1200);
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn clock_skew_observed_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::ClockSkewObserved),
        "facts"
    );
}

#[test]
fn clock_skew_observed_round_trips_through_serde() {
    let record = ProvenanceRecord::clock_skew_observed(clock_skew_observed_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::ClockSkewObserved);
}

// ── Failure & compensation (ADR 0070) ───────────────────────────────────

fn commit_attempt_failure_input() -> CommitAttemptFailureInput<'static> {
    CommitAttemptFailureInput {
        target_event_hash: "sha256:target-1",
        failure_kind: CommitFailureKind::NetworkTimeout,
        attempt_count: 3,
        retry_budget_remaining_ms: 5000,
        error_payload: Some(serde_json::json!({"errno": "ETIMEDOUT"})),
        context: None,
    }
}

#[test]
fn commit_attempt_failure_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::commit_attempt_failure(commit_attempt_failure_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "commitAttemptFailure");
    assert_eq!(json["data"]["targetEventHash"], "sha256:target-1");
    assert_eq!(json["data"]["failureKind"], "networkTimeout");
    assert_eq!(json["data"]["attemptCount"], 3);
    assert_eq!(json["data"]["retryBudgetRemainingMs"], 5000);
    assert_eq!(json["data"]["errorPayload"]["errno"], "ETIMEDOUT");
}

#[test]
fn commit_attempt_failure_failure_kind_typed_enum_round_trips() {
    for (kind, expected) in [
        (CommitFailureKind::NetworkTimeout, "networkTimeout"),
        (CommitFailureKind::SubstrateDown, "substrateDown"),
        (CommitFailureKind::HashConflict, "hashConflict"),
        (CommitFailureKind::Other, "other"),
    ] {
        let mut input = commit_attempt_failure_input();
        input.failure_kind = kind;
        let record = ProvenanceRecord::commit_attempt_failure(input);
        let json = serde_json::to_value(&record).expect("serialize");
        assert_eq!(json["data"]["failureKind"], expected);
    }
}

#[test]
fn commit_attempt_failure_omits_error_payload_when_none() {
    let mut input = commit_attempt_failure_input();
    input.error_payload = None;
    let record = ProvenanceRecord::commit_attempt_failure(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert!(json["data"].get("errorPayload").is_none());
}

#[test]
fn commit_attempt_failure_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "failureKind".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = commit_attempt_failure_input();
    input.context = Some(context);
    let record = ProvenanceRecord::commit_attempt_failure(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["failureKind"], "networkTimeout");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn commit_attempt_failure_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::CommitAttemptFailure),
        "facts"
    );
}

#[test]
fn commit_attempt_failure_round_trips_through_serde() {
    let record = ProvenanceRecord::commit_attempt_failure(commit_attempt_failure_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::CommitAttemptFailure);
}

fn authorization_rejected_input() -> AuthorizationRejectedInput<'static> {
    AuthorizationRejectedInput {
        attempted_actor_id: "applicant-1",
        attempted_action: "transition:approve",
        target_resource_id: "case-2026-0001",
        rejection_reason: "actor lacks approve role",
        policy_decision_ref: Some("urn:policy:decision:abc123"),
        context: None,
    }
}

#[test]
fn authorization_rejected_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::authorization_rejected(authorization_rejected_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "authorizationRejected");
    assert_eq!(json["actorId"], "applicant-1");
    assert_eq!(json["data"]["attemptedActorId"], "applicant-1");
    assert_eq!(json["data"]["attemptedAction"], "transition:approve");
    assert_eq!(json["data"]["targetResourceId"], "case-2026-0001");
    assert_eq!(json["data"]["rejectionReason"], "actor lacks approve role");
    assert_eq!(
        json["data"]["policyDecisionRef"],
        "urn:policy:decision:abc123"
    );
}

#[test]
fn authorization_rejected_omits_policy_decision_ref_when_none() {
    let mut input = authorization_rejected_input();
    input.policy_decision_ref = None;
    let record = ProvenanceRecord::authorization_rejected(input);
    let json = serde_json::to_value(&record).expect("serialize");
    assert!(json["data"].get("policyDecisionRef").is_none());
}

#[test]
fn authorization_rejected_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "attemptedAction".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = authorization_rejected_input();
    input.context = Some(context);
    let record = ProvenanceRecord::authorization_rejected(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["data"]["attemptedAction"], "transition:approve");
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn authorization_rejected_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::AuthorizationRejected),
        "facts"
    );
}

#[test]
fn authorization_rejected_round_trips_through_serde() {
    let record = ProvenanceRecord::authorization_rejected(authorization_rejected_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::AuthorizationRejected);
}

// ── Migration & version pins (ADR 0071) ─────────────────────────────────

fn migration_pin_changed_input() -> MigrationPinChangedInput<'static> {
    MigrationPinChangedInput {
        prior_pin_set: serde_json::json!({
            "formspec.definitionVersion": "1.0.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "B"
        }),
        new_pin_set: serde_json::json!({
            "formspec.definitionVersion": "1.1.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "B"
        }),
        authorizing_actor_id: "platform-admin-1",
        authority_basis: serde_json::json!({"kind": "uri", "value": "https://agency.gov/auth/migrate"}),
        migration_rationale: "definition v1.1.0 enables new field",
        context: None,
    }
}

#[test]
fn migration_pin_changed_constructor_serializes_required_fields() {
    let record = ProvenanceRecord::migration_pin_changed(migration_pin_changed_input());
    let json = serde_json::to_value(&record).expect("serialize");

    assert_eq!(json["recordKind"], "migrationPinChanged");
    assert_eq!(json["actorId"], "platform-admin-1");
    assert_eq!(
        json["data"]["priorPinSet"]["formspec.definitionVersion"],
        "1.0.0"
    );
    assert_eq!(
        json["data"]["newPinSet"]["formspec.definitionVersion"],
        "1.1.0"
    );
    assert_eq!(json["data"]["authorizingActorId"], "platform-admin-1");
    assert_eq!(
        json["data"]["migrationRationale"],
        "definition v1.1.0 enables new field"
    );
    assert_eq!(json["data"]["authorityBasis"]["kind"], "uri");
}

#[test]
fn migration_pin_changed_pin_sets_carry_four_field_tree() {
    let record = ProvenanceRecord::migration_pin_changed(migration_pin_changed_input());
    let json = serde_json::to_value(&record).expect("serialize");
    for set_name in ["priorPinSet", "newPinSet"] {
        let set = &json["data"][set_name];
        for field in [
            "formspec.definitionVersion",
            "wos.$wosWorkflowVersion",
            "trellis.envelopeVersion",
            "trellis.conformanceClass",
        ] {
            assert!(
                set.get(field).is_some(),
                "{set_name} missing {field} (Q33 4-field pin tree)"
            );
        }
    }
}

#[test]
fn migration_pin_changed_drops_context_keys_that_collide_with_required_fields() {
    let mut context = serde_json::Map::new();
    context.insert(
        "newPinSet".to_string(),
        serde_json::Value::String("attacker".to_string()),
    );
    context.insert(
        "auxNote".to_string(),
        serde_json::Value::String("kept".to_string()),
    );
    let mut input = migration_pin_changed_input();
    input.context = Some(context);
    let record = ProvenanceRecord::migration_pin_changed(input);
    let json = serde_json::to_value(&record).expect("serialize");

    assert!(
        json["data"]["newPinSet"].is_object(),
        "constructor's typed pin set MUST win over context override"
    );
    assert_eq!(json["data"]["auxNote"], "kept");
}

#[test]
fn migration_pin_changed_classifies_as_facts() {
    assert_eq!(
        audit_layer_for_kind(ProvenanceKind::MigrationPinChanged),
        "facts"
    );
}

#[test]
fn migration_pin_changed_round_trips_through_serde() {
    let record = ProvenanceRecord::migration_pin_changed(migration_pin_changed_input());
    let json = serde_json::to_string(&record).expect("serialize");
    let restored: ProvenanceRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.record_kind, ProvenanceKind::MigrationPinChanged);
}
