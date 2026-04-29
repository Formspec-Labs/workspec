// Tier 2 lint rule tests — cross-document resolution checks.
//
// These tests construct a `WosProject` with multiple documents and call
// `wos_lint::lint_project()` indirectly by running tier1 + tier2 checks
// against the manually assembled project.
//
// Since `lint_project()` requires a filesystem directory, we instead
// assemble documents using `WosProject::push()` and call the check
// functions through the public `lint_document()` for T1, plus we build
// a temporary directory to test `lint_project()` end-to-end.

use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use wos_lint::LintSeverity;

// ── Helpers ────────────────────────────────────────────────────

fn has_rule(diagnostics: &[wos_lint::LintDiagnostic], rule_id: &str) -> bool {
    diagnostics.iter().any(|d| d.rule_id == rule_id)
}

fn severity_of(diagnostics: &[wos_lint::LintDiagnostic], rule_id: &str) -> Option<LintSeverity> {
    diagnostics
        .iter()
        .find(|d| d.rule_id == rule_id)
        .map(|d| d.severity)
}

fn path_of(diagnostics: &[wos_lint::LintDiagnostic], rule_id: &str) -> Option<String> {
    diagnostics
        .iter()
        .find(|d| d.rule_id == rule_id)
        .map(|d| d.path.clone())
}

/// Write multiple WOS documents to a temporary directory and run `lint_project`.
fn lint_project_with_docs(docs: Vec<(&str, serde_json::Value)>) -> Vec<wos_lint::LintDiagnostic> {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    for (filename, doc) in &docs {
        let path = dir.path().join(filename);
        let mut file = std::fs::File::create(&path).expect("failed to create file");
        let json_str = serde_json::to_string_pretty(doc).expect("serialization failed");
        file.write_all(json_str.as_bytes())
            .expect("failed to write");
    }
    wos_lint::lint_project(dir.path()).expect("lint_project returned Err")
}

/// Minimal valid kernel document for cross-doc tests.
fn base_kernel() -> serde_json::Value {
    json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/test",
        "impactLevel": "operational",
        "actors": [
            { "id": "alice", "type": "human" },
            { "id": "bob", "type": "human" },
            { "id": "system", "type": "system" }
        ],
        "caseFile": {
            "fields": {
                "amount": { "type": "number" },
                "status": { "type": "string" },
                "filingDate": { "type": "date" }
            }
        },
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": {
                    "type": "atomic",
                    "tags": ["intake"],
                    "transitions": [
                        {
                            "event": "submit",
                            "target": "review",
                            "tags": ["determination"]
                        }
                    ]
                },
                "review": {
                    "type": "atomic",
                    "tags": ["review"],
                    "transitions": [
                        {
                            "event": "approve",
                            "target": "completed",
                            "tags": ["determination"]
                        },
                        {
                            "event": "deny",
                            "target": "denied",
                            "tags": ["adverse-decision", "determination"]
                        }
                    ]
                },
                "holdState": {
                    "type": "atomic",
                    "tags": ["hold"],
                    "transitions": [
                        { "event": "resume", "target": "review" }
                    ]
                },
                "completed": { "type": "final" },
                "denied": { "type": "final" }
            }
        }
    })
}

/// Minimal governance document that targets the base kernel.
fn base_governance() -> serde_json::Value {
    json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/governance/test",
        "targetWorkflow": "https://example.com/workflow/test"
    })
}

fn schema_valid_delegation(id: &str, delegator: &str, delegate: &str) -> serde_json::Value {
    json!({
        "id": id,
        "delegator": delegator,
        "delegate": delegate,
        "scope": {
            "impactLevels": ["operational"]
        },
        "authority": "determination"
    })
}

fn schema_valid_hold_policy(resume_trigger: &str) -> serde_json::Value {
    json!({
        "holdType": "pending-applicant-response",
        "expectedDuration": "P30D",
        "resumeTrigger": resume_trigger,
        "timeoutAction": "escalate"
    })
}

// ========================================================================
// G-034: targetWorkflow MUST match kernel url.
// ========================================================================

#[test]
fn g034_target_workflow_mismatch_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["targetWorkflow"] = json!("https://wrong.example.com/workflow");

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-034"), "expected G-034: {diags:?}");
    assert_eq!(severity_of(&diags, "G-034"), Some(LintSeverity::Error));
}

#[test]
fn g034_target_workflow_matches_clean() {
    let kernel = base_kernel();
    let gov = base_governance();

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-034"), "unexpected G-034: {diags:?}");
}

// ========================================================================
// G-011: Review protocol tags MUST exist in kernel.
// ========================================================================

#[test]
fn g011_review_tag_not_in_kernel_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["nonExistentTag"], "protocol": "standard" }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-011"), "expected G-011: {diags:?}");
    assert_eq!(severity_of(&diags, "G-011"), Some(LintSeverity::Warning));
}

#[test]
fn g011_review_tag_exists_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["intake"], "protocol": "standard" }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-011"), "unexpected G-011: {diags:?}");
}

// ========================================================================
// G-046: Delegation actor MUST exist in kernel.
// ========================================================================

