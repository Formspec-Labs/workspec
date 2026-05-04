// Rust guideline compliant 2026-02-21

//! Tier 1 lint rules — single-document structural checks.
//!
//! These rules examine one WOS document in isolation. They require no
//! cross-document resolution, no FEL parsing, and no runtime execution.
//! See LINT-MATRIX.md for the complete rule catalog.
//!
//! Where typed models exist (Kernel, Governance, AI Integration), checks
//! operate on deserialized structs from `wos-core`.

use serde_json::Value;

use wos_core::model::governance::GovernanceDocument;
use wos_core::model::kernel::{ActionKind, KernelDocument, State, StateKind, TransitionEvent};

use crate::diagnostic::LintDiagnostic;
use crate::document::{DocumentKind, WosDocument};

/// Run all Tier 1 checks applicable to the document's kind.
pub fn check(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    match doc.kind {
        // $wosWorkflow carries lifecycle, governance, agents, signature, bindings,
        // custody, advanced, assurance in one envelope (ADR 0076).
        DocumentKind::Workflow => check_workflow(doc, diagnostics),
        // $wosDelivery carries calendar, notification templates, correspondence.
        DocumentKind::Delivery => {
            check_delivery(doc, diagnostics);
            check_sidecar_target_workflow(doc, diagnostics);
        }
        // $wosOntologyAlignment is a sidecar; only the target-workflow rule applies for now.
        DocumentKind::OntologyAlignment => {
            check_sidecar_target_workflow(doc, diagnostics);
        }
        // Other canonical markers have no Tier 1 rules yet.
        DocumentKind::CaseInstance | DocumentKind::ProvenanceLog | DocumentKind::Tooling => {}
    }
}

// ---------------------------------------------------------------------------
// Kernel rules
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// $wosWorkflow checks — merged envelope (ADR 0076)
// ---------------------------------------------------------------------------

/// Run all Tier 1 checks on a `$wosWorkflow` document.
///
/// The workflow envelope carries the former kernel lifecycle, plus optional
/// embedded blocks: `governance`, `agents`, `aiOversight`, `signature`,
/// `custody`, `advanced`, `assurance`, and `bindings`. Each block is checked
/// via the appropriate sub-checker.
fn check_workflow(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let root = &doc.value;

    // --- Lifecycle / kernel-surface checks ---
    if let Ok(kernel) = serde_json::from_value::<KernelDocument>(root.clone()) {
        let all_state_ids = collect_all_state_ids_typed(&kernel.lifecycle.states);
        for (name, state) in &kernel.lifecycle.states {
            let path = format!("/lifecycle/states/{name}");
            check_state_type_semantics_typed(state, &path, &all_state_ids, diagnostics);
        }

        check_initial_state_keys_into_states_typed(&kernel, diagnostics); // K-016
        check_set_data_paths_typed(&kernel, diagnostics);
        check_milestone_uniqueness_typed(&kernel, diagnostics);
        check_timer_exclusivity_typed(&kernel, diagnostics);
        check_case_relationship_type_prefix_typed(&kernel, diagnostics);
        check_actor_id_uniqueness_typed(&kernel, diagnostics);
        check_provenance_actor_ids_typed(&kernel, diagnostics);
    }

    check_digest_algorithm(root, diagnostics);
    check_extension_prefixes(root, "", diagnostics);
    check_ver_level_for_fallback_chain(root, diagnostics);

    // --- governance embedded block ---
    if let Some(gov_block) = root.get("governance") {
        check_governance_block(gov_block, diagnostics);
    }

    // --- agents / aiOversight embedded blocks ---
    check_ai_integration_block(root, diagnostics);

    // --- bindings embedded block (integration-profile content) ---
    if let Some(bindings) = root.get("bindings") {
        check_bindings_block(bindings, "/bindings", diagnostics);
    }

    // --- ADR 0063: embedded-vs-sidecar identity boundary ---
    check_embedded_no_target_workflow(root, diagnostics);
    check_embedded_no_independent_identity(root, diagnostics);
}

/// Tier 1 checks on the `governance:` embedded block of a `$wosWorkflow` document.
///
/// Covers rules previously applied to standalone `$wosWorkflowGovernance`,
/// `$wosPolicyParameters`, `$wosAssertionLibrary`, and `$wosDueProcess` documents.
fn check_governance_block(gov: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    // --- Hold policies (G-055) ---
    if let Some(holds) = gov
        .get("holds")
        .and_then(|h| h.get("policies"))
        .and_then(Value::as_array)
        .or_else(|| gov.get("holdPolicies").and_then(Value::as_array))
    {
        for (i, policy) in holds.iter().enumerate() {
            let path = format!("/governance/holdPolicies/{i}");
            check_hold_expected_duration_raw(policy, &path, diagnostics);
        }
    }

    // --- Delegation dates (G-044, G-045) ---
    let gov_doc_value = serde_json::json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": "",
        "delegations": gov.get("delegation").and_then(|d| d.get("delegations")).cloned().unwrap_or(serde_json::Value::Null),
        "holdPolicies": gov.get("holds").and_then(|h| h.get("policies")).cloned().unwrap_or(serde_json::Value::Null),
    });
    if let Ok(typed_gov) = serde_json::from_value::<GovernanceDocument>(gov_doc_value) {
        check_delegation_dates_typed(&typed_gov, diagnostics);
    }

    // Also check directly at governance.delegations (flat shape)
    if let Some(delegations) = gov.get("delegations").and_then(Value::as_array) {
        let gov_flat = serde_json::json!({
            "$wosWorkflow": "1.0",
            "targetWorkflow": "",
            "delegations": delegations,
        });
        if let Ok(typed_gov) = serde_json::from_value::<GovernanceDocument>(gov_flat) {
            check_delegation_dates_typed(&typed_gov, diagnostics);
        }
    }

    // --- Policy parameters block (G-047, G-048, G-050, G-057) ---
    let policy_params_root = if let Some(pp) = gov.get("policyParameters") {
        Some(pp.clone())
    } else if gov.get("parameters").is_some() || gov.get("bindings").is_some() {
        Some(gov.clone())
    } else {
        None
    };
    if let Some(pp) = policy_params_root {
        check_policy_parameters_value(&pp, "/governance/policyParameters", diagnostics);
    }

    // --- Assertion library block (G-037, G-038, G-039) ---
    let assertions_root = if let Some(al) = gov.get("assertionLibrary") {
        Some(al.clone())
    } else if gov.get("assertions").is_some() {
        Some(gov.clone())
    } else {
        None
    };
    if let Some(al) = assertions_root {
        check_assertion_library_value(&al, "/governance/assertionLibrary", diagnostics);
    }
}

