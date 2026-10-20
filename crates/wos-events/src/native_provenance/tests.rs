use super::*;
use crate::{CustodyAppendContext, CustodyAppendInput};
use integrity_cbor::{encode_cbor_value, json_to_cbor_value, json_to_dcbor_bytes};
use stack_common_typeid::testing::fixture_minter;

fn record() -> ProvenanceRecord {
    let mut record = ProvenanceRecord::state_entered(&fixture_minter(), "Draft");
    record.timestamp = "2026-10-09T14:00:00.123Z".into();
    record.definition_version = Some("1.0.0".into());
    record.audit_layer = Some("facts".into());
    record.event = Some("original.trigger".into());
    record.data =
        Some(serde_json::json!({"nested":[null, true, {"original":"fact"}], "number":17}));
    record
}

fn payload() -> Value {
    serde_json::json!({"record": record()})
}

#[test]
fn native_provenance_registry_parity_and_every_known_route() {
    let registry: Value = serde_json::from_str(include_str!(
        "../../../../schemas/record-kind-registry.json"
    ))
    .unwrap();
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let mut remaining = Vec::new();
    let mut canonical = 0;
    for entry in registry["recordKinds"].as_array().unwrap() {
        let mut original = record();
        original.record_kind = serde_json::from_value(entry["literal"].clone()).unwrap();
        match original.record_kind.canonical_event_literal() {
            Some(literal) => {
                canonical += 1;
                assert_eq!(entry["eventLiteral"], literal);
                assert!(matches!(
                    NativeProvenanceEvent::from_record(&original),
                    Err(NativeProvenanceError::CanonicalKind)
                ));
            }
            None => {
                assert!(entry.get("eventLiteral").is_none());
                remaining.push(entry["literal"].clone());
                if matches!(
                    original.record_kind,
                    ProvenanceKind::ReasoningTierRecorded
                        | ProvenanceKind::CounterfactualTierRecorded
                ) {
                    original = human_record(original.record_kind);
                }
                let event = NativeProvenanceEvent::from_record(&original).unwrap();
                assert_eq!(event.record_kind(), original.record_kind);
                assert_eq!(event.record(), &serde_json::to_value(&original).unwrap());
                assert_eq!(event.record_id(), original.id);
            }
        }
    }
    let entries = registry["recordKinds"].as_array().unwrap();
    assert_eq!(
        canonical,
        entries
            .iter()
            .filter(|entry| entry.get("eventLiteral").is_some())
            .count()
    );
    assert_eq!(
        canonical, 30,
        "existing specialized routes remain unchanged"
    );
    assert_eq!(
        entries.len(),
        registry["totalCount"].as_u64().unwrap() as usize
    );
    assert_eq!(remaining.len(), entries.len() - canonical);
    assert_eq!(remaining.len(), 108);
    assert_eq!(registry["registryVersion"], "1.4.0");
    assert_eq!(
        registry["schemaValidatedCount"],
        entries
            .iter()
            .filter(|entry| entry["schemaValidated"] == true)
            .count()
    );
    assert_eq!(
        schema["properties"]["record"]["properties"]["recordKind"]["enum"],
        Value::Array(remaining)
    );
}

#[test]
fn native_provenance_preserves_original_json_and_actual_timestamp_tag() {
    let original = record();
    let context = CustodyAppendContext {
        event_type_prefix: "wos.kernel".into(),
        case_id: None,
        max_inline_record_bytes: None,
        workflow_ref: None,
    };
    let metadata = context
        .metadata_for_provenance_record(&fixture_minter().mint_case_ledger_id(), 0, &original)
        .unwrap();
    assert_eq!(metadata.event_type, WOS_NATIVE_PROVENANCE_EVENT_LITERAL);
    let input = CustodyAppendInput::from_provenance_record(&original, &context, metadata).unwrap();
    let bytes_before = input.record.clone();
    let decoded = decode_cbor_value(input.record_bytes()).unwrap();
    let CborValue::Map(fields) = decoded else {
        panic!("record map")
    };
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == &CborValue::Text("timestamp".into())
                && matches!(value, CborValue::Tag(0, _)))
    );
    let event = NativeProvenanceEvent::from_record_dcbor_slice(input.record_bytes()).unwrap();
    assert_eq!(event.record(), &serde_json::to_value(&original).unwrap());
    assert_eq!(input.record, bytes_before);
    let wrapped = json_to_dcbor_bytes(event.payload(), &[]).unwrap();
    assert_eq!(
        NativeProvenanceEvent::from_dcbor_slice(&wrapped)
            .unwrap()
            .payload(),
        event.payload()
    );
}

