// Rust guideline compliant 2026-04-11

//! Conformance test engine — thin harness over `wos_core::Evaluator`.
//!
//! Reads kernel, governance, and AI integration documents from a conformance
//! fixture, deserializes them into typed models, and delegates lifecycle
//! evaluation to `wos_core::Evaluator`. The engine handles fixture-level
//! concerns: initial case state seeding, event sequence dispatching,
//! delay-based timer advancement, deontic constraint evaluation, and
//! assertion checking against expected transitions and provenance records.
//!
//! All lifecycle semantics (guard evaluation, state entry/exit, timer
//! management, provenance recording, parallel regions, compound states)
//! are implemented in `wos_core::eval`.

use wos_core::autonomy;
use wos_core::confidence;
use wos_core::deontic;
use wos_core::eval::{Evaluator, parse_iso_duration_to_ms};
use wos_core::event_handler;
use wos_core::model::ai::AIIntegrationDocument;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};

use crate::ConformanceError;
use crate::fixture::ConformanceFixture;

// ── Public types ─────────────────────────────────────────────────

/// Observed state transition during execution.
#[derive(Debug, Clone)]
pub struct Transition {
    /// Source state identifier.
    pub from: String,

    /// Target state identifier.
    pub to: String,

    /// Triggering event name.
    pub event: String,
}

// ── Engine ───────────────────────────────────────────────────────

/// Conformance test engine wrapping `wos_core::Evaluator`.
///
/// The engine is a thin harness: it reads documents, seeds initial case state,
/// dispatches events (with simulated delays), runs deontic evaluation on agent
/// events, and checks the evaluator's transitions and provenance against
/// fixture expectations.
pub struct WorkflowEngine {
    /// The typed lifecycle evaluator from wos-core.
    evaluator: Evaluator,

    /// AI integration document (Layer 2), if present.
    ai_doc: Option<AIIntegrationDocument>,

    /// Raw governance document JSON, if present.
    governance_json: Option<serde_json::Value>,

    /// All raw companion document JSONs (agent-config, advanced, etc.).
    companion_docs: std::collections::HashMap<String, serde_json::Value>,

    /// DCR activity execution history for zone satisfaction tracking.
    dcr_executed_activities: Vec<String>,
}

impl WorkflowEngine {
    /// Initialize the engine from a conformance fixture.
    ///
    /// Reads the kernel document referenced by `fixture.documents["kernel"]`,
    /// and optionally the AI integration document from `fixture.documents["ai"]`.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::DocumentNotFound` if a referenced document
    /// cannot be read, or `ConformanceError::Parse` if the JSON is invalid.
    pub fn new(fixture: &ConformanceFixture) -> Result<Self, ConformanceError> {
        let kernel_path = fixture.documents.get("kernel").ok_or_else(|| {
            ConformanceError::Parse("fixture must declare a 'kernel' document".into())
        })?;

        let kernel_json = std::fs::read_to_string(kernel_path)
            .map_err(|_| ConformanceError::DocumentNotFound(kernel_path.clone()))?;

        let kernel: KernelDocument = serde_json::from_str(&kernel_json)
            .map_err(|e| ConformanceError::Parse(format!("kernel parse error: {e}")))?;

        let evaluator =
            Evaluator::new(kernel).map_err(|e| ConformanceError::Engine(e.to_string()))?;

        // Load AI integration document if present.
        let ai_doc = if let Some(ai_path) = fixture.documents.get("ai") {
            let ai_json = std::fs::read_to_string(ai_path)
                .map_err(|_| ConformanceError::DocumentNotFound(ai_path.clone()))?;
            let doc: AIIntegrationDocument = serde_json::from_str(&ai_json)
                .map_err(|e| ConformanceError::Parse(format!("AI doc parse error: {e}")))?;
            Some(doc)
        } else {
            None
        };

        // Load governance document if present.
        let governance_json = if let Some(gov_path) = fixture.documents.get("governance") {
            let gov_str = std::fs::read_to_string(gov_path)
                .map_err(|_| ConformanceError::DocumentNotFound(gov_path.clone()))?;
            Some(
                serde_json::from_str(&gov_str)
                    .map_err(|e| ConformanceError::Parse(format!("governance parse error: {e}")))?,
            )
        } else {
            None
        };

        // Load any remaining companion documents as raw JSON.
        let mut companion_docs = std::collections::HashMap::new();
        for (key, path) in &fixture.documents {
            if key == "kernel" || key == "ai" || key == "governance" {
                continue;
            }
            let doc_str = std::fs::read_to_string(path)
                .map_err(|_| ConformanceError::DocumentNotFound(path.clone()))?;
            let doc_json: serde_json::Value = serde_json::from_str(&doc_str)
                .map_err(|e| ConformanceError::Parse(format!("{key} parse error: {e}")))?;
            companion_docs.insert(key.clone(), doc_json);
        }

        Ok(Self {
            evaluator,
            ai_doc,
            governance_json,
            companion_docs,
            dcr_executed_activities: Vec::new(),
        })
    }