/// Tier 1 checks on the `agents` / `aiOversight` embedded blocks of a `$wosWorkflow`.
///
/// Covers rules previously applied to standalone `$wosAIIntegration` documents.
fn check_ai_integration_block(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    // AI-049: narrativeProvenance authoritative
    if let Some(np) = root.get("narrativeProvenance") {
        check_narrative_tier_authoritative_in(np, "/narrativeProvenance", diagnostics);
    }
    if let Some(ai_oversight) = root.get("aiOversight") {
        if let Some(np) = ai_oversight.get("narrativeProvenance") {
            check_narrative_tier_authoritative_in(np, "/aiOversight/narrativeProvenance", diagnostics);
        }
    }

    // AI-041: fallback chain termination
    if let Some(agents) = root.get("agents").and_then(Value::as_array) {
        for (i, agent) in agents.iter().enumerate() {
            if let Some(chain) = agent.get("fallbackChain").and_then(Value::as_array) {
                let path = format!("/agents/{i}/fallbackChain");
                check_fallback_chain_termination_raw(chain, &path, diagnostics);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WOS-EMBED-TARGET-001 / WOS-EMBED-IDENTITY-001 (ADR 0063 §2.1):
// Embedded blocks MUST NOT carry targetWorkflow, url, or version. The merged
// $wosWorkflow envelope is the sole author-time identity boundary.
// ---------------------------------------------------------------------------

const EMBEDDED_BLOCKS: &[&str] = &[
    "governance",
    "aiOversight",
    "signature",
    "custody",
    "advanced",
    "assurance",
];

fn check_embedded_no_target_workflow(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    for block in EMBEDDED_BLOCKS {
        if let Some(value) = root.get(*block) {
            if value.get("targetWorkflow").is_some() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "WOS-EMBED-TARGET-001",
                    format!("/{block}/targetWorkflow"),
                    format!(
                        "embedded `{block}` block declares `targetWorkflow`; embedded blocks govern \
                         the enclosing $wosWorkflow envelope and MUST NOT target other workflows \
                         (ADR 0063 §2.1). Remove `targetWorkflow` from the embedded block; if the \
                         block must point at a different workflow, extract it as a sidecar."
                    ),
                ));
            }
        }
    }

    // agents[] is an array of declarations; each entry is treated as part of the
    // enclosing workflow and MUST NOT carry its own targetWorkflow.
    if let Some(agents) = root.get("agents").and_then(Value::as_array) {
        for (i, agent) in agents.iter().enumerate() {
            if agent.get("targetWorkflow").is_some() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "WOS-EMBED-TARGET-001",
                    format!("/agents/{i}/targetWorkflow"),
                    "agents[] entry declares `targetWorkflow`; agent declarations are embedded in \
                     the enclosing $wosWorkflow envelope and MUST NOT target other workflows \
                     (ADR 0063 §2.1)."
                        .to_string(),
                ));
            }
        }
    }
}

