// Rust guideline compliant 2026-02-21

//! Tier 1 lint rules — single-document structural checks.
//!
//! These rules examine one WOS document in isolation. They require no
//! cross-document resolution, no FEL parsing, and no runtime execution.
//! See LINT-MATRIX.md for the complete rule catalog.

use serde_json::Value;

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
        _ => {} // Other document types: no Tier 1 rules yet.
    }
}

// ---------------------------------------------------------------------------
// Kernel rules
// ---------------------------------------------------------------------------

fn check_kernel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(lifecycle) = root.get("lifecycle") {
        if let Some(states) = lifecycle.get("states").and_then(Value::as_object) {
            // K-006 fix: collect ALL state IDs across all nesting levels so
            // cross-scope transitions (substate targeting top-level state) are
            // not falsely flagged.
            let all_state_ids = collect_all_state_ids(states);
            for (name, state) in states {
                let path = format!("/lifecycle/states/{name}");
                check_state_type_semantics(state, &path, &all_state_ids, diagnostics);
            }
        }
    }

    check_set_data_paths(root, diagnostics);
    check_milestone_uniqueness(root, diagnostics);
    check_timer_exclusivity(root, diagnostics);
    check_digest_algorithm(root, diagnostics);
    check_case_relationship_type_prefix(root, diagnostics);
    check_extension_prefixes(root, "", diagnostics);
}

/// K-001: Final states MUST NOT have transitions.
/// K-002: Compound states MUST have initialState and states.
/// K-003: Parallel states MUST have regions.
/// K-004: cancellationPolicy only on parallel states.
/// K-005: historyState only on compound states.
/// K-007: Event names MUST NOT use $ prefix.
/// K-008: Parallel state outgoing transitions MUST use $join.
fn check_state_type_semantics(
    state: &Value,
    path: &str,
    all_state_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let state_type = state.get("type").and_then(Value::as_str).unwrap_or("");

    match state_type {
        "final" => {
            // K-001
            if state.get("transitions").is_some_and(|t| t.as_array().is_some_and(|a| !a.is_empty())) {
                diagnostics.push(Diagnostic::error(
                    "K-001",
                    path,
                    "final state must not have outgoing transitions",
                ));
            }
        }
        "compound" => {
            // K-002
            if state.get("initialState").is_none() {
                diagnostics.push(Diagnostic::error(
                    "K-002",
                    path,
                    "compound state must declare initialState",
                ));
            }
            if state.get("states").is_none() {
                diagnostics.push(Diagnostic::error(
                    "K-002",
                    path,
                    "compound state must declare substates in states map",
                ));
            }
        }
        "parallel" => {
            // K-003
            if state.get("regions").is_none() {
                diagnostics.push(Diagnostic::error(
                    "K-003",
                    path,
                    "parallel state must declare regions",
                ));
            }
        }
        _ => {}
    }

    // K-004: cancellationPolicy only on parallel
    if state.get("cancellationPolicy").is_some() && state_type != "parallel" {
        diagnostics.push(Diagnostic::error(
            "K-004",
            path,
            "cancellationPolicy is only valid on parallel states",
        ));
    }

    // K-005: historyState only on compound
    if state.get("historyState").is_some() && state_type != "compound" {
        diagnostics.push(Diagnostic::error(
            "K-005",
            path,
            "historyState is only valid on compound states",
        ));
    }

    // K-006: transition targets must reference existing states
    // K-007: event names must not use $ prefix
    // K-008: parallel outgoing transitions must use $join
    if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
        for (i, transition) in transitions.iter().enumerate() {
            let t_path = format!("{path}/transitions/{i}");

            // K-006
            if let Some(target) = transition.get("target").and_then(Value::as_str) {
                if !all_state_ids.contains(target) {
                    diagnostics.push(Diagnostic::error(
                        "K-006",
                        &t_path,
                        format!("transition target '{target}' does not exist in states map"),
                    ));
                }
            }

            // K-007
            if let Some(event) = transition.get("event").and_then(Value::as_str) {
                if event.starts_with('$') && event != "$join" {
                    diagnostics.push(Diagnostic::error(
                        "K-007",
                        &t_path,
                        format!("event name '{event}' uses reserved $ prefix"),
                    ));
                }
            }

            // K-008
            if state_type == "parallel" {
                if let Some(event) = transition.get("event").and_then(Value::as_str) {
                    if event != "$join" {
                        diagnostics.push(Diagnostic::error(
                            "K-008",
                            &t_path,
                            format!("parallel state outgoing transition must use '$join' event, found '{event}'"),
                        ));
                    }
                }
            }
        }
    }

    // Recurse into compound substates
    if let Some(substates) = state.get("states").and_then(Value::as_object) {
        for (name, substate) in substates {
            let sub_path = format!("{path}/states/{name}");
            check_state_type_semantics(substate, &sub_path, all_state_ids, diagnostics);
        }
    }

    // Recurse into parallel regions
    if let Some(regions) = state.get("regions").and_then(Value::as_object) {
        for (region_name, region) in regions {
            if let Some(region_states) = region.get("states").and_then(Value::as_object) {
                for (name, region_state) in region_states {
                    let r_path = format!("{path}/regions/{region_name}/states/{name}");
                    check_state_type_semantics(region_state, &r_path, all_state_ids, diagnostics);
                }
            }
        }
    }
}

