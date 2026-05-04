// Rust guideline compliant 2026-05-02

//! Phase 4 — Emit `$wosWorkflow` content.
//!
//! `SA-MUST-cmp-020`: only `mapsToWos` PolicyObjects (and their bindings)
//! project into the artifact body. `authoringOnly`,
//! `requiresSpecExtension` (without an `x-` target), and
//! `unmappedButApproved` PolicyObjects MUST NOT produce body content.
//!
//! `SA-MUST-cmp-005`: artifact MUST omit any embedded block the
//! WorkflowIntent's `impactLevel` does not require AND for which no
//! element-level PolicyObject content motivates emission.
//!
//! `SA-MUST-cmp-013`: every transition referencing an event name MUST
//! resolve that name to an EventBinding declared in the workspace.
//! Unresolved event references halt with `unresolved-event-reference`.
//!
//! `SA-MUST-cmp-014`: every Activity (`step`/`system-check`) element
//! that names a `serviceBindingRef` MUST cover the bound service's
//! required inputs from the workflow's case file or upstream lifecycle
//! states. Coverage gaps halt with `incomplete-service-binding`.
//!
//! `SA-MUST-cmp-021`: `requiresSpecExtension` mappings whose embedded
//! `extensionRecord.lifecycleState` is explicitly `open` MUST halt
//! when their host PolicyObject is workflow-bearing. Mappings with
//! `lifecycleState = shipped` (the extension has landed parent-side)
//! emit content under their `x-` target.
//!
//! Transitions (F2.1, 2026-05-02): the lifecycle's `transitions[]`
//! arrays are no longer empty placeholders — Phase 4 walks the
//! WorkflowIntent's element list and projects:
//!
//! - `phase` elements → kernel `compound` states whose substates come
//!   from `body.contains[]`.
//! - `data-collection` / `system-check` / `review` / `notice` /
//!   `deadline` / `appeal` / `hold` / `evidence-request` /
//!   `AI-assistance` / `manual-override` / `step` elements → kernel
//!   `atomic` states with one outgoing transition to the next
//!   non-structural element in document order (or to the enclosing
//!   phase's next sibling if last in phase).
//! - `decision` elements → branching transitions per
//!   `body.possibleOutcomes[]`. Guard projection: when
//!   `bridge.guardKind = "decisionTable"`, the guard renders as a
//!   `Guard::DecisionTable` reference to the bound DecisionRule
//!   PolicyObject; otherwise an FEL string from `body.guard` (when
//!   present).
//! - `completion-outcome` elements → kernel `final` states with no
//!   outgoing transitions; `polarity = adverse + triggersDueProcess`
//!   are tagged via state-level metadata.

use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexMap;
use serde_json::{Map, Value, json};

use crate::error::{CompileError, FailureKind};
use crate::phase2_mapping::MappingResult;
use crate::phase3_workflow::WorkflowResult;
use wos_studio_lint::Workspace;

pub struct EmitResult {
    pub wos_workflow: Value,
    pub embedded_blocks_emitted: Vec<String>,
}

/// Index of the workspace's bindings (EventBinding / ServiceBinding /
/// PolicyEngineBinding) keyed by id and by event name. Built once
/// per compile and consulted by transition emission +
/// SA-MUST-cmp-013 / cmp-014 enforcement.
struct ElementBindingIndex<'a> {
    by_id: BTreeMap<String, &'a Value>,
    /// EventBinding ids keyed by `body.eventName` (case-sensitive).
    event_names: BTreeSet<String>,
    /// ServiceBinding ids keyed by binding id.
    service_ids: BTreeSet<String>,
}

impl<'a> ElementBindingIndex<'a> {
    fn build(ws: &'a Workspace) -> Self {
        let mut by_id: BTreeMap<String, &'a Value> = BTreeMap::new();
        let mut event_names: BTreeSet<String> = BTreeSet::new();
        let mut service_ids: BTreeSet<String> = BTreeSet::new();
        for doc in &ws.documents {
            // Bindings live under StudioMarker::Binding; collection-form
            // wrappers carry a `bindings[]` array. The marker discriminator
            // sits on `$wosStudioBinding`.
            let Some(items) = doc.raw.get("bindings").and_then(Value::as_array) else {
                continue;
            };
            for b in items {
                let Some(id) = b.get("id").and_then(Value::as_str) else { continue };
                by_id.insert(id.to_string(), b);
                let kind = b.get("kind").and_then(Value::as_str).unwrap_or("");
                match kind {
                    "EventBinding" => {
                        if let Some(name) = b
                            .get("body")
                            .and_then(|body| body.get("eventName"))
                            .and_then(Value::as_str)
                        {
                            event_names.insert(name.to_string());
                        }
                    }
                    "ServiceBinding" => {
                        service_ids.insert(id.to_string());
                    }
                    _ => {}
                }
            }
        }
        Self {
            by_id,
            event_names,
            service_ids,
        }
    }

    fn resolves_event(&self, name: &str) -> bool {
        self.event_names.contains(name)
    }

