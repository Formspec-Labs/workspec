// Rust guideline compliant 2026-02-21

//! Deterministic lifecycle evaluation algorithm.
//!
//! Implements the algorithm from the Lifecycle Detail Companion (S2):
//! document-order guard evaluation, first-match-wins, exit innermost
//! first, enter outermost first.
//!
//! Operates on typed [`KernelDocument`] models, not raw JSON.

use std::collections::HashMap;

use fel_core::{
    ast::Expr, convert::fel_to_json, dependencies::extract_dependencies, evaluate, parse,
    types::FelValue,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::context::EvalContext;
use crate::instance::CaseInstance;
use crate::model::kernel::{
    Action, ActionKind, CancellationPolicy, HistoryMode, KernelDocument, Region, State, StateKind,
    Transition, TransitionEvent,
};
use crate::provenance::{CaseFileSnapshot, ProvenanceLog, ProvenanceRecord};
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
    /// Immediate parent state ID (compound or parallel), if any.
    ///
    /// Used by deep history restore to reconstruct the full ancestor
    /// chain from a leaf state up to the history-bearing compound.
    pub direct_parent: Option<String>,
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

    /// Guard evaluations observed during this evaluator lifetime.
    ///
    /// Captures every guard expression tested — including those that
    /// evaluated false and short-circuited their transition. Drained
    /// per-event by `wos-runtime::drain_once`.
    guard_evaluations: Vec<GuardEvaluation>,

    /// Saved history configurations keyed by compound state ID.
    ///
    /// When a compound state with `historyState` is exited, the active
    /// substate configuration is saved here. On re-entry, the saved
    /// configuration is restored instead of using `initialState`.
    history_store: HashMap<String, Vec<String>>,
}

/// Event name recorded on provenance and guard traces when a transition fires
/// from continuous-mode post-mutation re-scan (Runtime Companion §10.3).
///
/// This string is **never** read from a kernel document — it is synthesized
/// by the evaluator for traces only. Authored §10.3 re-scan participation uses
/// guard-only transitions (omit `event`); authored `$`-prefixed transition
/// events are rejected by lint K-007.
const CONTINUOUS_RESCAN_EVENT: &str = "$postMutationRescan";

/// An observed state transition.
#[derive(Debug, Clone)]
pub struct ObservedTransition {
    /// Source state.
    pub from: String,
    /// Target state.
    pub to: String,
    /// Triggering event.
    pub event: String,
    /// Semantic tags declared on the transition.
    pub tags: Vec<String>,
}

/// A single guard-expression evaluation observed during event processing.
///
/// Recorded every time the evaluator tests a transition's `guard` FEL
/// expression — including short-circuited `false` evaluations on transitions
/// that did not fire. Downstream consumers (the `wos-runtime` drain loop
/// and `wos-conformance` trace builder) use these records to produce a
/// teaching signal for LLM-authored workflows: when a fixture fails, the
/// trace can show which guard evaluated false and against which inputs.
///
/// `guard_id` is synthesized from the transition's shape
/// (`{source_state}->{target_state}:{event}`) — kernel transitions do not
/// carry explicit guard identifiers today. `inputs` is a JSON subset of the
/// evaluation context limited to the paths the guard expression actually
/// references, preserving FEL namespace nesting (`caseFile.*` / `event.*`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuardEvaluation {
    /// Synthesized identifier: `{source_state}->{target_state}:{event}`.
    pub guard_id: String,
    /// Source state of the transition whose guard was evaluated.
    pub source_state: String,
    /// Target state of the transition whose guard was evaluated.
    pub target_state: String,
    /// Event name that triggered the guard test.
    pub event: String,
    /// The raw FEL expression text.
    pub expression: String,
    /// Result of the expression (true fires the transition, false skips it).
    pub result: bool,
    /// Inputs actually read by the guard, nested by FEL namespace.
    pub inputs: serde_json::Value,
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
    /// Runtime event payload that triggered this action, if any.
    ///
    /// For transition actions this is the triggering event's `data` field.
    /// For onEntry / onExit actions this is the event data that caused the
    /// state to be entered or exited. Integration handlers (event-consume,
    /// callback inbound) read the CloudEvent envelope from this field.
    pub event_data: Option<serde_json::Value>,
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