#[test]
fn g046_delegation_actor_not_in_kernel_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([schema_valid_delegation("d1", "nonExistentActor", "bob")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-046"), "expected G-046: {diags:?}");
}

#[test]
fn g046_delegation_actors_exist_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([schema_valid_delegation("d1", "alice", "bob")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-046"), "unexpected G-046: {diags:?}");
}

// ========================================================================
// G-029: Hold resumeTrigger MUST be a kernel event.
// ========================================================================

#[test]
fn g029_resume_trigger_not_a_kernel_event_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([schema_valid_hold_policy("nonExistentEvent")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-029"), "expected G-029: {diags:?}");
    assert_eq!(severity_of(&diags, "G-029"), Some(LintSeverity::Warning));
}

#[test]
fn g029_resume_trigger_is_a_kernel_event_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([schema_valid_hold_policy("resume")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-029"), "unexpected G-029: {diags:?}");
}

// ========================================================================
// G-031: resolutionDateRef MUST be a kernel caseFile field.
// ========================================================================

#[test]
fn g031_resolution_date_ref_not_in_kernel_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["slaConfig"] = json!({
        "resolutionDateRef": "caseFile.nonExistentField"
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-031"), "expected G-031: {diags:?}");
}

#[test]
fn g031_resolution_date_ref_exists_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["slaConfig"] = json!({
        "resolutionDateRef": "caseFile.filingDate"
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-031"), "unexpected G-031: {diags:?}");
}

// ========================================================================
// G-001: Due process required for rights/safety-impacting.
// ========================================================================

#[test]
fn g001_rights_impacting_without_due_process_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let gov = base_governance();

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-001"), "expected G-001: {diags:?}");
}

#[test]
fn g001_rights_impacting_with_due_process_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "notice": {
            "determinationField": "status",
            "reasonCodes": ["RC-01"],
            "appealInstructions": "See form XYZ"
        },
        "explanationLevel": "individualized",
        "counterfactuals": {
            "positive": { "description": "If income was higher..." },
            "negative": { "description": "Because income is below..." }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-001"), "unexpected G-001: {diags:?}");
}

// ========================================================================
// G-004: explanationLevel MUST be individualized for rights-impacting.
// ========================================================================

#[test]
fn g004_explanation_not_individualized_for_rights_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "general"
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-004"), "expected G-004: {diags:?}");
}

// ========================================================================
// G-005: Counterfactuals required for rights-impacting.
// ========================================================================

#[test]
fn g005_missing_counterfactuals_for_rights_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "individualized"
        // missing counterfactuals
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-005"), "expected G-005: {diags:?}");
}

#[test]
fn g005_missing_positive_counterfactual_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "individualized",
        "counterfactuals": {
            "negative": { "description": "Because..." }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        has_rule(&diags, "G-005"),
        "expected G-005 for missing positive: {diags:?}"
    );
}

// ========================================================================
// G-009: adverseDecisionPolicy requires kernel adverse-decision transitions.
// ========================================================================

#[test]
fn g009_adverse_policy_without_kernel_tag_flagged() {
    // Make a kernel with no adverse-decision tagged transitions.
    let mut kernel = base_kernel();
    // Remove adverse-decision tags from all transitions.
    if let Some(states) = kernel.pointer_mut("/lifecycle/states") {
        if let Some(review) = states.get_mut("review") {
            if let Some(transitions) = review.get_mut("transitions").and_then(|t| t.as_array_mut())
            {
                for t in transitions.iter_mut() {
                    if let Some(tags) = t.get_mut("tags").and_then(|t| t.as_array_mut()) {
                        tags.retain(|tag| tag.as_str() != Some("adverse-decision"));
                    }
                }
            }
        }
    }

    let mut gov = base_governance();
    gov["adverseDecisionPolicy"] = json!({ "enabled": true });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-009"), "expected G-009: {diags:?}");
}

// ========================================================================
// G-014: Reasoning tier required for determination-tagged transitions.
// ========================================================================

#[test]
fn g014_determination_tag_without_reasoning_tier_flagged() {
    let kernel = base_kernel(); // has determination-tagged transitions
    let gov = base_governance(); // no reasoningTier

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-014"), "expected G-014: {diags:?}");
}

#[test]
fn g014_determination_tag_with_reasoning_tier_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reasoningTier"] = json!({ "enabled": true });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-014"), "unexpected G-014: {diags:?}");
}

// ========================================================================
// G-015: Counterfactual tier required for adverse-decision in rights-impacting.
// ========================================================================

#[test]
fn g015_adverse_decision_rights_without_counterfactual_tier_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "individualized",
        "counterfactuals": {
            "positive": { "description": "..." },
            "negative": { "description": "..." }
        }
    });
    gov["reasoningTier"] = json!({ "enabled": true });
    // no counterfactualTier

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-015"), "expected G-015: {diags:?}");
}

// ========================================================================
// G-022: Actor in both potentialOwner and excludedOwner.
// ========================================================================

#[test]
fn g022_actor_in_both_potential_and_excluded_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "potentialOwner": ["alice", "bob"],
            "excludedOwner": ["bob"]
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-022"), "expected G-022: {diags:?}");
    assert_eq!(severity_of(&diags, "G-022"), Some(LintSeverity::Warning));
}

// ========================================================================
// G-027: Sub-delegation exceeds maxDelegationDepth.
// ========================================================================

#[test]
fn g027_sub_delegation_exceeds_max_depth_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["maxDelegationDepth"] = json!(1);
    gov["delegations"] = json!([
        {
            "id": "d1",
            "delegator": "alice",
            "delegate": "bob",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01",
            "allowsSubDelegation": true
        },
        {
            "id": "d2",
            "delegator": "bob",
            "delegate": "system",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-027"), "expected G-027: {diags:?}");
}

#[test]
fn g027_delegation_within_max_depth_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["maxDelegationDepth"] = json!(1);
    gov["delegations"] = json!([schema_valid_delegation("d1", "alice", "bob")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-027"), "unexpected G-027: {diags:?}");
}

#[test]
fn g027_default_max_depth_applies_when_absent_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    // maxDelegationDepth is absent, so the schema default of 1 applies.
    gov["delegations"] = json!([
        {
            "id": "d1",
            "delegator": "alice",
            "delegate": "bob",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01",
            "allowsSubDelegation": true
        },
        {
            "id": "d2",
            "delegator": "bob",
            "delegate": "system",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        has_rule(&diags, "G-027"),
        "expected G-027 with default maxDelegationDepth: {diags:?}"
    );
}

// ========================================================================
// G-028: hold policies require at least one hold-tagged kernel state.
// ========================================================================

#[test]
fn g028_hold_policy_without_hold_tagged_state_flagged() {
    let mut kernel = base_kernel();
    if let Some(states) = kernel.pointer_mut("/lifecycle/states") {
        if let Some(hold_state) = states.get_mut("holdState") {
            if let Some(tags) = hold_state
                .get_mut("tags")
                .and_then(|value| value.as_array_mut())
            {
                tags.retain(|tag| tag.as_str() != Some("hold"));
            }
        }
    }
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([schema_valid_hold_policy("resume")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-028"), "expected G-028: {diags:?}");
}

#[test]
fn g028_hold_policy_with_hold_tagged_state_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([schema_valid_hold_policy("resume")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-028"), "unexpected G-028: {diags:?}");
}

// ========================================================================
// G-035: targetGovernance MUST reference a valid governance document.
// ========================================================================

#[test]
fn g035_target_governance_invalid_flagged() {
    let kernel = base_kernel();
    let gov = base_governance();
    let dp = json!({
        "$wosWorkflow": "1.0",
        "targetGovernance": "https://wrong.example.com/governance"
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("due-process.json", dp),
    ]);
    assert!(has_rule(&diags, "G-035"), "expected G-035: {diags:?}");
}

#[test]
fn g035_target_governance_valid_clean() {
    let kernel = base_kernel();
    let gov = base_governance();
    let dp = json!({
        "$wosWorkflow": "1.0",
        "targetGovernance": "https://example.com/governance/test"
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("due-process.json", dp),
    ]);
    assert!(!has_rule(&diags, "G-035"), "unexpected G-035: {diags:?}");
}

// ========================================================================
// G-040: Consistency assertion referenceStage must be a governance stage.
// ========================================================================

#[test]
fn g040_reference_stage_not_in_pipeline_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["pipeline"] = json!([
        { "id": "stage-1", "assertions": [] }
    ]);
    let al = json!({
        "$wosWorkflow": "1.0",
        "assertions": [
            {
                "id": "consistency-check",
                "type": "consistency",
                "referenceStage": "nonExistentStage",
                "fields": ["amount"]
            }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("assertions.json", al),
    ]);
    assert!(has_rule(&diags, "G-040"), "expected G-040: {diags:?}");
}

#[test]
fn g040_reference_stage_exists_in_pipeline_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["pipeline"] = json!([
        { "id": "stage-1", "assertions": [] }
    ]);
    let al = json!({
        "$wosWorkflow": "1.0",
        "assertions": [
            {
                "id": "consistency-check",
                "type": "consistency",
                "referenceStage": "stage-1",
                "fields": ["amount"]
            }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("assertions.json", al),
    ]);
    assert!(!has_rule(&diags, "G-040"), "unexpected G-040: {diags:?}");
}

// ========================================================================
// G-041: Pipeline assertion ids must exist in assertion library.
// ========================================================================

#[test]
fn g041_pipeline_assertion_not_in_library_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["pipeline"] = json!([
        { "id": "stage-1", "assertions": ["nonExistentAssertion"] }
    ]);
    let al = json!({
        "$wosWorkflow": "1.0",
        "assertions": [
            { "id": "real-assertion", "type": "arithmetic", "expression": "1 + 1" }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("assertions.json", al),
    ]);
    assert!(has_rule(&diags, "G-041"), "expected G-041: {diags:?}");
}

#[test]
fn g041_pipeline_assertion_exists_in_library_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["pipeline"] = json!([
        { "id": "stage-1", "assertions": ["real-assertion"] }
    ]);
    let al = json!({
        "$wosWorkflow": "1.0",
        "assertions": [
            { "id": "real-assertion", "type": "arithmetic", "expression": "1 + 1" }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("assertions.json", al),
    ]);
    assert!(!has_rule(&diags, "G-041"), "unexpected G-041: {diags:?}");
}

// ========================================================================
// G-053: Sub-delegation requires allowsSubDelegation.
// ========================================================================

#[test]
fn g053_sub_delegation_without_permission_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        schema_valid_delegation("d1", "alice", "bob"),
        schema_valid_delegation("d2", "bob", "system")
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-053"), "expected G-053: {diags:?}");
}

#[test]
fn g053_sub_delegation_with_permission_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        {
            "id": "d1",
            "delegator": "alice",
            "delegate": "bob",
            "scope": { "impactLevels": ["operational"] },
            "authority": "determination",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01",
            "allowsSubDelegation": true
        },
        schema_valid_delegation("d2", "bob", "system")
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-053"), "unexpected G-053: {diags:?}");
}

#[test]
fn g053_no_sub_delegations_skips_check() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    // Only direct delegations — no delegator is also a delegate elsewhere.
    gov["delegations"] = json!([schema_valid_delegation("d1", "alice", "bob")]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        !has_rule(&diags, "G-053"),
        "unexpected G-053 without sub-delegations: {diags:?}"
    );
}

// ========================================================================
// AI-046 (cross-doc): Rights-impacting kernel + AI doc without disclosure.
// ========================================================================

#[test]
fn ai046_cross_doc_rights_kernel_ai_without_disclosure_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {}
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(
        has_rule(&diags, "AI-046"),
        "expected AI-046 cross-doc: {diags:?}"
    );
}

#[test]
fn ai046_cross_doc_rights_kernel_ai_with_disclosure_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agentDisclosure": {
            "discloseThatAgentAssisted": true
        },
        "agents": {}
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(!has_rule(&diags, "AI-046"), "unexpected AI-046: {diags:?}");
}

#[test]
fn ai046_cross_doc_safety_kernel_skips_disclosure_check() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("safety-impacting");
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {}
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-046"),
        "unexpected AI-046 for safety-impacting: {diags:?}"
    );
}

