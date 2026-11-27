// Rust guideline compliant 2026-02-21
//! Selected task rechecks compare the nested filed data without rewriting it.

use serde_json::{Value, json};
use wos_formspec_binding::{
    FormspecProcessor,
    selected::{ImmutableFormspecProcessor, SelectedDocument, SelectedDocumentKind},
};
use wos_runtime::BindingError;

const URL: &str = "urn:test:selected-evaluation";
const VERSION: &str = "1.0.0";

struct Clock;
impl wos_runtime::Clock for Clock {
    fn now_ms(&self) -> u64 {
        0
    }
}

fn definition(items: Value, binds: Value) -> Value {
    json!({
        "$formspec": "1.0", "url": URL, "version": VERSION,
        "status": "draft", "title": "Selected task", "items": items, "binds": binds
    })
}

fn processor(definition: &Value) -> ImmutableFormspecProcessor {
    ImmutableFormspecProcessor::new(&[SelectedDocument {
        kind: SelectedDocumentKind::Definition,
        url: definition["url"].as_str().unwrap().into(),
        version: definition["version"].as_str().unwrap().into(),
        bytes: serde_json::to_vec(definition).unwrap(),
        definition: None,
    }])
    .unwrap()
    .with_clock(Clock)
}

fn repeat() -> Value {
    json!({"key":"rows", "type":"group", "label":"Rows", "repeatable":true,
        "children":[{"key":"x", "type":"field", "dataType":"integer", "label":"X"}]})
}

#[test]
fn nested_group_and_repeat_response_passes_selected_recheck() {
    let definition = definition(
        json!([
            {"key":"g", "type":"group", "label":"Group", "children":[
                {"key":"a", "type":"field", "dataType":"string", "label":"A"}, repeat()
            ]}
        ]),
        json!([]),
    );
    assert_eq!(
        processor(&definition)
            .validate_response_definition(URL, VERSION, &json!({"data":{"g":{"a":"filed", "rows":[{"x":3}]}}}))
            .unwrap(),
        None
    );
}

#[test]
fn repeat_response_passes_selected_recheck() {
    let definition = definition(json!([repeat()]), json!([]));
    assert_eq!(
        processor(&definition)
            .validate_response_definition(URL, VERSION, &json!({"data":{"rows":[{"x":3},{"x":5}]}}))
            .unwrap(),
        None
    );
}

#[test]
fn nested_calculated_field_passes_when_filed_value_matches() {
    let definition = calculated_definition();
    assert_eq!(
        processor(&definition)
            .validate_response_definition(URL, VERSION, &json!({"data":{"g":{"a":3,"double":6}}}))
            .unwrap(),
        None
    );
}

fn calculated_definition() -> Value {
    definition(
        json!([
            {"key":"g", "type":"group", "label":"Group", "children":[
                {"key":"a", "type":"field", "dataType":"integer", "label":"A"},
                {"key":"double", "type":"field", "dataType":"integer", "label":"Double"}
            ]}
        ]),
        json!([{"path":"g.double", "calculate":"$g.a * 2"}]),
    )
}