#[test]
fn native_provenance_preserves_native_task_expiry_without_broadening_other_outcomes() {
    // Mirrors retire_expired_tasks: preserve its maintained constructor,
    // system actor, detail shape and unmodified native outcome literal.
    let mut expired = ProvenanceRecord::task_lifecycle(
        &fixture_minter(),
        ProvenanceKind::TaskFailed,
        "review-task",
        None,
        Some(
            serde_json::json!({"taskStatus":"expired", "outcome":"expired",
            "expiresAt":"2026-10-09T14:00:00Z", "timerRef":null}),
        ),
    );
    expired.timestamp = "2026-10-09T14:00:00Z".into();
    expired.actor_type = Some("system".into());
    expired.outcome = Some("expired".into());
    expired.audit_layer = Some("facts".into());
    expired.definition_version = Some("1.0.0".into());
    let native = NativeProvenanceEvent::from_record(&expired).unwrap();
    assert_eq!(native.record(), &serde_json::to_value(&expired).unwrap());
    let bytes = json_to_dcbor_bytes(native.payload(), &[]).unwrap();
    assert_eq!(
        NativeProvenanceEvent::from_dcbor_slice(&bytes)
            .unwrap()
            .record(),
        native.record()
    );
    expired.record_kind = ProvenanceKind::TaskCreated;
    assert!(NativeProvenanceEvent::from_record(&expired).is_err());
    expired.record_kind = ProvenanceKind::TaskFailed;
    expired.outcome = Some("unknown-native-outcome".into());
    assert!(NativeProvenanceEvent::from_record(&expired).is_err());
}

#[test]
fn native_provenance_refuses_missing_unknown_and_malformed_fields() {
    let mut invalids = Vec::new();
    for required in [
        "id",
        "recordKind",
        "timestamp",
        "auditLayer",
        "definitionVersion",
    ] {
        let mut value = payload();
        value["record"].as_object_mut().unwrap().remove(required);
        invalids.push(value);
    }
    for (field, value) in [
        ("recordKind", serde_json::json!("x-unknown")),
        ("recordKind", Value::Null),
        ("id", serde_json::json!("")),
        ("id", serde_json::json!("arbitrary-id")),
        ("timestamp", serde_json::json!("not-a-time")),
        ("timestamp", serde_json::json!("")),
        ("auditLayer", serde_json::json!("unknown")),
        ("inputs", serde_json::json!([7])),
        ("fromState", serde_json::json!({})),
        ("definitionVersion", serde_json::json!("")),
        ("canonicalEventHash", serde_json::json!("invalid")),
    ] {
        let mut p = payload();
        p["record"][field] = value;
        invalids.push(p);
    }
    let mut extra = payload();
    extra["unexpected"] = true.into();
    invalids.push(extra);
    let mut extra = payload();
    extra["record"]["unexpected"] = true.into();
    invalids.push(extra);
    invalids.extend([
        Value::Null,
        serde_json::json!({"record": []}),
        serde_json::json!({}),
    ]);
    for value in invalids {
        assert!(
            NativeProvenanceEvent::from_json(&value).is_err(),
            "accepted {value}"
        );
    }
}

