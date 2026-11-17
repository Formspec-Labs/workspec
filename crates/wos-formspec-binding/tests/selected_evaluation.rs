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
        url: URL.into(),
        version: VERSION.into(),
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
            .validate_definition(URL, VERSION, &json!({"g":{"a":"filed", "rows":[{"x":3}]}}))
            .unwrap(),
        None
    );
}

#[test]
fn repeat_response_passes_selected_recheck() {
    let definition = definition(json!([repeat()]), json!([]));
    assert_eq!(
        processor(&definition)
            .validate_definition(URL, VERSION, &json!({"rows":[{"x":3},{"x":5}]}))
            .unwrap(),
        None
    );
}

#[test]
fn nested_calculated_field_passes_when_filed_value_matches() {
    let definition = calculated_definition();
    assert_eq!(
        processor(&definition)
            .validate_definition(URL, VERSION, &json!({"g":{"a":3,"double":6}}))
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
        assert!(matches!(processor.validate_definition(URL, VERSION, &data),
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
            processor.validate_definition(URL, VERSION, &data).unwrap(),
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
            .validate_definition(URL, VERSION, &json!({}))
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
        matches!(processor(&definition).validate_definition(URL, VERSION, &json!({"hidden":"filed"})),
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
            .validate_definition(URL, VERSION, &json!({"outer":[{}, {"g":{"rows":[]}}]}))
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
        matches!(processor(&definition).validate_definition(URL, VERSION, &json!({})),
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
        matches!(processor(&definition).validate_definition(URL, VERSION, &json!({})),
        Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation")
    );
}

#[test]
fn omitted_min_repeat_is_refused() {
    let mut rows = repeat();
    rows["minRepeat"] = json!(1);
    let definition = definition(json!([rows]), json!([]));
    let evaluated = formspec_eval::evaluate(
        &definition.clone().into(),
        &Default::default(),
        &formspec_eval::EvalOptions::default().trigger(formspec_eval::EvalTrigger::Submit),
    );
    assert_eq!(evaluated.data, json!({"rows":[{}]}));
    // Once fs-i731 stops padding, expect a MIN_REPEAT finding instead of a data mismatch refusal.
    assert!(
        matches!(processor(&definition).validate_definition(URL, VERSION, &json!({})),
        Err(BindingError::Unsupported(message)) if message == "filed Response data differs from its evaluation")
    );
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
        .validate_definition(URL, VERSION, &json!({"g":{"a":null}}))
        .unwrap()
        .unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error["path"] == "g.a" && error["code"] == "REQUIRED"),
        "{errors:?}"
    );
}