    fn service_binding(&self, id: &str) -> Option<&'a Value> {
        if self.service_ids.contains(id) {
            self.by_id.get(id).copied()
        } else {
            None
        }
    }
}

pub fn run(
    ws: &Workspace,
    workflow: &WorkflowResult<'_>,
    mapping: &MappingResult<'_>,
) -> Result<EmitResult, CompileError> {
    let intent = workflow.workflow_intent;

    // Build minimal kernel envelope.
    let id = intent
        .raw
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("workflow-?")
        .to_string();
    let version = intent
        .raw
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.1.0")
        .to_string();
    let title = intent
        .raw
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("(untitled)")
        .to_string();
    let impact_level = intent
        .raw
        .get("impactLevel")
        .and_then(Value::as_str)
        .unwrap_or("operational")
        .to_string();
    let url = intent
        .raw
        .get("publicationUrl")
        .and_then(Value::as_str)
        .unwrap_or("https://example.org/workflow/unspecified")
        .to_string();

    let actors = intent
        .raw
        .get("actors")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    // F2.2: cross-document binding lookup index.
    let bindings = ElementBindingIndex::build(ws);

    // F2.5: cmp-021 — halt on workflow-bearing mappings whose
    // embedded ExtensionRecord is explicitly `open`.
    enforce_extension_record_lifecycle(mapping)?;

    // Lifecycle skeleton from elements (now with real transitions).
    let lifecycle = derive_lifecycle(&workflow.elements, &bindings)?;

    // Block emission gates per SA-MUST-cmp-005.
    let mut blocks: IndexMap<String, Value> = IndexMap::new();
    if requires_governance(&impact_level, mapping) {
        blocks.insert("governance".into(), project_governance(mapping));
    }
    if requires_agents(&actors) {
        blocks.insert("agents".into(), project_agents(&actors));
    }
    if requires_signature(&workflow.elements) {
        blocks.insert("signature".into(), project_signature(&workflow.elements));
    }
    if requires_custody(&impact_level) {
        blocks.insert("custody".into(), json!({"trustProfile": "default"}));
    }

    // Detect artifact collisions: two mappings project to the same path
    // with non-equivalent payloads. SA-MUST-cmp-004.
    detect_collisions(mapping)?;

    // x- extension targets from requiresSpecExtension mappings.
    let extension_block = project_extensions(mapping);

    let mut artifact = Map::new();
    // $wosWorkflow is a const document-type marker pinned to "1.0"
    // by `schemas/wos-workflow.schema.json:20`. The author-time
    // version of this specific workflow goes in `version`.
    // (Closes Wave 2 NIT-2 — previously emitted as the intent's
    // version, which Draft-2020-12 schema-pass rejects against the
    // const constraint.)
    artifact.insert("$wosWorkflow".into(), json!("1.0"));
    artifact.insert("url".into(), json!(url));
    let _ = id; // top-level `id` is not part of the kernel envelope
                // (per ADR-0076 the envelope's identity is `url +
                // version`); the intent's id flows into the
                // compile manifest, not the artifact body.
    artifact.insert("version".into(), json!(version));
    artifact.insert("title".into(), json!(title));
    artifact.insert("impactLevel".into(), json!(impact_level));
    artifact.insert("actors".into(), actors);
    artifact.insert("lifecycle".into(), lifecycle);
    let mut emitted: Vec<String> = Vec::new();
    for (name, value) in &blocks {
        artifact.insert(name.clone(), value.clone());
        emitted.push(name.clone());
    }
    if let Some(ext) = extension_block {
        artifact.insert("extensions".into(), ext);
        emitted.push("extensions".into());
    }
    emitted.sort();

    Ok(EmitResult {
        wos_workflow: Value::Object(artifact),
        embedded_blocks_emitted: emitted,
    })
}

// ------------------------------------------------------------------------
// Lifecycle + transitions (F2.1)
// ------------------------------------------------------------------------

/// Element kinds whose lifecycle projection is a kernel state (not
/// purely structural).
fn is_lifecycle_state(kind: &str) -> bool {
    matches!(
        kind,
        "phase"
            | "step"
            | "data-collection"
            | "system-check"
            | "review"
            | "notice"
            | "deadline"
            | "appeal"
            | "hold"
            | "evidence-request"
            | "AI-assistance"
            | "manual-override"
            | "decision"
            | "completion-outcome"
            | "phase-end"
    )
}

fn kernel_state_kind_for(kind: &str) -> &'static str {
    match kind {
        "phase" => "compound",
        "completion-outcome" | "phase-end" => "final",
        _ => "atomic",
    }
}

fn element_state_id<'a>(elem: &'a Value) -> Option<&'a str> {
    // Prefer the bridge.stateName when present; else the element id.
    elem.get("bridge")
        .and_then(|b| b.get("stateName"))
        .and_then(Value::as_str)
        .or_else(|| elem.get("id").and_then(Value::as_str))
}