#[test]
fn native_provenance_json_and_cbor_share_numeric_and_depth_bounds() {
    for number in [serde_json::json!(u64::MAX), serde_json::json!(-0.0)] {
        let mut value = payload();
        value["record"]["data"] = number;
        assert!(NativeProvenanceEvent::from_json(&value).is_err());
    }
    let mut value = payload();
    value["record"]["data"] = serde_json::json!([i64::MIN, i64::MAX, 1.25]);
    assert!(NativeProvenanceEvent::from_json(&value).is_ok());
    assert!(
        NativeProvenanceEvent::from_dcbor_slice(&json_to_dcbor_bytes(&value, &[]).unwrap()).is_ok()
    );
    let mut nested = Value::Null;
    for _ in 0..126 {
        nested = Value::Array(vec![nested]);
    }
    value["record"]["data"] = nested;
    assert!(NativeProvenanceEvent::from_json(&value).is_ok());
    assert!(
        NativeProvenanceEvent::from_dcbor_slice(&json_to_dcbor_bytes(&value, &[]).unwrap()).is_ok()
    );
    value["record"]["data"] = Value::Array(vec![value["record"]["data"].take()]);
    assert!(NativeProvenanceEvent::from_json(&value).is_err());
    assert!(
        NativeProvenanceEvent::from_dcbor_slice(&json_to_dcbor_bytes(&value, &[]).unwrap())
            .is_err()
    );
}

#[test]
fn native_provenance_refuses_duplicate_keys_before_json_conversion() {
    let mut wrapped = json_to_cbor_value(&payload(), &[]).unwrap();
    let CborValue::Map(fields) = &mut wrapped else {
        panic!("wrapper map")
    };
    fields.push(fields[0].clone());
    assert!(matches!(
        NativeProvenanceEvent::from_dcbor_slice(&encode_cbor_value(&wrapped).unwrap()),
        Err(NativeProvenanceError::InvalidCbor(_))
    ));
    let mut bare = json_to_cbor_value(&serde_json::to_value(record()).unwrap(), &[]).unwrap();
    let CborValue::Map(fields) = &mut bare else {
        panic!("record map")
    };
    let discriminator = fields
        .iter()
        .find(|(key, _)| key == &CborValue::Text("recordKind".into()))
        .unwrap()
        .clone();
    fields.push(discriminator);
    assert!(matches!(
        NativeProvenanceEvent::from_record_dcbor_slice(&encode_cbor_value(&bare).unwrap()),
        Err(NativeProvenanceError::InvalidCbor(_))
    ));
}

#[test]
fn native_provenance_refuses_bytes_tags_and_trailing_input() {
    for bad in [
        CborValue::Bytes(vec![1]),
        CborValue::Tag(32, Box::new(CborValue::Text("uri".into()))),
        CborValue::Tag(0, Box::new(CborValue::Text("2026-10-09T00:00:00Z".into()))),
    ] {
        let mut wrapped = json_to_cbor_value(&payload(), &[]).unwrap();
        let CborValue::Map(fields) = &mut wrapped else {
            panic!("wrapper map")
        };
        fields.push((CborValue::Text("extra".into()), bad));
        assert!(matches!(
            NativeProvenanceEvent::from_dcbor_slice(&encode_cbor_value(&wrapped).unwrap()),
            Err(NativeProvenanceError::InvalidCbor(_))
        ));
    }
    for (field, bad) in [
        (
            "data",
            CborValue::Tag(0, Box::new(CborValue::Text("2026-10-09T00:00:00Z".into()))),
        ),
        (
            "data",
            CborValue::Map(vec![(
                CborValue::Text("timestamp".into()),
                CborValue::Tag(0, Box::new(CborValue::Text("2026-10-09T00:00:00Z".into()))),
            )]),
        ),
        (
            "timestamp",
            CborValue::Tag(0, Box::new(CborValue::Integer(7.into()))),
        ),
        (
            "timestamp",
            CborValue::Tag(
                0,
                Box::new(CborValue::Tag(
                    0,
                    Box::new(CborValue::Text("2026-10-09T00:00:00Z".into())),
                )),
            ),
        ),
    ] {
        let mut bare = json_to_cbor_value(&serde_json::to_value(record()).unwrap(), &[]).unwrap();
        let CborValue::Map(fields) = &mut bare else {
            panic!("record map")
        };
        let (_, value) = fields
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Text(field.into()))
            .unwrap();
        *value = bad;
        assert!(matches!(
            NativeProvenanceEvent::from_record_dcbor_slice(&encode_cbor_value(&bare).unwrap()),
            Err(NativeProvenanceError::InvalidCbor(_))
        ));
    }
    let mut bytes = json_to_dcbor_bytes(&payload(), &[]).unwrap();
    bytes.push(0);
    assert!(NativeProvenanceEvent::from_dcbor_slice(&bytes).is_err());
}

