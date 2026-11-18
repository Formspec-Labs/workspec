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
