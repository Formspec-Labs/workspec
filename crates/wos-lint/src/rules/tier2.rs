// Rust guideline compliant 2026-02-21

//! Tier 2 lint rules — cross-document resolution + FEL AST analysis.
//!
//! These rules require loading multiple documents together (a "project")
//! and/or parsing FEL expressions. See LINT-MATRIX.md for the catalog.
//!
//! # Rule coverage
//!
//! ## Governance — due process
//! | Rule  | Sev     | What is checked                                                      |
//! |-------|---------|----------------------------------------------------------------------|
//! | G-001 | error   | Governance MUST have dueProcess when kernel is rights/safety-impacting |
//! | G-003 | warning | Rights-impacting notice must declare individualized content fields   |
//! | G-004 | error   | explanationLevel MUST be `individualized` for rights-impacting       |
//! | G-005 | error   | Counterfactuals section with positive + negative entries required     |
//! | G-008 | warning | continuationOfServices requires a `hold`-tagged kernel state         |
//! | G-009 | error   | adverseDecisionPolicy requires kernel adverse-decision transitions   |
//! | G-014 | error   | Reasoning tier required for determination-tagged kernel transitions  |
//! | G-015 | error   | Counterfactual tier required for adverse-decision in rights-impacting|
//!
//! ## Governance — task assignment / calendar / delegation
//! | Rule  | Sev     | What is checked                                                      |
//! |-------|---------|----------------------------------------------------------------------|
//! | G-011 | warning | Review protocol tags exist in kernel                                 |
//! | G-022 | warning | Actor in both potentialOwner and excludedOwner                       |
//! | G-023 | warning | SLA should set calendarType=business when calendar sidecar present   |
//! | G-024 | warning | Delegation verification config present when kernel has determination  |
//! | G-027 | error   | Sub-delegation chain depth must not exceed maxDelegationDepth        |
//! | G-028 | error   | holdPolicy stateRef must reference a hold-tagged kernel state        |
//! | G-029 | warning | resumeTrigger must be a kernel event                                 |
//! | G-031 | warning | resolutionDateRef must be a kernel caseFile field (policy params)    |
//! | G-033 | warning | Parameter values array must not be empty (coverage gap)              |
//! | G-034 | error   | targetWorkflow must match kernel url                                 |
//! | G-035 | error   | targetGovernance must reference a valid governance document          |
//! | G-036 | warning | independenceConstraint must encode a prevention mechanism            |
//! | G-040 | error   | consistency assertion referenceStage must be a governance stage id   |
//! | G-041 | error   | Pipeline assertion ids must exist in assertion library               |
//! | G-046 | warning | Delegation delegator/delegate must be kernel actors                  |
//! | G-053 | error   | Sub-delegator must have allowsSubDelegation in their original grant  |
//! | G-056 | warning | Binding resolutionDateRef must be a kernel caseFile field            |
//!
//! ## AI integration
//! | Rule   | Sev     | What is checked                                                     |
//! |--------|---------|---------------------------------------------------------------------|
//! | AI-007 | error   | cascadingInvocations required when autonomous agents invoke others  |
//! | AI-018 | warning | autonomous agents should have deontic constraints                   |
//! | AI-020 | warning | supervisory agents should define reviewWindow                       |
//! | AI-026 | warning | escalationRules should declare escalationExpiry                     |
//! | AI-031 | warning | agent outputContract formUrl should match kernel formUrl            |
//! | AI-042 | warning | agent modelConfig should disclose trainingDataCharacteristics       |
//! | AI-043 | warning | agent modelConfig should disclose optimizationObjective             |
//! | AI-046 | error   | rights/safety-impacting kernel requires discloseThatAgentAssisted   |
//! | AI-056 | warning | autonomy should be declared per action site, not at agent level     |
//!
//! ## Advanced governance / drift / verification
//! | Rule   | Sev     | What is checked                                                     |
//! |--------|---------|---------------------------------------------------------------------|
//! | AG-008 | warning | side-effect tools at autonomous need sideEffectPolicy               |
//! | AG-017 | warning | shadow mode recommended for rights-impacting                        |
//! | DM-002 | warning | rights/safety workflows should follow shadow→canary→production      |
//! | VR-003 | error   | counterexample required when result is proven-unsafe                |

use serde_json::Value;

use crate::diagnostic::Diagnostic;
use crate::document::{DocumentKind, WosProject};

use super::fel_analysis;

// ---------------------------------------------------------------------------
// Pre-computed kernel index
// ---------------------------------------------------------------------------

/// Pre-computed index of kernel data used by multiple Tier 2 rules.
///
/// Building these collections once per `check()` call avoids repeatedly
/// walking the entire state hierarchy for each governance document rule
/// (Finding #1/#3).
struct KernelCollections {
    /// All tags appearing on any kernel state or transition.
    tags: std::collections::HashSet<String>,
    /// All event names referenced by kernel transitions.
    events: std::collections::HashSet<String>,
    /// All case file field names in `caseFile.fields` (as `"caseFile.<name>"`).
    case_fields: std::collections::HashSet<String>,
}