fn human_record(kind: ProvenanceKind) -> ProvenanceRecord {
    let mut original = record();
    original.record_kind = kind;
    original.audit_layer = Some(crate::provenance::audit_layer_for_kind(kind).into());
    original.data = Some(match kind {
        ProvenanceKind::ReasoningTierRecorded => serde_json::json!({
            "factsRecordRef":"actual-facts-id","snapshotSha256":"a".repeat(64),
            "sourceRef":"accepted-task-artifact","reason":"This evidence does not meet the selected criterion.",
            "rulesApplied":[{"ruleId":"rule:evidence","sourceAuthority":"regulation"}],
            "evidenceConsulted":[{"kind":"caseField","caseFieldPath":"/evidence"}],
            "criteriaChecked":[{"criterion":"sufficientEvidence","satisfied":false}],
            "adverseNotice":{"factsRecordRef":"actual-facts-id","snapshotSha256":"a".repeat(64),
                "templateKey":"decisionNotice","templateSha256":format!("sha256:{}","b".repeat(64)),
                "machineReadable":{"reason":"The evidence does not meet the selected criterion."},
                "humanReadable":"The evidence does not meet the selected criterion.",
                "contentSha256":format!("sha256:{}","c".repeat(64))}}),
        ProvenanceKind::CounterfactualTierRecorded => serde_json::json!({
            "factsRecordRef":"actual-facts-id","snapshotSha256":"a".repeat(64),
            "sourceRef":"accepted-task-artifact",
            "positiveCounterfactuals":[{"polarity":"positive","factor":"evidence","effect":"Clear evidence would permit reconsideration."}],
            "negativeCounterfactuals":[{"polarity":"negative","factor":"sex","effect":"Sex did not enter this assessment."}]}),
        _ => panic!("human kind required"),
    });
    original
}

#[test]
fn native_provenance_human_evidence_roundtrip_retains_business_event_and_data() {
    for kind in [
        ProvenanceKind::ReasoningTierRecorded,
        ProvenanceKind::CounterfactualTierRecorded,
    ] {
        let original = human_record(kind);
        assert_eq!(kind.canonical_event_literal(), None);
        let value = serde_json::to_value(&original).unwrap();
        let admitted = NativeProvenanceEvent::from_record(&original).unwrap();
        assert_eq!(admitted.record(), &value);
        assert_eq!(admitted.record()["event"], "original.trigger");
        let bytes = json_to_dcbor_bytes(admitted.payload(), &[]).unwrap();
        assert_eq!(
            NativeProvenanceEvent::from_dcbor_slice(&bytes).unwrap(),
            admitted
        );
        match kind {
            ProvenanceKind::ReasoningTierRecorded => {
                let typed: crate::governance::ReasoningEvidence =
                    serde_json::from_value(value["data"].clone()).unwrap();
                assert_eq!(serde_json::to_value(typed).unwrap(), value["data"]);
            }
            _ => {
                let typed: crate::governance::CounterfactualEvidence =
                    serde_json::from_value(value["data"].clone()).unwrap();
                assert_eq!(serde_json::to_value(typed).unwrap(), value["data"]);
            }
        }
    }
}