// ========================================================================
// AI-007: cascadingInvocations required for autonomous-to-autonomous.
// ========================================================================

#[test]
fn ai007_cascading_invocations_not_declared_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous",
                "invokes": ["agentB"],
                "deonticConstraints": { "permissions": [] }
            },
            "agentB": {
                "autonomy": "autonomous",
                "deonticConstraints": { "permissions": [] }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-007"), "expected AI-007: {diags:?}");
}

// ========================================================================
// AI-018: Autonomous agents should have deontic constraints.
// ========================================================================

#[test]
fn ai018_autonomous_without_deontic_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous"
                // no deonticConstraints, permissions, prohibitions, or obligations
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-018"), "expected AI-018: {diags:?}");
}

#[test]
fn ai018_autonomous_with_deontic_clean() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous",
                "permissions": [{ "condition": "true" }]
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(!has_rule(&diags, "AI-018"), "unexpected AI-018: {diags:?}");
}

// ========================================================================
// AI-020: Supervisory agents should define reviewWindow.
// ========================================================================

#[test]
fn ai020_supervisory_without_review_window_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "supervisory"
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-020"), "expected AI-020: {diags:?}");
}

// ========================================================================
// VR-003: counterexample required when result is proven-unsafe.
// ========================================================================

#[test]
fn vr003_proven_unsafe_without_counterexample_flagged() {
    let vr = json!({
        "$wosWorkflow": "1.0",
        "results": [
            { "result": "proven-unsafe", "constraintId": "c-1" }
        ]
    });

    let diags = lint_project_with_docs(vec![("verification-report.json", vr)]);
    assert!(has_rule(&diags, "VR-003"), "expected VR-003: {diags:?}");
}

#[test]
fn vr003_proven_unsafe_with_counterexample_clean() {
    let vr = json!({
        "$wosWorkflow": "1.0",
        "results": [
            {
                "result": "proven-unsafe",
                "constraintId": "c-1",
                "counterexample": { "input": { "x": 5 }, "output": "violation" }
            }
        ]
    });

    let diags = lint_project_with_docs(vec![("verification-report.json", vr)]);
    assert!(!has_rule(&diags, "VR-003"), "unexpected VR-003: {diags:?}");
}

// ========================================================================
// G-056: Binding resolutionDateRef must reference kernel case field.
// ========================================================================

