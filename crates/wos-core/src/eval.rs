// Rust guideline compliant 2026-02-21

//! Deterministic lifecycle evaluation algorithm.
//!
//! Implements the algorithm from the Lifecycle Detail Companion (S2):
//! document-order guard evaluation, first-match-wins, exit innermost
//! first, enter outermost first.
//!
//! Operates on typed [`KernelDocument`] models, not raw JSON.

use std::collections::{HashMap, HashSet};

use fel_core::{
    ast::Expr, dependencies::extract_dependencies, evaluate, fel_to_json, parse, types::Value,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::context::EvalContext;
use crate::instance::WorkflowProcess;
use crate::model::due_process::TransitionLocation;
use crate::model::kernel::{
    Action, ActionKind, CancellationPolicy, HistoryMode, KernelDocument, MergeStrategy, Region,
    State, StateKind, Transition, TransitionEvent,
};
use crate::timer::Timers;
use stack_common_typeid::TypeIdMinter;
use wos_events::provenance::{CaseFileSnapshot, ProvenanceLog, ProvenanceRecord};

/// Active state configuration tracking leaf states.
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    /// Currently active leaf state identifiers.
    active: Vec<String>,
    // Entered composite scopes are logical activation, not persisted leaves.
    scopes: HashSet<String>,
}

impl Configuration {
    /// The currently active states.
    pub fn active_states(&self) -> &[String] {
        &self.active
    }

    /// Whether a leaf or its enclosing logical scope is active.
    pub fn contains(&self, state_id: &str) -> bool {
        self.active.iter().any(|s| s == state_id) || self.scopes.contains(state_id)
    }

    /// Add a state to the active configuration.
    pub fn enter(&mut self, state_id: String) {
        if !self.active.contains(&state_id) {
            self.active.push(state_id);
        }
    }

    fn enter_scope(&mut self, state_id: String) {
        self.scopes.insert(state_id);
    }

    /// Remove a state from the active configuration.
    pub fn exit(&mut self, state_id: &str) {
        self.active.retain(|s| s != state_id);
        self.scopes.remove(state_id);
    }
}

/// Actual dispatch result; exited scopes invalidate work even after same-ID reentry.
#[derive(Default)]
struct DispatchOutcome {
    fired: bool,
    exited_roots: Vec<String>,
}
impl DispatchOutcome {
    fn fired(exited_roots: Vec<String>) -> Self {
        Self {
            fired: true,
            exited_roots,
        }
    }
}

/// Flat index entry for a state at any nesting depth.
#[derive(Debug, Clone)]
pub struct IndexedState {
    /// Exact JSON pointer of the authored state.
    pub pointer: String,
    /// The typed state definition.
    pub state: State,
    /// Deterministic authored order for active leaf configuration.
    pub document_order: usize,
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

    initial_case_state: serde_json::Value,
    entered_state_pointers: Vec<String>,

    /// Timer tracking.
    timers: Timers,

    /// Provenance log.
    provenance: ProvenanceLog,

    /// Mints provenance-record IDs under the owning process's tenant.
    minter: TypeIdMinter,

    /// Simulated time in milliseconds (for timer tests).
    simulated_time_ms: u64,

    /// All transitions that fired during execution.
    transitions: Vec<ObservedTransition>,

    /// All actions executed during this evaluator lifetime.
    executed_actions: Vec<ObservedAction>,

    /// Case data immediately after each prospective action, for effect preflight.
    action_case_states: Vec<serde_json::Value>,

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

    // Actual exits during synchronous firing/entry, discarded at its boundary.
    exit_contexts: Vec<Vec<String>>,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObservedTransition {
    /// Exact authored transition selected by native evaluation.
    pub location: TransitionLocation,
    /// Actual StateTransition provenance record created for this firing.
    pub provenance_record_id: String,
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
    /// Exact authored transition tested, when evaluated as a lifecycle guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<TransitionLocation>,
    /// Actual selected native decision row and evaluated output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_table: Option<DecisionTableObservation>,
    /// Actual firing record; absent for a tested but unselected guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firing_record_id: Option<String>,
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

/// Actual decision-table selection observed during native evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTableObservation {
    /// Selected authored table identifier.
    pub table_ref: String,
    /// All matching row identifiers in document order.
    pub matched_row_ids: Vec<String>,
    /// Winning row, absent when no row matched.
    pub selected_row_id: Option<String>,
    /// Actual evaluated output column.
    pub output_column: String,
    /// Actual evaluated output, absent when no row matched.
    pub output: Option<serde_json::Value>,
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
    /// Actual emitEvent ActionExecuted record, captured during evaluation.
    pub provenance_record_id: Option<String>,
    /// Nearest actual transition that executed this action, if any.
    pub transition_record_id: Option<String>,
    /// Exact timer present when an explicitly bounded createTask action executed.
    /// Runtime applies its selected calendar to this ephemeral capture; later
    /// timer replacement or cancellation cannot change the task's admission expiry.
    pub task_expiry_timer: Option<crate::timer::Timer>,
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

    /// Timer expression, duration, or timestamp validation failed.
    #[error("timer '{timer_id}': {message}")]
    Timer { timer_id: String, message: String },

    /// `ForEach` state evaluation failure (Kernel §4.3.1) — `collection` did
    /// not parse, did not evaluate to a bounded array, or another iteration
    /// invariant was violated. Sub-PR D-2 introduced this variant alongside
    /// the runtime iteration semantics.
    #[error("foreach error in state '{state}': {message}")]
    ForEach { state: String, message: String },
}

/// Validated timer timing, including optional authored-origin provenance.
struct TimerTiming {
    origin_ms: u64,
    duration_ms: u64,
    deadline_ms: u64,
    origin_at: Option<String>,
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
    ///
    /// # Errors
    /// Returns evaluation errors, including invalid anchored timer input.
    pub fn new(kernel: KernelDocument, minter: &TypeIdMinter) -> Result<Self, EvalError> {
        Self::with_time(kernel, minter, 0)
    }

    /// Create an evaluator using the provided millisecond clock.
    ///
    /// # Errors
    /// Returns evaluation errors, including invalid anchored timer input.
    pub fn with_time(
        kernel: KernelDocument,
        minter: &TypeIdMinter,
        current_time_ms: u64,
    ) -> Result<Self, EvalError> {
        Self::with_time_and_case_state(kernel, minter, current_time_ms, None)
    }

    /// Create an evaluator using the provided millisecond clock and seeded case state.
    ///
    /// # Errors
    /// Returns evaluation errors, including invalid anchored timer input.
    pub fn with_time_and_case_state(
        kernel: KernelDocument,
        minter: &TypeIdMinter,
        current_time_ms: u64,
        initial_case_state: Option<&serde_json::Value>,
    ) -> Result<Self, EvalError> {
        Self::with_time_and_case_state_and_actor(
            kernel,
            minter,
            current_time_ms,
            initial_case_state,
            None,
        )
    }

    /// Initialize evaluation with an explicitly admitted initiating principal.
    ///
    /// `initial_initiating_actor_id` attributes initial lifecycle control actions
    /// to the principal initiating evaluation, not a machine or external-service
    /// executor. The host must admit that identity; `None` remains anonymous.
    /// Initial evaluation has no event data or synthetic triggering event.
    ///
    /// # Errors
    /// Returns evaluation errors, including invalid anchored timer input.
    pub fn with_time_and_case_state_and_actor(
        kernel: KernelDocument,
        minter: &TypeIdMinter,
        current_time_ms: u64,
        initial_case_state: Option<&serde_json::Value>,
        initial_initiating_actor_id: Option<&str>,
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
            initial_case_state: serde_json::json!(seeded_case_state),
            entered_state_pointers: Vec::new(),
            case_state: seeded_case_state,
            timers: Timers::default(),
            provenance: ProvenanceLog::default(),
            minter: minter.clone(),
            simulated_time_ms: current_time_ms,
            transitions: Vec::new(),
            executed_actions: Vec::new(),
            action_case_states: Vec::new(),
            guard_evaluations: Vec::new(),
            history_store: HashMap::new(),
            exit_contexts: Vec::new(),
        };
        eval.enter_state(&initial, initial_initiating_actor_id, None)?;
        Ok(eval)
    }

