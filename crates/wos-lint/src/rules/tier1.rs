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

use wos_core::model::ai::{AIIntegrationDocument, FallbackAction};
use wos_core::model::governance::GovernanceDocument;
use wos_core::model::kernel::{ActionKind, KernelDocument, State, StateKind};

use crate::diagnostic::Diagnostic;
use crate::document::{DocumentKind, WosDocument};

/// Run all Tier 1 checks applicable to the document's kind.
pub fn check(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    match doc.kind {
        DocumentKind::Kernel => check_kernel(doc, diagnostics),
        DocumentKind::WorkflowGovernance => check_governance(doc, diagnostics),
        DocumentKind::AiIntegration => check_ai_integration(doc, diagnostics),
        DocumentKind::AssertionLibrary => check_assertion_library(doc, diagnostics),
        DocumentKind::PolicyParameters => check_policy_parameters(doc, diagnostics),
        DocumentKind::CorrespondenceMetadata => check_correspondence_metadata(doc, diagnostics),
        DocumentKind::BusinessCalendar => check_business_calendar(doc, diagnostics),
        DocumentKind::NotificationTemplate => check_notification_template(doc, diagnostics),
        _ => {} // Other document types: no Tier 1 rules yet.
    }
}

// ---------------------------------------------------------------------------
// Kernel rules
// ---------------------------------------------------------------------------

fn check_kernel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Ok(kernel) = serde_json::from_value::<KernelDocument>(root.clone()) {
        let all_state_ids = collect_all_state_ids_typed(&kernel.lifecycle.states);
        for (name, state) in &kernel.lifecycle.states {
            let path = format!("/lifecycle/states/{name}");
            check_state_type_semantics_typed(state, &path, &all_state_ids, diagnostics);
        }

        check_set_data_paths_typed(&kernel, diagnostics);
        check_milestone_uniqueness_typed(&kernel, diagnostics);
        check_timer_exclusivity_typed(&kernel, diagnostics);
        check_case_relationship_type_prefix_typed(&kernel, diagnostics);
        check_actor_id_uniqueness_typed(&kernel, diagnostics);
        check_provenance_actor_ids_typed(&kernel, diagnostics);
    }

    check_digest_algorithm(root, diagnostics);
    check_extension_prefixes(root, "", diagnostics);
}

// ---------------------------------------------------------------------------
// Typed kernel checks
// ---------------------------------------------------------------------------