#[test]
fn g056_binding_resolution_date_ref_not_in_kernel_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["bindings"] = json!({
        "b1": {
            "resolutionDateRef": "caseFile.nonExistent"
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-056"), "expected G-056: {diags:?}");
}

#[test]
fn g056_binding_resolution_date_ref_exists_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["bindings"] = json!({
        "b1": {
            "resolutionDateRef": "caseFile.filingDate"
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-056"), "unexpected G-056: {diags:?}");
}

// ========================================================================
// G-033: Parameter values array must not be empty (coverage gap).
// ========================================================================

#[test]
fn g033_empty_parameter_values_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["parameters"] = json!({
        "threshold": {
            "type": "number",
            "values": []
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-033"), "expected G-033: {diags:?}");
}

// ========================================================================
// DM-002: Deployment sequence for rights/safety-impacting.
// ========================================================================

#[test]
fn dm002_missing_shadow_phase_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let dm = json!({
        "$wosWorkflow": "1.0",
        "deploymentSequence": ["canary", "production"]
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("drift-monitor.json", dm)]);
    assert!(has_rule(&diags, "DM-002"), "expected DM-002: {diags:?}");
}

#[test]
fn dm002_correct_sequence_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let dm = json!({
        "$wosWorkflow": "1.0",
        "deploymentSequence": ["shadow", "canary", "production"]
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("drift-monitor.json", dm)]);
    assert!(!has_rule(&diags, "DM-002"), "unexpected DM-002: {diags:?}");
}

#[test]
fn dm002_safety_impacting_missing_shadow_phase_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("safety-impacting");
    let dm = json!({
        "$wosWorkflow": "1.0",
        "deploymentSequence": ["canary", "production"]
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("drift-monitor.json", dm)]);
    assert!(
        has_rule(&diags, "DM-002"),
        "expected DM-002 for safety-impacting: {diags:?}"
    );
}

// ========================================================================
// AG-008: Side-effect tools at autonomous need sideEffectPolicy.
// ========================================================================

#[test]
fn ag008_side_effect_tool_without_policy_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let adv = json!({
        "$wosWorkflow": "1.0",
        "tools": {
            "sendEmail": {
                "hasSideEffects": true,
                "autonomy": "autonomous"
                // no sideEffectPolicy
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("advanced-governance.json", adv),
    ]);
    assert!(has_rule(&diags, "AG-008"), "expected AG-008: {diags:?}");
}

// ========================================================================
// AI-042: Training data disclosure.
// ========================================================================

#[test]
fn ai042_missing_training_data_disclosure_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "modelConfig": {}
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-042"), "expected AI-042: {diags:?}");
}

// ========================================================================
// AI-043: Optimization objective disclosure.
// ========================================================================

#[test]
fn ai043_missing_optimization_objective_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "modelConfig": {
                    "trainingDataCharacteristics": "public data"
                }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-043"), "expected AI-043: {diags:?}");
}

// ========================================================================
// AI-056: Autonomy is action-site property.
// ========================================================================

#[test]
fn ai056_agent_level_autonomy_without_action_sites_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous"
                // no actionSites or per-action autonomy
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-056"), "expected AI-056: {diags:?}");
}

// ========================================================================
// AG-017: Shadow mode recommended for rights-impacting.
// ========================================================================

#[test]
fn ag017_rights_impacting_without_shadow_mode_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let adv = json!({
        "$wosWorkflow": "1.0",
        "tools": {}
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("advanced-governance.json", adv),
    ]);
    assert!(has_rule(&diags, "AG-017"), "expected AG-017: {diags:?}");
}

#[test]
fn ag017_safety_impacting_skips_shadow_mode_check() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("safety-impacting");
    let adv = json!({
        "$wosWorkflow": "1.0",
        "tools": {}
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("advanced-governance.json", adv),
    ]);
    assert!(
        !has_rule(&diags, "AG-017"),
        "unexpected AG-017 for safety-impacting: {diags:?}"
    );
}

// ========================================================================
// G-003: Notice MUST include determinationField, reasonCodes, appealInstructions
//        when kernel is rights-impacting.
// ========================================================================

#[test]
fn g003_rights_impacting_notice_missing_fields_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "individualized",
        "notice": {
            // Missing determinationField, reasonCodes, appealInstructions.
            "method": "mail"
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-003"), "expected G-003: {diags:?}");
    assert_eq!(severity_of(&diags, "G-003"), Some(LintSeverity::Warning));
}

#[test]
fn g003_rights_impacting_notice_with_all_fields_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "explanationLevel": "individualized",
        "notice": {
            "determinationField": "caseFile.determination",
            "reasonCodes": ["R-001", "R-002"],
            "appealInstructions": "You have the right to appeal within 30 days."
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-003"), "unexpected G-003: {diags:?}");
}

#[test]
fn g003_operational_impact_skips_check() {
    // Non-rights-impacting kernel should not trigger G-003 even with sparse notice.
    let kernel = base_kernel(); // impactLevel: "operational"
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "notice": {}
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        !has_rule(&diags, "G-003"),
        "unexpected G-003 for operational: {diags:?}"
    );
}

// ========================================================================
// G-008: continuationOfServices requires hold-tagged state in kernel.
// ========================================================================

#[test]
fn g008_continuation_of_services_without_hold_tag_flagged() {
    // Use a kernel with NO hold-tagged state.
    let mut kernel = base_kernel();
    // Remove the hold tag from holdState.
    kernel["lifecycle"]["states"]["holdState"]["tags"] = json!([]);
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "continuationOfServices": true
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-008"), "expected G-008: {diags:?}");
    assert_eq!(severity_of(&diags, "G-008"), Some(LintSeverity::Warning));
}

#[test]
fn g008_continuation_of_services_with_hold_tag_clean() {
    let kernel = base_kernel(); // has holdState with tags: ["hold"]
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "continuationOfServices": true
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-008"), "unexpected G-008: {diags:?}");
}

#[test]
fn g008_no_continuation_skips_check() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "continuationOfServices": false
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        !has_rule(&diags, "G-008"),
        "unexpected G-008 when continuation is false: {diags:?}"
    );
}

// ========================================================================
// G-023: SLA evaluation MUST use business calendar when sidecar is present.
// ========================================================================

#[test]
fn g023_sla_without_business_calendar_type_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "sla": {
                "duration": "P5D",
                "calendarType": "calendar"
            }
        }
    });
    // Business calendar MUST target the same workflow as governance (G-060 scoping).
    let calendar = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("business-calendar.json", calendar),
    ]);
    assert!(has_rule(&diags, "G-023"), "expected G-023: {diags:?}");
    assert_eq!(severity_of(&diags, "G-023"), Some(LintSeverity::Warning));
    assert!(has_rule(&diags, "G-060"), "expected G-060: {diags:?}");
    assert_eq!(severity_of(&diags, "G-060"), Some(LintSeverity::Error));
}

#[test]
fn g023_sla_with_business_calendar_type_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "sla": {
                "duration": "P5D",
                "calendarType": "business"
            }
        }
    });
    let calendar = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("business-calendar.json", calendar),
    ]);
    assert!(!has_rule(&diags, "G-023"), "unexpected G-023: {diags:?}");
    assert!(!has_rule(&diags, "G-060"), "unexpected G-060: {diags:?}");
}

#[test]
fn g023_no_calendar_sidecar_skips_check() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "sla": {
                "duration": "P5D",
                "calendarType": "calendar"
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        !has_rule(&diags, "G-023"),
        "unexpected G-023 without calendar: {diags:?}"
    );
}

#[test]
fn g060_calendar_different_workflow_skips_mandatory_sla_check() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "sla": { "duration": "P5D", "calendarType": "calendar" }
        }
    });
    let calendar = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://other.example.com/other-workflow",
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("business-calendar.json", calendar),
    ]);
    assert!(
        !has_rule(&diags, "G-060"),
        "G-060 must only apply when the calendar targets the same workflow: {diags:?}"
    );
    assert!(
        !has_rule(&diags, "G-023"),
        "G-023 should also be scoped to matching targetWorkflow: {diags:?}"
    );
}

// ========================================================================
// G-063: Template keys MUST resolve to Notification Template sidecar keys.
// ========================================================================

#[test]
fn g063_notice_template_key_without_sidecar_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "noticeRequired": true,
        "noticeTemplateKey": "missingTemplate"
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-063"), "expected G-063: {diags:?}");
    assert_eq!(severity_of(&diags, "G-063"), Some(LintSeverity::Error));
}

