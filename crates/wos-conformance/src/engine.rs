// Rust guideline compliant 2026-02-21

//! Deterministic workflow engine implementing the Lifecycle Detail algorithm.
//!
//! Implements Lifecycle Detail S2 (transition evaluation), S4 (parallel execution),
//! and S6 (timer semantics). Guard evaluation uses `fel-core` with a context built
//! from `caseFile`, `event`, and `instance` variables (Kernel S7.2).

use std::collections::HashMap;

use fel_core::{MapEnvironment, evaluate, json_to_fel, parse, types::FelValue};
use serde_json::Value;

use wos_core::parse_iso_duration_to_ms;

use crate::fixture::ConformanceFixture;
use crate::provenance::{ProvenanceKind, ProvenanceRecord};
use crate::ConformanceError;

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

// ── Simulated timer ──────────────────────────────────────────────

/// A durable timer tracked by the conformance engine.
///
/// Timer durability (Kernel S9.1 G4) is approximated in conformance tests by
/// simulated elapsed milliseconds advanced by `delay` entries in the fixture.
#[derive(Debug, Clone)]
struct Timer {
    /// Unique timer ID within the workflow instance.
    timer_id: String,
    /// ISO 8601 duration string as declared in the kernel document.
    ///
    /// Retained for provenance accuracy; the engine converts it to `deadline_ms`
    /// for comparison against `simulated_time_ms`.
    #[expect(dead_code, reason = "retained for provenance accuracy in future extensions")]
    duration_iso: String,
    /// Simulated deadline in milliseconds from epoch zero.
    deadline_ms: u64,
    /// Event emitted when the timer fires (Lifecycle Detail S6.2).
    fires_event: String,
}

// ── Engine ───────────────────────────────────────────────────────

/// Deterministic WOS workflow engine for conformance testing.
///
/// Implements the full evaluation algorithm from the Lifecycle Detail Companion
/// (S2): document-order guard evaluation, first-match-wins, exit innermost
/// first, enter outermost first. Supports compound states, parallel states with
/// `$join` (wait-all), `setData` case state mutations, and basic simulated timers.
pub struct WorkflowEngine {
    /// Currently active leaf states (one per parallel region, or just one for flat lifecycles).
    configuration: Vec<String>,

    /// Accumulated provenance records.
    provenance: Vec<ProvenanceRecord>,

    /// All transitions that fired during execution.
    transitions: Vec<Transition>,

    /// Parsed kernel document.
    ///
    /// Retained for future debugging and JSON serialization; engine logic now
    /// delegates state lookup to `state_index` which was derived from this at init.
    #[expect(dead_code, reason = "retained for debugging and future serialization")]
    kernel: Value,

    /// Flat index of every state in the kernel hierarchy, built once at init (Finding #11).
    ///
    /// Pre-building this map at startup avoids cloning the entire `lifecycle.states` object
    /// on every event dispatch in `try_fire_transition`.
    state_index: HashMap<String, Value>,

    /// Case file data — mutated by `setData` actions (Kernel S5).
    case_state: HashMap<String, Value>,

    /// Active timers keyed by timer ID.
    timers: HashMap<String, Timer>,

    /// Simulated clock in milliseconds.
    ///
    /// Advanced by `delay` entries in the fixture event sequence.
    simulated_time_ms: u64,

    /// Workflow instance metadata exposed in the FEL context (Kernel S7.2).
    instance_id: String,
}