/// K-001 through K-008: State type semantics (typed model).
fn check_state_type_semantics_typed(
    state: &State,
    path: &str,
    all_state_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match state.kind {
        StateKind::Final => {
            // K-001
            if !state.transitions.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "K-001",
                    path,
                    "final state must not have outgoing transitions",
                ));
            }
        }
        StateKind::Compound => {
            // K-002
            if state.initial_state.is_none() {
                diagnostics.push(Diagnostic::error(
                    "K-002",
                    path,
                    "compound state must declare initialState",
                ));
            }
            if state.states.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "K-002",
                    path,
                    "compound state must declare substates in states map",
                ));
            }
        }
        StateKind::Parallel => {
            // K-003
            if state.regions.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "K-003",
                    path,
                    "parallel state must declare regions",
                ));
            }
        }
        StateKind::Atomic => {}
    }

    // K-004: cancellationPolicy only on parallel
    if state.cancellation_policy.is_some() && state.kind != StateKind::Parallel {
        diagnostics.push(Diagnostic::error(
            "K-004",
            path,
            "cancellationPolicy is only valid on parallel states",
        ));
    }

    // K-005: historyState only on compound
    if state.history_state.is_some() && state.kind != StateKind::Compound {
        diagnostics.push(Diagnostic::error(
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
            diagnostics.push(Diagnostic::error(
                "K-006",
                &t_path,
                format!(
                    "transition target '{}' does not exist in states map",
                    transition.target
                ),
            ));
        }

        // K-007
        if transition.event.starts_with('$') && transition.event != "$join" {
            diagnostics.push(Diagnostic::error(
                "K-007",
                &t_path,
                format!("event name '{}' uses reserved $ prefix", transition.event),
            ));
        }

        // K-008
        if state.kind == StateKind::Parallel && transition.event != "$join" {
            diagnostics.push(Diagnostic::error(
                "K-008",
                &t_path,
                format!(
                    "parallel state outgoing transition must use '$join' event, found '{}'",
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

/// K-015: setData path must reference a declared caseFile field (typed).
fn check_set_data_paths_typed(kernel: &KernelDocument, diagnostics: &mut Vec<Diagnostic>) {
    let Some(case_file) = &kernel.case_file else {
        return;
    };

    visit_actions_typed(&kernel.lifecycle.states, &mut |action, action_path| {
        if action.action == ActionKind::SetData {
            if let Some(path_val) = &action.path {
                let field_name = path_val.strip_prefix("caseFile.").unwrap_or(path_val);
                let top_field = field_name.split('.').next().unwrap_or(field_name);
                if !case_file.fields.contains_key(top_field) {
                    diagnostics.push(Diagnostic::error(
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
fn check_milestone_uniqueness_typed(kernel: &KernelDocument, diagnostics: &mut Vec<Diagnostic>) {
    for id in kernel.lifecycle.milestones.keys() {
        if id.is_empty() {
            diagnostics.push(Diagnostic::error(
                "K-014",
                "/lifecycle/milestones",
                "milestone id must not be empty",
            ));
        }
    }
}

/// K-029: startTimer must specify exactly one of duration or deadline (typed).
fn check_timer_exclusivity_typed(kernel: &KernelDocument, diagnostics: &mut Vec<Diagnostic>) {
    visit_actions_typed(&kernel.lifecycle.states, &mut |action, action_path| {
        if action.action == ActionKind::StartTimer {
            let has_duration = action.duration.is_some();
            let has_deadline = action.deadline.is_some();

            if has_duration && has_deadline {
                diagnostics.push(Diagnostic::error(
                    "K-029",
                    action_path,
                    "startTimer must specify exactly one of 'duration' or 'deadline', not both",
                ));
            } else if !has_duration && !has_deadline {
                diagnostics.push(Diagnostic::error(
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

fn check_governance(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Ok(gov) = serde_json::from_value::<GovernanceDocument>(root.clone()) {
        check_delegation_dates_typed(&gov, diagnostics);
        check_hold_expected_duration_typed(&gov, diagnostics);
    }

    check_extension_prefixes(root, "", diagnostics);
}

/// G-055: Hold policy expectedDuration (typed).
fn check_hold_expected_duration_typed(gov: &GovernanceDocument, diagnostics: &mut Vec<Diagnostic>) {
    for (i, hold) in gov.hold_policies.iter().enumerate() {
        let duration = &hold.expected_duration;
        if duration != "indefinite" && !duration.starts_with('P') {
            diagnostics.push(Diagnostic::error(
                "G-055",
                &format!("/holdPolicies/{i}/expectedDuration"),
                format!(
                    "expectedDuration '{duration}' is not a valid ISO 8601 duration or 'indefinite'"
                ),
            ));
        }
    }
}

/// G-044 / G-045: Delegation dates (typed).
fn check_delegation_dates_typed(gov: &GovernanceDocument, diagnostics: &mut Vec<Diagnostic>) {
    for (i, delegation) in gov.delegations.iter().enumerate() {
        let path = format!("/delegations/{i}");
        let effective = delegation.effective_date.as_deref();
        let expiration = delegation.expiration_date.as_deref();
        let revoked = delegation.revoked_date.as_deref();

        if let (Some(eff), Some(exp)) = (effective, expiration) {
            if exp <= eff {
                diagnostics.push(Diagnostic::error(
                    "G-044",
                    &path,
                    format!("expirationDate '{exp}' must be after effectiveDate '{eff}'"),
                ));
            }
        }

        if let (Some(eff), Some(rev)) = (effective, revoked) {
            if rev < eff {
                diagnostics.push(Diagnostic::error(
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

fn check_ai_integration(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Ok(ai) = serde_json::from_value::<AIIntegrationDocument>(root.clone()) {
        check_fallback_chain_termination_typed(&ai.fallback_chain, "/fallbackChain", diagnostics);

        for (i, agent) in ai.agents.iter().enumerate() {
            if !agent.fallback_chain.is_empty() {
                let path = format!("/agents/{i}/fallbackChain");
                check_fallback_chain_termination_typed(&agent.fallback_chain, &path, diagnostics);
            }
        }
    }

    // AI-049: Narrative tier authoritative (always Value-based — dynamic structure).
    check_narrative_tier_authoritative(root, diagnostics);

    check_extension_prefixes(root, "", diagnostics);
}

/// AI-049: Every Narrative provenance record must have `authoritative: false`.
fn check_narrative_tier_authoritative(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(records) = root.get("narrativeProvenance").and_then(Value::as_array) {
        for (i, record) in records.iter().enumerate() {
            check_authoritative_false(record, &format!("/narrativeProvenance/{i}"), diagnostics);
        }
    }

    if let Some(records) = root.get("provenance").and_then(Value::as_array) {
        for (i, record) in records.iter().enumerate() {
            if record.get("tier").and_then(Value::as_str) == Some("narrative") {
                check_authoritative_false(record, &format!("/provenance/{i}"), diagnostics);
            }
        }
    }
}

fn check_authoritative_false(record: &Value, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    match record.get("authoritative") {
        Some(Value::Bool(false)) => {}
        None => {
            diagnostics.push(Diagnostic::warning(
                "AI-049",
                path,
                "narrative tier record missing required 'authoritative' field (must be false)",
            ));
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                "AI-049",
                path,
                "narrative tier provenance record must have 'authoritative' set to false",
            ));
        }
    }
}

/// AI-041: Fallback chain termination (typed).
fn check_fallback_chain_termination_typed(
    chain: &[wos_core::model::ai::FallbackLevel],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if chain.is_empty() {
        return;
    }

    if let Some(last) = chain.last() {
        if last.action != FallbackAction::EscalateToHuman && last.action != FallbackAction::Fail {
            let action_str = match last.action {
                FallbackAction::Retry => "retry",
                FallbackAction::AlternateAgent => "alternateAgent",
                FallbackAction::EscalateToHuman => "escalateToHuman",
                FallbackAction::Fail => "fail",
            };
            diagnostics.push(Diagnostic::error(
                "AI-041",
                path,
                format!(
                    "fallback chain must terminate in 'escalateToHuman' or 'fail', found '{action_str}'"
                ),
            ));
        }
    }

    let mut seen_alternate_agents = std::collections::HashSet::new();
    for (i, level) in chain.iter().enumerate() {
        if let Some(agent_ref) = &level.alternate_agent_ref {
            if !seen_alternate_agents.insert(agent_ref.as_str()) {
                diagnostics.push(Diagnostic::error(
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

// ---------------------------------------------------------------------------
// Assertion Library rules (no typed model — Value walking)
// ---------------------------------------------------------------------------

fn check_assertion_library(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(assertions) = root.get("assertions").and_then(Value::as_array) {
        let mut seen_ids = std::collections::HashSet::new();
        for (i, assertion) in assertions.iter().enumerate() {
            let path = format!("/assertions/{i}");

            // G-037
            if let Some(id) = assertion.get("id").and_then(Value::as_str) {
                if !seen_ids.insert(id) {
                    diagnostics.push(Diagnostic::error(
                        "G-037",
                        &path,
                        format!("duplicate assertion id '{id}'"),
                    ));
                }
            }

            check_assertion_expression_fields(assertion, &path, diagnostics);
        }
    }

    check_extension_prefixes(root, "", diagnostics);
}

/// G-038 / G-039: Assertion expression / fields recommendations.
fn check_assertion_expression_fields(
    assertion: &Value,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let assertion_type = assertion.get("type").and_then(Value::as_str).unwrap_or("");

    match assertion_type {
        "arithmetic" | "range" | "temporal" => {
            if assertion.get("expression").is_none() {
                diagnostics.push(Diagnostic::warning(
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
                diagnostics.push(Diagnostic::warning(
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

fn check_policy_parameters(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(params) = root.get("parameters").and_then(Value::as_object) {
        for (name, param) in params {
            let param_path = format!("/parameters/{name}");

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
            let binding_path = format!("/bindings/{key}");

            if let Some(id) = binding.get("id").and_then(Value::as_str) {
                if id != key {
                    diagnostics.push(Diagnostic::error(
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

    check_extension_prefixes(root, "", diagnostics);
}

/// Shared helper for G-047 and G-057.
fn check_values_ascending_effective_date(
    values: &[Value],
    path: &str,
    rule_id: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut prev_date: Option<&str> = None;
    for (i, entry) in values.iter().enumerate() {
        if let Some(date) = entry.get("effectiveDate").and_then(Value::as_str) {
            if let Some(prev) = prev_date {
                if date <= prev {
                    diagnostics.push(Diagnostic::error(
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
    diagnostics: &mut Vec<Diagnostic>,
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
        diagnostics.push(Diagnostic::error(
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
    diagnostics: &mut Vec<Diagnostic>,
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
            diagnostics.push(Diagnostic::error(
                "K-048",
                &format!("/caseFile/relationships/{i}/type"),
                format!("non-standard case relationship type '{rel_type}' must use 'x-' prefix"),
            ));
        }
    }
}

/// K-021: Provenance `actorId` MUST reference a declared kernel actor.
fn check_provenance_actor_ids_typed(kernel: &KernelDocument, diagnostics: &mut Vec<Diagnostic>) {
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
            diagnostics.push(Diagnostic::error(
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
fn check_actor_id_uniqueness_typed(kernel: &KernelDocument, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = std::collections::HashSet::new();

    for (index, actor) in kernel.actors.iter().enumerate() {
        if !seen.insert(actor.id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "K-009",
                &format!("/actors/{index}/id"),
                format!("duplicate actor id '{}'", actor.id),
            ));
        }
    }
}

/// CM-001: Entry template ids MUST be unique within the correspondence sidecar.
fn check_correspondence_metadata(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(entry_templates) = root.get("entryTemplates").and_then(Value::as_array) {
        let mut seen = std::collections::HashSet::new();

        for (index, template) in entry_templates.iter().enumerate() {
            let Some(id) = template.get("id").and_then(Value::as_str) else {
                continue;
            };

            if !seen.insert(id) {
                diagnostics.push(Diagnostic::error(
                    "CM-001",
                    &format!("/entryTemplates/{index}/id"),
                    format!("duplicate correspondence entry template id '{id}'"),
                ));
            }
        }
    }

    check_extension_prefixes(root, "", diagnostics);
}

/// G-058 / G-059: Business calendar structural validity.
fn check_business_calendar(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(holidays) = root.get("holidays").and_then(Value::as_array) {
        for (i, h) in holidays.iter().enumerate() {
            let path = format!("/holidays/{i}");
            let has_date = h.get("date").is_some();
            let has_rule = h.get("rule").is_some();
            if has_date == has_rule {
                diagnostics.push(Diagnostic::error(
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

    if let Some(oh) = root.get("operatingHours") {
        let start = oh.get("start").and_then(Value::as_str);
        let end = oh.get("end").and_then(Value::as_str);
        if let (Some(s), Some(e)) = (start, end) {
            let start_m = hh_mm_to_minutes(s);
            let end_m = hh_mm_to_minutes(e);
            match (start_m, end_m) {
                (Some(sm), Some(em)) if em <= sm => {
                    diagnostics.push(Diagnostic::error(
                        "G-059",
                        "/operatingHours/end",
                        "operating hours 'end' MUST be strictly after 'start'",
                    ));
                }
                (Some(_), Some(_)) => {}
                _ => {
                    diagnostics.push(Diagnostic::error(
                        "G-059",
                        "/operatingHours",
                        "operating hours 'start' and 'end' MUST be valid 24-hour HH:MM values",
                    ));
                }
            }
        }
    }

    check_extension_prefixes(root, "", diagnostics);
}

/// G-062 / G-065: Notification template content and section id uniqueness.
fn check_notification_template(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    let Some(templates) = root.get("templates").and_then(Value::as_object) else {
        check_extension_prefixes(root, "", diagnostics);
        return;
    };

    for (key, template) in templates {
        let base = format!("/templates/{key}");
        check_adverse_decision_template_sections(template, &base, diagnostics);
        check_template_section_id_uniqueness(template, &base, diagnostics);
    }

    check_extension_prefixes(root, "", diagnostics);
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
    diagnostics: &mut Vec<Diagnostic>,
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
        diagnostics.push(Diagnostic::error(
            "G-062",
            path,
            "adverse-decision template MUST include a section with id 'determination'",
        ));
    }
    if !has_reason_codes {
        diagnostics.push(Diagnostic::error(
            "G-062",
            path,
            "adverse-decision template MUST include reason code coverage (section id 'reasons', 'reasonCodes', or 'reason')",
        ));
    }
    if !has_appeal_rights {
        diagnostics.push(Diagnostic::error(
            "G-062",
            path,
            "adverse-decision template MUST include appeal rights (section id 'appealRights' or contentType 'appeal-rights')",
        ));
    }
    if !has_appeal_instructions {
        diagnostics.push(Diagnostic::error(
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
    diagnostics: &mut Vec<Diagnostic>,
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
            diagnostics.push(Diagnostic::error(
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
fn check_digest_algorithm(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    visit_all_objects(root, "", &mut |obj, obj_path| {
        if obj.contains_key("digest") {
            let has_algorithm = obj
                .get("extensions")
                .and_then(Value::as_object)
                .is_some_and(|ext| ext.contains_key("algorithm"));

            if !has_algorithm {
                diagnostics.push(Diagnostic::error(
                    "K-022",
                    obj_path,
                    "object has 'digest' but no 'algorithm' key in its extensions map",
                ));
            }
        }
    });
}

/// K-030: Extension keys must be x- prefixed.
fn check_extension_prefixes(value: &Value, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(obj) = value.as_object() {
        if let Some(extensions) = obj.get("extensions").and_then(Value::as_object) {
            let ext_path = if path.is_empty() {
                "/extensions".to_string()
            } else {
                format!("{path}/extensions")
            };
            for key in extensions.keys() {
                if !key.starts_with("x-") {
                    diagnostics.push(Diagnostic::error(
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