fn check_embedded_no_independent_identity(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    for block in EMBEDDED_BLOCKS {
        let Some(value) = root.get(*block) else {
            continue;
        };
        for key in &["url", "version"] {
            if value.get(*key).is_some() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "WOS-EMBED-IDENTITY-001",
                    format!("/{block}/{key}"),
                    format!(
                        "embedded `{block}` block declares `{key}`; the enclosing $wosWorkflow \
                         envelope's url and version are the sole identity. Embedded blocks have no \
                         independent identity (ADR 0063 §2.1). Remove `{key}` from the embedded block."
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WOS-SIDECAR-TARGET-001 (ADR 0063 §2.2): Sidecar documents MUST declare
// targetWorkflow as a non-empty workflow URI.
// ---------------------------------------------------------------------------

fn check_sidecar_target_workflow(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let target = doc.value.get("targetWorkflow").and_then(Value::as_str);
    let kind_label = match doc.kind {
        DocumentKind::Delivery => "$wosDelivery",
        DocumentKind::OntologyAlignment => "$wosOntologyAlignment",
        _ => return,
    };
    match target {
        Some(uri) if !uri.is_empty() => {}
        _ => {
            diagnostics.push(LintDiagnostic::t1_error(
                "WOS-SIDECAR-TARGET-001",
                "/targetWorkflow",
                format!(
                    "sidecar document `{kind_label}` MUST declare `targetWorkflow` as a non-empty \
                     workflow URI (ADR 0063 §2.2). Sidecars bind to a workflow at deploy time; the \
                     URI MUST match a $wosWorkflow envelope's `url`."
                ),
            ));
        }
    }
}

/// $wosDelivery checks — calendar, notification templates, correspondence.
fn check_delivery(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let root = &doc.value;
    // Business calendar rules (G-058, G-059) live inside delivery.calendar
    if let Some(cal) = root.get("calendar") {
        check_business_calendar_value(cal, "/calendar", diagnostics);
    }
    // Notification template rules (G-062, G-065)
    if let Some(templates) = root.get("notificationTemplates") {
        check_notification_template_value(templates, "/notificationTemplates", diagnostics);
    }
    // Correspondence (CM-001)
    if let Some(corr) = root.get("correspondence") {
        check_correspondence_metadata_value(corr, "/correspondence", diagnostics);
    }
}

// ---------------------------------------------------------------------------
// WOS-VER-LEVEL-001: agents declaring fallbackChain SHOULD also declare
// at least one verificationLevel (ADR 0076 step 12 / Q6 owner decision).
// ---------------------------------------------------------------------------

/// Walk the merged-document root for agents[*].fallbackChain declarations.
/// When any agent declares a fallback chain, the workflow SHOULD declare at
/// least one `verificationLevel` somewhere — typically on `bindings[*]` for
/// governed output paths, or on `advanced.verifiableConstraints` for SMT-
/// verifiable shapes. Warn (not error) when fallbackChain is declared with no
/// verificationLevel anywhere in the document.
fn check_ver_level_for_fallback_chain(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(agents) = root.get("agents").and_then(Value::as_array) else {
        return;
    };

    let agents_with_fallback: Vec<(usize, &str)> = agents
        .iter()
        .enumerate()
        .filter_map(|(i, agent)| {
            let has_fallback = agent
                .get("fallbackChain")
                .and_then(Value::as_array)
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if !has_fallback {
                return None;
            }
            let id = agent.get("id").and_then(Value::as_str)?;
            Some((i, id))
        })
        .collect();

    if agents_with_fallback.is_empty() {
        return;
    }

    let has_verification_level = root
        .get("bindings")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .any(|b| b.get("verificationLevel").is_some())
        })
        .unwrap_or(false)
        || root
            .get("advanced")
            .and_then(|a| a.get("verifiableConstraints"))
            .and_then(Value::as_array)
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

    if has_verification_level {
        return;
    }

    for (i, id) in agents_with_fallback {
        diagnostics.push(LintDiagnostic::t1_warning(
            "WOS-VER-LEVEL-001",
            &format!("/agents/{i}/fallbackChain"),
            format!(
                "agent '{id}' declares fallbackChain but the workflow has no verificationLevel \
                 anywhere; consider declaring bindings[].verificationLevel for governed output paths"
            ),
        ));
    }
}

// ---------------------------------------------------------------------------
// Typed kernel checks
// ---------------------------------------------------------------------------

/// K-001 through K-008: State type semantics (typed model).
fn check_state_type_semantics_typed(
    state: &State,
    path: &str,
    all_state_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    match state.kind {
        StateKind::Final => {
            // K-001
            if !state.transitions.is_empty() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-001",
                    path,
                    "final state must not have outgoing transitions",
                ));
            }
        }
        StateKind::Compound => {
            // K-002
            if state.initial_state.is_none() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-002",
                    path,
                    "compound state must declare initialState",
                ));
            }
            if state.states.is_empty() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-002",
                    path,
                    "compound state must declare substates in states map",
                ));
            }
        }
        StateKind::Parallel => {
            // K-003
            if state.regions.is_empty() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-003",
                    path,
                    "parallel state must declare regions",
                ));
            }
        }
        StateKind::ForEach => {
            // K-FOREACH-001: foreach states MUST declare a non-empty
            // `collection` FEL expression that evaluates to an array at
            // runtime.
            if state.collection.as_deref().unwrap_or("").is_empty() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-FOREACH-001",
                    path,
                    "foreach state must declare a non-empty `collection` FEL expression",
                ));
            }
            // K-FOREACH-002: foreach states MUST declare a `body` State that
            // executes once per iteration. Schema models this as an inline
            // `body: State` field (not the Compound `initialState` + `states`
            // map shape).
            if state.body.is_none() {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-FOREACH-002",
                    path,
                    "foreach state must declare a `body` state to execute per iteration",
                ));
            }
            // K-FOREACH-003: concurrency, when present, MUST be at least 1.
            // Zero would deadlock the iteration; the schema's `minimum: 1`
            // catches authoring; this catches programmatic construction.
            if state.concurrency == Some(0) {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-FOREACH-003",
                    path,
                    "foreach state `concurrency` MUST be at least 1 (zero would deadlock)",
                ));
            }
        }
        StateKind::Atomic => {}
    }

    // K-FOREACH-004: iteration fields (collection, itemVariable,
    // indexVariable, concurrency, breakCondition, outputPath, mergeStrategy,
    // body) are valid only on foreach-typed states. Catch authoring drift
    // where they leak onto a non-foreach state.
    if state.kind != StateKind::ForEach
        && (state.collection.is_some()
            || state.item_variable.is_some()
            || state.index_variable.is_some()
            || state.concurrency.is_some()
            || state.break_condition.is_some()
            || state.output_path.is_some()
            || state.merge_strategy.is_some()
            || state.body.is_some())
    {
        diagnostics.push(LintDiagnostic::t1_error(
            "K-FOREACH-004",
            path,
            "collection / itemVariable / indexVariable / concurrency / breakCondition / \
             outputPath / mergeStrategy / body are only valid on foreach-typed states",
        ));
    }

    // K-004: cancellationPolicy only on parallel
    if state.cancellation_policy.is_some() && state.kind != StateKind::Parallel {
        diagnostics.push(LintDiagnostic::t1_error(
            "K-004",
            path,
            "cancellationPolicy is only valid on parallel states",
        ));
    }

    // K-005: historyState only on compound
    if state.history_state.is_some() && state.kind != StateKind::Compound {
        diagnostics.push(LintDiagnostic::t1_error(
            "K-005",
            path,
            "historyState is only valid on compound states",
        ));
    }

    // K-006, K-007, K-008: transition checks
    for (i, transition) in state.transitions.iter().enumerate() {
        let t_path = format!("{path}/transitions/{i}");

        // K-006
        if !all_state_ids.contains(&transition.target) {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-006",
                &t_path,
                format!(
                    "transition target '{}' does not exist in states map",
                    transition.target
                ),
            ));
        }

        // K-007 — `message` names must not start with `$` (reserved for other
        // TransitionEvent kinds and kernel signals). `signal` allows `$join`
        // and `$compensation.complete` only; other `$…` signal names are invalid.
        // (JSON Schema also constrains author-time documents; this catches the
        // typed model after legacy string coercion.)
        if let Some(ev) = &transition.event {
            match ev {
                TransitionEvent::Message { name, .. } if name.starts_with('$') => {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "K-007",
                        &t_path,
                        format!(
                            "message event name must not use reserved `$` prefix (found '{name}')"
                        ),
                    ));
                }
                TransitionEvent::Signal { name, .. }
                    if name.starts_with('$')
                        && name != "$join"
                        && name != "$compensation.complete" =>
                {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "K-007",
                        &t_path,
                        format!(
                            "signal may only use `$` prefix for '$join' or '$compensation.complete' (found '{name}')"
                        ),
                    ));
                }
                _ => {}
            }
        }

        // K-008
        if state.kind == StateKind::Parallel && !transition.is_parallel_join_transition() {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-008",
                &t_path,
                format!(
                    "parallel state outgoing transition must use signal '$join' (instance scope), found {:?}",
                    transition.event
                ),
            ));
        }
    }

    // Recurse into compound substates
    for (name, substate) in &state.states {
        let sub_path = format!("{path}/states/{name}");
        check_state_type_semantics_typed(substate, &sub_path, all_state_ids, diagnostics);
    }

    // Recurse into parallel regions
    for (region_name, region) in &state.regions {
        for (name, region_state) in &region.states {
            let r_path = format!("{path}/regions/{region_name}/states/{name}");
            check_state_type_semantics_typed(region_state, &r_path, all_state_ids, diagnostics);
        }
    }
}