fn transition_matches_dispatch(
    transition: &Transition,
    event: &str,
    continuous_rescan: bool,
) -> bool {
    if continuous_rescan {
        transition.participates_in_continuous_rescan()
    } else {
        transition
            .event
            .as_ref()
            .is_some_and(|ev| ev.matches_runtime_dispatch(event))
    }
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
            guard_evaluations: Vec::new(),
            history_store: HashMap::new(),
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
            // Fall back to reconstructing start from deadline-duration for
            // instances persisted before created_at_ms was introduced.
            let created_at_ms = timer
                .created_at_ms
                .unwrap_or_else(|| deadline_ms.saturating_sub(timer.duration_ms.unwrap_or(0)));
            timers.create(crate::timer::Timer {
                id: timer.timer_id.clone(),
                created_at_ms,
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
            guard_evaluations: Vec::new(),
            history_store: instance.history_store.clone(),
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

    /// All guard evaluations observed since the last `take_guard_evaluations`.
    ///
    /// Includes guards that evaluated false and short-circuited their
    /// transition — the teaching-signal use case needs to see exactly which
    /// guards the evaluator tested and against what inputs.
    pub fn guard_evaluations(&self) -> &[GuardEvaluation] {
        &self.guard_evaluations
    }

    /// Drain the guard-evaluation buffer.
    ///
    /// Called by `wos-runtime::drain_once` after each event step so that
    /// `DrainOnceResult.guard_evaluations` scopes to the single drained event.
    pub fn take_guard_evaluations(&mut self) -> Vec<GuardEvaluation> {
        std::mem::take(&mut self.guard_evaluations)
    }

    /// The kernel document.
    pub fn kernel(&self) -> &KernelDocument {
        &self.kernel
    }

    /// The history store (saved configurations keyed by compound state ID).
    pub fn history_store(&self) -> &HashMap<String, Vec<String>> {
        &self.history_store
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
        if self.try_fire_transition(event, actor, data, false)? {
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

    /// Re-run transition guards after a case-file mutation in `continuous` mode.
    ///
    /// Implements Runtime Companion §10.3: collect every transition that omits
    /// `event` (guard-only), walk the active configuration in the same order as
    /// explicit events, and fire the first whose guard is now true. Provenance
    /// records the synthetic [`CONTINUOUS_RESCAN_EVENT`] label (not an authored
    /// event name).
    ///
    /// Returns `true` if a transition fired, `false` if the configuration is
    /// already stable.
    pub fn rescan_on_mutation(&mut self) -> Result<bool, EvalError> {
        self.try_fire_transition(CONTINUOUS_RESCAN_EVENT, None, None, true)
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
        continuous_rescan: bool,
    ) -> Result<bool, EvalError> {
        let dispatch_event = if continuous_rescan {
            CONTINUOUS_RESCAN_EVENT
        } else {
            event
        };

        // Route to parallel parents first.
        let parallel_parents = self.find_parallel_parents();
        for parallel_id in &parallel_parents {
            if self.try_fire_in_parallel(
                parallel_id,
                event,
                actor,
                event_data,
                continuous_rescan,
            )? {
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
                let event_matches =
                    transition_matches_dispatch(transition, event, continuous_rescan);
                if !event_matches {
                    continue;
                }
                if !self.evaluate_transition_guard(
                    transition.guard.as_ref(),
                    event_data,
                    active_state,
                    &transition.target,
                    dispatch_event,
                )? {
                    continue;
                }

                self.fire_transition(
                    active_state,
                    &transition.target,
                    dispatch_event,
                    actor,
                    &transition.actions,
                    &transition.tags,
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
        continuous_rescan: bool,
    ) -> Result<bool, EvalError> {
        if event == "$join" {
            return Ok(false);
        }

        let dispatch_event = if continuous_rescan {
            CONTINUOUS_RESCAN_EVENT
        } else {
            event
        };

        let indexed = match self.state_index.get(parallel_id) {
            Some(s) => s.clone(),
            None => return Ok(false),
        };

        let regions = indexed.state.regions.clone();
        let mut any_fired = false;

        for (region_name, region_def) in &regions {
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
                let event_matches =
                    transition_matches_dispatch(transition, event, continuous_rescan);
                if !event_matches {
                    continue;
                }
                if !self.evaluate_transition_guard(
                    transition.guard.as_ref(),
                    event_data,
                    &active,
                    &transition.target,
                    dispatch_event,
                )? {
                    continue;
                }

                let target = transition.target.clone();
                let case_file_snapshot = self.case_file_snapshot_for_transition(&transition.tags);

                // Execute onExit.
                self.execute_on_exit_actions(&active, actor, event_data)?;

                // Execute transition actions.
                for action in &transition.actions {
                    self.execute_action_in_state(action, actor, &active, event_data)?;
                }

                // Update configuration.
                self.config.exit(&active);
                self.enter_state(&target, actor, event_data)?;

                self.transitions.push(ObservedTransition {
                    from: active.clone(),
                    to: target.clone(),
                    event: dispatch_event.to_string(),
                    tags: transition.tags.clone(),
                });
                self.provenance
                    .push(ProvenanceRecord::tagged_state_transition(
                        &active,
                        &target,
                        dispatch_event,
                        actor,
                        &transition.tags,
                        case_file_snapshot,
                    ));

                self.apply_parallel_cancellation_policy(
                    parallel_id,
                    &regions,
                    region_name,
                    &target,
                    actor,
                    event_data,
                )?;
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

    fn apply_parallel_cancellation_policy(
        &mut self,
        parallel_id: &str,
        regions: &indexmap::IndexMap<String, Region>,
        fired_region_name: &str,
        target: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let Some(parallel_state) = self
            .state_index
            .get(parallel_id)
            .map(|indexed| &indexed.state)
        else {
            return Ok(());
        };
        let policy = parallel_state
            .cancellation_policy
            .unwrap_or(CancellationPolicy::WaitAll);
        if policy == CancellationPolicy::WaitAll {
            return Ok(());
        }

        let Some(target_state) = self.state_index.get(target).map(|indexed| &indexed.state) else {
            return Ok(());
        };
        let reached_final = target_state.kind == StateKind::Final;
        let reached_error_final =
            reached_final && target_state.tags.iter().any(|tag| tag == "error");
        let should_cancel = match policy {
            CancellationPolicy::WaitAll => false,
            CancellationPolicy::CancelSiblings => reached_final,
            CancellationPolicy::FailFast => reached_error_final,
        };
        if !should_cancel {
            return Ok(());
        }

        for (region_name, region) in regions {
            if region_name == fired_region_name {
                continue;
            }
            let active_states: Vec<String> = self
                .config
                .active
                .iter()
                .filter(|state_id| region.states.contains_key(*state_id))
                .cloned()
                .collect();
            for active_state in active_states {
                self.execute_on_exit_actions(&active_state, actor, event_data)?;
                self.cancel_timers_created_in_state_tree(&active_state, "region-cancellation");
                self.exit_state_and_descendants(&active_state);
            }
        }

        Ok(())
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
                self.provenance
                    .push(ProvenanceRecord::state_entered(state_id));
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                if let Some(saved) = self.history_store.get(state_id).cloned() {
                    self.restore_history(state_id, &saved, actor, event_data)?;
                } else {
                    let initial = indexed.state.initial_state.as_deref().ok_or_else(|| {
                        EvalError::Internal(format!(
                            "compound state '{state_id}' missing initialState"
                        ))
                    })?;
                    self.enter_state(initial, actor, event_data)?;
                }
                Ok(())
            }
            StateKind::Parallel => {
                self.config.enter(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(state_id));
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                for (_name, region) in &indexed.state.regions {
                    let region_initial = &region.initial_state;
                    self.enter_state(region_initial, actor, event_data)?;
                }
                Ok(())
            }
            StateKind::Atomic | StateKind::Final => {
                self.config.enter(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(state_id));
                if indexed.state.kind != StateKind::Final {
                    self.execute_on_entry_actions(state_id, actor, event_data)?;
                }
                Ok(())
            }
        }
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
        tags: &[String],
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let case_file_snapshot = self.case_file_snapshot_for_transition(tags);

        self.execute_on_exit_actions(source, actor, event_data)?;

        for action in actions {
            self.execute_action_in_state(action, actor, source, event_data)?;
        }

        // Remove source and all its descendant states from the configuration.
        // This handles compound/parallel state exits where substates would
        // otherwise be orphaned.
        self.exit_state_and_descendants(source);
        self.enter_state(target, actor, event_data)?;

        self.provenance
            .push(ProvenanceRecord::tagged_state_transition(
                source,
                target,
                event,
                actor,
                tags,
                case_file_snapshot,
            ));
        self.transitions.push(ObservedTransition {
            from: source.to_string(),
            to: target.to_string(),
            event: event.to_string(),
            tags: tags.to_vec(),
        });

        Ok(())
    }

    fn case_file_snapshot_for_transition(&self, tags: &[String]) -> Option<CaseFileSnapshot> {
        if tags.iter().any(|tag| tag == "determination") {
            Some(CaseFileSnapshot::from_case_state(&self.case_state_json()))
        } else {
            None
        }
    }

    // ── Action execution ─────────────────────────────────────────

    fn execute_action_in_state(
        &mut self,
        action: &Action,
        actor: Option<&str>,
        lifecycle_state: &str,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
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
                    lifecycle_state,
                ));
            }
            ActionKind::StartTimer => {
                let timer_id = action.timer_id.as_deref().unwrap_or("");
                let duration = action.duration.as_deref().unwrap_or("P0D");
                let fires_event = action
                    .event
                    .as_ref()
                    .map(TransitionEvent::start_timer_fires_string)
                    .unwrap_or_default();

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
                    created_at_ms: self.simulated_time_ms,
                    deadline_ms,
                    fires_event: fires_event.to_string(),
                    created_in_state: lifecycle_state.to_string(),
                    duration_iso: duration.to_string(),
                    duration_ms,
                });

                self.provenance.push(ProvenanceRecord::timer_created(
                    timer_id,
                    duration,
                    fires_event.as_str(),
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
                let action_name = format!("{:?}", action.action);
                let action_name_camel = match action.action {
                    ActionKind::CreateTask => "createTask",
                    ActionKind::InvokeService => "invokeService",
                    ActionKind::EmitEvent => "emitEvent",
                    ActionKind::Log => "log",
                    _ => &action_name,
                };
                self.provenance.push(ProvenanceRecord::action_executed(
                    lifecycle_state,
                    action_name_camel,
                ));
            }
        }

        self.executed_actions.push(ObservedAction {
            lifecycle_state: lifecycle_state.to_string(),
            actor_id: actor.map(String::from),
            action: action.clone(),
            event_data: event_data.cloned(),
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
            self.execute_action_in_state(action, actor, state_id, event_data)?;
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
            self.execute_action_in_state(action, actor, state_id, event_data)?;
        }
        Ok(())
    }

    // ── Guard evaluation ─────────────────────────────────────────

    /// Dispatch a transition guard per Kernel §4.5.1.1.
    ///
    /// The polymorphic [`crate::model::decision_table::Guard`] form admits a
    /// FEL string or a structured `DecisionTableGuard`; routes accordingly.
    /// `None` means "no guard" — always fires, no [`GuardEvaluation`]
    /// recorded.
    fn evaluate_transition_guard(
        &mut self,
        guard: Option<&crate::model::decision_table::Guard>,
        event_data: Option<&serde_json::Value>,
        source_state: &str,
        target_state: &str,
        event: &str,
    ) -> Result<bool, EvalError> {
        match guard {
            None => Ok(true),
            Some(crate::model::decision_table::Guard::Fel(expr)) => self.evaluate_guard(
                Some(expr.as_str()),
                event_data,
                source_state,
                target_state,
                event,
            ),
            Some(crate::model::decision_table::Guard::DecisionTable(dt_guard)) => self
                .evaluate_decision_table_guard(
                    dt_guard,
                    event_data,
                    source_state,
                    target_state,
                    event,
                ),
        }
    }

    /// Evaluate a FEL guard expression. Missing guard = always true.
    ///
    /// Records a [`GuardEvaluation`] on every call that actually tests a
    /// guard expression (including `false` results). Missing guards are
    /// not recorded — `None` means "no constraint" and carries no teaching
    /// signal.
    fn evaluate_guard(
        &mut self,
        guard: Option<&str>,
        event_data: Option<&serde_json::Value>,
        source_state: &str,
        target_state: &str,
        event: &str,
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
        let passed = matches!(result.value, FelValue::Boolean(true));

        let inputs = build_guard_inputs(&parsed, &self.case_state, event_data);
        self.guard_evaluations.push(GuardEvaluation {
            guard_id: format!("{source_state}->{target_state}:{event}"),
            source_state: source_state.to_string(),
            target_state: target_state.to_string(),
            event: event.to_string(),
            expression: guard_expr.to_string(),
            result: passed,
            inputs,
        });

        Ok(passed)
    }

    /// Evaluate a structured `DecisionTableGuard` per Kernel §4.5.1.2.
    ///
    /// Algorithm:
    /// 1. Look up the referenced `DecisionTable`. Missing → `Err`
    ///    (`K-051`-shape; lint normally catches this earlier).
    /// 2. For each declared input, evaluate its `inputBindings[name]`
    ///    expression in the full transition context (caseFile + event
    ///    namespaces) and bind the result into the row scope under the
    ///    input name.
    /// 3. For each row in document order: evaluate every input cell
    ///    against the row scope. Non-boolean → `Err` (K-053). All true
    ///    ⇒ row matches.
    /// 4. Apply the table's hit policy (`unique`/`first`/`priority`).
    ///    `collect` is rejected for guard usage (K-053). `unique` with
    ///    multiple matches → `Err` (K-052 at runtime). `priority` ties
    ///    → `Err`.
    /// 5. Zero matches: `OnNoMatch::Fail` → `Err`; otherwise → `false`.
    /// 6. Resolve the output column. Non-boolean output type → `Err`
    ///    (K-053). Evaluate the selected row's matching output cell in
    ///    the row scope. Non-boolean result → `Err` (K-053).
    ///
    /// Records a single [`GuardEvaluation`] capturing the synthesized
    /// "expression" `decisionTable(<tableId>).<outputColumn>` and the
    /// resolved row scope as the inputs payload (so the teaching-signal
    /// trace shows what the table actually saw).
    fn evaluate_decision_table_guard(
        &mut self,
        guard: &crate::model::decision_table::DecisionTableGuard,
        event_data: Option<&serde_json::Value>,
        source_state: &str,
        target_state: &str,
        event: &str,
    ) -> Result<bool, EvalError> {
        use crate::model::decision_table::{FelType, HitPolicy, OnNoMatch};

        let table_ref = guard.table_ref.as_str();
        let table = self
            .kernel
            .decision_tables
            .iter()
            .find(|t| t.id == table_ref)
            .ok_or_else(|| {
                EvalError::Guard(format!(
                    "K-051: decisionTable ref '{table_ref}' does not resolve"
                ))
            })?
            .clone();

        // ── Step 2: build the row scope from input bindings.
        // Each binding expression is evaluated in the full transition
        // context (caseFile + event); the result is bound under the input's
        // declared name in a fresh, namespace-free MapEnvironment used for
        // every row's cell evaluation.
        let outer_ctx = EvalContext::from_case_state(&self.case_state, event_data);
        let outer_env = outer_ctx.to_fel_environment();

        let mut row_scope_fields: HashMap<String, FelValue> = HashMap::new();
        for input_decl in &table.inputs {
            let binding_expr = guard.input_bindings.get(&input_decl.name).ok_or_else(|| {
                EvalError::Guard(format!(
                    "K-051: decisionTable guard for table '{table_ref}' is missing inputBindings entry for declared input '{}'",
                    input_decl.name
                ))
            })?;
            let parsed = parse(binding_expr).map_err(|e| {
                EvalError::Guard(format!(
                    "decisionTable inputBinding for '{}' on table '{table_ref}' failed to parse: {e}",
                    input_decl.name
                ))
            })?;
            let bound = evaluate(&parsed, &outer_env);
            row_scope_fields.insert(input_decl.name.clone(), bound.value);
        }

        let row_env = fel_core::MapEnvironment::with_fields(row_scope_fields.clone());

        // ── Step 3: collect matching rows in document order.
        let mut matches: Vec<usize> = Vec::new();
        for (row_idx, row) in table.rows.iter().enumerate() {
            let mut all_true = true;
            // Spec: zip(row.inputCells, table.inputs) — extra/missing input
            // cells are an authoring error caught by lint K-053. At runtime
            // we evaluate however many cells the row has against the
            // declared inputs in declaration order.
            for cell_expr in &row.input_cells {
                let parsed = parse(cell_expr).map_err(|e| {
                    EvalError::Guard(format!(
                        "decisionTable input cell on row '{}' (table '{table_ref}') failed to parse: {e}",
                        row.id
                    ))
                })?;
                let result = evaluate(&parsed, &row_env);
                match result.value {
                    FelValue::Boolean(true) => {}
                    FelValue::Boolean(false) => {
                        all_true = false;
                        break;
                    }
                    other => {
                        return Err(EvalError::Guard(format!(
                            "K-053: decisionTable input cell on row '{}' (table '{table_ref}') did not evaluate to boolean (got {})",
                            row.id,
                            other.type_name()
                        )));
                    }
                }
            }
            if all_true {
                matches.push(row_idx);
            }
        }

        // ── Step 4: hit-policy selection.
        let selected: Option<usize> = match table.hit_policy {
            HitPolicy::Unique => match matches.as_slice() {
                [] => None,
                [only] => Some(*only),
                more => {
                    let ids: Vec<&str> =
                        more.iter().map(|i| table.rows[*i].id.as_str()).collect();
                    return Err(EvalError::Guard(format!(
                        "K-052: decisionTable '{table_ref}' has hitPolicy=unique but {} rows matched: [{}]",
                        more.len(),
                        ids.join(", ")
                    )));
                }
            },
            HitPolicy::First => matches.first().copied(),
            HitPolicy::Priority => {
                if matches.is_empty() {
                    None
                } else {
                    // Among matched rows, pick the one with the lowest
                    // priority integer. Rows missing `priority` sort after
                    // every numbered row (treated as +∞). Ties are K-052.
                    let mut best: Option<(usize, Option<i64>)> = None;
                    let mut tied_with_best: Vec<usize> = Vec::new();
                    for &idx in &matches {
                        let prio = table.rows[idx].priority;
                        match best {
                            None => {
                                best = Some((idx, prio));
                                tied_with_best.clear();
                            }
                            Some((_, current_prio)) => match (prio, current_prio) {
                                (Some(p), Some(c)) if p < c => {
                                    best = Some((idx, Some(p)));
                                    tied_with_best.clear();
                                }
                                (Some(p), Some(c)) if p == c => {
                                    tied_with_best.push(idx);
                                }
                                (Some(_), None) => {
                                    best = Some((idx, prio));
                                    tied_with_best.clear();
                                }
                                _ => {}
                            },
                        }
                    }
                    let (best_idx, _) = best.expect("matches non-empty checked above");
                    if !tied_with_best.is_empty() {
                        let mut ids: Vec<&str> = vec![table.rows[best_idx].id.as_str()];
                        ids.extend(tied_with_best.iter().map(|i| table.rows[*i].id.as_str()));
                        return Err(EvalError::Guard(format!(
                            "K-052: decisionTable '{table_ref}' priority-tie among matched rows: [{}]",
                            ids.join(", ")
                        )));
                    }
                    Some(best_idx)
                }
            }
            HitPolicy::Collect => {
                // K-053: lint MUST reject `collect` for transition-guard
                // tables; runtime defends in case lint was bypassed.
                return Err(EvalError::Guard(format!(
                    "K-053: decisionTable '{table_ref}' has hitPolicy=collect but is referenced as a transition guard; collect is reserved for non-guard consumers"
                )));
            }
        };

        // Synthesize a teaching-signal expression and a row-scope inputs
        // snapshot used by GuardEvaluation regardless of which branch fires.
        let synthesized_expr = format!("decisionTable({table_ref}).{}", guard.output_column);
        let row_scope_inputs = row_scope_to_json(&row_scope_fields);

        let selected_idx = match selected {
            Some(i) => i,
            None => {
                // No row matched.
                let on_no_match = guard.on_no_match.unwrap_or(OnNoMatch::False);
                let result_value = match on_no_match {
                    OnNoMatch::False => false,
                    OnNoMatch::Fail => {
                        return Err(EvalError::Guard(format!(
                            "decisionTable '{table_ref}' produced no match and onNoMatch = fail"
                        )));
                    }
                };
                self.guard_evaluations.push(GuardEvaluation {
                    guard_id: format!("{source_state}->{target_state}:{event}"),
                    source_state: source_state.to_string(),
                    target_state: target_state.to_string(),
                    event: event.to_string(),
                    expression: synthesized_expr,
                    result: result_value,
                    inputs: row_scope_inputs,
                });
                return Ok(result_value);
            }
        };

        // ── Step 6: resolve output column and evaluate the matching cell.
        let output_idx = table
            .outputs
            .iter()
            .position(|o| o.name == guard.output_column)
            .ok_or_else(|| {
                EvalError::Guard(format!(
                    "K-051: decisionTable '{table_ref}' outputColumn '{}' does not resolve",
                    guard.output_column
                ))
            })?;
        let output_decl = &table.outputs[output_idx];
        if output_decl.kind != FelType::Boolean {
            return Err(EvalError::Guard(format!(
                "K-053: decisionTable '{table_ref}' guard outputColumn '{}' must be boolean (declared {:?})",
                guard.output_column, output_decl.kind
            )));
        }

        let selected_row = &table.rows[selected_idx];
        let cell_expr = selected_row.output_cells.get(output_idx).ok_or_else(|| {
            EvalError::Guard(format!(
                "decisionTable '{table_ref}' row '{}' missing outputCells[{output_idx}] for outputColumn '{}'",
                selected_row.id, guard.output_column
            ))
        })?;
        let parsed_out = parse(cell_expr).map_err(|e| {
            EvalError::Guard(format!(
                "decisionTable output cell on row '{}' (table '{table_ref}') failed to parse: {e}",
                selected_row.id
            ))
        })?;
        let result = evaluate(&parsed_out, &row_env);
        let passed = match result.value {
            FelValue::Boolean(b) => b,
            other => {
                return Err(EvalError::Guard(format!(
                    "K-053: decisionTable output cell on row '{}' (table '{table_ref}') did not evaluate to boolean (got {})",
                    selected_row.id,
                    other.type_name()
                )));
            }
        };

        self.guard_evaluations.push(GuardEvaluation {
            guard_id: format!("{source_state}->{target_state}:{event}"),
            source_state: source_state.to_string(),
            target_state: target_state.to_string(),
            event: event.to_string(),
            expression: synthesized_expr,
            result: passed,
            inputs: row_scope_inputs,
        });

        Ok(passed)
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
                if let Some(history_mode) = &indexed.state.history_state {
                    self.capture_history(state_id, *history_mode);
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

    /// Capture the active substate configuration of a compound state.
    ///
    /// **Shallow:** record only the direct active substates.
    /// **Deep:** record all active leaf states within the compound subtree.
    fn capture_history(&mut self, compound_id: &str, mode: HistoryMode) {
        let indexed = match self.state_index.get(compound_id) {
            Some(s) => s,
            None => return,
        };

        let direct_substates: Vec<String> = indexed
            .state
            .states
            .keys()
            .filter(|id| self.config.contains(*id))
            .cloned()
            .collect();

        let saved = match mode {
            HistoryMode::Shallow => direct_substates,
            HistoryMode::Deep => {
                let mut leaves = Vec::new();
                for sub_id in &direct_substates {
                    self.collect_deep_leaves(sub_id, &mut leaves);
                }
                leaves
            }
        };

        if !saved.is_empty() {
            self.history_store.insert(compound_id.to_string(), saved);
        }
    }

    /// Collect all active leaf states within a subtree.
    fn collect_deep_leaves(&self, state_id: &str, leaves: &mut Vec<String>) {
        if let Some(indexed) = self.state_index.get(state_id) {
            match indexed.state.kind {
                StateKind::Compound => {
                    let has_active_child = indexed
                        .state
                        .states
                        .keys()
                        .any(|id| self.config.contains(id));
                    if has_active_child {
                        for sub_id in indexed.state.states.keys() {
                            if self.config.contains(sub_id) {
                                self.collect_deep_leaves(sub_id, leaves);
                            }
                        }
                    } else if self.config.contains(state_id) {
                        leaves.push(state_id.to_string());
                    }
                }
                StateKind::Parallel => {
                    if self.config.contains(state_id) {
                        for region in indexed.state.regions.values() {
                            for region_state_id in region.states.keys() {
                                if self.config.contains(region_state_id) {
                                    self.collect_deep_leaves(region_state_id, leaves);
                                }
                            }
                        }
                    }
                }
                StateKind::Atomic | StateKind::Final => {
                    if self.config.contains(state_id) {
                        leaves.push(state_id.to_string());
                    }
                }
            }
        }
    }

    /// Restore a previously saved history configuration.
    ///
    /// For Shallow history: the saved state is a direct child of the history
    /// compound. `enter_state` recurses normally into its `initialState`.
    ///
    /// For Deep history: the saved states are leaf states. Each leaf may be
    /// nested inside intermediate compound states that were also active. We
    /// reconstruct those intermediate ancestors by walking the `compound_parent`
    /// chain up to (but not including) the history compound, then enter each
    /// ancestor top-down before entering the leaf. Intermediate compound states
    /// are entered without re-invoking their child-init logic (since the leaf
    /// entry handles that).
    fn restore_history(
        &mut self,
        compound_id: &str,
        saved: &[String],
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        for leaf_id in saved {
            // Collect the chain of compound ancestors between the history
            // compound and the saved leaf (exclusive of both endpoints).
            let ancestors = self.compound_ancestors_within(compound_id, leaf_id);

            // Enter each ancestor compound top-down (outermost first).
            // Use direct entry (config + onEntry only) to avoid triggering
            // their normal child-init logic — the leaf entry below handles that.
            for ancestor_id in &ancestors {
                if !self.config.contains(ancestor_id) {
                    self.enter_state_direct(ancestor_id, actor, event_data)?;
                }
            }

            // Enter the saved leaf state itself.
            if !self.config.contains(leaf_id) {
                self.enter_state(leaf_id, actor, event_data)?;
            }
        }

        self.history_store.remove(compound_id);
        Ok(())
    }

    /// Collect all state ancestors of `leaf_id` that sit strictly between
    /// `boundary_id` and the leaf. Returns ancestors in outermost-first order
    /// (closest to `boundary_id` first).
    ///
    /// Walks the `direct_parent` chain, which includes both compound and
    /// parallel parents, so intermediate parallel states are included.
    fn compound_ancestors_within(&self, boundary_id: &str, leaf_id: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = leaf_id;

        loop {
            let parent = match self.state_index.get(current) {
                Some(indexed) => indexed.direct_parent.as_deref(),
                None => break,
            };

            match parent {
                None => break,
                Some(p) if p == boundary_id => break,
                Some(p) => {
                    ancestors.push(p.to_string());
                    current = p;
                }
            }
        }

        ancestors.reverse();
        ancestors
    }

    /// Enter a state into the active configuration and run its `onEntry` actions,
    /// but do NOT recurse into children.
    ///
    /// Used during deep history restore to activate intermediate compound states
    /// without triggering their normal `initialState` or history-restore logic.
    fn enter_state_direct(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        if self
            .state_index
            .get(state_id)
            .ok_or_else(|| EvalError::StateNotFound(state_id.to_string()))?
            .state
            .kind
            == StateKind::Final
        {
            return Ok(());
        }

        self.config.enter(state_id.to_string());
        self.provenance
            .push(ProvenanceRecord::state_entered(state_id));
        self.execute_on_entry_actions(state_id, actor, event_data)?;
        Ok(())
    }

    fn cancel_timers_created_in_state_tree(&mut self, state_id: &str, reason: &str) {
        for timer in self.timers.cancel_in_state(state_id) {
            self.provenance
                .push(ProvenanceRecord::timer_cancelled(&timer.id, reason));
        }

        let indexed = match self.state_index.get(state_id) {
            Some(s) => s.clone(),
            None => return,
        };

        match indexed.state.kind {
            StateKind::Compound => {
                let substate_ids: Vec<String> = indexed.state.states.keys().cloned().collect();
                for sub_id in &substate_ids {
                    self.cancel_timers_created_in_state_tree(sub_id, reason);
                }
            }
            StateKind::Parallel => {
                for region in indexed.state.regions.values() {
                    let region_ids: Vec<String> = region.states.keys().cloned().collect();
                    for region_state_id in &region_ids {
                        self.cancel_timers_created_in_state_tree(region_state_id, reason);
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
    index_states_recursive(&kernel.lifecycle.states, None, None, None, &mut index);
    index
}

/// Recursively index states from a states map.
fn index_states_recursive(
    states: &indexmap::IndexMap<String, State>,
    parallel_parent: Option<&str>,
    region_name: Option<&str>,
    direct_parent: Option<&str>,
    index: &mut HashMap<String, IndexedState>,
) {
    for (name, state) in states {
        index.insert(
            name.clone(),
            IndexedState {
                state: state.clone(),
                parallel_parent: parallel_parent.map(String::from),
                region_name: region_name.map(String::from),
                direct_parent: direct_parent.map(String::from),
            },
        );

        if state.kind == StateKind::Compound {
            index_states_recursive(
                &state.states,
                parallel_parent,
                region_name,
                Some(name),
                index,
            );
        }

        if state.kind == StateKind::Parallel {
            for (rname, region) in &state.regions {
                index_states_recursive(&region.states, Some(name), Some(rname), Some(name), index);
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

    let date_ms = parse_duration_segment(date_part, false).map_err(|_| duration)?;
    let time_ms = parse_duration_segment(time_part, true).map_err(|_| duration)?;

    Ok(date_ms + time_ms)
}

/// Parse a date or time segment of an ISO 8601 duration string.
///
/// Returns `Err(())` when the segment contains an unknown unit letter
/// (e.g., `B` in `P20BD`). Silently accepting unknown units would let a
/// `startTimer` with an unrecognized duration fire at 0ms, which is worse
/// than a loud parse failure.
fn parse_duration_segment(segment: &str, is_time: bool) -> Result<u64, ()> {
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
                    _ => return Err(()),
                }
            } else {
                match ch {
                    'Y' => MS_PER_YEAR,
                    'M' => MS_PER_MONTH,
                    'W' => 7 * MS_PER_DAY,
                    'D' => MS_PER_DAY,
                    _ => return Err(()),
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

    Ok(ms)
}

/// Extract the JSON subset actually referenced by a guard expression.
///
/// Walks FEL dependencies and produces a JSON object nested by namespace
/// (`caseFile.*` / `event.*`). Paths not resolvable against the supplied
/// state are omitted — the output is a lossy teaching-signal snapshot, not
/// a complete evaluation context.
///
/// Wildcard paths (`caseFile.relationships[*].kind`, produced by FEL
/// expressions like `every(caseFile.relationships, $.kind == 'parent')`)
/// are expanded: the `[*]` segment is replaced with the full array, so
/// the teaching signal shows every element the guard reasoned over rather
/// than silently dropping the dependency.
fn build_guard_inputs(
    expr: &Expr,
    case_state: &HashMap<String, serde_json::Value>,
    event_data: Option<&serde_json::Value>,
) -> serde_json::Value {
    let deps = extract_dependencies(expr);
    let mut inputs = serde_json::Map::new();

    for field_path in &deps.fields {
        let (namespace, rest) = match field_path.split_once('.') {
            Some((ns, rest)) => (ns, rest),
            None => continue,
        };

        // Strip trailing `[*]` from the first segment for lookup; we resolve
        // the full array and keep the wildcard implicit in the shape.
        let first_segment = rest.split_once('.').map_or(rest, |(h, _)| h);
        let lookup_head = first_segment.trim_end_matches("[*]");
        let root_value = match namespace {
            "caseFile" => case_state.get(lookup_head),
            "event" => event_data
                .and_then(|ev| ev.as_object())
                .and_then(|obj| obj.get(lookup_head)),
            _ => continue,
        };

        let Some(top_value) = root_value else {
            continue;
        };

        let tail = rest.split_once('.').map_or("", |(_, t)| t);
        let leaf_value = walk_json_path(top_value, tail);
        insert_nested(&mut inputs, namespace, lookup_head, tail, leaf_value);
    }

    serde_json::Value::Object(inputs)
}

/// Snapshot a decision-table row scope as a JSON object for the
/// [`GuardEvaluation::inputs`] teaching-signal payload.
///
/// Each declared input name maps to its bound FEL value rendered to JSON.
/// Unlike [`build_guard_inputs`] (which infers dependencies from a parsed
/// expression and groups by namespace), the row scope is the complete
/// per-row evaluation context for the table — namespace-free by spec
/// (Kernel §4.5.1.3) — so the snapshot mirrors it directly.
fn row_scope_to_json(scope: &HashMap<String, FelValue>) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (name, value) in scope {
        out.insert(name.clone(), fel_to_json(value));
    }
    serde_json::Value::Object(out)
}

/// Navigate dotted tail segments into a JSON value; returns the value itself
/// if the tail is empty and `None` on any missing segment.
fn walk_json_path<'a>(value: &'a serde_json::Value, tail: &str) -> Option<&'a serde_json::Value> {
    if tail.is_empty() {
        return Some(value);
    }
    let mut cursor = value;
    for segment in tail.split('.') {
        let obj = cursor.as_object()?;
        cursor = obj.get(segment)?;
    }
    Some(cursor)
}

/// Insert `value` into `inputs[namespace][head][.tail...]`, preserving nesting.
fn insert_nested(
    inputs: &mut serde_json::Map<String, serde_json::Value>,
    namespace: &str,
    head: &str,
    tail: &str,
    value: Option<&serde_json::Value>,
) {
    let Some(value) = value else { return };
    let ns_entry = inputs
        .entry(namespace.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let Some(ns_map) = ns_entry.as_object_mut() else {
        return;
    };

    if tail.is_empty() {
        ns_map.insert(head.to_string(), value.clone());
        return;
    }

    let head_entry = ns_map
        .entry(head.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let Some(mut cursor) = head_entry.as_object_mut() else {
        return;
    };

    let segments: Vec<&str> = tail.split('.').collect();
    for (i, seg) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            cursor.insert(seg.to_string(), value.clone());
        } else {
            let next = cursor
                .entry(seg.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            let Some(next_map) = next.as_object_mut() else {
                return;
            };
            cursor = next_map;
        }
    }
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

    #[test]
    fn parse_iso_duration_rejects_unknown_units() {
        // `BD` (business-day) is not an ISO 8601 unit. Silently treating it as
        // 0ms means a kernel `startTimer` with `duration: "P20BD"` fires
        // immediately — the caller has no way to know the input was malformed.
        // The parser MUST surface an error so callers can emit an
        // `invalid_duration` provenance record instead of booking a 0ms timer.
        assert!(parse_iso_duration_to_ms("P20BD").is_err());
        assert!(parse_iso_duration_to_ms("PT5Q").is_err());
    }

    // ── Decision-table guard evaluator (Kernel §4.5.1.2) ─────────

    mod decision_table_guard {
        //! Unit tests for [`Evaluator::evaluate_decision_table_guard`] driven
        //! through the public `process_event` surface. Each test constructs
        //! a minimal kernel document with a single guarded transition whose
        //! guard is a `DecisionTableGuard` and asserts the transition fires
        //! (or doesn't) per the algorithm in Kernel §4.5.1.2.

        use super::*;
        use crate::model::decision_table::{
            DecisionTable, DecisionTableGuard, DecisionTableGuardKind, DecisionTableInput,
            DecisionTableOutput, DecisionTableRow, FelType, Guard, HitPolicy, OnNoMatch,
        };
        use crate::model::kernel::*;
        use indexmap::IndexMap;

        fn atomic(transitions: Vec<Transition>) -> State {
            State {
                kind: StateKind::Atomic,
                description: None,
                transitions,
                tags: vec![],
                on_entry: vec![],
                on_exit: vec![],
                initial_state: None,
                states: IndexMap::new(),
                regions: IndexMap::new(),
                cancellation_policy: None,
                history_state: None,
                outcome_code: None,
                extensions: HashMap::new(),
            }
        }

        fn final_state() -> State {
            State {
                kind: StateKind::Final,
                description: None,
                transitions: vec![],
                tags: vec![],
                on_entry: vec![],
                on_exit: vec![],
                initial_state: None,
                states: IndexMap::new(),
                regions: IndexMap::new(),
                cancellation_policy: None,
                history_state: None,
                outcome_code: None,
                extensions: HashMap::new(),
            }
        }

        /// Build a kernel doc with a single guarded transition `start --> end`
        /// triggered by event `decide`, plus the supplied decision-table list.
        fn doc_with_guard(
            tables: Vec<DecisionTable>,
            guard: DecisionTableGuard,
        ) -> KernelDocument {
            let mut states = IndexMap::new();
            states.insert(
                "start".into(),
                atomic(vec![Transition {
                    event: Some(TransitionEvent::from_authoring_trigger("decide")),
                    target: "end".into(),
                    guard: Some(Guard::DecisionTable(guard)),
                    actions: vec![],
                    description: None,
                    tags: vec![],
                }]),
            );
            states.insert("end".into(), final_state());

            KernelDocument {
                wos_workflow: "1.0".to_string(),
                schema: None,
                url: None,
                version: None,
                title: None,
                description: None,
                status: None,
                impact_level: None,
                actors: vec![],
                lifecycle: Lifecycle {
                    initial_state: "start".to_string(),
                    states,
                    milestones: HashMap::new(),
                },
                case_file: None,
                contracts: HashMap::new(),
                provenance: None,
                execution: None,
                evaluation_mode: None,
                max_relationship_event_depth: None,
                decision_tables: tables,
                extensions: HashMap::new(),
            }
        }

        /// Income-bracket eligibility table — 2 rows, returns boolean
        /// `eligible`. Inputs: `income` (number), `householdSize` (integer).
        fn income_eligibility_table(hit_policy: HitPolicy) -> DecisionTable {
            DecisionTable {
                id: "incomeElig".to_string(),
                description: None,
                inputs: vec![
                    DecisionTableInput {
                        name: "income".to_string(),
                        kind: FelType::Number,
                        description: None,
                    },
                    DecisionTableInput {
                        name: "householdSize".to_string(),
                        kind: FelType::Integer,
                        description: None,
                    },
                ],
                outputs: vec![DecisionTableOutput {
                    name: "eligible".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![
                    DecisionTableRow {
                        id: "r-low-income-small-household".to_string(),
                        input_cells: vec![
                            "income <= 1473".to_string(),
                            "householdSize <= 2".to_string(),
                        ],
                        output_cells: vec!["true".to_string()],
                        priority: None,
                        rationale: None,
                    },
                    DecisionTableRow {
                        id: "r-low-income-large-household".to_string(),
                        input_cells: vec![
                            "income <= 2500".to_string(),
                            "householdSize >= 3".to_string(),
                        ],
                        output_cells: vec!["true".to_string()],
                        priority: None,
                        rationale: None,
                    },
                ],
                hit_policy,
            }
        }

        fn income_guard() -> DecisionTableGuard {
            let mut bindings = IndexMap::new();
            bindings.insert("income".to_string(), "caseFile.income".to_string());
            bindings.insert(
                "householdSize".to_string(),
                "caseFile.householdSize".to_string(),
            );
            DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "incomeElig".to_string(),
                output_column: "eligible".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            }
        }

        #[test]
        fn happy_path_low_income_small_household_fires() {
            let doc = doc_with_guard(
                vec![income_eligibility_table(HitPolicy::First)],
                income_guard(),
            );
            let case = serde_json::json!({"income": 1200, "householdSize": 2});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            assert!(eval.process_event("decide", None, None).unwrap());
            assert!(eval.configuration().contains("end"));
            // Trace was recorded.
            let traces = eval.guard_evaluations();
            assert_eq!(traces.len(), 1);
            assert!(traces[0].expression.starts_with("decisionTable("));
            assert!(traces[0].result);
        }

        #[test]
        fn no_match_with_default_returns_false() {
            // income too high to match either row.
            let doc = doc_with_guard(
                vec![income_eligibility_table(HitPolicy::First)],
                income_guard(),
            );
            let case = serde_json::json!({"income": 9999, "householdSize": 2});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            assert!(!eval.process_event("decide", None, None).unwrap());
            assert!(eval.configuration().contains("start"));
            let traces = eval.guard_evaluations();
            assert_eq!(traces.len(), 1);
            assert!(!traces[0].result);
        }

        #[test]
        fn no_match_with_fail_returns_err() {
            let mut g = income_guard();
            g.on_no_match = Some(OnNoMatch::Fail);
            let doc = doc_with_guard(vec![income_eligibility_table(HitPolicy::First)], g);
            let case = serde_json::json!({"income": 9999, "householdSize": 2});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected onNoMatch=fail to surface as Err");
            match err {
                EvalError::Guard(msg) => assert!(msg.contains("onNoMatch")),
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn first_hit_policy_picks_first_match_in_document_order() {
            // Build a table where two rows BOTH match — first declared row
            // returns true, second returns false. `first` MUST pick the first.
            let table = DecisionTable {
                id: "ordered".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![
                    DecisionTableRow {
                        id: "r-first".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["true".to_string()],
                        priority: None,
                        rationale: None,
                    },
                    DecisionTableRow {
                        id: "r-second".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["false".to_string()],
                        priority: None,
                        rationale: None,
                    },
                ],
                hit_policy: HitPolicy::First,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "ordered".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            assert!(eval.process_event("decide", None, None).unwrap());
        }

        #[test]
        fn unique_with_multiple_matches_returns_err() {
            // Sanity probe: a case that matches exactly one row succeeds
            // under hitPolicy=unique (no K-052 raised).
            let case_one = serde_json::json!({"income": 1200, "householdSize": 2});
            let mut eval = Evaluator::with_time_and_case_state(
                doc_with_guard(
                    vec![income_eligibility_table(HitPolicy::Unique)],
                    income_guard(),
                ),
                0,
                Some(&case_one),
            )
            .unwrap();
            assert!(eval.process_event("decide", None, None).unwrap());

            // Now build a case that matches BOTH rows: a custom table with
            // two overlapping rows under hitPolicy=unique MUST raise K-052.
            let overlap_table = DecisionTable {
                id: "overlap".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![
                    DecisionTableRow {
                        id: "r-a".to_string(),
                        input_cells: vec!["x > 0".to_string()],
                        output_cells: vec!["true".to_string()],
                        priority: None,
                        rationale: None,
                    },
                    DecisionTableRow {
                        id: "r-b".to_string(),
                        input_cells: vec!["x > 0".to_string()],
                        output_cells: vec!["true".to_string()],
                        priority: None,
                        rationale: None,
                    },
                ],
                hit_policy: HitPolicy::Unique,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let overlap_guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "overlap".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![overlap_table], overlap_guard);
            let case_overlap = serde_json::json!({"x": 5});
            let mut eval =
                Evaluator::with_time_and_case_state(doc, 0, Some(&case_overlap)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-052 violation at runtime");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-052"), "expected K-052 in: {msg}");
                    assert!(msg.contains("unique"), "expected 'unique' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn priority_picks_lowest_priority_among_matches() {
            let table = DecisionTable {
                id: "prio".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![
                    DecisionTableRow {
                        id: "r-low-prio".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["false".to_string()],
                        priority: Some(10),
                        rationale: None,
                    },
                    DecisionTableRow {
                        id: "r-high-prio".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["true".to_string()],
                        priority: Some(1),
                        rationale: None,
                    },
                ],
                hit_policy: HitPolicy::Priority,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "prio".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            // Lowest priority integer wins → r-high-prio (priority=1, output=true).
            assert!(eval.process_event("decide", None, None).unwrap());
        }

        #[test]
        fn priority_tie_among_matches_is_err() {
            let table = DecisionTable {
                id: "tie".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![
                    DecisionTableRow {
                        id: "r-a".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["true".to_string()],
                        priority: Some(5),
                        rationale: None,
                    },
                    DecisionTableRow {
                        id: "r-b".to_string(),
                        input_cells: vec!["x >= 0".to_string()],
                        output_cells: vec!["false".to_string()],
                        priority: Some(5),
                        rationale: None,
                    },
                ],
                hit_policy: HitPolicy::Priority,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "tie".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-052 priority tie");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-052"), "expected K-052 in: {msg}");
                    assert!(msg.contains("priority"), "expected 'priority' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn missing_table_ref_returns_err() {
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "nonexistent".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            // No tables declared on the document.
            let doc = doc_with_guard(vec![], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-051 missing table ref");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-051"), "expected K-051 in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn non_boolean_output_cell_is_err() {
            let table = DecisionTable {
                id: "stringy".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![DecisionTableRow {
                    id: "r-bad".to_string(),
                    input_cells: vec!["x >= 0".to_string()],
                    // Output cell evaluates to a STRING, not a boolean —
                    // even though the output column declares boolean.
                    output_cells: vec!["\"yes\"".to_string()],
                    priority: None,
                    rationale: None,
                }],
                hit_policy: HitPolicy::First,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "stringy".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-053 non-boolean output cell");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-053"), "expected K-053 in: {msg}");
                    assert!(msg.contains("output cell"), "expected 'output cell' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn non_boolean_input_cell_is_err() {
            let table = DecisionTable {
                id: "bad-in".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![DecisionTableRow {
                    id: "r-bad".to_string(),
                    // Input cell evaluates to a NUMBER (not boolean).
                    input_cells: vec!["x + 1".to_string()],
                    output_cells: vec!["true".to_string()],
                    priority: None,
                    rationale: None,
                }],
                hit_policy: HitPolicy::First,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "bad-in".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-053 non-boolean input cell");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-053"), "expected K-053 in: {msg}");
                    assert!(msg.contains("input cell"), "expected 'input cell' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn collect_hit_policy_is_err_for_guard_usage() {
            let table = DecisionTable {
                id: "collect-bad".to_string(),
                description: None,
                inputs: vec![DecisionTableInput {
                    name: "x".to_string(),
                    kind: FelType::Number,
                    description: None,
                }],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![DecisionTableRow {
                    id: "r1".to_string(),
                    input_cells: vec!["x >= 0".to_string()],
                    output_cells: vec!["true".to_string()],
                    priority: None,
                    rationale: None,
                }],
                hit_policy: HitPolicy::Collect,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "collect-bad".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-053 collect on guard");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-053"), "expected K-053 in: {msg}");
                    assert!(msg.contains("collect"), "expected 'collect' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn missing_input_binding_is_err() {
            // Guard binds `x` but table also declares `y`.
            let table = DecisionTable {
                id: "two-inputs".to_string(),
                description: None,
                inputs: vec![
                    DecisionTableInput {
                        name: "x".to_string(),
                        kind: FelType::Number,
                        description: None,
                    },
                    DecisionTableInput {
                        name: "y".to_string(),
                        kind: FelType::Number,
                        description: None,
                    },
                ],
                outputs: vec![DecisionTableOutput {
                    name: "ok".to_string(),
                    kind: FelType::Boolean,
                    description: None,
                }],
                rows: vec![DecisionTableRow {
                    id: "r1".to_string(),
                    input_cells: vec!["x >= 0".to_string(), "y >= 0".to_string()],
                    output_cells: vec!["true".to_string()],
                    priority: None,
                    rationale: None,
                }],
                hit_policy: HitPolicy::First,
            };
            let mut bindings = IndexMap::new();
            bindings.insert("x".to_string(), "caseFile.x".to_string());
            // intentionally NOT binding `y`
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "two-inputs".to_string(),
                output_column: "ok".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"x": 5, "y": 5});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-051 missing inputBindings entry");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-051"), "expected K-051 in: {msg}");
                    assert!(msg.contains("inputBindings"), "expected 'inputBindings' in: {msg}");
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }

        #[test]
        fn output_column_does_not_resolve_is_err() {
            let table = income_eligibility_table(HitPolicy::First);
            let mut bindings = IndexMap::new();
            bindings.insert("income".to_string(), "caseFile.income".to_string());
            bindings.insert(
                "householdSize".to_string(),
                "caseFile.householdSize".to_string(),
            );
            let guard = DecisionTableGuard {
                kind: DecisionTableGuardKind::DecisionTable,
                table_ref: "incomeElig".to_string(),
                output_column: "doesNotExist".to_string(),
                input_bindings: bindings,
                on_no_match: None,
            };
            let doc = doc_with_guard(vec![table], guard);
            let case = serde_json::json!({"income": 1200, "householdSize": 2});
            let mut eval = Evaluator::with_time_and_case_state(doc, 0, Some(&case)).unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-051 outputColumn does not resolve");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-051"), "expected K-051 in: {msg}");
                    assert!(
                        msg.contains("outputColumn"),
                        "expected 'outputColumn' in: {msg}"
                    );
                }
                other => panic!("expected EvalError::Guard, got {other:?}"),
            }
        }
    }
}
