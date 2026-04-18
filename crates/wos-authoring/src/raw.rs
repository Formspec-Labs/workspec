// Rust guideline compliant 2026-02-21

//! `RawWosProject` — the low-level authoring core.
//!
//! `IWosProjectCore` defines the dispatch/undo/redo/snapshot contract.
//! `RawWosProject` owns the `KernelDocument` and implements that contract.

use std::collections::HashMap;

use wos_core::{
    ActorKind, ImpactLevel, KernelDocument, Lifecycle, StateKind,
    model::kernel::{Actor, State, Transition},
};

use crate::{
    command::{AppliedCommand, Command, CommandResult},
    diagnostics::{AuthoringDiagnostic, Severity},
};

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Core authoring contract: undo/redo, snapshot, diagnostics.
///
/// `dispatch` is deliberately NOT on this trait — it takes the `pub(crate)`
/// `Command` enum, so exposing it on a public trait would leak a private type.
/// Command dispatch is an inherent `pub(crate)` method on `RawWosProject`
/// and is the seam the forthcoming `WosProject` façade (Task 6) calls into.
/// External consumers only see the intent-driven helper methods on
/// `WosProject`, never `Command` itself.
pub trait IWosProjectCore {
    /// Reverse the last command.
    ///
    /// Returns `Err` until undo is fully implemented (Task 5).
    fn undo(&mut self) -> Result<(), AuthoringDiagnostic>;

    /// Re-apply a reversed command.
    ///
    /// Returns `Err` until redo is fully implemented (Task 5).
    fn redo(&mut self) -> Result<(), AuthoringDiagnostic>;

    /// Return a clone of the current document state.
    fn snapshot(&self) -> KernelDocument;