// K-009: Actor identifiers must be unique.
// This rule is satisfied by the JSON object key uniqueness guarantee: the `actors`
// map is a JSON object, so duplicate keys are structurally impossible. No runtime
// check is required here.

/// K-015: setData path must reference a declared caseFile field.
fn check_set_data_paths(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let fields = root
        .pointer("/caseFile/fields")
        .and_then(Value::as_object);

    let Some(fields) = fields else { return };

    // Walk all actions in all states looking for setData
    visit_actions(root, &mut |action, action_path| {
        if action.get("action").and_then(Value::as_str) == Some("setData") {
            if let Some(path_val) = action.get("path").and_then(Value::as_str) {
                // Strip "caseFile." prefix if present
                let field_name = path_val.strip_prefix("caseFile.").unwrap_or(path_val);
                // Check top-level field name (dotted paths reference nested fields)
                let top_field = field_name.split('.').next().unwrap_or(field_name);
                if !fields.contains_key(top_field) {
                    diagnostics.push(Diagnostic::error(
                        "K-015",
                        action_path,
                        format!("setData path '{path_val}' references undeclared field '{top_field}'"),
                    ));
                }
            }
        }
    });
}

/// K-014: Milestone ids must be unique.
fn check_milestone_uniqueness(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let milestones = root
        .pointer("/lifecycle/milestones")
        .and_then(Value::as_object);

    let Some(milestones) = milestones else { return };

    // Milestones are a keyed map — inherently unique in JSON.
    // Check for empty ids.
    for (id, _) in milestones {
        if id.is_empty() {
            diagnostics.push(Diagnostic::error(
                "K-014",
                "/lifecycle/milestones",
                "milestone id must not be empty",
            ));
        }
    }
}

