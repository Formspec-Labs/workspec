// Rust guideline compliant 2026-02-21

//! Regression checks for absent optional fields and authored JSON nulls.

use serde_json::{Value, json};
use stack_common_typeid::testing::fixture_minter;
use wos_core::model::kernel::{
    Action, Actor, ContractReference, FieldDefinition, State, Transition,
};

#[test]
fn action_export_distinguishes_absent_data_from_authored_null() {
    let absent: Action = serde_json::from_value(json!({"action": "log"})).unwrap();
    let absent = serde_json::to_value(absent).unwrap();
    assert!(absent.get("value").is_none());
    assert!(absent.get("data").is_none());
    assert!(absent.get("description").is_none());

    let authored = json!({
        "action": "setData", "path": "caseFile.optionalValue", "value": null,
        "data": {"nested": null, "items": [null]},
        "description": "Clear the optional value",
        "extensions": {"x-audit": null}
    });
    let action: Action = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(action.value, Some(Value::Null));
    let output = serde_json::to_value(action).unwrap();
    for (key, value) in authored.as_object().unwrap() {
        assert_eq!(
            output.get(key),
            Some(value),
            "authored property changed: {key}"
        );
    }

    let explicit_data: Action =
        serde_json::from_value(json!({"action": "emitEvent", "data": null})).unwrap();
    assert_eq!(explicit_data.data, Some(Value::Null));
    assert_eq!(
        serde_json::to_value(explicit_data).unwrap().get("data"),
        Some(&Value::Null)
    );
}

#[test]
fn field_export_preserves_null_default_and_optional_description() {
    let absent: FieldDefinition = serde_json::from_value(json!({"type": "string"})).unwrap();
    let output = serde_json::to_value(absent).unwrap();
    assert!(output.get("default").is_none());
    assert!(output.get("description").is_none());

    let authored = json!({"type": "string", "default": null, "description": "Optional value"});
    let field: FieldDefinition = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(field.default, Some(Value::Null));
    assert_eq!(serde_json::to_value(field).unwrap(), authored);
}

#[test]
fn explicit_null_default_initializes_a_case_field() {
    let document = json!({
        "$wosWorkflow": "1.0",
        "caseFile": {"fields": {
            "cleared": {"type": "string", "default": null},
            "absent": {"type": "string"}
        }},
        "lifecycle": {"initialState": "waiting", "states": {
            "waiting": {"type": "atomic"}
        }}
    });
    let kernel = serde_json::from_value(document).unwrap();
    let evaluator = wos_core::eval::Evaluator::new(kernel, &fixture_minter()).unwrap();
    let state = evaluator.case_state_json();
    assert_eq!(state.get("cleared"), Some(&Value::Null));
    assert!(state.get("absent").is_none());
}

#[test]
fn open_description_metadata_preserves_absent_null_and_values() {
    fn check<T: serde::de::DeserializeOwned + serde::Serialize>(base: Value) {
        let absent: T = serde_json::from_value(base.clone()).unwrap();
        assert!(
            serde_json::to_value(absent)
                .unwrap()
                .get("description")
                .is_none()
        );
        for description in [
            Value::Null,
            json!("Reviewing"),
            json!({"label": "Reviewing", "details": null}),
            json!(42),
        ] {
            let mut input = base.clone();
            input["description"] = description.clone();
            let typed: T = serde_json::from_value(input).unwrap();
            let output = serde_json::to_value(typed).unwrap();
            assert_eq!(output.get("description"), Some(&description));
            let reopened: T = serde_json::from_value(output.clone()).unwrap();
            assert_eq!(serde_json::to_value(reopened).unwrap(), output);
        }
    }
    check::<Actor>(json!({"id": "reviewer", "type": "system"}));
    check::<State>(json!({"type": "atomic"}));
    check::<Transition>(json!({"target": "done"}));
    check::<Action>(json!({"action": "log"}));
    check::<ContractReference>(json!({"binding": "jsonschema", "ref": "urn:test:contract"}));
}

#[test]
fn atomic_export_omits_only_empty_hierarchy_maps() {
    let atomic: State = serde_json::from_value(json!({"type": "atomic"})).unwrap();
    let output = serde_json::to_value(atomic).unwrap();
    assert!(output.get("states").is_none());
    assert!(output.get("regions").is_none());
    assert!(output.get("initialState").is_none());
    for kind in ["compound", "parallel", "foreach", "final"] {
        let input = json!({"type": kind, "states": {}, "regions": {}, "description": null});
        let state: State = serde_json::from_value(input).unwrap();
        let output = serde_json::to_value(state).unwrap();
        assert_eq!(output["states"], json!({}));
        assert_eq!(output["regions"], json!({}));
        assert_eq!(output["description"], Value::Null);
    }
    // Invalid nonempty atomic topology stays visible for owning validation.
    let invalid = json!({
        "type": "atomic", "states": {"child": {"type": "final"}},
        "regions": {"review": {"initialState": "child", "states": {"child": {"type": "final"}}}}
    });
    let invalid: State = serde_json::from_value(invalid).unwrap();
    let output = serde_json::to_value(invalid).unwrap();
    assert_eq!(output["states"]["child"]["type"], "final");
    assert_eq!(
        output["regions"]["review"]["states"]["child"]["type"],
        "final"
    );
}

#[test]
fn state_export_preserves_all_populated_fields() {
    // Serializer parity covers all fields; this mixed-kind shape is not runtime policy.
    let leaf = json!({
        "type": "final", "transitions": [], "tags": [], "onEntry": [], "onExit": [],
        "states": {}, "regions": {}, "extensions": {}
    });
    let authored = json!({
        "type": "foreach", "description": {"label": "Review", "detail": null},
        "transitions": [], "tags": ["review"], "onEntry": [], "onExit": [],
        "initialState": "leaf", "states": {"leaf": leaf.clone()},
        "regions": {"review": {"initialState": "leaf", "states": {"leaf": leaf.clone()}}},
        "cancellationPolicy": "wait-all", "historyState": "shallow", "outcomeCode": "done",
        "collection": "[1]", "itemVariable": "item", "indexVariable": "index",
        "concurrency": 1, "breakCondition": "false", "outputPath": "caseFile.result",
        "mergeStrategy": "collect", "body": leaf, "extensions": {"x-test": {"null": null}}
    });
    let state: State = serde_json::from_value(authored.clone()).unwrap();
    let output = serde_json::to_value(state).unwrap();
    assert_eq!(output, authored);
    let reopened: State = serde_json::from_value(output.clone()).unwrap();
    assert_eq!(serde_json::to_value(reopened).unwrap(), output);
}