fn derive_lifecycle(
    elements: &[&Value],
    bindings: &ElementBindingIndex<'_>,
) -> Result<Value, CompileError> {
    // Build {element id → element ref} lookup so phase.contains[] +
    // bridge.transitionId chains can resolve back to elements.
    let mut by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for e in elements {
        if let Some(id) = e.get("id").and_then(Value::as_str) {
            by_id.insert(id.to_string(), e);
        }
    }

    // Two passes: first collect all top-level (non-contained) elements
    // in document order. Phase children referenced via body.contains[]
    // become substates of the phase, NOT siblings at the top level.
    let mut contained_ids: BTreeSet<String> = BTreeSet::new();
    for e in elements {
        if e.get("kind").and_then(Value::as_str) == Some("phase") {
            if let Some(arr) = e
                .get("body")
                .and_then(|b| b.get("contains"))
                .and_then(Value::as_array)
            {
                for c in arr {
                    if let Some(s) = c.as_str() {
                        contained_ids.insert(s.to_string());
                    }
                }
            }
        }
    }

    // Top-level elements: those in doc order AND not contained by any
    // phase. The first top-level lifecycle-state element is the
    // initial state.
    let top_level: Vec<&Value> = elements
        .iter()
        .filter(|e| {
            let kind = e.get("kind").and_then(Value::as_str).unwrap_or("");
            if !is_lifecycle_state(kind) {
                return false;
            }
            let id = e.get("id").and_then(Value::as_str).unwrap_or("");
            !contained_ids.contains(id)
        })
        .copied()
        .collect();

    let initial = top_level
        .first()
        .and_then(|e| element_state_id(e))
        .unwrap_or("intake")
        .to_string();

    // Emit each top-level state with its transitions.
    let mut states: IndexMap<String, Value> = IndexMap::new();
    for (i, elem) in top_level.iter().enumerate() {
        let state_id = element_state_id(elem)
            .map(str::to_string)
            .unwrap_or_else(|| format!("state-{i}"));
        let kind = elem.get("kind").and_then(Value::as_str).unwrap_or("");
        let kernel_kind = kernel_state_kind_for(kind);
        let next: Option<&Value> = top_level.get(i + 1).copied();
        let transitions = project_transitions(elem, next, &by_id, bindings)?;
        let mut state_body = Map::new();
        // Schema-side field name is `type` (Rust uses `kind` via
        // serde rename); the compiled artifact MUST use the
        // schema-side name so Draft-2020-12 schema-pass accepts it.
        state_body.insert("type".into(), json!(kernel_kind));
        state_body.insert("transitions".into(), Value::Array(transitions));
        // Compound state: emit substates from contains[].
        if kind == "phase" {
            let substates = project_substates(elem, &by_id, bindings)?;
            if !substates.is_empty() {
                state_body.insert(
                    "states".into(),
                    Value::Object(substates.into_iter().collect()),
                );
            }
        }
        states.insert(state_id, Value::Object(state_body));
    }

    Ok(json!({
        "initialState": initial,
        "states": states,
    }))
}