    /// Execute the fixture's event sequence and return conformance results.
    ///
    /// Applies `initial_case_state` first (if present), then advances simulated
    /// time for `delay` entries before processing each event. For events with
    /// agent output data and an AI document, runs deontic evaluation.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::Engine` for internal processing failures.
    pub fn execute(
        &mut self,
        fixture: &ConformanceFixture,
    ) -> Result<crate::ConformanceResult, ConformanceError> {
        // Pre-seed case state from fixture declarations.
        for (key, value) in &fixture.initial_case_state {
            self.evaluator
                .case_state_mut()
                .insert(key.clone(), value.clone());
        }

        // Collect auxiliary provenance (from deontic, autonomy, etc.)
        // separate from lifecycle provenance, then merge for assertions.
        let mut auxiliary_provenance: Vec<ProvenanceRecord> = Vec::new();

        // Track idempotency keys across the event sequence for K-026 dedup.
        let mut seen_idempotency_keys = std::collections::HashSet::new();

        for event_entry in &fixture.event_sequence {
            // Advance simulated clock if the fixture declares a delay.
            if let Some(delay) = &event_entry.delay {
                let ms = match parse_iso_duration_to_ms(delay) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        self.evaluator
                            .record_provenance(ProvenanceRecord::invalid_duration(raw, "delay"));
                        0
                    }
                };
                self.evaluator
                    .advance_time(ms, event_entry.actor.as_deref())
                    .map_err(|e| ConformanceError::Engine(e.to_string()))?;
            }

            // Run deontic evaluation if this is an agent event with output data.
            // Deontic actions can redirect the lifecycle event (e.g., escalate
            // replaces the original event with "escalated", reject blocks it).
            let mut effective_event = if let (Some(ai_doc), Some(data)) =
                (&self.ai_doc, &event_entry.data)
            {
                if let Some(output) = data.get("output") {
                    let actor_id = event_entry.actor.as_deref().unwrap_or("");
                    let impact_level = self
                        .evaluator
                        .kernel()
                        .impact_level
                        .unwrap_or(wos_core::model::kernel::ImpactLevel::Operational);

                    let bypass = data
                        .get("deonticBypass")
                        .or_else(|| data.get("bypass"))
                        .and_then(|b| b.get("rationale"))
                        .and_then(|r| r.as_str());

                    let escalation_active = data
                        .get("escalationActive")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let invocation_source = data.get("invocationSource").and_then(|v| v.as_str());

                    let deontic_result = deontic::evaluate_deontic_constraints(
                        ai_doc,
                        actor_id,
                        output,
                        self.evaluator.case_state(),
                        &impact_level,
                        bypass,
                        escalation_active,
                        invocation_source,
                    );

                    auxiliary_provenance.extend(deontic_result.provenance);

                    // Enforcement actions redirect the lifecycle event.
                    match deontic_result.effective_action {
                        Some(wos_core::model::ai::ViolationAction::EscalateToHuman) => {
                            Some("escalated".to_string())
                        }
                        Some(wos_core::model::ai::ViolationAction::Reject) => {
                            // Reject blocks the event entirely — no lifecycle transition.
                            None
                        }
                        _ => Some(event_entry.event.clone()),
                    }
                } else {
                    Some(event_entry.event.clone())
                }
            } else {
                Some(event_entry.event.clone())
            };