#[test]
fn g063_notice_template_key_resolves_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "noticeRequired": true,
        "noticeTemplateKey": "adverseTpl"
    });
    let notifications = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "templates": {
            "adverseTpl": {
                "category": "adverse-decision",
                "sections": [
                    { "id": "determination", "contentType": "structured", "content": "d" },
                    { "id": "reasons", "contentType": "structured", "content": "r" },
                    { "id": "appealRights", "contentType": "appeal-rights", "content": "a" },
                    { "id": "appealInstructions", "contentType": "action-required", "content": "i" }
                ]
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
        ("notifications.json", notifications),
    ]);
    assert!(!has_rule(&diags, "G-063"), "unexpected G-063: {diags:?}");
}

#[test]
fn g063_no_template_refs_skips_check() {
    let kernel = base_kernel();
    let gov = base_governance();

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-063"), "unexpected G-063: {diags:?}");
}

#[test]
fn g063_notice_template_key_runs_without_kernel() {
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "noticeRequired": true,
        "noticeTemplateKey": "adverseTpl"
    });
    let notifications = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "templates": {
            "adverseTpl": {
                "category": "adverse-decision",
                "sections": [
                    { "id": "determination", "contentType": "structured", "content": "d" },
                    { "id": "reasons", "contentType": "structured", "content": "r" },
                    { "id": "appealRights", "contentType": "appeal-rights", "content": "a" },
                    { "id": "appealInstructions", "contentType": "action-required", "content": "i" }
                ]
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("governance.json", gov),
        ("notifications.json", notifications),
    ]);
    assert!(!has_rule(&diags, "G-063"), "unexpected G-063: {diags:?}");
}

#[test]
fn g063_notice_template_key_without_kernel_missing_sidecar_flagged() {
    let mut gov = base_governance();
    gov["dueProcess"] = json!({
        "noticeRequired": true,
        "noticeTemplateKey": "orphanRef"
    });

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(has_rule(&diags, "G-063"), "expected G-063: {diags:?}");
}

// ========================================================================
// G-066: BreachPolicy escalationStepId MUST resolve in the same task pattern.
// ========================================================================

#[test]
fn g066_unknown_escalation_step_id_flagged() {
    let mut gov = base_governance();
    gov["taskCatalog"] = json!([
        {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "missingStep"
            },
            "escalationChain": [
                {
                    "id": "supervisor",
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    ]);

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(has_rule(&diags, "G-066"), "expected G-066: {diags:?}");
    assert_eq!(severity_of(&diags, "G-066"), Some(LintSeverity::Error));
}

#[test]
fn g066_named_escalation_step_id_resolves_clean() {
    let mut gov = base_governance();
    gov["taskCatalog"] = json!([
        {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "supervisor"
            },
            "escalationChain": [
                {
                    "id": "supervisor",
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    ]);

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-066"), "unexpected G-066: {diags:?}");
}

#[test]
fn g066_level_escalation_step_id_resolves_clean() {
    let mut gov = base_governance();
    gov["taskCatalog"] = json!([
        {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "level-1"
            },
            "escalationChain": [
                {
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    ]);

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-066"), "unexpected G-066: {diags:?}");
}

#[test]
fn g066_no_escalation_step_id_skips_check() {
    let mut gov = base_governance();
    gov["taskCatalog"] = json!([
        {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "notify",
                "templateKey": "slaWarning"
            },
            "escalationChain": [
                {
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    ]);

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-066"), "unexpected G-066: {diags:?}");
}

#[test]
fn g066_tasks_object_unknown_step_id_flagged() {
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "ghost"
            },
            "escalationChain": [
                {
                    "id": "supervisor",
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    });

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(has_rule(&diags, "G-066"), "expected G-066: {diags:?}");
}

#[test]
fn g066_tasks_object_named_step_resolves_clean() {
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "supervisor"
            },
            "escalationChain": [
                {
                    "id": "supervisor",
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate"
                }
            ]
        }
    });

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-066"), "unexpected G-066: {diags:?}");
}

#[test]
fn g066_escalation_step_id_without_chain_flagged() {
    let mut gov = base_governance();
    gov["taskCatalog"] = json!([
        {
            "pattern": "reviewTask",
            "verifiable": "yes",
            "breachPolicy": {
                "action": "escalate",
                "escalationStepId": "supervisor"
            }
        }
    ]);

    let diags = lint_project_with_docs(vec![("governance.json", gov)]);
    assert!(has_rule(&diags, "G-066"), "expected G-066: {diags:?}");
}

#[test]
fn g060_sla_violation_runs_without_kernel() {
    let mut gov = base_governance();
    gov["tasks"] = json!({
        "reviewTask": {
            "sla": {
                "duration": "P5D",
                "calendarType": "calendar"
            }
        }
    });
    let calendar = json!({
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });

    let diags = lint_project_with_docs(vec![
        ("governance.json", gov),
        ("business-calendar.json", calendar),
    ]);
    assert!(has_rule(&diags, "G-060"), "expected G-060: {diags:?}");
    assert_eq!(severity_of(&diags, "G-060"), Some(LintSeverity::Error));
    assert!(has_rule(&diags, "G-023"), "expected G-023: {diags:?}");
}

// ========================================================================
// G-024: Determination-tagged transitions require delegation verification.
// ========================================================================

#[test]
fn g024_determination_without_delegation_flagged() {
    let kernel = base_kernel(); // has determination-tagged transitions
    let gov = base_governance(); // no delegationVerification or delegations

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-024"), "expected G-024: {diags:?}");
    assert_eq!(severity_of(&diags, "G-024"), Some(LintSeverity::Warning));
}

#[test]
fn g024_determination_with_delegation_verification_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegationVerification"] = json!({
        "method": "identity-check"
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-024"), "unexpected G-024: {diags:?}");
}

#[test]
fn g024_determination_with_delegations_list_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        { "delegator": "alice", "delegate": "bob", "scope": "all" }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-024"), "unexpected G-024: {diags:?}");
}

// ========================================================================
// G-036: independenceConstraint MUST encode prevention mechanism.
// ========================================================================

#[test]
fn g036_review_protocols_without_independence_constraint_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["review"], "protocol": "standard" }
    ]);
    // No independenceConstraint.

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(has_rule(&diags, "G-036"), "expected G-036: {diags:?}");
    assert_eq!(severity_of(&diags, "G-036"), Some(LintSeverity::Warning));
}

#[test]
fn g036_review_protocols_with_empty_independence_constraint_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["review"], "protocol": "standard" }
    ]);
    gov["independenceConstraint"] = json!("");

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        has_rule(&diags, "G-036"),
        "expected G-036 for empty constraint: {diags:?}"
    );
}

#[test]
fn g036_review_protocols_with_valid_independence_constraint_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["review"], "protocol": "standard" }
    ]);
    gov["independenceConstraint"] = json!("actorId != caseFile.originalDecisionMaker");

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(!has_rule(&diags, "G-036"), "unexpected G-036: {diags:?}");
}

#[test]
fn g036_no_review_protocols_skips_check() {
    let kernel = base_kernel();
    let gov = base_governance();

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("governance.json", gov)]);
    assert!(
        !has_rule(&diags, "G-036"),
        "unexpected G-036 without reviewProtocols: {diags:?}"
    );
}

