// Rust guideline compliant 2026-02-21

//! Processor-level conformance claim verification tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wos_conformance::{
    AssistGovernanceProxyEvidence, ClaimStatus, ProcessorClaims, ProcessorEvidence,
    ProcessorManifest, observe_assist_governance_proxy, observe_delegated_formspec_evaluation,
    verify_processor_manifest,
};
use wos_core::model::ai::AIIntegrationDocument;
use wos_core::model::kernel::ImpactLevel;
use wos_core::traits::{ContractValidator, ValidationResult};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn status_for(
    report: &wos_conformance::ProcessorConformanceReport,
    rule_id: &str,
) -> wos_conformance::ClaimStatus {
    report
        .claim(rule_id)
        .unwrap_or_else(|| panic!("missing claim result for {rule_id}"))
        .status
        .clone()
}

#[derive(Debug)]
struct RecordingValidator {
    calls: std::sync::Mutex<Vec<(String, serde_json::Value)>>,
    result: ValidationResult,
}

impl RecordingValidator {
    fn new(result: ValidationResult) -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
            result,
        }
    }

    fn calls(&self) -> Vec<(String, serde_json::Value)> {
        self.calls.lock().expect("validator calls lock").clone()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("recording validator error")]
struct RecordingValidatorError;

impl ContractValidator for RecordingValidator {
    type Error = RecordingValidatorError;

    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<ValidationResult, Self::Error> {
        self.calls
            .lock()
            .expect("validator calls lock")
            .push((contract_ref.to_string(), data.clone()));
        Ok(self.result.clone())
    }
}

fn minimal_proxy_ai_document() -> AIIntegrationDocument {
    serde_json::from_value(serde_json::json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.test/workflows/intake",
        "agents": [
            {
                "id": "eligibilityAgent",
                "type": "agent",
                "agentType": "generative",
                "modelIdentifier": "test-model",
                "modelVersion": "1.0",
                "deonticConstraints": {
                    "permissions": [
                        {
                            "id": "perm-income-range",
                            "bounds": "output.income >= 0 and output.income <= 500000",
                            "onViolation": "reject"
                        }
                    ]
                }
            }
        ]
    }))
    .expect("parse inline AI document")
}

#[test]
fn profile_backed_claims_verify_against_current_fixture_inventory() {
    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            governance_basic: true,
            governance_complete: true,
            agent_registration: true,
            confidence_framework: true,
            ..ProcessorClaims::default()
        },
        ..ProcessorManifest::default()
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");

    assert_eq!(status_for(&report, "G-051"), ClaimStatus::Verified);
    assert_eq!(status_for(&report, "G-052"), ClaimStatus::Verified);
    assert_eq!(status_for(&report, "AI-001"), ClaimStatus::Verified);
    assert_eq!(status_for(&report, "AI-002"), ClaimStatus::Verified);
}

#[test]
fn ai004_claim_without_evidence_fails() {
    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            delegates_formspec_evaluation: true,
            ..ProcessorClaims::default()
        },
        ..ProcessorManifest::default()
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-004"), ClaimStatus::Failed);
}

#[test]
fn ai004_claim_with_delegation_evidence_verifies() {
    let validator = RecordingValidator::new(ValidationResult {
        valid: true,
        errors: Vec::new(),
    });
    let response_envelope = serde_json::json!({
        "taskRef": "intakeReview",
        "submittedAt": "2026-04-12T12:00:00Z",
        "response": {
            "income": 42000,
            "determination": "eligible"
        }
    });
    let evidence = observe_delegated_formspec_evaluation(
        &validator,
        "urn:formspec:test:review:1.0",
        &response_envelope,
        "formspec-core-s1.4",
    )
    .expect("observe delegated validation");

    assert_eq!(
        validator.calls(),
        vec![(
            "urn:formspec:test:review:1.0".to_string(),
            response_envelope.clone()
        )]
    );

    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            delegates_formspec_evaluation: true,
            ..ProcessorClaims::default()
        },
        evidence: ProcessorEvidence {
            delegated_formspec_evaluation: Some(evidence),
            ..ProcessorEvidence::default()
        },
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-004"), ClaimStatus::Verified);
}

#[test]
fn ai050_claim_without_proxy_evidence_fails() {
    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            assist_governance_proxy_conformant: true,
            ..ProcessorClaims::default()
        },
        ..ProcessorManifest::default()
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-050"), ClaimStatus::Failed);
}

#[test]
fn ai050_claim_with_proxy_evidence_verifies() {
    let ai_doc = minimal_proxy_ai_document();
    let mut case_state = HashMap::new();
    case_state.insert("income".to_string(), serde_json::json!(50000));
    let event_data = serde_json::json!({
        "output": {
            "income": -500,
            "determination": "eligible"
        }
    });
    let evidence = observe_assist_governance_proxy(
        &ai_doc,
        "eligibilityAgent",
        "determined",
        &event_data,
        &case_state,
        ImpactLevel::Operational,
    );

    assert!(evidence.differential_check_passed);
    assert!(evidence.strictness_preserved);
    assert!(evidence.provenance_preserved);

    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            assist_governance_proxy_conformant: true,
            ..ProcessorClaims::default()
        },
        evidence: ProcessorEvidence {
            assist_governance_proxy: Some(evidence),
            ..ProcessorEvidence::default()
        },
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-050"), ClaimStatus::Verified);
}

#[test]
fn ai050_claim_with_false_differential_check_fails() {
    let evidence = AssistGovernanceProxyEvidence {
        differential_check_passed: false,
        strictness_preserved: true,
        provenance_preserved: true,
    };

    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            assist_governance_proxy_conformant: true,
            ..ProcessorClaims::default()
        },
        evidence: ProcessorEvidence {
            assist_governance_proxy: Some(evidence),
            ..ProcessorEvidence::default()
        },
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-050"), ClaimStatus::Failed);
}

#[test]
fn ai004_claim_with_invalid_validation_result_fails() {
    let validator = RecordingValidator::new(ValidationResult {
        valid: false,
        errors: vec!["income: required field missing".to_string()],
    });
    let response_envelope = serde_json::json!({
        "taskRef": "intakeReview",
        "submittedAt": "2026-04-12T12:00:00Z",
        "response": {}
    });
    let evidence = observe_delegated_formspec_evaluation(
        &validator,
        "urn:formspec:test:review:1.0",
        &response_envelope,
        "formspec-core-s1.4",
    )
    .expect("observe delegated validation");

    assert!(!evidence.full_response_envelope_validated);

    let manifest = ProcessorManifest {
        processor_name: "reference-processor".to_string(),
        claims: ProcessorClaims {
            delegates_formspec_evaluation: true,
            ..ProcessorClaims::default()
        },
        evidence: ProcessorEvidence {
            delegated_formspec_evaluation: Some(evidence),
            ..ProcessorEvidence::default()
        },
    };

    let report = verify_processor_manifest(&manifest, &fixtures_dir()).expect("verify manifest");
    assert_eq!(status_for(&report, "AI-004"), ClaimStatus::Failed);
}
