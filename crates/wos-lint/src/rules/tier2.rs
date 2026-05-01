// Rust guideline compliant 2026-02-21

//! Tier 2 lint rules — cross-document resolution + FEL AST analysis.
//!
//! These rules require loading multiple documents together (a "project")
//! and/or parsing FEL expressions. See LINT-MATRIX.md for the catalog.
//!
//! # Rule coverage
//!
//! ## Kernel — cross-path
//! | Rule      | Sev     | What is checked                                                      |
//! |-----------|---------|----------------------------------------------------------------------|
//! | K-010     | error   | createTask assignTo MUST reference a declared kernel actor          |
//! | K-037     | error   | Fail-fast parallel regions MUST have an error-tagged final state    |
//! | K-049     | warning (LoadBearing) | Continuous-mode setData/guard dependency cycles (see `continuous_mode`) |
//! | K-050     | error   | Final state `outcomeCode` MUST NOT duplicate any entry in `tags` (Kernel S4.3) |
//! | K-EXT-002 | warning | Keys using the reserved `x-wos-*` namespace (Kernel §10.6)           |
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
//! | G-023 | warning | SLA should set calendarType=business when scoped calendar sidecar present |
//! | G-060 | error   | SLA MUST use business days when a Business Calendar targets this workflow (no kernel required) |
//! | G-063 | error   | `templateKey`, `notificationTemplateKey`, and `noticeTemplateKey` MUST resolve to template keys (no kernel required) |
//! | G-066 | error   | `BreachPolicy.escalationStepId` MUST resolve within the same task pattern (no kernel required) |
//! | G-024 | warning | Delegation verification config present when kernel has determination  |
//! | G-027 | error   | Sub-delegation chain depth must not exceed maxDelegationDepth        |
//! | G-028 | error   | hold policies MUST attach to hold-tagged kernel states               |
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
//! | AI-023 | error   | agent-free completion path must be reachable                       |
//! | AI-026 | warning | escalationRules should declare escalationExpiry                     |
//! | AI-031 | warning | agent outputContract formUrl should match kernel formUrl            |
//! | AI-042 | warning | agent modelConfig should disclose trainingDataCharacteristics       |
//! | AI-043 | warning | agent modelConfig should disclose optimizationObjective             |
//! | AI-046 | error   | rights-impacting kernel requires discloseThatAgentAssisted          |
//! | AI-056 | warning | autonomy should be declared per action site, not at agent level     |
//!
//! ## Advanced governance / drift / verification
//! | Rule   | Sev     | What is checked                                                     |
//! |--------|---------|---------------------------------------------------------------------|
//! | AG-008 | warning | side-effect tools at autonomous need sideEffectPolicy               |
//! | AG-017 | warning | shadow mode recommended for rights-impacting                        |
//! | DM-002 | warning | rights/safety workflows should follow shadow→canary→production      |
//! | VR-003 | error   | counterexample required when result is proven-unsafe                |

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use wos_core::model::kernel::ActorKind;
use wos_core::model::kernel::{CancellationPolicy, ImpactLevel, KernelDocument, State, StateKind};

use crate::diagnostic::LintDiagnostic;
use crate::document::{DocumentKind, WosProject};

use super::fel_analysis;

// ---------------------------------------------------------------------------
// Pre-computed kernel index
// ---------------------------------------------------------------------------

/// Pre-computed index of kernel data used by multiple Tier 2 rules.
///
/// Building these collections once per `check()` call avoids repeatedly
/// walking the entire typed state tree for each governance document rule.
struct KernelCollections {
    /// All tags appearing on any kernel state or transition.
    tags: std::collections::HashSet<String>,
    /// All event names referenced by kernel transitions.
    events: std::collections::HashSet<String>,
    /// All case file field names in `caseFile.fields` (as `"caseFile.<name>"`).
    case_fields: std::collections::HashSet<String>,
    /// All declared actor IDs from the kernel `actors` array.
    actor_ids: std::collections::HashSet<String>,
    /// All human actor IDs from the kernel `actors` array.
    human_actor_ids: std::collections::HashSet<String>,
}

impl KernelCollections {
    fn from_typed(kernel: &KernelDocument) -> Self {
        let mut tags = std::collections::HashSet::new();
        collect_tags_typed(&kernel.lifecycle.states, &mut tags);

        let mut events = std::collections::HashSet::new();
        collect_events_typed(&kernel.lifecycle.states, &mut events);

        let case_fields = kernel
            .case_file
            .as_ref()
            .map(|cf| cf.fields.keys().map(|k| format!("caseFile.{k}")).collect())
            .unwrap_or_default();

        let actor_ids = kernel.actors.iter().map(|a| a.id.clone()).collect();
        let human_actor_ids = kernel
            .actors
            .iter()
            .filter(|actor| actor.kind == ActorKind::Human)
            .map(|actor| actor.id.clone())
            .collect();

        Self {
            tags,
            events,
            case_fields,
            actor_ids,
            human_actor_ids,
        }
    }
}

fn check_outcome_code_not_in_tags(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    check_outcome_code_recursive(&kernel.lifecycle.states, "/lifecycle/states", diagnostics);
}

fn check_outcome_code_recursive(
    states: &indexmap::IndexMap<String, State>,
    parent_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, state) in states {
        let state_path = format!("{parent_path}/{name}");
        if let Some(ref code) = state.outcome_code {
            if state.tags.iter().any(|t| t == code) {
                diagnostics.push(LintDiagnostic::t2_error(
                    "K-050",
                    &format!("{state_path}/outcomeCode"),
                    format!(
                        "state '{name}' outcomeCode '{code}' duplicates a tags entry; outcomeCode MUST be distinct from tags"
                    ),
                ));
            }
        }
        check_outcome_code_recursive(&state.states, &format!("{state_path}/states"), diagnostics);
        for (region_name, region) in &state.regions {
            check_outcome_code_recursive(
                &region.states,
                &format!("{state_path}/regions/{region_name}/states"),
                diagnostics,
            );
        }
    }
}

/// Collect tags from typed state tree.
fn collect_tags_typed(
    states: &indexmap::IndexMap<String, State>,
    tags: &mut std::collections::HashSet<String>,
) {
    for state in states.values() {
        for tag in &state.tags {
            tags.insert(tag.clone());
        }
        for transition in &state.transitions {
            for tag in &transition.tags {
                tags.insert(tag.clone());
            }
        }
        collect_tags_typed(&state.states, tags);
        for region in state.regions.values() {
            collect_tags_typed(&region.states, tags);
        }
    }
}