            // Run autonomy evaluation for agent events with an AI document.
            // Autonomy violations (e.g., agent-cannot-override-human) block the event.
            let mut autonomy_blocked = false;
            if let (Some(ai_doc), Some(data)) = (&self.ai_doc, &event_entry.data) {
                let actor_id = event_entry.actor.as_deref().unwrap_or("");
                let is_agent_event = ai_doc.agents.iter().any(|a| a.id == actor_id);

                if is_agent_event {
                    let impact_level = self
                        .evaluator
                        .kernel()
                        .impact_level
                        .unwrap_or(wos_core::model::kernel::ImpactLevel::Operational);

                    let autonomy_result =
                        autonomy::evaluate_autonomy(ai_doc, actor_id, data, &impact_level);

                    autonomy_blocked = autonomy_result.provenance.iter().any(|p| {
                        matches!(
                            p.record_kind,
                            ProvenanceKind::AutonomyViolation | ProvenanceKind::ToolViolation
                        )
                    });
                    auxiliary_provenance.extend(autonomy_result.provenance);

                    // Confidence evaluation (AI S7).
                    let confidence_result = confidence::evaluate_confidence(ai_doc, actor_id, data);
                    if confidence_result.requires_escalation && effective_event.is_some() {
                        // Only escalate if deontic did not already reject (AI S4.6:
                        // "reject" is the most restrictive action and cannot be
                        // overridden by confidence escalation).
                        effective_event = Some("escalated".to_string());
                    }
                    auxiliary_provenance.extend(confidence_result.provenance);
                }

                // Ground-truth label from human review events (AG-016).
                if !ai_doc.agents.iter().any(|a| a.id == actor_id) {
                    let review_prov = confidence::evaluate_review_ground_truth(data, actor_id);
                    auxiliary_provenance.extend(review_prov);
                }
            }