impl WorkflowEngine {
    /// Initialize the engine from a conformance fixture.
    ///
    /// Reads the kernel document referenced by `fixture.documents["kernel"]`,
    /// resolves the initial state, and enters it (executing `onEntry` actions).
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::DocumentNotFound` if the kernel document path
    /// cannot be read, or `ConformanceError::Parse` if the JSON is invalid.
    pub fn new(fixture: &ConformanceFixture) -> Result<Self, ConformanceError> {
        let kernel_path = fixture
            .documents
            .get("kernel")
            .ok_or_else(|| ConformanceError::Parse("fixture must declare a 'kernel' document".into()))?;

        let kernel_json = std::fs::read_to_string(kernel_path)
            .map_err(|_| ConformanceError::DocumentNotFound(kernel_path.clone()))?;

        let kernel: Value = serde_json::from_str(&kernel_json)
            .map_err(|e| ConformanceError::Parse(e.to_string()))?;

        let initial_state = kernel
            .pointer("/lifecycle/initialState")
            .and_then(Value::as_str)
            .ok_or_else(|| ConformanceError::Parse("kernel must have lifecycle.initialState".into()))?
            .to_string();

        // Build default case state from caseFile field declarations.
        let case_state = build_default_case_state(&kernel);

        let instance_id = kernel
            .pointer("/url")
            .and_then(Value::as_str)
            .unwrap_or("conformance-instance-001")
            .to_string();

        // Pre-index all states once so `try_fire_transition` doesn't clone the entire
        // states map on every event (Finding #11).
        let state_index = build_state_index(&kernel);

        let mut engine = Self {
            configuration: Vec::new(),
            provenance: Vec::new(),
            transitions: Vec::new(),
            kernel,
            state_index,
            case_state,
            timers: HashMap::new(),
            simulated_time_ms: 0,
            instance_id,
        };

        // Enter the initial state (may be compound or parallel).
        engine.enter_state(&initial_state, None, None)?;

        Ok(engine)
    }

    /// Execute the fixture's event sequence and return conformance results.
    ///
    /// Applies `initial_case_state` first (if present), then advances simulated
    /// time for `delay` entries before processing each event.
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
            self.case_state.insert(key.clone(), value.clone());
        }

        for event_entry in &fixture.event_sequence {
            // Advance simulated clock if the fixture declares a delay.
            if let Some(delay) = &event_entry.delay {
                let ms = match parse_iso_duration_to_ms(delay) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        // Unknown duration format — emit warning and treat as 0 ms.
                        self.provenance.push(ProvenanceRecord::invalid_duration(raw, "delay"));
                        0
                    }
                };
                self.simulated_time_ms += ms;
                self.fire_expired_timers(event_entry.actor.as_deref())?;
            }

            self.process_event(
                &event_entry.event,
                event_entry.actor.as_deref(),
                event_entry.data.as_ref(),
            )?;
        }

        let mut failures = Vec::new();

        for (i, expected) in fixture.expected_transitions.iter().enumerate() {
            match self.transitions.get(i) {
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

        // Report extra (unexpected) transitions if the fixture only expects a subset.
        if self.transitions.len() > fixture.expected_transitions.len()
            && !fixture.expected_transitions.is_empty()
        {
            let extra = self.transitions.len() - fixture.expected_transitions.len();
            failures.push(format!(
                "{extra} unexpected extra transition(s) fired after the expected sequence"
            ));
        }

        // Check expected_provenance: each expected record must partially match an actual one.
        // Partial match means: every non-null field in the expected record matches the actual.
        for (i, expected_prov) in fixture.expected_provenance.iter().enumerate() {
            let matched = self
                .provenance
                .iter()
                .any(|actual| provenance_partial_match(expected_prov, actual));
            if !matched {
                failures.push(format!(
                    "expected_provenance[{i}]: no actual provenance record matched {expected_prov}"
                ));
            }
        }

        // Check expected_errors: each expected error string must appear in failure messages
        // or in an InvalidDuration provenance record.
        for (i, expected_err) in fixture.expected_errors.iter().enumerate() {
            let in_failures = failures.iter().any(|f| f.contains(expected_err.as_str()));
            let in_provenance = self.provenance.iter().any(|p| {
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
            transitions: self.transitions.clone(),
            provenance: self.provenance.clone(),
        })
    }

    // ── Event processing ────────────────────────────────────────

    /// Process a single event against the current configuration.
    ///
    /// Implements Lifecycle Detail S2.3 (collect candidates, evaluate guards in
    /// document order, first match wins).  For parallel states, routes the event
    /// to each active region independently (S4.2).  Unmatched events are recorded
    /// in provenance without changing state (Kernel S4.9).
    fn process_event(
        &mut self,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        // Try top-level states first (includes parallel states and their transitions).
        if self.try_fire_transition(event, actor, event_data)? {
            return Ok(());
        }

        // Unmatched event — record in provenance, no state change (Kernel S4.9).
        self.provenance.push(ProvenanceRecord::unmatched_event(event, actor));
        Ok(())
    }

    /// Attempt to find and fire a matching transition from the current configuration.
    ///
    /// Returns `true` if a transition fired, `false` if no match was found.
    fn try_fire_transition(
        &mut self,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<bool, ConformanceError> {
        // Derive the parallel parents from the pre-built state index rather than
        // cloning the full states map on every event (Finding #11).
        let parallel_parents = self.find_parallel_parents();

        // Clone only the IDs and the map reference needed for routing — not the full value tree.
        // We clone `parallel_parents` (Vec<String>) since it's needed after mutable borrows.
        let parallel_parents_clone = parallel_parents.clone();

        for parallel_id in &parallel_parents_clone {
            if self.try_fire_in_parallel_indexed(parallel_id, event, actor, event_data)? {
                return Ok(true);
            }
        }

        // For each active state (that is not inside a parallel we already handled),
        // collect transitions and fire the first matching one.
        for active_state in self.configuration.clone() {
            // Skip states managed by a parallel parent (already handled above).
            if parallel_parents_clone.iter().any(|p| {
                self.state_is_in_parallel_region_indexed(p, &active_state)
            }) {
                continue;
            }

            // Use find_state_anywhere (backed by the pre-built index) to locate the state.
            let state_def = match self.find_state_anywhere(&active_state) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Some(transitions) = state_def
                .get("transitions")
                .and_then(Value::as_array)
            {
                for transition in transitions.iter() {
                    let t_event = transition.get("event").and_then(Value::as_str);
                    if t_event != Some(event) {
                        continue;
                    }

                    if !self.evaluate_guard(transition, event_data)? {
                        continue;
                    }

                    let target = transition
                        .get("target")
                        .and_then(Value::as_str)
                        .ok_or_else(|| ConformanceError::Engine("transition missing target".into()))?
                        .to_string();

                    self.fire_transition(
                        &active_state.clone(),
                        &target,
                        event,
                        actor,
                        transition,
                        event_data,
                    )?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Route an event to all active states within a parallel state's regions (S4.2).
    ///
    /// Each region evaluates its transitions independently.  If all regions are now
    /// final, generates a `$join` synthetic event (Kernel S4.8).  Returns `true` if
    /// at least one region fired a transition.
    ///
    /// This variant uses `self.state_index` instead of a separate `states_map` clone
    /// to avoid allocating the full map on every event (Finding #11).
    fn try_fire_in_parallel_indexed(
        &mut self,
        parallel_id: &str,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<bool, ConformanceError> {
        let parallel_def = match self.state_index.get(parallel_id) {
            Some(s) => s.clone(),
            None => return Ok(false),
        };

        // Only route to this parallel if "event" is NOT `$join`.
        // $join is generated by the engine itself and handled via normal transitions.
        if event == "$join" {
            return Ok(false);
        }

        let regions = match parallel_def.get("regions").and_then(Value::as_object) {
            Some(r) => r.clone(),
            None => return Ok(false),
        };

        let mut any_fired = false;

        for (region_name, region_def) in &regions {
            let region_states = match region_def.get("states").and_then(Value::as_object) {
                Some(s) => s.clone(),
                None => continue,
            };

            // Find the active state within this region.
            let active_in_region = self.configuration.iter()
                .find(|s| region_states.contains_key(*s))
                .cloned();

            let Some(active) = active_in_region else {
                continue;
            };

            let state_def = match region_states.get(&active) {
                Some(s) => s.clone(),
                None => continue,
            };

            let state_type = state_def.get("type").and_then(Value::as_str).unwrap_or("atomic");
            if state_type == "final" {
                // Already done in this region.
                continue;
            }

            let transitions = match state_def.get("transitions").and_then(Value::as_array) {
                Some(t) => t.clone(),
                None => continue,
            };

            for transition in &transitions {
                let t_event = transition.get("event").and_then(Value::as_str);
                if t_event != Some(event) {
                    continue;
                }

                if !self.evaluate_guard(transition, event_data)? {
                    continue;
                }

                let target = transition
                    .get("target")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ConformanceError::Engine("region transition missing target".into()))?
                    .to_string();

                // Execute onExit for the exiting state.
                self.execute_on_exit_actions(&active, actor, event_data)?;

                // Execute transition actions.
                if let Some(actions) = transition.get("actions").and_then(Value::as_array) {
                    for action in actions.iter() {
                        self.execute_action(action, actor, event_data)?;
                    }
                }

                // Enter the target state within the region.
                let target_def = region_states.get(&target).cloned();
                self.configuration.retain(|s| s != &active);

                if let Some(td) = &target_def {
                    let target_type = td.get("type").and_then(Value::as_str).unwrap_or("atomic");
                    self.configuration.push(target.clone());
                    if target_type != "final" {
                        self.execute_on_entry_actions(&target, actor, event_data)?;
                    }
                } else {
                    self.configuration.push(target.clone());
                }

                self.transitions.push(Transition {
                    from: active.clone(),
                    to: target.clone(),
                    event: event.to_string(),
                });
                self.provenance.push(ProvenanceRecord::state_transition(
                    &active, &target, event, actor,
                ));

                any_fired = true;

                let _ = region_name; // Used in the outer loop key only.
                break;
            }
        }

        // After routing, check if all regions have reached a final state (S4.8 wait-all).
        if any_fired {
            let all_final = regions.iter().all(|(_, region_def)| {
                let region_states = match region_def.get("states").and_then(Value::as_object) {
                    Some(s) => s,
                    None => return false,
                };
                self.configuration
                    .iter()
                    .any(|s| {
                        if let Some(sd) = region_states.get(s) {
                            sd.get("type").and_then(Value::as_str) == Some("final")
                        } else {
                            false
                        }
                    })
            });

            if all_final {
                // Generate synthetic `$join` event (Kernel S4.8).
                // Remove all region states from configuration; add parallel state itself as active.
                let region_state_ids: Vec<String> = regions
                    .values()
                    .filter_map(|rd| rd.get("states").and_then(Value::as_object))
                    .flat_map(|ss| ss.keys().cloned())
                    .collect();

                self.configuration.retain(|s| !region_state_ids.contains(s));
                // Add the parallel state to the active configuration so its transitions can match $join.
                if !self.configuration.contains(&parallel_id.to_string()) {
                    self.configuration.push(parallel_id.to_string());
                }

                self.process_event("$join", actor, event_data)?;
            }
        }

        Ok(any_fired)
    }

    // ── State entry / exit ───────────────────────────────────────

    /// Enter a state, handling compound and parallel initialization.
    ///
    /// For `compound` states: enters `initialState` recursively.
    /// For `parallel` states: activates all regions (S4.1).
    /// For `atomic` and `final` states: adds to configuration and fires `onEntry`.
    fn enter_state(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let state_def = self.find_state_anywhere(state_id)?;
        let state_type = state_def.get("type").and_then(Value::as_str).unwrap_or("atomic");

        match state_type {
            "compound" => {
                self.configuration.push(state_id.to_string());
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                let initial = state_def
                    .get("initialState")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ConformanceError::Engine(format!(
                            "compound state '{state_id}' missing initialState"
                        ))
                    })?
                    .to_string();

                // Enter the initial substate; it lives in state_def["states"].
                self.enter_substate(&initial, &state_def, actor, event_data)?;
            }
            "parallel" => {
                self.configuration.push(state_id.to_string());
                self.execute_on_entry_actions(state_id, actor, event_data)?;
                self.activate_all_regions(state_id, &state_def, actor, event_data)?;
            }
            _ => {
                // atomic or final
                self.configuration.push(state_id.to_string());
                if state_type != "final" {
                    self.execute_on_entry_actions(state_id, actor, event_data)?;
                }
            }
        }

        Ok(())
    }

    /// Enter a substate defined within a parent state's `states` map.
    ///
    /// Used for compound states where the substates are nested under the parent's JSON.
    fn enter_substate(
        &mut self,
        state_id: &str,
        parent_def: &Value,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let sub_states = parent_def
            .get("states")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                ConformanceError::Engine("compound state missing 'states' object".into())
            })?;

        let state_def = sub_states.get(state_id).cloned().ok_or_else(|| {
            ConformanceError::Engine(format!("substate '{state_id}' not found in parent"))
        })?;

        let state_type = state_def.get("type").and_then(Value::as_str).unwrap_or("atomic");

        match state_type {
            "compound" => {
                self.configuration.push(state_id.to_string());
                self.execute_on_entry_actions(state_id, actor, event_data)?;
                let initial = state_def
                    .get("initialState")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ConformanceError::Engine(format!(
                            "nested compound state '{state_id}' missing initialState"
                        ))
                    })?
                    .to_string();
                self.enter_substate(&initial, &state_def, actor, event_data)?;
            }
            _ => {
                self.configuration.push(state_id.to_string());
                if state_type != "final" {
                    self.execute_on_entry_actions(state_id, actor, event_data)?;
                }
            }
        }

        Ok(())
    }

    /// Activate all regions of a parallel state simultaneously (Lifecycle Detail S4.1).
    fn activate_all_regions(
        &mut self,
        parallel_id: &str,
        parallel_def: &Value,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let regions = match parallel_def.get("regions").and_then(Value::as_object) {
            Some(r) => r.clone(),
            None => {
                return Err(ConformanceError::Engine(format!(
                    "parallel state '{parallel_id}' missing regions"
                )))
            }
        };

        for (region_name, region_def) in &regions {
            let initial = region_def
                .get("initialState")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ConformanceError::Engine(format!(
                        "region '{region_name}' in '{parallel_id}' missing initialState"
                    ))
                })?
                .to_string();

            // Enter the initial state of each region.
            let region_states = region_def
                .get("states")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    ConformanceError::Engine(format!("region '{region_name}' missing states"))
                })?;

            let init_def = region_states.get(&initial).cloned().unwrap_or(Value::Null);
            let init_type = init_def.get("type").and_then(Value::as_str).unwrap_or("atomic");

            self.configuration.push(initial.clone());
            if init_type != "final" {
                self.execute_on_entry_actions(&initial, actor, event_data)?;
            }
        }

        Ok(())
    }

    // ── Transition firing ────────────────────────────────────────

    /// Fire a transition from `source_state` to `target`.
    ///
    /// Implements Lifecycle Detail S2.4:
    /// 1. Execute `onExit` for source (innermost first).
    /// 2. Execute transition `actions`.
    /// 3. Execute `onEntry` for target (outermost first).
    /// 4. Update configuration.
    /// 5. Emit provenance.
    fn fire_transition(
        &mut self,
        source_state: &str,
        target: &str,
        event: &str,
        actor: Option<&str>,
        transition: &Value,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        // Step 1: Execute onExit for source state.
        self.execute_on_exit_actions(source_state, actor, event_data)?;

        // Step 2: Execute transition actions.
        if let Some(actions) = transition.get("actions").and_then(Value::as_array) {
            for action in actions.iter() {
                self.execute_action(action, actor, event_data)?;
            }
        }

        // Step 3–4: Remove source, enter target.
        self.configuration.retain(|s| s != source_state);
        self.enter_state(target, actor, event_data)?;

        // Step 5: Emit provenance and record transition.
        self.provenance.push(ProvenanceRecord::state_transition(
            source_state, target, event, actor,
        ));
        self.transitions.push(Transition {
            from: source_state.to_string(),
            to: target.to_string(),
            event: event.to_string(),
        });

        Ok(())
    }

    // ── Action execution ─────────────────────────────────────────

    /// Execute a single kernel action (Kernel S9.2).
    ///
    /// Handles: `setData`, `startTimer`, `cancelTimer`.
    /// Other action types (`createTask`, `invokeService`, `emitEvent`, `log`)
    /// are recorded in provenance but have no side effects in the conformance engine.
    fn execute_action(
        &mut self,
        action: &Value,
        actor: Option<&str>,
        _event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let action_type = action.get("action").and_then(Value::as_str).unwrap_or("unknown");

        match action_type {
            "setData" => {
                let path = action.get("path").and_then(Value::as_str).unwrap_or("");
                // The `value` field may be a literal JSON value or an interpolation string.
                // For conformance, we store the raw JSON value; string interpolation is left unresolved.
                let value = action.get("value").cloned().unwrap_or(Value::Null);

                // Determine current lifecycle state for mutation history (Kernel S5.4).
                let lifecycle_state = self.configuration.first().cloned().unwrap_or_default();

                // Strip the "caseFile." prefix when storing into case_state.
                let key = path.strip_prefix("caseFile.").unwrap_or(path);
                self.case_state.insert(key.to_string(), value.clone());

                self.provenance.push(ProvenanceRecord::case_state_mutation(
                    path,
                    &value,
                    actor,
                    &lifecycle_state,
                ));
            }
            "startTimer" => {
                let timer_id = action.get("timerId").and_then(Value::as_str).unwrap_or("");
                let duration = action.get("duration").and_then(Value::as_str).unwrap_or("P0D");
                let fires_event = action.get("event").and_then(Value::as_str).unwrap_or("");

                let duration_ms = match parse_iso_duration_to_ms(duration) {
                    Ok(ms) => ms,
                    Err(raw) => {
                        // Unknown duration format — emit warning, timer fires immediately.
                        self.provenance.push(ProvenanceRecord::invalid_duration(raw, timer_id));
                        0
                    }
                };
                let deadline_ms = self.simulated_time_ms + duration_ms;

                // Cancel any existing timer with the same ID (Lifecycle Detail S6.4 reentry).
                if self.timers.contains_key(timer_id) {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        timer_id,
                        "reentry-cancel",
                    ));
                }

                self.timers.insert(
                    timer_id.to_string(),
                    Timer {
                        timer_id: timer_id.to_string(),
                        duration_iso: duration.to_string(),
                        deadline_ms,
                        fires_event: fires_event.to_string(),
                    },
                );

                self.provenance.push(ProvenanceRecord::timer_created(
                    timer_id, duration, fires_event,
                ));
            }
            "cancelTimer" => {
                let timer_id = action.get("timerId").and_then(Value::as_str).unwrap_or("");
                if self.timers.remove(timer_id).is_some() {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        timer_id, "explicit-cancel",
                    ));
                }
                // No-op if the timer does not exist (Lifecycle Detail S6.3).
            }
            // createTask, invokeService, emitEvent, log: record in provenance, no engine side-effect.
            // Use action_executed (not on_entry) so the record_kind correctly reflects that
            // this is a generic action, not a lifecycle entry hook (Finding #14/#15).
            other => {
                let current_state = self.configuration.first().cloned().unwrap_or_default();
                self.provenance.push(ProvenanceRecord::action_executed(
                    &current_state, other,
                ));
            }
        }

        Ok(())
    }

    /// Execute all `onEntry` actions for a state.
    ///
    /// Emits an `onEntry` provenance record for each action before executing it,
    /// mirroring the behavior of `execute_on_exit_actions` (Finding #15).
    fn execute_on_entry_actions(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let actions = self.collect_on_entry_actions(state_id);
        for action in actions {
            let action_type = action.get("action").and_then(Value::as_str).unwrap_or("unknown");
            self.provenance.push(ProvenanceRecord::on_entry(state_id, action_type));
            self.execute_action(&action, actor, event_data)?;
        }
        Ok(())
    }

    /// Execute all `onExit` actions for a state.
    ///
    /// Emits an `onExit` provenance record for each action before executing it.
    fn execute_on_exit_actions(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&Value>,
    ) -> Result<(), ConformanceError> {
        let actions = self.collect_on_exit_actions(state_id);
        for action in actions {
            let action_type = action.get("action").and_then(Value::as_str).unwrap_or("unknown");
            self.provenance.push(ProvenanceRecord::on_exit(state_id, action_type));
            self.execute_action(&action, actor, event_data)?;
        }
        Ok(())
    }

    /// Collect `onEntry` actions from any state definition (top-level or nested in regions/states).
    fn collect_on_entry_actions(&self, state_id: &str) -> Vec<Value> {
        if let Ok(state_def) = self.find_state_anywhere(state_id) {
            return state_def
                .get("onEntry")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        Vec::new()
    }

    /// Collect `onExit` actions from any state definition.
    fn collect_on_exit_actions(&self, state_id: &str) -> Vec<Value> {
        if let Ok(state_def) = self.find_state_anywhere(state_id) {
            return state_def
                .get("onExit")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        Vec::new()
    }

    // ── Guard evaluation ─────────────────────────────────────────

    /// Evaluate a transition guard using FEL (Lifecycle Detail S2.3, Kernel S7.2).
    ///
    /// Builds an evaluation context with `caseFile`, `event`, and `instance` variables.
    /// A missing guard (no `guard` property) is treated as always-true.
    ///
    /// # Errors
    ///
    /// Returns `ConformanceError::Engine` if the FEL expression fails to parse.
    fn evaluate_guard(
        &self,
        transition: &Value,
        event_data: Option<&Value>,
    ) -> Result<bool, ConformanceError> {
        let guard_expr = match transition.get("guard").and_then(Value::as_str) {
            Some(g) => g,
            None => return Ok(true), // No guard means always-true (Kernel S4.6).
        };

        let env = self.build_fel_context(event_data);

        let parsed = parse(guard_expr).map_err(|e| {
            ConformanceError::Engine(format!("guard parse error in '{guard_expr}': {e}"))
        })?;

        let result = evaluate(&parsed, &env);

        Ok(matches!(result.value, FelValue::Boolean(true)))
    }

    /// Build a FEL `MapEnvironment` from the current engine state (Kernel S7.2).
    ///
    /// Populates:
    /// - `caseFile.*` — current case state fields.
    /// - `event.*` — event payload data (if any).
    /// - `instance.id`, `instance.currentStates` — workflow instance metadata.
    fn build_fel_context(&self, event_data: Option<&Value>) -> MapEnvironment {
        let mut fields: HashMap<String, FelValue> = HashMap::new();

        // Populate caseFile fields (stored without the "caseFile." prefix internally).
        let case_obj: Vec<(String, FelValue)> = self
            .case_state
            .iter()
            .map(|(k, v)| (k.clone(), json_to_fel(v)))
            .collect();

        fields.insert(
            "caseFile".to_string(),
            FelValue::Object(case_obj.clone()),
        );

        // Also expose each field as a direct dotted path so expressions like
        // `caseFile.amount` resolve correctly in both object-walk and flat-key lookups.
        for (k, v) in &case_obj {
            fields.insert(format!("caseFile.{k}"), v.clone());
        }

        // Populate event data.
        if let Some(data) = event_data {
            let event_pairs: Vec<(String, FelValue)> = data
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), json_to_fel(v)))
                        .collect()
                })
                .unwrap_or_default();

            fields.insert("event".to_string(), FelValue::Object(event_pairs.clone()));
            for (k, v) in &event_pairs {
                fields.insert(format!("event.{k}"), v.clone());
            }
        }

        // Populate instance metadata.
        let current_states: Vec<FelValue> = self
            .configuration
            .iter()
            .map(|s| FelValue::String(s.clone()))
            .collect();

        let instance_pairs = vec![
            ("id".to_string(), FelValue::String(self.instance_id.clone())),
            ("currentStates".to_string(), FelValue::Array(current_states)),
        ];
        fields.insert("instance".to_string(), FelValue::Object(instance_pairs.clone()));
        for (k, v) in &instance_pairs {
            fields.insert(format!("instance.{k}"), v.clone());
        }

        MapEnvironment::with_fields(fields)
    }

    // ── Timer management ─────────────────────────────────────────

    /// Fire all timers whose deadline has been reached by the current simulated time.
    ///
    /// Timers are fired in deadline order (earliest first).
    fn fire_expired_timers(&mut self, actor: Option<&str>) -> Result<(), ConformanceError> {
        let current_time = self.simulated_time_ms;

        // Collect expired timers sorted by deadline.
        let mut expired: Vec<Timer> = self
            .timers
            .values()
            .filter(|t| t.deadline_ms <= current_time)
            .cloned()
            .collect();

        expired.sort_by_key(|t| t.deadline_ms);

        for timer in expired {
            self.timers.remove(&timer.timer_id);
            self.provenance
                .push(ProvenanceRecord::timer_fired(&timer.timer_id, &timer.fires_event));

            // Deliver the timer event into the lifecycle.
            self.process_event(&timer.fires_event.clone(), actor, None)?;
        }

        Ok(())
    }

    // ── State lookup helpers ─────────────────────────────────────

    /// Find a state definition by ID using the pre-built state index.
    ///
    /// The index covers the entire state hierarchy at any nesting depth (Finding #12).
    /// Using the index avoids cloning the states map on every lookup (Finding #11).
    fn find_state_anywhere(&self, state_id: &str) -> Result<Value, ConformanceError> {
        self.state_index
            .get(state_id)
            .cloned()
            .ok_or_else(|| {
                ConformanceError::Engine(format!(
                    "state '{state_id}' not found in kernel document"
                ))
            })
    }

    /// Collect the parallel state IDs that are ancestors of any currently active state.
    ///
    /// Uses `self.state_index` (pre-built at init) rather than cloning the states map
    /// each call (Finding #11).
    fn find_parallel_parents(&self) -> Vec<String> {
        let mut parents = Vec::new();

        for (state_id, state_def) in &self.state_index {
            let state_type = state_def.get("type").and_then(Value::as_str).unwrap_or("atomic");
            if state_type != "parallel" {
                continue;
            }

            let regions = match state_def.get("regions").and_then(Value::as_object) {
                Some(r) => r,
                None => continue,
            };

            let has_active_child = regions.values().any(|region_def| {
                region_def
                    .get("states")
                    .and_then(Value::as_object)
                    .map(|region_states| {
                        self.configuration
                            .iter()
                            .any(|active| region_states.contains_key(active))
                    })
                    .unwrap_or(false)
            });

            if has_active_child {
                parents.push(state_id.clone());
            }
        }

        parents
    }

    /// Return `true` if `state_id` lives within a region of the named parallel state.
    ///
    /// Uses `self.state_index` instead of a borrowed states map (Finding #11).
    fn state_is_in_parallel_region_indexed(&self, parallel_id: &str, state_id: &str) -> bool {
        let parallel_def = match self.state_index.get(parallel_id) {
            Some(d) => d,
            None => return false,
        };

        let regions = match parallel_def.get("regions").and_then(Value::as_object) {
            Some(r) => r,
            None => return false,
        };

        regions.values().any(|region_def| {
            region_def
                .get("states")
                .and_then(Value::as_object)
                .map(|ss| ss.contains_key(state_id))
                .unwrap_or(false)
        })
    }
}

// ── Module-level helpers ─────────────────────────────────────────

/// Build a flat index of every state in the kernel hierarchy at any nesting depth.
///
/// Pre-building this at engine init avoids cloning the entire `lifecycle.states` map
/// on every event in `try_fire_transition` (Finding #11).  The index is also used by
/// `find_state_anywhere` to resolve states at arbitrary nesting depth (Finding #12).
fn build_state_index(kernel: &Value) -> HashMap<String, Value> {
    let mut index = HashMap::new();

    if let Some(states) = kernel
        .pointer("/lifecycle/states")
        .and_then(Value::as_object)
    {
        index_states_recursive(states, &mut index);
    }

    index
}

/// Recursively insert every state from `states` into `index`.
fn index_states_recursive(
    states: &serde_json::Map<String, Value>,
    index: &mut HashMap<String, Value>,
) {
    for (name, def) in states {
        index.insert(name.clone(), def.clone());

        let state_type = def.get("type").and_then(Value::as_str).unwrap_or("atomic");

        if state_type == "compound" {
            if let Some(substates) = def.get("states").and_then(Value::as_object) {
                index_states_recursive(substates, index);
            }
        }

        if state_type == "parallel" {
            if let Some(regions) = def.get("regions").and_then(Value::as_object) {
                for region_def in regions.values() {
                    if let Some(region_states) = region_def.get("states").and_then(Value::as_object) {
                        index_states_recursive(region_states, index);
                    }
                }
            }
        }
    }
}


/// Check whether an expected provenance record (a JSON object with arbitrary fields)
/// is a partial match for an actual `ProvenanceRecord`.
///
/// A partial match requires every field present in `expected` to equal the
/// corresponding field in the serialized actual record. Fields absent from
/// `expected` are not checked (wildcard). The `record_type` field in `expected`
/// matches the camelCase serialization of `actual.record_kind`.
fn provenance_partial_match(expected: &serde_json::Value, actual: &ProvenanceRecord) -> bool {
    // Serialize the actual record so field names and values are comparable.
    let actual_json = match serde_json::to_value(actual) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let expected_obj = match expected.as_object() {
        Some(o) => o,
        None => return false,
    };

    for (key, expected_val) in expected_obj {
        // Support both "record_kind" (Rust field name) and "record_type" (legacy alias)
        // so fixture authors can use either.
        let actual_val = if key == "record_type" {
            actual_json.get("record_kind")
        } else {
            actual_json.get(key)
        };

        match actual_val {
            None => return false,
            Some(av) => {
                if av != expected_val {
                    return false;
                }
            }
        }
    }

    true
}

/// Build the initial case state map from `caseFile.fields` default values (Kernel S5.2).
fn build_default_case_state(kernel: &Value) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    let fields = match kernel
        .pointer("/caseFile/fields")
        .and_then(Value::as_object)
    {
        Some(f) => f,
        None => return map,
    };

    for (field_name, field_def) in fields {
        if let Some(default) = field_def.get("default") {
            map.insert(field_name.clone(), default.clone());
        }
    }

    map
}

// parse_iso_duration_to_ms is imported from wos_core (consolidated).

// Duration parsing tests now live in wos-core (the canonical implementation).