/// Collect events from typed state tree.
fn collect_events_typed(
    states: &indexmap::IndexMap<String, State>,
    events: &mut std::collections::HashSet<String>,
) {
    for state in states.values() {
        for transition in &state.transitions {
            if let Some(ev) = &transition.event {
                events.insert(ev.runtime_dispatch_label());
            }
        }
        collect_events_typed(&state.states, events);
        for region in state.regions.values() {
            collect_events_typed(&region.states, events);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all Tier 2 cross-document checks across the project.
///
/// After ADR 0076, a single `$wosWorkflow` document is the author-time
/// envelope. Governance, agents, signature, advanced, and other concerns
/// live as embedded blocks in that document's JSON. Cross-doc checks that
/// previously iterated over standalone `$wosWorkflowGovernance`, `$wosAIIntegration`,
/// etc., now read the corresponding embedded block from the workflow envelope.
///
/// Delivery (`$wosDelivery`) remains a separate sidecar for calendar,
/// notification templates, and correspondence.
pub fn check(project: &WosProject, diagnostics: &mut Vec<LintDiagnostic>) {
    // K-EXT-002: Reserved `x-wos-*` namespace check runs across every document.
    for doc in project.documents() {
        check_reserved_wos_namespace(&doc.value, "", diagnostics);
    }

    // The `$wosWorkflow` document is the single author-time source. All cross-doc
    // rules now read from its embedded blocks.
    let kernel_doc = project.kernel();
    let typed_kernel: Option<KernelDocument> =
        kernel_doc.and_then(|k| serde_json::from_value::<KernelDocument>(k.value.clone()).ok());

    let kernel_collections = typed_kernel.as_ref().map(KernelCollections::from_typed);

    if let (Some(kernel), Some(kc)) = (&typed_kernel, kernel_collections.as_ref()) {
        check_action_actor_references_typed(kernel, &kc.actor_ids, diagnostics);
        check_fail_fast_error_final_states_typed(kernel, diagnostics);
        check_outcome_code_not_in_tags(kernel, diagnostics);
        super::continuous_mode::check(kernel, diagnostics);
    }

    // Governance rules — read from $wosWorkflow.governance embedded block.
    // The governance block value is promoted to a synthetic WosDocument so
    // existing rule functions (which take &WosDocument) can run unchanged.
    for wf in project.of_kind(DocumentKind::Workflow) {
        // Promote governance block to a synthetic governance-shaped document.
        if let Some(gov_block) = wf.value.get("governance") {
            // Build a doc value that the governance rule functions expect:
            // they look for top-level fields like "targetWorkflow", "dueProcess",
            // "holdPolicies", "delegations", etc.
            let gov_url = wf.value.get("url").and_then(Value::as_str).unwrap_or("");
            let mut gov_value = gov_block.clone();
            // Inject targetWorkflow + url so cross-doc rules can resolve them.
            // The synthesized doc is dispatcher scaffolding — never emitted,
            // never validated against a schema, so no marker key is needed.
            if let Some(obj) = gov_value.as_object_mut() {
                obj.entry("targetWorkflow")
                    .or_insert_with(|| Value::String(gov_url.to_string()));
                obj.entry("url")
                    .or_insert_with(|| Value::String(gov_url.to_string()));
                // Flatten hold policies from holds.policies → holdPolicies for typed model.
                if let Some(holds_obj) = obj.get("holds").cloned() {
                    if let Some(policies) = holds_obj.get("policies") {
                        obj.entry("holdPolicies")
                            .or_insert_with(|| policies.clone());
                    }
                }
                // Flatten delegations from delegation.delegations → delegations.
                if let Some(del_obj) = obj.get("delegation").cloned() {
                    if let Some(delegations) = del_obj.get("delegations") {
                        obj.entry("delegations")
                            .or_insert_with(|| delegations.clone());
                    }
                    if let Some(max_depth) = del_obj.get("maxDelegationDepth") {
                        obj.entry("maxDelegationDepth")
                            .or_insert_with(|| max_depth.clone());
                    }
                }
            }
            let gov = crate::document::WosDocument {
                kind: DocumentKind::Workflow,
                value: gov_value,
                source: wf.source.clone(),
            };
            if let Some(target) = gov.value.get("targetWorkflow").and_then(Value::as_str)
                && target != gov_url
            {
                diagnostics.push(LintDiagnostic::t2_error(
                    "G-034",
                    "/governance/targetWorkflow",
                    format!(
                        "embedded governance targetWorkflow '{target}' does not match workflow url '{gov_url}'"
                    ),
                ));
            }
            if let Some(target_governance) =
                gov.value.get("targetGovernance").and_then(Value::as_str)
            {
                let expected_governance = gov
                    .value
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or(gov_url);
                if target_governance != expected_governance {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "G-035",
                        "/targetGovernance",
                        format!(
                            "targetGovernance '{target_governance}' does not match governance url '{expected_governance}'"
                        ),
                    ));
                }
            }
            let typed_gov =
                serde_json::from_value::<wos_core::GovernanceDocument>(gov.value.clone()).ok();

            // G-023 / G-060 / G-063 / G-066 need governance + project sidecars.
            check_sla_business_calendar(&gov, project, diagnostics);
            check_notification_template_refs(&gov, project, diagnostics);
            check_sla_escalation_step_ids(&gov, diagnostics);

            if let (Some(kernel), Some(kc)) = (&typed_kernel, kernel_collections.as_ref()) {
                if let Some(tg) = &typed_gov {
                    check_delegation_actors_exist_typed(tg, kernel, diagnostics);
                    check_hold_policies_attach_to_hold_states_typed(tg, kernel, diagnostics);
                    check_hold_resume_triggers_typed(tg, &kc.events, diagnostics);
                    check_sub_delegation_depth_typed(tg, diagnostics);
                }
                check_due_process_for_impact_typed(&gov, kernel, diagnostics);
                check_notice_individualized_for_rights_typed(&gov, kernel, diagnostics);
                check_explanation_level_for_rights_typed(&gov, kernel, diagnostics);
                check_counterfactual_for_rights_typed(&gov, kernel, diagnostics);
                check_counterfactual_tier_for_adverse_typed(&gov, kernel, &kc.tags, diagnostics);
                check_governance_tags_exist(&gov, &kc.tags, diagnostics);
                check_resolution_date_refs(&gov, &kc.case_fields, diagnostics);
                check_continuation_of_services(&gov, &kc.tags, diagnostics);
                check_adverse_decision_due_process(&gov, &kc.tags, diagnostics);
                check_reasoning_tier_for_determination(&gov, &kc.tags, diagnostics);
                check_excluded_owner_override(&gov, diagnostics);
                check_delegation_verification_on_determination(&gov, &kc.tags, diagnostics);
                check_binding_resolution_date_refs(&gov, &kc.case_fields, diagnostics);
            }

            check_parameter_coverage(&gov, diagnostics);
            check_independence_constraint(&gov, diagnostics);
            check_sub_delegation_permission_value(&gov, diagnostics);

            // Assertion library: governance.assertionLibrary or governance.assertions
            let al_value = gov
                .value
                .get("assertionLibrary")
                .cloned()
                .or_else(|| gov.value.get("assertions").map(|_| gov.value.clone()));
            if let Some(al_val) = al_value {
                let al_doc = crate::document::WosDocument {
                    kind: DocumentKind::Workflow,
                    value: al_val,
                    source: wf.source.clone(),
                };
                check_consistency_reference_stage(&al_doc, project, diagnostics);
            }
            check_pipeline_assertion_ids(&gov, project, diagnostics);
        }

        // AI integration rules — read from $wosWorkflow top-level `agents` /
        // `aiOversight` blocks. Existing AI-* rule fns were authored against
        // a standalone AI integration document; bridge by synthesizing one
        // in-process. The synthetic doc is internal scaffolding — never
        // emitted, never validated against any schema.
        let workflow_url = wf.value.get("url").and_then(Value::as_str).unwrap_or("");
        let has_agents = wf.value.get("agents").is_some() || wf.value.get("aiOversight").is_some();
        if has_agents {
            let mut ai_value = serde_json::json!({
                "targetWorkflow": workflow_url,
            });
            // Copy agents, aiOversight, fallbackChain, narrativeProvenance, provenance
            // from the workflow root into the synthetic AI doc.
            for field in &[
                "agents",
                "aiOversight",
                "fallbackChain",
                "narrativeProvenance",
                "provenance",
                "agentDisclosure",
            ] {
                if let Some(v) = wf.value.get(*field) {
                    ai_value[field] = v.clone();
                }
            }
            let ai_doc = crate::document::WosDocument {
                kind: DocumentKind::Workflow,
                value: ai_value,
                source: wf.source.clone(),
            };
            if let Some(kernel) = &typed_kernel {
                check_ai_disclosure_for_impact_typed(&ai_doc, kernel, diagnostics);
                if let Some(kernel_raw) = kernel_doc {
                    check_agent_output_contract(&ai_doc, kernel_raw, diagnostics);
                    check_agent_free_completion_path(&ai_doc, kernel_raw, diagnostics);
                }
            }
            check_cascading_invocations_declared(&ai_doc, diagnostics);
            check_autonomous_actions_have_deontic(&ai_doc, diagnostics);
            check_supervisory_actions_review_window(&ai_doc, diagnostics);
            check_escalation_expiry_present(&ai_doc, diagnostics);
            check_training_data_disclosure(&ai_doc, diagnostics);
            check_optimization_objective_disclosure(&ai_doc, diagnostics);
            check_autonomy_is_action_site_property(&ai_doc, diagnostics);
        }

        // Signature rules — read from $wosWorkflow.signature embedded block.
        // Existing SIG-* rule fns were authored against a standalone signature
        // profile document; bridge by synthesizing one in-process from the
        // embedded block. The kernel's url becomes the synthetic profile's
        // targetWorkflow.url unless the embedded block carries its own
        // targetWorkflow override (used by SIG-001 mismatch fixtures).
        if wf.value.get("signature").is_some() {
            let mut sig_value = serde_json::json!({
                "targetWorkflow": { "url": workflow_url },
            });
            if let Some(sig_block) = wf.value.get("signature") {
                if let Some(sig_obj) = sig_block.as_object() {
                    for (k, v) in sig_obj {
                        sig_value[k] = v.clone();
                    }
                }
            }
            let sig_doc = crate::document::WosDocument {
                kind: DocumentKind::Workflow,
                value: sig_value,
                source: wf.source.clone(),
            };
            if let (Some(kernel), Some(kc)) = (&typed_kernel, kernel_collections.as_ref()) {
                check_signature_profile(&sig_doc, kernel, kc, diagnostics);
            } else {
                check_signature_profile_without_kernel(&sig_doc, diagnostics);
            }
        }

        // Advanced governance rules — read from $wosWorkflow.advanced block.
        // Existing rule fns were authored against a standalone advanced doc;
        // bridge by synthesizing one in-process from the embedded block.
        if wf.value.get("advanced").is_some() {
            let mut adv_value = serde_json::json!({});
            if let Some(adv_block) = wf.value.get("advanced") {
                if let Some(adv_obj) = adv_block.as_object() {
                    for (k, v) in adv_obj {
                        adv_value[k] = v.clone();
                    }
                }
            }
            let adv_doc = crate::document::WosDocument {
                kind: DocumentKind::Workflow,
                value: adv_value,
                source: wf.source.clone(),
            };
            if let Some(kernel) = &typed_kernel {
                check_side_effect_tools_policy(&adv_doc, diagnostics);
                check_shadow_mode_recommended_typed(&adv_doc, kernel, diagnostics);
            }
        }

        // Drift monitoring — read from agents[*].driftMonitoring or a top-level
        // driftMonitoring block. Synthesize an internal drift-monitor doc to
        // satisfy existing rule-fn signatures; the synthetic doc is dispatcher
        // scaffolding, never validated against a schema.
        {
            let mut dm_value = serde_json::json!({});
            // Top-level deploymentSequence (from workflow.advanced or workflow root).
            if let Some(seq) = wf.value.pointer("/advanced/deploymentSequence") {
                dm_value["deploymentSequence"] = seq.clone();
            } else if let Some(seq) = wf.value.get("deploymentSequence") {
                dm_value["deploymentSequence"] = seq.clone();
            }
            if dm_value.get("deploymentSequence").is_some() {
                let dm_doc = crate::document::WosDocument {
                    kind: DocumentKind::Workflow,
                    value: dm_value,
                    source: wf.source.clone(),
                };
                if let Some(kernel) = &typed_kernel {
                    check_deployment_sequence_typed(&dm_doc, kernel, diagnostics);
                }
            }
        }

        // Verification results — read from $wosWorkflow.advanced.verifiableConstraints
        // or any provenance records with result: "proven-unsafe".
        // VR-003 checks are on the $wosProvenanceLog artifact, not author-time docs.
        // Moved to ProvenanceLog handling below.
    }

    // $wosDelivery: calendar and notification-template cross-doc rules.
    for delivery in project.of_kind(DocumentKind::Delivery) {
        // G-060/G-063: SLA + notification refs need governance from the workflow doc.
        // These rules previously needed a paired governance doc. Now we look for
        // the workflow doc that targets the same URL.
        let _ = delivery; // Delivery cross-doc rules are future work in this migration.
    }

    // $wosProvenanceLog: VR-003 counterexample check.
    for prov_log in project.of_kind(DocumentKind::ProvenanceLog) {
        // The VR-003 rule checks verification results. Map from provenance log
        // records with result: "proven-unsafe" to the VR-003 diagnostic.
        let vr_synthetic = crate::document::WosDocument {
            kind: DocumentKind::Workflow,
            value: prov_log.value.clone(),
            source: prov_log.source.clone(),
        };
        check_counterexample_on_unsafe(&vr_synthetic, diagnostics);
    }

    // ADR 0076 D-2 cross-reference rules on the workflow envelope.
    if let Some(kernel) = kernel_doc {
        check_agent_xref(&kernel.value, diagnostics);
        check_signature_coverage(&kernel.value, diagnostics);
    }

    // FEL AST analysis (T2-ast rules).
    fel_analysis::check(project, diagnostics);
}

// ---------------------------------------------------------------------------
// WOS-AGENT-XREF-001: every actor with type=='agent' MUST have a matching
// agents[].id (ADR 0076 D-2 cross-reference rule).
// ---------------------------------------------------------------------------

/// Walk the merged-document root for agent-typed actors and confirm each has a
/// matching `agents[].id`.
fn check_agent_xref(doc: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(actors) = doc.get("actors").and_then(Value::as_array) else {
        return;
    };

    let agent_ids: std::collections::HashSet<&str> = doc
        .get("agents")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("id").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();

    for (i, actor) in actors.iter().enumerate() {
        let is_agent = actor
            .get("type")
            .and_then(Value::as_str)
            .map(|t| t == "agent")
            .unwrap_or(false);
        if !is_agent {
            continue;
        }
        let Some(id) = actor.get("id").and_then(Value::as_str) else {
            // Agent-typed actor without `id` is a separate conformance error
            // (caught by schema), but still surface it here so the xref check
            // doesn't silently pass on a malformed actor.
            diagnostics.push(LintDiagnostic::t2_error(
                "WOS-AGENT-XREF-001",
                &format!("/actors/{i}"),
                "actor has type='agent' but no `id` field; cannot xref to agents[]".to_string(),
            ));
            continue;
        };
        if !agent_ids.contains(id) {
            diagnostics.push(LintDiagnostic::t2_error(
                "WOS-AGENT-XREF-001",
                &format!("/actors/{i}/id"),
                format!(
                    "actor id '{id}' has type='agent' but no matching agents[] entry; \
                     declare an agents[] entry with id='{id}' or change the actor type"
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// WOS-SIG-COVER-001: signature-gated transitions MUST be covered by signers
// (ADR 0076 D-2 cross-reference rule).
// ---------------------------------------------------------------------------

/// Walk lifecycle transitions for any with `on.kind == "signature"` and confirm
/// the document declares a signature block whose signers[] cover the gating
/// actor.
fn check_signature_coverage(doc: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    let signature_block = doc.get("signature");
    let signer_actor_ids: std::collections::HashSet<&str> = signature_block
        .and_then(|s| s.get("signers"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("actorId").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();

    let mut findings: Vec<(String, String)> = Vec::new();
    walk_transitions(
        doc.get("lifecycle").and_then(|l| l.get("states")),
        "/lifecycle/states",
        &mut |path, transition| {
            let is_signature_kind = transition
                .get("on")
                .and_then(|on| on.get("kind"))
                .and_then(Value::as_str)
                .map(|k| k == "signature")
                .unwrap_or(false);
            if !is_signature_kind {
                return;
            }

            if signature_block.is_none() {
                findings.push((
                    path.to_string(),
                    "the document has no signature block".to_string(),
                ));
                return;
            }
            if signer_actor_ids.is_empty() {
                findings.push((path.to_string(), "signature.signers[] is empty".to_string()));
                return;
            }

            // A signature-gated transition may name its signing actor on a
            // single `actor` field (single-signer case), an `actors[]` array
            // (multi-party signing where any/all listed actors sign), or
            // implicitly via signers[*].actorId when the transition omits both
            // (multi-signer ordering driven entirely by the signature block).
            // The xref check covers all three: each named actor MUST appear
            // in signers[]. When the transition names no actor, the rule
            // only requires signers[] to be non-empty (already checked above).
            let mut transition_actor_ids: Vec<&str> = Vec::new();
            if let Some(id) = transition.get("actor").and_then(Value::as_str) {
                transition_actor_ids.push(id);
            }
            if let Some(arr) = transition.get("actors").and_then(Value::as_array) {
                for v in arr {
                    if let Some(id) = v.as_str() {
                        transition_actor_ids.push(id);
                    }
                }
            }
            for id in transition_actor_ids {
                if !signer_actor_ids.contains(id) {
                    findings.push((
                        path.to_string(),
                        format!("signature.signers[] does not include actorId='{id}'"),
                    ));
                }
            }
        },
    );

    for (path, issue) in findings {
        diagnostics.push(LintDiagnostic::t2_error(
            "WOS-SIG-COVER-001",
            &path,
            format!("transition '{path}' is signature-gated but {issue}"),
        ));
    }
}

/// Recursively walk lifecycle.states, yielding (path, transition) pairs for
/// every transition encountered.
fn walk_transitions<F: FnMut(&str, &Value)>(
    states: Option<&Value>,
    path_prefix: &str,
    visit: &mut F,
) {
    let Some(Value::Object(map)) = states else {
        return;
    };
    for (state_name, state) in map {
        let state_path = format!("{path_prefix}/{}", json_pointer_escape(state_name));
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for (i, transition) in transitions.iter().enumerate() {
                let transition_path = format!("{state_path}/transitions/{i}");
                visit(&transition_path, transition);
            }
        }
        walk_transitions(state.get("states"), &format!("{state_path}/states"), visit);
        if let Some(Value::Object(regions)) = state.get("regions") {
            for (region_name, region) in regions {
                let region_path =
                    format!("{state_path}/regions/{}", json_pointer_escape(region_name));
                walk_transitions(
                    region.get("states"),
                    &format!("{region_path}/states"),
                    visit,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// K-010: createTask assignTo MUST reference a declared kernel actor
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// SIG-* Signature Profile rules
// ---------------------------------------------------------------------------

/// Run Signature Profile cross-document checks.
fn check_signature_profile(
    profile: &crate::document::WosDocument,
    kernel: &KernelDocument,
    kc: &KernelCollections,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    check_signature_target_workflow(profile, kernel, diagnostics);
    check_signature_roles(profile, kc, diagnostics);
    check_signature_auth_policies(profile, diagnostics);
    check_signature_steps(profile, diagnostics);
    check_signature_guards(profile, diagnostics);
    check_signature_lifecycle_tags(profile, kc, diagnostics);
    check_signature_timer_events(profile, kc, diagnostics);
    check_signature_evidence_inputs(profile, kc, diagnostics);
    check_signature_naming(profile, diagnostics);
}

/// Run Signature Profile checks that do not require a kernel document.
fn check_signature_profile_without_kernel(
    profile: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    check_signature_auth_policies(profile, diagnostics);
    check_signature_steps(profile, diagnostics);
    check_signature_guards(profile, diagnostics);
    check_signature_naming(profile, diagnostics);
}

fn check_signature_target_workflow(
    profile: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let target = pointer_str(&profile.value, "/targetWorkflow/url");
    let kernel_url = kernel.url.as_deref();
    if let (Some(target), Some(kernel_url)) = (target, kernel_url)
        && target != kernel_url
    {
        diagnostics.push(LintDiagnostic::t2_error(
            "SIG-001",
            "/targetWorkflow/url",
            format!("Signature Profile targetWorkflow.url '{target}' does not match kernel url '{kernel_url}'"),
        ));
    }
}

fn check_signature_roles(
    profile: &crate::document::WosDocument,
    kc: &KernelCollections,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(roles) = profile.value.pointer("/roles").and_then(Value::as_array) else {
        return;
    };
    for (index, role) in roles.iter().enumerate() {
        let actor_id = role.get("actorId").and_then(Value::as_str);
        if let Some(actor_id) = actor_id {
            if !kc.actor_ids.contains(actor_id) {
                diagnostics.push(LintDiagnostic::t2_error(
                    "SIG-002",
                    &format!("/roles/{index}/actorId"),
                    format!(
                        "signature role actorId '{actor_id}' does not resolve to a kernel actor"
                    ),
                ));
            } else if !kc.human_actor_ids.contains(actor_id) {
                diagnostics.push(LintDiagnostic::t2_error(
                    "SIG-003",
                    &format!("/roles/{index}/actorId"),
                    format!("signature role actorId '{actor_id}' is not a human kernel actor"),
                ));
            }
        }
    }
}

fn check_signature_auth_policies(
    profile: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let policy_keys = string_set_at(profile, "/authenticationPolicies", "key");
    let Some(roles) = profile.value.pointer("/roles").and_then(Value::as_array) else {
        return;
    };
    for (index, role) in roles.iter().enumerate() {
        let Some(policy_key) = role.get("authenticationPolicyKey").and_then(Value::as_str) else {
            continue;
        };
        if !policy_keys.contains(policy_key) {
            diagnostics.push(LintDiagnostic::t2_error(
                "SIG-004",
                &format!("/roles/{index}/authenticationPolicyKey"),
                format!("authenticationPolicyKey '{policy_key}' does not resolve to authenticationPolicies[*].key"),
            ));
        }
    }
}

fn check_signature_steps(
    profile: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let role_ids = string_set_at(profile, "/roles", "id");
    let document_ids = string_set_at(profile, "/documents", "id");
    let Some(steps) = profile
        .value
        .pointer("/signingFlow/steps")
        .and_then(Value::as_array)
    else {
        return;
    };

    let step_ids: HashSet<&str> = steps
        .iter()
        .filter_map(|step| step.get("id").and_then(Value::as_str))
        .collect();

    for (index, step) in steps.iter().enumerate() {
        if let Some(role_id) = step.get("roleId").and_then(Value::as_str)
            && !role_ids.contains(role_id)
        {
            diagnostics.push(LintDiagnostic::t2_error(
                "SIG-005",
                &format!("/signingFlow/steps/{index}/roleId"),
                format!("signing step roleId '{role_id}' does not resolve to roles[*].id"),
            ));
        }
        if let Some(document_id) = step.get("documentId").and_then(Value::as_str)
            && !document_ids.contains(document_id)
        {
            diagnostics.push(LintDiagnostic::t2_error(
                "SIG-006",
                &format!("/signingFlow/steps/{index}/documentId"),
                format!(
                    "signing step documentId '{document_id}' does not resolve to documents[*].id"
                ),
            ));
        }
        if let Some(depends_on) = step.get("dependsOn").and_then(Value::as_array) {
            for (dep_index, dependency) in depends_on.iter().enumerate() {
                let Some(dependency) = dependency.as_str() else {
                    continue;
                };
                if !step_ids.contains(dependency) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "SIG-007",
                        &format!("/signingFlow/steps/{index}/dependsOn/{dep_index}"),
                        format!("signing step dependency '{dependency}' does not resolve to a sibling step id"),
                    ));
                }
            }
        }
    }

    if signature_step_graph_has_cycle(steps) {
        diagnostics.push(LintDiagnostic::t2_error(
            "SIG-007",
            "/signingFlow/steps",
            "signing step dependencies MUST NOT cycle",
        ));
    }
}

fn check_signature_guards(
    profile: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(steps) = profile
        .value
        .pointer("/signingFlow/steps")
        .and_then(Value::as_array)
    else {
        return;
    };
    for (index, step) in steps.iter().enumerate() {
        let Some(guard) = step.get("guard").and_then(Value::as_str) else {
            continue;
        };
        if let Err(error) = fel_core::parse(guard) {
            diagnostics.push(LintDiagnostic::t2_error(
                "SIG-008",
                &format!("/signingFlow/steps/{index}/guard"),
                format!("routed signing guard failed to parse as FEL: {error}"),
            ));
        }
    }
}

fn check_signature_lifecycle_tags(
    profile: &crate::document::WosDocument,
    kc: &KernelCollections,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for tag in [
        "awaiting-signature",
        "signature-complete",
        "signature-declined",
        "signature-expired",
        "signature-voided",
    ] {
        if !kc.tags.contains(tag) {
            diagnostics.push(LintDiagnostic::t2_warning(
                "SIG-009",
                "/signingFlow",
                format!(
                    "Signature Profile lifecycle tag '{tag}' does not appear in the target kernel"
                ),
            ));
        }
    }
    let _ = profile;
}

fn check_signature_timer_events(
    profile: &crate::document::WosDocument,
    kc: &KernelCollections,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for path in ["/reminders/eventName", "/expiryPolicy/eventName"] {
        let Some(event_name) = pointer_str(&profile.value, path) else {
            continue;
        };
        if !kc.events.contains(event_name) {
            diagnostics.push(LintDiagnostic::t2_error(
                "SIG-010",
                path,
                format!("signature timer eventName '{event_name}' does not map to a typed kernel timer/message event"),
            ));
        }
    }
}

fn check_signature_evidence_inputs(
    profile: &crate::document::WosDocument,
    kc: &KernelCollections,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(required_fields) = profile
        .value
        .pointer("/evidence/requiredFields")
        .and_then(Value::as_array)
    else {
        return;
    };
    for (index, field) in required_fields.iter().enumerate() {
        let Some(field) = field.as_str() else {
            continue;
        };
        if field.starts_with("response.") {
            continue;
        }
        if field.starts_with("caseFile.") && kc.case_fields.contains(field) {
            continue;
        }
        diagnostics.push(LintDiagnostic::t2_error(
            "SIG-011",
            &format!("/evidence/requiredFields/{index}"),
            format!("SignatureAffirmation evidence field '{field}' is not satisfiable from response.* or the kernel caseFile"),
        ));
    }
}

fn check_signature_naming(
    profile: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    check_signature_naming_recursive(&profile.value, "", diagnostics);
}

fn check_signature_naming_recursive(
    value: &Value,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}/{}", escape_json_pointer(key));
                if key.ends_with("Ref") && !child.as_str().is_some_and(is_uri_like) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "SIG-012",
                        &child_path,
                        format!("Signature Profile field '{key}' ends with Ref and MUST carry a URI-like cross-artifact reference"),
                    ));
                }
                if key.ends_with("Key") && child.as_str().is_some_and(is_uri_like) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "SIG-012",
                        &child_path,
                        format!("Signature Profile field '{key}' ends with Key and MUST carry a package-local key, not a URI"),
                    ));
                }
                check_signature_naming_recursive(child, &child_path, diagnostics);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                check_signature_naming_recursive(child, &format!("{path}/{index}"), diagnostics);
            }
        }
        _ => {}
    }
}

