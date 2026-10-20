use super::*;
use serde_json::json;

#[test]
fn governance_alignment_rule_omits_absent_schema_forbidden_nulls() {
    let rule: RuleReference = serde_json::from_value(json!({"ruleId":"rule:test"})).unwrap();
    assert_eq!(
        serde_json::to_value(rule).unwrap(),
        json!({"ruleId":"rule:test"})
    );
}

#[test]
fn governance_alignment_raw_unknown_fields_and_null_refuse() {
    for value in [
        json!({"dueProcess":{"notDeclared":true}}),
        json!({"dueProcess":{"scope":null}}),
        json!({"dueProcess":{"adverseDecisionPolicy":{"notDeclared":true}}}),
        json!({"dueProcess":{"adverseDecisionPolicy":{"noticeTemplateKey":null}}}),
        json!({"dueProcess":{"adverseDecisionPolicy":{"appealMechanism":{"reviewerConstraint":"independentFromOriginal","notDeclared":true}}}}),
    ] {
        assert!(serde_json::from_value::<GovernanceDocument>(value).is_err());
    }
}

#[test]
fn governance_alignment_effective_tag_defaults_preserve_explicit_empty() {
    let audit: AuditConfig =
        serde_json::from_value(json!({"reasoningTier":{},"counterfactualTier":{}})).unwrap();
    assert_eq!(
        audit.reasoning_tier.unwrap().required_for_tags,
        ["determination"]
    );
    assert_eq!(
        audit.counterfactual_tier.unwrap().required_for_tags,
        ["adverse-decision"]
    );
    let explicit: AuditConfig =
        serde_json::from_value(json!({"reasoningTier":{"requiredForTags":[]}})).unwrap();
    assert!(
        explicit
            .reasoning_tier
            .unwrap()
            .required_for_tags
            .is_empty()
    );
}

#[test]
fn governance_alignment_shared_rule_type_identity_and_present_values_survive() {
    let raw = json!({"ruleId":"rule:test","description":"Applied rule","sourceAuthority":"statute","citation":"Section 2"});
    let core: RuleReference = serde_json::from_value(raw.clone()).unwrap();
    let events: wos_events::governance::RuleReference = core;
    assert_eq!(serde_json::to_value(events).unwrap(), raw);
    // RuleReference historically ignores additional fields during typed decoding.
    // Original schema validation remains a separate boundary; keep this behavior.
    let extra: RuleReference =
        serde_json::from_value(json!({"ruleId":"rule:test","extra":true})).unwrap();
    assert_eq!(
        serde_json::to_value(extra).unwrap(),
        json!({"ruleId":"rule:test"})
    );
}

#[test]
fn governance_alignment_complete_native_selections_and_vendor_values_survive() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../wos-conformance/tests/fixtures/reference-due-process-replay.json"
    )))
    .unwrap();
    let bytes = fixture["request"]["candidateBytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    let workflow: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let reference = workflow["governance"].clone();
    let before = reference.clone();
    let typed: GovernanceDocument = serde_json::from_value(reference.clone()).unwrap();
    assert!(typed.due_process.as_ref().unwrap().execution.is_some());
    assert_eq!(
        serde_json::to_value(typed.due_process.unwrap()).unwrap()["execution"],
        reference["dueProcess"]["execution"]
    );
    assert_eq!(reference, before);
    let mut unsupported = reference.clone();
    unsupported["dueProcess"]["execution"]["profile"] = json!("invented-profile");
    assert!(serde_json::from_value::<GovernanceDocument>(unsupported).is_err());
    // Exact generic D1 block extracted from immutable Workflow0.4.1 SHA53c33d2f…;
    // this validates shape only, without installing that runtime capability.
    let mut generic: serde_json::Value =
        serde_json::from_str(include_str!("d1-governance.json")).unwrap();
    generic["dueProcess"]["x-policy"] = json!({"retained":[1,true]});
    generic["dueProcess"]["adverseDecisionPolicy"]["x-notice"] = json!("retained");
    generic["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"]["x-review"] =
        json!({"retained":true});
    let raw = generic.clone();
    let typed: GovernanceDocument = serde_json::from_value(generic.clone()).unwrap();
    let output = serde_json::to_value(typed).unwrap();
    assert_eq!(
        output["dueProcess"]["x-policy"],
        generic["dueProcess"]["x-policy"]
    );
    assert_eq!(
        output["dueProcess"]["adverseDecisionPolicy"]["x-notice"],
        "retained"
    );
    assert_eq!(
        output["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"]["x-review"],
        json!({"retained":true})
    );
    assert_eq!(generic, raw);
    for key in ["continuationPolicyRef", "continuationScope"] {
        assert_eq!(
            output["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"][key],
            generic["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"][key]
        );
    }
}

#[test]
fn governance_alignment_audit_fragment_schema_and_extension_fidelity() {
    let owner: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../schemas/wos-workflow.schema.json"
    ))
    .unwrap();
    let validator=jsonschema::options().with_retriever(OfflineGovernanceSchemas).build(&json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$ref":"#/$defs/AuditConfig","$defs":owner["$defs"]})).unwrap();
    let raw = json!({"x-audit":{"retained":true},"reasoningTier":{"x-reason":17},"counterfactualTier":{"x-counter":[true]}});
    assert!(validator.is_valid(&raw));
    let typed: AuditConfig = serde_json::from_value(raw.clone()).unwrap();
    let output = serde_json::to_value(typed).unwrap();
    assert_eq!(output["x-audit"], raw["x-audit"]);
    assert_eq!(output["reasoningTier"]["x-reason"], 17);
    assert_eq!(output["counterfactualTier"]["x-counter"], json!([true]));
    assert!(!validator.is_valid(&json!({"reasoningTier":{"notDeclared":true}})));
    assert!(!validator.is_valid(&json!({"counterfactualTier":null})));
}
