// Rust guideline compliant 2026-02-21

//! Deterministic lifecycle evaluation algorithm.
//!
//! Implements the algorithm from the Lifecycle Detail Companion (S2):
//! document-order guard evaluation, first-match-wins, exit innermost
//! first, enter outermost first.
//!
//! Operates on typed [`KernelDocument`] models, not raw JSON.

use std::collections::HashMap;

use fel_core::{evaluate, parse, types::FelValue};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::context::EvalContext;
use crate::instance::CaseInstance;
use crate::model::kernel::{Action, ActionKind, KernelDocument, State, StateKind};
use crate::provenance::{ProvenanceLog, ProvenanceRecord};
use crate::timer::Timers;

/// Active state configuration tracking leaf states.
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    /// Currently active leaf state identifiers.
    active: Vec<String>,
}

impl Configuration {
    /// The currently active states.
    pub fn active_states(&self) -> &[String] {
        &self.active
    }

    /// Whether a state is currently active.
    pub fn contains(&self, state_id: &str) -> bool {
        self.active.iter().any(|s| s == state_id)
    }

    /// Add a state to the active configuration.
    pub fn enter(&mut self, state_id: String) {
        self.active.push(state_id);
    }

    /// Remove a state from the active configuration.
    pub fn exit(&mut self, state_id: &str) {
        self.active.retain(|s| s != state_id);
    }
}

/// Flat index entry for a state at any nesting depth.
#[derive(Debug, Clone)]
pub struct IndexedState {
    /// The typed state definition.
    pub state: State,
    /// Parent parallel state ID, if this state lives in a region.
    pub parallel_parent: Option<String>,
    /// Region name, if this state lives in a parallel region.
    pub region_name: Option<String>,
}

/// The lifecycle evaluator.
///
/// Holds the kernel document and mutable execution state (configuration,
/// case data, timers, provenance). Processes events one at a time per the
/// deterministic evaluation algorithm (Kernel S4.2).
pub struct Evaluator {
    /// The kernel document being evaluated.
    kernel: KernelDocument,

    /// Pre-built flat index of all states at any depth.
    state_index: HashMap<String, IndexedState>,

    /// Active state configuration.
    config: Configuration,

    /// Case state (field name -> value).
    case_state: HashMap<String, serde_json::Value>,

    /// Timer tracking.
    timers: Timers,

    /// Provenance log.
    provenance: ProvenanceLog,

    /// Simulated time in milliseconds (for timer tests).
    simulated_time_ms: u64,

    /// All transitions that fired during execution.
    transitions: Vec<ObservedTransition>,

    /// All actions executed during this evaluator lifetime.
    executed_actions: Vec<ObservedAction>,
}

/// An observed state transition.
#[derive(Debug, Clone)]
pub struct ObservedTransition {
    /// Source state.
    pub from: String,
    /// Target state.
    pub to: String,
    /// Triggering event.
    pub event: String,
}

/// An action executed by the evaluator during a single event step.
#[derive(Debug, Clone)]
pub struct ObservedAction {
    /// Lifecycle state active when the action executed.
    pub lifecycle_state: String,
    /// Actor associated with the triggering event, if any.
    pub actor_id: Option<String>,
    /// The concrete action definition.
    pub action: Action,
}