// ========================================================================
// AI-026: Escalation rules MUST declare escalationExpiry.
// ========================================================================

#[test]
fn ai026_escalation_without_expiry_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised",
                "escalationRules": [
                    {
                        "trigger": "confidence < 0.7",
                        "action": "escalateToHuman"
                        // Missing escalationExpiry.
                    }
                ]
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-026"), "expected AI-026: {diags:?}");
    assert_eq!(severity_of(&diags, "AI-026"), Some(LintSeverity::Warning));
}

#[test]
fn ai026_escalation_with_expiry_clean() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised",
                "escalationRules": [
                    {
                        "trigger": "confidence < 0.7",
                        "action": "escalateToHuman",
                        "escalationExpiry": "PT24H"
                    }
                ]
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(!has_rule(&diags, "AI-026"), "unexpected AI-026: {diags:?}");
}

#[test]
fn ai026_no_escalation_rules_skips_check() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised"
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-026"),
        "unexpected AI-026 without escalation rules: {diags:?}"
    );
}

// ========================================================================
// AI-031: Agent output contract formUrl MUST match kernel formUrl.
// ========================================================================

#[test]
fn ai031_output_contract_mismatch_flagged() {
    let mut kernel = base_kernel();
    kernel["formUrl"] = json!("https://forms.example.com/intake-form");
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised",
                "outputContract": {
                    "formUrl": "https://forms.example.com/DIFFERENT-form"
                }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(has_rule(&diags, "AI-031"), "expected AI-031: {diags:?}");
    assert_eq!(severity_of(&diags, "AI-031"), Some(LintSeverity::Warning));
}

#[test]
fn ai031_output_contract_matches_kernel_clean() {
    let mut kernel = base_kernel();
    kernel["formUrl"] = json!("https://forms.example.com/intake-form");
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised",
                "outputContract": {
                    "formUrl": "https://forms.example.com/intake-form"
                }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(!has_rule(&diags, "AI-031"), "unexpected AI-031: {diags:?}");
}

#[test]
fn ai031_no_kernel_form_url_skips_check() {
    let kernel = base_kernel(); // No formUrl.
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "eligibilityAgent": {
                "autonomy": "supervised",
                "outputContract": {
                    "formUrl": "https://forms.example.com/some-form"
                }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai-integration.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-031"),
        "unexpected AI-031 without kernel formUrl: {diags:?}"
    );
}

// ========================================================================
// K-010: Action assignTo MUST reference a declared kernel actor.
// ========================================================================