    /// Restore an evaluator from a serialized workflow process.
    ///
    /// Provenance IDs are minted under the tenant in `instance.process_id`; the
    /// persisted `tenant` field is never a minting source.
    ///
    /// # Errors
    /// Returns [`EvalError::Internal`] when the process ID carries no tenant, and
    /// timer errors for invalid persisted timers.
    pub fn from_instance(
        kernel: KernelDocument,
        instance: &WorkflowProcess,
        current_time_ms: u64,
    ) -> Result<Self, EvalError> {
        let minter = TypeIdMinter::for_parent(&instance.process_id).ok_or_else(|| {
            EvalError::Internal(format!(
                "process id `{}` carries no tenant to mint under",
                instance.process_id
            ))
        })?;
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
                absolute_deadline: timer.absolute_deadline,
                fires_event: timer.event.clone(),
                created_in_state: timer.scope_state.clone().unwrap_or_default(),
                duration_iso: timer
                    .duration_iso
                    .clone()
                    .unwrap_or_else(|| "P0D".to_string()),
                duration_ms: timer.duration_ms.unwrap_or(0),
            });
        }

        let config = restored_configuration(&state_index, &instance.configuration)?;
        Ok(Self {
            kernel,
            state_index,
            config,
            initial_case_state: instance.case_state.clone(),
            entered_state_pointers: Vec::new(),
            case_state: case_state_from_value(&instance.case_state),
            timers,
            provenance: ProvenanceLog::default(),
            minter,
            simulated_time_ms: current_time_ms,
            transitions: Vec::new(),
            executed_actions: Vec::new(),
            action_case_states: Vec::new(),
            guard_evaluations: Vec::new(),
            history_store: instance.history_store.clone(),
            exit_contexts: Vec::new(),
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

    /// The minter that issues this evaluator's provenance-record IDs.
    pub fn minter(&self) -> &TypeIdMinter {
        &self.minter
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

    /// Case data before any prospective entry or transition actions.
    pub fn initial_case_state(&self) -> &serde_json::Value {
        &self.initial_case_state
    }

    /// Exact state pointers entered, including transient and initial states.
    pub fn entered_state_pointers(&self) -> &[String] {
        &self.entered_state_pointers
    }

    /// Case data immediately after each action, in executed-action order.
    pub fn action_case_states(&self) -> &[serde_json::Value] {
        &self.action_case_states
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

    /// Evaluates one actual authored guard against an isolated retained snapshot.
    ///
    /// This invokes the same guard and decision-table evaluators without entering
    /// states, applying actions, firing transitions, or creating firing evidence.
    ///
    /// # Errors
    /// Rejects unresolved locations, absent guards, and native evaluation failures.
    pub fn evaluate_reference_alternative(
        kernel: &KernelDocument,
        minter: &TypeIdMinter,
        location: &TransitionLocation,
        case_state: &serde_json::Value,
        now_ms: u64,
    ) -> Result<GuardEvaluation, String> {
        let definition = serde_json::to_value(kernel).map_err(|error| error.to_string())?;
        let state = definition
            .pointer(&location.state_pointer)
            .ok_or("state pointer does not resolve")?;
        let transition = state
            .get("transitions")
            .and_then(serde_json::Value::as_array)
            .and_then(|transitions| transitions.get(location.transition_index))
            .ok_or("transition index does not resolve")?;
        let transition: Transition =
            serde_json::from_value(transition.clone()).map_err(|error| error.to_string())?;
        if transition.guard.is_none() {
            return Err("reference alternative requires an actual selected guard".into());
        }
        if !case_state.is_object() {
            return Err("reference alternative case snapshot must be an object".into());
        }
        let mut evaluator = Self {
            minter: minter.clone(),
            kernel: kernel.clone(),
            state_index: build_state_index(kernel),
            config: Configuration::default(),
            initial_case_state: case_state.clone(),
            entered_state_pointers: Vec::new(),
            case_state: case_state_from_value(case_state),
            timers: Timers::default(),
            provenance: ProvenanceLog::default(),
            simulated_time_ms: now_ms,
            transitions: Vec::new(),
            executed_actions: Vec::new(),
            action_case_states: Vec::new(),
            guard_evaluations: Vec::new(),
            history_store: HashMap::new(),
            exit_contexts: Vec::new(),
        };
        evaluator
            .evaluate_transition_guard(
                transition.guard.as_ref(),
                location,
                None,
                &location.state_pointer,
                &transition.target,
                "$referenceAlternative",
            )
            .map_err(|error| error.to_string())?;
        evaluator
            .guard_evaluations
            .pop()
            .ok_or_else(|| "native guard produced no observation".into())
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
    ///
    /// # Errors
    /// Returns transition evaluation errors, including invalid anchored timer input.
    pub fn process_event(
        &mut self,
        event: &str,
        actor: Option<&str>,
        data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        if self.try_fire_transition(event, actor, data, false)?.fired {
            return Ok(true);
        }

        // Unmatched event — record in provenance, no state change.
        self.provenance.push(ProvenanceRecord::unmatched_event(
            &self.minter,
            event,
            actor,
        ));
        Ok(false)
    }

    /// Advance simulated time and fire expired timers.
    ///
    /// # Errors
    /// Returns errors evaluating transitions triggered by expired timers.
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
            .map(|outcome| outcome.fired)
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
    ) -> Result<DispatchOutcome, EvalError> {
        let dispatch_event = if continuous_rescan {
            CONTINUOUS_RESCAN_EVENT
        } else {
            event
        };

        // Route to parallel parents first.
        let parallel_parents = self.find_parallel_parents();
        for parallel_id in &parallel_parents {
            let outcome = self.try_fire_in_parallel(
                parallel_id,
                event,
                actor,
                event_data,
                continuous_rescan,
            )?;
            if outcome.fired {
                return Ok(outcome);
            }
        }

        // For each active state not inside a parallel we already handled.
        let active_snapshot = self.active_candidates();
        for active_state in &active_snapshot {
            if event != "$join"
                && parallel_parents
                    .iter()
                    .any(|p| self.state_is_in_parallel_region(p, active_state))
            {
                continue;
            }

            let indexed = match self.state_index.get(active_state) {
                Some(s) => s.clone(),
                None => continue,
            };

            if event == "$join"
                && (indexed.state.kind != StateKind::Parallel || !self.parallel_ready(active_state))
            {
                continue;
            }

            for (transition_index, transition) in indexed.state.transitions.iter().enumerate() {
                let location = TransitionLocation {
                    state_pointer: indexed.pointer.clone(),
                    transition_index,
                };
                let event_matches =
                    transition_matches_dispatch(transition, event, continuous_rescan);
                if !event_matches {
                    continue;
                }
                if !self.evaluate_transition_guard(
                    transition.guard.as_ref(),
                    &location,
                    event_data,
                    active_state,
                    &transition.target,
                    dispatch_event,
                )? {
                    continue;
                }

                let exited = self.fire_transition(
                    active_state,
                    &transition.target,
                    &location,
                    dispatch_event,
                    actor,
                    &transition.actions,
                    &transition.tags,
                    event_data,
                )?;
                return Ok(DispatchOutcome::fired(exited));
            }
        }

        Ok(DispatchOutcome::default())
    }

    /// Route an event to active states within a parallel state's regions.
    fn try_fire_in_parallel(
        &mut self,
        parallel_id: &str,
        event: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
        continuous_rescan: bool,
    ) -> Result<DispatchOutcome, EvalError> {
        if event == "$join" {
            return Ok(DispatchOutcome::default());
        }

        let dispatch_event = if continuous_rescan {
            CONTINUOUS_RESCAN_EVENT
        } else {
            event
        };

        let indexed = match self.state_index.get(parallel_id) {
            Some(s) => s.clone(),
            None => return Ok(DispatchOutcome::default()),
        };

        let regions = indexed.state.regions.clone();
        let mut any_fired = false;
        let mut exited_roots = Vec::new();

        'regions: for (region_name, _) in &regions {
            let nested: Vec<_> = self
                .active_candidates()
                .into_iter()
                .filter(|state| {
                    self.state_index
                        .get(state)
                        .is_some_and(|entry| entry.state.kind == StateKind::Parallel)
                        && self.state_is_in_region(parallel_id, region_name, state)
                        && self
                            .ancestors(state)
                            .iter()
                            .skip(1)
                            .find(|parent| {
                                self.state_index
                                    .get(*parent)
                                    .is_some_and(|entry| entry.state.kind == StateKind::Parallel)
                            })
                            .is_some_and(|parent| parent == parallel_id)
                })
                .collect();
            let mut nested_fired = false;
            for child in &nested {
                let nested =
                    self.try_fire_in_parallel(child, event, actor, event_data, continuous_rescan)?;
                nested_fired |= nested.fired;
                let invalidated = self.scope_exited(&nested.exited_roots, parallel_id);
                exited_roots.extend(nested.exited_roots);
                if invalidated || !self.config.contains(parallel_id) {
                    return Ok(DispatchOutcome {
                        fired: true,
                        exited_roots,
                    });
                }
            }
            if nested_fired {
                any_fired = true;
                let actions_start = self.executed_actions.len();
                let firing_id = self
                    .transitions
                    .last()
                    .expect("nested dispatch fired")
                    .provenance_record_id
                    .clone();
                self.apply_parallel_cancellation_policy(
                    parallel_id,
                    &regions,
                    region_name,
                    actor,
                    event_data,
                )?;
                self.bind_emitted_action_origins(actions_start, &firing_id);
                continue 'regions;
            }
            let candidates: Vec<_> = self
                .active_candidates()
                .into_iter()
                .filter(|state| {
                    self.state_is_in_region(parallel_id, region_name, state)
                        && !nested
                            .iter()
                            .any(|parent| self.is_descendant(state, parent))
                })
                .collect();
            for active in candidates {
                let state_def = self
                    .state_index
                    .get(&active)
                    .ok_or_else(|| EvalError::StateNotFound(active.clone()))?
                    .clone();
                if state_def.state.kind == StateKind::Final {
                    continue;
                }
                for (transition_index, transition) in state_def.state.transitions.iter().enumerate()
                {
                    if !transition_matches_dispatch(transition, event, continuous_rescan) {
                        continue;
                    }
                    let location = TransitionLocation {
                        state_pointer: state_def.pointer.clone(),
                        transition_index,
                    };
                    if !self.evaluate_transition_guard(
                        transition.guard.as_ref(),
                        &location,
                        event_data,
                        &active,
                        &transition.target,
                        dispatch_event,
                    )? {
                        continue;
                    }
                    let actions_start = self.executed_actions.len();
                    let exited = self.fire_transition(
                        &active,
                        &transition.target,
                        &location,
                        dispatch_event,
                        actor,
                        &transition.actions,
                        &transition.tags,
                        event_data,
                    )?;
                    any_fired = true;
                    let invalidated = self.scope_exited(&exited, parallel_id);
                    exited_roots.extend(exited);
                    // Actual exit invalidates old work even when this same ID reentered.
                    if invalidated || !self.config.contains(parallel_id) {
                        return Ok(DispatchOutcome {
                            fired: true,
                            exited_roots,
                        });
                    }
                    let firing_id = self
                        .transitions
                        .last()
                        .expect("recorded transition")
                        .provenance_record_id
                        .clone();
                    self.apply_parallel_cancellation_policy(
                        parallel_id,
                        &regions,
                        region_name,
                        actor,
                        event_data,
                    )?;
                    self.bind_emitted_action_origins(actions_start, &firing_id);
                    continue 'regions;
                }
            }
        }
        // Preserve completed regional leaves unless a real parent transition exits.
        if any_fired && self.parallel_ready(parallel_id) {
            let joined = self.try_fire_transition("$join", actor, event_data, false)?;
            if !joined.fired {
                self.provenance.push(ProvenanceRecord::unmatched_event(
                    &self.minter,
                    "$join",
                    actor,
                ));
            }
            exited_roots.extend(joined.exited_roots);
        }

        Ok(DispatchOutcome {
            fired: any_fired,
            exited_roots,
        })
    }

    fn apply_parallel_cancellation_policy(
        &mut self,
        parallel_id: &str,
        regions: &indexmap::IndexMap<String, Region>,
        fired_region_name: &str,
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
        if !parallel_cancellation_triggered(parallel_state, &self.config) {
            return Ok(());
        }

        for (region_name, region) in regions {
            if region_name == fired_region_name {
                continue;
            }
            let active_states: Vec<String> = region
                .states
                .keys()
                .filter(|state| self.config.contains(state))
                .cloned()
                .collect();
            for active_state in active_states {
                self.execute_subtree_exit_actions(&active_state, actor, event_data)?;
                self.cancel_timers_created_in_state_tree(&active_state, "region-cancellation");
                self.exit_state_and_descendants(&active_state);
                self.clear_subtree_history(&active_state, "region-cancellation");
                self.record_region_cancellation(parallel_id, region_name, &active_state);
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
        self.exit_contexts.push(Vec::new());
        let result = self.enter_state_inner(state_id, actor, event_data);
        self.finish_exit_context();
        result
    }

    fn enter_state_inner(
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

        self.entered_state_pointers.push(indexed.pointer.clone());
        match indexed.state.kind {
            StateKind::Compound => {
                self.config.enter_scope(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(&self.minter, state_id));
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
                self.config.enter_scope(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(&self.minter, state_id));
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                for region in indexed.state.regions.values() {
                    let checkpoint = self.exit_checkpoint();
                    self.enter_state(&region.initial_state, actor, event_data)?;
                    if self.scope_exited_since(checkpoint, state_id) {
                        break;
                    }
                }
                Ok(())
            }
            StateKind::ForEach => {
                // ForEach iteration semantics per Kernel §4.3.1 (Sub-PR D-2).
                //
                //   1. Enter state, run onEntry actions.
                //   2. Evaluate `collection` FEL → Vec<Value>; reject non-array.
                //   3. Empty-collection fast path: skip iteration loop entirely.
                //   4. For each item:
                //      - bind item under `itemVariable` (default `$item`),
                //        index under `indexVariable` (default `$index`).
                //      - emit ForEachIterationStarted provenance.
                //      - run the `body` subtree (atomic onEntry/onExit; compound
                //        walk to Final; output_path merge after body).
                //      - check `breakCondition` (FEL predicate); set
                //        break_triggered if true.
                //      - emit ForEachIterationCompleted provenance.
                //      - if break_triggered, exit loop.
                //   5. Restore prior `$item` / `$index` bindings.
                //   6. Emit ForEachCompleted provenance.
                //   7. Fire the foreach state's first eligible anonymous
                //      outgoing transition (if any). When no eligible
                //      transition exists, the foreach state stays active —
                //      this matches kernel-level non-final-state semantics.

                self.config.enter(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(&self.minter, state_id));
                self.execute_on_entry_actions(state_id, actor, event_data)?;

                let collection_expr =
                    indexed
                        .state
                        .collection
                        .clone()
                        .ok_or_else(|| EvalError::ForEach {
                            state: state_id.to_string(),
                            message: "missing required `collection` FEL expression".to_string(),
                        })?;

                let item_var = indexed
                    .state
                    .item_variable
                    .clone()
                    .unwrap_or_else(|| "$item".to_string());
                let index_var = indexed
                    .state
                    .index_variable
                    .clone()
                    .unwrap_or_else(|| "$index".to_string());
                let break_expr = indexed.state.break_condition.clone();

                let items =
                    self.evaluate_foreach_collection(state_id, &collection_expr, event_data)?;

                // Save prior bindings so foreach is transparent w.r.t. case
                // state — per spec, per-iteration bindings do NOT persist into
                // case state after the foreach completes.
                let prior_item = self.case_state.get(&item_var).cloned();
                let prior_index = self.case_state.get(&index_var).cloned();

                let mut iterations: u32 = 0;
                let mut broke = false;

                for (i, item) in items.iter().enumerate() {
                    let i_u32 = u32::try_from(i).map_err(|_| EvalError::ForEach {
                        state: state_id.to_string(),
                        message: format!("iteration index {i} exceeds u32"),
                    })?;

                    self.case_state.insert(item_var.clone(), item.clone());
                    self.case_state
                        .insert(index_var.clone(), serde_json::Value::Number(i_u32.into()));

                    self.provenance
                        .push(ProvenanceRecord::foreach_iteration_started(
                            &self.minter,
                            state_id,
                            i_u32,
                            item,
                        ));

                    iterations += 1;

                    // Body execution per Sub-PR D-3.
                    //
                    //   - Atomic body: per-iteration `body.onEntry` actions
                    //     run with the current `$item` / `$index` bindings
                    //     visible in case state, then `body.onExit` actions.
                    //   - Compound / parallel body (transitions inside the
                    //     body subtree, nested state machines): tracked as
                    //     Sub-PR D-5. The body's `kind` field is read but
                    //     non-atomic kinds are accepted-and-ignored at the
                    //     transition level for now; their `onEntry` and
                    //     `onExit` actions still run.
                    //
                    // The synthetic lifecycle-state label `<state>:body`
                    // propagates into action / mutation provenance so audit
                    // tooling can attribute mutations to the body of a
                    // specific foreach state.
                    let body_state_label = format!("{state_id}:body");
                    if let Some(body) = indexed.state.body.as_deref() {
                        match body.kind {
                            // Atomic body: onEntry actions, then onExit
                            // actions. The body has no internal transition
                            // graph to walk.
                            StateKind::Atomic => {
                                let entry_actions = body.on_entry.clone();
                                for action in &entry_actions {
                                    let action_name = action_kind_camel(action.action);
                                    self.provenance.push(ProvenanceRecord::on_entry(
                                        &self.minter,
                                        &body_state_label,
                                        action_name,
                                    ));
                                    self.execute_action_in_state(
                                        action,
                                        actor,
                                        &body_state_label,
                                        event_data,
                                    )?;
                                }
                                let exit_actions = body.on_exit.clone();
                                for action in &exit_actions {
                                    let action_name = action_kind_camel(action.action);
                                    self.provenance.push(ProvenanceRecord::on_exit(
                                        &self.minter,
                                        &body_state_label,
                                        action_name,
                                    ));
                                    self.execute_action_in_state(
                                        action,
                                        actor,
                                        &body_state_label,
                                        event_data,
                                    )?;
                                }
                            }
                            // Compound body: walk the body's substate graph
                            // to a Final state, firing anonymous transitions
                            // in document order. Sub-PR D-5.
                            StateKind::Compound => {
                                let body_clone = body.clone();
                                self.run_compound_body_to_completion(
                                    state_id,
                                    &body_clone,
                                    actor,
                                    event_data,
                                )?;
                            }
                            // Parallel / ForEach / Final body kinds are not
                            // yet implemented at runtime. The schema admits
                            // parallel bodies; nested foreach is permitted
                            // but discouraged per spec §4.3.1. Reject so
                            // authors get a clear signal rather than silently
                            // skipping unsupported body kinds.
                            other => {
                                return Err(EvalError::ForEach {
                                    state: state_id.to_string(),
                                    message: format!(
                                        "body.kind = {other:?} is not yet implemented at \
                                         runtime; supported body kinds are atomic and compound \
                                         (Sub-PR D-5). Parallel-body and nested-foreach iteration \
                                         are tracked as a follow-up runtime PR."
                                    ),
                                });
                            }
                        }
                    }

                    // Per-iteration output write per Sub-PR D-4. The
                    // post-body value of `case_state[item_var]` is the
                    // per-iteration "output" — for atomic bodies that
                    // body cannot mutate today, this collapses to the input
                    // item, which is still useful for filter / pass-through
                    // patterns. When a future PR adds FEL-evaluated setData
                    // values or nested-path traversal the same hook captures
                    // the transformed value without further surgery.
                    if let Some(output_path) = indexed.state.output_path.as_deref() {
                        let strategy = indexed
                            .state
                            .merge_strategy
                            .unwrap_or(MergeStrategy::Collect);
                        let item_value = self
                            .case_state
                            .get(&item_var)
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        self.merge_foreach_output(
                            state_id,
                            output_path,
                            strategy,
                            item_value,
                            actor,
                        )?;
                    }

                    let mut break_triggered = false;
                    if let Some(ref expr) = break_expr {
                        if self.evaluate_foreach_predicate(state_id, expr, event_data)? {
                            break_triggered = true;
                            broke = true;
                        }
                    }

                    self.provenance
                        .push(ProvenanceRecord::foreach_iteration_completed(
                            &self.minter,
                            state_id,
                            i_u32,
                            break_triggered,
                        ));

                    if break_triggered {
                        break;
                    }
                }

                // Restore prior bindings.
                match prior_item {
                    Some(v) => {
                        self.case_state.insert(item_var.clone(), v);
                    }
                    None => {
                        self.case_state.remove(&item_var);
                    }
                }
                match prior_index {
                    Some(v) => {
                        self.case_state.insert(index_var.clone(), v);
                    }
                    None => {
                        self.case_state.remove(&index_var);
                    }
                }

                self.provenance.push(ProvenanceRecord::foreach_completed(
                    &self.minter,
                    state_id,
                    iterations,
                    broke,
                ));

                self.fire_foreach_outgoing(state_id, actor, event_data)?;

                Ok(())
            }
            StateKind::Atomic | StateKind::Final => {
                self.config.enter(state_id.to_string());
                self.provenance
                    .push(ProvenanceRecord::state_entered(&self.minter, state_id));
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
        location: &TransitionLocation,
        event: &str,
        actor: Option<&str>,
        actions: &[Action],
        tags: &[String],
        event_data: Option<&serde_json::Value>,
    ) -> Result<Vec<String>, EvalError> {
        self.exit_contexts.push(Vec::new());
        let result = self.fire_transition_inner(
            source, target, location, event, actor, actions, tags, event_data,
        );
        let exited = self.finish_exit_context();
        result.map(|()| exited)
    }

    fn fire_transition_inner(
        &mut self,
        source: &str,
        target: &str,
        location: &TransitionLocation,
        event: &str,
        actor: Option<&str>,
        actions: &[Action],
        tags: &[String],
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let case_file_snapshot = self.case_file_snapshot_for_transition(tags);
        let actions_start = self.executed_actions.len();

        let (exit_root, entry_path) = self.transition_paths(source, target)?;
        let mut cancelled_regions = Vec::new();
        if let Some(root) = &exit_root {
            self.execute_subtree_exit_actions(root, actor, event_data)?;
            for scope in self
                .active_candidates()
                .into_iter()
                .filter(|state| state == root || self.is_descendant(state, root))
            {
                if let Some(entry) = self
                    .state_index
                    .get(&scope)
                    .filter(|entry| entry.state.kind == StateKind::Parallel)
                {
                    for (region_name, region) in &entry.state.regions {
                        for child in region
                            .states
                            .keys()
                            .filter(|state| self.config.contains(state))
                        {
                            cancelled_regions.push((
                                scope.clone(),
                                region_name.clone(),
                                child.clone(),
                            ));
                        }
                    }
                }
            }
            for (_, _, root) in &cancelled_regions {
                self.cancel_timers_created_in_state_tree(root, "region-cancellation");
            }
        }
        for action in actions {
            self.execute_action_in_state(action, actor, source, event_data)?;
        }
        if let Some(root) = &exit_root {
            self.exit_state_and_descendants(root);
            self.exit_contexts
                .last_mut()
                .expect("active firing context")
                .push(root.clone());
        }
        for (parallel, region, root) in cancelled_regions {
            self.clear_subtree_history(&root, "region-cancellation");
            self.record_region_cancellation(&parallel, &region, &root);
        }
        self.enter_transition_path(&entry_path, actor, event_data)?;
        self.config.active.sort_by_key(|state| {
            self.state_index
                .get(state)
                .map(|entry| entry.document_order)
                .unwrap_or(usize::MAX)
        });

        self.record_transition(
            ObservedTransition {
                location: location.clone(),
                provenance_record_id: String::new(),
                from: source.to_string(),
                to: target.to_string(),
                event: event.to_string(),
                tags: tags.to_vec(),
            },
            actor,
            case_file_snapshot,
            actions_start,
        );

        Ok(())
    }

    /// Retain an actual transition and bind its still-unbound emissions.
    fn record_transition(
        &mut self,
        mut transition: ObservedTransition,
        actor: Option<&str>,
        case_file_snapshot: Option<CaseFileSnapshot>,
        actions_start: usize,
    ) {
        let mut record = ProvenanceRecord::tagged_state_transition(
            &self.minter,
            &transition.from,
            &transition.to,
            &transition.event,
            actor,
            &transition.tags,
            case_file_snapshot,
        );
        let data = record.data.get_or_insert_with(|| serde_json::json!({}));
        data["nativeTransitionLocation"] =
            serde_json::to_value(&transition.location).expect("transition location serializes");
        let record_id = record.id.clone();
        transition.provenance_record_id = record_id.clone();
        if let Some(guard) = self.guard_evaluations.iter_mut().rev().find(|guard| {
            guard.location.as_ref() == Some(&transition.location)
                && guard.result
                && guard.firing_record_id.is_none()
        }) {
            guard.firing_record_id = Some(record_id.clone());
        }
        self.provenance.push(record);
        self.bind_emitted_action_origins(actions_start, &record_id);
        self.transitions.push(transition);
    }

    /// Bind still-unbound actions to their nearest actual transition.
    fn bind_emitted_action_origins(&mut self, start: usize, transition_record_id: &str) {
        for action in &mut self.executed_actions[start..] {
            // Nested transitions already own their actions; initial entry has no
            // transition to invent. Every governed effect retains its real origin.
            if action.transition_record_id.is_none() {
                action.transition_record_id = Some(transition_record_id.to_string());
            }
        }
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
        if action.callback_phase.is_some() && action.action != ActionKind::InvokeService {
            return Err(EvalError::Internal(
                "callbackPhase is valid only for invokeService".into(),
            ));
        }
        if action.expires_with_timer.is_some() && action.action != ActionKind::CreateTask {
            return Err(EvalError::Internal(
                "expiresWithTimer is valid only for createTask".into(),
            ));
        }
        if action.start_at_expression.is_some() && action.action != ActionKind::StartTimer {
            return Err(EvalError::Timer {
                timer_id: action.timer_id.clone().unwrap_or_default(),
                message: "startAtExpression is valid only for startTimer".into(),
            });
        }
        let task_expiry_timer = if action.action == ActionKind::CreateTask {
            match action.expires_with_timer.as_deref() {
                None => None,
                Some(timer_id) => {
                    if timer_id.trim().is_empty() {
                        return Err(EvalError::Internal("expiresWithTimer is blank".into()));
                    }
                    let mut matching = self.timers.iter().filter(|timer| timer.id == timer_id);
                    let timer = matching.next().ok_or_else(|| {
                        EvalError::Internal(format!(
                            "expiresWithTimer references absent timer '{timer_id}'"
                        ))
                    })?;
                    if matching.next().is_some() {
                        return Err(EvalError::Internal(
                            "expiresWithTimer references duplicate timers".into(),
                        ));
                    }
                    Some(timer.clone())
                }
            }
        } else {
            None
        };
        let mut provenance_record_id = None;
        match action.action {
            ActionKind::SetData => {
                let path = action.path.as_deref().unwrap_or("");
                let value = action.value.clone().unwrap_or(serde_json::Value::Null);

                let key = path.strip_prefix("caseFile.").unwrap_or(path);
                self.case_state.insert(key.to_string(), value.clone());

                self.provenance.push(ProvenanceRecord::case_state_mutation(
                    &self.minter,
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

                // Resolve and validate anchored input before replacing an existing timer.
                let timing = if let Some(expression) = action.start_at_expression.as_deref() {
                    self.anchored_timer_timing(action, expression, event_data)?
                } else if let Some(deadline) = action.deadline.as_deref() {
                    self.absolute_timer_timing(action, deadline)?
                } else {
                    // Preserve unanchored duration parsing and current-time behavior.
                    let duration_ms = parse_iso_duration_to_ms(duration).unwrap_or_else(|raw| {
                        self.provenance.push(ProvenanceRecord::invalid_duration(
                            &self.minter,
                            raw,
                            timer_id,
                        ));
                        0
                    });
                    TimerTiming {
                        origin_ms: self.simulated_time_ms,
                        duration_ms,
                        deadline_ms: self.simulated_time_ms + duration_ms,
                        origin_at: None,
                    }
                };

                // Cancel existing timer with same ID (reentry, Lifecycle Detail S6.4).
                if self.timers.cancel(timer_id).is_some() {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        &self.minter,
                        timer_id,
                        "reentry-cancel",
                    ));
                }

                let duration_iso = if action.deadline.is_some() {
                    format!(
                        "PT{}.{:03}S",
                        timing.duration_ms / 1_000,
                        timing.duration_ms % 1_000
                    )
                } else {
                    duration.to_string()
                };
                self.timers.create(crate::timer::Timer {
                    id: timer_id.to_string(),
                    created_at_ms: timing.origin_ms,
                    deadline_ms: timing.deadline_ms,
                    absolute_deadline: action.deadline.is_some(),
                    fires_event: fires_event.to_string(),
                    created_in_state: lifecycle_state.to_string(),
                    duration_iso: duration_iso.clone(),
                    duration_ms: timing.duration_ms,
                });

                let mut created = ProvenanceRecord::timer_created(
                    &self.minter,
                    timer_id,
                    &duration_iso,
                    fires_event.as_str(),
                );
                if let Some(deadline) = &action.deadline {
                    if let Some(data) = created.data.as_mut().and_then(|data| data.as_object_mut())
                    {
                        data.remove("duration");
                        data.insert(
                            "deadline".into(),
                            serde_json::Value::String(deadline.clone()),
                        );
                    }
                }
                if let Some(origin_at) = timing.origin_at {
                    if let Some(data) = created.data.as_mut().and_then(|data| data.as_object_mut())
                    {
                        data.insert("originAt".into(), serde_json::Value::String(origin_at));
                        data.insert(
                            "startAtExpression".into(),
                            serde_json::Value::String(
                                action.start_at_expression.clone().unwrap_or_default(),
                            ),
                        );
                    }
                }
                self.provenance.push(created);
            }
            ActionKind::CancelTimer => {
                let timer_id = action.timer_id.as_deref().unwrap_or("");
                if self.timers.cancel(timer_id).is_some() {
                    self.provenance.push(ProvenanceRecord::timer_cancelled(
                        &self.minter,
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
                let mut action_record = ProvenanceRecord::action_executed(
                    &self.minter,
                    lifecycle_state,
                    action_name_camel,
                );
                action_record.actor_id = actor.map(String::from);
                if action.action == ActionKind::EmitEvent {
                    action_record.data.as_mut().expect("action record has data")["eventType"] =
                        serde_json::json!(action.event_type);
                    provenance_record_id = Some(action_record.id.clone());
                }
                self.provenance.push(action_record);
            }
        }

        if self
            .kernel
            .governance
            .as_ref()
            .and_then(|value| value.pointer("/dueProcess/execution"))
            .is_some()
        {
            self.action_case_states.push(self.case_state_json());
        }
        self.executed_actions.push(ObservedAction {
            provenance_record_id,
            transition_record_id: None,
            lifecycle_state: lifecycle_state.to_string(),
            actor_id: actor.map(String::from),
            action: action.clone(),
            event_data: event_data.cloned(),
            task_expiry_timer,
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
            self.provenance.push(ProvenanceRecord::on_entry(
                &self.minter,
                state_id,
                action_name,
            ));
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
            self.provenance.push(ProvenanceRecord::on_exit(
                &self.minter,
                state_id,
                action_name,
            ));
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
        location: &TransitionLocation,
        event_data: Option<&serde_json::Value>,
        source_state: &str,
        target_state: &str,
        event: &str,
    ) -> Result<bool, EvalError> {
        let start = self.guard_evaluations.len();
        let result = match guard {
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
        };
        for evaluation in &mut self.guard_evaluations[start..] {
            evaluation.location = Some(location.clone());
        }
        result
    }

    /// Validate an absolute deadline before replacing a timer.
    fn absolute_timer_timing(
        &self,
        action: &Action,
        deadline: &str,
    ) -> Result<TimerTiming, EvalError> {
        let fail = |message: String| EvalError::Timer {
            timer_id: action.timer_id.clone().unwrap_or_default(),
            message,
        };
        if action.duration.is_some() {
            return Err(fail("deadline excludes duration".into()));
        }
        if action
            .timer_id
            .as_deref()
            .is_none_or(|id| id.trim().is_empty())
            || action
                .event
                .as_ref()
                .is_none_or(|event| event.start_timer_fires_string().trim().is_empty())
        {
            return Err(fail(
                "absolute timer requires a nonblank timerId and firing event".into(),
            ));
        }
        let deadline_ms = parse_rfc3339_to_ms(deadline).map_err(|error| fail(error.to_string()))?;
        Ok(TimerTiming {
            origin_ms: self.simulated_time_ms,
            duration_ms: deadline_ms.saturating_sub(self.simulated_time_ms),
            deadline_ms,
            origin_at: None,
        })
    }

    /// Validate an anchored duration without mutating timers or provenance.
    fn anchored_timer_timing(
        &self,
        action: &Action,
        expression: &str,
        event_data: Option<&serde_json::Value>,
    ) -> Result<TimerTiming, EvalError> {
        let fail = |message: String| EvalError::Timer {
            timer_id: action.timer_id.clone().unwrap_or_default(),
            message,
        };
        if action
            .timer_id
            .as_deref()
            .is_none_or(|id| id.trim().is_empty())
        {
            return Err(fail("anchored timer requires a nonblank timerId".into()));
        }
        if action
            .event
            .as_ref()
            .is_none_or(|event| event.start_timer_fires_string().trim().is_empty())
        {
            return Err(fail(
                "anchored timer requires a nonblank firing event".into(),
            ));
        }
        if action.deadline.is_some() {
            return Err(fail("startAtExpression excludes deadline".into()));
        }
        let duration = action
            .duration
            .as_deref()
            .ok_or_else(|| fail("startAtExpression requires duration".into()))?;
        // FEL accepts signed durations and an empty trailing time part; timer origins
        // require a nonnegative duration with a complete ISO duration spelling.
        if duration.trim().starts_with('-') || duration.trim().ends_with('T') {
            return Err(fail("invalid anchored timer duration".into()));
        }
        let duration_ms = match fel_core::parse_iso8601_duration(duration) {
            fel_core::IsoDurationParse::Milliseconds(ms) => {
                u64::try_from(ms).map_err(|_| fail("negative anchored timer duration".into()))?
            }
            fel_core::IsoDurationParse::Invalid => {
                return Err(fail("invalid anchored timer duration".into()));
            }
            fel_core::IsoDurationParse::OutOfRange => {
                return Err(fail("anchored timer duration is out of range".into()));
            }
        };
        let parsed = parse(expression)
            .map_err(|error| fail(format!("anchor expression parse error: {error}")))?;
        let environment =
            EvalContext::from_case_state(&self.case_state, event_data).to_fel_environment();
        let result = evaluate(&parsed, &environment);
        if !result.diagnostics.is_empty() {
            return Err(fail(
                "anchor expression evaluation reported diagnostics".into(),
            ));
        }
        let Value::String(value) = result.value else {
            return Err(fail(
                "anchor must evaluate to an RFC 3339 timestamp string".into(),
            ));
        };
        let timestamp = OffsetDateTime::parse(&value, &Rfc3339)
            .map_err(|error| fail(format!("invalid anchor timestamp: {error}")))?;
        let nanos = timestamp.unix_timestamp_nanos();
        if nanos < 0 {
            return Err(fail("anchor predates Unix epoch".into()));
        }
        if nanos > i128::from(self.simulated_time_ms) * 1_000_000 {
            return Err(fail("anchor is later than current logical time".into()));
        }
        let origin_ms = u64::try_from(nanos / 1_000_000)
            .map_err(|_| fail("anchor timestamp is out of range".into()))?;
        let deadline_ms = origin_ms
            .checked_add(duration_ms)
            .ok_or_else(|| fail("anchored timer deadline overflow".into()))?;
        // Validate the runtime's timestamp representation before timer replacement.
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(deadline_ms) * 1_000_000)
            .map_err(|_| fail("anchored timer deadline timestamp is out of range".into()))?;
        let origin_at =
            OffsetDateTime::from_unix_timestamp_nanos(i128::from(origin_ms) * 1_000_000)
                .map_err(|_| fail("anchor timestamp is out of range".into()))?
                .format(&Rfc3339)
                .map_err(|error| fail(format!("anchor formatting failed: {error}")))?;
        Ok(TimerTiming {
            origin_ms,
            duration_ms,
            deadline_ms,
            origin_at: Some(origin_at),
        })
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
        let passed = matches!(result.value, Value::Boolean(true));

        let inputs = build_guard_inputs(&parsed, &self.case_state, event_data);
        self.guard_evaluations.push(GuardEvaluation {
            location: None,
            decision_table: None,
            firing_record_id: None,
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

    // ── ForEach iteration helpers (Kernel §4.3.1; Sub-PR D-2) ────────────────

    /// Evaluate a `ForEach` state's `collection` FEL expression. Returns the
    /// items as `Vec<serde_json::Value>` for sequential iteration. Rejects
    /// any non-array result with a [`EvalError::ForEach`] — the spec demands
    /// a bounded array; unbounded streams are not supported.
    fn evaluate_foreach_collection(
        &self,
        state_id: &str,
        expression: &str,
        event_data: Option<&serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, EvalError> {
        let ctx = EvalContext::from_case_state(&self.case_state, event_data);
        let env = ctx.to_fel_environment();

        let parsed = parse(expression).map_err(|e| EvalError::ForEach {
            state: state_id.to_string(),
            message: format!("collection parse error in '{expression}': {e}"),
        })?;
        let result = evaluate(&parsed, &env);

        let value = fel_to_json(&result.value);
        match value {
            serde_json::Value::Array(items) => Ok(items),
            other => Err(EvalError::ForEach {
                state: state_id.to_string(),
                message: format!(
                    "collection MUST evaluate to a bounded array; got {} for expression '{}'",
                    json_kind(&other),
                    expression
                ),
            }),
        }
    }

    /// Evaluate a `ForEach` state's `breakCondition` FEL predicate against
    /// case state (with the current iteration's `$item` / `$index` bindings
    /// in place). Returns true to terminate iteration early.
    fn evaluate_foreach_predicate(
        &self,
        state_id: &str,
        expression: &str,
        event_data: Option<&serde_json::Value>,
    ) -> Result<bool, EvalError> {
        let ctx = EvalContext::from_case_state(&self.case_state, event_data);
        let env = ctx.to_fel_environment();

        let parsed = parse(expression).map_err(|e| EvalError::ForEach {
            state: state_id.to_string(),
            message: format!("breakCondition parse error in '{expression}': {e}"),
        })?;
        let result = evaluate(&parsed, &env);
        Ok(matches!(result.value, Value::Boolean(true)))
    }

    /// Walk a compound `body` substate graph to a Final state (Sub-PR D-5).
    ///
    /// Per Kernel §4.3.1: "The body executes to completion (a final state
    /// within `body`, or an outgoing transition from `body`)." This helper
    /// implements the canonical sequential semantic for that:
    ///
    ///   1. Enter the body's `initial_state`.
    ///   2. Run that substate's `onEntry` actions.
    ///   3. If the substate is `kind: final`, the body has completed.
    ///   4. Otherwise, walk the substate's outgoing transitions in document
    ///      order. Pick the first transition whose `event` is `None`
    ///      (anonymous) AND whose guard passes. Run the transition's
    ///      `actions`, then the source substate's `onExit` actions, then
    ///      enter the target substate.
    ///   5. Repeat until a Final substate is reached or the per-iteration
    ///      step cap is hit.
    ///
    /// Bodies with explicit-event transitions (kind: `message` / `signal` /
    /// `timer` / `error`) cannot fire here — body execution is synchronous
    /// per-iteration and the runtime never receives external events between
    /// substate-level steps. Authors MUST shape their compound bodies with
    /// anonymous (auto-firing) transitions only.
    ///
    /// All body provenance carries the synthetic lifecycle-state label
    /// `<foreach-state>:body:<substate-id>` so audit tooling can trace
    /// per-iteration sub-graph progression alongside foreach iteration
    /// records.
    ///
    /// A 100-step cap on substate transitions per iteration prevents an
    /// infinite-loop body from blocking the runtime; exceeding the cap
    /// surfaces as [`EvalError::ForEach`].
    fn run_compound_body_to_completion(
        &mut self,
        foreach_state_id: &str,
        body: &State,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        const MAX_BODY_STEPS: u32 = 100;
        const SYNTHETIC_EVENT: &str = "$bodyAuto";

        let initial_id = body
            .initial_state
            .as_deref()
            .ok_or_else(|| EvalError::ForEach {
                state: foreach_state_id.to_string(),
                message: "compound body MUST declare `initialState`".to_string(),
            })?;
        if body.states.is_empty() {
            return Err(EvalError::ForEach {
                state: foreach_state_id.to_string(),
                message: "compound body MUST declare a non-empty `states` map".to_string(),
            });
        }

        let mut current_id = initial_id.to_string();
        let mut entered_by = None;

        for _step in 0..MAX_BODY_STEPS {
            let substate = body
                .states
                .get(&current_id)
                .ok_or_else(|| EvalError::ForEach {
                    state: foreach_state_id.to_string(),
                    message: format!("compound body references missing substate '{current_id}'"),
                })?;

            let substate_label = format!("{foreach_state_id}:body:{current_id}");
            let foreach_pointer = &self
                .state_index
                .get(foreach_state_id)
                .ok_or_else(|| EvalError::StateNotFound(foreach_state_id.into()))?
                .pointer;
            let substate_key = current_id.replace('~', "~0").replace('/', "~1");
            self.entered_state_pointers
                .push(format!("{foreach_pointer}/body/states/{substate_key}"));

            // Substate onEntry — atomic and final substates emit them; we
            // intentionally don't recurse for nested compound substates in
            // this evaluator path (parallel / foreach / nested compound inside a body
            // are deferred). Only Atomic and Final substate kinds are
            // supported here.
            match substate.kind {
                StateKind::Atomic | StateKind::Final => {}
                other => {
                    return Err(EvalError::ForEach {
                        state: foreach_state_id.to_string(),
                        message: format!(
                            "compound body substate '{current_id}' has kind {other:?}; \
                             only Atomic and Final substates are supported in this PR. \
                             Nested compound / parallel / foreach inside a body is a \
                             future runtime extension."
                        ),
                    });
                }
            }

            let entry_actions = substate.on_entry.clone();
            for action in &entry_actions {
                let action_name = action_kind_camel(action.action);
                self.provenance.push(ProvenanceRecord::on_entry(
                    &self.minter,
                    &substate_label,
                    action_name,
                ));
                self.execute_action_in_state(action, actor, &substate_label, event_data)?;
            }

            // The transition owns its actions, source exit and destination entry.
            // Initial body entry has no internal transition to invent here.
            if let Some((transition, snapshot, actions_start)) = entered_by.take() {
                self.record_transition(transition, actor, snapshot, actions_start);
            }

            // Final substate ⇒ body has completed.
            if substate.kind == StateKind::Final {
                return Ok(());
            }

            // Find the first eligible anonymous outgoing transition.
            let transitions = substate.transitions.clone();
            let mut next_id = None;
            for (transition_index, transition) in transitions.iter().enumerate() {
                let body_pointer = &self
                    .state_index
                    .get(foreach_state_id)
                    .ok_or_else(|| EvalError::StateNotFound(foreach_state_id.into()))?
                    .pointer;
                let substate_key = current_id.replace('~', "~0").replace('/', "~1");
                let location = TransitionLocation {
                    state_pointer: format!("{body_pointer}/body/states/{substate_key}"),
                    transition_index,
                };
                if transition.event.is_some() {
                    // Explicit-event transitions do not auto-fire inside a
                    // body; they would require the runtime to receive an
                    // external event between substate steps, which body
                    // execution does not accommodate.
                    continue;
                }
                let passes = self.evaluate_transition_guard(
                    transition.guard.as_ref(),
                    &location,
                    event_data,
                    &current_id,
                    &transition.target,
                    SYNTHETIC_EVENT,
                )?;
                if !passes {
                    continue;
                }
                let actions_start = self.executed_actions.len();
                let snapshot = self.case_file_snapshot_for_transition(&transition.tags);
                // Run transition actions — attributed to the source substate
                // for provenance — before walking the source's onExit.
                for action in &transition.actions {
                    self.execute_action_in_state(action, actor, &substate_label, event_data)?;
                }
                next_id = Some((
                    transition.target.clone(),
                    transition.tags.clone(),
                    location,
                    snapshot,
                    actions_start,
                ));
                break;
            }

            let (next, tags, location, snapshot, actions_start) = match next_id {
                Some(n) => n,
                None => {
                    return Err(EvalError::ForEach {
                        state: foreach_state_id.to_string(),
                        message: format!(
                            "compound body stuck at substate '{current_id}': no eligible \
                             anonymous transition (event=None with passing guard) and the \
                             substate is not Final. Compound bodies MUST progress to a Final \
                             substate via auto-firing transitions."
                        ),
                    });
                }
            };

            // Source onExit before transitioning out.
            let exit_actions = substate.on_exit.clone();
            for action in &exit_actions {
                let action_name = action_kind_camel(action.action);
                self.provenance.push(ProvenanceRecord::on_exit(
                    &self.minter,
                    &substate_label,
                    action_name,
                ));
                self.execute_action_in_state(action, actor, &substate_label, event_data)?;
            }

            entered_by = Some((
                ObservedTransition {
                    location,
                    provenance_record_id: String::new(),
                    from: substate_label,
                    to: format!("{foreach_state_id}:body:{next}"),
                    event: SYNTHETIC_EVENT.to_string(),
                    tags,
                },
                snapshot,
                actions_start,
            ));
            current_id = next;
        }

        Err(EvalError::ForEach {
            state: foreach_state_id.to_string(),
            message: format!(
                "compound body exceeded {MAX_BODY_STEPS}-step cap at substate '{current_id}'; \
                 possible infinite loop in body's auto-firing transitions"
            ),
        })
    }

    /// Merge a per-iteration value into a `ForEach` state's `outputPath`
    /// according to `mergeStrategy` (Sub-PR D-4 semantics).
    ///
    /// - [`MergeStrategy::Collect`]: append `item_value` to the array at
    ///   `output_path`. Initializes an empty array when the path is absent.
    ///   Errors with [`EvalError::ForEach`] if the existing value is non-null
    ///   and non-array (mismatch between author intent and runtime state).
    /// - [`MergeStrategy::Shallow`]: top-level keys of `item_value` (which
    ///   MUST be a JSON object) are copied into the existing object at
    ///   `output_path`, replacing any colliding keys. Initializes an empty
    ///   object when absent.
    /// - [`MergeStrategy::Deep`]: recursive merge. Nested objects merge
    ///   key-by-key; non-object values at any level are replaced wholesale.
    ///
    /// On every successful merge the runtime emits a `caseStateMutation`
    /// provenance record attributed to the synthetic lifecycle-state label
    /// `<foreach-state>:output` so audit tooling can distinguish foreach
    /// output writes from state-level onEntry / onExit mutations and from
    /// per-iteration body mutations (`<foreach-state>:body`).
    fn merge_foreach_output(
        &mut self,
        state_id: &str,
        output_path: &str,
        strategy: MergeStrategy,
        item_value: serde_json::Value,
        actor: Option<&str>,
    ) -> Result<(), EvalError> {
        let key = output_path
            .strip_prefix("caseFile.")
            .unwrap_or(output_path)
            .to_string();
        let existing = self.case_state.get(&key).cloned();

        let merged = match strategy {
            MergeStrategy::Collect => {
                let mut arr = match existing {
                    Some(serde_json::Value::Array(a)) => a,
                    None | Some(serde_json::Value::Null) => Vec::new(),
                    Some(other) => {
                        return Err(EvalError::ForEach {
                            state: state_id.to_string(),
                            message: format!(
                                "outputPath '{output_path}' has non-array value of kind {}; \
                                 mergeStrategy=collect requires an array (or absent)",
                                json_kind(&other)
                            ),
                        });
                    }
                };
                arr.push(item_value);
                serde_json::Value::Array(arr)
            }
            MergeStrategy::Shallow => {
                let item_obj = match item_value {
                    serde_json::Value::Object(m) => m,
                    other => {
                        return Err(EvalError::ForEach {
                            state: state_id.to_string(),
                            message: format!(
                                "mergeStrategy=shallow requires per-iteration item to be an \
                                 object; got {} for outputPath '{output_path}'",
                                json_kind(&other)
                            ),
                        });
                    }
                };
                let mut existing_obj = match existing {
                    Some(serde_json::Value::Object(m)) => m,
                    None | Some(serde_json::Value::Null) => serde_json::Map::new(),
                    Some(other) => {
                        return Err(EvalError::ForEach {
                            state: state_id.to_string(),
                            message: format!(
                                "outputPath '{output_path}' has non-object value of kind {}; \
                                 mergeStrategy=shallow requires an object (or absent)",
                                json_kind(&other)
                            ),
                        });
                    }
                };
                for (k, v) in item_obj {
                    existing_obj.insert(k, v);
                }
                serde_json::Value::Object(existing_obj)
            }
            MergeStrategy::Deep => {
                if !matches!(item_value, serde_json::Value::Object(_)) {
                    return Err(EvalError::ForEach {
                        state: state_id.to_string(),
                        message: format!(
                            "mergeStrategy=deep requires per-iteration item to be an object; \
                             got {} for outputPath '{output_path}'",
                            json_kind(&item_value)
                        ),
                    });
                }
                let base = match existing {
                    Some(serde_json::Value::Object(m)) => serde_json::Value::Object(m),
                    None | Some(serde_json::Value::Null) => {
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                    Some(other) => {
                        return Err(EvalError::ForEach {
                            state: state_id.to_string(),
                            message: format!(
                                "outputPath '{output_path}' has non-object value of kind {}; \
                                 mergeStrategy=deep requires an object (or absent)",
                                json_kind(&other)
                            ),
                        });
                    }
                };
                deep_merge_json(base, item_value)
            }
        };

        self.case_state.insert(key, merged.clone());

        let output_label = format!("{state_id}:output");
        self.provenance.push(ProvenanceRecord::case_state_mutation(
            &self.minter,
            output_path,
            &merged,
            actor,
            &output_label,
        ));

        Ok(())
    }

    /// Fire the first eligible outgoing transition of a `ForEach` state after
    /// iteration completes. "Eligible" means anonymous (no `event`), guard
    /// passes (or absent). When no eligible transition exists the foreach
    /// state stays active — outgoing transitions with explicit events still
    /// fire later through normal `process_event` dispatch.
    ///
    /// The synthetic event name `$foreachComplete` labels the firing on
    /// provenance and observed-transition records so downstream consumers can
    /// distinguish foreach-completion firings from external events.
    fn fire_foreach_outgoing(
        &mut self,
        state_id: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let transitions = self
            .state_index
            .get(state_id)
            .map(|s| s.state.transitions.clone())
            .unwrap_or_default();

        const SYNTHETIC_EVENT: &str = "$foreachComplete";

        for (transition_index, transition) in transitions.into_iter().enumerate() {
            let location = TransitionLocation {
                state_pointer: self
                    .state_index
                    .get(state_id)
                    .ok_or_else(|| EvalError::StateNotFound(state_id.into()))?
                    .pointer
                    .clone(),
                transition_index,
            };
            // Anonymous transitions only (event = None). Foreach-state
            // transitions with an explicit event still fire later via normal
            // dispatch when that event arrives.
            if transition.event.is_some() {
                continue;
            }
            let guard_passes = self.evaluate_transition_guard(
                transition.guard.as_ref(),
                &location,
                event_data,
                state_id,
                &transition.target,
                SYNTHETIC_EVENT,
            )?;
            if !guard_passes {
                continue;
            }
            self.fire_transition(
                state_id,
                &transition.target,
                &location,
                SYNTHETIC_EVENT,
                actor,
                &transition.actions,
                &transition.tags,
                event_data,
            )?;
            return Ok(());
        }
        Ok(())
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

        let mut row_scope_fields: HashMap<String, Value> = HashMap::new();
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
                    Value::Boolean(true) => {}
                    Value::Boolean(false) => {
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
                    let ids: Vec<&str> = more.iter().map(|i| table.rows[*i].id.as_str()).collect();
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
                    location: None,
                    decision_table: Some(DecisionTableObservation {
                        table_ref: table_ref.to_string(),
                        matched_row_ids: matches
                            .iter()
                            .map(|index| table.rows[*index].id.clone())
                            .collect(),
                        selected_row_id: None,
                        output_column: guard.output_column.clone(),
                        output: None,
                    }),
                    firing_record_id: None,
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
            Value::Boolean(b) => b,
            other => {
                return Err(EvalError::Guard(format!(
                    "K-053: decisionTable output cell on row '{}' (table '{table_ref}') did not evaluate to boolean (got {})",
                    selected_row.id,
                    other.type_name()
                )));
            }
        };

        self.guard_evaluations.push(GuardEvaluation {
            location: None,
            decision_table: Some(DecisionTableObservation {
                table_ref: table_ref.to_string(),
                matched_row_ids: matches
                    .iter()
                    .map(|index| table.rows[*index].id.clone())
                    .collect(),
                selected_row_id: Some(selected_row.id.clone()),
                output_column: guard.output_column.clone(),
                output: Some(serde_json::json!(passed)),
            }),
            firing_record_id: None,
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
            self.provenance.push(ProvenanceRecord::timer_fired(
                &self.minter,
                &timer.id,
                &timer.fires_event,
            ));

            // Check tolerance: if the timer fired significantly after its deadline,
            // emit a toleranceViolation diagnostic (LCD S6.6, Runtime S7.2).
            let lateness_ms = current_time.saturating_sub(timer.deadline_ms);
            let max_tolerance = crate::timer::max_tolerance_ms(timer.duration_ms);
            if lateness_ms > max_tolerance {
                let tolerance_iso = crate::timer::tolerance_to_iso(max_tolerance);
                self.provenance.push(ProvenanceRecord::tolerance_violation(
                    &self.minter,
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
            StateKind::Compound | StateKind::ForEach => {
                // ForEach uses Compound-shaped substate nesting (initial_state
                // + states IndexMap). On exit, walk the body subtree the same
                // way to ensure no orphaned substates remain.
                if let Some(history_mode) = &indexed.state.history_state {
                    self.capture_history(state_id, *history_mode);
                }
                let substate_ids: Vec<String> = indexed.state.states.keys().cloned().collect();
                for sub_id in &substate_ids {
                    self.exit_state_and_descendants(sub_id);
                    self.clear_subtree_history(sub_id, "parent-exit");
                }
            }
            StateKind::Parallel => {
                for region in indexed.state.regions.values() {
                    let region_ids: Vec<String> = region.states.keys().cloned().collect();
                    for region_state_id in &region_ids {
                        self.exit_state_and_descendants(region_state_id);
                        self.clear_subtree_history(region_state_id, "parent-exit");
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
                StateKind::Compound | StateKind::ForEach => {
                    // ForEach uses Compound-shaped nesting; walk the body
                    // subtree the same way for leaf collection.
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
        let restoration_checkpoint = self.exit_checkpoint();
        for leaf_id in saved {
            if self.scope_exited_since(restoration_checkpoint, leaf_id) {
                continue;
            }
            let checkpoint = self.exit_checkpoint();
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
            if self.scope_exited_since(checkpoint, compound_id) {
                // Interrupted entry has not established a complete configuration.
                // Preserve prior history only if no parent-exit rule cleared it.
                let parent_exited = self.exit_contexts.last().expect("active exit context")
                    [checkpoint..]
                    .iter()
                    .any(|root| self.is_descendant(compound_id, root));
                if !parent_exited {
                    self.history_store
                        .insert(compound_id.into(), saved.to_vec());
                }
                return Ok(());
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

        self.entered_state_pointers.push(
            self.state_index
                .get(state_id)
                .ok_or_else(|| EvalError::StateNotFound(state_id.into()))?
                .pointer
                .clone(),
        );
        let kind = self
            .state_index
            .get(state_id)
            .expect("resolved state")
            .state
            .kind;
        if matches!(kind, StateKind::Compound | StateKind::Parallel) {
            self.config.enter_scope(state_id.to_string());
        } else {
            self.config.enter(state_id.to_string());
        }
        self.provenance
            .push(ProvenanceRecord::state_entered(&self.minter, state_id));
        self.execute_on_entry_actions(state_id, actor, event_data)?;
        Ok(())
    }

    fn cancel_timers_created_in_state_tree(&mut self, state_id: &str, reason: &str) {
        for timer in self.timers.cancel_in_state(state_id) {
            self.provenance.push(ProvenanceRecord::timer_cancelled(
                &self.minter,
                &timer.id,
                reason,
            ));
        }

        let indexed = match self.state_index.get(state_id) {
            Some(s) => s.clone(),
            None => return,
        };

        match indexed.state.kind {
            StateKind::Compound | StateKind::ForEach => {
                // ForEach uses Compound-shaped substate nesting; walk the body
                // subtree to cancel timers that may have been created in body
                // states during prior iterations.
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

    fn execute_subtree_exit_actions(
        &mut self,
        root: &str,
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let mut exiting: Vec<_> = self
            .active_candidates()
            .into_iter()
            .filter(|state| state == root || self.is_descendant(state, root))
            .collect();
        exiting.sort_by_key(|state| std::cmp::Reverse(self.ancestors(state).len()));
        for state in exiting {
            self.execute_on_exit_actions(&state, actor, event_data)?;
        }
        Ok(())
    }
    fn clear_subtree_history(&mut self, root: &str, reason: &str) {
        let mut cleared: Vec<_> = self
            .history_store
            .keys()
            .filter(|state| state.as_str() == root || self.is_descendant(state, root))
            .cloned()
            .collect();
        cleared.sort();
        for state in cleared {
            self.history_store.remove(&state);
            self.provenance.push(ProvenanceRecord::history_cleared(
                &self.minter,
                &state,
                reason,
            ));
        }
    }
    fn record_region_cancellation(&mut self, parallel: &str, region: &str, root: &str) {
        // This records the actual scope exit, without inventing an authored action.
        let mut record = ProvenanceRecord::blank(&self.minter, crate::ProvenanceKind::OnExit);
        record.from_state = Some(root.into());
        record.data = Some(
            serde_json::json!({"parallelState":parallel,"region":region,"reason":"region-cancellation"}),
        );
        self.provenance.push(record);
    }
    fn finish_exit_context(&mut self) -> Vec<String> {
        let exited = self.exit_contexts.pop().expect("active exit context");
        if let Some(parent) = self.exit_contexts.last_mut() {
            parent.extend(exited.iter().cloned());
        }
        exited
    }
    fn exit_checkpoint(&self) -> usize {
        self.exit_contexts
            .last()
            .expect("active exit context")
            .len()
    }
    fn scope_exited_since(&self, checkpoint: usize, scope: &str) -> bool {
        self.scope_exited(
            &self.exit_contexts.last().expect("active exit context")[checkpoint..],
            scope,
        )
    }
    fn scope_exited(&self, roots: &[String], scope: &str) -> bool {
        roots
            .iter()
            .any(|root| scope == root || self.is_descendant(scope, root))
    }
    fn ancestors(&self, state: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = Some(state);
        while let Some(state) = current {
            result.push(state.to_owned());
            current = self
                .state_index
                .get(state)
                .and_then(|value| value.direct_parent.as_deref());
        }
        result
    }
    fn is_descendant(&self, state: &str, ancestor: &str) -> bool {
        self.ancestors(state)
            .iter()
            .skip(1)
            .any(|state| state == ancestor)
    }
    fn active_candidates(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for leaf in &self.config.active {
            for state in self.ancestors(leaf) {
                if self.config.contains(&state) && seen.insert(state.clone()) {
                    result.push(state);
                }
            }
        }
        result
    }
    fn find_parallel_parents(&self) -> Vec<String> {
        self.active_candidates()
            .into_iter()
            .filter(|state| {
                self.state_index
                    .get(state)
                    .is_some_and(|value| value.state.kind == StateKind::Parallel)
                    && !self.ancestors(state).iter().skip(1).any(|parent| {
                        self.state_index
                            .get(parent)
                            .is_some_and(|value| value.state.kind == StateKind::Parallel)
                            && self.config.contains(parent)
                    })
            })
            .collect()
    }
    fn state_is_in_parallel_region(&self, parallel_id: &str, state_id: &str) -> bool {
        self.is_descendant(state_id, parallel_id)
    }
    fn state_is_in_region(&self, parallel_id: &str, region_name: &str, state_id: &str) -> bool {
        self.ancestors(state_id).iter().any(|state| {
            self.state_index.get(state).is_some_and(|value| {
                value.direct_parent.as_deref() == Some(parallel_id)
                    && value.region_name.as_deref() == Some(region_name)
            })
        })
    }
    fn parallel_ready(&self, parallel_id: &str) -> bool {
        self.state_index.get(parallel_id).is_some_and(|value| {
            parallel_cancellation_triggered(&value.state, &self.config)
                || (!value.state.regions.is_empty()
                    && value.state.regions.values().all(|region| {
                        region.states.iter().any(|(name, state)| {
                            state.kind == StateKind::Final && self.config.contains(name)
                        })
                    }))
        })
    }
    fn transition_paths(
        &self,
        source: &str,
        target: &str,
    ) -> Result<(Option<String>, Vec<String>), EvalError> {
        if !self.state_index.contains_key(target) {
            return Err(EvalError::StateNotFound(target.into()));
        }
        let source_chain = self.ancestors(source);
        let target_chain = self.ancestors(target);
        let mut boundary = source_chain
            .iter()
            .find(|state| target_chain.contains(state))
            .cloned();
        // Kernel §4.8.6: a parallel-declared transition exits every region.
        // Crossing orthogonal regions also reenters their common parallel scope.
        if let Some(common) = boundary.clone().filter(|common| {
            self.state_index
                .get(common)
                .is_some_and(|entry| entry.state.kind == StateKind::Parallel)
        }) {
            let crosses_regions = self.state_index.get(&common).is_some_and(|entry| {
                entry.state.regions.keys().any(|region| {
                    self.state_is_in_region(&common, region, source)
                        && !self.state_is_in_region(&common, region, target)
                })
            });
            if source == common || crosses_regions {
                boundary = self
                    .state_index
                    .get(&common)
                    .and_then(|entry| entry.direct_parent.clone());
            }
        }
        // Targeting the active source or its ancestor is external reentry.
        if source == target || self.is_descendant(source, target) {
            boundary = self
                .state_index
                .get(target)
                .and_then(|state| state.direct_parent.clone());
        }
        let exit_root = source_chain
            .iter()
            .take_while(|state| Some(*state) != boundary.as_ref())
            .last()
            .cloned();
        // A parent-declared transition into another child exits that active branch.
        let exit_root = exit_root.or_else(|| {
            self.config
                .active
                .iter()
                .find(|leaf| {
                    self.is_descendant(leaf, source)
                        && self.state_index.get(source).is_none_or(|entry| {
                            entry.state.kind != StateKind::Parallel
                                || entry.state.regions.keys().any(|region| {
                                    self.state_is_in_region(source, region, target)
                                        && self.state_is_in_region(source, region, leaf)
                                })
                        })
                })
                .and_then(|leaf| {
                    self.ancestors(leaf)
                        .into_iter()
                        .take_while(|state| state != source)
                        .last()
                })
        });
        let mut entry: Vec<_> = target_chain
            .into_iter()
            .take_while(|state| Some(state) != boundary.as_ref())
            .collect();
        entry.reverse();
        Ok((exit_root, entry))
    }
    fn enter_transition_path(
        &mut self,
        path: &[String],
        actor: Option<&str>,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), EvalError> {
        let Some((head, tail)) = path.split_first() else {
            return Ok(());
        };
        if tail.is_empty() {
            return self.enter_state(head, actor, event_data);
        }
        let state = self
            .state_index
            .get(head)
            .ok_or_else(|| EvalError::StateNotFound(head.clone()))?
            .state
            .clone();
        self.enter_state_direct(head, actor, event_data)?;
        if state.kind == StateKind::Parallel {
            for (name, region) in &state.regions {
                let checkpoint = self.exit_checkpoint();
                if self.state_is_in_region(head, name, &tail[0]) {
                    self.enter_transition_path(tail, actor, event_data)?;
                } else {
                    self.enter_state(&region.initial_state, actor, event_data)?;
                }
                if self.scope_exited_since(checkpoint, head) {
                    break;
                }
            }
            Ok(())
        } else {
            self.enter_transition_path(tail, actor, event_data)
        }
    }
}

// ── Module-level helpers ─────────────────────────────────────────

fn restored_configuration(
    index: &HashMap<String, IndexedState>,
    leaves: &[String],
) -> Result<Configuration, EvalError> {
    let mut config = Configuration::default();
    for leaf in leaves {
        let state = index
            .get(leaf)
            .ok_or_else(|| EvalError::StateNotFound(leaf.clone()))?;
        if matches!(state.state.kind, StateKind::Compound | StateKind::Parallel) {
            return Err(EvalError::Internal(
                "persisted configuration must contain active leaf states".into(),
            ));
        }
        if config.active.contains(leaf) {
            return Err(EvalError::Internal(
                "persisted configuration contains duplicate leaves".into(),
            ));
        }
        if config.active.last().is_some_and(|previous| {
            index
                .get(previous)
                .is_some_and(|entry| entry.document_order >= state.document_order)
        }) {
            return Err(EvalError::Internal(
                "persisted leaves must retain authored document order".into(),
            ));
        }
        config.enter(leaf.clone());
        let mut parent = state.direct_parent.as_deref();
        while let Some(name) = parent {
            config.enter_scope(name.into());
            parent = index
                .get(name)
                .and_then(|state| state.direct_parent.as_deref());
        }
    }
    let roots = index
        .iter()
        .filter(|(_, entry)| entry.direct_parent.is_none())
        .filter(|(name, _)| config.contains(name))
        .count();
    if !leaves.is_empty() && roots != 1 {
        return Err(EvalError::Internal(
            "persisted configuration contains exclusive root branches".into(),
        ));
    }
    for scope in &config.scopes {
        let state = &index
            .get(scope)
            .ok_or_else(|| EvalError::StateNotFound(scope.clone()))?
            .state;
        let invalid = match state.kind {
            StateKind::Compound => {
                state
                    .states
                    .keys()
                    .filter(|name| config.contains(name))
                    .count()
                    != 1
            }
            StateKind::Parallel => {
                let cancelled = parallel_cancellation_triggered(state, &config);
                state.regions.values().any(|region| {
                    let count = region
                        .states
                        .keys()
                        .filter(|name| config.contains(name))
                        .count();
                    count > 1 || (!cancelled && count != 1)
                })
            }
            _ => true,
        };
        if invalid {
            return Err(EvalError::Internal(
                "persisted configuration has inconsistent composite branches".into(),
            ));
        }
    }
    Ok(config)
}

fn parallel_cancellation_triggered(state: &State, config: &Configuration) -> bool {
    let policy = state
        .cancellation_policy
        .unwrap_or(CancellationPolicy::WaitAll);
    state.regions.values().any(|region| {
        region.states.iter().any(|(name, child)| {
            config.contains(name)
                && child.kind == StateKind::Final
                && match policy {
                    CancellationPolicy::WaitAll => false,
                    CancellationPolicy::CancelSiblings => true,
                    CancellationPolicy::FailFast => child.tags.iter().any(|tag| tag == "error"),
                }
        })
    })
}

/// Build a flat state index from the typed kernel document.
fn build_state_index(kernel: &KernelDocument) -> HashMap<String, IndexedState> {
    let mut index = HashMap::new();
    index_states_recursive(
        &kernel.lifecycle.states,
        "/lifecycle/states",
        None,
        None,
        None,
        &mut index,
    );
    index
}

/// Validates that every active configuration leaf exists in `kernel`.
///
/// Used by instance migration (Kernel S11.2 step 1). When any active state
/// id is absent from the target definition, returns [`EvalError::StateNotFound`].
pub fn validate_migration_configuration(
    kernel: &KernelDocument,
    configuration: &[String],
) -> Result<(), EvalError> {
    let index = build_state_index(kernel);
    for id in configuration {
        if !index.contains_key(id) {
            return Err(EvalError::StateNotFound(id.clone()));
        }
    }
    Ok(())
}

/// Recursively index states from a states map.
fn index_states_recursive(
    states: &indexmap::IndexMap<String, State>,
    states_pointer: &str,
    parallel_parent: Option<&str>,
    region_name: Option<&str>,
    direct_parent: Option<&str>,
    index: &mut HashMap<String, IndexedState>,
) {
    for (name, state) in states {
        let escaped = name.replace('~', "~0").replace('/', "~1");
        let pointer = format!("{states_pointer}/{escaped}");
        index.insert(
            name.clone(),
            IndexedState {
                pointer: pointer.clone(),
                state: state.clone(),
                document_order: index.len(),
                parallel_parent: parallel_parent.map(String::from),
                region_name: region_name.map(String::from),
                direct_parent: direct_parent.map(String::from),
            },
        );

        if state.kind == StateKind::Compound {
            index_states_recursive(
                &state.states,
                &format!("{pointer}/states"),
                parallel_parent,
                region_name,
                Some(name),
                index,
            );
        }

        if state.kind == StateKind::Parallel {
            for (rname, region) in &state.regions {
                let region_key = rname.replace('~', "~0").replace('/', "~1");
                index_states_recursive(
                    &region.states,
                    &format!("{pointer}/regions/{region_key}/states"),
                    Some(name),
                    Some(rname),
                    Some(name),
                    index,
                );
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
    let nanos = parsed.unix_timestamp_nanos();
    if nanos < 0 {
        return Err(EvalError::Internal(format!(
            "timer deadline '{value}' predates Unix epoch"
        )));
    }
    let millis = nanos / 1_000_000;
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
/// Map an [`ActionKind`] to its camelCase string form (the same labels
/// `execute_on_entry_actions` / `execute_on_exit_actions` use). Lifted as a
/// module-level helper so the foreach body executor can attribute body
/// actions to the same provenance shape as state-level onEntry / onExit
/// actions without duplicating the table.
fn action_kind_camel(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::CreateTask => "createTask",
        ActionKind::InvokeService => "invokeService",
        ActionKind::SetData => "setData",
        ActionKind::EmitEvent => "emitEvent",
        ActionKind::StartTimer => "startTimer",
        ActionKind::CancelTimer => "cancelTimer",
        ActionKind::Log => "log",
    }
}

/// Recursively merge `incoming` into `base`. Both arguments SHOULD be JSON
/// objects when called from [`Evaluator::merge_foreach_output`] under
/// [`MergeStrategy::Deep`]; the type guards there enforce the precondition.
/// Within objects: keys in `incoming` overwrite keys in `base`, but when both
/// sides are objects the merge recurses. Non-object collisions (e.g. base has
/// `{"x": [1, 2]}`, incoming has `{"x": [3]}`) are resolved by replacement —
/// `incoming` wins. Arrays are NOT element-merged; this matches typical
/// "deep merge" semantics in JSON tooling and keeps behavior predictable
/// when authors collect heterogenous shapes.
fn deep_merge_json(base: serde_json::Value, incoming: serde_json::Value) -> serde_json::Value {
    match (base, incoming) {
        (serde_json::Value::Object(mut b), serde_json::Value::Object(i)) => {
            for (k, v) in i {
                let merged = match b.remove(&k) {
                    Some(existing) => deep_merge_json(existing, v),
                    None => v,
                };
                b.insert(k, merged);
            }
            serde_json::Value::Object(b)
        }
        (_, incoming) => incoming,
    }
}

/// Human-readable JSON kind label for diagnostic messages — one of
/// `"null" | "bool" | "number" | "string" | "object" | "array"`.
fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

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
fn row_scope_to_json(scope: &HashMap<String, Value>) -> serde_json::Value {
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
    use stack_common_typeid::testing::fixture_minter;

    mod initial_initiating_actor {
        use super::*;

        fn kernel() -> KernelDocument {
            serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "lifecycle": { "initialState": "waiting", "states": {
                    "waiting": { "type": "atomic", "onEntry": [
                        { "action": "emitEvent", "eventType": "checks.completed" }
                    ] }
                } }
            }))
            .unwrap()
        }

        #[test]
        fn explicit_initial_actor_is_optional_and_not_event_data() {
            for actor in [Some("actor:human:initiator"), Some("intake-service"), None] {
                let eval = Evaluator::with_time_and_case_state_and_actor(
                    kernel(),
                    &fixture_minter(),
                    42,
                    None,
                    actor,
                )
                .unwrap();
                let action = eval.executed_actions().first().unwrap();
                assert_eq!(action.actor_id.as_deref(), actor);
                assert!(action.event_data.is_none());
                let record = eval
                    .provenance
                    .records()
                    .iter()
                    .find(|record| record.record_kind == crate::ProvenanceKind::ActionExecuted)
                    .unwrap();
                assert_eq!(record.actor_id.as_deref(), actor);
                assert!(action.transition_record_id.is_none());
            }
        }

        #[test]
        fn existing_initializers_remain_actorless() {
            for eval in [
                Evaluator::new(kernel(), &fixture_minter()).unwrap(),
                Evaluator::with_time(kernel(), &fixture_minter(), 42).unwrap(),
                Evaluator::with_time_and_case_state(kernel(), &fixture_minter(), 42, None).unwrap(),
            ] {
                assert!(
                    eval.executed_actions()
                        .iter()
                        .all(|action| action.actor_id.is_none())
                );
                assert!(
                    eval.provenance
                        .records()
                        .iter()
                        .all(|record| record.actor_id.is_none())
                );
            }
        }
    }

    mod action_actor_preservation {
        use super::*;

        fn check_actor(actor: Option<&str>) {
            let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "lifecycle": {
                    "initialState": "waiting",
                    "states": {
                        "waiting": {
                            "type": "atomic",
                            "onExit": [{ "action": "log", "message": "leaving" }],
                            "transitions": [{
                                "event": { "kind": "message", "name": "review.completed" },
                                "target": "done",
                                "actions": [{ "action": "emitEvent", "eventType": "notice.ready" }]
                            }]
                        },
                        "done": {
                            "type": "atomic",
                            "onEntry": [
                                { "action": "invokeService", "serviceRef": "notice" },
                                { "action": "createTask", "taskRef": "review.followup" }
                            ]
                        }
                    }
                }
            }))
            .unwrap();
            let mut eval = Evaluator::new(kernel, &fixture_minter()).unwrap();
            assert!(eval.process_event("review.completed", actor, None).unwrap());
            let actions: Vec<_> = eval
                .provenance
                .records()
                .iter()
                .filter(|record| record.record_kind == crate::ProvenanceKind::ActionExecuted)
                .collect();
            assert_eq!(actions.len(), 4);
            for record in &actions {
                assert_eq!(record.actor_id.as_deref(), actor);
                assert!(record.actor_type.is_none());
                let json = serde_json::to_value(record).unwrap();
                assert_eq!(
                    json.get("actorId").and_then(serde_json::Value::as_str),
                    actor
                );
                if actor.is_none() {
                    assert!(json.get("actorId").is_none());
                }
            }
            let transition = eval
                .provenance
                .records()
                .iter()
                .find(|record| record.event.as_deref() == Some("wos.kernel.state_transition"))
                .unwrap();
            assert_eq!(transition.actor_id.as_deref(), actor);
            assert_eq!(
                transition.data.as_ref().unwrap()["transitionEvent"],
                "review.completed"
            );
            let emission = eval
                .executed_actions()
                .iter()
                .find(|action| action.action.action == ActionKind::EmitEvent)
                .unwrap();
            assert_eq!(emission.actor_id.as_deref(), actor);
            assert_eq!(
                emission.transition_record_id.as_deref(),
                Some(transition.id.as_str())
            );
            let emission_record = actions
                .iter()
                .find(|record| record.data.as_ref().unwrap()["actionType"] == "emitEvent")
                .unwrap();
            assert_eq!(
                emission.provenance_record_id.as_deref(),
                Some(emission_record.id.as_str())
            );
            assert_eq!(
                emission_record.data.as_ref().unwrap()["eventType"],
                "notice.ready"
            );
        }

        #[test]
        fn human_actor_survives_action_provenance() {
            check_actor(Some("actor:human:actual-supplied-id"));
        }

        #[test]
        fn system_actor_survives_action_provenance() {
            check_actor(Some("scheduler:actual-supplied-id"));
        }

        #[test]
        fn absent_actor_stays_absent_in_action_provenance() {
            check_actor(None);
        }
    }

    mod anchored_timers {
        use super::*;

        fn evaluator(now: u64) -> Evaluator {
            let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
                "$wosWorkflow": "1.0",
                "lifecycle": {
                    "initialState": "waiting",
                    "states": {
                        "waiting": { "type": "atomic", "transitions": [
                            { "event": { "kind": "message", "name": "elapsed" }, "target": "done" }
                        ] },
                        "done": { "type": "final" }
                    }
                }
            }))
            .unwrap();
            Evaluator::with_time(kernel, &fixture_minter(), now).unwrap()
        }

        fn action(expression: Option<&str>, duration: &str) -> Action {
            let mut value = serde_json::json!({
                "action": "startTimer", "timerId": "anchored", "duration": duration,
                "event": { "kind": "timer", "timerId": "anchored", "source": "custom", "firesAs": "elapsed" }
            });
            if let Some(expression) = expression {
                value["startAtExpression"] = expression.into();
            }
            serde_json::from_value(value).unwrap()
        }

        #[test]
        fn task_lifetime_field_refuses_non_create_task_actions_before_effects() {
            let mut eval = evaluator(10_000);
            // Initialization enters the initial state and retains genuine provenance.
            assert!(
                eval.provenance
                    .records()
                    .iter()
                    .any(
                        |record| record.record_kind == crate::ProvenanceKind::OnEntry
                            && record.to_state.as_deref() == Some("waiting")
                            && record.data.as_ref()
                                == Some(&serde_json::json!({"state":"waiting"}))
                    )
            );
            // Keep an existing same-ID timer, so refusal also proves no replacement.
            eval.execute_action_in_state(&action(None, "PT5S"), None, "waiting", None)
                .unwrap();
            let timer_values = |eval: &Evaluator| {
                let mut timers: Vec<_> = eval
                    .timers
                    .iter()
                    .map(|timer| {
                        (
                            timer.id.clone(),
                            timer.created_at_ms,
                            timer.deadline_ms,
                            timer.fires_event.clone(),
                            timer.created_in_state.clone(),
                            timer.duration_iso.clone(),
                            timer.duration_ms,
                        )
                    })
                    .collect();
                timers.sort_by(|left, right| left.0.cmp(&right.0));
                timers
            };
            let observation_values = |eval: &Evaluator| {
                eval.executed_actions
                    .iter()
                    .map(|observed| {
                        (
                            observed.lifecycle_state.clone(),
                            observed.actor_id.clone(),
                            serde_json::to_value(&observed.action).unwrap(),
                            observed.event_data.clone(),
                            observed.task_expiry_timer.as_ref().map(|timer| {
                                (
                                    timer.id.clone(),
                                    timer.created_at_ms,
                                    timer.deadline_ms,
                                    timer.fires_event.clone(),
                                    timer.created_in_state.clone(),
                                    timer.duration_iso.clone(),
                                    timer.duration_ms,
                                )
                            }),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let before_provenance = serde_json::to_value(eval.provenance.records()).unwrap();
            let before_timers = timer_values(&eval);
            let before_observations = observation_values(&eval);
            let before_case = eval.case_state_json();
            let before_configuration = eval.config.active_states().to_vec();
            let before_transitions = serde_json::to_value(&eval.transitions).unwrap();
            let mut invalid = action(None, "PT1S");
            invalid.expires_with_timer = Some("anchored".into());
            assert!(matches!(
                eval.execute_action_in_state(&invalid, None, "waiting", None),
                Err(EvalError::Internal(_))
            ));
            assert_eq!(
                serde_json::to_value(eval.provenance.records()).unwrap(),
                before_provenance
            );
            assert_eq!(timer_values(&eval), before_timers);
            assert_eq!(observation_values(&eval), before_observations);
            assert_eq!(eval.case_state_json(), before_case);
            assert_eq!(eval.config.active_states(), before_configuration);
            assert_eq!(
                serde_json::to_value(&eval.transitions).unwrap(),
                before_transitions
            );
        }

        #[test]
        fn bounded_task_observation_keeps_timer_before_later_replace_or_cancel() {
            for cancel in [false, true] {
                let mut eval = evaluator(10_000);
                eval.execute_action_in_state(&action(None, "PT1S"), None, "waiting", None)
                    .unwrap();
                let create:Action=serde_json::from_value(serde_json::json!({"action":"createTask","taskRef":"response","expiresWithTimer":"anchored"})).unwrap();
                eval.execute_action_in_state(&create, None, "waiting", None)
                    .unwrap();
                if cancel {
                    let cancellation: Action = serde_json::from_value(
                        serde_json::json!({"action":"cancelTimer","timerId":"anchored"}),
                    )
                    .unwrap();
                    eval.execute_action_in_state(&cancellation, None, "waiting", None)
                        .unwrap();
                } else {
                    eval.execute_action_in_state(&action(None, "PT10S"), None, "waiting", None)
                        .unwrap();
                }
                let observed = eval
                    .executed_actions
                    .iter()
                    .find(|o| o.action.action == ActionKind::CreateTask)
                    .unwrap();
                assert_eq!(
                    observed.task_expiry_timer.as_ref().unwrap().deadline_ms,
                    11_000
                );
                assert_eq!(
                    observed.task_expiry_timer.as_ref().unwrap().created_at_ms,
                    10_000
                );
                assert!(
                    eval.executed_actions
                        .iter()
                        .filter(|o| o.action.action != ActionKind::CreateTask)
                        .all(|o| o.task_expiry_timer.is_none())
                );
                if cancel {
                    assert!(eval.timers.is_empty());
                } else {
                    assert_eq!(timer(&eval).deadline_ms, 20_000);
                }
            }
        }

        fn timer(eval: &Evaluator) -> &crate::timer::Timer {
            eval.timers.iter().find(|t| t.id == "anchored").unwrap()
        }

        #[test]
        fn anchored_timer_uses_past_event_origin_and_normalizes_offset() {
            let origin = parse_rfc3339_to_ms("2026-01-01T00:00:00Z").unwrap();
            let mut eval = evaluator(origin + 2 * 86_400_000);
            let event = serde_json::json!({"sentAt": "2025-12-31T19:00:00-05:00"});
            let action = action(Some("event.sentAt"), "P90D");
            eval.execute_action_in_state(&action, None, "waiting", Some(&event))
                .unwrap();
            assert_eq!(timer(&eval).created_at_ms, origin);
            assert_eq!(timer(&eval).deadline_ms, origin + 90 * 86_400_000);
            let created = eval
                .provenance
                .records()
                .last()
                .unwrap()
                .data
                .as_ref()
                .unwrap();
            assert_eq!(created["originAt"], "2026-01-01T00:00:00Z");
            assert_eq!(created["startAtExpression"], "event.sentAt");
        }

        #[test]
        fn anchored_timer_reads_case_context_and_allows_current_origin() {
            let mut eval = evaluator(10_000);
            eval.case_state.insert(
                "receipt".into(),
                serde_json::json!({"sentAt": "1970-01-01T00:00:10Z"}),
            );
            eval.execute_action_in_state(
                &action(Some("caseFile.receipt.sentAt"), "PT1.5S"),
                None,
                "waiting",
                None,
            )
            .unwrap();
            assert_eq!(timer(&eval).created_at_ms, 10_000);
            assert_eq!(timer(&eval).deadline_ms, 11_500);
        }

        #[test]
        fn already_expired_anchor_keeps_deadline_and_fires_normal_timer_path() {
            let mut eval = evaluator(100_000);
            eval.execute_action_in_state(
                &action(Some("'1970-01-01T00:00:01Z'"), "PT2S"),
                None,
                "waiting",
                None,
            )
            .unwrap();
            assert_eq!(timer(&eval).deadline_ms, 3_000);
            eval.advance_time(0, None).unwrap();
            assert!(eval.timers.is_empty());
            assert!(eval.config.contains("done"));
            assert!(
                eval.provenance
                    .records()
                    .iter()
                    .any(|r| r.record_kind == crate::ProvenanceKind::TimerFired)
            );
        }

        #[test]
        fn default_timer_retains_current_time_and_omits_anchor_metadata() {
            let mut eval = evaluator(20_000);
            let action = action(None, "PT2S");
            assert!(
                serde_json::to_value(&action)
                    .unwrap()
                    .get("startAtExpression")
                    .is_none()
            );
            eval.execute_action_in_state(&action, None, "waiting", None)
                .unwrap();
            assert_eq!(timer(&eval).created_at_ms, 20_000);
            assert_eq!(timer(&eval).deadline_ms, 22_000);
            let data = eval
                .provenance
                .records()
                .last()
                .unwrap()
                .data
                .as_ref()
                .unwrap();
            assert!(data.get("originAt").is_none());
            assert!(data.get("startAtExpression").is_none());
        }

        #[test]
        fn malformed_anchors_refuse_before_same_id_timer_or_provenance_changes() {
            let mut eval = evaluator(20_000);
            eval.execute_action_in_state(&action(None, "PT2S"), None, "waiting", None)
                .unwrap();
            let old_timer = format!("{:?}", timer(&eval));
            let old_provenance = serde_json::to_value(eval.provenance.records()).unwrap();
            let old_actions = eval.executed_actions.len();
            let expressions = [
                "event.missing",
                "42",
                "'not-a-timestamp'",
                "'1969-12-31T23:59:59.999999999Z'",
                "'1970-01-01T00:00:21Z'",
                "'1970-01-01T00:00:20.000000001Z'",
                "'10000-01-01T00:00:00Z'",
                "(",
            ];
            for expression in expressions {
                let result = eval.execute_action_in_state(
                    &action(Some(expression), "PT1S"),
                    None,
                    "waiting",
                    None,
                );
                assert!(
                    matches!(result, Err(EvalError::Timer { .. })),
                    "{expression}"
                );
                assert_eq!(format!("{:?}", timer(&eval)), old_timer);
                assert_eq!(
                    serde_json::to_value(eval.provenance.records()).unwrap(),
                    old_provenance
                );
                assert_eq!(eval.executed_actions.len(), old_actions);
            }
        }

        #[test]
        fn invalid_duration_and_deadline_range_preserve_existing_timer() {
            let mut eval = evaluator(20_000);
            eval.execute_action_in_state(&action(None, "PT2S"), None, "waiting", None)
                .unwrap();
            let old_timer = format!("{:?}", timer(&eval));
            let old_provenance = eval.provenance.len();
            for duration in [
                "bad",
                "-PT1S",
                "-PT0S",
                "P1DT",
                "PT",
                "PT9223372036854775807S",
                "P999999999999999999999999999999999999999999999D",
                "PT9223372036854775S",
            ] {
                let result = eval.execute_action_in_state(
                    &action(Some("'1970-01-01T00:00:01Z'"), duration),
                    None,
                    "waiting",
                    None,
                );
                assert!(matches!(result, Err(EvalError::Timer { .. })), "{duration}");
                assert_eq!(format!("{:?}", timer(&eval)), old_timer);
                assert_eq!(eval.provenance.len(), old_provenance);
            }
            let mut missing = action(Some("'1970-01-01T00:00:01Z'"), "PT1S");
            missing.duration = None;
            assert!(matches!(
                eval.execute_action_in_state(&missing, None, "waiting", None),
                Err(EvalError::Timer { .. })
            ));
            let mut conflicting = action(Some("'1970-01-01T00:00:01Z'"), "PT1S");
            conflicting.deadline = Some("1970-01-01T00:00:02Z".into());
            assert!(matches!(
                eval.execute_action_in_state(&conflicting, None, "waiting", None),
                Err(EvalError::Timer { .. })
            ));
            assert_eq!(format!("{:?}", timer(&eval)), old_timer);
            assert_eq!(eval.provenance.len(), old_provenance);
        }

        #[test]
        fn typed_anchor_fields_cannot_be_ignored_or_omit_timer_identity_and_event() {
            let mut eval = evaluator(20_000);
            eval.execute_action_in_state(&action(None, "PT2S"), None, "waiting", None)
                .unwrap();
            let old_timer = format!("{:?}", timer(&eval));
            let old_provenance = serde_json::to_value(eval.provenance.records()).unwrap();
            let original = action(Some("'1970-01-01T00:00:01Z'"), "PT1S");
            let mut invalid_actions = Vec::new();
            for kind in [
                ActionKind::SetData,
                ActionKind::CancelTimer,
                ActionKind::EmitEvent,
                ActionKind::CreateTask,
                ActionKind::InvokeService,
                ActionKind::Log,
            ] {
                let mut invalid = original.clone();
                invalid.action = kind;
                invalid.path = Some("caseFile.unwanted".into());
                invalid.value = Some(true.into());
                invalid_actions.push(invalid);
            }
            for id in [None, Some(""), Some("  ")] {
                let mut invalid = original.clone();
                invalid.timer_id = id.map(str::to_owned);
                invalid_actions.push(invalid);
            }
            let mut missing_event = original.clone();
            missing_event.event = None;
            invalid_actions.push(missing_event);
            for name in ["", "   "] {
                let mut invalid = original.clone();
                invalid.event = Some(TransitionEvent::Message {
                    name: name.into(),
                    correlation_key: None,
                    data: None,
                });
                invalid_actions.push(invalid);
            }
            for invalid in invalid_actions {
                assert!(matches!(
                    eval.execute_action_in_state(&invalid, None, "waiting", None),
                    Err(EvalError::Timer { .. })
                ));
                assert_eq!(format!("{:?}", timer(&eval)), old_timer);
                assert_eq!(
                    serde_json::to_value(eval.provenance.records()).unwrap(),
                    old_provenance
                );
                assert!(!eval.case_state.contains_key("unwanted"));
            }
        }
    }

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
                collection: None,
                item_variable: None,
                index_variable: None,
                concurrency: None,
                break_condition: None,
                output_path: None,
                merge_strategy: None,
                body: None,
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
                collection: None,
                item_variable: None,
                index_variable: None,
                concurrency: None,
                break_condition: None,
                output_path: None,
                merge_strategy: None,
                body: None,
                extensions: HashMap::new(),
            }
        }

        /// Build a kernel doc with a single guarded transition `start --> end`
        /// triggered by event `decide`, plus the supplied decision-table list.
        fn doc_with_guard(tables: Vec<DecisionTable>, guard: DecisionTableGuard) -> KernelDocument {
            let mut states = IndexMap::new();
            states.insert(
                "start".into(),
                atomic(vec![Transition {
                    event: Some(TransitionEvent::from_authoring_trigger("decide")),
                    target: "end".into(),
                    guard: Some(Guard::DecisionTable(guard)),
                    actions: vec![],
                    actor: None,
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
                governance: None,
                agents: Vec::new(),
                ai_oversight: None,
                signature: None,
                custody: None,
                advanced: None,
                assurance: None,
                intake: None,
                bindings: Vec::new(),
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
                &fixture_minter(),
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
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case_overlap))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-053 non-boolean output cell");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-053"), "expected K-053 in: {msg}");
                    assert!(
                        msg.contains("output cell"),
                        "expected 'output cell' in: {msg}"
                    );
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-053 non-boolean input cell");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-053"), "expected K-053 in: {msg}");
                    assert!(
                        msg.contains("input cell"),
                        "expected 'input cell' in: {msg}"
                    );
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
            let err = eval
                .process_event("decide", None, None)
                .expect_err("expected K-051 missing inputBindings entry");
            match err {
                EvalError::Guard(msg) => {
                    assert!(msg.contains("K-051"), "expected K-051 in: {msg}");
                    assert!(
                        msg.contains("inputBindings"),
                        "expected 'inputBindings' in: {msg}"
                    );
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
            let mut eval =
                Evaluator::with_time_and_case_state(doc, &fixture_minter(), 0, Some(&case))
                    .unwrap();
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