/// Errors from the evaluation algorithm.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// Guard expression failed to parse or evaluate.
    #[error("guard evaluation error: {0}")]
    Guard(String),

    /// State referenced in a transition does not exist.
    #[error("state not found: {0}")]
    StateNotFound(String),

    /// Internal consistency error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl Evaluator {
    /// Create an evaluator for a kernel document.
    ///
    /// Enters the initial state and executes its onEntry actions.
    pub fn new(kernel: KernelDocument) -> Result<Self, EvalError> {
        Self::with_time(kernel, 0)
    }

    /// Create an evaluator using the provided millisecond clock.
    pub fn with_time(kernel: KernelDocument, current_time_ms: u64) -> Result<Self, EvalError> {
        Self::with_time_and_case_state(kernel, current_time_ms, None)
    }

    /// Create an evaluator using the provided millisecond clock and seeded case state.
    pub fn with_time_and_case_state(
        kernel: KernelDocument,
        current_time_ms: u64,
        initial_case_state: Option<&serde_json::Value>,
    ) -> Result<Self, EvalError> {
        let initial = kernel.lifecycle.initial_state.clone();
        let state_index = build_state_index(&kernel);
        let case_state = build_default_case_state(&kernel);
        let mut seeded_case_state = case_state;

        if let Some(initial_case_state) = initial_case_state {
            if let Some(initial_object) = initial_case_state.as_object() {
                seeded_case_state.extend(
                    initial_object
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
            }
        }

        let mut eval = Self {
            kernel,
            state_index,
            config: Configuration::default(),
            case_state: seeded_case_state,
            timers: Timers::default(),
            provenance: ProvenanceLog::default(),
            simulated_time_ms: current_time_ms,
            transitions: Vec::new(),
            executed_actions: Vec::new(),
        };
        eval.enter_state(&initial, None, None)?;
        Ok(eval)
    }

    /// Restore an evaluator from a serialized case instance.
    pub fn from_instance(
        kernel: KernelDocument,
        instance: &CaseInstance,
        current_time_ms: u64,
    ) -> Result<Self, EvalError> {
        let state_index = build_state_index(&kernel);
        let mut timers = Timers::default();

        for timer in &instance.timers {
            let deadline_ms = parse_rfc3339_to_ms(&timer.deadline)?;
            timers.create(crate::timer::Timer {
                id: timer.timer_id.clone(),
                deadline_ms,
                fires_event: timer.event.clone(),
                created_in_state: timer.scope_state.clone().unwrap_or_default(),
                duration_iso: timer
                    .duration_iso
                    .clone()
                    .unwrap_or_else(|| "P0D".to_string()),
                duration_ms: timer.duration_ms.unwrap_or(0),
            });
        }

        Ok(Self {
            kernel,
            state_index,
            config: Configuration {
                active: instance.configuration.clone(),
            },
            case_state: case_state_from_value(&instance.case_state),
            timers,
            provenance: ProvenanceLog::default(),
            simulated_time_ms: current_time_ms,
            transitions: Vec::new(),
            executed_actions: Vec::new(),
        })
    }

    /// The current active state configuration.
    pub fn configuration(&self) -> &Configuration {
        &self.config
    }

    /// The current case state.
    pub fn case_state(&self) -> &HashMap<String, serde_json::Value> {
        &self.case_state
    }

    /// Mutable access to case state for pre-seeding.
    pub fn case_state_mut(&mut self) -> &mut HashMap<String, serde_json::Value> {
        &mut self.case_state
    }

    /// The provenance log.
    pub fn provenance(&self) -> &ProvenanceLog {
        &self.provenance
    }

    /// Append an external provenance record.
    ///
    /// Used by conformance and runtime harnesses to record events that
    /// originate outside the lifecycle algorithm (e.g., delay parse errors
    /// in fixture event sequences).
    pub fn record_provenance(&mut self, record: ProvenanceRecord) {
        self.provenance.push(record);
    }

    /// The timer state.
    pub fn timers(&self) -> &Timers {
        &self.timers
    }

    /// All observed transitions in order.
    pub fn transitions(&self) -> &[ObservedTransition] {
        &self.transitions
    }

    /// All actions executed during this evaluator lifetime.
    pub fn executed_actions(&self) -> &[ObservedAction] {
        &self.executed_actions
    }

    /// Consume the executed-action log.
    pub fn take_executed_actions(&mut self) -> Vec<ObservedAction> {
        std::mem::take(&mut self.executed_actions)
    }

    /// The kernel document.
    pub fn kernel(&self) -> &KernelDocument {
        &self.kernel
    }

    /// Serialize case state into a JSON object.
    pub fn case_state_json(&self) -> serde_json::Value {
        let object = self
            .case_state
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        serde_json::Value::Object(object)
    }

    // ── Event processing ────────────────────────────────────────

    /// Process a single event.
    ///
    /// Implements Lifecycle Detail S2.3: collect candidates from active
    /// states, evaluate guards in document order, fire first match.
    /// Events matching no transition are recorded in provenance but do
    /// not change state (Kernel S4.9).
    pub fn process_event(
        &mut self,
        event: &str,
        actor: Option<&str>,
        data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        if self.try_fire_transition(event, actor, data)? {
            return Ok(true);
        }

        // Unmatched event — record in provenance, no state change.
        self.provenance
            .push(ProvenanceRecord::unmatched_event(event, actor));
        Ok(false)
    }

    /// Advance simulated time and fire expired timers.
    pub fn advance_time(&mut self, duration_ms: u64, actor: Option<&str>) -> Result<(), EvalError> {
        self.simulated_time_ms += duration_ms;
        self.fire_expired_timers(actor)
    }

    /// Try to fire a `$continuous` transition in the current configuration.
    ///
    /// Used by continuous evaluation mode (Runtime S10.3). Scans all active
    /// states for transitions on event `$continuous` whose guards evaluate
    /// to true. Fires the first match (document order).
    ///
    /// Returns `true` if a transition fired, `false` if no guards were satisfied.
    pub fn try_fire_guardless_transition(&mut self) -> Result<bool, EvalError> {
        self.try_fire_transition("$continuous", None, None)
    }

    // ── Transition dispatch ─────────────────────────────────────

    /// Attempt to find and fire a matching transition.
    ///
    /// Returns `true` if a transition fired.
    fn try_fire_transition(
        &mut self,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        // Route to parallel parents first.
        let parallel_parents = self.find_parallel_parents();
        for parallel_id in &parallel_parents {
            if self.try_fire_in_parallel(parallel_id, event, actor, event_data)? {
                return Ok(true);
            }
        }

        // For each active state not inside a parallel we already handled.
        let active_snapshot = self.config.active.clone();
        for active_state in &active_snapshot {
            if parallel_parents
                .iter()
                .any(|p| self.state_is_in_parallel_region(p, active_state))
            {
                continue;
            }

            let indexed = match self.state_index.get(active_state) {
                Some(s) => s.clone(),
                None => continue,
            };

            for transition in &indexed.state.transitions {
                if transition.event != event {
                    continue;
                }
                if !self.evaluate_guard(transition.guard.as_deref(), event_data)? {
                    continue;
                }

                self.fire_transition(
                    active_state,
                    &transition.target.clone(),
                    event,
                    actor,
                    &transition.actions.clone(),
                    event_data,
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Route an event to active states within a parallel state's regions.
    fn try_fire_in_parallel(
        &mut self,
        parallel_id: &str,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        if event == "$join" {
            return Ok(false);
        }

        let indexed = match self.state_index.get(parallel_id) {
            Some(s) => s.clone(),
            None => return Ok(false),
        };

        let regions = indexed.state.regions.clone();
        let mut any_fired = false;

        for (_region_name, region_def) in &regions {
            let active_in_region = self
                .config
                .active
                .iter()
                .find(|s| region_def.states.contains_key(*s))
                .cloned();

            let Some(active) = active_in_region else {
                continue;
            };

            let state_def = match region_def.states.get(&active) {
                Some(s) => s.clone(),
                None => continue,
            };

            if state_def.kind == StateKind::Final {
                continue;
            }

            for transition in &state_def.transitions {
                if transition.event != event {
                    continue;
                }
                if !self.evaluate_guard(transition.guard.as_deref(), event_data)? {
                    continue;
                }

                let target = transition.target.clone();

                // Execute onExit.
                self.execute_on_exit_actions(&active, actor, event_data)?;

                // Execute transition actions.
                for action in &transition.actions {
                    self.execute_action(action, actor, event_data)?;
                }

                // Update configuration.
                self.config.exit(&active);
                let target_def = region_def.states.get(&target);
                self.config.enter(target.clone());
                if let Some(td) = target_def {
                    if td.kind != StateKind::Final {
                        self.execute_on_entry_actions(&target, actor, event_data)?;
                    }
                }

                self.transitions.push(ObservedTransition {
                    from: active.clone(),
                    to: target.clone(),
                    event: event.to_string(),
                });
                self.provenance.push(ProvenanceRecord::state_transition(
                    &active, &target, event, actor,
                ));

                any_fired = true;
                break;
            }
        }

        // Check if all regions reached final (S4.8 wait-all).
        if any_fired {
            let all_final = regions.iter().all(|(_, region_def)| {
                self.config.active.iter().any(|s| {
                    region_def
                        .states
                        .get(s)
                        .is_some_and(|sd| sd.kind == StateKind::Final)
                })
            });

            if all_final {
                // Collect all region state IDs and remove from config.
                let region_state_ids: Vec<String> = regions
                    .values()
                    .flat_map(|rd| rd.states.keys().cloned())
                    .collect();
                self.config.active.retain(|s| !region_state_ids.contains(s));

                if !self.config.contains(parallel_id) {
                    self.config.enter(parallel_id.to_string());
                }

                self.process_event("$join", actor, event_data)?;
            }
        }

        Ok(any_fired)
    }

    // ── State entry / exit ───────────────────────────────────────

    /// Enter a state, handling compound and parallel initialization.
    fn enter_state(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let indexed = self
            .state_index
            .get(state_id)
            .ok_or_else(|| EvalError::StateNotFound(state_id.to_string()))?
            .clone();

        match indexed.state.kind {
            StateKind::Compound => {
                self.config.enter(state_id.to_string());
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                let initial = indexed.state.initial_state.as_deref().ok_or_else(|| {
                    EvalError::Internal(format!("compound state '{state_id}' missing initialState"))
                })?;
                self.enter_state(initial, actor, event_data)?;
            }
            StateKind::Parallel => {
                self.config.enter(state_id.to_string());
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                for (_name, region) in &indexed.state.regions {
                    let region_initial = &region.initial_state;
                    let init_def = region.states.get(region_initial.as_str());
                    self.config.enter(region_initial.clone());

                    if let Some(sd) = init_def {
                        if sd.kind != StateKind::Final {
                            self.execute_on_entry_actions(region_initial, actor, event_data)?;
                        }
                    }
                }
            }
            StateKind::Atomic | StateKind::Final => {
                self.config.enter(state_id.to_string());
                if indexed.state.kind != StateKind::Final {
                    self.execute_on_entry_actions(state_id, actor, event_data)?;
                }
            }
        }

        Ok(())
    }

    // ── Transition firing ────────────────────────────────────────

    /// Fire a transition: exit source, run actions, enter target.
    fn fire_transition(
        &mut self,
        source: &str,
        target: &str,
        event: &str,
        actor: Option<&str>,
        actions: &[Action],
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        self.execute_on_exit_actions(source, actor, event_data)?;

        for action in actions {
            self.execute_action(action, actor, event_data)?;
        }

        // Remove source and all its descendant states from the configuration.
        // This handles compound/parallel state exits where substates would
        // otherwise be orphaned.
        self.exit_state_and_descendants(source);
        self.enter_state(target, actor, event_data)?;

        self.provenance.push(ProvenanceRecord::state_transition(
            source, target, event, actor,
        ));
        self.transitions.push(ObservedTransition {
            from: source.to_string(),
            to: target.to_string(),
            event: event.to_string(),
        });

        Ok(())
    }

    // ── Action execution ─────────────────────────────────────────

    /// Execute a single kernel action (Kernel S9.2).
    fn execute_action(
        &mut self,
        action: &Action,
        actor: Option<&str>,
        _event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let lifecycle_state = self.config.active.first().cloned().unwrap_or_default();
        match action.action {
            ActionKind::SetData => {
                let path = action.path.as_deref().unwrap_or("");
                let value = action.value.clone().unwrap_or(serde_json::Value::Null);

                let key = path.strip_prefix("caseFile.").unwrap_or(path);
                self.case_state.insert(key.to_string(), value.clone());

                self.provenance.push(ProvenanceRecord::case_state_mutation(
                    path,
                    &value,
                    actor,
                    &lifecycle_state,
                ));
            }
            ActionKind::StartTimer => {
                let timer_id = action.timer_id.as_deref().unwrap_or("");
                let duration = action.duration.as_deref().unwrap_or("P0D");
                let fires_event = action.event.as_deref().unwrap_or("");

                let duration_ms = parse_iso_duration_to_ms(duration).unwrap_or_else(|raw| {
                    self.provenance
                        .push(ProvenanceRecord::invalid_duration(raw, timer_id));
                    0
                });
                let deadline_ms = self.simulated_time_ms + duration_ms;

                // Cancel existing timer with same ID (reentry, Lifecycle Detail S6.4).
                if self.timers.cancel(timer_id).is_some() {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        timer_id,
                        "reentry-cancel",
                    ));
                }

                self.timers.create(crate::timer::Timer {
                    id: timer_id.to_string(),
                    deadline_ms,
                    fires_event: fires_event.to_string(),
                    created_in_state: self.config.active.first().cloned().unwrap_or_default(),
                    duration_iso: duration.to_string(),
                    duration_ms,
                });

                self.provenance.push(ProvenanceRecord::timer_created(
                    timer_id,
                    duration,
                    fires_event,
                ));
            }
            ActionKind::CancelTimer => {
                let timer_id = action.timer_id.as_deref().unwrap_or("");
                if self.timers.cancel(timer_id).is_some() {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        timer_id,
                        "explicit-cancel",
                    ));
                }
            }
            _ => {
                let current_state = self.config.active.first().cloned().unwrap_or_default();
                let action_name = format!("{:?}", action.action);
                let action_name_camel = match action.action {
                    ActionKind::CreateTask => "createTask",
                    ActionKind::InvokeService => "invokeService",
                    ActionKind::EmitEvent => "emitEvent",
                    ActionKind::Log => "log",
                    _ => &action_name,
                };
                self.provenance.push(ProvenanceRecord::action_executed(
                    &current_state,
                    action_name_camel,
                ));
            }
        }

        self.executed_actions.push(ObservedAction {
            lifecycle_state,
            actor_id: actor.map(String::from),
            action: action.clone(),
        });

        Ok(())
    }

    /// Execute onEntry actions for a state.
    fn execute_on_entry_actions(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let actions = self
            .state_index
            .get(state_id)
            .map(|s| s.state.on_entry.clone())
            .unwrap_or_default();

        for action in &actions {
            let action_name = match action.action {
                ActionKind::CreateTask => "createTask",
                ActionKind::InvokeService => "invokeService",
                ActionKind::SetData => "setData",
                ActionKind::EmitEvent => "emitEvent",
                ActionKind::StartTimer => "startTimer",
                ActionKind::CancelTimer => "cancelTimer",
                ActionKind::Log => "log",
            };
            self.provenance
                .push(ProvenanceRecord::on_entry(state_id, action_name));
            self.execute_action(action, actor, event_data)?;
        }
        Ok(())
    }

    /// Execute onExit actions for a state.
    fn execute_on_exit_actions(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let actions = self
            .state_index
            .get(state_id)
            .map(|s| s.state.on_exit.clone())
            .unwrap_or_default();

        for action in &actions {
            let action_name = match action.action {
                ActionKind::CreateTask => "createTask",
                ActionKind::InvokeService => "invokeService",
                ActionKind::SetData => "setData",
                ActionKind::EmitEvent => "emitEvent",
                ActionKind::StartTimer => "startTimer",
                ActionKind::CancelTimer => "cancelTimer",
                ActionKind::Log => "log",
            };
            self.provenance
                .push(ProvenanceRecord::on_exit(state_id, action_name));
            self.execute_action(action, actor, event_data)?;
        }
        Ok(())
    }

    // ── Guard evaluation ─────────────────────────────────────────

    /// Evaluate a FEL guard expression. Missing guard = always true.
    fn evaluate_guard(
        &self,
        guard: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        let guard_expr = match guard {
            Some(g) => g,
            None => return Ok(true),
        };

        let ctx = EvalContext::from_case_state(&self.case_state, event_data);
        let env = ctx.to_fel_environment();

        let parsed = parse(guard_expr)
            .map_err(|e| EvalError::Guard(format!("parse error in '{guard_expr}': {e}")))?;

        let result = evaluate(&parsed, &env);
        Ok(matches!(result.value, FelValue::Boolean(true)))
    }

    // ── Timer management ─────────────────────────────────────────

    /// Fire all expired timers, checking tolerance (LCD S6.6, Runtime S7.2).
    fn fire_expired_timers(&mut self, actor: Option<&str>) -> Result<(), EvalError> {
        let current_time = self.simulated_time_ms;
        let expired = self.timers.collect_expired(current_time);

        for timer in expired {
            self.provenance
                .push(ProvenanceRecord::timer_fired(&timer.id, &timer.fires_event));

            // Check tolerance: if the timer fired significantly after its deadline,
            // emit a toleranceViolation diagnostic (LCD S6.6, Runtime S7.2).
            let lateness_ms = current_time.saturating_sub(timer.deadline_ms);
            let max_tolerance = crate::timer::max_tolerance_ms(timer.duration_ms);
            if lateness_ms > max_tolerance {
                let tolerance_iso = crate::timer::tolerance_to_iso(max_tolerance);
                self.provenance.push(ProvenanceRecord::tolerance_violation(
                    &timer.id,
                    &timer.duration_iso,
                    &tolerance_iso,
                ));
            }

            let event = timer.fires_event.clone();
            self.process_event(&event, actor, None)?;
        }

        Ok(())
    }

    // ── State lookup helpers ─────────────────────────────────────

    /// Remove a state and all its descendants from the configuration.
    ///
    /// For compound states, removes substates. For parallel states,
    /// removes all region states. Prevents orphaned substates after
    /// transitions that exit a compound or parallel ancestor.
    fn exit_state_and_descendants(&mut self, state_id: &str) {
        self.config.exit(state_id);

        let indexed = match self.state_index.get(state_id) {
            Some(s) => s.clone(),
            None => return,
        };

        match indexed.state.kind {
            StateKind::Compound => {
                if indexed.state.history_state.is_some() {
                    self.provenance
                        .push(ProvenanceRecord::history_cleared(state_id, "parent-exit"));
                }
                let substate_ids: Vec<String> = indexed.state.states.keys().cloned().collect();
                for sub_id in &substate_ids {
                    self.exit_state_and_descendants(sub_id);
                }
            }
            StateKind::Parallel => {
                for region in indexed.state.regions.values() {
                    let region_ids: Vec<String> = region.states.keys().cloned().collect();
                    for region_state_id in &region_ids {
                        self.exit_state_and_descendants(region_state_id);
                    }
                }
            }
            _ => {}
        }
    }

    /// Find parallel states that are ancestors of active states.
    fn find_parallel_parents(&self) -> Vec<String> {
        let mut parents = Vec::new();
        for (state_id, indexed) in &self.state_index {
            if indexed.state.kind != StateKind::Parallel {
                continue;
            }
            let has_active = indexed.state.regions.values().any(|region| {
                self.config
                    .active
                    .iter()
                    .any(|active| region.states.contains_key(active))
            });
            if has_active {
                parents.push(state_id.clone());
            }
        }
        parents
    }

    /// Check if a state lives in a parallel state's region.
    fn state_is_in_parallel_region(&self, parallel_id: &str, state_id: &str) -> bool {
        self.state_index.get(parallel_id).is_some_and(|indexed| {
            indexed
                .state
                .regions
                .values()
                .any(|region| region.states.contains_key(state_id))
        })
    }
}