#[test]
fn changed_or_missing_calculated_value_is_still_refused() {
    let processor = processor(&calculated_definition());
    for data in [json!({"g":{"a":3,"double":7}}), json!({"g":{"a":3}})] {
        assert!(matches!(processor.validate_response_definition(URL, VERSION, &json!({"data":data})),
            Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation"));
    }
}

#[test]
fn omitted_empty_repeat_and_its_enclosing_groups_are_unchanged() {
    let mut top_repeat = repeat();
    top_repeat["key"] = json!("topRows");
    top_repeat["children"][0]["key"] = json!("topX");
    let definition = definition(
        json!([
            top_repeat,
            {"key":"g", "type":"group", "label":"Group", "children":[
                {"key":"inner", "type":"group", "label":"Inner", "children":[repeat()]}
            ]}
        ]),
        json!([]),
    );
    let processor = processor(&definition);
    for data in [
        json!({}),
        json!({"g":{}}),
        json!({"g":{"inner":{}}}),
        json!({"topRows":[],"g":{"inner":{"rows":[]}}}),
    ] {
        assert_eq!(
            processor.validate_response_definition(URL, VERSION, &json!({"data":data})).unwrap(),
            None
        );
    }
}

#[test]
fn omitted_empty_repeat_beside_a_non_relevant_repeat_is_unchanged() {
    let mut hidden_repeat = repeat();
    hidden_repeat["key"] = json!("hidden");
    hidden_repeat["children"][0]["key"] = json!("hiddenX");
    let definition = definition(
        json!([repeat(), hidden_repeat]),
        json!([{"path":"hidden", "relevant":"false"}]),
    );
    assert_eq!(
        processor(&definition)
            .validate_response_definition(URL, VERSION, &json!({"data":{}}))
            .unwrap(),
        None
    );
}

#[test]
fn removal_of_a_filed_value_is_still_refused() {
    let definition = definition(
        json!([
            repeat(), {"key":"hidden", "type":"field", "dataType":"string", "label":"Hidden"}
        ]),
        json!([{"path":"hidden", "relevant":"false"}]),
    );
    assert!(
        matches!(processor(&definition).validate_response_definition(URL, VERSION, &json!({"data":{"hidden":"filed"}})),
        Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation")
    );
}

#[test]
fn omitted_empty_repeat_inside_existing_rows_is_unchanged() {
    let definition = definition(
        json!([
            {"key":"outer", "type":"group", "label":"Outer", "repeatable":true,
                "children":[{"key":"g", "type":"group", "label":"Group", "children":[repeat()]}]}
        ]),
        json!([]),
    );
    assert_eq!(
        processor(&definition)
            .validate_response_definition(URL, VERSION, &json!({"data":{"outer":[{}, {"g":{"rows":[]}}]}}))
            .unwrap(),
        None
    );
}

#[test]
fn empty_repeat_exception_does_not_allow_a_missing_calculated_value() {
    let definition = definition(
        json!([
            repeat(), {"key":"added", "type":"field", "dataType":"integer", "label":"Added"}
        ]),
        json!([{"path":"added", "calculate":"42"}]),
    );
    assert!(
        matches!(processor(&definition).validate_response_definition(URL, VERSION, &json!({"data":{}})),
        Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation")
    );
}

#[test]
fn absent_group_with_calculated_value_beside_empty_repeat_is_refused() {
    let definition = definition(
        json!([
            {"key":"g", "type":"group", "label":"Group", "children":[
                {"key":"c", "type":"field", "dataType":"integer", "label":"Calculated"},
                repeat()
            ]}
        ]),
        json!([{"path":"g.c", "calculate":"42"}]),
    );
    let evaluated = formspec_eval::evaluate(
        &definition.clone().into(),
        &Default::default(),
        &formspec_eval::EvalOptions::default().trigger(formspec_eval::EvalTrigger::Submit),
    );
    assert_eq!(evaluated.data, json!({"g":{"c":42,"rows":[]}}));
    assert!(
        matches!(processor(&definition).validate_response_definition(URL, VERSION, &json!({"data":{}})),
        Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation")
    );
}

#[test]
fn saved_response_with_no_rows_keeps_zero_rows_and_reports_min_repeat() {
    let mut rows = repeat();
    rows["minRepeat"] = json!(1);
    let definition = definition(json!([rows]), json!([]));
    let evaluated = formspec_eval::evaluate(
        &definition.clone().into(),
        &Default::default(),
        &formspec_eval::EvalOptions::default().trigger(formspec_eval::EvalTrigger::Submit),
    );
    assert_eq!(evaluated.data, json!({"rows":[]}));
    // A saved Response keeps its zero rows and receives a cardinality finding.
    let findings = processor(&definition)
        .validate_response_definition(URL, VERSION, &json!({"data":{}}))
        .unwrap()
        .unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["path"], "rows");
    assert_eq!(findings[0]["code"], "MIN_REPEAT");
}

#[test]
fn nested_required_error_reaches_the_task_caller() {
    let definition = definition(
        json!([
            {"key":"g", "type":"group", "label":"Group", "children":[
                {"key":"a", "type":"field", "dataType":"string", "label":"A"}
            ]}
        ]),
        json!([{"path":"g.a", "required":"true"}]),
    );
    let errors = processor(&definition)
        .validate_response_definition(URL, VERSION, &json!({"data":{"g":{"a":null}}}))
        .unwrap()
        .unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error["path"] == "g.a" && error["code"] == "REQUIRED"),
        "{errors:?}"
    );
}

#[test]
fn stored_subject_survives_native_task_recheck() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../formspec/tests/conformance/fixtures/subject-recheck.json"
    )).unwrap();
    let definition = &fixture["definition"];
    let response = &fixture["response"];
    let task: wos_core::instance::ActiveTask = serde_json::from_value(json!({
        "taskId":"subject-recheck", "taskRef":"review", "status":"assigned", "binding":"formspec",
        "definitionUrl":response["definitionUrl"], "definitionVersion":response["definitionVersion"],
        "createdAt":"2026-01-01T00:00:00Z", "updatedAt":"2026-01-01T00:00:00Z"
    })).unwrap();
    let processor = processor(definition);
    assert_eq!(processor.validate_task_definition(&task, response).unwrap(), None);
    let mut invalid = response.clone();
    invalid["subject"]["keys"]["serial"] = json!("wrong type");
    let findings = processor.validate_task_definition(&task, &invalid).unwrap().unwrap();
    assert_eq!(findings[0]["path"], "#subjectKeys.serial");
    invalid["subject"]["keys"]["serial"] = json!(9_007_199_254_740_993_i64);
    let findings = processor.validate_response_definition(
        response["definitionUrl"].as_str().unwrap(),
        response["definitionVersion"].as_str().unwrap(),
        &invalid,
    ).unwrap().unwrap();
    assert_eq!(findings[0]["path"], "#subjectKeys.serial");
    assert!(findings[0]["message"].as_str().unwrap().contains("I-JSON"));
    let mut changed = response.clone();
    changed["data"]["serial"] = json!(4);
    assert!(matches!(processor.validate_task_definition(&task, &changed),
        Err(BindingError::Unsupported(_))));
}