impl KernelCollections {
    fn from_kernel(kernel: &crate::document::WosDocument) -> Self {
        Self {
            tags: collect_kernel_tags(kernel),
            events: collect_kernel_events(kernel),
            case_fields: collect_kernel_case_fields(kernel),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all Tier 2 cross-document checks across the project.
pub fn check(project: &WosProject, diagnostics: &mut Vec<Diagnostic>) {
    let kernel = project.kernel();

    // Pre-compute kernel index once so rule functions don't rebuild it per call.
    let kernel_collections = kernel.map(KernelCollections::from_kernel);

    // Governance documents — all checks that need the kernel.
    for gov in project.of_kind(DocumentKind::WorkflowGovernance) {
        if let (Some(kernel), Some(kc)) = (kernel, kernel_collections.as_ref()) {
            check_target_workflow_match(gov, kernel, diagnostics);
            check_governance_tags_exist(gov, &kc.tags, diagnostics);
            check_delegation_actors_exist(gov, kernel, diagnostics);
            check_hold_resume_triggers(gov, &kc.events, diagnostics);
            check_resolution_date_refs(gov, &kc.case_fields, diagnostics);
            check_due_process_for_impact(gov, kernel, diagnostics);
            check_notice_individualized_for_rights(gov, kernel, diagnostics);
            check_explanation_level_for_rights(gov, kernel, diagnostics);
            check_counterfactual_for_rights(gov, kernel, diagnostics);
            check_continuation_of_services(gov, &kc.tags, diagnostics);
            check_adverse_decision_due_process(gov, &kc.tags, diagnostics);
            check_reasoning_tier_for_determination(gov, &kc.tags, diagnostics);
            check_counterfactual_tier_for_adverse(gov, kernel, &kc.tags, diagnostics);
            check_excluded_owner_override(gov, diagnostics);
            check_sla_business_calendar(gov, project, diagnostics);
            check_delegation_verification_on_determination(gov, &kc.tags, diagnostics);
            check_sub_delegation_depth(gov, diagnostics);
            check_hold_policies_on_hold_states(gov, kernel, diagnostics);
            check_binding_resolution_date_refs(gov, &kc.case_fields, diagnostics);
        }

        // Checks that only need the governance document itself.
        check_parameter_coverage(gov, diagnostics);
        check_independence_constraint(gov, diagnostics);
        check_sub_delegation_permission(gov, diagnostics);
    }

    // Due-process documents.
    for dp in project.of_kind(DocumentKind::DueProcess) {
        check_target_governance_valid(dp, project, diagnostics);
        check_independence_constraint_in_due_process(dp, diagnostics);
    }

    // Assertion library cross-references governance pipeline stages.
    for al in project.of_kind(DocumentKind::AssertionLibrary) {
        check_consistency_reference_stage(al, project, diagnostics);
    }

    // Governance documents reference assertion library ids.
    for gov in project.of_kind(DocumentKind::WorkflowGovernance) {
        check_pipeline_assertion_ids(gov, project, diagnostics);
    }

    // AI integration documents.
    for ai in project.of_kind(DocumentKind::AiIntegration) {
        if let Some(kernel) = kernel {
            check_target_workflow_match(ai, kernel, diagnostics);
            check_ai_disclosure_for_impact(ai, kernel, diagnostics);
            check_agent_output_contract(ai, kernel, diagnostics);
        }
        check_cascading_invocations_declared(ai, diagnostics);
        check_autonomous_actions_have_deontic(ai, diagnostics);
        check_supervisory_actions_review_window(ai, diagnostics);
        check_escalation_expiry_present(ai, diagnostics);
        check_training_data_disclosure(ai, diagnostics);
        check_optimization_objective_disclosure(ai, diagnostics);
        check_autonomy_is_action_site_property(ai, diagnostics);
    }

    // Policy parameters documents.
    for pp in project.of_kind(DocumentKind::PolicyParameters) {
        if let Some(kc) = kernel_collections.as_ref() {
            check_policy_param_date_refs(pp, &kc.case_fields, diagnostics);
        }
    }

    // Advanced governance documents.
    for adv in project.of_kind(DocumentKind::Advanced) {
        if let Some(kernel) = kernel {
            check_side_effect_tools_policy(adv, diagnostics);
            check_shadow_mode_recommended(adv, kernel, diagnostics);
        }
    }

    // Drift monitor documents.
    for dm in project.of_kind(DocumentKind::DriftMonitor) {
        if let Some(kernel) = kernel {
            check_deployment_sequence(dm, kernel, diagnostics);
        }
    }

    // Verification report documents.
    for vr in project.of_kind(DocumentKind::VerificationReport) {
        check_counterexample_on_unsafe(vr, diagnostics);
    }

    // FEL AST analysis (T2-ast rules).
    fel_analysis::check(project, diagnostics);
}

// ---------------------------------------------------------------------------
// G-034: targetWorkflow must match kernel url
// ---------------------------------------------------------------------------

/// G-034: `targetWorkflow` must match the `url` of the target kernel document.
fn check_target_workflow_match(
    doc: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let target = doc.value.get("targetWorkflow").and_then(Value::as_str);
    let kernel_url = kernel.value.get("url").and_then(Value::as_str);

    if let (Some(target), Some(url)) = (target, kernel_url) {
        if target != url {
            diagnostics.push(Diagnostic::error(
                "G-034",
                "/targetWorkflow",
                format!("targetWorkflow '{target}' does not match kernel url '{url}'"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-001: Due process required for rights/safety-impacting
// ---------------------------------------------------------------------------

/// G-001: Governance MUST declare a `dueProcess` section when the kernel's
/// `impactLevel` is `rights-impacting` or `safety-impacting`.
fn check_due_process_for_impact(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_rights_or_safety_impacting(kernel) {
        return;
    }
    if gov.value.get("dueProcess").is_none() {
        diagnostics.push(Diagnostic::error(
            "G-001",
            "/dueProcess",
            "kernel impactLevel is rights/safety-impacting; governance MUST declare a dueProcess section",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-003: Notice content must be individualized for rights-impacting
// ---------------------------------------------------------------------------

/// G-003: The `dueProcess.notice` section MUST declare `determinationField`,
/// `reasonCodes`, and `appealInstructions` when `impactLevel` is `rights-impacting`.
fn check_notice_individualized_for_rights(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if kernel.value.get("impactLevel").and_then(Value::as_str) != Some("rights-impacting") {
        return;
    }
    let Some(notice) = gov.value.pointer("/dueProcess/notice") else {
        return; // G-001 already reported the missing dueProcess.
    };
    let path = "/dueProcess/notice";
    for field in ["determinationField", "reasonCodes", "appealInstructions"] {
        if notice.get(field).is_none() {
            diagnostics.push(Diagnostic::warning(
                "G-003",
                path,
                format!("rights-impacting notice must declare '{field}' for individualized content"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-004: Explanation level must be individualized for rights-impacting
// ---------------------------------------------------------------------------

/// G-004: `dueProcess.explanationLevel` MUST be `"individualized"` when the
/// kernel's `impactLevel` is `rights-impacting`.
fn check_explanation_level_for_rights(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if kernel.value.get("impactLevel").and_then(Value::as_str) != Some("rights-impacting") {
        return;
    }
    let level = gov
        .value
        .pointer("/dueProcess/explanationLevel")
        .and_then(Value::as_str);
    if level != Some("individualized") {
        diagnostics.push(Diagnostic::error(
            "G-004",
            "/dueProcess/explanationLevel",
            "rights-impacting kernel requires explanationLevel 'individualized' in governance dueProcess",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-005: Counterfactual required for rights-impacting adverse decisions
// ---------------------------------------------------------------------------

/// G-005: Adverse decisions MUST include positive and negative counterfactuals
/// when `impactLevel` is `rights-impacting`.
fn check_counterfactual_for_rights(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if kernel.value.get("impactLevel").and_then(Value::as_str) != Some("rights-impacting") {
        return;
    }
    match gov.value.pointer("/dueProcess/counterfactuals") {
        None => diagnostics.push(Diagnostic::error(
            "G-005",
            "/dueProcess/counterfactuals",
            "rights-impacting kernel requires counterfactuals section with positive and negative entries",
        )),
        Some(cf) => {
            for polarity in ["positive", "negative"] {
                if cf.get(polarity).is_none() {
                    diagnostics.push(Diagnostic::error(
                        "G-005",
                        &format!("/dueProcess/counterfactuals/{polarity}"),
                        format!("rights-impacting adverse decision must declare '{polarity}' counterfactual"),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-008: continuationOfServices requires kernel topology to support it
// ---------------------------------------------------------------------------

/// G-008: When `continuationOfServices` is true, the kernel MUST have at
/// least one `hold`-tagged state (a static proxy for topology support).
///
/// Full freeze-during-appeal enforcement is a runtime property (T3).
fn check_continuation_of_services(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let declared = gov
        .value
        .pointer("/dueProcess/continuationOfServices")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !declared {
        return;
    }
    if !kernel_tags.contains("hold") {
        diagnostics.push(Diagnostic::warning(
            "G-008",
            "/dueProcess/continuationOfServices",
            "continuationOfServices is true but kernel has no state tagged 'hold'; topology may not support service continuation during appeal",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-009: adverse-decision tag transitions must enforce due process policy
// ---------------------------------------------------------------------------

/// G-009: When governance declares an `adverseDecisionPolicy`, the kernel
/// MUST have at least one transition tagged `adverse-decision`.
///
/// The runtime enforcement side of this rule is T3.
fn check_adverse_decision_due_process(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if gov.value.get("adverseDecisionPolicy").is_none() {
        return;
    }
    if !kernel_tags.contains("adverse-decision") {
        diagnostics.push(Diagnostic::error(
            "G-009",
            "/adverseDecisionPolicy",
            "governance declares adverseDecisionPolicy but kernel has no transition tagged 'adverse-decision'",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-011: Review protocol tags must exist in kernel
// ---------------------------------------------------------------------------

/// G-011: Review protocol tags MUST match tags that actually appear in the
/// target kernel document.
fn check_governance_tags_exist(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(protocols) = gov.value.get("reviewProtocols").and_then(Value::as_array) else {
        return;
    };
    for (i, protocol) in protocols.iter().enumerate() {
        let Some(tags) = protocol.get("tags").and_then(Value::as_array) else {
            continue;
        };
        for tag in tags.iter().filter_map(Value::as_str) {
            if !kernel_tags.contains(tag) {
                diagnostics.push(Diagnostic::warning(
                    "G-011",
                    &format!("/reviewProtocols/{i}/tags"),
                    format!("tag '{tag}' not found on any kernel state or transition"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-014: Reasoning tier required for determination-tagged transitions
// ---------------------------------------------------------------------------

/// G-014: Governance MUST declare a reasoning tier when the kernel has any
/// `determination`-tagged transitions.
fn check_reasoning_tier_for_determination(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !kernel_tags.contains("determination") {
        return;
    }
    let has_reasoning = gov.value.get("reasoningTier").is_some()
        || gov.value.pointer("/provenanceTiers/reasoning").is_some();
    if !has_reasoning {
        diagnostics.push(Diagnostic::error(
            "G-014",
            "/reasoningTier",
            "kernel has determination-tagged transitions; governance MUST declare a reasoning tier",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-015: Counterfactual tier required for adverse-decision in rights-impacting
// ---------------------------------------------------------------------------

/// G-015: Governance MUST declare a counterfactual tier when the workflow is
/// `rights-impacting` and has `adverse-decision`-tagged transitions.
fn check_counterfactual_tier_for_adverse(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if kernel.value.get("impactLevel").and_then(Value::as_str) != Some("rights-impacting") {
        return;
    }
    if !kernel_tags.contains("adverse-decision") {
        return;
    }
    let has_counterfactual = gov.value.get("counterfactualTier").is_some()
        || gov.value.pointer("/provenanceTiers/counterfactual").is_some();
    if !has_counterfactual {
        diagnostics.push(Diagnostic::error(
            "G-015",
            "/counterfactualTier",
            "rights-impacting workflow with adverse-decision transitions MUST declare a counterfactual tier",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-022: excludedOwner overrides potentialOwner
// ---------------------------------------------------------------------------

/// G-022: When an actor appears in both `potentialOwner` and `excludedOwner`,
/// `excludedOwner` takes precedence — warn so authors can verify the intent.
fn check_excluded_owner_override(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(tasks) = gov.value.get("tasks").and_then(Value::as_object) else {
        return;
    };
    for (task_name, task) in tasks {
        let potential: std::collections::HashSet<&str> = task
            .get("potentialOwner")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let excluded: std::collections::HashSet<&str> = task
            .get("excludedOwner")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for actor in potential.intersection(&excluded) {
            diagnostics.push(Diagnostic::warning(
                "G-022",
                &format!("/tasks/{task_name}"),
                format!("actor '{actor}' is in both potentialOwner and excludedOwner; excludedOwner takes precedence — verify this is intentional"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-023: SLA uses business calendar when sidecar present
// ---------------------------------------------------------------------------

/// G-023: SLA evaluation MUST use business calendar days when a Business
/// Calendar sidecar is present in the project.
fn check_sla_business_calendar(
    gov: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Detect a business calendar sidecar by any document that carries the
    // $wosBusinessCalendar marker key or a top-level businessCalendar field.
    let has_calendar = project.documents().iter().any(|d| {
        d.value.get("$wosBusinessCalendar").is_some() || d.value.get("businessCalendar").is_some()
    });
    if !has_calendar {
        return;
    }
    let Some(tasks) = gov.value.get("tasks").and_then(Value::as_object) else {
        return;
    };
    for (task_name, task) in tasks {
        let Some(sla) = task.get("sla") else { continue };
        let uses_business = sla
            .get("calendarType")
            .and_then(Value::as_str)
            .map(|c| c == "business")
            .unwrap_or(false);
        if !uses_business {
            diagnostics.push(Diagnostic::warning(
                "G-023",
                &format!("/tasks/{task_name}/sla"),
                "a business calendar sidecar is present; SLA should set calendarType to 'business'",
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-024: Delegation verification on determination-tagged transitions
// ---------------------------------------------------------------------------

/// G-024: When the kernel has `determination`-tagged transitions, governance
/// SHOULD declare `delegationVerification` or a non-empty `delegations` list.
fn check_delegation_verification_on_determination(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !kernel_tags.contains("determination") {
        return;
    }
    let has_verification = gov.value.get("delegationVerification").is_some()
        || gov
            .value
            .get("delegations")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
    if !has_verification {
        diagnostics.push(Diagnostic::warning(
            "G-024",
            "/delegationVerification",
            "kernel has determination-tagged transitions; governance should declare delegationVerification or delegations",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-027: Sub-delegation depth traversal
// ---------------------------------------------------------------------------

/// G-027: Sub-delegation MUST respect `maxDelegationDepth`.
///
/// Traverses the delegation chain and reports any branch that exceeds the
/// declared ceiling.
fn check_sub_delegation_depth(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(max_depth) = gov.value.get("maxDelegationDepth").and_then(Value::as_u64) else {
        return; // No ceiling declared; nothing to enforce statically.
    };
    let max_depth = max_depth as usize;
    let Some(delegations) = gov.value.get("delegations").and_then(Value::as_array) else {
        return;
    };

    // Build (delegate → delegator) pairs for chain traversal.
    let links: Vec<(&str, &str)> = delegations
        .iter()
        .filter_map(|d| {
            let delegator = d.get("delegator").and_then(Value::as_str)?;
            let delegate = d.get("delegate").and_then(Value::as_str)?;
            Some((delegate, delegator))
        })
        .collect();

    for (i, delegation) in delegations.iter().enumerate() {
        let Some(delegate) = delegation.get("delegate").and_then(Value::as_str) else {
            continue;
        };
        let depth = delegation_chain_depth(delegate, &links, 0);
        if depth > max_depth {
            diagnostics.push(Diagnostic::error(
                "G-027",
                &format!("/delegations/{i}"),
                format!("sub-delegation chain depth {depth} exceeds maxDelegationDepth {max_depth}"),
            ));
        }
    }
}

/// Walk the delegation chain for `actor`, counting hops upward.
///
/// Terminates at a hard ceiling of 64 levels to avoid runaway loops on
/// cycles (cycles are a T1/T3 violation handled separately).
fn delegation_chain_depth(actor: &str, links: &[(&str, &str)], current: usize) -> usize {
    // 64 is far beyond any realistic delegation depth.
    const DEPTH_CEILING: usize = 64;
    if current >= DEPTH_CEILING {
        return current;
    }
    match links.iter().find(|(delegate, _)| *delegate == actor) {
        Some((_, parent)) => delegation_chain_depth(parent, links, current + 1),
        None => current,
    }
}

// ---------------------------------------------------------------------------
// G-028: Hold policies attach to hold-tagged kernel states
// ---------------------------------------------------------------------------

/// G-028: Every `holdPolicy.stateRef` MUST reference a kernel state that
/// carries the `hold` tag.
fn check_hold_policies_on_hold_states(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let hold_states = collect_kernel_states_with_tag(kernel, "hold");
    let Some(hold_policies) = gov.value.get("holdPolicies").and_then(Value::as_array) else {
        return;
    };
    for (i, policy) in hold_policies.iter().enumerate() {
        let Some(state_ref) = policy.get("stateRef").and_then(Value::as_str) else {
            continue;
        };
        if !hold_states.contains(state_ref) {
            diagnostics.push(Diagnostic::error(
                "G-028",
                &format!("/holdPolicies/{i}/stateRef"),
                format!("holdPolicy references state '{state_ref}' which is not tagged 'hold' in the kernel"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-029: Hold resumeTrigger must correspond to a kernel event
// ---------------------------------------------------------------------------

/// G-029: `holdPolicy.resumeTrigger` MUST correspond to an event in the
/// target kernel document.
fn check_hold_resume_triggers(
    gov: &crate::document::WosDocument,
    kernel_events: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(holds) = gov.value.get("holdPolicies").and_then(Value::as_array) else {
        return;
    };
    for (i, hold) in holds.iter().enumerate() {
        let Some(trigger) = hold.get("resumeTrigger").and_then(Value::as_str) else {
            continue;
        };
        if !kernel_events.contains(trigger) {
            diagnostics.push(Diagnostic::warning(
                "G-029",
                &format!("/holdPolicies/{i}/resumeTrigger"),
                format!("resumeTrigger '{trigger}' not found as a kernel event"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-031: resolutionDateRef (governance direct embed)
// ---------------------------------------------------------------------------

/// G-031: `resolutionDateRef` in a governance document MUST point to a kernel
/// case file field.
///
/// Governance documents that embed `resolutionDateRef` directly (outside of
/// a PolicyParameters sidecar) are validated here. The PolicyParameters path
/// is handled by `check_policy_param_date_refs`.
fn check_resolution_date_refs(
    gov: &crate::document::WosDocument,
    kernel_case_fields: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = kernel_case_fields;

    if let Some(sla_config) = gov.value.get("slaConfig") {
        if let Some(date_ref) = sla_config.get("resolutionDateRef").and_then(Value::as_str) {
            if !fields.contains(date_ref) {
                diagnostics.push(Diagnostic::warning(
                    "G-031",
                    "/slaConfig/resolutionDateRef",
                    format!("resolutionDateRef '{date_ref}' not found in kernel caseFile.fields"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-033: Parameter coverage for early dates
// ---------------------------------------------------------------------------

/// G-033: A parameter `values` array that is empty means no resolution date
/// is covered — warn because behavior is undefined.
fn check_parameter_coverage(gov: &crate::document::WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let Some(params) = gov.value.get("parameters").and_then(Value::as_object) else {
        return;
    };
    for (name, param) in params {
        if let Some(values) = param.get("values").and_then(Value::as_array) {
            if values.is_empty() {
                diagnostics.push(Diagnostic::warning(
                    "G-033",
                    &format!("/parameters/{name}/values"),
                    format!("parameter '{name}' has no values entries; resolution date may not be covered"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-035: targetGovernance references valid governance document
// ---------------------------------------------------------------------------

/// G-035: `targetGovernance` in a DueProcess document MUST match the `url`
/// of a WorkflowGovernance document loaded in the project.
fn check_target_governance_valid(
    dp: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(target) = dp.value.get("targetGovernance").and_then(Value::as_str) else {
        return;
    };
    let governance_urls: std::collections::HashSet<&str> = project
        .of_kind(DocumentKind::WorkflowGovernance)
        .filter_map(|g| g.value.get("url").and_then(Value::as_str))
        .collect();
    if !governance_urls.contains(target) {
        diagnostics.push(Diagnostic::error(
            "G-035",
            "/targetGovernance",
            format!("targetGovernance '{target}' does not match any governance document url in the project"),
        ));
    }
}

// ---------------------------------------------------------------------------
// G-036: independenceConstraint encodes actual prevention
// ---------------------------------------------------------------------------

/// G-036: `independenceConstraint` MUST encode a mechanism preventing the
/// original decision-maker from reviewing.
///
/// Statically checks the field is present and non-empty; semantic adequacy of
/// the constraint content is a T3 property.
fn check_independence_constraint(gov: &crate::document::WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if gov.value.get("reviewProtocols").is_none() {
        return;
    }
    match gov.value.get("independenceConstraint") {
        None => diagnostics.push(Diagnostic::warning(
            "G-036",
            "/independenceConstraint",
            "governance has reviewProtocols but no independenceConstraint; must encode prevention of self-review",
        )),
        Some(c) => {
            let is_empty = c.as_str().map(str::is_empty).unwrap_or(false)
                || c.as_object().is_some_and(|m| m.is_empty())
                || c.as_array().is_some_and(|a| a.is_empty());
            if is_empty {
                diagnostics.push(Diagnostic::warning(
                    "G-036",
                    "/independenceConstraint",
                    "independenceConstraint is empty; must encode an actual prevention mechanism",
                ));
            }
        }
    }
}

/// G-036 (DueProcess variant): same check applied to DueProcess documents.
fn check_independence_constraint_in_due_process(
    dp: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if dp.value.get("reviewProtocols").is_some() && dp.value.get("independenceConstraint").is_none() {
        diagnostics.push(Diagnostic::warning(
            "G-036",
            "/independenceConstraint",
            "due-process document has reviewProtocols but no independenceConstraint",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-040: Consistency assertion referenceStage exists in pipeline
// ---------------------------------------------------------------------------

/// G-040: When assertion type is `consistency`, `referenceStage` MUST refer
/// to a pipeline stage id that exists in a governance document in the project.
fn check_consistency_reference_stage(
    al: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(assertions) = al.value.get("assertions").and_then(Value::as_array) else {
        return;
    };
    let pipeline_stages: std::collections::HashSet<String> = project
        .of_kind(DocumentKind::WorkflowGovernance)
        .filter_map(|g| g.value.get("pipeline").and_then(Value::as_array))
        .flatten()
        .filter_map(|stage| stage.get("id").and_then(Value::as_str))
        .map(String::from)
        .collect();
    for (i, assertion) in assertions.iter().enumerate() {
        if assertion.get("type").and_then(Value::as_str) != Some("consistency") {
            continue;
        }
        let Some(ref_stage) = assertion.get("referenceStage").and_then(Value::as_str) else {
            continue; // G-039 (T1) handles the missing-field case.
        };
        if !pipeline_stages.contains(ref_stage) {
            diagnostics.push(Diagnostic::error(
                "G-040",
                &format!("/assertions/{i}/referenceStage"),
                format!("referenceStage '{ref_stage}' is not a pipeline stage id in any governance document"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-041: Pipeline assertion ids exist in targeted library
// ---------------------------------------------------------------------------

/// G-041: Every assertion `id` referenced by a governance pipeline stage MUST
/// exist in an assertion library document in the project.
fn check_pipeline_assertion_ids(
    gov: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(pipeline) = gov.value.get("pipeline").and_then(Value::as_array) else {
        return;
    };
    let library_ids: std::collections::HashSet<String> = project
        .of_kind(DocumentKind::AssertionLibrary)
        .filter_map(|al| al.value.get("assertions").and_then(Value::as_array))
        .flatten()
        .filter_map(|a| a.get("id").and_then(Value::as_str))
        .map(String::from)
        .collect();
    for (si, stage) in pipeline.iter().enumerate() {
        let Some(refs) = stage.get("assertions").and_then(Value::as_array) else {
            continue;
        };
        for (ai, assertion_ref) in refs.iter().enumerate() {
            // The ref may be a plain string id or an object with an "id" field.
            let id = assertion_ref
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| assertion_ref.as_str());
            let Some(id) = id else { continue };
            if !library_ids.contains(id) {
                diagnostics.push(Diagnostic::error(
                    "G-041",
                    &format!("/pipeline/{si}/assertions/{ai}"),
                    format!("assertion id '{id}' not found in any assertion library in the project"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-046: Delegation actors must exist in kernel
// ---------------------------------------------------------------------------

/// G-046: `delegator` and `delegate` MUST correspond to actors in the target
/// kernel document.
fn check_delegation_actors_exist(
    gov: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let kernel_actors: std::collections::HashSet<&str> = kernel
        .value
        .get("actors")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("id").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    let Some(delegations) = gov.value.get("delegations").and_then(Value::as_array) else {
        return;
    };
    for (i, delegation) in delegations.iter().enumerate() {
        let path = format!("/delegations/{i}");
        for field in ["delegator", "delegate"] {
            if let Some(actor) = delegation.get(field).and_then(Value::as_str) {
                if !kernel_actors.contains(actor) {
                    diagnostics.push(Diagnostic::warning(
                        "G-046",
                        &path,
                        format!("{field} '{actor}' not found in kernel actors"),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-053: Sub-delegation only if original permits
// ---------------------------------------------------------------------------

/// G-053: Sub-delegation MUST only be permitted if the original delegation
/// explicitly sets `allowsSubDelegation: true`.
fn check_sub_delegation_permission(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(delegations) = gov.value.get("delegations").and_then(Value::as_array) else {
        return;
    };
    // Actors who are themselves delegates may attempt to sub-delegate further.
    let delegates: std::collections::HashSet<&str> = delegations
        .iter()
        .filter_map(|d| d.get("delegate").and_then(Value::as_str))
        .collect();
    for (i, delegation) in delegations.iter().enumerate() {
        let Some(delegator) = delegation.get("delegator").and_then(Value::as_str) else {
            continue;
        };
        // This delegation is a sub-delegation only when its delegator is
        // themselves a delegate in another entry.
        if !delegates.contains(delegator) {
            continue;
        }
        let original_permits = delegations.iter().any(|d| {
            d.get("delegate").and_then(Value::as_str) == Some(delegator)
                && d.get("allowsSubDelegation")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        });
        if !original_permits {
            diagnostics.push(Diagnostic::error(
                "G-053",
                &format!("/delegations/{i}"),
                format!("delegator '{delegator}' is sub-delegating but their original delegation does not set allowsSubDelegation: true"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-056: Binding resolutionDateRef references kernel case field
// ---------------------------------------------------------------------------

/// G-056: `bindings[*].resolutionDateRef` MUST reference a field path that
/// exists in the kernel's case state.
fn check_binding_resolution_date_refs(
    gov: &crate::document::WosDocument,
    kernel_case_fields: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = kernel_case_fields;
    let Some(bindings) = gov.value.get("bindings").and_then(Value::as_object) else {
        return;
    };
    for (binding_name, binding) in bindings {
        let Some(date_ref) = binding.get("resolutionDateRef").and_then(Value::as_str) else {
            continue;
        };
        if !fields.contains(date_ref) {
            diagnostics.push(Diagnostic::warning(
                "G-056",
                &format!("/bindings/{binding_name}/resolutionDateRef"),
                format!("resolutionDateRef '{date_ref}' not found in kernel caseFile.fields"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-031: Policy parameter resolutionDateRef (PolicyParameters sidecar)
// ---------------------------------------------------------------------------

/// G-031: Policy parameter `resolutionDateRef` must point to a kernel case
/// file field.
fn check_policy_param_date_refs(
    pp: &crate::document::WosDocument,
    kernel_case_fields: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = kernel_case_fields;
    let Some(params) = pp.value.get("parameters").and_then(Value::as_object) else {
        return;
    };
    for (name, param) in params {
        let Some(date_ref) = param.get("resolutionDateRef").and_then(Value::as_str) else {
            continue;
        };
        if !fields.contains(date_ref) {
            diagnostics.push(Diagnostic::warning(
                "G-031",
                &format!("/parameters/{name}/resolutionDateRef"),
                format!("resolutionDateRef '{date_ref}' not found in kernel caseFile.fields"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-007: cascadingInvocations declared for autonomous-invoking-autonomous
// ---------------------------------------------------------------------------

/// AI-007: When any autonomous agent invokes another autonomous agent,
/// `cascadingInvocations` MUST be declared in the AI document.
fn check_cascading_invocations_declared(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    let autonomous: std::collections::HashSet<&str> = agents
        .iter()
        .filter(|(_, a)| a.get("autonomy").and_then(Value::as_str) == Some("autonomous"))
        .map(|(name, _)| name.as_str())
        .collect();
    if autonomous.len() < 2 {
        return;
    }
    let cascades = agents.values().any(|agent| {
        agent
            .get("invokes")
            .and_then(Value::as_array)
            .is_some_and(|invocations| {
                invocations.iter().any(|inv| {
                    inv.as_str()
                        .or_else(|| inv.get("agentId").and_then(Value::as_str))
                        .map(|id| autonomous.contains(id))
                        .unwrap_or(false)
                })
            })
    });
    if cascades && ai.value.get("cascadingInvocations").is_none() {
        diagnostics.push(Diagnostic::error(
            "AI-007",
            "/cascadingInvocations",
            "autonomous agents invoke other autonomous agents; cascadingInvocations MUST be declared",
        ));
    }
}

// ---------------------------------------------------------------------------
// AI-018: autonomous actions have deontic constraints
// ---------------------------------------------------------------------------

/// AI-018: `autonomous` agents MUST have associated deontic constraints.
fn check_autonomous_actions_have_deontic(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        if agent.get("autonomy").and_then(Value::as_str) != Some("autonomous") {
            continue;
        }
        let has_deontic = agent.get("deonticConstraints").is_some()
            || agent.get("permissions").is_some()
            || agent.get("prohibitions").is_some()
            || agent.get("obligations").is_some();
        if !has_deontic {
            diagnostics.push(Diagnostic::warning(
                "AI-018",
                &format!("/agents/{name}"),
                format!("autonomous agent '{name}' should have deontic constraints (permissions, prohibitions, or obligations)"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-020: supervisory actions define reviewWindow
// ---------------------------------------------------------------------------

/// AI-020: `supervisory` agents MUST define `reviewWindow`.
fn check_supervisory_actions_review_window(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        if agent.get("autonomy").and_then(Value::as_str) != Some("supervisory") {
            continue;
        }
        if agent.get("reviewWindow").is_none() {
            diagnostics.push(Diagnostic::warning(
                "AI-020",
                &format!("/agents/{name}"),
                format!("supervisory agent '{name}' should define reviewWindow"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-026: escalationExpiry present on escalation rules
// ---------------------------------------------------------------------------

/// AI-026: Escalation MUST have `escalationExpiry`; agent reverts when expired.
fn check_escalation_expiry_present(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        let Some(rules) = agent.get("escalationRules").and_then(Value::as_array) else {
            continue;
        };
        for (i, rule) in rules.iter().enumerate() {
            if rule.get("escalationExpiry").is_none() {
                diagnostics.push(Diagnostic::warning(
                    "AI-026",
                    &format!("/agents/{name}/escalationRules/{i}"),
                    format!("escalation rule in agent '{name}' should declare escalationExpiry"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AI-031: Agent output contract same as human-facing form
// ---------------------------------------------------------------------------

/// AI-031: The agent `outputContract.formUrl` MUST apply the same rules as
/// the kernel's human-facing `formUrl`.
fn check_agent_output_contract(
    ai: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(kernel_form_url) = kernel.value.get("formUrl").and_then(Value::as_str) else {
        return; // No human-facing form declared; nothing to compare.
    };
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        let Some(contract) = agent.get("outputContract") else { continue };
        let contract_form = contract.get("formUrl").and_then(Value::as_str);
        if contract_form != Some(kernel_form_url) {
            diagnostics.push(Diagnostic::warning(
                "AI-031",
                &format!("/agents/{name}/outputContract/formUrl"),
                format!(
                    "agent '{name}' outputContract formUrl '{}' should match kernel formUrl '{kernel_form_url}'",
                    contract_form.unwrap_or("<missing>")
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-042 / AI-043: Training data and optimization objective disclosure
// ---------------------------------------------------------------------------

/// AI-042: Agent config MUST disclose training data characteristics.
fn check_training_data_disclosure(ai: &crate::document::WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        let has_disclosure = agent
            .get("modelConfig")
            .and_then(|c| c.get("trainingDataCharacteristics"))
            .is_some();
        if !has_disclosure {
            diagnostics.push(Diagnostic::warning(
                "AI-042",
                &format!("/agents/{name}/modelConfig/trainingDataCharacteristics"),
                format!("agent '{name}' should disclose training data characteristics in modelConfig"),
            ));
        }
    }
}

/// AI-043: Agent config MUST disclose optimization objective.
fn check_optimization_objective_disclosure(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        let has_objective = agent
            .get("modelConfig")
            .and_then(|c| c.get("optimizationObjective"))
            .is_some();
        if !has_objective {
            diagnostics.push(Diagnostic::warning(
                "AI-043",
                &format!("/agents/{name}/modelConfig/optimizationObjective"),
                format!("agent '{name}' should disclose optimization objective in modelConfig"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-046: Disclosure for impact
// ---------------------------------------------------------------------------

/// AI-046: Rights-impacting or safety-impacting workflows require
/// `agentDisclosure.discloseThatAgentAssisted: true`.
fn check_ai_disclosure_for_impact(
    ai: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_rights_or_safety_impacting(kernel) {
        return;
    }
    let disclosed = ai
        .value
        .get("agentDisclosure")
        .and_then(|d| d.get("discloseThatAgentAssisted"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !disclosed {
        diagnostics.push(Diagnostic::error(
            "AI-046",
            "/agentDisclosure/discloseThatAgentAssisted",
            "rights-impacting or safety-impacting workflow requires discloseThatAgentAssisted: true",
        ));
    }
}

// ---------------------------------------------------------------------------
// AI-056: Autonomy is action-site property across document
// ---------------------------------------------------------------------------

/// AI-056: Autonomy is an action-site property, not an agent-level default.
///
/// Warns when an agent declares a global `autonomy` without any per-action
/// site overrides, which hides site-specific autonomy differences.
fn check_autonomy_is_action_site_property(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(agents) = ai.value.get("agents").and_then(Value::as_object) else {
        return;
    };
    for (name, agent) in agents {
        if agent.get("autonomy").is_none() {
            continue;
        }
        let has_action_sites = agent.get("actionSites").is_some()
            || agent
                .get("actions")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|action| action.get("autonomy").is_some()));
        if !has_action_sites {
            diagnostics.push(Diagnostic::warning(
                "AI-056",
                &format!("/agents/{name}/autonomy"),
                format!("agent '{name}' sets autonomy at agent level; autonomy should be declared per action site"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AG-008: Side-effect tools at autonomous need sideEffectPolicy
// ---------------------------------------------------------------------------

/// AG-008: Side-effect tools at `autonomous` autonomy level MUST have a
/// `sideEffectPolicy`.
fn check_side_effect_tools_policy(
    adv: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(tools) = adv.value.get("tools").and_then(Value::as_object) else {
        return;
    };
    for (tool_name, tool) in tools {
        let is_side_effect = tool
            .get("hasSideEffects")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let autonomy = tool.get("autonomy").and_then(Value::as_str);
        if is_side_effect && autonomy == Some("autonomous") && tool.get("sideEffectPolicy").is_none() {
            diagnostics.push(Diagnostic::warning(
                "AG-008",
                &format!("/tools/{tool_name}"),
                format!("tool '{tool_name}' has side effects at autonomous level but no sideEffectPolicy"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AG-017: Shadow mode recommended for rights-impacting
// ---------------------------------------------------------------------------

/// AG-017: Shadow mode is RECOMMENDED before granting operational authority
/// in rights-impacting workflows.
fn check_shadow_mode_recommended(
    adv: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_rights_or_safety_impacting(kernel) {
        return;
    }
    let has_shadow = adv.value.get("shadowMode").is_some()
        || adv
            .value
            .get("deploymentSequence")
            .and_then(Value::as_array)
            .is_some_and(|stages| {
                stages
                    .iter()
                    .any(|s| s.get("mode").and_then(Value::as_str) == Some("shadow"))
            });
    if !has_shadow {
        diagnostics.push(Diagnostic::warning(
            "AG-017",
            "/shadowMode",
            "rights-impacting workflow: shadow mode is recommended before granting operational authority",
        ));
    }
}

// ---------------------------------------------------------------------------
// DM-002: Rights/safety workflows should follow deployment sequence
// ---------------------------------------------------------------------------

/// DM-002: Rights/safety workflows SHOULD follow the shadow → canary →
/// production deployment sequence.
fn check_deployment_sequence(
    dm: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_rights_or_safety_impacting(kernel) {
        return;
    }
    let stages: Vec<&str> = dm
        .value
        .get("deploymentSequence")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    for phase in ["shadow", "canary", "production"] {
        if !stages.contains(&phase) {
            diagnostics.push(Diagnostic::warning(
                "DM-002",
                "/deploymentSequence",
                format!("rights/safety-impacting workflow: deployment sequence should include '{phase}' phase"),
            ));
        }
    }

    // Order checks: shadow before canary, canary before production.
    let phase_order = [("shadow", "canary"), ("canary", "production")];
    for (earlier, later) in phase_order {
        let earlier_pos = stages.iter().position(|&s| s == earlier);
        let later_pos = stages.iter().position(|&s| s == later);
        if let (Some(ep), Some(lp)) = (earlier_pos, later_pos) {
            if ep > lp {
                diagnostics.push(Diagnostic::warning(
                    "DM-002",
                    "/deploymentSequence",
                    format!("'{earlier}' phase should precede '{later}' phase in deployment sequence"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// VR-003: counterexample present when result is proven-unsafe
// ---------------------------------------------------------------------------

/// VR-003: `counterexample` MUST be present when a verification result is
/// `proven-unsafe`.
fn check_counterexample_on_unsafe(
    vr: &crate::document::WosDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(results) = vr.value.get("results").and_then(Value::as_array) else {
        return;
    };
    for (i, result) in results.iter().enumerate() {
        if result.get("result").and_then(Value::as_str) == Some("proven-unsafe")
            && result.get("counterexample").is_none()
        {
            diagnostics.push(Diagnostic::error(
                "VR-003",
                &format!("/results/{i}/counterexample"),
                "result is 'proven-unsafe' but counterexample is missing",
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return true if the kernel's `impactLevel` is `rights-impacting` or
/// `safety-impacting`.
fn is_rights_or_safety_impacting(kernel: &crate::document::WosDocument) -> bool {
    matches!(
        kernel.value.get("impactLevel").and_then(Value::as_str),
        Some("rights-impacting") | Some("safety-impacting")
    )
}

/// Collect all field paths from the kernel's `caseFile.fields` map, formatted
/// as `caseFile.<field_name>` for direct comparison with `resolutionDateRef`.
fn collect_kernel_case_fields(kernel: &crate::document::WosDocument) -> std::collections::HashSet<String> {
    kernel
        .value
        .pointer("/caseFile/fields")
        .and_then(Value::as_object)
        .map(|m| m.keys().map(|k| format!("caseFile.{k}")).collect())
        .unwrap_or_default()
}

/// Collect all tags from kernel states and transitions (recursive).
fn collect_kernel_tags(kernel: &crate::document::WosDocument) -> std::collections::HashSet<String> {
    let mut tags = std::collections::HashSet::new();
    if let Some(states) = kernel.value.pointer("/lifecycle/states").and_then(Value::as_object) {
        collect_tags_recursive(states, &mut tags);
    }
    tags
}

fn collect_tags_recursive(
    states: &serde_json::Map<String, Value>,
    tags: &mut std::collections::HashSet<String>,
) {
    for state in states.values() {
        if let Some(state_tags) = state.get("tags").and_then(Value::as_array) {
            for tag in state_tags.iter().filter_map(Value::as_str) {
                tags.insert(tag.to_string());
            }
        }
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for transition in transitions {
                if let Some(t_tags) = transition.get("tags").and_then(Value::as_array) {
                    for tag in t_tags.iter().filter_map(Value::as_str) {
                        tags.insert(tag.to_string());
                    }
                }
            }
        }
        // Recurse into substates and regions.
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            collect_tags_recursive(substates, tags);
        }
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for region in regions.values() {
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    collect_tags_recursive(rstates, tags);
                }
            }
        }
    }
}

/// Collect the names of all kernel states that carry a specific tag.
fn collect_kernel_states_with_tag(
    kernel: &crate::document::WosDocument,
    tag: &str,
) -> std::collections::HashSet<String> {
    let mut matching = std::collections::HashSet::new();
    if let Some(states) = kernel.value.pointer("/lifecycle/states").and_then(Value::as_object) {
        collect_states_with_tag_recursive(states, tag, &mut matching);
    }
    matching
}

fn collect_states_with_tag_recursive(
    states: &serde_json::Map<String, Value>,
    tag: &str,
    matching: &mut std::collections::HashSet<String>,
) {
    for (name, state) in states {
        let has_tag = state
            .get("tags")
            .and_then(Value::as_array)
            .is_some_and(|tags| tags.iter().any(|t| t.as_str() == Some(tag)));
        if has_tag {
            matching.insert(name.clone());
        }
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            collect_states_with_tag_recursive(substates, tag, matching);
        }
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for region in regions.values() {
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    collect_states_with_tag_recursive(rstates, tag, matching);
                }
            }
        }
    }
}

/// Collect all event names from kernel transitions (recursive).
fn collect_kernel_events(kernel: &crate::document::WosDocument) -> std::collections::HashSet<String> {
    let mut events = std::collections::HashSet::new();
    if let Some(states) = kernel.value.pointer("/lifecycle/states").and_then(Value::as_object) {
        collect_events_recursive(states, &mut events);
    }
    events
}

fn collect_events_recursive(
    states: &serde_json::Map<String, Value>,
    events: &mut std::collections::HashSet<String>,
) {
    for state in states.values() {
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for transition in transitions {
                if let Some(event) = transition.get("event").and_then(Value::as_str) {
                    events.insert(event.to_string());
                }
            }
        }
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            collect_events_recursive(substates, events);
        }
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for region in regions.values() {
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    collect_events_recursive(rstates, events);
                }
            }
        }
    }
}