/// K-029: startTimer must specify exactly one of duration or deadline.
fn check_timer_exclusivity(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    visit_actions(root, &mut |action, action_path| {
        if action.get("action").and_then(Value::as_str) == Some("startTimer") {
            let has_duration = action.get("duration").is_some();
            let has_deadline = action.get("deadline").is_some();

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

/// K-022: A provenance digest field implies the hashing algorithm must be recorded in extensions.
///
/// When any provenance record or caseFile entry contains a `digest` field, an `algorithm` key
/// MUST appear in the same object's `extensions` map. Schema cannot enforce this inter-field
/// dependency because `digest` and `extensions.algorithm` are in separate locations.
fn check_digest_algorithm(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    // Walk the whole document tree looking for objects that have "digest".
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

/// K-048: Case relationship `type` non-standard values MUST use `x-` prefix.
///
/// The standard values are the enum values defined in the schema. Any additional
/// extension value must begin with `x-` to avoid collisions with future standard types.
fn check_case_relationship_type_prefix(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    /// Standard case relationship type values from the WOS Kernel spec S5.5.
    const STANDARD_RELATIONSHIP_TYPES: &[&str] = &[
        "parent",
        "child",
        "sibling",
        "related",
        "derived-from",
        "merged-into",
        "split-from",
    ];

    let relationships = root
        .pointer("/caseRelationships")
        .and_then(Value::as_array);

    let Some(relationships) = relationships else { return };

    for (i, rel) in relationships.iter().enumerate() {
        if let Some(rel_type) = rel.get("type").and_then(Value::as_str) {
            let is_standard = STANDARD_RELATIONSHIP_TYPES.contains(&rel_type);
            let is_extension = rel_type.starts_with("x-");

            if !is_standard && !is_extension {
                diagnostics.push(Diagnostic::error(
                    "K-048",
                    &format!("/caseRelationships/{i}/type"),
                    format!(
                        "non-standard case relationship type '{rel_type}' must use 'x-' prefix"
                    ),
                ));
            }
        }
    }
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

        // Recurse into all nested objects
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

// ---------------------------------------------------------------------------
// Governance rules
// ---------------------------------------------------------------------------

fn check_governance(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    check_delegation_dates(root, diagnostics);
    check_hold_expected_duration(root, diagnostics);
    check_extension_prefixes(root, "", diagnostics);
}

/// G-055: Hold policy `expectedDuration` must be a valid ISO 8601 duration or the literal "indefinite".
///
/// ISO 8601 duration strings begin with `P` (e.g., `P1Y`, `PT30M`). This rule rejects values
/// that are neither `"indefinite"` nor start with `P`.
fn check_hold_expected_duration(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let holds = root.get("holdPolicies").and_then(Value::as_array);
    let Some(holds) = holds else { return };

    for (i, hold) in holds.iter().enumerate() {
        if let Some(duration) = hold.get("expectedDuration").and_then(Value::as_str) {
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
}

// ---------------------------------------------------------------------------
// Assertion Library governance rules (called from check_assertion_library)
// ---------------------------------------------------------------------------

/// G-044: Delegation expirationDate must be after effectiveDate.
/// G-045: Delegation revokedDate must be on or after effectiveDate.
fn check_delegation_dates(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let delegations = root.get("delegations").and_then(Value::as_array);
    let Some(delegations) = delegations else { return };

    for (i, delegation) in delegations.iter().enumerate() {
        let path = format!("/delegations/{i}");
        let effective = delegation.get("effectiveDate").and_then(Value::as_str);
        let expiration = delegation.get("expirationDate").and_then(Value::as_str);
        let revoked = delegation.get("revokedDate").and_then(Value::as_str);

        // G-044
        if let (Some(eff), Some(exp)) = (effective, expiration) {
            if exp <= eff {
                diagnostics.push(Diagnostic::error(
                    "G-044",
                    &path,
                    format!("expirationDate '{exp}' must be after effectiveDate '{eff}'"),
                ));
            }
        }

        // G-045
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

    check_fallback_chain_termination(root, "fallbackChain", "/fallbackChain", diagnostics);

    // Check per-agent fallback chains too.
    if let Some(agents) = root.get("agents").and_then(Value::as_array) {
        for (i, agent) in agents.iter().enumerate() {
            if agent.get("fallbackChain").is_some() {
                let path = format!("/agents/{i}/fallbackChain");
                check_fallback_chain_termination(agent, "fallbackChain", &path, diagnostics);
            }
        }
    }

    // AI-046: rights-impacting workflow disclosure requirement.
    check_rights_impacting_disclosure(root, diagnostics);

    // AI-049: Narrative tier authoritative field must be false.
    check_narrative_tier_authoritative(root, diagnostics);

    check_extension_prefixes(root, "", diagnostics);
}

/// AI-046: When `impactLevel` is `rights-impacting`, `discloseThatAgentAssisted` MUST be `true`.
///
/// This is the within-document portion of the rule; the cross-document check (against the kernel's
/// `impactLevel`) is Tier 2. Here we check that any AI integration document that itself declares
/// `impactLevel: "rights-impacting"` carries the required disclosure flag.
fn check_rights_impacting_disclosure(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let impact_level = root.get("impactLevel").and_then(Value::as_str);
    if impact_level != Some("rights-impacting") {
        return;
    }

    let disclosed = root
        .get("discloseThatAgentAssisted")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !disclosed {
        diagnostics.push(Diagnostic::error(
            "AI-046",
            "/discloseThatAgentAssisted",
            "rights-impacting AI integration must set 'discloseThatAgentAssisted' to true",
        ));
    }
}

/// AI-049: Every Narrative provenance record must have `authoritative: false`.
///
/// Narrative tier records are explicitly non-authoritative. Any record in `narrativeProvenance`
/// (or in any `provenance` array entry whose `tier` is `"narrative"`) that omits or sets
/// `authoritative: true` is a conformance error.
fn check_narrative_tier_authoritative(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    // Check top-level narrativeProvenance array.
    if let Some(records) = root.get("narrativeProvenance").and_then(Value::as_array) {
        for (i, record) in records.iter().enumerate() {
            check_authoritative_false(record, &format!("/narrativeProvenance/{i}"), diagnostics);
        }
    }

    // Also check any generic provenance array entries whose tier is "narrative".
    if let Some(records) = root.get("provenance").and_then(Value::as_array) {
        for (i, record) in records.iter().enumerate() {
            if record.get("tier").and_then(Value::as_str) == Some("narrative") {
                check_authoritative_false(record, &format!("/provenance/{i}"), diagnostics);
            }
        }
    }
}

/// Check that a single provenance record's `authoritative` field is absent or explicitly `false`.
fn check_authoritative_false(record: &Value, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    match record.get("authoritative") {
        Some(Value::Bool(false)) => {
            // Correct: explicitly false.
        }
        None => {
            // Spec S13.3 says authoritative is REQUIRED. Absent is a warning.
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

/// AI-041: Fallback chain must terminate in escalateToHuman or fail; must not cycle.
fn check_fallback_chain_termination(
    parent: &Value,
    key: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let chain = parent.get(key).and_then(Value::as_array);
    let Some(chain) = chain else { return };

    if chain.is_empty() {
        return;
    }

    // Check terminal action
    if let Some(last) = chain.last() {
        let action = last.get("action").and_then(Value::as_str).unwrap_or("");
        if action != "escalateToHuman" && action != "fail" {
            diagnostics.push(Diagnostic::error(
                "AI-041",
                path,
                format!("fallback chain must terminate in 'escalateToHuman' or 'fail', found '{action}'"),
            ));
        }
    }

    // Check for cycles (duplicate action types — simplified check)
    let mut seen_alternate_agents = std::collections::HashSet::new();
    for (i, level) in chain.iter().enumerate() {
        if let Some(agent_ref) = level.get("alternateAgentRef").and_then(Value::as_str) {
            if !seen_alternate_agents.insert(agent_ref) {
                diagnostics.push(Diagnostic::error(
                    "AI-041",
                    &format!("{path}/{i}"),
                    format!("fallback chain cycles: alternateAgent '{agent_ref}' appears more than once"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion Library rules
// ---------------------------------------------------------------------------

fn check_assertion_library(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(assertions) = root.get("assertions").and_then(Value::as_array) {
        let mut seen_ids = std::collections::HashSet::new();
        for (i, assertion) in assertions.iter().enumerate() {
            let path = format!("/assertions/{i}");

            // G-037: Assertion ids must be unique.
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

/// G-038: arithmetic/range/temporal assertion types SHOULD have `expression`.
/// G-039: source-grounded/consistency assertion types SHOULD have `fields`.
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
                    format!(
                        "assertion of type '{assertion_type}' should include a 'fields' array"
                    ),
                ));
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Policy Parameters rules
// ---------------------------------------------------------------------------

fn check_policy_parameters(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    let root = &doc.value;

    if let Some(params) = root.get("parameters").and_then(Value::as_object) {
        for (name, param) in params {
            let param_path = format!("/parameters/{name}");

            // G-047: Parameter values must be in ascending effectiveDate order.
            if let Some(values) = param.get("values").and_then(Value::as_array) {
                check_values_ascending_effective_date(values, &format!("{param_path}/values"), "G-047", diagnostics);
            }

            // G-050: Each resolved value must be type-consistent with the parameter's declared type.
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

    // G-048: Binding id must match map key.
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

            // G-057: Binding values entries must be in ascending effectiveDate order.
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

/// Shared helper for G-047 and G-057: `values` array entries must be in ascending `effectiveDate` order.
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

/// G-050: Resolved parameter value must be type-consistent with the declared `type`.
///
/// Validates that the JSON type of the `value` field in a DateValue entry is consistent
/// with the ParameterDefinition's declared `type` string (e.g., "number", "boolean", "string").
fn check_parameter_value_type(
    entry: &Value,
    declared_type: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = entry.get("value") else { return };

    let type_matches = match declared_type {
        "number" | "integer" => value.is_number(),
        "boolean" => value.is_boolean(),
        "string" | "date" | "datetime" | "duration" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        // Unknown declared types cannot be validated statically.
        _ => true,
    };

    if !type_matches {
        let actual_kind = json_type_name(value);
        diagnostics.push(Diagnostic::error(
            "G-050",
            path,
            format!(
                "parameter value is {actual_kind} but declared type is '{declared_type}'"
            ),
        ));
    }
}

/// Return a human-readable JSON type name for use in diagnostics.
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
// Helpers
// ---------------------------------------------------------------------------

/// Collect all state identifiers across all nesting levels.
///
/// Walks compound substates and parallel regions recursively to build a
/// complete set of reachable state IDs. This prevents false-positive K-006
/// errors on cross-scope transitions (e.g., a substate targeting a top-level
/// state).
fn collect_all_state_ids(states: &serde_json::Map<String, Value>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    collect_state_ids_recursive(states, &mut ids);
    ids
}

fn collect_state_ids_recursive(
    states: &serde_json::Map<String, Value>,
    ids: &mut std::collections::HashSet<String>,
) {
    for (name, state) in states {
        ids.insert(name.clone());
        // Recurse into compound substates
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            collect_state_ids_recursive(substates, ids);
        }
        // Recurse into parallel regions
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for region in regions.values() {
                if let Some(region_states) = region.get("states").and_then(Value::as_object) {
                    collect_state_ids_recursive(region_states, ids);
                }
            }
        }
    }
}

/// Recursively visit every JSON object node in the document tree, calling `f` with the object
/// and its JSON pointer path.
fn visit_all_objects(value: &Value, path: &str, f: &mut dyn FnMut(&serde_json::Map<String, Value>, &str)) {
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

/// Walk all actions in a kernel document, calling `f` for each action found.
fn visit_actions(root: &Value, f: &mut dyn FnMut(&Value, &str)) {
    if let Some(lifecycle) = root.get("lifecycle") {
        if let Some(states) = lifecycle.get("states").and_then(Value::as_object) {
            for (name, state) in states {
                let path = format!("/lifecycle/states/{name}");
                visit_state_actions(state, &path, f);
            }
        }
    }
}

fn visit_state_actions(state: &Value, path: &str, f: &mut dyn FnMut(&Value, &str)) {
    // onEntry actions
    if let Some(actions) = state.get("onEntry").and_then(Value::as_array) {
        for (i, action) in actions.iter().enumerate() {
            f(action, &format!("{path}/onEntry/{i}"));
        }
    }

    // onExit actions
    if let Some(actions) = state.get("onExit").and_then(Value::as_array) {
        for (i, action) in actions.iter().enumerate() {
            f(action, &format!("{path}/onExit/{i}"));
        }
    }

    // Transition actions
    if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
        for (ti, transition) in transitions.iter().enumerate() {
            if let Some(actions) = transition.get("actions").and_then(Value::as_array) {
                for (ai, action) in actions.iter().enumerate() {
                    f(action, &format!("{path}/transitions/{ti}/actions/{ai}"));
                }
            }
        }
    }

    // Recurse into substates
    if let Some(substates) = state.get("states").and_then(Value::as_object) {
        for (name, substate) in substates {
            visit_state_actions(substate, &format!("{path}/states/{name}"), f);
        }
    }

    // Recurse into regions
    if let Some(regions) = state.get("regions").and_then(Value::as_object) {
        for (rname, region) in regions {
            if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                for (name, rstate) in rstates {
                    visit_state_actions(rstate, &format!("{path}/regions/{rname}/states/{name}"), f);
                }
            }
        }
    }
}