#[test]
fn native_provenance_human_closed_shapes_and_original_association_fields_refuse() {
    for kind in [
        ProvenanceKind::ReasoningTierRecorded,
        ProvenanceKind::CounterfactualTierRecorded,
    ] {
        let value = serde_json::json!({"record": human_record(kind)});
        for key in ["factsRecordRef", "snapshotSha256", "sourceRef"] {
            let mut bad = value.clone();
            bad["record"]["data"].as_object_mut().unwrap().remove(key);
            assert!(
                NativeProvenanceEvent::from_json(&bad).is_err(),
                "missing {key}"
            );
        }
        for (pointer, replacement) in [
            (
                "/record/data/snapshotSha256",
                serde_json::json!("sha256:wrong"),
            ),
            ("/record/data/sourceRef", serde_json::json!("")),
            ("/record/auditLayer", serde_json::json!("facts")),
            (
                "/record/recordKind",
                serde_json::json!("reasoningTierRecordedButUnknown"),
            ),
        ] {
            let mut bad = value.clone();
            *bad.pointer_mut(pointer).unwrap() = replacement;
            assert!(NativeProvenanceEvent::from_json(&bad).is_err(), "{pointer}");
        }
        let mut bad = value.clone();
        bad["record"]["data"]["undeclared"] = serde_json::json!(true);
        assert!(NativeProvenanceEvent::from_json(&bad).is_err());
    }
    let reasoning =
        serde_json::json!({"record":human_record(ProvenanceKind::ReasoningTierRecorded)});
    for (pointer, replacement) in [
        (
            "/record/data/evidenceConsulted/0",
            serde_json::json!({"kind":"document"}),
        ),
        (
            "/record/data/evidenceConsulted/0",
            serde_json::json!({"kind":"document","uri":"not a URI"}),
        ),
        (
            "/record/data/adverseNotice/templateSha256",
            serde_json::json!("b".repeat(64)),
        ),
        (
            "/record/data/adverseNotice/contentSha256",
            serde_json::json!("wrong"),
        ),
        (
            "/record/data/adverseNotice/machineReadable",
            serde_json::json!("not an object"),
        ),
    ] {
        let mut bad = reasoning.clone();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        assert!(NativeProvenanceEvent::from_json(&bad).is_err(), "{pointer}");
    }
    let mut counter =
        serde_json::json!({"record":human_record(ProvenanceKind::CounterfactualTierRecorded)});
    counter["record"]["data"]["positiveCounterfactuals"][0]["polarity"] =
        serde_json::json!("unknown");
    assert!(NativeProvenanceEvent::from_json(&counter).is_err());
    assert!(
        serde_json::from_value::<crate::governance::CounterfactualEvidence>(
            counter["record"]["data"].clone()
        )
        .is_err()
    );
    // Changing the raw kind cannot reuse the opposite human payload.
    let mut wrong = reasoning;
    wrong["record"]["recordKind"] = serde_json::json!("counterfactualTierRecorded");
    wrong["record"]["auditLayer"] = serde_json::json!("counterfactual");
    assert!(NativeProvenanceEvent::from_json(&wrong).is_err());
}

#[test]
fn native_provenance_shared_evidence_definitions_match_existing_api_owner() {
    let native: Value = serde_json::from_str(SCHEMA).unwrap();
    let api: Value = serde_json::from_str(include_str!(
        "../../../../schemas/api/provenance.schema.json"
    ))
    .unwrap();
    for name in [
        "RuleReference",
        "SourceAuthority",
        "EvidenceReference",
        "EvidenceKind",
        "CriterionResult",
        "Counterfactual",
        "CounterfactualPolarity",
    ] {
        assert_eq!(native["$defs"][name], api["$defs"][name], "{name}");
    }
}