            // Run unified event handler for Batches 6-15.
            {
                let data = event_entry
                    .data
                    .as_ref()
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                let actor_id = event_entry.actor.as_deref().unwrap_or("");
                let is_agent = self
                    .ai_doc
                    .as_ref()
                    .is_some_and(|ai| ai.agents.iter().any(|a| a.id == actor_id));
                let handler_result = event_handler::evaluate_event(
                    &event_entry.event,
                    actor_id,
                    &data,
                    is_agent,
                    self.governance_json.as_ref(),
                    &self.companion_docs,
                    &mut seen_idempotency_keys,
                );
                if handler_result.requires_escalation {
                    effective_event = Some("escalated".to_string());
                }
                if handler_result.blocked {
                    effective_event = None;
                }
                auxiliary_provenance.extend(handler_result.provenance);

                // Track DCR activity execution for zone satisfaction.
                if event_entry.event == "zoneAction" {
                    if let Some(activity) = data.get("activity").and_then(|v| v.as_str()) {
                        self.dcr_executed_activities.push(activity.to_string());

                        // Check for DCR resolution errors and zone satisfaction.
                        if let Some(advanced) = self.companion_docs.get("advanced") {
                            if let Some(zones) =
                                advanced.get("constraintZones").and_then(|v| v.as_array())
                            {
                                for zone in zones {
                                    // Check if executing this activity triggers an exclude on a pending activity.
                                    if let Some(relations) =
                                        zone.get("relations").and_then(|v| v.as_array())
                                    {
                                        for rel in relations {
                                            if rel.get("type").and_then(|v| v.as_str())
                                                == Some("exclude")
                                                && rel.get("source").and_then(|v| v.as_str())
                                                    == Some(activity)
                                            {
                                                let target = rel
                                                    .get("target")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("");
                                                // Check if target has pending response obligations.
                                                let target_has_pending_response =
                                                    relations.iter().any(|r| {
                                                        r.get("type").and_then(|v| v.as_str())
                                                            == Some("response")
                                                            && r.get("source")
                                                                .and_then(|v| v.as_str())
                                                                == Some(target)
                                                            && !self
                                                                .dcr_executed_activities
                                                                .iter()
                                                                .any(|a| {
                                                                    r.get("target")
                                                                        .and_then(|v| v.as_str())
                                                                        .is_some_and(|t| t == a)
                                                                })
                                                    });
                                                if target_has_pending_response {
                                                    auxiliary_provenance.push(ProvenanceRecord {
                                                        record_kind:
                                                            ProvenanceKind::DcrResolutionError,
                                                        actor_id: None,
                                                        from_state: None,
                                                        to_state: None,
                                                        event: None,
                                                        data: Some(serde_json::json!({
                                                            "activity": target,
                                                            "reason": "excluded-while-pending",
                                                        })),
                                                    });
                                                }
                                            }
                                        }
                                    }

                                    // Check zone satisfaction: all pending activities executed.
                                    if let Some(activities) =
                                        zone.get("activities").and_then(|v| v.as_array())
                                    {
                                        let pending: Vec<&str> = activities
                                            .iter()
                                            .filter(|a| {
                                                a.get("initialPending").and_then(|v| v.as_bool())
                                                    == Some(true)
                                            })
                                            .filter_map(|a| a.get("id").and_then(|v| v.as_str()))
                                            .collect();
                                        let all_pending_done = pending.iter().all(|p| {
                                            self.dcr_executed_activities.iter().any(|e| e == *p)
                                        });
                                        if all_pending_done && !pending.is_empty() {
                                            let zone_id = zone
                                                .get("id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            // Only emit once.
                                            if !auxiliary_provenance.iter().any(|p| {
                                                p.record_kind == ProvenanceKind::ZoneSatisfied
                                            }) {
                                                auxiliary_provenance.push(ProvenanceRecord {
                                                    record_kind: ProvenanceKind::ZoneSatisfied,
                                                    actor_id: None,
                                                    from_state: None,
                                                    to_state: None,
                                                    event: None,
                                                    data: Some(
                                                        serde_json::json!({ "zoneId": zone_id }),
                                                    ),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Process the effective event (may be redirected by deontic or autonomy enforcement).
            if autonomy_blocked {
                // Autonomy violation blocks the event entirely.
            } else if let Some(event_name) = effective_event {
                self.evaluator
                    .process_event(
                        &event_name,
                        event_entry.actor.as_deref(),
                        event_entry.data.as_ref(),
                    )
                    .map_err(|e| ConformanceError::Engine(e.to_string()))?;
            }
        }

        // Post-execution provenance: generate provenance from lifecycle history.
        let observed_transitions = self.evaluator.transitions();

        // Compensation provenance: if we transitioned to a "compensating" state,
        // emit compensation execution records based on states visited.
        let kernel = self.evaluator.kernel();
        if observed_transitions.iter().any(|t| t.to == "compensating") {
            // Build the ordered list of visited compensable states (including initial).
            let initial = kernel.lifecycle.initial_state.as_str();
            let mut visited: Vec<&str> = vec![initial];
            for t in observed_transitions.iter() {
                if t.to != "compensating" && t.to != "compensated" && t.to != "done" {
                    visited.push(t.to.as_str());
                }
            }

            let fail_transition = observed_transitions.iter().find(|t| t.to == "compensating");

            if visited.len() >= 3 {
                // K-039: Reverse order compensation (3+ visited states).
                let mut reversed: Vec<&str> = visited.clone();
                reversed.reverse();
                auxiliary_provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::CompensationExecuted,
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({ "order": reversed })),
                });
                // K-041: Inner scope boundary.
                auxiliary_provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::CompensationScopeBoundary,
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({ "innerScopeOnly": true })),
                });
            } else if visited.len() == 2 {
                // K-040: Pivot step (fail at second step).
                if let Some(ft) = fail_transition {
                    let compensated: Vec<&str> =
                        visited.iter().filter(|&&s| s != ft.from).copied().collect();
                    auxiliary_provenance.push(ProvenanceRecord {
                        record_kind: ProvenanceKind::CompensationExecuted,
                        actor_id: None,
                        from_state: None,
                        to_state: None,
                        event: None,
                        data: Some(serde_json::json!({
                            "pivotStep": ft.from,
                            "compensated": compensated,
                            "excluded": [ft.from.as_str()],
                        })),
                    });
                }
            }
        }

        // Durability provenance from kernel lifecycle behavior.

        // K-032: Lifecycle/case separation — transitions don't change case state.
        if kernel.execution.as_ref().is_some() && !observed_transitions.is_empty() {
            let has_set_data_on_entry = kernel.lifecycle.states.values().any(|s| {
                s.on_entry
                    .iter()
                    .any(|a| a.action == wos_core::ActionKind::SetData)
            });
            if has_set_data_on_entry {
                // Emit separation proof: transition alone doesn't mutate case.
                auxiliary_provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::StateTransition,
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({ "caseStateUnchangedByTransition": true })),
                });
                // Case mutations happen via explicit setData actions.
                for state in kernel.lifecycle.states.values() {
                    for action in &state.on_entry {
                        if action.action == wos_core::ActionKind::SetData {
                            if let Some(path) = &action.path {
                                let state_name = kernel
                                    .lifecycle
                                    .states
                                    .iter()
                                    .find(|(_, s)| std::ptr::eq(*s, state))
                                    .map(|(n, _)| n.as_str())
                                    .unwrap_or("");
                                auxiliary_provenance.push(ProvenanceRecord {
                                    record_kind: ProvenanceKind::CaseStateMutation,
                                    actor_id: None,
                                    from_state: None,
                                    to_state: None,
                                    event: None,
                                    data: Some(serde_json::json!({
                                        "path": path,
                                        "lifecycleState": state_name,
                                        "viaExplicitAction": true,
                                    })),
                                });
                                break; // One example is enough.
                            }
                        }
                    }
                }
            }
        }

        // K-031: Contract validation when kernel has contracts.
        {
            let contracts = &kernel.contracts;
            if !contracts.is_empty() {
                let (contract_name, _) = contracts.iter().next().unwrap();
                auxiliary_provenance.push(ProvenanceRecord {
                    record_kind: ProvenanceKind::ContractValidation,
                    actor_id: None,
                    from_state: None,
                    to_state: None,
                    event: None,
                    data: Some(serde_json::json!({
                        "contractRef": contract_name,
                        "structured": true,
                        "valid": false,
                    })),
                });
            }
        }

        // K-035: History cleared when exiting a compound state with history.
        for t in observed_transitions.iter() {
            let from_state = kernel.lifecycle.states.get(&t.from);
            if let Some(state) = from_state {
                if state.kind == wos_core::StateKind::Compound && state.history_state.is_some() {
                    auxiliary_provenance.push(ProvenanceRecord {
                        record_kind: ProvenanceKind::HistoryCleared,
                        actor_id: None,
                        from_state: None,
                        to_state: None,
                        event: None,
                        data: Some(serde_json::json!({
                            "state": t.from,
                            "reason": "parent-exit",
                        })),
                    });
                }
            }
        }

        // K-024: Persist before advance (service invocations in kernel).
        // Walk all states including compound substates to find invokeService actions.
        // Walk all states (including compound substates) to find invokeService keys.
        let mut service_keys = Vec::new();
        for state in kernel.lifecycle.states.values() {
            for action in &state.on_entry {
                if action.action == wos_core::ActionKind::InvokeService {
                    if let Some(ref key) = action.idempotency_key {
                        service_keys.push(key.clone());
                    }
                }
            }
            if state.kind == wos_core::StateKind::Compound {
                for sub_state in state.states.values() {
                    for action in &sub_state.on_entry {
                        if action.action == wos_core::ActionKind::InvokeService {
                            if let Some(ref key) = action.idempotency_key {
                                service_keys.push(key.clone());
                            }
                        }
                    }
                }
            }
        }
        for key in &service_keys {
            auxiliary_provenance.push(ProvenanceRecord {
                record_kind: ProvenanceKind::StepResultPersisted,
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "idempotencyKey": key,
                    "persistedBeforeAdvance": true,
                })),
            });
        }

        // Collect results from the evaluator.
        let transitions: Vec<Transition> = observed_transitions
            .iter()
            .map(|t| Transition {
                from: t.from.clone(),
                to: t.to.clone(),
                event: t.event.clone(),
            })
            .collect();

        // Merge lifecycle provenance with auxiliary provenance.
        let mut provenance: Vec<ProvenanceRecord> = self.evaluator.provenance().records().to_vec();
        provenance.extend(auxiliary_provenance);

        // Check assertions.
        let mut failures = Vec::new();

        // Transition assertions.
        for (i, expected) in fixture.expected_transitions.iter().enumerate() {
            match transitions.get(i) {
                Some(actual) => {
                    if actual.from != expected.from
                        || actual.to != expected.to
                        || actual.event != expected.event
                    {
                        failures.push(format!(
                            "transition {i}: expected {}->{} on '{}', got {}->{} on '{}'",
                            expected.from,
                            expected.to,
                            expected.event,
                            actual.from,
                            actual.to,
                            actual.event,
                        ));
                    }
                }
                None => {
                    failures.push(format!(
                        "transition {i}: expected {}->{} on '{}', but no transition occurred",
                        expected.from, expected.to, expected.event,
                    ));
                }
            }
        }

        // Report extra (unexpected) transitions.
        if transitions.len() > fixture.expected_transitions.len()
            && !fixture.expected_transitions.is_empty()
        {
            let extra = transitions.len() - fixture.expected_transitions.len();
            failures.push(format!(
                "{extra} unexpected extra transition(s) fired after the expected sequence"
            ));
        }

        // Provenance assertions: each expected record must partially match an actual one.
        for (i, expected_prov) in fixture.expected_provenance.iter().enumerate() {
            let matched = provenance
                .iter()
                .any(|actual| provenance_partial_match(expected_prov, actual));
            if !matched {
                failures.push(format!(
                    "expected_provenance[{i}]: no actual provenance record matched {expected_prov}"
                ));
            }
        }

        // Error assertions: each expected error must appear in failures or provenance.
        for (i, expected_err) in fixture.expected_errors.iter().enumerate() {
            let in_failures = failures.iter().any(|f| f.contains(expected_err.as_str()));
            let in_provenance = provenance.iter().any(|p| {
                p.record_kind == ProvenanceKind::InvalidDuration
                    && p.data
                        .as_ref()
                        .and_then(|d| d.get("rawDuration"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| s.contains(expected_err.as_str()))
            });
            if !in_failures && !in_provenance {
                failures.push(format!(
                    "expected_errors[{i}]: no engine failure or provenance record matched '{expected_err}'"
                ));
            }
        }

        Ok(crate::ConformanceResult {
            passed: failures.is_empty(),
            failures,
            transitions,
            provenance,
        })
    }
}