fn signature_step_graph_has_cycle(steps: &[Value]) -> bool {
    let graph: HashMap<&str, Vec<&str>> = steps
        .iter()
        .filter_map(|step| {
            let id = step.get("id")?.as_str()?;
            let deps = step
                .get("dependsOn")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            Some((id, deps))
        })
        .collect();

    fn visit<'a>(
        id: &'a str,
        graph: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> bool {
        if visited.contains(id) {
            return false;
        }
        if !visiting.insert(id) {
            return true;
        }
        for dependency in graph.get(id).into_iter().flatten() {
            if graph.contains_key(dependency) && visit(dependency, graph, visiting, visited) {
                return true;
            }
        }
        visiting.remove(id);
        visited.insert(id);
        false
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    graph
        .keys()
        .any(|id| visit(id, &graph, &mut visiting, &mut visited))
}

fn string_set_at<'a>(
    doc: &'a crate::document::WosDocument,
    pointer: &str,
    field: &str,
) -> HashSet<&'a str> {
    doc.value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get(field).and_then(Value::as_str))
        .collect()
}

fn pointer_str<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn is_uri_like(value: &str) -> bool {
    value.contains(':')
}

fn escape_json_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// K-010 (typed): Action `assignTo` fields MUST reference a declared kernel actor.
fn check_action_actor_references_typed(
    kernel: &KernelDocument,
    actor_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    check_action_actors_recursive_typed(
        &kernel.lifecycle.states,
        "/lifecycle/states",
        actor_ids,
        diagnostics,
    );
}

