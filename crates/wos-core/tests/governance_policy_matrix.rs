// Rust guideline compliant 2026-02-21
//! Canonical policy values shared with JSON Schema and actual lint entry points.

use serde_json::Value;
use wos_core::GovernanceDocument;

fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/validation/governance-policy-matrix.json"
    ))
    .unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn original_governance_values_agree_with_the_owning_schema() {
    for case in cases() {
        let parsed = serde_json::from_value::<GovernanceDocument>(case["governance"].clone());
        assert_eq!(
            parsed.is_ok(),
            case["structurallyValid"].as_bool().unwrap(),
            "{}: {parsed:?}",
            case["id"]
        );
    }
}

#[test]
fn canonical_policy_serialization_is_valid_and_retains_nested_extensions() {
    for case in cases()
        .into_iter()
        .filter(|c| c["structurallyValid"] == true)
    {
        let original = &case["governance"];
        let typed: GovernanceDocument = serde_json::from_value(original.clone()).unwrap();
        let serialized = serde_json::to_value(&typed).unwrap();
        let _: GovernanceDocument = serde_json::from_value(serialized.clone()).unwrap();
        for path in [
            "/dueProcess/x-review",
            "/dueProcess/adverseDecisionPolicy/x-review",
            "/dueProcess/adverseDecisionPolicy/appealMechanism/x-review",
            "/audit/x-review",
            "/audit/reasoningTier/x-review",
            "/audit/counterfactualTier/x-review",
        ] {
            if let Some(value) = original.pointer(path) {
                assert_eq!(serialized.pointer(path), Some(value), "{path}");
            }
        }
    }
}

#[test]
fn schema_defaults_apply_only_when_the_authored_tag_list_is_absent() {
    let typed: GovernanceDocument =
        serde_json::from_value(cases()[0]["governance"].clone()).unwrap();
    let audit = typed.audit.unwrap();
    assert_eq!(
        audit.reasoning_tier.unwrap().required_for_tags,
        ["determination"]
    );
    assert_eq!(
        audit.counterfactual_tier.unwrap().required_for_tags,
        ["adverse-decision"]
    );
    let empty = cases()
        .into_iter()
        .find(|c| c["id"] == "explicit-empty-reasoning-tags")
        .unwrap();
    let typed: GovernanceDocument = serde_json::from_value(empty["governance"].clone()).unwrap();
    assert!(
        typed
            .audit
            .unwrap()
            .reasoning_tier
            .unwrap()
            .required_for_tags
            .is_empty()
    );
}