#[test]
fn k010_action_assign_to_undeclared_actor_flagged() {
    let mut kernel = base_kernel();
    // Add a createTask action that references a non-existent actor.
    kernel["lifecycle"]["states"]["intake"]["onEntry"] = json!([
        {
            "action": "createTask",
            "taskRef": "intake-form",
            "assignTo": "charlie"
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(has_rule(&diags, "K-010"), "expected K-010: {diags:?}");
    assert_eq!(severity_of(&diags, "K-010"), Some(LintSeverity::Error));
}

#[test]
fn k010_action_assign_to_declared_actor_clean() {
    let mut kernel = base_kernel();
    // assignTo references "alice" which is declared in actors.
    kernel["lifecycle"]["states"]["intake"]["onEntry"] = json!([
        {
            "action": "createTask",
            "taskRef": "intake-form",
            "assignTo": "alice"
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(!has_rule(&diags, "K-010"), "unexpected K-010: {diags:?}");
}

#[test]
fn k010_transition_action_undeclared_actor_flagged() {
    let mut kernel = base_kernel();
    // Add assignTo on a transition action.
    kernel["lifecycle"]["states"]["intake"]["transitions"] = json!([
        {
            "event": "submit",
            "target": "review",
            "actions": [
                {
                    "action": "createTask",
                    "taskRef": "review-task",
                    "assignTo": "nonexistent_actor"
                }
            ]
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(has_rule(&diags, "K-010"), "expected K-010: {diags:?}");
}

#[test]
fn k010_no_assign_to_skips_check() {
    // Actions without assignTo should not trigger K-010.
    let mut kernel = base_kernel();
    kernel["lifecycle"]["states"]["intake"]["onEntry"] = json!([
        { "action": "setData", "path": "caseFile.status", "value": "active" }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(
        !has_rule(&diags, "K-010"),
        "unexpected K-010 for non-createTask: {diags:?}"
    );
}

// ========================================================================
// K-037: Fail-fast parallel regions MUST have an error-tagged final state.
// ========================================================================

#[test]
fn k037_fail_fast_without_error_final_flagged() {
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/fail-fast",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "parallel",
            "states": {
                "parallel": {
                    "type": "parallel",
                    "cancellationPolicy": "fail-fast",
                    "regions": {
                        "regionA": {
                            "initialState": "taskA",
                            "states": {
                                "taskA": {
                                    "type": "atomic",
                                    "transitions": [
                                        { "event": "doneA", "target": "completedA" }
                                    ]
                                },
                                "completedA": { "type": "final" }
                            }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(has_rule(&diags, "K-037"), "expected K-037: {diags:?}");
    assert_eq!(severity_of(&diags, "K-037"), Some(LintSeverity::Error));
}

#[test]
fn k037_fail_fast_with_error_final_clean() {
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/fail-fast-ok",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "parallel",
            "states": {
                "parallel": {
                    "type": "parallel",
                    "cancellationPolicy": "fail-fast",
                    "regions": {
                        "regionA": {
                            "initialState": "taskA",
                            "states": {
                                "taskA": {
                                    "type": "atomic",
                                    "transitions": [
                                        { "event": "doneA", "target": "completedA" },
                                        { "event": "failA", "target": "errorA" }
                                    ]
                                },
                                "completedA": { "type": "final" },
                                "errorA": { "type": "final", "tags": ["error"] }
                            }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(!has_rule(&diags, "K-037"), "unexpected K-037: {diags:?}");
}

#[test]
fn k037_wait_all_without_error_final_skips_check() {
    // wait-all policy should not trigger K-037.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/wait-all",
        "actors": [{ "id": "user", "type": "human" }],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "parallel",
            "states": {
                "parallel": {
                    "type": "parallel",
                    "cancellationPolicy": "wait-all",
                    "regions": {
                        "regionA": {
                            "initialState": "taskA",
                            "states": {
                                "taskA": {
                                    "type": "atomic",
                                    "transitions": [
                                        { "event": "doneA", "target": "completedA" }
                                    ]
                                },
                                "completedA": { "type": "final" }
                            }
                        }
                    },
                    "transitions": [
                        { "event": "$join", "target": "done" }
                    ]
                },
                "done": { "type": "final" }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(
        !has_rule(&diags, "K-037"),
        "unexpected K-037 for wait-all: {diags:?}"
    );
}

// ========================================================================
// K-010: assignTo in compound substate (verifies recursive path accuracy).
// ========================================================================

#[test]
fn k010_compound_substate_undeclared_actor_has_correct_path() {
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/compound",
        "actors": [{ "id": "alice", "type": "human" }],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "outer",
            "states": {
                "outer": {
                    "type": "compound",
                    "initialState": "inner",
                    "states": {
                        "inner": {
                            "type": "atomic",
                            "onEntry": [
                                {
                                    "action": "createTask",
                                    "taskRef": "sub-task",
                                    "assignTo": "ghost"
                                }
                            ],
                            "transitions": [
                                { "event": "done", "target": "innerDone" }
                            ]
                        },
                        "innerDone": { "type": "final" }
                    },
                    "transitions": [
                        { "event": "$join", "target": "finished" }
                    ]
                },
                "finished": { "type": "final" }
            }
        }
    });

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(has_rule(&diags, "K-010"), "expected K-010: {diags:?}");

    // Path must include the compound parent: /lifecycle/states/outer/states/inner/onEntry/0/assignTo
    let path = path_of(&diags, "K-010").expect("K-010 diagnostic missing");
    assert!(
        path.contains("/outer/states/inner/"),
        "expected path through compound parent, got: {path}"
    );
}

// ========================================================================
// K-010: assignTo in onExit action (verifies onExit path is checked).
// ========================================================================

#[test]
fn k010_on_exit_undeclared_actor_flagged() {
    let mut kernel = base_kernel();
    kernel["lifecycle"]["states"]["review"]["onExit"] = json!([
        {
            "action": "createTask",
            "taskRef": "cleanup-task",
            "assignTo": "phantom"
        }
    ]);

    let diags = lint_project_with_docs(vec![("kernel.json", kernel)]);
    assert!(
        has_rule(&diags, "K-010"),
        "expected K-010 for onExit: {diags:?}"
    );

    let path = path_of(&diags, "K-010").expect("K-010 diagnostic missing");
    assert!(
        path.contains("/review/onExit/"),
        "expected path through onExit, got: {path}"
    );
}

// ========================================================================
// AG-012: Quantifiers must quantify over finite domains.
// ========================================================================

/// AG-012: `every` with arity other than two produces a warning.
#[test]
fn ag012_quantifier_in_verifiable_constraint_flagged() {
    let adv = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "verifiableConstraints": [
            { "expression": "every($items)" }
        ]
    });
    let diags =
        lint_project_with_docs(vec![("kernel.json", base_kernel()), ("advanced.json", adv)]);
    assert!(
        has_rule(&diags, "AG-012"),
        "expected AG-012 warning for non-standard every(): {diags:?}"
    );
    assert_eq!(
        severity_of(&diags, "AG-012"),
        Some(LintSeverity::Warning),
        "AG-012 should be a warning (manual review needed)"
    );
}

/// Built-in two-argument `every` does not trigger AG-012.
#[test]
fn ag012_every_two_args_clean() {
    let adv = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "verifiableConstraints": [
            { "expression": "every($items, $ > 0)" }
        ]
    });
    let diags =
        lint_project_with_docs(vec![("kernel.json", base_kernel()), ("advanced.json", adv)]);
    assert!(
        !has_rule(&diags, "AG-012"),
        "unexpected AG-012 for standard every(): {diags:?}"
    );
}

/// AG-012: Expression without quantifiers in verifiable constraint is clean.
#[test]
fn ag012_no_quantifier_clean() {
    let adv = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "verifiableConstraints": [
            { "expression": "$amount > 0 and $amount < 100000" }
        ]
    });
    let diags =
        lint_project_with_docs(vec![("kernel.json", base_kernel()), ("advanced.json", adv)]);
    assert!(!has_rule(&diags, "AG-012"), "unexpected AG-012: {diags:?}");
}

/// AG-012: `some` with arity other than two produces a warning.
#[test]
fn ag012_some_quantifier_flagged() {
    let adv = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "verifiableConstraints": [
            { "expression": "some($items)" }
        ]
    });
    let diags =
        lint_project_with_docs(vec![("kernel.json", base_kernel()), ("advanced.json", adv)]);
    assert!(
        has_rule(&diags, "AG-012"),
        "expected AG-012 warning for non-standard some(): {diags:?}"
    );
}

// ========================================================================
// AI-023: Agent-free completion path must be reachable.
// ========================================================================

/// AI-023: Kernel where only agents are assigned tasks but a human path
/// exists through transitions — no warning.
#[test]
fn ai023_agent_free_path_exists_clean() {
    // Kernel: intake -> review (agent assigned) -> completed (final)
    // But intake has no agent, so there IS a path through non-agent states.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/test",
        "impactLevel": "operational",
        "actors": [
            { "id": "caseworker", "type": "human" },
            { "id": "classifier", "type": "system" }
        ],
        "lifecycle": {
            "initialState": "intake",
            "states": {
                "intake": {
                    "type": "atomic",
                    "transitions": [
                        { "event": "submit", "target": "review" },
                        { "event": "manualComplete", "target": "completed" }
                    ]
                },
                "review": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "createTask", "taskRef": "classify", "assignTo": "classifier" }
                    ],
                    "transitions": [
                        { "event": "classified", "target": "completed" }
                    ]
                },
                "completed": { "type": "final" }
            }
        },
        "caseFile": { "fields": {} }
    });
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": [
            { "id": "classifier", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" }
        ]
    });
    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-023"),
        "unexpected AI-023: agent-free path exists via intake->completed: {diags:?}"
    );
}

/// AI-023: Every non-final state requires an agent — no agent-free path.
#[test]
fn ai023_no_agent_free_path_flagged() {
    // Kernel: agentOnly -> agentReview -> completed
    // Both non-final states assign tasks exclusively to agents.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/test",
        "impactLevel": "operational",
        "actors": [
            { "id": "triageBot", "type": "system" },
            { "id": "reviewBot", "type": "system" }
        ],
        "lifecycle": {
            "initialState": "agentOnly",
            "states": {
                "agentOnly": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "createTask", "taskRef": "triage", "assignTo": "triageBot" }
                    ],
                    "transitions": [
                        { "event": "triaged", "target": "agentReview" }
                    ]
                },
                "agentReview": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "createTask", "taskRef": "review", "assignTo": "reviewBot" }
                    ],
                    "transitions": [
                        { "event": "reviewed", "target": "completed" }
                    ]
                },
                "completed": { "type": "final" }
            }
        },
        "caseFile": { "fields": {} }
    });
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": [
            { "id": "triageBot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" },
            { "id": "reviewBot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" }
        ]
    });
    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai.json", ai)]);
    assert!(
        has_rule(&diags, "AI-023"),
        "expected AI-023: all non-final states are agent-only: {diags:?}"
    );
}

/// AI-023: No AI integration document means no agents — skip check.
#[test]
fn ai023_no_ai_doc_skips() {
    let diags = lint_project_with_docs(vec![("kernel.json", base_kernel())]);
    assert!(
        !has_rule(&diags, "AI-023"),
        "unexpected AI-023 without AI document: {diags:?}"
    );
}