fn check_action_actors_recursive_typed(
    states: &indexmap::IndexMap<String, State>,
    parent_path: &str,
    actor_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, state) in states {
        let state_path = format!("{parent_path}/{name}");
        // onEntry
        for (i, action) in state.on_entry.iter().enumerate() {
            if let Some(assign_to) = &action.assign_to {
                if !actor_ids.contains(assign_to.as_str()) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "K-010",
                        &format!("{state_path}/onEntry/{i}/assignTo"),
                        format!(
                            "action assignTo '{assign_to}' does not reference a declared actor"
                        ),
                    ));
                }
            }
        }
        // onExit
        for (i, action) in state.on_exit.iter().enumerate() {
            if let Some(assign_to) = &action.assign_to {
                if !actor_ids.contains(assign_to.as_str()) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "K-010",
                        &format!("{state_path}/onExit/{i}/assignTo"),
                        format!(
                            "action assignTo '{assign_to}' does not reference a declared actor"
                        ),
                    ));
                }
            }
        }
        // Transition actions
        for (ti, transition) in state.transitions.iter().enumerate() {
            for (ai, action) in transition.actions.iter().enumerate() {
                if let Some(assign_to) = &action.assign_to {
                    if !actor_ids.contains(assign_to.as_str()) {
                        diagnostics.push(LintDiagnostic::t2_error(
                            "K-010",
                            &format!("{state_path}/transitions/{ti}/actions/{ai}/assignTo"),
                            format!(
                                "action assignTo '{assign_to}' does not reference a declared actor"
                            ),
                        ));
                    }
                }
            }
        }
        // Recurse into substates
        check_action_actors_recursive_typed(
            &state.states,
            &format!("{state_path}/states"),
            actor_ids,
            diagnostics,
        );
        // Recurse into regions
        for (region_name, region) in &state.regions {
            check_action_actors_recursive_typed(
                &region.states,
                &format!("{state_path}/regions/{region_name}/states"),
                actor_ids,
                diagnostics,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// K-037: Fail-fast $join fires only on error final state
// ---------------------------------------------------------------------------

/// K-037 (typed): Fail-fast parallel regions MUST have an error-tagged final state.
fn check_fail_fast_error_final_states_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    check_fail_fast_recursive_typed(&kernel.lifecycle.states, "/lifecycle/states", diagnostics);
}

fn check_fail_fast_recursive_typed(
    states: &indexmap::IndexMap<String, State>,
    parent_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, state) in states {
        let state_path = format!("{parent_path}/{name}");

        if state.kind == StateKind::Parallel
            && state.cancellation_policy == Some(CancellationPolicy::FailFast)
        {
            for (region_name, region) in &state.regions {
                let has_error_final = region
                    .states
                    .values()
                    .any(|s| s.kind == StateKind::Final && s.tags.iter().any(|t| t == "error"));
                if !has_error_final {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "K-037",
                        &format!("{state_path}/regions/{region_name}"),
                        format!(
                            "fail-fast parallel '{name}' region '{region_name}' has no final state tagged 'error'; fail-fast cannot trigger"
                        ),
                    ));
                }
            }
        }

        // Recurse
        check_fail_fast_recursive_typed(
            &state.states,
            &format!("{state_path}/states"),
            diagnostics,
        );
        for (region_name, region) in &state.regions {
            check_fail_fast_recursive_typed(
                &region.states,
                &format!("{state_path}/regions/{region_name}/states"),
                diagnostics,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// G-001: Due process required for rights/safety-impacting
// ---------------------------------------------------------------------------

/// G-001 (typed): Governance MUST declare `dueProcess` for rights/safety-impacting kernels.
fn check_due_process_for_impact_typed(
    gov: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !is_rights_or_safety_impacting_typed(kernel) {
        return;
    }
    if gov.value.get("dueProcess").is_none() {
        diagnostics.push(LintDiagnostic::t2_error(
            "G-001",
            "/dueProcess",
            "kernel impactLevel is rights/safety-impacting; governance MUST declare a dueProcess section",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-003: Notice content must be individualized for rights-impacting
// ---------------------------------------------------------------------------

/// G-003 (typed): Notice must declare individualized content fields for rights-impacting.
fn check_notice_individualized_for_rights_typed(
    gov: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if kernel.impact_level != Some(ImpactLevel::RightsImpacting) {
        return;
    }
    let Some(notice) = gov.value.pointer("/dueProcess/notice") else {
        return;
    };
    let path = "/dueProcess/notice";
    for field in ["determinationField", "reasonCodes", "appealInstructions"] {
        if notice.get(field).is_none() {
            diagnostics.push(LintDiagnostic::t2_warning(
                "G-003",
                path,
                format!(
                    "rights-impacting notice must declare '{field}' for individualized content"
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-004: Explanation level must be individualized for rights-impacting
// ---------------------------------------------------------------------------

/// G-004 (typed): explanationLevel MUST be 'individualized' for rights-impacting.
fn check_explanation_level_for_rights_typed(
    gov: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if kernel.impact_level != Some(ImpactLevel::RightsImpacting) {
        return;
    }
    let level = gov
        .value
        .pointer("/dueProcess/explanationLevel")
        .and_then(Value::as_str);
    if level != Some("individualized") {
        diagnostics.push(LintDiagnostic::t2_error(
            "G-004",
            "/dueProcess/explanationLevel",
            "rights-impacting kernel requires explanationLevel 'individualized' in governance dueProcess",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-005: Counterfactual required for rights-impacting adverse decisions
// ---------------------------------------------------------------------------

/// G-005 (typed): Counterfactuals required for rights-impacting.
fn check_counterfactual_for_rights_typed(
    gov: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if kernel.impact_level != Some(ImpactLevel::RightsImpacting) {
        return;
    }
    match gov.value.pointer("/dueProcess/counterfactuals") {
        None => diagnostics.push(LintDiagnostic::t2_error(
            "G-005",
            "/dueProcess/counterfactuals",
            "rights-impacting kernel requires counterfactuals section with positive and negative entries",
        )),
        Some(cf) => {
            for polarity in ["positive", "negative"] {
                if cf.get(polarity).is_none() {
                    diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
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
        diagnostics.push(LintDiagnostic::t2_warning(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if gov.value.get("adverseDecisionPolicy").is_none() {
        return;
    }
    if !kernel_tags.contains("adverse-decision") {
        diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
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
                diagnostics.push(LintDiagnostic::t2_warning(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !kernel_tags.contains("determination") {
        return;
    }
    let has_reasoning = gov.value.get("reasoningTier").is_some()
        || gov.value.pointer("/provenanceTiers/reasoning").is_some();
    if !has_reasoning {
        diagnostics.push(LintDiagnostic::t2_error(
            "G-014",
            "/reasoningTier",
            "kernel has determination-tagged transitions; governance MUST declare a reasoning tier",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-015: Counterfactual tier required for adverse-decision in rights-impacting
// ---------------------------------------------------------------------------

/// G-015 (typed): Counterfactual tier required for rights-impacting + adverse-decision.
fn check_counterfactual_tier_for_adverse_typed(
    gov: &crate::document::WosDocument,
    kernel: &KernelDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if kernel.impact_level != Some(ImpactLevel::RightsImpacting) {
        return;
    }
    if !kernel_tags.contains("adverse-decision") {
        return;
    }
    let has_counterfactual = gov.value.get("counterfactualTier").is_some()
        || gov
            .value
            .pointer("/provenanceTiers/counterfactual")
            .is_some();
    if !has_counterfactual {
        diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
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
            diagnostics.push(LintDiagnostic::t2_warning(
                "G-022",
                &format!("/tasks/{task_name}"),
                format!("actor '{actor}' is in both potentialOwner and excludedOwner; excludedOwner takes precedence — verify this is intentional"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-023 / G-060: SLA uses business days when scoped Business Calendar exists
// ---------------------------------------------------------------------------

/// True when a Business Calendar is present for `workflow_url`.
///
/// After ADR 0076, calendars live inside `$wosDelivery.calendar`. We check
/// both Delivery sidecars (canonical) and any Workflow doc that embeds a
/// `governance.policyParameters.calendarRef` or similar reference.
fn business_calendar_targets_workflow(project: &WosProject, workflow_url: &str) -> bool {
    // Check Delivery sidecars first.
    let in_delivery = project.of_kind(DocumentKind::Delivery).any(|d| {
        let target = d
            .value
            .get("targetWorkflow")
            .and_then(Value::as_str)
            .unwrap_or("");
        (target == workflow_url || target.is_empty()) && d.value.get("calendar").is_some()
    });
    if in_delivery {
        return true;
    }
    // Legacy: accept any document in the project that carries the old
    // $wosBusinessCalendar marker shape (targetWorkflow field at root).
    project.documents().iter().any(|d| {
        d.value
            .get("targetWorkflow")
            .and_then(Value::as_str)
            .is_some_and(|t| t == workflow_url)
            && d.value.get("workWeek").is_some()
    })
}

/// Template keys declared in Notification Template blocks for `workflow_url`.
///
/// After ADR 0076, templates live inside `$wosDelivery.notificationTemplates`.
fn notification_template_keys_for_workflow(
    project: &WosProject,
    workflow_url: &str,
) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();

    // Delivery sidecars.
    for d in project.of_kind(DocumentKind::Delivery) {
        let target = d
            .value
            .get("targetWorkflow")
            .and_then(Value::as_str)
            .unwrap_or("");
        if target != workflow_url && !target.is_empty() {
            continue;
        }
        // Templates may live at .notificationTemplates or .templates.
        for field in &["notificationTemplates", "templates"] {
            if let Some(templates) = d.value.get(*field).and_then(Value::as_object) {
                for k in templates.keys() {
                    keys.insert(k.clone());
                }
            }
        }
    }

    // Legacy: accept any document in the project that matches the old
    // $wosNotificationTemplate shape (targetWorkflow + templates).
    for d in project.documents() {
        let matches = d
            .value
            .get("targetWorkflow")
            .and_then(Value::as_str)
            .is_some_and(|t| t == workflow_url)
            && d.value.get("templates").is_some();
        if matches {
            if let Some(templates) = d.value.get("templates").and_then(Value::as_object) {
                for k in templates.keys() {
                    keys.insert(k.clone());
                }
            }
        }
    }

    keys
}

/// Collect `(jsonPath, keyValue)` for notification-template catalog surfaces:
/// `templateKey` (SLA warning/breach), `notificationTemplateKey` (hold
/// policies), and `noticeTemplateKey` (due process notices).
fn collect_governance_template_refs(value: &Value, base: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let p = if base.is_empty() {
                    format!("/{k}")
                } else {
                    format!("{base}/{k}")
                };
                if (k == "notificationTemplateKey"
                    || k == "noticeTemplateKey"
                    || k == "templateKey")
                    && let Some(s) = v.as_str()
                {
                    out.push((p, s.to_string()));
                    continue;
                }
                collect_governance_template_refs(v, &p, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                collect_governance_template_refs(v, &format!("{base}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// G-023: When a Business Calendar targets this workflow, SLA should use `calendarType: business`.
///
/// G-060 (BC S6.1): Same condition is a MUST — emit an error.
fn check_sla_business_calendar(
    gov: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(target_wf) = gov.value.get("targetWorkflow").and_then(Value::as_str) else {
        return;
    };
    if !business_calendar_targets_workflow(project, target_wf) {
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
            let path = format!("/tasks/{task_name}/sla");
            // G-023 (Governance S10): authoring SHOULD; G-060 (BC S6.1): normative MUST — both fire so
            // authors get a warning-level hint plus the error-level obligation.
            diagnostics.push(LintDiagnostic::t2_warning(
                "G-023",
                &path,
                "a business calendar sidecar targets this workflow; SLA should set calendarType to 'business'",
            ));
            diagnostics.push(LintDiagnostic::t2_error(
                "G-060",
                &path,
                "when a Business Calendar sidecar targets this workflow, SLA evaluation MUST use business days (set calendarType to 'business')",
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-063: Notification template references resolve
// ---------------------------------------------------------------------------

/// G-063: `templateKey`, `notificationTemplateKey`, and `noticeTemplateKey`
/// MUST resolve to a template key in a Notification Template sidecar for the
/// same `targetWorkflow`.
fn check_notification_template_refs(
    gov: &crate::document::WosDocument,
    project: &WosProject,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(target_wf) = gov.value.get("targetWorkflow").and_then(Value::as_str) else {
        return;
    };

    let mut refs = Vec::new();
    collect_governance_template_refs(&gov.value, "", &mut refs);
    if refs.is_empty() {
        return;
    }

    let keys = notification_template_keys_for_workflow(project, target_wf);

    if keys.is_empty() {
        for (path, r) in refs {
            diagnostics.push(LintDiagnostic::t2_error(
                "G-063",
                &path,
                format!(
                    "notification template key '{r}' but no Notification Template sidecar targets this workflow"
                ),
            ));
        }
        return;
    }

    for (path, r) in refs {
        if !keys.contains(&r) {
            diagnostics.push(LintDiagnostic::t2_error(
                "G-063",
                &path,
                format!(
                    "notification template key '{r}' does not match any template key in the Notification Template sidecar"
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// G-066: BreachPolicy escalation step ids resolve within task pattern
// ---------------------------------------------------------------------------

/// G-066: `BreachPolicy.escalationStepId` MUST resolve to an
/// `EscalationStep` in the same task pattern by explicit `id` or `level-N`.
fn check_sla_escalation_step_ids(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(task_catalog) = gov.value.get("taskCatalog").and_then(Value::as_array) {
        for (idx, task) in task_catalog.iter().enumerate() {
            check_task_pattern_escalation_step_id(
                task,
                &format!("/taskCatalog/{idx}"),
                diagnostics,
            );
        }
    }

    if let Some(tasks) = gov.value.get("tasks").and_then(Value::as_object) {
        for (task_name, task) in tasks {
            check_task_pattern_escalation_step_id(
                task,
                &format!("/tasks/{task_name}"),
                diagnostics,
            );
        }
    }
}

fn check_task_pattern_escalation_step_id(
    task: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(step_id) = task
        .pointer("/breachPolicy/escalationStepId")
        .and_then(Value::as_str)
    else {
        return;
    };

    let targets = escalation_step_targets(task);
    if targets.contains(step_id) {
        return;
    }

    diagnostics.push(LintDiagnostic::t2_error(
        "G-066",
        &format!("{base_path}/breachPolicy/escalationStepId"),
        format!(
            "BreachPolicy escalationStepId '{step_id}' does not match any escalationChain step id or level token in this task pattern"
        ),
    ));
}

fn escalation_step_targets(task: &Value) -> std::collections::HashSet<String> {
    let mut targets = std::collections::HashSet::new();
    let Some(chain) = task.get("escalationChain").and_then(Value::as_array) else {
        return targets;
    };

    for step in chain {
        if let Some(id) = step.get("id").and_then(Value::as_str) {
            targets.insert(id.to_string());
        }
        if let Some(level) = step.get("level").and_then(Value::as_u64) {
            targets.insert(format!("level-{level}"));
        }
    }

    targets
}

// ---------------------------------------------------------------------------
// G-024: Delegation verification on determination-tagged transitions
// ---------------------------------------------------------------------------

/// G-024: When the kernel has `determination`-tagged transitions, governance
/// SHOULD declare `delegationVerification` or a non-empty `delegations` list.
fn check_delegation_verification_on_determination(
    gov: &crate::document::WosDocument,
    kernel_tags: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
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
        diagnostics.push(LintDiagnostic::t2_warning(
            "G-024",
            "/delegationVerification",
            "kernel has determination-tagged transitions; governance should declare delegationVerification or delegations",
        ));
    }
}

// ---------------------------------------------------------------------------
// G-027: Sub-delegation depth traversal
// ---------------------------------------------------------------------------

/// G-027 (typed): Sub-delegation MUST respect `maxDelegationDepth`.
fn check_sub_delegation_depth_typed(
    gov: &wos_core::GovernanceDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let max_depth = gov.max_delegation_depth as usize;
    if gov.delegations.is_empty() {
        return;
    }

    let links: Vec<(&str, &str)> = gov
        .delegations
        .iter()
        .map(|d| (d.delegate.as_str(), d.delegator.as_str()))
        .collect();

    for (i, delegation) in gov.delegations.iter().enumerate() {
        let depth = delegation_chain_depth(&delegation.delegate, &links, 0);
        if depth > max_depth {
            diagnostics.push(LintDiagnostic::t2_error(
                "G-027",
                &format!("/delegations/{i}"),
                format!(
                    "sub-delegation chain depth {depth} exceeds maxDelegationDepth {max_depth}"
                ),
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

/// G-028 (typed): hold policies require at least one hold-tagged kernel state.
fn check_hold_policies_attach_to_hold_states_typed(
    gov: &wos_core::GovernanceDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if gov.hold_policies.is_empty() {
        return;
    }
    let hold_states = collect_states_with_tag_typed(&kernel.lifecycle.states, "hold");
    if hold_states.is_empty() {
        diagnostics.push(LintDiagnostic::t2_error(
            "G-028",
            "/holdPolicies",
            "hold policies declare tag-based attachment, but the kernel has no state tagged 'hold'",
        ));
    }
}

/// Collect states with a given tag from typed model.
fn collect_states_with_tag_typed(
    states: &indexmap::IndexMap<String, State>,
    tag: &str,
) -> std::collections::HashSet<String> {
    let mut matching = std::collections::HashSet::new();
    collect_states_with_tag_recursive_typed(states, tag, &mut matching);
    matching
}

fn collect_states_with_tag_recursive_typed(
    states: &indexmap::IndexMap<String, State>,
    tag: &str,
    matching: &mut std::collections::HashSet<String>,
) {
    for (name, state) in states {
        if state.tags.iter().any(|t| t == tag) {
            matching.insert(name.clone());
        }
        collect_states_with_tag_recursive_typed(&state.states, tag, matching);
        for region in state.regions.values() {
            collect_states_with_tag_recursive_typed(&region.states, tag, matching);
        }
    }
}

// ---------------------------------------------------------------------------
// G-029: Hold resumeTrigger must correspond to a kernel event
// ---------------------------------------------------------------------------

/// G-029 (typed): `holdPolicy.resumeTrigger` MUST correspond to a kernel event.
fn check_hold_resume_triggers_typed(
    gov: &wos_core::GovernanceDocument,
    kernel_events: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (i, hold) in gov.hold_policies.iter().enumerate() {
        let trigger = hold.resume_trigger.as_str();
        if !kernel_events.contains(trigger) {
            diagnostics.push(LintDiagnostic::t2_warning(
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
/// Governance documents that embed `resolutionDateRef` directly (in
/// `slaConfig` or sibling slots) are validated here. The PolicyParameters
/// embedded-block path is currently un-wired into the dispatcher.
fn check_resolution_date_refs(
    gov: &crate::document::WosDocument,
    kernel_case_fields: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let fields = kernel_case_fields;

    if let Some(sla_config) = gov.value.get("slaConfig") {
        if let Some(date_ref) = sla_config.get("resolutionDateRef").and_then(Value::as_str) {
            if !fields.contains(date_ref) {
                diagnostics.push(LintDiagnostic::t2_warning(
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
fn check_parameter_coverage(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(params) = gov.value.get("parameters").and_then(Value::as_object) else {
        return;
    };
    for (name, param) in params {
        if let Some(values) = param.get("values").and_then(Value::as_array) {
            if values.is_empty() {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "G-033",
                    &format!("/parameters/{name}/values"),
                    format!("parameter '{name}' has no values entries; resolution date may not be covered"),
                ));
            }
        }
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
fn check_independence_constraint(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if gov.value.get("reviewProtocols").is_none() {
        return;
    }
    match gov.value.get("independenceConstraint") {
        None => diagnostics.push(LintDiagnostic::t2_warning(
            "G-036",
            "/independenceConstraint",
            "governance has reviewProtocols but no independenceConstraint; must encode prevention of self-review",
        )),
        Some(c) => {
            let is_empty = c.as_str().map(str::is_empty).unwrap_or(false)
                || c.as_object().is_some_and(|m| m.is_empty())
                || c.as_array().is_some_and(|a| a.is_empty());
            if is_empty {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "G-036",
                    "/independenceConstraint",
                    "independenceConstraint is empty; must encode an actual prevention mechanism",
                ));
            }
        }
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(assertions) = al.value.get("assertions").and_then(Value::as_array) else {
        return;
    };
    // After ADR 0076, pipeline stages live inside $wosWorkflow.governance.pipelines
    // (or .pipeline for the flat shape). Collect from all Workflow docs.
    let pipeline_stages: std::collections::HashSet<String> = project
        .of_kind(DocumentKind::Workflow)
        .flat_map(|g| {
            let from_gov_pipelines = g
                .value
                .pointer("/governance/pipelines")
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            let from_gov_pipeline = g
                .value
                .pointer("/governance/pipeline")
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            from_gov_pipelines.chain(from_gov_pipeline)
        })
        .filter_map(|stage| stage.get("id").and_then(Value::as_str))
        .map(String::from)
        // Also accept pipeline stages from the synthetic governance doc value directly.
        .chain(
            al.value
                .get("pipeline")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|stage| stage.get("id").and_then(Value::as_str))
                .map(String::from),
        )
        .collect();
    for (i, assertion) in assertions.iter().enumerate() {
        if assertion.get("type").and_then(Value::as_str) != Some("consistency") {
            continue;
        }
        let Some(ref_stage) = assertion.get("referenceStage").and_then(Value::as_str) else {
            continue; // G-039 (T1) handles the missing-field case.
        };
        if !pipeline_stages.contains(ref_stage) {
            diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(pipeline) = gov.value.get("pipeline").and_then(Value::as_array) else {
        return;
    };
    // After ADR 0076, assertion libraries live inside $wosWorkflow.governance.assertionLibrary
    // (or .assertions directly on the governance block). Collect from all Workflow docs.
    let library_ids: std::collections::HashSet<String> = project
        .of_kind(DocumentKind::Workflow)
        .flat_map(|wf| {
            let from_al = wf
                .value
                .pointer("/governance/assertionLibrary/assertions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            let from_assertions = wf
                .value
                .pointer("/governance/assertions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            from_al.chain(from_assertions)
        })
        .filter_map(|a| a.get("id").and_then(Value::as_str))
        .map(String::from)
        // Also accept assertion ids from the synthetic governance doc passed in.
        .chain(
            gov.value
                .get("assertions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|a| a.get("id").and_then(Value::as_str))
                .map(String::from),
        )
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
                diagnostics.push(LintDiagnostic::t2_error(
                    "G-041",
                    &format!("/pipeline/{si}/assertions/{ai}"),
                    format!(
                        "assertion id '{id}' not found in any assertion library in the project"
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-046: Delegation actors must exist in kernel
// ---------------------------------------------------------------------------

/// G-046 (typed): delegator and delegate MUST correspond to kernel actors.
fn check_delegation_actors_exist_typed(
    gov: &wos_core::GovernanceDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let kernel_actors: std::collections::HashSet<&str> =
        kernel.actors.iter().map(|a| a.id.as_str()).collect();
    for (i, delegation) in gov.delegations.iter().enumerate() {
        let path = format!("/delegations/{i}");
        for field in ["delegator", "delegate"] {
            let actor = match field {
                "delegator" => delegation.delegator.as_str(),
                "delegate" => delegation.delegate.as_str(),
                _ => unreachable!(),
            };
            if !kernel_actors.contains(actor) {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "G-046",
                    &path,
                    format!("{field} '{actor}' not found in kernel actors"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G-053: Sub-delegation only if original permits
// ---------------------------------------------------------------------------

/// G-053: Sub-delegation MUST only be permitted if the original delegation
/// explicitly sets `allowsSubDelegation: true`.
/// G-053 (Value): Sub-delegation only if original permits.
fn check_sub_delegation_permission_value(
    gov: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(delegations) = gov.value.get("delegations").and_then(Value::as_array) else {
        return;
    };
    let delegates: std::collections::HashSet<&str> = delegations
        .iter()
        .filter_map(|d| d.get("delegate").and_then(Value::as_str))
        .collect();
    for (i, delegation) in delegations.iter().enumerate() {
        let Some(delegator) = delegation.get("delegator").and_then(Value::as_str) else {
            continue;
        };
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
            diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
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
            diagnostics.push(LintDiagnostic::t2_warning(
                "G-056",
                &format!("/bindings/{binding_name}/resolutionDateRef"),
                format!("resolutionDateRef '{date_ref}' not found in kernel caseFile.fields"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helper: iterate agents from both array and object formats
// ---------------------------------------------------------------------------

/// Extract `(id, &Value)` pairs from an AI Integration document's `agents` field.
///
/// The AI Integration schema defines agents as `"type": "array"` with each
/// element having an `"id"` field. However, some documents may use the legacy
/// object format (`"agents": {"agentId": {...}}`). This helper normalises both
/// representations so that all AI rules work with either format.
fn iter_agents(ai: &crate::document::WosDocument) -> Vec<(String, &serde_json::Value)> {
    let Some(agents) = ai.value.get("agents") else {
        return Vec::new();
    };
    if let Some(arr) = agents.as_array() {
        arr.iter()
            .filter_map(|agent| {
                agent
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(|id| (id.to_string(), agent))
            })
            .collect()
    } else if let Some(obj) = agents.as_object() {
        obj.iter().map(|(k, v)| (k.clone(), v)).collect()
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// AI-007: cascadingInvocations declared for autonomous-invoking-autonomous
// ---------------------------------------------------------------------------

/// AI-007: When any autonomous agent invokes another autonomous agent,
/// `cascadingInvocations` MUST be declared in the AI document.
fn check_cascading_invocations_declared(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let agents = iter_agents(ai);
    if agents.is_empty() {
        return;
    }
    let autonomous: std::collections::HashSet<&str> = agents
        .iter()
        .filter(|(_, a)| a.get("autonomy").and_then(Value::as_str) == Some("autonomous"))
        .map(|(name, _)| name.as_str())
        .collect();
    if autonomous.len() < 2 {
        return;
    }
    let cascades = agents.iter().any(|(_, agent)| {
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
        diagnostics.push(LintDiagnostic::t2_error(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        if agent.get("autonomy").and_then(Value::as_str) != Some("autonomous") {
            continue;
        }
        let has_deontic = agent.get("deonticConstraints").is_some()
            || agent.get("permissions").is_some()
            || agent.get("prohibitions").is_some()
            || agent.get("obligations").is_some();
        if !has_deontic {
            diagnostics.push(LintDiagnostic::t2_warning(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        if agent.get("autonomy").and_then(Value::as_str) != Some("supervisory") {
            continue;
        }
        if agent.get("reviewWindow").is_none() {
            diagnostics.push(LintDiagnostic::t2_warning(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        let Some(rules) = agent.get("escalationRules").and_then(Value::as_array) else {
            continue;
        };
        for (i, rule) in rules.iter().enumerate() {
            if rule.get("escalationExpiry").is_none() {
                diagnostics.push(LintDiagnostic::t2_warning(
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(kernel_form_url) = kernel.value.get("formUrl").and_then(Value::as_str) else {
        return; // No human-facing form declared; nothing to compare.
    };
    for (name, agent) in iter_agents(ai) {
        let Some(contract) = agent.get("outputContract") else {
            continue;
        };
        let contract_form = contract.get("formUrl").and_then(Value::as_str);
        if contract_form != Some(kernel_form_url) {
            diagnostics.push(LintDiagnostic::t2_warning(
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
fn check_training_data_disclosure(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        let has_disclosure = agent
            .get("modelConfig")
            .and_then(|c| c.get("trainingDataCharacteristics"))
            .is_some();
        if !has_disclosure {
            diagnostics.push(LintDiagnostic::t2_warning(
                "AI-042",
                &format!("/agents/{name}/modelConfig/trainingDataCharacteristics"),
                format!(
                    "agent '{name}' should disclose training data characteristics in modelConfig"
                ),
            ));
        }
    }
}

/// AI-043: Agent config MUST disclose optimization objective.
fn check_optimization_objective_disclosure(
    ai: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        let has_objective = agent
            .get("modelConfig")
            .and_then(|c| c.get("optimizationObjective"))
            .is_some();
        if !has_objective {
            diagnostics.push(LintDiagnostic::t2_warning(
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

/// AI-046 (typed): Rights-impacting requires discloseThatAgentAssisted.
fn check_ai_disclosure_for_impact_typed(
    ai: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !is_rights_impacting_typed(kernel) {
        return;
    }
    let disclosed = ai
        .value
        .get("agentDisclosure")
        .and_then(|d| d.get("discloseThatAgentAssisted"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !disclosed {
        diagnostics.push(LintDiagnostic::t2_error(
            "AI-046",
            "/agentDisclosure/discloseThatAgentAssisted",
            "rights-impacting workflow requires discloseThatAgentAssisted: true",
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (name, agent) in iter_agents(ai) {
        if agent.get("autonomy").is_none() {
            continue;
        }
        let has_action_sites = agent.get("actionSites").is_some()
            || agent
                .get("actions")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|action| action.get("autonomy").is_some()));
        if !has_action_sites {
            diagnostics.push(LintDiagnostic::t2_warning(
                "AI-056",
                &format!("/agents/{name}/autonomy"),
                format!("agent '{name}' sets autonomy at agent level; autonomy should be declared per action site"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// AI-023: Every agent invocation MUST have a reachable agent-free completion path
// ---------------------------------------------------------------------------

/// AI-023: There MUST be at least one path from the initial state to a final
/// state that does not pass through any agent-only state.
///
/// **This is a conservative global approximation, not a per-invocation check.**
/// The spec (AI S5.3 constraint 6) requires that every agent invocation has a
/// reachable path to workflow completion without requiring any agent to succeed.
/// A true per-invocation check would verify that from each agent-assigned state,
/// an alternative non-agent path exists (e.g., via fallback transitions or
/// parallel paths). This implementation checks a weaker property: whether ANY
/// agent-free path from the initial state to a final state exists at all. If
/// this global check fails, the per-invocation property certainly fails too —
/// no agent-free path exists AT ALL. If the global check passes, there may
/// still be individual agent states without fallback paths.
fn check_agent_free_completion_path(
    ai: &crate::document::WosDocument,
    kernel: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    // Collect agent IDs from the AI document. Support both array and object formats.
    let agent_ids: std::collections::HashSet<String> = collect_ai_agent_ids(ai);
    if agent_ids.is_empty() {
        return;
    }

    let Some(states) = kernel
        .value
        .pointer("/lifecycle/states")
        .and_then(Value::as_object)
    else {
        return;
    };

    // Build a flat map of state_name -> (is_final, is_agent_only, transition_targets).
    let graph = build_lifecycle_graph(states, &agent_ids);

    // Find all final states.
    let final_states: std::collections::HashSet<&str> = graph
        .iter()
        .filter(|(_, info)| info.is_final)
        .map(|(name, _)| name.as_str())
        .collect();

    if final_states.is_empty() {
        return;
    }

    // Check: from the initial state, can we reach a final state without going
    // through any agent-only state?
    let initial = kernel
        .value
        .pointer("/lifecycle/initialState")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if initial.is_empty() {
        return;
    }

    // BFS: only traverse through states that are NOT agent-only.
    let reachable = bfs_reachable_non_agent(initial, &graph);

    // If no final state is reachable without agents, emit an error.
    let can_complete = reachable.iter().any(|s| final_states.contains(s.as_str()));
    if !can_complete {
        diagnostics.push(LintDiagnostic::t2_error(
            "AI-023",
            "/lifecycle",
            "no agent-free path from the initial state to a final state exists; \
             every agent invocation MUST have a reachable completion path that \
             does not require any agent to succeed"
                .to_string(),
        ));
    }
}

/// State metadata for the lifecycle graph used by AI-023.
struct StateInfo {
    /// Whether this state is a final state (type = "final").
    is_final: bool,
    /// Whether every createTask in onEntry assigns to an agent.
    is_agent_only: bool,
    /// Target state names reachable via outgoing transitions.
    targets: Vec<String>,
}

/// Build a flat lifecycle graph from the kernel states map.
///
/// Recursively walks compound states and parallel regions to build a
/// flat name->info map. For compound states, outgoing transitions from
/// the parent are included for all substates. Parallel regions contribute
/// their states as flat entries with parent-path-prefixed names so that
/// identically named substates in different regions do not collide.
fn build_lifecycle_graph(
    states: &serde_json::Map<String, Value>,
    agent_ids: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, StateInfo> {
    let mut graph = std::collections::HashMap::new();
    collect_states_into_graph(states, agent_ids, &[], "", &mut graph);
    graph
}

/// Recursively collect states into a flat graph with path-prefixed names.
///
/// `parent_prefix` disambiguates identically named substates in compound
/// states and parallel regions.  Top-level states have an empty prefix;
/// compound substates get `"{compound_name}."`, and parallel region
/// substates get `"{parallel_name}.{region_name}."`.
fn collect_states_into_graph(
    states: &serde_json::Map<String, Value>,
    agent_ids: &std::collections::HashSet<String>,
    parent_targets: &[String],
    parent_prefix: &str,
    graph: &mut std::collections::HashMap<String, StateInfo>,
) {
    for (name, state) in states {
        let prefixed_name = format!("{parent_prefix}{name}");
        let state_type = state
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("atomic");
        let is_final = state_type == "final";

        // Determine if this state is agent-only: every createTask in onEntry assigns to an agent.
        let is_agent_only = is_state_agent_only(state, agent_ids);

        // Collect outgoing transition targets, applying the parent prefix so
        // that targets referencing sibling states within the same compound or
        // region are correctly resolved to their prefixed names.
        let mut targets: Vec<String> = state
            .get("transitions")
            .and_then(Value::as_array)
            .map(|transitions| {
                transitions
                    .iter()
                    .filter_map(|t| {
                        t.get("target")
                            .and_then(Value::as_str)
                            .map(|tgt| format!("{parent_prefix}{tgt}"))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Include parent targets (compound states can be exited via parent transitions).
        targets.extend(parent_targets.iter().cloned());

        graph.insert(
            prefixed_name.clone(),
            StateInfo {
                is_final,
                is_agent_only,
                targets: targets.clone(),
            },
        );

        // Recurse into compound substates.
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            let compound_prefix = format!("{prefixed_name}.");
            collect_states_into_graph(substates, agent_ids, &targets, &compound_prefix, graph);
        }

        // Recurse into parallel regions.
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for (region_name, region) in regions {
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    let region_prefix = format!("{prefixed_name}.{region_name}.");
                    collect_states_into_graph(rstates, agent_ids, &targets, &region_prefix, graph);
                }
            }
        }
    }
}

/// Returns true if every `createTask` in the state's `onEntry` assigns to an agent.
///
/// A state with no `createTask` actions is NOT agent-only (it may be a
/// system/automatic state). A state with a mix of agent and human tasks is
/// NOT agent-only (the human task provides the fallback path).
fn is_state_agent_only(state: &Value, agent_ids: &std::collections::HashSet<String>) -> bool {
    let Some(on_entry) = state.get("onEntry").and_then(Value::as_array) else {
        return false;
    };

    let create_tasks: Vec<&Value> = on_entry
        .iter()
        .filter(|a| a.get("action").and_then(Value::as_str) == Some("createTask"))
        .collect();

    if create_tasks.is_empty() {
        return false;
    }

    create_tasks.iter().all(|task| {
        task.get("assignTo")
            .and_then(Value::as_str)
            .is_some_and(|actor| agent_ids.contains(actor))
    })
}

/// BFS from `start`, only traversing through states that are NOT agent-only.
///
/// Returns all reachable state names. Agent-only states are dead ends in this
/// traversal (their targets are not explored), but the agent-only states
/// themselves ARE included in the reachable set if they can be reached.
fn bfs_reachable_non_agent(
    start: &str,
    graph: &std::collections::HashMap<String, StateInfo>,
) -> std::collections::HashSet<String> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();

    visited.insert(start.to_string());
    queue.push_back(start.to_string());

    while let Some(current) = queue.pop_front() {
        let Some(info) = graph.get(&current) else {
            continue;
        };

        // If this state is agent-only, we can reach it but cannot proceed through it.
        // A final state that happens to be agent-only is still reachable.
        if info.is_agent_only && !info.is_final {
            continue;
        }

        for target in &info.targets {
            if visited.insert(target.clone()) {
                queue.push_back(target.clone());
            }
        }
    }

    visited
}

/// Collect agent IDs from an AI Integration document.
///
/// Delegates to `iter_agents` which already handles both the array and
/// object agent formats, eliminating duplicated parsing logic.
fn collect_ai_agent_ids(ai: &crate::document::WosDocument) -> std::collections::HashSet<String> {
    iter_agents(ai).into_iter().map(|(id, _)| id).collect()
}

// ---------------------------------------------------------------------------
// AG-008: Side-effect tools at autonomous need sideEffectPolicy
// ---------------------------------------------------------------------------

/// AG-008: Side-effect tools at `autonomous` autonomy level MUST have a
/// `sideEffectPolicy`.
fn check_side_effect_tools_policy(
    adv: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
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
        if is_side_effect
            && autonomy == Some("autonomous")
            && tool.get("sideEffectPolicy").is_none()
        {
            diagnostics.push(LintDiagnostic::t2_warning(
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

/// AG-017 (typed): Shadow mode recommended for rights-impacting.
fn check_shadow_mode_recommended_typed(
    adv: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !is_rights_impacting_typed(kernel) {
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
        diagnostics.push(LintDiagnostic::t2_warning(
            "AG-017",
            "/shadowMode",
            "rights-impacting workflow: shadow mode is recommended before granting operational authority",
        ));
    }
}

// ---------------------------------------------------------------------------
// DM-002: Rights/safety workflows should follow deployment sequence
// ---------------------------------------------------------------------------

/// DM-002 (typed): Deployment sequence for rights/safety workflows.
fn check_deployment_sequence_typed(
    dm: &crate::document::WosDocument,
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if !is_rights_or_safety_impacting_typed(kernel) {
        return;
    }
    check_deployment_sequence_impl(dm, diagnostics);
}

/// DM-002 shared implementation (operates on the drift monitor document).
fn check_deployment_sequence_impl(
    dm: &crate::document::WosDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let stages: Vec<&str> = dm
        .value
        .get("deploymentSequence")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    for phase in ["shadow", "canary", "production"] {
        if !stages.contains(&phase) {
            diagnostics.push(LintDiagnostic::t2_warning(
                "DM-002",
                "/deploymentSequence",
                format!("rights/safety-impacting workflow: deployment sequence should include '{phase}' phase"),
            ));
        }
    }

    let phase_order = [("shadow", "canary"), ("canary", "production")];
    for (earlier, later) in phase_order {
        let earlier_pos = stages.iter().position(|&s| s == earlier);
        let later_pos = stages.iter().position(|&s| s == later);
        if let (Some(ep), Some(lp)) = (earlier_pos, later_pos) {
            if ep > lp {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "DM-002",
                    "/deploymentSequence",
                    format!(
                        "'{earlier}' phase should precede '{later}' phase in deployment sequence"
                    ),
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
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(results) = vr.value.get("results").and_then(Value::as_array) else {
        return;
    };
    for (i, result) in results.iter().enumerate() {
        if result.get("result").and_then(Value::as_str) == Some("proven-unsafe")
            && result.get("counterexample").is_none()
        {
            diagnostics.push(LintDiagnostic::t2_error(
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

/// Typed: check if kernel impact level is rights or safety impacting.
fn is_rights_or_safety_impacting_typed(kernel: &KernelDocument) -> bool {
    kernel
        .impact_level
        .is_some_and(|il| il.requires_due_process())
}

/// Typed: check if kernel impact level is rights-impacting.
fn is_rights_impacting_typed(kernel: &KernelDocument) -> bool {
    kernel.impact_level == Some(ImpactLevel::RightsImpacting)
}

// ---------------------------------------------------------------------------
// K-EXT-002: Reserved `x-wos-*` namespace
// ---------------------------------------------------------------------------

/// K-EXT-002: Warn when any key uses the reserved `x-wos-*` namespace.
///
/// Per Kernel §10.6, the prefix `x-wos-` is RESERVED for future normative use
/// by the WOS specification. Implementations and vendors MUST NOT author keys
/// beginning with `x-wos-` until a future spec version publishes them under
/// that namespace.
///
/// The check is case-sensitive (lowercase per §10.6) and requires a non-empty
/// suffix. The bare prefix `x-wos-` (no suffix) is malformed but not a
/// reserved-namespace usage, so it is ignored here. Other vendor prefixes
/// like `x-acme-` or `x-vendor-` are unaffected.
fn check_reserved_wos_namespace(value: &Value, path: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(obj) = value.as_object() {
        for (key, child) in obj {
            let child_path = format!("{path}/{}", json_pointer_escape(key));
            if is_reserved_wos_key(key) {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "K-EXT-002",
                    &child_path,
                    format!(
                        "key '{key}' uses reserved namespace 'x-wos-*'; reserved for future normative use per Kernel §10.6"
                    ),
                ));
            }
            check_reserved_wos_namespace(child, &child_path, diagnostics);
        }
    } else if let Some(arr) = value.as_array() {
        for (i, child) in arr.iter().enumerate() {
            let child_path = format!("{path}/{i}");
            check_reserved_wos_namespace(child, &child_path, diagnostics);
        }
    }
}

/// Test whether a key falls inside the reserved `x-wos-*` namespace.
///
/// Lowercase-only (per §10.6) and requires a non-empty suffix after the
/// `x-wos-` prefix.
fn is_reserved_wos_key(key: &str) -> bool {
    key.strip_prefix("x-wos-")
        .is_some_and(|suffix| !suffix.is_empty())
}

/// Escape a JSON object key for inclusion in a JSON Pointer (RFC 6901).
///
/// `~` becomes `~0` and `/` becomes `~1`.
fn json_pointer_escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(value: Value) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        check_reserved_wos_namespace(&value, "", &mut diags);
        diags
    }

    #[test]
    fn k_ext_002_root_level_x_wos_key_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "x-wos-future": true
        });
        let diags = run(doc);
        let matches: Vec<_> = diags.iter().filter(|d| d.rule_id == "K-EXT-002").collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one K-EXT-002: {diags:?}"
        );
        assert_eq!(matches[0].severity, crate::LintSeverity::Warning);
        assert_eq!(matches[0].path, "/x-wos-future");
        assert!(matches[0].message.contains("x-wos-future"));
        assert!(matches[0].message.contains("§10.6"));
    }

    #[test]
    fn k_ext_002_nested_x_wos_key_has_correct_path() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "lifecycle": {
                "states": {
                    "approved": {
                        "x-wos-experimental": { "enabled": true }
                    }
                }
            }
        });
        let diags = run(doc);
        let matches: Vec<_> = diags.iter().filter(|d| d.rule_id == "K-EXT-002").collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one K-EXT-002: {diags:?}"
        );
        assert_eq!(
            matches[0].path,
            "/lifecycle/states/approved/x-wos-experimental"
        );
    }

    #[test]
    fn k_ext_002_other_vendor_prefix_not_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "x-vendor-foo": "hello",
            "x-acme-bar": { "nested": true }
        });
        let diags = run(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "K-EXT-002"),
            "unexpected K-EXT-002: {diags:?}"
        );
    }

    #[test]
    fn k_ext_002_x_prefix_inside_extensions_not_flagged() {
        // K-030 / K-EXT-001 territory: vendor keys inside `extensions` are fine
        // as long as they don't use the reserved `x-wos-` namespace.
        let doc = json!({
            "$wosWorkflow": "1.0",
            "extensions": {
                "x-acme-foo": "value",
                "x-vendor-config": { "k": 1 }
            }
        });
        let diags = run(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "K-EXT-002"),
            "unexpected K-EXT-002: {diags:?}"
        );
    }

    #[test]
    fn k_ext_002_reserved_namespace_inside_extensions_still_flagged() {
        // §10.6 reserves `x-wos-*` regardless of location. A vendor can't
        // smuggle `x-wos-*` through the `extensions` container.
        let doc = json!({
            "$wosWorkflow": "1.0",
            "extensions": {
                "x-acme-foo": "allowed",
                "x-wos-future": "RESERVED"
            }
        });
        let diags = run(doc);
        let hits: Vec<&_> = diags.iter().filter(|d| d.rule_id == "K-EXT-002").collect();
        assert_eq!(
            hits.len(),
            1,
            "expected exactly 1 K-EXT-002, got: {diags:?}"
        );
        assert_eq!(hits[0].path, "/extensions/x-wos-future");
    }

    #[test]
    fn k_ext_002_bare_prefix_and_uppercase_not_flagged() {
        // `x-wos-` (empty suffix) is malformed but not reserved-use.
        // `X-WOS-*` is uppercase; §10.6 specifies lowercase.
        let doc = json!({
            "$wosWorkflow": "1.0",
            "x-wos-": "empty suffix",
            "X-WOS-future": "uppercase",
            "X-Wos-Mixed": "mixed case"
        });
        let diags = run(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "K-EXT-002"),
            "unexpected K-EXT-002 for non-matching keys: {diags:?}"
        );
    }

    #[test]
    fn k_ext_002_inside_array_elements_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [
                { "id": "alice", "x-wos-trait": "experimental" }
            ]
        });
        let diags = run(doc);
        let matches: Vec<_> = diags.iter().filter(|d| d.rule_id == "K-EXT-002").collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one K-EXT-002: {diags:?}"
        );
        assert_eq!(matches[0].path, "/actors/0/x-wos-trait");
    }

    #[test]
    fn json_pointer_escape_handles_reserved_chars() {
        assert_eq!(json_pointer_escape("plain"), "plain");
        assert_eq!(json_pointer_escape("a/b"), "a~1b");
        assert_eq!(json_pointer_escape("a~b"), "a~0b");
        // Order matters: ~ must be escaped before /.
        assert_eq!(json_pointer_escape("a~/b"), "a~0~1b");
    }

    // ----- WOS-AGENT-XREF-001 ----------------------------------------------

    #[test]
    fn wos_agent_xref_001_actor_without_matching_agent_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [
                {"id": "extractor", "type": "agent"},
                {"id": "reviewer", "type": "human"}
            ],
            "agents": []
        });
        let mut diags = Vec::new();
        check_agent_xref(&doc, &mut diags);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-AGENT-XREF-001")
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one diagnostic: {diags:?}"
        );
        assert_eq!(matches[0].path, "/actors/0/id");
        assert!(matches[0].message.contains("extractor"));
    }

    #[test]
    fn wos_agent_xref_001_matched_agent_clean() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [
                {"id": "extractor", "type": "agent"}
            ],
            "agents": [
                {"id": "extractor", "modelIdentifier": "claude-3"}
            ]
        });
        let mut diags = Vec::new();
        check_agent_xref(&doc, &mut diags);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-AGENT-XREF-001"),
            "expected no WOS-AGENT-XREF-001 diagnostic: {diags:?}"
        );
    }

    // ----- WOS-SIG-COVER-001 -----------------------------------------------

    #[test]
    fn wos_sig_cover_001_signature_transition_without_signature_block_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [{"id": "signer", "type": "human"}],
            "lifecycle": {
                "initial": "draft",
                "states": {
                    "draft": {
                        "type": "atomic",
                        "transitions": [
                            {"to": "signed", "on": {"kind": "signature"}, "actor": "signer"}
                        ]
                    },
                    "signed": {"type": "final"}
                }
            }
        });
        let mut diags = Vec::new();
        check_signature_coverage(&doc, &mut diags);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-SIG-COVER-001")
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one diagnostic: {diags:?}"
        );
        assert!(matches[0].message.contains("no signature block"));
    }

    #[test]
    fn wos_sig_cover_001_covered_signer_clean() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [{"id": "signer", "type": "human"}],
            "lifecycle": {
                "initial": "draft",
                "states": {
                    "draft": {
                        "type": "atomic",
                        "transitions": [
                            {"to": "signed", "on": {"kind": "signature"}, "actor": "signer"}
                        ]
                    },
                    "signed": {"type": "final"}
                }
            },
            "signature": {
                "signers": [{"actorId": "signer"}],
                "order": "sequential"
            }
        });
        let mut diags = Vec::new();
        check_signature_coverage(&doc, &mut diags);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-SIG-COVER-001"),
            "expected no WOS-SIG-COVER-001 diagnostic: {diags:?}"
        );
    }

    #[test]
    fn wos_sig_cover_001_signers_missing_actor_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "actors": [
                {"id": "signer-a", "type": "human"},
                {"id": "signer-b", "type": "human"}
            ],
            "lifecycle": {
                "initial": "draft",
                "states": {
                    "draft": {
                        "type": "atomic",
                        "transitions": [
                            {"to": "signed", "on": {"kind": "signature"}, "actor": "signer-b"}
                        ]
                    },
                    "signed": {"type": "final"}
                }
            },
            "signature": {
                "signers": [{"actorId": "signer-a"}],
                "order": "sequential"
            }
        });
        let mut diags = Vec::new();
        check_signature_coverage(&doc, &mut diags);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-SIG-COVER-001")
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one diagnostic: {diags:?}"
        );
        assert!(matches[0].message.contains("signer-b"));
    }
}