fn registry_doc_bytes(entries: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "$formspecRegistry": "1.1",
        "publisher": {
            "name": "Example Registry Publisher",
            "url": "https://example.test/registry"
        },
        "published": "2026-01-01T00:00:00Z",
        "entries": entries
    }))
    .unwrap()
}

#[test]
fn selected_recheck_enforces_pinned_registry_extensions_and_rejects_invalid_registries() {
    let def = definition(
        json!([
            {
                "key": "code",
                "type": "field",
                "dataType": "string",
                "label": "Code",
                "extensions": { "x-test-code": true }
            }
        ]),
        json!([]),
    );
    let registry_bytes = registry_doc_bytes(json!([
        {
            "name": "x-test-code",
            "category": "dataType",
            "baseType": "string",
            "version": "1.0.0",
            "status": "stable",
            "description": "Patterned alphanumeric code",
            "compatibility": { "formspecVersion": ">=1.0.0 <2.0.0" },
            "metadata": { "displayName": "Test Code" },
            "constraints": { "pattern": "^[A-Z]{2}-[0-9]{3}$" }
        }
    ]));

    // Without pinned registries, Definition admission fails closed on the unresolved extension.
    let missing = ImmutableFormspecProcessor::new(&[SelectedDocument {
        kind: SelectedDocumentKind::Definition,
        url: URL.into(),
        version: VERSION.into(),
        bytes: serde_json::to_vec(&def).unwrap(),
        definition: None,
    }]);
    assert!(missing.is_err());

    // With pinned registry, valid answers pass with zero UNRESOLVED_EXTENSION findings.
    let processor = ImmutableFormspecProcessor::new(&[
        SelectedDocument {
            kind: SelectedDocumentKind::Registry,
            url: "urn:test:registry:codes".into(),
            version: "1.0.0".into(),
            bytes: registry_bytes.clone(),
            definition: None,
        },
        SelectedDocument {
            kind: SelectedDocumentKind::Definition,
            url: URL.into(),
            version: VERSION.into(),
            bytes: serde_json::to_vec(&def).unwrap(),
            definition: None,
        },
    ])
    .unwrap()
    .with_clock(Clock);
    assert_eq!(
        processor
            .validate_response_definition(URL, VERSION, &json!({"data": {"code": "AB-123"}}))
            .unwrap(),
        None
    );

    // Violating the pinned registry pattern produces PATTERN_MISMATCH (never UNRESOLVED_EXTENSION).
    let findings = processor
        .validate_response_definition(URL, VERSION, &json!({"data": {"code": "bad"}}))
        .unwrap()
        .unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["code"], "PATTERN_MISMATCH");
    assert_eq!(findings[0]["path"], "code");

    // Duplicate extension entries across pinned registries fail closed.
    let dup = ImmutableFormspecProcessor::new(&[
        SelectedDocument {
            kind: SelectedDocumentKind::Registry,
            url: "urn:test:registry:a".into(),
            version: "1.0.0".into(),
            bytes: registry_bytes.clone(),
            definition: None,
        },
        SelectedDocument {
            kind: SelectedDocumentKind::Registry,
            url: "urn:test:registry:b".into(),
            version: "1.0.0".into(),
            bytes: registry_bytes,
            definition: None,
        },
    ]);
    assert!(dup.is_err());

    // Unsupported constraint keys fail closed.
    let unsupported = registry_doc_bytes(json!([
        {
            "name": "x-test-unsupported",
            "category": "dataType",
            "version": "1.0.0",
            "status": "stable",
            "description": "Unsupported constraint key",
            "compatibility": { "formspecVersion": ">=1.0.0 <2.0.0" },
            "constraints": { "minLength": 2 }
        }
    ]));
    assert!(
        ImmutableFormspecProcessor::new(&[SelectedDocument {
            kind: SelectedDocumentKind::Registry,
            url: "urn:test:registry:unsupported".into(),
            version: "1.0.0".into(),
            bytes: unsupported,
            definition: None,
        }])
        .is_err()
    );
}