/// K-016: `lifecycle.initialState` MUST key into `lifecycle.states`, and any
/// compound state that declares `initialState` MUST point at a key in its own
/// substate map. The schema cannot express this cross-property binding under
/// pure Draft 2020-12 (JSON Schema has no native value-references-key
/// keyword); lint covers the gap. Closes STUDIO-DEFER-003 Tranche B.
fn check_initial_state_keys_into_states_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let init = &kernel.lifecycle.initial_state;
    if !kernel.lifecycle.states.contains_key(init) {
        diagnostics.push(LintDiagnostic::t1_error(
            "K-016",
            "/lifecycle/initialState",
            format!(
                "initialState '{init}' does not exist in lifecycle.states (keys: {})",
                kernel
                    .lifecycle
                    .states
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    for (name, state) in &kernel.lifecycle.states {
        check_compound_initial_state_typed(state, &format!("/lifecycle/states/{name}"), diagnostics);
    }
}

fn check_compound_initial_state_typed(
    state: &State,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(init) = &state.initial_state {
        if !state.states.contains_key(init) {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-016",
                &format!("{path}/initialState"),
                format!(
                    "compound initialState '{init}' does not exist in this state's substates (keys: {})",
                    state.states.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
    }
    for (name, sub) in &state.states {
        check_compound_initial_state_typed(sub, &format!("{path}/states/{name}"), diagnostics);
    }
    for (rname, region) in &state.regions {
        // Region carries its own value-references-key obligation:
        // `region.initialState` MUST key into `region.states`. Schema
        // is silent for the same reason it's silent for the compound
        // case, so K-016 covers it here.
        if !region.states.contains_key(&region.initial_state) {
            let init = &region.initial_state;
            let keys = region.states.keys().cloned().collect::<Vec<_>>().join(", ");
            diagnostics.push(LintDiagnostic::t1_error(
                "K-016",
                &format!("{path}/regions/{rname}/initialState"),
                format!(
                    "region initialState '{init}' does not exist in this region's states (keys: {keys})"
                ),
            ));
        }
        for (sname, sstate) in &region.states {
            check_compound_initial_state_typed(
                sstate,
                &format!("{path}/regions/{rname}/states/{sname}"),
                diagnostics,
            );
        }
    }
    // ForEach-body recursion: `state.body` is itself a `State` that
    // may be Compound / Parallel with further nesting, including its
    // own `initialState`. Walk it like any other sub-state.
    if let Some(body) = &state.body {
        check_compound_initial_state_typed(body, &format!("{path}/body"), diagnostics);
    }
}

/// K-015: setData path must reference a declared caseFile field (typed).
fn check_set_data_paths_typed(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(case_file) = &kernel.case_file else {
        return;
    };

    visit_actions_typed(&kernel.lifecycle.states, &mut |action, action_path| {
        if action.action == ActionKind::SetData {
            if let Some(path_val) = &action.path {
                let field_name = path_val.strip_prefix("caseFile.").unwrap_or(path_val);
                let top_field = field_name.split('.').next().unwrap_or(field_name);
                if !case_file.fields.contains_key(top_field) {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "K-015",
                        action_path,
                        format!(
                            "setData path '{path_val}' references undeclared field '{top_field}'"
                        ),
                    ));
                }
            }
        }
    });
}

/// K-014: Milestone ids must be unique (typed).
fn check_milestone_uniqueness_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for id in kernel.lifecycle.milestones.keys() {
        if id.is_empty() {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-014",
                "/lifecycle/milestones",
                "milestone id must not be empty",
            ));
        }
    }
}

