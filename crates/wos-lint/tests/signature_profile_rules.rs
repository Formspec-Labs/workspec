// Rust guideline compliant 2026-02-21

//! Signature Profile Tier 2 lint coverage.
//!
//! Post-ADR 0076: signature is an embedded block on the single `$wosWorkflow`
//! document. The dispatcher in `tier2.rs` triggers SIG-* rules whenever a
//! workflow carries a `signature` object; SIG-* rules check
//! `$wosWorkflow.signature.{roles, documents, signingFlow, evidence,
//! authenticationPolicies, ...}`. Fixtures here construct one workflow with
//! the embedded block — no standalone signature-profile document.

use std::io::Write;

fn has_rule(diagnostics: &[wos_lint::LintDiagnostic], rule_id: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.rule_id == rule_id)
}

fn lint_project_with_docs(docs: Vec<(&str, serde_json::Value)>) -> Vec<wos_lint::LintDiagnostic> {
    let dir = tempfile::tempdir().expect("temp dir");
    for (filename, doc) in &docs {
        let path = dir.path().join(filename);
        let mut file = std::fs::File::create(&path).expect("create file");
        let json = serde_json::to_string_pretty(doc).expect("json");
        file.write_all(json.as_bytes()).expect("write file");
    }
    wos_lint::lint_project(dir.path()).expect("lint project")
}

/// One workflow document carrying the signature block embedded under
/// `signature`. Returns the `serde_json::Value` so individual tests can
/// mutate the embedded block to provoke specific SIG-* failures.
fn workflow_with_signature() -> serde_json::Value {
    serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "urn:test:signature",
        "version": "1.0.0",
        "actors": [
            { "id": "applicant", "type": "human" },
            { "id": "caseworker", "type": "human" },
            { "id": "system", "type": "system" }
        ],
        "caseFile": {
            "fields": {
                "signature": { "type": "object" },
                "identity": { "type": "object" }
            }
        },
        "lifecycle": {
            "initialState": "draft",
            "states": {
                "draft": {
                    "type": "atomic",
                    "transitions": [{ "event": "start", "target": "awaiting" }]
                },
                "awaiting": {
                    "type": "atomic",
                    "tags": ["awaiting-signature"],
                    "transitions": [
                        { "event": { "kind": "timer", "timerId": "signatureExpiry", "source": "task", "firesAs": "signature.expired" }, "target": "expired" },
                        { "event": "signature.reminder", "target": "awaiting" },
                        { "event": "signature.completed", "target": "complete" }
                    ]
                },
                "complete": { "type": "final", "tags": ["signature-complete"] },
                "expired": { "type": "final", "tags": ["signature-expired"] }
            }
        },
        "signature": {
            "roles": [
                {
                    "id": "applicantSigner",
                    "role": "signer",
                    "actorId": "applicant",
                    "authenticationPolicyKey": "emailOtp"
                }
            ],
            "documents": [
                {
                    "id": "application",
                    "documentRef": "urn:test:document:application",
                    "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    "documentHashAlgorithm": "sha-256",
                    "formspecResponseRef": "urn:test:response:1"
                }
            ],
            "authenticationPolicies": [
                {
                    "key": "emailOtp",
                    "method": "email-otp",
                    "assuranceLevel": "standard"
                }
            ],
            "signingFlow": {
                "type": "sequential",
                "steps": [
                    {
                        "id": "applicantSigns",
                        "roleId": "applicantSigner",
                        "documentId": "application"
                    }
                ]
            },
            "evidence": {
                "recordKind": "signatureAffirmation",
                "requiredFields": ["response.signature.acceptedAt"],
                "consentReference": {
                    "consentTextRef": "urn:test:consent",
                    "consentVersion": "1.0.0",
                    "acceptedAtPath": "response.signature.acceptedAt",
                    "affirmationPath": "response.signature.affirmed"
                },
                "identityBinding": {
                    "method": "email-otp",
                    "assuranceLevel": "standard"
                },
                "custodyHookEligible": true
            },
            "reminders": {
                "eventName": "signature.reminder",
                "schedule": ["P1D"]
            },
            "expiryPolicy": {
                "eventName": "signature.expired",
                "after": "P7D"
            }
        }
    })
}

#[test]
fn signature_profile_valid_project_is_clean_for_sig_errors() {
    let diagnostics = lint_project_with_docs(vec![("workflow.json", workflow_with_signature())]);
    let sig_errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id.starts_with("SIG-"))
        .filter(|diagnostic| diagnostic.severity == wos_lint::LintSeverity::Error)
        .collect();
    assert!(
        sig_errors.is_empty(),
        "unexpected SIG errors: {diagnostics:?}"
    );
}

#[test]
fn signature_profile_target_workflow_mismatch_is_flagged() {
    // SIG-001 fires when the synthesized profile's targetWorkflow.url does not
    // match the workflow's url. The dispatcher constructs the synthetic profile
    // with `targetWorkflow.url = workflow_url` whenever the workflow has its
    // own url, so the only way to provoke a mismatch from the embedded shape
    // is to override targetWorkflow inside the signature block — the
    // dispatcher's per-key copy lets a hand-set `targetWorkflow` survive.
    let mut workflow = workflow_with_signature();
    workflow["signature"]["targetWorkflow"] =
        serde_json::json!({ "url": "urn:test:wrong" });
    let diagnostics = lint_project_with_docs(vec![("workflow.json", workflow)]);
    assert!(
        has_rule(&diagnostics, "SIG-001"),
        "expected SIG-001: {diagnostics:?}"
    );
}

#[test]
fn signature_profile_system_actor_is_flagged() {
    let mut workflow = workflow_with_signature();
    workflow["signature"]["roles"][0]["actorId"] = serde_json::json!("system");
    let diagnostics = lint_project_with_docs(vec![("workflow.json", workflow)]);
    assert!(
        has_rule(&diagnostics, "SIG-003"),
        "expected SIG-003: {diagnostics:?}"
    );
}

#[test]
fn signature_profile_bad_step_references_are_flagged() {
    let mut workflow = workflow_with_signature();
    workflow["signature"]["signingFlow"]["steps"][0]["roleId"] =
        serde_json::json!("missingRole");
    workflow["signature"]["signingFlow"]["steps"][0]["documentId"] =
        serde_json::json!("missingDocument");
    workflow["signature"]["signingFlow"]["steps"][0]["dependsOn"] =
        serde_json::json!(["missingStep"]);
    let diagnostics = lint_project_with_docs(vec![("workflow.json", workflow)]);
    assert!(
        has_rule(&diagnostics, "SIG-005"),
        "expected SIG-005: {diagnostics:?}"
    );
    assert!(
        has_rule(&diagnostics, "SIG-006"),
        "expected SIG-006: {diagnostics:?}"
    );
    assert!(
        has_rule(&diagnostics, "SIG-007"),
        "expected SIG-007: {diagnostics:?}"
    );
}

#[test]
fn signature_profile_invalid_guard_is_flagged() {
    let mut workflow = workflow_with_signature();
    workflow["signature"]["signingFlow"]["type"] = serde_json::json!("routed");
    workflow["signature"]["signingFlow"]["steps"][0]["guard"] =
        serde_json::json!("caseFile.identity. == true");
    let diagnostics = lint_project_with_docs(vec![("workflow.json", workflow)]);
    assert!(
        has_rule(&diagnostics, "SIG-008"),
        "expected SIG-008: {diagnostics:?}"
    );
}