#[test]
fn selected_recheck_preserves_exact_28_digit_decimals_against_exact_registry_bounds() {
    let def = definition(
        json!([
            {
                "key": "rate",
                "type": "field",
                "dataType": "decimal",
                "label": "Rate",
                "extensions": { "x-test-exact-rate": true }
            }
        ]),
        json!([]),
    );
    // Use raw JSON text so exact 28-digit bounds are never routed through f64.
    let registry_raw = br#"{
        "$formspecRegistry": "1.1",
        "publisher": {
            "name": "Example Registry Publisher",
            "url": "https://example.test/registry"
        },
        "published": "2026-01-01T00:00:00Z",
        "entries": [
            {
                "name": "x-test-exact-rate",
                "category": "dataType",
                "version": "1.0.0",
                "status": "stable",
                "description": "High-precision decimal bound",
                "compatibility": { "formspecVersion": ">=1.0.0 <2.0.0" },
                "baseType": "decimal",
                "constraints": {
                    "minimum": 0.1234567890123456789012345678,
                    "maximum": 0.1234567890123456789012345678
                }
            }
        ]
    }"#;

    let processor = ImmutableFormspecProcessor::new(&[
        SelectedDocument {
            kind: SelectedDocumentKind::Registry,
            url: "urn:test:registry:exact".into(),
            version: "1.0.0".into(),
            bytes: registry_raw.to_vec(),
            definition: None,
        },
        SelectedDocument {
            kind: SelectedDocumentKind::Definition,
            url: URL.into(),
            version: VERSION.into(),
            bytes: serde_json::to_vec(&def).unwrap(),
            definition: None,
        },
    ])
    .unwrap()
    .with_clock(Clock);

    let binding = wos_formspec_binding::FormspecBinding::new(processor.clone());
    let task: wos_core::instance::ActiveTask = serde_json::from_value(json!({
        "taskId": "exact-decimal-task",
        "taskRef": "review",
        "status": "assigned",
        "binding": "formspec",
        "definitionUrl": URL,
        "definitionVersion": VERSION,
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }))
    .unwrap();

    // Both numbers collapse to the same IEEE-754 f64; exact JSON must distinguish digit 28.
    let exact_ok = integrity_canonical::ResponseJson::parse(format!(
        r#"{{"$formspecResponse":"1.0","definitionUrl":"{URL}","definitionVersion":"{VERSION}","status":"completed","authored":"2026-01-01T00:00:00Z","data":{{"rate":0.1234567890123456789012345678}}}}"#
    ))
    .unwrap();
    let exact_over = integrity_canonical::ResponseJson::parse(format!(
        r#"{{"$formspecResponse":"1.0","definitionUrl":"{URL}","definitionVersion":"{VERSION}","status":"completed","authored":"2026-01-01T00:00:00Z","data":{{"rate":0.1234567890123456789012345679}}}}"#
    ))
    .unwrap();

    assert_eq!(
        processor
            .validate_response_definition_json(URL, VERSION, &exact_ok)
            .unwrap(),
        None
    );
    let over_findings = processor
        .validate_response_definition_json(URL, VERSION, &exact_over)
        .unwrap()
        .unwrap();
    assert_eq!(over_findings.len(), 1);
    assert_eq!(over_findings[0]["code"], "RANGE_OVERFLOW");
    assert_eq!(over_findings[0]["path"], "rate");

    let ok_outcome = wos_runtime::ContractBindingAdapter::validate_submission_json(
        &binding, &task, &exact_ok,
    )
    .unwrap();
    assert!(ok_outcome.validation_outcome.definition_valid);

    let over_outcome = wos_runtime::ContractBindingAdapter::validate_submission_json(
        &binding,
        &task,
        &exact_over,
    )
    .unwrap();
    assert!(!over_outcome.validation_outcome.definition_valid);
    assert_eq!(
        over_outcome.validation_outcome.validation_results.unwrap()[0]["code"],
        "RANGE_OVERFLOW"
    );
}