// ── Module-level helpers ─────────────────────────────────────────

/// Build a flat state index from the typed kernel document.
fn build_state_index(kernel: &KernelDocument) -> HashMap<String, IndexedState> {
    let mut index = HashMap::new();
    index_states_recursive(&kernel.lifecycle.states, None, None, &mut index);
    index
}

/// Recursively index states from a states map.
fn index_states_recursive(
    states: &indexmap::IndexMap<String, State>,
    parallel_parent: Option<&str>,
    region_name: Option<&str>,
    index: &mut HashMap<String, IndexedState>,
) {
    for (name, state) in states {
        index.insert(
            name.clone(),
            IndexedState {
                state: state.clone(),
                parallel_parent: parallel_parent.map(String::from),
                region_name: region_name.map(String::from),
            },
        );

        if state.kind == StateKind::Compound {
            index_states_recursive(&state.states, parallel_parent, region_name, index);
        }

        if state.kind == StateKind::Parallel {
            for (rname, region) in &state.regions {
                index_states_recursive(&region.states, Some(name), Some(rname), index);
            }
        }
    }
}

/// Build initial case state from field defaults.
fn build_default_case_state(kernel: &KernelDocument) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    if let Some(case_file) = &kernel.case_file {
        for (field_name, field_def) in &case_file.fields {
            if let Some(default) = &field_def.default {
                map.insert(field_name.clone(), default.clone());
            }
        }
    }
    map
}

