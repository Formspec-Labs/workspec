//! Unit tests for WS-024 (NoopSigner), WS-025 (JsonRenderer),
//! WS-026 (PolicyLayeredValidator), WS-027 (RoleBasedAccessControl).

use wos_core::model::governance::DelegationScope;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::{AccessControl, ContractValidator, ProvenanceSigner, ReportRenderer};
use wos_server::runtime::access::RoleBasedAccessControl;
use wos_server::runtime::renderer::JsonRenderer;
use wos_server::runtime::signer::NoopSigner;
use wos_server::runtime::validator::{PermissiveValidator, PolicyLayeredValidator};

fn stub_record() -> ProvenanceRecord {
    ProvenanceRecord {
        id: "test".into(),
        record_kind: wos_core::provenance::ProvenanceKind::StateTransition,
        timestamp: "2026-04-24T00:00:00Z".into(),
        actor_id: None,
        from_state: None,
        to_state: None,
        event: Some("submit".into()),
        data: None,
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        input_digest: None,
        output_digest: None,
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    }
}

#[test]
fn noop_signer_produces_empty_signature() {
    let signer = NoopSigner;
    let sig = signer.sign(&stub_record()).unwrap();
    assert!(sig.is_empty());
}

#[test]
fn noop_signer_verifies_any_signature() {
    let signer = NoopSigner;
    assert!(signer.verify(&stub_record(), b"anything").unwrap());
}

#[test]
fn json_renderer_produces_valid_json_explanation() {
    let renderer = JsonRenderer;
    let input = serde_json::json!({"steps": ["a", "b"]});
    let output = renderer.render_explanation(&input, "test").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["steps"][0], "a");
}

#[test]
fn json_renderer_produces_valid_json_audit() {
    let renderer = JsonRenderer;
    let records = vec![stub_record()];
    let output = renderer.render_audit(&records, "json").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn permissive_validator_accepts_anything() {
    let v = PermissiveValidator;
    let result = v.validate("any", &serde_json::json!({})).unwrap();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[test]
fn layered_validator_rejects_rights_impacting_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "rights-impacting"
            }),
        )
        .unwrap();
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("§15.7")));
}

#[test]
fn layered_validator_accepts_rights_impacting_with_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "rights-impacting",
                "respondentLedgerRef": "ledger-123"
            }),
        )
        .unwrap();
    assert!(result.valid);
}

#[test]
fn layered_validator_accepts_low_impact_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "low"
            }),
        )
        .unwrap();
    assert!(result.valid);
}

#[test]
fn layered_validator_rejects_safety_impacting_without_ledger() {
    let v = PolicyLayeredValidator::new(PermissiveValidator);
    let result = v
        .validate(
            "contract",
            &serde_json::json!({
                "impactLevel": "safety-impacting"
            }),
        )
        .unwrap();
    assert!(!result.valid);
}

#[test]
fn role_based_allows_non_review_transition() {
    let ac = RoleBasedAccessControl::new();
    assert!(ac.can_transition("actor-1", "submit"));
    assert!(ac.can_transition("actor-1", "approve"));
}

#[test]
fn role_based_rejects_self_review() {
    let ac = RoleBasedAccessControl::new();
    assert!(!ac.can_transition("actor-1", "review:actor-1"));
}

#[test]
fn role_based_allows_review_of_different_author() {
    let ac = RoleBasedAccessControl::new();
    assert!(ac.can_transition("reviewer", "review:author-1"));
}

#[test]
fn role_based_delegation_permits_review() {
    let ac = RoleBasedAccessControl::new();
    ac.record_delegation("author-1", "delegate-1");
    assert!(ac.can_transition("delegate-1", "review:author-1"));
}

#[test]
fn role_based_delegation_rejects_self_delegation() {
    let ac = RoleBasedAccessControl::new();
    let scope = DelegationScope {
        impact_levels: Vec::new(),
        case_types: Vec::new(),
        max_dollar_threshold: None,
        conditions: None,
    };
    assert!(!ac.can_delegate("a", "a", &scope));
    assert!(ac.can_delegate("a", "b", &scope));
}