#[test]
fn selected_recheck_enforces_trusted_remote_option_lists_and_fails_closed_when_absent_or_tampered() {
    let def = json!({
        "$formspec": "1.0",
        "url": URL,
        "version": VERSION,
        "status": "draft",
        "title": "Remote options task",
        "optionSets": {
            "regions": {
                "source": "https://example.test/options/regions",
                "valueField": "code",
                "labelField": "name"
            }
        },
        "items": [
            {
                "key": "region",
                "type": "field",
                "dataType": "choice",
                "label": "Region",
                "optionSet": "regions"
            },
            {
                "key": "category",
                "type": "field",
                "dataType": "choice",
                "label": "Category",
                "options": "urn:example:options:categories"
            }
        ]
    });

    // Without resolved option lists, remote option fields fail closed with OPTIONS_UNAVAILABLE.
    let bare = processor(&def);
    let missing = bare
        .validate_response_definition(
            URL,
            VERSION,
            &json!({"data": {"region": "north", "category": "cat-a"}}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(missing.len(), 2);
    assert!(missing.iter().all(|f| f["code"] == "OPTIONS_UNAVAILABLE"));

    // With trusted resolved option lists pinned by snapshot_digest(), valid values pass and out-of-list values fail TYPE_MISMATCH.
    let lists = formspec_eval::ResolvedOptionLists::new()
        .with_option_set("regions", ["north", "south"])
        .with_uri("urn:example:options:categories", ["cat-a", "cat-b"]);
    let digest = lists.snapshot_digest().unwrap();

    let trusted = bare
        .clone()
        .with_trusted_resolved_option_lists(lists.clone(), &digest)
        .unwrap();
    assert_eq!(
        trusted
            .validate_response_definition(
                URL,
                VERSION,
                &json!({"data": {"region": "north", "category": "cat-a"}}),
            )
            .unwrap(),
        None
    );
    let invalid = trusted
        .validate_response_definition(
            URL,
            VERSION,
            &json!({"data": {"region": "west", "category": "cat-z"}}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(invalid.len(), 2);
    assert!(invalid.iter().all(|f| f["code"] == "TYPE_MISMATCH"));

    // A tampered option list against the pinned digest fails closed.
    let tampered = lists.with_option_set("regions", ["north", "south", "west"]);
    let err = bare
        .with_trusted_resolved_option_lists(tampered, &digest)
        .unwrap_err();
    assert!(matches!(err, BindingError::InvalidInput(msg) if msg.contains("OPTIONS_UNAVAILABLE")));
}