fn case_state_from_value(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_rfc3339_to_ms(value: &str) -> Result<u64, EvalError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        EvalError::Internal(format!("invalid timer deadline '{value}': {error}"))
    })?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    u64::try_from(millis).map_err(|_| {
        EvalError::Internal(format!(
            "timer deadline '{value}' predates Unix epoch and cannot be restored"
        ))
    })
}

/// Parse an ISO 8601 duration to milliseconds.
///
/// Supports `Y`, `M` (months), `W`, `D` in the date part and `H`, `M`
/// (minutes), `S` in the time part (after `T`). Fractional values are
/// supported (e.g., `PT1.5H`).
///
/// # Errors
///
/// Returns the original string when the format is unrecognized
/// (e.g., missing leading `P`).
pub fn parse_iso_duration_to_ms(duration: &str) -> Result<u64, &str> {
    let rest = duration.strip_prefix('P').ok_or(duration)?;

    // Split at 'T' into date and time segments.
    let (date_part, time_part) = match rest.find('T') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };

    let ms = parse_duration_segment(date_part, false) + parse_duration_segment(time_part, true);

    Ok(ms)
}

/// Parse a date or time segment of an ISO 8601 duration string.
fn parse_duration_segment(segment: &str, is_time: bool) -> u64 {
    const MS_PER_SECOND: u64 = 1_000;
    const MS_PER_MINUTE: u64 = 60 * MS_PER_SECOND;
    const MS_PER_HOUR: u64 = 60 * MS_PER_MINUTE;
    const MS_PER_DAY: u64 = 24 * MS_PER_HOUR;
    /// Approximate; exact calendar not needed for simulated timers.
    const MS_PER_MONTH: u64 = 30 * MS_PER_DAY;
    /// Approximate; exact calendar not needed for simulated timers.
    const MS_PER_YEAR: u64 = 365 * MS_PER_DAY;

    let mut ms = 0u64;
    let mut num_buf = String::new();

    for ch in segment.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else {
            let value: f64 = num_buf.parse().unwrap_or(0.0);
            num_buf.clear();

            let unit_ms = if is_time {
                match ch {
                    'H' => MS_PER_HOUR,
                    'M' => MS_PER_MINUTE,
                    'S' => MS_PER_SECOND,
                    _ => 0,
                }
            } else {
                match ch {
                    'Y' => MS_PER_YEAR,
                    'M' => MS_PER_MONTH,
                    'W' => 7 * MS_PER_DAY,
                    'D' => MS_PER_DAY,
                    _ => 0,
                }
            };

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "ISO duration values are always non-negative; truncation acceptable for simulation"
            )]
            {
                ms += (value * unit_ms as f64) as u64;
            }
        }
    }

    ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iso_duration_seconds() {
        assert_eq!(parse_iso_duration_to_ms("PT30S").unwrap(), 30_000);
    }

    #[test]
    fn parse_iso_duration_minutes() {
        assert_eq!(parse_iso_duration_to_ms("PT5M").unwrap(), 300_000);
    }

    #[test]
    fn parse_iso_duration_days() {
        assert_eq!(
            parse_iso_duration_to_ms("P7D").unwrap(),
            7 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn parse_iso_duration_years() {
        assert_eq!(
            parse_iso_duration_to_ms("P1Y").unwrap(),
            365 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn parse_iso_duration_composite() {
        assert_eq!(
            parse_iso_duration_to_ms("P1DT12H").unwrap(),
            36 * 60 * 60 * 1000
        );
    }

    #[test]
    fn parse_iso_duration_months() {
        assert_eq!(
            parse_iso_duration_to_ms("P3M").unwrap(),
            3 * 30 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn parse_iso_duration_weeks() {
        assert_eq!(
            parse_iso_duration_to_ms("P2W").unwrap(),
            14 * 24 * 60 * 60 * 1000
        );
    }

    #[test]
    fn parse_iso_duration_invalid() {
        assert!(parse_iso_duration_to_ms("invalid").is_err());
    }
}