    /// Return all diagnostics accumulated during this session.
    fn diagnostics(&self) -> &[AuthoringDiagnostic];
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Construct a minimal valid `State` with all optional fields at defaults.
fn empty_state(kind: StateKind) -> State {
    State {
        kind,
        description: None,
        transitions: Vec::new(),
        tags: Vec::new(),
        on_entry: Vec::new(),
        on_exit: Vec::new(),
        initial_state: None,
        states: indexmap::IndexMap::new(),
        regions: indexmap::IndexMap::new(),
        cancellation_policy: None,
        history_state: None,
        extensions: HashMap::new(),
    }
}

/// Build the minimal `KernelDocument` used by `RawWosProject::new`.
fn minimal_document(impact_level: ImpactLevel, title: String) -> KernelDocument {
    KernelDocument {
        wos_kernel: "1.0".into(),
        schema: None,
        url: None,
        version: None,
        title: Some(title),
        description: None,
        status: None,
        impact_level: Some(impact_level),
        actors: Vec::new(),
        lifecycle: Lifecycle {
            // Placeholder; callers must set an initial state before the document
            // is used for evaluation.  The empty string is the conventional
            // "no initial state yet" sentinel used during authoring.
            initial_state: String::new(),
            states: indexmap::IndexMap::new(),
            milestones: HashMap::new(),
        },
        case_file: None,
        contracts: HashMap::new(),
        provenance: None,
        execution: None,
        evaluation_mode: None,
        max_relationship_event_depth: None,
        extensions: HashMap::new(),
    }
}

// ── RawWosProject ─────────────────────────────────────────────────────────────

/// Low-level authoring core: owns a `KernelDocument` and drives it via commands.
///
/// Handlers for `AddState` and `AddTransition` are fully implemented.
/// The remaining eight command variants return `Err(AuthoringDiagnostic)` until
/// their handlers land in Task 4.
///
/// Undo and redo return `Err` stubs; Task 5 replaces them with snapshot-based
/// implementations.
#[derive(Debug)]
pub struct RawWosProject {
    doc: KernelDocument,
    /// Applied commands (for future undo; stubs in Tasks 1-3).
    history: Vec<AppliedCommand>,
    /// Reverted commands pending redo.
    redo_stack: Vec<AppliedCommand>,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl RawWosProject {
    /// Construct a minimal valid project with the given impact level and title.
    ///
    /// The document has no states, no actors, and no initial state yet.
    pub fn new(impact_level: ImpactLevel, title: impl Into<String>) -> Self {
        Self {
            doc: minimal_document(impact_level, title.into()),
            history: Vec::new(),
            redo_stack: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    // ── AddState handler ──────────────────────────────────────────────────

    fn apply_add_state(&mut self, id: String, kind: StateKind) -> CommandResult {
        if self.doc.lifecycle.states.contains_key(&id) {
            return Err(AuthoringDiagnostic::error(
                format!("/lifecycle/states/{id}"),
                format!("state '{id}' already exists"),
            ));
        }

        self.doc.lifecycle.states.insert(id.clone(), empty_state(kind));

        Ok(AppliedCommand::with_inverse(
            format!("AddState({id})"),
            Command::RemoveState { id },
        ))
    }

    // ── RemoveState handler ───────────────────────────────────────────────

    fn apply_remove_state(&mut self, id: String) -> CommandResult {
        if !self.doc.lifecycle.states.contains_key(&id) {
            return Err(AuthoringDiagnostic::error(
                format!("/lifecycle/states/{id}"),
                format!("state '{id}' not found"),
            ));
        }

        // Warn about dangling transitions that target the removed state.
        for (src_id, src_state) in &self.doc.lifecycle.states {
            for transition in &src_state.transitions {
                if transition.target == id {
                    self.diagnostics.push(AuthoringDiagnostic {
                        command_index: None,
                        severity: Severity::Warning,
                        path: format!(
                            "/lifecycle/states/{src_id}/transitions/{}",
                            transition.event
                        ),
                        message: format!(
                            "transition from '{src_id}' on '{event}' targets removed state '{id}'",
                            event = transition.event
                        ),
                    });
                }
            }
        }

        self.doc.lifecycle.states.shift_remove(&id);

        Ok(AppliedCommand::without_inverse(format!("RemoveState({id})")))
    }

    // ── RenameState handler (stub) ────────────────────────────────────────

    fn apply_rename_state(&mut self, old_id: String, new_id: String) -> CommandResult {
        Err(AuthoringDiagnostic::error(
            format!("/lifecycle/states/{old_id}"),
            format!("RenameState('{old_id}' → '{new_id}') not yet implemented — lands in Task 4"),
        ))
    }

    // ── AddTransition handler ─────────────────────────────────────────────

    fn apply_add_transition(
        &mut self,
        from_state: String,
        to_state: String,
        guard: Option<String>,
        event: Option<String>,
    ) -> CommandResult {
        // Both endpoints must exist before a transition can be created.
        if !self.doc.lifecycle.states.contains_key(&from_state) {
            return Err(AuthoringDiagnostic::error(
                format!("/lifecycle/states/{from_state}"),
                format!("source state '{from_state}' does not exist"),
            ));
        }
        if !self.doc.lifecycle.states.contains_key(&to_state) {
            return Err(AuthoringDiagnostic::error(
                format!("/lifecycle/states/{to_state}"),
                format!("target state '{to_state}' does not exist"),
            ));
        }

        let event_name = event.unwrap_or_default();

        let transition = Transition {
            event: event_name.clone(),
            target: to_state.clone(),
            guard,
            actions: Vec::new(),
            description: None,
            tags: Vec::new(),
        };

        // Unwrap is safe: we verified `from_state` exists above.
        self.doc
            .lifecycle
            .states
            .get_mut(&from_state)
            .expect("state verified to exist")
            .transitions
            .push(transition);

        Ok(AppliedCommand::without_inverse(format!(
            "AddTransition({from_state} --[{event_name}]--> {to_state})"
        )))
    }

    // ── AddActor / RemoveActor handlers ───────────────────────────────────

    fn apply_add_actor(&mut self, id: String, kind: ActorKind) -> CommandResult {
        if self.doc.actors.iter().any(|a| a.id == id) {
            return Err(AuthoringDiagnostic::error(
                format!("/actors/{id}"),
                format!("actor '{id}' already exists"),
            ));
        }

        self.doc.actors.push(Actor {
            id: id.clone(),
            kind,
            description: None,
            extensions: HashMap::new(),
        });

        Ok(AppliedCommand::with_inverse(
            format!("AddActor({id})"),
            Command::RemoveActor { id },
        ))
    }

    fn apply_remove_actor(&mut self, id: String) -> CommandResult {
        let index = match self.doc.actors.iter().position(|a| a.id == id) {
            Some(idx) => idx,
            None => {
                return Err(AuthoringDiagnostic::error(
                    format!("/actors/{id}"),
                    format!("actor '{id}' not found"),
                ));
            }
        };

        // Warn (do not error) when the actor is referenced by any
        // transition's `assignTo` action — authors may be mid-migration.
        for (state_id, state) in &self.doc.lifecycle.states {
            for transition in &state.transitions {
                for (action_idx, action) in transition.actions.iter().enumerate() {
                    if action.assign_to.as_deref() == Some(id.as_str()) {
                        self.diagnostics.push(AuthoringDiagnostic::warning(
                            format!(
                                "/lifecycle/states/{state_id}/transitions/{}/actions/{action_idx}",
                                transition.event
                            ),
                            format!(
                                "action assigns to removed actor '{id}' \
                                 on transition '{state_id}' --[{}]--> '{}'",
                                transition.event, transition.target
                            ),
                        ));
                    }
                }
            }
        }

        self.doc.actors.remove(index);

        Ok(AppliedCommand::without_inverse(format!("RemoveActor({id})")))
    }

    // ── SetImpactLevel handler (stub) ─────────────────────────────────────

    fn apply_set_impact_level(&mut self, level: ImpactLevel) -> CommandResult {
        Err(AuthoringDiagnostic::error(
            "/impactLevel",
            format!("SetImpactLevel({level:?}) not yet implemented — lands in Task 4"),
        ))
    }

    // ── AddContract handler (stub) ────────────────────────────────────────

    fn apply_add_contract(
        &mut self,
        name: String,
        binding: String,
        ref_uri: String,
    ) -> CommandResult {
        Err(AuthoringDiagnostic::error(
            format!("/contracts/{name}"),
            format!(
                "AddContract('{name}', '{binding}', '{ref_uri}') not yet implemented — lands in Task 4"
            ),
        ))
    }

    // ── AddActorDeontic handler (stub) ────────────────────────────────────

    fn apply_add_actor_deontic(
        &mut self,
        constraint_id: String,
        _rule: String,
    ) -> CommandResult {
        Err(AuthoringDiagnostic::error(
            format!("/extensions/x-wos-ai/deonticConstraints/{constraint_id}"),
            format!(
                "AddActorDeontic('{constraint_id}') not yet implemented — lands in Task 4"
            ),
        ))
    }

    // ── SetTimer handler (stub) ───────────────────────────────────────────

    fn apply_set_timer(&mut self, timer_id: String, duration: String) -> CommandResult {
        Err(AuthoringDiagnostic::error(
            format!("/extensions/x-wos-timers/{timer_id}"),
            format!(
                "SetTimer('{timer_id}', '{duration}') not yet implemented — lands in Task 4"
            ),
        ))
    }

    // ── AddExtensionKey handler (stub) ────────────────────────────────────

    fn apply_add_extension_key(
        &mut self,
        key: String,
        _value: serde_json::Value,
    ) -> CommandResult {
        if !key.starts_with("x-") {
            return Err(AuthoringDiagnostic::error(
                format!("/extensions/{key}"),
                format!("extension key '{key}' must start with 'x-'"),
            ));
        }
        Err(AuthoringDiagnostic::error(
            format!("/extensions/{key}"),
            format!(
                "AddExtensionKey('{key}') not yet implemented — lands in Task 4"
            ),
        ))
    }
}

// ── Inherent command dispatch ─────────────────────────────────────────────────

impl RawWosProject {
    /// Apply a command to the document, returning a record of what was done.
    ///
    /// `pub(crate)` because `Command` is an internal dispatch enum; external
    /// consumers interact with `WosProject` / `IWosProjectCore` helper methods.
    pub(crate) fn dispatch(&mut self, cmd: Command) -> CommandResult {
        let result = match cmd {
            Command::AddState { id, kind } => self.apply_add_state(id, kind),
            Command::RemoveState { id } => self.apply_remove_state(id),
            Command::RenameState { old_id, new_id } => self.apply_rename_state(old_id, new_id),
            Command::AddTransition {
                from_state,
                to_state,
                guard,
                event,
            } => self.apply_add_transition(from_state, to_state, guard, event),
            Command::AddActor { id, kind } => self.apply_add_actor(id, kind),
            Command::RemoveActor { id } => self.apply_remove_actor(id),
            Command::SetImpactLevel { level } => self.apply_set_impact_level(level),
            Command::AddContract {
                name,
                binding,
                ref_uri,
            } => self.apply_add_contract(name, binding, ref_uri),
            Command::AddActorDeontic {
                constraint_id,
                rule,
            } => self.apply_add_actor_deontic(constraint_id, rule),
            Command::SetTimer { timer_id, duration } => self.apply_set_timer(timer_id, duration),
            Command::AddExtensionKey { key, value } => self.apply_add_extension_key(key, value),
        };

        if let Ok(ref applied) = result {
            self.history.push(applied.clone());
            // A new forward command clears the redo stack.
            self.redo_stack.clear();
        }

        result
    }
}

// ── IWosProjectCore implementation ────────────────────────────────────────────

impl IWosProjectCore for RawWosProject {
    fn undo(&mut self) -> Result<(), AuthoringDiagnostic> {
        // Stub — Task 5 replaces this with snapshot-based restoration.
        Err(AuthoringDiagnostic::error(
            "/",
            "undo not yet implemented — lands in Task 5",
        ))
    }

    fn redo(&mut self) -> Result<(), AuthoringDiagnostic> {
        // Stub — Task 5 replaces this with snapshot-based restoration.
        Err(AuthoringDiagnostic::error(
            "/",
            "redo not yet implemented — lands in Task 5",
        ))
    }

    fn snapshot(&self) -> KernelDocument {
        self.doc.clone()
    }

    fn diagnostics(&self) -> &[AuthoringDiagnostic] {
        &self.diagnostics
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project() -> RawWosProject {
        RawWosProject::new(ImpactLevel::Operational, "Test Project")
    }

    // ── AddState ──────────────────────────────────────────────────────────

    /// Dispatching AddState inserts the state into the document snapshot.
    #[test]
    fn add_state_appears_in_snapshot() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
        })
        .expect("AddState must succeed on an empty project");

        let snap = p.snapshot();
        assert!(
            snap.lifecycle.states.contains_key("draft"),
            "snapshot must contain the newly added state"
        );
    }

    /// A second AddState with the same ID must fail.
    #[test]
    fn add_state_duplicate_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
        })
        .expect("first AddState must succeed");