/// K-029: startTimer must specify exactly one of duration or deadline (typed).
fn check_timer_exclusivity_typed(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    visit_actions_typed(&kernel.lifecycle.states, &mut |action, action_path| {
        if action.action == ActionKind::StartTimer {
            let has_duration = action.duration.is_some();
            let has_deadline = action.deadline.is_some();

            if has_duration && has_deadline {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-029",
                    action_path,
                    "startTimer must specify exactly one of 'duration' or 'deadline', not both",
                ));
            } else if !has_duration && !has_deadline {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-029",
                    action_path,
                    "startTimer must specify one of 'duration' or 'deadline'",
                ));
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Governance rules
// ---------------------------------------------------------------------------

/// G-044 / G-045: Delegation dates (typed).
fn check_delegation_dates_typed(gov: &GovernanceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    for (i, delegation) in gov.delegations.iter().enumerate() {
        let path = format!("/delegations/{i}");
        let effective = delegation.effective_date.as_deref();
        let expiration = delegation.expiration_date.as_deref();
        let revoked = delegation.revoked_date.as_deref();

        if let (Some(eff), Some(exp)) = (effective, expiration) {
            if exp <= eff {
                diagnostics.push(LintDiagnostic::t1_error(
                    "G-044",
                    &path,
                    format!("expirationDate '{exp}' must be after effectiveDate '{eff}'"),
                ));
            }
        }

        if let (Some(eff), Some(rev)) = (effective, revoked) {
            if rev < eff {
                diagnostics.push(LintDiagnostic::t1_error(
                    "G-045",
                    &path,
                    format!("revokedDate '{rev}' must not be before effectiveDate '{eff}'"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AI Integration rules
// ---------------------------------------------------------------------------

fn check_authoritative_false(record: &Value, path: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    match record.get("authoritative") {
        Some(Value::Bool(false)) => {}
        None => {
            diagnostics.push(LintDiagnostic::t1_warning(
                "AI-049",
                path,
                "narrative tier record missing required 'authoritative' field (must be false)",
            ));
        }
        _ => {
            diagnostics.push(LintDiagnostic::t1_error(
                "AI-049",
                path,
                "narrative tier provenance record must have 'authoritative' set to false",
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion Library rules (no typed model — Value walking)
// ---------------------------------------------------------------------------

/// G-037 / G-038 / G-039: Assertion library checks (value-level, reusable).
fn check_assertion_library_value(
    root: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let assertions = root
        .get("assertions")
        .and_then(Value::as_array)
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    let mut seen_ids = std::collections::HashSet::new();
    for (i, assertion) in assertions.iter().enumerate() {
        let path = format!("{base_path}/assertions/{i}");

        if let Some(id) = assertion.get("id").and_then(Value::as_str) {
            if !seen_ids.insert(id) {
                diagnostics.push(LintDiagnostic::t1_error(
                    "G-037",
                    &path,
                    format!("duplicate assertion id '{id}'"),
                ));
            }
        }
        check_assertion_expression_fields(assertion, &path, diagnostics);
    }
}

/// G-038 / G-039: Assertion expression / fields recommendations.
fn check_assertion_expression_fields(
    assertion: &Value,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let assertion_type = assertion.get("type").and_then(Value::as_str).unwrap_or("");

    match assertion_type {
        "arithmetic" | "range" | "temporal" => {
            if assertion.get("expression").is_none() {
                diagnostics.push(LintDiagnostic::t1_warning(
                    "G-038",
                    path,
                    format!(
                        "assertion of type '{assertion_type}' should include an 'expression' field"
                    ),
                ));
            }
        }
        "source-grounded" | "consistency" => {
            if assertion.get("fields").is_none() {
                diagnostics.push(LintDiagnostic::t1_warning(
                    "G-039",
                    path,
                    format!("assertion of type '{assertion_type}' should include a 'fields' array"),
                ));
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Policy Parameters rules (no typed model — Value walking)
// ---------------------------------------------------------------------------

/// G-047 / G-048 / G-050 / G-057: Policy parameter checks (value-level, reusable).
fn check_policy_parameters_value(
    root: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(params) = root.get("parameters").and_then(Value::as_object) {
        for (name, param) in params {
            let param_path = format!("{base_path}/parameters/{name}");

            if let Some(values) = param.get("values").and_then(Value::as_array) {
                check_values_ascending_effective_date(
                    values,
                    &format!("{param_path}/values"),
                    "G-047",
                    diagnostics,
                );
            }

            if let Some(declared_type) = param.get("type").and_then(Value::as_str) {
                if let Some(values) = param.get("values").and_then(Value::as_array) {
                    for (i, entry) in values.iter().enumerate() {
                        check_parameter_value_type(
                            entry,
                            declared_type,
                            &format!("{param_path}/values/{i}"),
                            diagnostics,
                        );
                    }
                }
            }
        }
    }

    if let Some(bindings) = root.get("bindings").and_then(Value::as_object) {
        for (key, binding) in bindings {
            let binding_path = format!("{base_path}/bindings/{key}");

            if let Some(id) = binding.get("id").and_then(Value::as_str) {
                if id != key {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "G-048",
                        &binding_path,
                        format!("binding id '{id}' must match map key '{key}'"),
                    ));
                }
            }

            if let Some(values) = binding.get("values").and_then(Value::as_array) {
                check_values_ascending_effective_date(
                    values,
                    &format!("{binding_path}/values"),
                    "G-057",
                    diagnostics,
                );
            }
        }
    }
}

/// G-055: Hold policy expectedDuration raw-value check (used in embedded blocks).
fn check_hold_expected_duration_raw(
    policy: &Value,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(duration) = policy.get("expectedDuration").and_then(Value::as_str) else {
        return;
    };
    if duration != "indefinite" && !duration.starts_with('P') {
        diagnostics.push(LintDiagnostic::t1_error(
            "G-055",
            &format!("{path}/expectedDuration"),
            format!(
                "expectedDuration '{duration}' is not a valid ISO 8601 duration or 'indefinite'"
            ),
        ));
    }
}

/// I-001: outputBinding JSONPath checks on the `bindings` embedded block.
fn check_bindings_block(
    bindings: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(bindings_obj) = bindings.as_object() else {
        return;
    };
    for (binding_key, binding) in bindings_obj {
        let Some(output_binding) = binding.get("outputBinding").and_then(Value::as_object) else {
            continue;
        };
        for (case_path, json_path_value) in output_binding {
            let Some(json_path) = json_path_value.as_str() else {
                continue;
            };
            if contains_unsupported_jsonpath_feature(json_path) {
                let path = format!("{base_path}/{binding_key}/outputBinding/{case_path}");
                diagnostics.push(LintDiagnostic::t1_error(
                    "I-001",
                    path,
                    format!(
                        "outputBinding JSONPath '{json_path}' uses a feature not supported in \
                         the RFC 9535 output-binding profile: filter expressions ([?(...)]) and \
                         recursive descent (..) are excluded for predictability and static \
                         analysability. Extend the profile via ADR if a future binding requires \
                         these features."
                    ),
                ));
            }
        }
    }
}

/// AI-049 narrative tier check on an array value (path-aware).
fn check_narrative_tier_authoritative_in(
    records: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(arr) = records.as_array() else {
        return;
    };
    for (i, record) in arr.iter().enumerate() {
        check_authoritative_false(record, &format!("{base_path}/{i}"), diagnostics);
    }
}

/// AI-041: Fallback chain termination check on a raw JSON array.
fn check_fallback_chain_termination_raw(
    chain: &[Value],
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if chain.is_empty() {
        return;
    }
    let last_action = chain
        .last()
        .and_then(|l| l.get("action").and_then(Value::as_str))
        .unwrap_or("");
    if last_action != "escalateToHuman" && last_action != "fail" {
        diagnostics.push(LintDiagnostic::t1_error(
            "AI-041",
            path,
            format!(
                "fallback chain must terminate in 'escalateToHuman' or 'fail', found '{last_action}'"
            ),
        ));
    }
    let mut seen_alternate_agents = std::collections::HashSet::new();
    for (i, level) in chain.iter().enumerate() {
        if let Some(agent_ref) = level.get("alternateAgentRef").and_then(Value::as_str) {
            if !seen_alternate_agents.insert(agent_ref) {
                diagnostics.push(LintDiagnostic::t1_error(
                    "AI-041",
                    &format!("{path}/{i}"),
                    format!(
                        "fallback chain cycles: alternateAgent '{agent_ref}' appears more than once"
                    ),
                ));
            }
        }
    }
}

/// Shared helper for G-047 and G-057.
fn check_values_ascending_effective_date(
    values: &[Value],
    path: &str,
    rule_id: &'static str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mut prev_date: Option<&str> = None;
    for (i, entry) in values.iter().enumerate() {
        if let Some(date) = entry.get("effectiveDate").and_then(Value::as_str) {
            if let Some(prev) = prev_date {
                if date <= prev {
                    diagnostics.push(LintDiagnostic::t1_error(
                        rule_id,
                        &format!("{path}/{i}"),
                        format!("effectiveDate '{date}' is not after previous '{prev}'"),
                    ));
                }
            }
            prev_date = Some(date);
        }
    }
}

/// G-050: Resolved parameter value type consistency.
fn check_parameter_value_type(
    entry: &Value,
    declared_type: &str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(value) = entry.get("value") else {
        return;
    };

    let type_matches = match declared_type {
        "number" | "integer" => value.is_number(),
        "boolean" => value.is_boolean(),
        "string" | "date" | "datetime" | "duration" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    };

    if !type_matches {
        let actual_kind = json_type_name(value);
        diagnostics.push(LintDiagnostic::t1_error(
            "G-050",
            path,
            format!("parameter value is {actual_kind} but declared type is '{declared_type}'"),
        ));
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// K-048: Case relationship type prefix.
// ---------------------------------------------------------------------------

fn check_case_relationship_type_prefix_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    const STANDARD_RELATIONSHIP_TYPES: &[&str] =
        &["parent", "child", "sibling", "related", "supersedes"];

    let Some(case_file) = &kernel.case_file else {
        return;
    };

    for (i, rel) in case_file.relationships.iter().enumerate() {
        let rel_type = rel.kind.as_str();
        let is_standard = STANDARD_RELATIONSHIP_TYPES.contains(&rel_type);
        let is_extension = rel_type.starts_with("x-");

        if !is_standard && !is_extension {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-048",
                &format!("/caseFile/relationships/{i}/type"),
                format!("non-standard case relationship type '{rel_type}' must use 'x-' prefix"),
            ));
        }
    }
}

/// K-021: Provenance `actorId` MUST reference a declared kernel actor.
fn check_provenance_actor_ids_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let actors: std::collections::HashSet<&str> =
        kernel.actors.iter().map(|a| a.id.as_str()).collect();

    let Some(prov) = &kernel.provenance else {
        return;
    };
    let Value::Array(records) = prov else {
        return;
    };

    for (i, rec) in records.iter().enumerate() {
        let Some(obj) = rec.as_object() else {
            continue;
        };
        let Some(actor_id) = obj.get("actorId").and_then(Value::as_str) else {
            continue;
        };
        if !actors.contains(actor_id) {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-021",
                &format!("/provenance/{i}/actorId"),
                format!(
                    "provenance actorId '{actor_id}' does not reference a declared kernel actor"
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Correspondence Metadata + Business Calendar + Notification Template (sidecars)
// ---------------------------------------------------------------------------

/// K-009: Actor identifiers MUST be unique within the kernel actor list.
fn check_actor_id_uniqueness_typed(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut seen = std::collections::HashSet::new();

    for (index, actor) in kernel.actors.iter().enumerate() {
        if !seen.insert(actor.id.as_str()) {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-009",
                &format!("/actors/{index}/id"),
                format!("duplicate actor id '{}'", actor.id),
            ));
        }
    }
}

/// CM-001: Entry template ids MUST be unique within the correspondence block.
fn check_correspondence_metadata_value(
    corr: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let entry_templates = corr
        .get("entryTemplates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let mut seen = std::collections::HashSet::new();
    for (index, template) in entry_templates.iter().enumerate() {
        let Some(id) = template.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(id) {
            diagnostics.push(LintDiagnostic::t1_error(
                "CM-001",
                &format!("{base_path}/entryTemplates/{index}/id"),
                format!("duplicate correspondence entry template id '{id}'"),
            ));
        }
    }
}

/// G-058 / G-059: Business calendar structural validity (value-level).
fn check_business_calendar_value(
    cal: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(holidays) = cal.get("holidays").and_then(Value::as_array) {
        for (i, h) in holidays.iter().enumerate() {
            let path = format!("{base_path}/holidays/{i}");
            let has_date = h.get("date").is_some();
            let has_rule = h.get("rule").is_some();
            if has_date == has_rule {
                diagnostics.push(LintDiagnostic::t1_error(
                    "G-058",
                    &path,
                    if !has_date {
                        "holiday entry MUST specify exactly one of 'date' or 'rule'"
                    } else {
                        "holiday entry MUST specify exactly one of 'date' or 'rule', not both"
                    },
                ));
            }
        }
    }

    if let Some(oh) = cal.get("operatingHours") {
        let start = oh.get("start").and_then(Value::as_str);
        let end = oh.get("end").and_then(Value::as_str);
        if let (Some(s), Some(e)) = (start, end) {
            let start_m = hh_mm_to_minutes(s);
            let end_m = hh_mm_to_minutes(e);
            match (start_m, end_m) {
                (Some(sm), Some(em)) if em <= sm => {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "G-059",
                        &format!("{base_path}/operatingHours/end"),
                        "operating hours 'end' MUST be strictly after 'start'",
                    ));
                }
                (Some(_), Some(_)) => {}
                _ => {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "G-059",
                        &format!("{base_path}/operatingHours"),
                        "operating hours 'start' and 'end' MUST be valid 24-hour HH:MM values",
                    ));
                }
            }
        }
    }
}

/// G-062 / G-065: Notification templates block (value-level).
fn check_notification_template_value(
    templates: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(template_map) = templates.as_object() else {
        return;
    };
    for (key, template) in template_map {
        let path = format!("{base_path}/{key}");
        check_adverse_decision_template_sections(template, &path, diagnostics);
        check_template_section_id_uniqueness(template, &path, diagnostics);
    }
}

/// G-062: Adverse-decision category requires four section classes (NT S4.4).
///
/// Uses **heuristic** id / `contentType` checks (canonical ids such as `determination`,
/// `reasons`, `appealRights`, `appealInstructions`, and `contentType: appeal-rights`).
/// Templates that satisfy the spec with different ids may need schema-only validation or
/// expanded matchers if false negatives appear in real documents.
fn check_adverse_decision_template_sections(
    template: &Value,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if template.get("category").and_then(Value::as_str) != Some("adverse-decision") {
        return;
    }
    let Some(sections) = template.get("sections").and_then(Value::as_array) else {
        return;
    };

    let mut has_determination = false;
    let mut has_reason_codes = false;
    let mut has_appeal_rights = false;
    let mut has_appeal_instructions = false;

    for sec in sections {
        let id = sec
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_lowercase)
            .unwrap_or_default();
        let ct = sec.get("contentType").and_then(Value::as_str).unwrap_or("");

        if id == "determination" {
            has_determination = true;
        }
        if matches!(id.as_str(), "reasons" | "reasoncodes" | "reason") {
            has_reason_codes = true;
        }
        if ct == "appeal-rights" || id == "appealrights" {
            has_appeal_rights = true;
        }
        if id == "appealinstructions" {
            has_appeal_instructions = true;
        }
    }

    if !has_determination {
        diagnostics.push(LintDiagnostic::t1_error(
            "G-062",
            path,
            "adverse-decision template MUST include a section with id 'determination'",
        ));
    }
    if !has_reason_codes {
        diagnostics.push(LintDiagnostic::t1_error(
            "G-062",
            path,
            "adverse-decision template MUST include reason code coverage (section id 'reasons', 'reasonCodes', or 'reason')",
        ));
    }
    if !has_appeal_rights {
        diagnostics.push(LintDiagnostic::t1_error(
            "G-062",
            path,
            "adverse-decision template MUST include appeal rights (section id 'appealRights' or contentType 'appeal-rights')",
        ));
    }
    if !has_appeal_instructions {
        diagnostics.push(LintDiagnostic::t1_error(
            "G-062",
            path,
            "adverse-decision template MUST include a section with id 'appealInstructions'",
        ));
    }
}

/// G-065: Section `id` values unique within each template.
fn check_template_section_id_uniqueness(
    template: &Value,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(sections) = template.get("sections").and_then(Value::as_array) else {
        return;
    };
    let mut seen = std::collections::HashSet::new();
    for (i, sec) in sections.iter().enumerate() {
        let Some(id) = sec.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(id) {
            diagnostics.push(LintDiagnostic::t1_error(
                "G-065",
                &format!("{path}/sections/{i}/id"),
                format!("duplicate section id '{id}' within template"),
            ));
        }
    }
}

/// Parse `HH:MM` (24h) to minutes since midnight; returns `None` if malformed.
fn hh_mm_to_minutes(s: &str) -> Option<u16> {
    let (h, m) = s.split_once(':')?;
    let hh: u16 = h.parse().ok()?;
    let mm: u16 = m.parse().ok()?;
    if hh > 23 || mm > 59 {
        return None;
    }
    Some(hh * 60 + mm)
}

// ---------------------------------------------------------------------------
// Shared helpers (always Value-based — structural tree walking)
// ---------------------------------------------------------------------------

/// K-022: A provenance digest implies algorithm must be recorded.
fn check_digest_algorithm(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    visit_all_objects(root, "", &mut |obj, obj_path| {
        if obj.contains_key("digest") {
            let has_algorithm = obj
                .get("extensions")
                .and_then(Value::as_object)
                .is_some_and(|ext| ext.contains_key("algorithm"));

            if !has_algorithm {
                diagnostics.push(LintDiagnostic::t1_error(
                    "K-022",
                    obj_path,
                    "object has 'digest' but no 'algorithm' key in its extensions map",
                ));
            }
        }
    });
}

/// K-030: Extension keys must be x- prefixed.
fn check_extension_prefixes(value: &Value, path: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(obj) = value.as_object() {
        if let Some(extensions) = obj.get("extensions").and_then(Value::as_object) {
            let ext_path = if path.is_empty() {
                "/extensions".to_string()
            } else {
                format!("{path}/extensions")
            };
            for key in extensions.keys() {
                if !key.starts_with("x-") {
                    diagnostics.push(LintDiagnostic::t1_error(
                        "K-030",
                        &ext_path,
                        format!("extension key '{key}' must be prefixed with 'x-'"),
                    ));
                }
            }
        }

        for (key, child) in obj {
            let child_path = if path.is_empty() {
                format!("/{key}")
            } else {
                format!("{path}/{key}")
            };
            check_extension_prefixes(child, &child_path, diagnostics);
        }
    } else if let Some(arr) = value.as_array() {
        for (i, child) in arr.iter().enumerate() {
            let child_path = format!("{path}/{i}");
            check_extension_prefixes(child, &child_path, diagnostics);
        }
    }
}

/// Collect all state identifiers (typed).
fn collect_all_state_ids_typed(
    states: &indexmap::IndexMap<String, State>,
) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    collect_state_ids_recursive_typed(states, &mut ids);
    ids
}

fn collect_state_ids_recursive_typed(
    states: &indexmap::IndexMap<String, State>,
    ids: &mut std::collections::HashSet<String>,
) {
    for (name, state) in states {
        ids.insert(name.clone());
        collect_state_ids_recursive_typed(&state.states, ids);
        for region in state.regions.values() {
            collect_state_ids_recursive_typed(&region.states, ids);
        }
    }
}

/// Walk all actions in a typed kernel lifecycle.
fn visit_actions_typed(
    states: &indexmap::IndexMap<String, State>,
    f: &mut dyn FnMut(&wos_core::model::kernel::Action, &str),
) {
    for (name, state) in states {
        let path = format!("/lifecycle/states/{name}");
        visit_state_actions_typed(state, &path, f);
    }
}

fn visit_state_actions_typed(
    state: &State,
    path: &str,
    f: &mut dyn FnMut(&wos_core::model::kernel::Action, &str),
) {
    for (i, action) in state.on_entry.iter().enumerate() {
        f(action, &format!("{path}/onEntry/{i}"));
    }
    for (i, action) in state.on_exit.iter().enumerate() {
        f(action, &format!("{path}/onExit/{i}"));
    }
    for (ti, transition) in state.transitions.iter().enumerate() {
        for (ai, action) in transition.actions.iter().enumerate() {
            f(action, &format!("{path}/transitions/{ti}/actions/{ai}"));
        }
    }
    for (name, substate) in &state.states {
        visit_state_actions_typed(substate, &format!("{path}/states/{name}"), f);
    }
    for (rname, region) in &state.regions {
        for (name, rstate) in &region.states {
            visit_state_actions_typed(rstate, &format!("{path}/regions/{rname}/states/{name}"), f);
        }
    }
}

// ---------------------------------------------------------------------------
// Output-binding JSONPath profile (I-001)
// ---------------------------------------------------------------------------
//
// The standalone `check_integration_profile` Tier-1 entry retired
// 2026-04-28: per ADR 0076 D-1 the integration-profile document was absorbed
// into the merged `$wosWorkflow` envelope and I-001 now fires from the
// embedded-`bindings` walk earlier in this file (search for `"I-001"`). The
// JSONPath feature-detection helper survives because the embedded walk
// reuses it.

/// Return `true` if the JSONPath string contains a filter expression or recursive descent.
///
/// Uses a minimal state machine that skips characters inside quoted key segments
/// (`'...'` or `"..."`), so quoted key names like `$['a..b']` do not produce
/// false positives. The only known false-positive pattern with the previous
/// substring search was `$['a..b']` — this implementation eliminates it.
fn contains_unsupported_jsonpath_feature(json_path: &str) -> bool {
    let mut chars = json_path.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev = '\0';

    while let Some(ch) = chars.next() {
        match ch {
            // Toggle single-quote state (not inside a double-quoted section).
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            // Toggle double-quote state (not inside a single-quoted section).
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            // Outside quotes: check for unsupported features.
            '.' if !in_single_quote && !in_double_quote => {
                if prev == '.' {
                    // Found `..` — recursive descent.
                    return true;
                }
            }
            '[' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'?') {
                    // Found `[?` — filter expression.
                    return true;
                }
            }
            _ => {}
        }
        prev = ch;
    }

    false
}

#[cfg(test)]
mod jsonpath_feature_tests {
    use super::contains_unsupported_jsonpath_feature;

    #[test]
    fn recursive_descent_is_flagged() {
        assert!(contains_unsupported_jsonpath_feature("$..field"));
        assert!(contains_unsupported_jsonpath_feature("$.foo..bar"));
    }

    #[test]
    fn filter_expression_is_flagged() {
        assert!(contains_unsupported_jsonpath_feature("$[?(@.x > 1)]"));
        assert!(contains_unsupported_jsonpath_feature("$.items[?(@ > 0)]"));
    }

    #[test]
    fn normal_paths_are_not_flagged() {
        assert!(!contains_unsupported_jsonpath_feature("$.foo.bar"));
        assert!(!contains_unsupported_jsonpath_feature("$.items[0]"));
        assert!(!contains_unsupported_jsonpath_feature("$['a']"));
        assert!(!contains_unsupported_jsonpath_feature("$.items[*].value"));
    }

    #[test]
    fn quoted_key_with_dots_is_not_flagged() {
        // $['a..b'] is the known false-positive pattern — must NOT be flagged.
        assert!(!contains_unsupported_jsonpath_feature("$['a..b']"));
        assert!(!contains_unsupported_jsonpath_feature("$[\"a..b\"]"));
    }
}

#[cfg(test)]
mod ver_level_tests {
    use super::*;
    use serde_json::json;

    fn run(value: Value) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        check_ver_level_for_fallback_chain(&value, &mut diags);
        diags
    }

    #[test]
    fn ver_level_001_fallback_without_verification_level_warns() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "agents": [
                {
                    "id": "extractor",
                    "fallbackChain": ["humanReviewer"]
                }
            ]
        });
        let diags = run(doc);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-VER-LEVEL-001")
            .collect();
        assert_eq!(matches.len(), 1, "expected exactly one diagnostic: {diags:?}");
        assert!(matches[0].message.contains("extractor"));
    }

    #[test]
    fn ver_level_001_fallback_with_binding_verification_level_clean() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "agents": [
                {
                    "id": "extractor",
                    "fallbackChain": ["humanReviewer"]
                }
            ],
            "bindings": [
                {
                    "on": "extracted",
                    "verificationLevel": "attested"
                }
            ]
        });
        let diags = run(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-VER-LEVEL-001"),
            "expected no WOS-VER-LEVEL-001 diagnostic: {diags:?}"
        );
    }

    #[test]
    fn ver_level_001_no_fallback_chain_silent() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "agents": [{"id": "extractor"}]
        });
        let diags = run(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-VER-LEVEL-001"),
            "expected no WOS-VER-LEVEL-001 diagnostic: {diags:?}"
        );
    }
}