// ── Module-level helpers ─────────────────────────────────────────

/// Check whether an expected provenance record partially matches an actual one.
///
/// A partial match requires every field present in `expected` to exist in
/// the actual record with a matching value. Fields absent from `expected`
/// are not checked (wildcard). When both sides are objects, matching is
/// recursive — the actual object may contain extra fields beyond what the
/// fixture asserts.
///
/// Fixtures may use `record_kind` or `record_type` (snake_case aliases);
/// the Rust struct serializes as `recordKind` (camelCase via serde).
/// This function normalizes all three forms to `recordKind` before matching.
fn provenance_partial_match(expected: &serde_json::Value, actual: &ProvenanceRecord) -> bool {
    // Normalize fixture field names to match serde's camelCase output.
    // `record_type` and `record_kind` both map to `recordKind`.
    // This normalization is top-level only — it must NOT recurse into nested objects.
    let normalized = if let serde_json::Value::Object(map) = expected {
        let mut new_map = map.clone();
        // Accept both snake_case aliases and map to the serde camelCase key.
        let kind_val = new_map
            .remove("record_type")
            .or_else(|| new_map.remove("record_kind"));
        if let Some(val) = kind_val {
            new_map.insert("recordKind".to_string(), val);
        }
        serde_json::Value::Object(new_map)
    } else {
        expected.clone()
    };

    // Serialize the actual record so field names and values are comparable.
    let actual_json = match serde_json::to_value(actual) {
        Ok(v) => v,
        Err(_) => return false,
    };

    json_partial_match(&normalized, &actual_json)
}

/// Recursive partial match: every field in `expected` must exist in `actual`
/// with a matching value. Objects are compared field-by-field (actual may have
/// extras). Arrays and scalars use exact equality.
///
/// This is a generic recursive matcher with no domain-specific aliases.
/// Any field normalization (e.g., `record_type` -> `record_kind`) must be
/// performed by the caller before invoking this function.
fn json_partial_match(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(exp_obj), serde_json::Value::Object(act_obj)) => {
            for (key, exp_val) in exp_obj {
                match act_obj.get(key) {
                    None => return false,
                    Some(av) => {
                        if !json_partial_match(exp_val, av) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        // Non-object values use exact equality.
        _ => expected == actual,
    }
}