        let err = p
            .dispatch(Command::AddState {
                id: "draft".into(),
                kind: StateKind::Compound,
            })
            .expect_err("duplicate AddState must return an error diagnostic");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("already exists"),
            "error message must mention 'already exists'"
        );
    }

    // ── AddTransition ──────────────────────────────────────────────────────

    /// Dispatching AddTransition inserts the transition into the source state.
    #[test]
    fn add_transition_appears_in_snapshot() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "approved".into(),
            kind: StateKind::Atomic,
        })
        .unwrap();

        p.dispatch(Command::AddTransition {
            from_state: "submitted".into(),
            to_state: "approved".into(),
            guard: Some("caseFile.amount <= 50000".into()),
            event: Some("approve".into()),
        })
        .expect("AddTransition must succeed when both states exist");

        let snap = p.snapshot();
        let transitions = &snap.lifecycle.states["submitted"].transitions;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].target, "approved");
        assert_eq!(transitions[0].event, "approve");
        assert_eq!(
            transitions[0].guard.as_deref(),
            Some("caseFile.amount <= 50000")
        );
    }

    /// AddTransition referencing an unknown source state must fail.
    #[test]
    fn add_transition_unknown_source_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "approved".into(),
            kind: StateKind::Atomic,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddTransition {
                from_state: "nonexistent".into(),
                to_state: "approved".into(),
                guard: None,
                event: Some("approve".into()),
            })
            .expect_err("AddTransition with unknown source must return an error");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("does not exist"),
            "error must mention 'does not exist'"
        );
    }

    /// AddTransition referencing an unknown target state must fail.
    #[test]
    fn add_transition_unknown_target_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddTransition {
                from_state: "submitted".into(),
                to_state: "nonexistent".into(),
                guard: None,
                event: Some("approve".into()),
            })
            .expect_err("AddTransition with unknown target must return an error");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("does not exist"),
            "error must mention 'does not exist'"
        );
    }

    // ── new() / snapshot() ────────────────────────────────────────────────

    /// A freshly constructed project has an empty state map.
    #[test]
    fn new_project_has_empty_lifecycle() {
        let p = make_project();
        let snap = p.snapshot();
        assert_eq!(snap.wos_kernel, "1.0");
        assert!(snap.lifecycle.states.is_empty());
    }

    // ── undo/redo stubs ───────────────────────────────────────────────────

    /// Undo returns Err until Task 5 implements it.
    #[test]
    fn undo_returns_stub_error() {
        let mut p = make_project();
        let result = p.undo();
        assert!(result.is_err(), "undo must return Err until Task 5");
    }

    /// Redo returns Err until Task 5 implements it.
    #[test]
    fn redo_returns_stub_error() {
        let mut p = make_project();
        let result = p.redo();
        assert!(result.is_err(), "redo must return Err until Task 5");
    }

    // ── AddActor / RemoveActor ────────────────────────────────────────────

    /// AddActor appends a human actor to the document.
    #[test]
    fn add_actor_human_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .expect("AddActor must succeed on empty project");

        let snap = p.snapshot();
        assert_eq!(snap.actors.len(), 1);
        assert_eq!(snap.actors[0].id, "approver");
        assert_eq!(snap.actors[0].kind, ActorKind::Human);
    }

    /// AddActor for the system kind also works — kernel §S3 allows only
    /// `human | system`. AI agents route through `x-wos-ai.agents`.
    #[test]
    fn add_actor_system_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "procurement".into(),
            kind: ActorKind::System,
        })
        .unwrap();

        let snap = p.snapshot();
        assert_eq!(snap.actors[0].kind, ActorKind::System);
    }

    /// AddActor twice with the same id must fail.
    #[test]
    fn add_actor_duplicate_returns_error() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddActor {
                id: "approver".into(),
                kind: ActorKind::Human,
            })
            .expect_err("duplicate AddActor must fail");

        assert!(err.message.contains("already exists"));
    }

    /// RemoveActor drops the actor from the vector.
    #[test]
    fn remove_actor_drops_entry() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .unwrap();
        p.dispatch(Command::RemoveActor {
            id: "approver".into(),
        })
        .expect("RemoveActor must succeed");

        assert!(p.snapshot().actors.is_empty());
    }

    /// RemoveActor for an unknown id errors.
    #[test]
    fn remove_actor_unknown_returns_error() {
        let mut p = make_project();
        let err = p
            .dispatch(Command::RemoveActor {
                id: "ghost".into(),
            })
            .expect_err("unknown actor must be rejected");
        assert!(err.message.contains("not found"));
    }

    // ── AddExtensionKey key validation ────────────────────────────────────

    /// AddExtensionKey with a key that lacks the `x-` prefix is rejected before dispatch.
    #[test]
    fn extension_key_must_start_with_x_dash() {
        let mut p = make_project();

        let err = p
            .dispatch(Command::AddExtensionKey {
                key: "custom-meta".into(),
                value: serde_json::json!({}),
            })
            .expect_err("missing x- prefix must be rejected");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("must start with 'x-'"),
            "error must mention the x- prefix requirement"
        );
    }
}
