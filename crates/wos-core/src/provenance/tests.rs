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
        formspec_response_ref: "urn:agency.gov:formspec:responses:benefits:case-2026-0001",
        custody_hook_eligible: true,
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
        json["data"]["formspecResponseRef"],
        "urn:agency.gov:formspec:responses:benefits:case-2026-0001"
    );
    assert_eq!(json["data"]["custodyHookEligible"], true);
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
    assert_eq!(json["data"]["unresolvedRef"], "DemotionRule.id::nonexistent");
    assert_eq!(json["data"]["workflowUri"], "urn:agency.gov:wos:benefits:v1");
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
    assert_eq!(restored.outcome.as_deref(), Some("preconditionNotSatisfied"));
    assert_eq!(restored.actor_id.as_deref(), Some("intake-classifier"));
}
