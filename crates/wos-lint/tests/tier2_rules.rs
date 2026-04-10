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
use wos_lint::Severity;

// ── Helpers ────────────────────────────────────────────────────

fn has_rule(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> bool {
    diagnostics.iter().any(|d| d.rule_id == rule_id)
}

fn severity_of(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> Option<Severity> {
    diagnostics.iter().find(|d| d.rule_id == rule_id).map(|d| d.severity)
}

/// Write multiple WOS documents to a temporary directory and run `lint_project`.
fn lint_project_with_docs(docs: Vec<(&str, serde_json::Value)>) -> Vec<wos_lint::Diagnostic> {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    for (filename, doc) in &docs {
        let path = dir.path().join(filename);
        let mut file = std::fs::File::create(&path).expect("failed to create file");
        let json_str = serde_json::to_string_pretty(doc).expect("serialization failed");
        file.write_all(json_str.as_bytes()).expect("failed to write");
    }
    wos_lint::lint_project(dir.path()).expect("lint_project returned Err")
}

/// Minimal valid kernel document for cross-doc tests.
fn base_kernel() -> serde_json::Value {
    json!({
        "$wosKernel": "1.0",
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
        "$wosWorkflowGovernance": "1.0",
        "url": "https://example.com/governance/test",
        "targetWorkflow": "https://example.com/workflow/test"
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-034"), "expected G-034: {diags:?}");
    assert_eq!(severity_of(&diags, "G-034"), Some(Severity::Error));
}

#[test]
fn g034_target_workflow_matches_clean() {
    let kernel = base_kernel();
    let gov = base_governance();

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-011"), "expected G-011: {diags:?}");
    assert_eq!(severity_of(&diags, "G-011"), Some(Severity::Warning));
}

#[test]
fn g011_review_tag_exists_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reviewProtocols"] = json!([
        { "tags": ["intake"], "protocol": "standard" }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(!has_rule(&diags, "G-011"), "unexpected G-011: {diags:?}");
}

// ========================================================================
// G-046: Delegation actor MUST exist in kernel.
// ========================================================================

#[test]
fn g046_delegation_actor_not_in_kernel_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        {
            "delegator": "nonExistentActor",
            "delegate": "bob",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-046"), "expected G-046: {diags:?}");
}

#[test]
fn g046_delegation_actors_exist_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        {
            "delegator": "alice",
            "delegate": "bob",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(!has_rule(&diags, "G-046"), "unexpected G-046: {diags:?}");
}

// ========================================================================
// G-029: Hold resumeTrigger MUST be a kernel event.
// ========================================================================

#[test]
fn g029_resume_trigger_not_a_kernel_event_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([
        {
            "stateRef": "holdState",
            "resumeTrigger": "nonExistentEvent",
            "expectedDuration": "P30D"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-029"), "expected G-029: {diags:?}");
    assert_eq!(severity_of(&diags, "G-029"), Some(Severity::Warning));
}

#[test]
fn g029_resume_trigger_is_a_kernel_event_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([
        {
            "stateRef": "holdState",
            "resumeTrigger": "resume",
            "expectedDuration": "P30D"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-031"), "expected G-031: {diags:?}");
}

#[test]
fn g031_resolution_date_ref_exists_in_kernel_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["slaConfig"] = json!({
        "resolutionDateRef": "caseFile.filingDate"
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-005"), "expected G-005 for missing positive: {diags:?}");
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
            if let Some(transitions) = review.get_mut("transitions").and_then(|t| t.as_array_mut()) {
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-009"), "expected G-009: {diags:?}");
}

// ========================================================================
// G-014: Reasoning tier required for determination-tagged transitions.
// ========================================================================

#[test]
fn g014_determination_tag_without_reasoning_tier_flagged() {
    let kernel = base_kernel(); // has determination-tagged transitions
    let gov = base_governance(); // no reasoningTier

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-014"), "expected G-014: {diags:?}");
}

#[test]
fn g014_determination_tag_with_reasoning_tier_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["reasoningTier"] = json!({ "enabled": true });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-022"), "expected G-022: {diags:?}");
    assert_eq!(severity_of(&diags, "G-022"), Some(Severity::Warning));
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
            "delegator": "alice",
            "delegate": "bob",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01",
            "allowsSubDelegation": true
        },
        {
            "delegator": "bob",
            "delegate": "system",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-027"), "expected G-027: {diags:?}");
}

// ========================================================================
// G-028: holdPolicy stateRef must reference a hold-tagged state.
// ========================================================================

#[test]
fn g028_hold_policy_references_non_hold_state_flagged() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([
        {
            "stateRef": "review",  // review is not tagged 'hold'
            "resumeTrigger": "resume",
            "expectedDuration": "P30D"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-028"), "expected G-028: {diags:?}");
}

#[test]
fn g028_hold_policy_references_hold_tagged_state_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["holdPolicies"] = json!([
        {
            "stateRef": "holdState",  // holdState IS tagged 'hold' in base_kernel
            "resumeTrigger": "resume",
            "expectedDuration": "P30D"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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
        "$wosDueProcess": "1.0",
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
        "$wosDueProcess": "1.0",
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
        "$wosAssertionLibrary": "1.0",
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
        "$wosAssertionLibrary": "1.0",
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
        "$wosAssertionLibrary": "1.0",
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
        "$wosAssertionLibrary": "1.0",
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
        {
            "delegator": "alice",
            "delegate": "bob",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
            // allowsSubDelegation is absent (defaults to false)
        },
        {
            "delegator": "bob",  // bob is a delegate, so this is sub-delegation
            "delegate": "system",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(has_rule(&diags, "G-053"), "expected G-053: {diags:?}");
}

#[test]
fn g053_sub_delegation_with_permission_clean() {
    let kernel = base_kernel();
    let mut gov = base_governance();
    gov["delegations"] = json!([
        {
            "delegator": "alice",
            "delegate": "bob",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01",
            "allowsSubDelegation": true
        },
        {
            "delegator": "bob",
            "delegate": "system",
            "effectiveDate": "2026-01-01",
            "expirationDate": "2027-01-01"
        }
    ]);

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
    assert!(!has_rule(&diags, "G-053"), "unexpected G-053: {diags:?}");
}

// ========================================================================
// AI-046 (cross-doc): Rights-impacting kernel + AI doc without disclosure.
// ========================================================================

#[test]
fn ai046_cross_doc_rights_kernel_ai_without_disclosure_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {}
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-046"), "expected AI-046 cross-doc: {diags:?}");
}

#[test]
fn ai046_cross_doc_rights_kernel_ai_with_disclosure_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agentDisclosure": {
            "discloseThatAgentAssisted": true
        },
        "agents": {}
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(!has_rule(&diags, "AI-046"), "unexpected AI-046: {diags:?}");
}

// ========================================================================
// AI-007: cascadingInvocations required for autonomous-to-autonomous.
// ========================================================================

#[test]
fn ai007_cascading_invocations_not_declared_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-007"), "expected AI-007: {diags:?}");
}

// ========================================================================
// AI-018: Autonomous agents should have deontic constraints.
// ========================================================================

#[test]
fn ai018_autonomous_without_deontic_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous"
                // no deonticConstraints, permissions, prohibitions, or obligations
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-018"), "expected AI-018: {diags:?}");
}

#[test]
fn ai018_autonomous_with_deontic_clean() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous",
                "permissions": [{ "condition": "true" }]
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(!has_rule(&diags, "AI-018"), "unexpected AI-018: {diags:?}");
}

// ========================================================================
// AI-020: Supervisory agents should define reviewWindow.
// ========================================================================

#[test]
fn ai020_supervisory_without_review_window_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "supervisory"
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-020"), "expected AI-020: {diags:?}");
}

// ========================================================================
// VR-003: counterexample required when result is proven-unsafe.
// ========================================================================

#[test]
fn vr003_proven_unsafe_without_counterexample_flagged() {
    let vr = json!({
        "$wosVerificationReport": "1.0",
        "results": [
            { "result": "proven-unsafe", "constraintId": "c-1" }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("verification-report.json", vr),
    ]);
    assert!(has_rule(&diags, "VR-003"), "expected VR-003: {diags:?}");
}

#[test]
fn vr003_proven_unsafe_with_counterexample_clean() {
    let vr = json!({
        "$wosVerificationReport": "1.0",
        "results": [
            {
                "result": "proven-unsafe",
                "constraintId": "c-1",
                "counterexample": { "input": { "x": 5 }, "output": "violation" }
            }
        ]
    });

    let diags = lint_project_with_docs(vec![
        ("verification-report.json", vr),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("governance.json", gov),
    ]);
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
        "$wosDriftMonitor": "1.0",
        "deploymentSequence": ["canary", "production"]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("drift-monitor.json", dm),
    ]);
    assert!(has_rule(&diags, "DM-002"), "expected DM-002: {diags:?}");
}

#[test]
fn dm002_correct_sequence_clean() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let dm = json!({
        "$wosDriftMonitor": "1.0",
        "deploymentSequence": ["shadow", "canary", "production"]
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("drift-monitor.json", dm),
    ]);
    assert!(!has_rule(&diags, "DM-002"), "unexpected DM-002: {diags:?}");
}

// ========================================================================
// AG-008: Side-effect tools at autonomous need sideEffectPolicy.
// ========================================================================

#[test]
fn ag008_side_effect_tool_without_policy_flagged() {
    let mut kernel = base_kernel();
    kernel["impactLevel"] = json!("rights-impacting");
    let adv = json!({
        "$wosAdvancedGovernance": "1.0",
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
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "modelConfig": {}
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-042"), "expected AI-042: {diags:?}");
}

// ========================================================================
// AI-043: Optimization objective disclosure.
// ========================================================================

#[test]
fn ai043_missing_optimization_objective_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "modelConfig": {
                    "trainingDataCharacteristics": "public data"
                }
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
    assert!(has_rule(&diags, "AI-043"), "expected AI-043: {diags:?}");
}

// ========================================================================
// AI-056: Autonomy is action-site property.
// ========================================================================

#[test]
fn ai056_agent_level_autonomy_without_action_sites_flagged() {
    let kernel = base_kernel();
    let ai = json!({
        "$wosAIIntegration": "1.0",
        "targetWorkflow": "https://example.com/workflow/test",
        "agents": {
            "agentA": {
                "autonomy": "autonomous"
                // no actionSites or per-action autonomy
            }
        }
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("ai-integration.json", ai),
    ]);
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
        "$wosAdvancedGovernance": "1.0",
        "tools": {}
    });

    let diags = lint_project_with_docs(vec![
        ("kernel.json", kernel),
        ("advanced-governance.json", adv),
    ]);
    assert!(has_rule(&diags, "AG-017"), "expected AG-017: {diags:?}");
}