/// AI-023: Compound state with substates — agent-free path goes through a
/// substate that is NOT agent-assigned, so the global check should pass.
#[test]
fn ai023_compound_substate_agent_free_path() {
    // Kernel:
    //   initialState: "processing" (compound)
    //     substates:
    //       "agentStep" (agent-only, transitions to "humanStep")
    //       "humanStep" (human, transitions to "done")
    //     parent transitions: target "done"
    //   "done" (final)
    //
    // The agent-free path: processing (compound, not agent-only) has parent
    // transition to "done". Additionally, "humanStep" substate is not agent-only
    // and transitions to "done". The initial state "processing" is the compound
    // parent — it is not agent-only, so the BFS can traverse through it.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/compound",
        "impactLevel": "operational",
        "actors": [
            { "id": "worker", "type": "human" },
            { "id": "bot", "type": "system" }
        ],
        "lifecycle": {
            "initialState": "processing",
            "states": {
                "processing": {
                    "type": "compound",
                    "transitions": [
                        { "event": "skip", "target": "done" }
                    ],
                    "states": {
                        "agentStep": {
                            "type": "atomic",
                            "onEntry": [
                                { "action": "createTask", "taskRef": "classify", "assignTo": "bot" }
                            ],
                            "transitions": [
                                { "event": "classified", "target": "humanStep" }
                            ]
                        },
                        "humanStep": {
                            "type": "atomic",
                            "onEntry": [
                                { "action": "createTask", "taskRef": "review", "assignTo": "worker" }
                            ],
                            "transitions": [
                                { "event": "reviewed", "target": "done" }
                            ]
                        }
                    }
                },
                "done": { "type": "final" }
            }
        },
        "caseFile": { "fields": {} }
    });
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/compound",
        "agents": [
            { "id": "bot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" }
        ]
    });
    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-023"),
        "unexpected AI-023: compound state has agent-free substate path: {diags:?}"
    );
}

/// AI-023: Parallel regions — one region is agent-only but another region
/// provides an agent-free path through the workflow, so the global check passes.
#[test]
fn ai023_parallel_region_one_agent_only_still_clean() {
    // Kernel:
    //   initialState: "parallel"
    //     regions:
    //       "agentRegion": states: { "agentWork" (agent-only) -> "agentDone" (final) }
    //       "humanRegion": states: { "humanWork" (human) -> "humanDone" (final) }
    //     parent transitions: target "completed"
    //   "completed" (final)
    //
    // The parent "parallel" state is not agent-only. Its parent transitions
    // reach "completed" (final). Also the humanRegion substates are human-only.
    // Global agent-free path: parallel -> completed.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/parallel",
        "impactLevel": "operational",
        "actors": [
            { "id": "analyst", "type": "human" },
            { "id": "aiBot", "type": "system" }
        ],
        "lifecycle": {
            "initialState": "parallel",
            "states": {
                "parallel": {
                    "type": "parallel",
                    "transitions": [
                        { "event": "allDone", "target": "completed" }
                    ],
                    "regions": {
                        "agentRegion": {
                            "states": {
                                "agentWork": {
                                    "type": "atomic",
                                    "onEntry": [
                                        { "action": "createTask", "taskRef": "autoClassify", "assignTo": "aiBot" }
                                    ],
                                    "transitions": [
                                        { "event": "classified", "target": "agentDone" }
                                    ]
                                },
                                "agentDone": { "type": "final" }
                            }
                        },
                        "humanRegion": {
                            "states": {
                                "humanWork": {
                                    "type": "atomic",
                                    "onEntry": [
                                        { "action": "createTask", "taskRef": "manualReview", "assignTo": "analyst" }
                                    ],
                                    "transitions": [
                                        { "event": "reviewed", "target": "humanDone" }
                                    ]
                                },
                                "humanDone": { "type": "final" }
                            }
                        }
                    }
                },
                "completed": { "type": "final" }
            }
        },
        "caseFile": { "fields": {} }
    });
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/parallel",
        "agents": [
            { "id": "aiBot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" }
        ]
    });
    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai.json", ai)]);
    assert!(
        !has_rule(&diags, "AI-023"),
        "unexpected AI-023: parallel state has agent-free parent transition: {diags:?}"
    );
}

/// AI-023: Verify the severity is `error`, not `warning`.
#[test]
fn ai023_severity_is_error() {
    // Same setup as ai023_no_agent_free_path_flagged — all non-final states are agent-only.
    let kernel = json!({
        "$wosWorkflow": "1.0",
        "url": "https://example.com/workflow/test",
        "impactLevel": "operational",
        "actors": [
            { "id": "triageBot", "type": "system" },
            { "id": "reviewBot", "type": "system" }
        ],
        "lifecycle": {
            "initialState": "agentOnly",
            "states": {
                "agentOnly": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "createTask", "taskRef": "triage", "assignTo": "triageBot" }
                    ],
                    "transitions": [
                        { "event": "triaged", "target": "agentReview" }
                    ]
                },
                "agentReview": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "createTask", "taskRef": "review", "assignTo": "reviewBot" }
                    ],
                    "transitions": [
                        { "event": "reviewed", "target": "completed" }
                    ]
                },
                "completed": { "type": "final" }
            }
        },
        "caseFile": { "fields": {} }
    });
    let ai = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": [
            { "id": "triageBot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" },
            { "id": "reviewBot", "type": "agent", "agentType": "generative", "modelIdentifier": "test", "modelVersion": "1" }
        ]
    });
    let diags = lint_project_with_docs(vec![("kernel.json", kernel), ("ai.json", ai)]);
    assert!(has_rule(&diags, "AI-023"), "expected AI-023: {diags:?}");
    assert_eq!(
        severity_of(&diags, "AI-023"),
        Some(LintSeverity::Error),
        "AI-023 should be error severity (MUST violation when no agent-free path exists)"
    );
}

fn wos_spec_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("wos-spec workspace root is two levels above wos-lint crate")
        .to_path_buf()
}

/// K-049 LoadBearing fixture: `k-049-load-bearing-self-loop.json`
#[test]
fn k049_load_bearing_fixture_self_loop_triggers_k049() {
    let root = wos_spec_workspace_root();
    let path = root.join("fixtures/validation/k-049-load-bearing-self-loop.json");
    let json = std::fs::read_to_string(&path).expect("read k-049-load-bearing-self-loop.json");
    let doc: serde_json::Value = serde_json::from_str(&json).expect("parse kernel JSON");
    let diags = lint_project_with_docs(vec![("kernel.json", doc)]);
    assert!(
        has_rule(&diags, "K-049"),
        "expected K-049 from k-049-load-bearing-self-loop.json: {diags:?}"
    );
}

/// K-049 LoadBearing fixture: `k-049-load-bearing-two-node-cycle.json`
#[test]
fn k049_load_bearing_fixture_two_node_cycle_triggers_k049() {
    let root = wos_spec_workspace_root();
    let path = root.join("fixtures/validation/k-049-load-bearing-two-node-cycle.json");
    let json = std::fs::read_to_string(&path).expect("read k-049-load-bearing-two-node-cycle.json");
    let doc: serde_json::Value = serde_json::from_str(&json).expect("parse kernel JSON");
    let diags = lint_project_with_docs(vec![("kernel.json", doc)]);
    assert!(
        has_rule(&diags, "K-049"),
        "expected K-049 from k-049-load-bearing-two-node-cycle.json: {diags:?}"
    );
}