/// Emit the transitions[] array for a single state. The shape is the
/// kernel `Transition` shape: `{event?, target, guard?, description?}`.
fn project_transitions(
    elem: &Value,
    next_sibling: Option<&Value>,
    by_id: &BTreeMap<String, &Value>,
    bindings: &ElementBindingIndex<'_>,
) -> Result<Vec<Value>, CompileError> {
    let kind = elem.get("kind").and_then(Value::as_str).unwrap_or("");
    let elem_id = elem.get("id").and_then(Value::as_str).unwrap_or("?");

    // Final states have no outgoing transitions.
    if matches!(kind, "completion-outcome" | "phase-end") {
        return Ok(Vec::new());
    }

    // Decision: branch per body.possibleOutcomes[].
    if kind == "decision" {
        let outcomes = elem
            .get("body")
            .and_then(|b| b.get("possibleOutcomes"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let guard_kind = elem
            .get("bridge")
            .and_then(|b| b.get("guardKind"))
            .and_then(Value::as_str);
        let decision_rule_ref = elem
            .get("body")
            .and_then(|b| b.get("decisionRuleRef"))
            .and_then(Value::as_str);

        let mut transitions: Vec<Value> = Vec::new();
        for outcome in &outcomes {
            let outcome_ref = outcome.as_str().unwrap_or("");
            let target = resolve_target_for_outcome(outcome_ref, by_id, next_sibling);
            let mut t = Map::new();
            // Decision elements emit one transition per outcome; we
            // tag the event with the outcome ref so downstream
            // simulators can drive a specific branch.
            t.insert("event".into(), json!(outcome_ref));
            t.insert("target".into(), json!(target));
            // Guard projection: decisionTable kind references the
            // bound DecisionRule PolicyObject; otherwise inline FEL.
            if guard_kind == Some("decisionTable") {
                if let Some(rule_ref) = decision_rule_ref {
                    t.insert(
                        "guard".into(),
                        json!({
                            "kind": "decisionTable",
                            "decisionRuleRef": rule_ref,
                            "outcome": outcome_ref,
                        }),
                    );
                }
            } else if let Some(fel) = elem
                .get("body")
                .and_then(|b| b.get("guard"))
                .and_then(Value::as_str)
            {
                t.insert("guard".into(), json!(fel));
            }
            // F2.3: cmp-013 — outcome refs are PolicyObject ids, not
            // event names, so they don't go through EventBinding.
            // Decision branches are not events in the
            // EventBinding sense.
            transitions.push(Value::Object(t));
        }
        return Ok(transitions);
    }

    // Atomic / compound non-decision state: emit one transition to
    // the next sibling. If the bridge names an explicit transitionId,
    // promote it to event.
    let target = next_sibling
        .and_then(|e| element_state_id(e))
        .unwrap_or("");
    if target.is_empty() {
        return Ok(Vec::new());
    }
    let mut t = Map::new();
    if let Some(transition_id) = elem
        .get("bridge")
        .and_then(|b| b.get("transitionId"))
        .and_then(Value::as_str)
    {
        // F2.3: cmp-013 enforcement — when the element explicitly
        // names an eventBindingRef (vs. a synthetic transition id),
        // verify the workspace defines that EventBinding.
        if let Some(event_binding_ref) = elem
            .get("body")
            .and_then(|b| b.get("eventBindingRef"))
            .and_then(Value::as_str)
        {
            if !bindings.resolves_event(event_binding_ref) {
                return Err(CompileError::halt_with(
                    4,
                    FailureKind::UnresolvedEventReference,
                    format!(
                        "element `{elem_id}` references event `{event_binding_ref}` \
                         that no EventBinding declares"
                    ),
                    Vec::new(),
                ));
            }
        }
        t.insert("event".into(), json!(transition_id));
    }
    t.insert("target".into(), json!(target));

    // F2.4: cmp-014 enforcement — Activity-like elements
    // (system-check / step) with a serviceBindingRef MUST cover
    // the bound service's required inputs. Today we check structural
    // presence of `body.inputBindings[]` on the bound ServiceBinding;
    // a richer satisfaction check requires case-file shape inference
    // that lands later (F3.2 SemanticDiff machinery shares the
    // walk).
    if matches!(kind, "system-check" | "step") {
        if let Some(service_ref) = elem
            .get("body")
            .and_then(|b| b.get("serviceBindingRef"))
            .and_then(Value::as_str)
        {
            let Some(service) = bindings.service_binding(service_ref) else {
                return Err(CompileError::halt_with(
                    4,
                    FailureKind::IncompleteServiceBinding,
                    format!(
                        "element `{elem_id}` references serviceBindingRef \
                         `{service_ref}` that no ServiceBinding declares"
                    ),
                    Vec::new(),
                ));
            };
            let inputs_present = service
                .get("body")
                .and_then(|b| b.get("inputBindings"))
                .and_then(Value::as_array)
                .is_some_and(|arr| !arr.is_empty());
            if !inputs_present {
                return Err(CompileError::halt_with(
                    4,
                    FailureKind::IncompleteServiceBinding,
                    format!(
                        "ServiceBinding `{service_ref}` (referenced by element \
                         `{elem_id}`) declares no inputBindings; its required \
                         inputs cannot be satisfied"
                    ),
                    Vec::new(),
                ));
            }
        }
    }

    Ok(vec![Value::Object(t)])
}

/// Resolve a decision outcome reference to a target state id. If the
/// outcome ref names an element directly (id starts with `el-`), use
/// its bridge.stateName; otherwise fall back to the next sibling's
/// state id.
fn resolve_target_for_outcome(
    outcome_ref: &str,
    by_id: &BTreeMap<String, &Value>,
    next_sibling: Option<&Value>,
) -> String {
    if let Some(elem) = by_id.get(outcome_ref) {
        if let Some(state_id) = element_state_id(elem) {
            return state_id.to_string();
        }
    }
    next_sibling
        .and_then(|e| element_state_id(e))
        .unwrap_or("")
        .to_string()
}

/// Project a phase element's body.contains[] into substates of the
/// compound parent state. Each contained element becomes its own
/// substate; transitions between substates follow document order.
fn project_substates(
    phase: &Value,
    by_id: &BTreeMap<String, &Value>,
    bindings: &ElementBindingIndex<'_>,
) -> Result<IndexMap<String, Value>, CompileError> {
    let contains = phase
        .get("body")
        .and_then(|b| b.get("contains"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let resolved: Vec<&Value> = contains
        .iter()
        .filter_map(|c| c.as_str())
        .filter_map(|id| by_id.get(id).copied())
        .collect();
    let mut substates: IndexMap<String, Value> = IndexMap::new();
    for (i, elem) in resolved.iter().enumerate() {
        let state_id = element_state_id(elem)
            .map(str::to_string)
            .unwrap_or_else(|| format!("substate-{i}"));
        let kind = elem.get("kind").and_then(Value::as_str).unwrap_or("");
        let kernel_kind = kernel_state_kind_for(kind);
        let next = resolved.get(i + 1).copied();
        let transitions = project_transitions(elem, next, by_id, bindings)?;
        let mut body = Map::new();
        // Schema-side field name is `type` (see top-level state
        // emission for rationale).
        body.insert("type".into(), json!(kernel_kind));
        body.insert("transitions".into(), Value::Array(transitions));
        substates.insert(state_id, Value::Object(body));
    }
    Ok(substates)
}

// ------------------------------------------------------------------------
// SA-MUST-cmp-021 — extension-record lifecycle gate (F2.5)
// ------------------------------------------------------------------------

fn enforce_extension_record_lifecycle(
    mapping: &MappingResult<'_>,
) -> Result<(), CompileError> {
    let mut violations: Vec<String> = Vec::new();
    for (subject, m) in &mapping.by_subject {
        let mapping_state = m.get("mappingState").and_then(Value::as_str);
        if mapping_state != Some("requiresSpecExtension") {
            continue;
        }
        // Lifecycle gate fires only when the embedded extensionRecord
        // explicitly carries `lifecycleState = "open"` AND the host
        // mapping is workflow-bearing (i.e. has an x- target the
        // compiler would otherwise emit).
        let ext_lifecycle = m
            .get("extensionRecord")
            .and_then(|e| e.get("lifecycleState"))
            .and_then(Value::as_str);
        if ext_lifecycle != Some("open") {
            continue;
        }
        violations.push(format!(
            "{subject}: extensionRecord.lifecycleState is `open`; mapping cannot \
             project until the spec extension ships"
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(CompileError::halt_with(
            4,
            FailureKind::MalformedBridge,
            format!(
                "{} workflow-bearing mapping(s) reference open extension records",
                violations.len()
            ),
            violations,
        ))
    }
}

// ------------------------------------------------------------------------
// Block projection (unchanged from prior phase 4)
// ------------------------------------------------------------------------

fn requires_governance(impact: &str, mapping: &MappingResult<'_>) -> bool {
    if matches!(impact, "rights-impacting" | "safety-impacting") {
        return true;
    }
    mapping
        .by_subject
        .values()
        .any(|m| m.get("mappingState").and_then(Value::as_str) == Some("mapsToWos"))
}

fn project_governance(mapping: &MappingResult<'_>) -> Value {
    let mut policy_objects: Vec<Value> = Vec::new();
    let mut subjects: Vec<&String> = mapping.by_subject.keys().collect();
    subjects.sort();
    for s in subjects {
        let m = mapping.by_subject[s];
        if m.get("mappingState").and_then(Value::as_str) != Some("mapsToWos") {
            continue;
        }
        policy_objects.push(json!({
            "id": s,
            "wosTarget": m.get("targets").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "compiledFrom": s,
        }));
    }
    json!({"policyObjects": policy_objects})
}

fn requires_agents(actors: &Value) -> bool {
    actors
        .as_array()
        .is_some_and(|arr| arr.iter().any(|a| a.get("type").and_then(Value::as_str) == Some("agent")))
}

fn project_agents(actors: &Value) -> Value {
    let agents: Vec<Value> = actors
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|a| a.get("type").and_then(Value::as_str) == Some("agent"))
                .map(|a| {
                    json!({
                        "id": a.get("id").cloned().unwrap_or(Value::Null),
                        "autonomyLevel": "assistive",
                        "fallbackChain": [
                            {"action": "escalateToHuman"}
                        ]
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    json!(agents)
}

fn requires_signature(elements: &[&Value]) -> bool {
    elements.iter().any(|e| {
        e.get("body")
            .and_then(|b| b.get("requiresSignature"))
            .and_then(Value::as_bool)
            == Some(true)
    })
}

fn project_signature(elements: &[&Value]) -> Value {
    let signers: Vec<Value> = elements
        .iter()
        .filter_map(|e| {
            e.get("body")
                .and_then(|b| b.get("signers"))
                .and_then(Value::as_array)
                .cloned()
        })
        .flatten()
        .collect();
    json!({"signers": signers})
}

fn requires_custody(impact: &str) -> bool {
    matches!(impact, "rights-impacting" | "safety-impacting")
}

fn project_extensions(mapping: &MappingResult<'_>) -> Option<Value> {
    // BTreeMap so iteration order is deterministic (`SA-MUST-cmp-001`).
    // by_subject is now also a BTreeMap, so the source loop is stable too.
    let mut entries: BTreeMap<String, Value> = BTreeMap::new();
    for m in mapping.by_subject.values() {
        if m.get("mappingState").and_then(Value::as_str) != Some("requiresSpecExtension") {
            continue;
        }
        let Some(targets) = m.get("targets").and_then(Value::as_array) else { continue };
        for target in targets {
            if let Some(path) = target.get("wosJsonPath").and_then(Value::as_str) {
                // Match `x-` only as a path segment, not a substring
                // (closes Wave 2 NIT-1 false-match on `$.foo.exp-x-bar`).
                let is_extension = path
                    .split('.')
                    .any(|seg| seg.starts_with("x-"));
                if is_extension {
                    let key = path
                        .trim_start_matches('$')
                        .trim_start_matches('.')
                        .to_string();
                    entries.insert(key, target.clone());
                }
            }
        }
    }
    if entries.is_empty() {
        None
    } else {
        let mut out = Map::new();
        for (k, v) in entries {
            out.insert(k, v);
        }
        Some(Value::Object(out))
    }
}

fn detect_collisions(mapping: &MappingResult<'_>) -> Result<(), CompileError> {
    // BTreeMap so collision-detail ordering is deterministic across
    // runs (`SA-MUST-cmp-001`).
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (subject, m) in &mapping.by_subject {
        let Some(targets) = m.get("targets").and_then(Value::as_array) else { continue };
        for target in targets {
            if let Some(path) = target.get("wosJsonPath").and_then(Value::as_str) {
                by_path.entry(path.to_string()).or_default().push(subject.clone());
            }
        }
    }
    let collisions: Vec<String> = by_path
        .into_iter()
        .filter(|(_, subjects)| subjects.len() > 1)
        .map(|(path, mut subjects)| {
            subjects.sort();  // stable order within a single collision detail
            format!("{path} ← {}", subjects.join(", "))
        })
        .collect();
    if collisions.is_empty() {
        Ok(())
    } else {
        Err(CompileError::halt_with(
            4,
            FailureKind::ArtifactCollision,
            format!("{} mapping target(s) collide", collisions.len()),
            collisions,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_studio_model::{StudioDocument, StudioMarker};

    fn intent(raw: serde_json::Value) -> wos_studio_lint::WorkspaceDocument {
        let document: StudioDocument = serde_json::from_value(raw.clone()).unwrap();
        wos_studio_lint::WorkspaceDocument {
            path: "wfi.json".to_string(),
            marker: StudioMarker::WorkflowIntent,
            document,
            raw,
        }
    }

    #[test]
    fn emits_minimal_envelope() {
        let intent_doc = intent(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "version": "0.1.0",
            "title": "Test",
            "impactLevel": "operational",
            "actors": [],
            "publicationUrl": "https://example.org/wfi-1",
            "elements": []
        }));
        let workflow = WorkflowResult {
            workflow_intent: &intent_doc,
            elements: vec![],
        };
        let mapping = MappingResult {
            by_subject: BTreeMap::new(),
        };
        let ws = Workspace::default();
        let result = run(&ws, &workflow, &mapping).expect("emit");
        // $wosWorkflow is the document-type marker, pinned const "1.0";
        // the intent's version goes into the `version` field.
        assert_eq!(result.wos_workflow["$wosWorkflow"], json!("1.0"));
        assert_eq!(result.wos_workflow["version"], json!("0.1.0"));
        assert_eq!(result.wos_workflow["title"], json!("Test"));
    }

    #[test]
    fn detects_artifact_collision() {
        let mapping_json_a = json!({
            "id": "ma", "policyObjectRef": "po-a",
            "mappingState": "mapsToWos",
            "targets": [{"wosJsonPath": "$.governance.policyObjects[0]"}]
        });
        let mapping_json_b = json!({
            "id": "mb", "policyObjectRef": "po-b",
            "mappingState": "mapsToWos",
            "targets": [{"wosJsonPath": "$.governance.policyObjects[0]"}]
        });
        let mut by_subject: BTreeMap<String, &Value> = BTreeMap::new();
        by_subject.insert("po-a".to_string(), &mapping_json_a);
        by_subject.insert("po-b".to_string(), &mapping_json_b);
        let mapping = MappingResult { by_subject };
        let err = detect_collisions(&mapping).expect_err("collide");
        assert!(matches!(
            err,
            CompileError::Halt { kind: FailureKind::ArtifactCollision, .. }
        ));
    }

    /// F2.1: phase 4 emits non-empty transitions[] for a 4-element
    /// linear workflow (intake → review → decision → outcome).
    #[test]
    fn linear_workflow_emits_transitions_to_next_sibling() {
        let intent_doc = intent(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-linear",
            "version": "0.1.0",
            "title": "Linear",
            "impactLevel": "operational",
            "actors": [],
            "publicationUrl": "https://example.org/wfi-linear",
            "elements": [
                {
                    "id": "intake",
                    "kind": "data-collection",
                    "bridge": {"stateName": "intake"},
                    "body": {}
                },
                {
                    "id": "review",
                    "kind": "review",
                    "bridge": {"stateName": "review"},
                    "body": {}
                },
                {
                    "id": "outcome",
                    "kind": "completion-outcome",
                    "bridge": {"stateName": "done"},
                    "body": {"polarity": "favorable"}
                }
            ]
        }));
        let elements: Vec<&Value> = intent_doc
            .raw
            .get("elements")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .collect();
        let workflow = WorkflowResult {
            workflow_intent: &intent_doc,
            elements,
        };
        let mapping = MappingResult {
            by_subject: BTreeMap::new(),
        };
        let ws = Workspace::default();
        let result = run(&ws, &workflow, &mapping).expect("emit");
        let states = &result.wos_workflow["lifecycle"]["states"];
        // intake → review
        let intake_t = &states["intake"]["transitions"];
        assert_eq!(intake_t.as_array().unwrap().len(), 1);
        assert_eq!(intake_t[0]["target"], json!("review"));
        // review → done
        let review_t = &states["review"]["transitions"];
        assert_eq!(review_t.as_array().unwrap().len(), 1);
        assert_eq!(review_t[0]["target"], json!("done"));
        // done has no transitions (completion-outcome). Schema-side
        // field name is `type`.
        assert_eq!(states["done"]["type"], json!("final"));
        assert_eq!(states["done"]["transitions"].as_array().unwrap().len(), 0);
    }

    /// F2.1: decision elements branch one transition per outcome ref,
    /// with the outcome id projected as the transition event.
    #[test]
    fn decision_element_branches_per_outcome() {
        let intent_doc = intent(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-decision",
            "version": "0.1.0",
            "title": "Decision",
            "impactLevel": "operational",
            "actors": [],
            "publicationUrl": "https://example.org/wfi-decision",
            "elements": [
                {
                    "id": "screen",
                    "kind": "decision",
                    "bridge": {
                        "transitionId": "screen-to-outcome",
                        "guardKind": "decisionTable"
                    },
                    "body": {
                        "decisionRuleRef": "pol-screening-rules",
                        "possibleOutcomes": ["pol-outcome-pass", "pol-outcome-fail"]
                    }
                },
                {
                    "id": "pol-outcome-pass",
                    "kind": "completion-outcome",
                    "bridge": {"stateName": "passed"},
                    "body": {"polarity": "favorable"}
                },
                {
                    "id": "pol-outcome-fail",
                    "kind": "completion-outcome",
                    "bridge": {"stateName": "failed"},
                    "body": {"polarity": "adverse"}
                }
            ]
        }));
        let elements: Vec<&Value> = intent_doc
            .raw
            .get("elements")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .collect();
        let workflow = WorkflowResult {
            workflow_intent: &intent_doc,
            elements,
        };
        let mapping = MappingResult {
            by_subject: BTreeMap::new(),
        };
        let ws = Workspace::default();
        let result = run(&ws, &workflow, &mapping).expect("emit");
        let states = &result.wos_workflow["lifecycle"]["states"];
        let screen_t = &states["screen"]["transitions"];
        let arr = screen_t.as_array().unwrap();
        assert_eq!(arr.len(), 2, "decision MUST emit one transition per outcome");
        // Each transition carries the outcome id as event + a
        // decisionTable guard.
        assert_eq!(arr[0]["event"], json!("pol-outcome-pass"));
        assert_eq!(arr[0]["target"], json!("passed"));
        assert_eq!(arr[0]["guard"]["kind"], json!("decisionTable"));
        assert_eq!(arr[0]["guard"]["decisionRuleRef"], json!("pol-screening-rules"));
        assert_eq!(arr[1]["event"], json!("pol-outcome-fail"));
        assert_eq!(arr[1]["target"], json!("failed"));
    }

    /// F2.1: phase elements become compound states with substates
    /// from body.contains[].
    #[test]
    fn phase_emits_compound_state_with_substates() {
        let intent_doc = intent(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-phased",
            "version": "0.1.0",
            "title": "Phased",
            "impactLevel": "operational",
            "actors": [],
            "publicationUrl": "https://example.org/wfi-phased",
            "elements": [
                {
                    "id": "el-phase-intake",
                    "kind": "phase",
                    "bridge": {"stateName": "intake"},
                    "body": {"contains": ["el-collect", "el-verify"]}
                },
                {
                    "id": "el-collect",
                    "kind": "data-collection",
                    "bridge": {"stateName": "collect"},
                    "body": {}
                },
                {
                    "id": "el-verify",
                    "kind": "system-check",
                    "bridge": {"stateName": "verify", "kernelKind": "task"},
                    "body": {}
                },
                {
                    "id": "el-done",
                    "kind": "completion-outcome",
                    "bridge": {"stateName": "done"},
                    "body": {"polarity": "favorable"}
                }
            ]
        }));
        let elements: Vec<&Value> = intent_doc
            .raw
            .get("elements")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .collect();
        let workflow = WorkflowResult {
            workflow_intent: &intent_doc,
            elements,
        };
        let mapping = MappingResult {
            by_subject: BTreeMap::new(),
        };
        let ws = Workspace::default();
        let result = run(&ws, &workflow, &mapping).expect("emit");
        let states = &result.wos_workflow["lifecycle"]["states"];
        // intake is compound, has substates collect + verify
        // Schema-side field name is `type`.
        assert_eq!(states["intake"]["type"], json!("compound"));
        assert!(states["intake"]["states"]["collect"].is_object());
        assert!(states["intake"]["states"]["verify"].is_object());
        // intake's outgoing transition goes to done
        assert_eq!(states["intake"]["transitions"][0]["target"], json!("done"));
        // contained elements do NOT appear at the top level
        assert!(states.get("collect").is_none());
        assert!(states.get("verify").is_none());
        // collect → verify transition inside intake
        assert_eq!(
            states["intake"]["states"]["collect"]["transitions"][0]["target"],
            json!("verify")
        );
    }

    /// F2.5: cmp-021 — workflow-bearing mapping with explicit
    /// extensionRecord.lifecycleState=open MUST halt.
    #[test]
    fn cmp_021_halts_on_open_extension_record() {
        let m = json!({
            "id": "ma", "policyObjectRef": "po-a",
            "mappingState": "requiresSpecExtension",
            "extensionRecord": {
                "id": "ext-a",
                "lifecycleState": "open"
            }
        });
        let mut by_subject: BTreeMap<String, &Value> = BTreeMap::new();
        by_subject.insert("po-a".to_string(), &m);
        let mapping = MappingResult { by_subject };
        let err = enforce_extension_record_lifecycle(&mapping).expect_err("halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::MalformedBridge);
            }
            _ => panic!("expected Halt"),
        }
    }

    /// F2.5: cmp-021 — extensionRecord without explicit lifecycleState
    /// (or != "open") does not halt.
    #[test]
    fn cmp_021_does_not_halt_when_extension_record_lifecycle_unset() {
        let m = json!({
            "id": "ma", "policyObjectRef": "po-a",
            "mappingState": "requiresSpecExtension",
            "extensionRecord": {
                "id": "ext-a"
            }
        });
        let mut by_subject: BTreeMap<String, &Value> = BTreeMap::new();
        by_subject.insert("po-a".to_string(), &m);
        let mapping = MappingResult { by_subject };
        enforce_extension_record_lifecycle(&mapping).expect("no halt without explicit open");
    }

    // ----------------------------------------------------------------
    // SA-MUST-cmp-005 — thin-projection per impactLevel.
    // ----------------------------------------------------------------
    //
    // The spec: phase-4 MUST omit any embedded block that the
    // WorkflowIntent's `impactLevel` does not require AND for which no
    // element-level PolicyObject content motivates emission. Producing
    // an empty-but-present block is a determinism violation per
    // SA-MUST-cmp-001.
    //
    // These three sentinels pin one workflow per impactLevel and
    // assert which of {governance, agents, signature, custody} appear
    // / are absent. `agents` here is gated on `actors[*].type ==
    // "agent"`, not on impactLevel; we keep actors empty so its
    // presence cleanly signals an emission bug.

    fn empty_mapping_for_impact(impact: &str) -> serde_json::Value {
        json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": format!("wfi-{impact}"),
            "version": "0.1.0",
            "title": format!("Thin-projection {impact}"),
            "impactLevel": impact,
            "actors": [],
            "publicationUrl": format!("https://example.org/wfi-{impact}"),
            "elements": []
        })
    }

    fn project_for_impact(impact: &str) -> Value {
        let intent_doc = intent(empty_mapping_for_impact(impact));
        let workflow = WorkflowResult {
            workflow_intent: &intent_doc,
            elements: vec![],
        };
        let mapping = MappingResult {
            by_subject: BTreeMap::new(),
        };
        let ws = Workspace::default();
        run(&ws, &workflow, &mapping)
            .expect("emit")
            .wos_workflow
    }

    #[test]
    fn cmp_005_operational_thin_projection_omits_optional_blocks() {
        // Operational + no mapsToWos mappings + no agent actors + no
        // signature-bearing elements: governance / agents / signature
        // / custody / extensions MUST all be absent.
        let wf = project_for_impact("operational");
        for block in ["governance", "agents", "signature", "custody", "extensions"] {
            assert!(
                wf.get(block).is_none(),
                "operational projection emitted unexpected `{block}` block: {wf}"
            );
        }
    }

    #[test]
    fn cmp_005_safety_impacting_emits_governance_and_custody_only() {
        // Safety-impacting unconditionally requires governance +
        // custody; agents / signature stay absent without their
        // motivating inputs.
        let wf = project_for_impact("safety-impacting");
        assert!(
            wf.get("governance").is_some(),
            "safety-impacting projection MUST emit governance: {wf}"
        );
        assert!(
            wf.get("custody").is_some(),
            "safety-impacting projection MUST emit custody: {wf}"
        );
        for block in ["agents", "signature", "extensions"] {
            assert!(
                wf.get(block).is_none(),
                "safety-impacting projection emitted unexpected `{block}` block: {wf}"
            );
        }
    }

    #[test]
    fn cmp_005_rights_impacting_emits_governance_and_custody_only() {
        // Rights-impacting matches safety-impacting for the
        // governance + custody floor; agents / signature still gate
        // on their motivating inputs (none supplied here).
        let wf = project_for_impact("rights-impacting");
        assert!(
            wf.get("governance").is_some(),
            "rights-impacting projection MUST emit governance: {wf}"
        );
        assert!(
            wf.get("custody").is_some(),
            "rights-impacting projection MUST emit custody: {wf}"
        );
        for block in ["agents", "signature", "extensions"] {
            assert!(
                wf.get(block).is_none(),
                "rights-impacting projection emitted unexpected `{block}` block: {wf}"
            );
        }
    }
}