#[cfg(test)]
mod adr0063_identity_boundary_tests {
    use super::*;
    use crate::document::{DocumentKind, WosDocument};
    use serde_json::json;

    fn run_target(value: Value) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        check_embedded_no_target_workflow(&value, &mut diags);
        diags
    }

    fn run_identity(value: Value) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        check_embedded_no_independent_identity(&value, &mut diags);
        diags
    }

    fn run_sidecar(kind: DocumentKind, value: Value) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        let doc = WosDocument {
            kind,
            value,
            source: None,
        };
        check_sidecar_target_workflow(&doc, &mut diags);
        diags
    }

    // ── WOS-EMBED-TARGET-001 ─────────────────────────────────────────────────

    #[test]
    fn embed_target_001_governance_with_target_workflow_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "governance": {
                "targetWorkflow": "https://other.example/workflows/x"
            }
        });
        let diags = run_target(doc);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-EMBED-TARGET-001")
            .collect();
        assert_eq!(matches.len(), 1, "expected exactly one diagnostic: {diags:?}");
        assert_eq!(matches[0].path, "/governance/targetWorkflow");
    }

    #[test]
    fn embed_target_001_agents_array_entry_with_target_workflow_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "agents": [
                {"id": "a", "targetWorkflow": "https://other.example/x"}
            ]
        });
        let diags = run_target(doc);
        assert!(
            diags.iter().any(|d| d.rule_id == "WOS-EMBED-TARGET-001" && d.path == "/agents/0/targetWorkflow"),
            "expected diagnostic on /agents/0/targetWorkflow: {diags:?}"
        );
    }

    #[test]
    fn embed_target_001_clean_envelope_silent() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "governance": {"dueProcess": {}},
            "agents": [{"id": "a"}]
        });
        let diags = run_target(doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-EMBED-TARGET-001"),
            "expected no diagnostic: {diags:?}"
        );
    }

    // ── WOS-EMBED-IDENTITY-001 ───────────────────────────────────────────────

    #[test]
    fn embed_identity_001_governance_with_url_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "governance": {"url": "https://other.example/x"}
        });
        let diags = run_identity(doc);
        assert!(
            diags.iter().any(|d| d.rule_id == "WOS-EMBED-IDENTITY-001" && d.path == "/governance/url"),
            "expected diagnostic on /governance/url: {diags:?}"
        );
    }

    #[test]
    fn embed_identity_001_advanced_with_version_flagged() {
        let doc = json!({
            "$wosWorkflow": "1.0",
            "advanced": {"version": "2.0.0"}
        });
        let diags = run_identity(doc);
        assert!(
            diags.iter().any(|d| d.rule_id == "WOS-EMBED-IDENTITY-001" && d.path == "/advanced/version"),
            "expected diagnostic on /advanced/version: {diags:?}"
        );
    }

    // ── WOS-SIDECAR-TARGET-001 ───────────────────────────────────────────────

    #[test]
    fn sidecar_target_001_delivery_without_target_workflow_flagged() {
        let doc = json!({
            "$wosDelivery": "1.0"
        });
        let diags = run_sidecar(DocumentKind::Delivery, doc);
        let matches: Vec<_> = diags
            .iter()
            .filter(|d| d.rule_id == "WOS-SIDECAR-TARGET-001")
            .collect();
        assert_eq!(matches.len(), 1, "expected exactly one diagnostic: {diags:?}");
    }

    #[test]
    fn sidecar_target_001_delivery_with_empty_target_workflow_flagged() {
        let doc = json!({
            "$wosDelivery": "1.0",
            "targetWorkflow": ""
        });
        let diags = run_sidecar(DocumentKind::Delivery, doc);
        assert!(
            diags.iter().any(|d| d.rule_id == "WOS-SIDECAR-TARGET-001"),
            "expected diagnostic on empty targetWorkflow: {diags:?}"
        );
    }

    #[test]
    fn sidecar_target_001_delivery_with_valid_target_workflow_silent() {
        let doc = json!({
            "$wosDelivery": "1.0",
            "targetWorkflow": "https://agency.gov/workflows/benefits"
        });
        let diags = run_sidecar(DocumentKind::Delivery, doc);
        assert!(
            diags.iter().all(|d| d.rule_id != "WOS-SIDECAR-TARGET-001"),
            "expected no diagnostic: {diags:?}"
        );
    }

    #[test]
    fn sidecar_target_001_ontology_alignment_without_target_workflow_flagged() {
        let doc = json!({
            "$wosOntologyAlignment": "1.0"
        });
        let diags = run_sidecar(DocumentKind::OntologyAlignment, doc);
        assert!(
            diags.iter().any(|d| d.rule_id == "WOS-SIDECAR-TARGET-001"),
            "expected diagnostic on missing targetWorkflow: {diags:?}"
        );
    }
}

/// Recursively visit every JSON object node.
fn visit_all_objects(
    value: &Value,
    path: &str,
    f: &mut dyn FnMut(&serde_json::Map<String, Value>, &str),
) {
    match value {
        Value::Object(obj) => {
            f(obj, path);
            for (key, child) in obj {
                let child_path = if path.is_empty() {
                    format!("/{key}")
                } else {
                    format!("{path}/{key}")
                };
                visit_all_objects(child, &child_path, f);
            }
        }
        Value::Array(arr) => {
            for (i, child) in arr.iter().enumerate() {
                let child_path = format!("{path}/{i}");
                visit_all_objects(child, &child_path, f);
            }
        }
        _ => {}
    }
}
