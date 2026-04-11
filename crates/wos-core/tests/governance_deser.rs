// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS Governance Documents.

use std::fs;
use wos_core::GovernanceDocument;

fn load_fixture(name: &str) -> GovernanceDocument {
    let path = format!(
        "{}/fixtures/governance/{name}",
        env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
    );
    let json =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn benefits_adjudication_governance_round_trips() {
    let doc = load_fixture("benefits-adjudication-governance.json");
    assert_eq!(doc.wos_workflow_governance, "1.0");
    assert!(doc.target_workflow.contains("benefits-adjudication"));

    // Due process
    let due = doc.due_process.as_ref().expect("due_process present");
    let adp = due
        .adverse_decision_policy
        .as_ref()
        .expect("adverse policy present");
    assert!(adp.notice_required);
    assert!(adp.counterfactual_required);
    let appeal = adp.appeal_mechanism.as_ref().expect("appeal present");
    assert!(appeal.enabled);

    // Review protocols
    assert!(!doc.review_protocols.is_empty());

    // Pipelines
    assert!(!doc.pipelines.is_empty());
    let pipeline = &doc.pipelines[0];
    assert!(!pipeline.stages.is_empty());

    // Quality controls
    let qc = doc.quality_controls.as_ref().expect("quality controls");
    let sampling = qc.review_sampling.as_ref().expect("sampling");
    assert!(sampling.rate > 0.0);

    // Task catalog
    assert!(!doc.task_catalog.is_empty());
}

#[test]
fn governance_new_phase2_fields() {
    // Verify scope, RuleReference, delegations, holdPolicies
    // deserialize from inline JSON (fixtures don't include these yet).
    let json = r#"{
        "$wosWorkflowGovernance": "1.0",
        "targetWorkflow": "https://example.gov/test",
        "dueProcess": {
            "scope": "impactLevel = 'rights-impacting'",
            "adverseDecisionPolicy": {
                "noticeRequired": true
            }
        },
        "reviewProtocols": [{
            "tags": ["determination"],
            "protocols": ["independentFirst"],
            "scope": "caseFile.amount > 10000"
        }],
        "delegations": [{
            "id": "d1",
            "delegator": "director",
            "delegate": "supervisor",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination"
        }],
        "holdPolicies": [{
            "holdType": "pending-applicant-response",
            "expectedDuration": "P30D",
            "resumeTrigger": "applicantResponse",
            "timeoutAction": "escalate",
            "scope": "caseFile.holdReason = 'docs-needed'"
        }]
    }"#;
    let doc: GovernanceDocument = serde_json::from_str(json).unwrap();

    // Due process scope
    let due = doc.due_process.as_ref().unwrap();
    assert_eq!(
        due.scope.as_deref(),
        Some("impactLevel = 'rights-impacting'")
    );

    // Review protocol scope
    assert_eq!(
        doc.review_protocols[0].scope.as_deref(),
        Some("caseFile.amount > 10000")
    );

    // Delegations
    assert_eq!(doc.delegations.len(), 1);
    assert_eq!(doc.delegations[0].id, "d1");
    assert_eq!(
        doc.delegations[0].authority,
        wos_core::model::governance::DelegationAuthority::Determination
    );

    // Hold policies with scope
    assert_eq!(doc.hold_policies.len(), 1);
    assert_eq!(
        doc.hold_policies[0].scope.as_deref(),
        Some("caseFile.holdReason = 'docs-needed'")
    );
}
