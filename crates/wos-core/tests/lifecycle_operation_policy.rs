//! Kernel S11.3: the workflow, not the caller, decides whether `suspend` and
//! `terminate` require a reason (`LifecycleOperationPolicy`).

use serde_json::{Value, json};
use wos_core::KernelDocument;
use wos_core::model::kernel::LifecycleOperationPolicy;

fn workflow(lifecycle_extra: Value) -> Value {
    let mut lifecycle = json!({
        "initialState": "open",
        "states": {
            "open": {"type": "atomic", "transitions": [{"target": "closed", "actor": "clerk"}]},
            "closed": {"type": "final", "outcomeCode": "closed"}
        }
    });
    lifecycle
        .as_object_mut()
        .expect("lifecycle is an object")
        .extend(
            lifecycle_extra
                .as_object()
                .expect("extra is an object")
                .clone(),
        );
    json!({
        "$wosWorkflow": "1.0",
        "url": "urn:wos:test:lifecycle-operation-policy",
        "version": "1.0.0",
        "title": "Lifecycle operation policy",
        "impactLevel": "operational",
        "actors": [{"id": "clerk", "type": "human"}],
        "lifecycle": lifecycle
    })
}

#[test]
fn lifecycle_carries_each_operation_reason_policy() {
    let document: KernelDocument = serde_json::from_value(workflow(json!({
        "suspend": {"reasonRequired": true},
        "terminate": {}
    })))
    .expect("policies deserialize");

    assert_eq!(
        document.lifecycle.suspend,
        Some(LifecycleOperationPolicy {
            reason_required: true
        })
    );
    assert_eq!(
        document.lifecycle.terminate,
        Some(LifecycleOperationPolicy::default())
    );
}

#[test]
fn absent_policies_leave_the_reason_optional_and_are_not_serialized() {
    let document: KernelDocument =
        serde_json::from_value(workflow(json!({}))).expect("workflow deserializes");

    assert_eq!(document.lifecycle.suspend, None);
    assert_eq!(document.lifecycle.terminate, None);
    let lifecycle = serde_json::to_value(&document.lifecycle).expect("lifecycle serializes");
    assert!(lifecycle.get("suspend").is_none() && lifecycle.get("terminate").is_none());
}

#[test]
fn a_policy_refuses_keys_it_does_not_define() {
    let result = serde_json::from_value::<KernelDocument>(workflow(json!({
        "suspend": {"reasonRequired": true, "waivable": true}
    })));

    assert!(result.is_err(), "an unknown policy key must not be ignored");
}
