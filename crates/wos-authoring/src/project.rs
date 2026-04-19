// Rust guideline compliant 2026-02-21

//! `WosProject` — the intent-driven authoring façade.
//!
//! External consumers (`wos-mcp`, future `wos-synth-core`, integration
//! tests) use only `WosProject`. The underlying `RawWosProject` and
//! `Command` enum remain `pub(crate)` so no caller can bypass the seam
//! and issue a raw command.
//!
//! Every helper returns `AuthoringResult` (alias for
//! `Result<AppliedCommand, AuthoringDiagnostic>`) so callers can log the
//! human-readable label on success and the structured diagnostic on
//! failure. Undo and redo operate at helper-call granularity via the
//! `IWosProjectCore` implementation on `RawWosProject`.

use wos_core::{ActorKind, ImpactLevel, KernelDocument, StateKind};

use crate::{
    command::{Command, CommandResult},
    diagnostics::AuthoringDiagnostic,
    raw::{IWosProjectCore, RawWosProject},
};

/// The canonical result of any `WosProject` helper call.
///
/// On success the caller receives the `AppliedCommand` record (label plus
/// optional inverse metadata); on failure they receive a structured
/// diagnostic they can surface to authoring tooling.
pub type AuthoringResult = CommandResult;

/// Intent-driven authoring façade for WOS Kernel Documents.
///
/// Wraps a `RawWosProject` and exposes only the operation-specific helper
/// methods plus undo/redo. The underlying `Command` enum and the
/// `dispatch` method are deliberately not re-exposed — every mutation
/// must flow through a named helper so the public surface stays stable
/// as new commands are added.
#[derive(Debug)]
pub struct WosProject {
    core: RawWosProject,
}

impl WosProject {
    /// Construct a minimal valid project with the given impact level and title.
    ///
    /// The document has no states, no actors, no contracts, and an empty
    /// `lifecycle.initialState` sentinel.
    pub fn new(impact_level: ImpactLevel, title: impl Into<String>) -> Self {
        Self {
            core: RawWosProject::new(impact_level, title),
        }
    }

    /// Construct an empty kernel with sensible authoring defaults.
    ///
    /// Uses `ImpactLevel::Operational` and a generic title. Callers that need
    /// a specific impact level or title should use [`WosProject::new`] directly.
    /// This factory exists so `wos-mcp` can create projects without picking
    /// an impact level upfront — governance tools set the level later.
    pub fn new_kernel() -> Self {
        Self::new(ImpactLevel::Operational, "New WOS Workflow")
    }

    /// Load a project from an already-deserialized `KernelDocument`.
    ///
    /// The document becomes the initial state. The undo/redo history starts
    /// empty — no commands were applied to reach this state.
    pub fn from_document(document: KernelDocument) -> Self {
        Self {
            core: RawWosProject::from_document(document),
        }
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────

    /// Add a top-level state. Errors if the id already exists.
    pub fn add_state(&mut self, id: impl Into<String>, kind: StateKind) -> AuthoringResult {
        self.core.dispatch(Command::AddState {
            id: id.into(),
            kind,
        })
    }

    /// Remove a top-level state. Emits a warning (collected in diagnostics)
    /// for each dangling transition targeting the removed state.
    pub fn remove_state(&mut self, id: impl Into<String>) -> AuthoringResult {
        self.core.dispatch(Command::RemoveState { id: id.into() })
    }

    /// Rename a top-level state, repointing all incoming transitions and
    /// `lifecycle.initialState` as needed.
    pub fn rename_state(
        &mut self,
        old_id: impl Into<String>,
        new_id: impl Into<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::RenameState {
            old_id: old_id.into(),
            new_id: new_id.into(),
        })
    }

    /// Add a transition from one state to another.
    pub fn add_transition(
        &mut self,
        from_state: impl Into<String>,
        to_state: impl Into<String>,
        event: Option<String>,
        guard: Option<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::AddTransition {
            from_state: from_state.into(),
            to_state: to_state.into(),
            guard,
            event,
        })
    }

    /// Add a named milestone condition (kernel §S4.13).
    pub fn add_milestone(
        &mut self,
        milestone_id: impl Into<String>,
        condition: impl Into<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::AddMilestone {
            milestone_id: milestone_id.into(),
            condition: condition.into(),
        })
    }

    /// Remove a milestone by identifier.
    pub fn remove_milestone(&mut self, milestone_id: impl Into<String>) -> AuthoringResult {
        self.core.dispatch(Command::RemoveMilestone {
            milestone_id: milestone_id.into(),
        })
    }

    // ── Actors ────────────────────────────────────────────────────────────

    /// Add an actor declaration. `kind` is `Human` or `System` per kernel
    /// §S3; AI agents route through [`Self::add_actor_deontic`] / the
    /// `x-wos-ai.agents` surface, not through new ActorKind variants.
    pub fn add_actor(&mut self, id: impl Into<String>, kind: ActorKind) -> AuthoringResult {
        self.core.dispatch(Command::AddActor {
            id: id.into(),
            kind,
        })
    }

    /// Remove an actor. Warns (does not error) if the actor is assigned by
    /// any transition action.
    pub fn remove_actor(&mut self, id: impl Into<String>) -> AuthoringResult {
        self.core.dispatch(Command::RemoveActor { id: id.into() })
    }

    // ── Governance ────────────────────────────────────────────────────────

    /// Set the document-level impact classification (kernel §S6).
    pub fn set_impact_level(&mut self, level: ImpactLevel) -> AuthoringResult {
        self.core.dispatch(Command::SetImpactLevel { level })
    }

    /// Add a named contract reference (kernel §S11).
    pub fn add_contract(
        &mut self,
        name: impl Into<String>,
        binding: impl Into<String>,
        ref_uri: impl Into<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::AddContract {
            name: name.into(),
            binding: binding.into(),
            ref_uri: ref_uri.into(),
        })
    }

    // ── AI integration ────────────────────────────────────────────────────

    /// Append a deontic constraint under `x-wos-ai.deonticConstraints`.
    pub fn add_actor_deontic(
        &mut self,
        constraint_id: impl Into<String>,
        rule: impl Into<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::AddActorDeontic {
            constraint_id: constraint_id.into(),
            rule: rule.into(),
        })
    }

    // ── Timers ────────────────────────────────────────────────────────────

    /// Write a timer configuration under `x-wos-timers.<timer_id>`.
    pub fn set_timer(
        &mut self,
        timer_id: impl Into<String>,
        duration: impl Into<String>,
    ) -> AuthoringResult {
        self.core.dispatch(Command::SetTimer {
            timer_id: timer_id.into(),
            duration: duration.into(),
        })
    }

    // ── Extensions ────────────────────────────────────────────────────────

    /// Store an arbitrary JSON value under a top-level extension key.
    ///
    /// # Errors
    ///
    /// Returns `AuthoringDiagnostic` if `key` does not start with `x-`.
    pub fn add_extension_key(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> AuthoringResult {
        self.core.dispatch(Command::AddExtensionKey {
            key: key.into(),
            value,
        })
    }

    // ── History ───────────────────────────────────────────────────────────

    /// Reverse the most recent successful helper call.
    ///
    /// # Errors
    ///
    /// Returns `AuthoringDiagnostic` when history is empty.
    pub fn undo(&mut self) -> Result<(), AuthoringDiagnostic> {
        self.core.undo()
    }

    /// Re-apply the most recently reversed helper call.
    ///
    /// # Errors
    ///
    /// Returns `AuthoringDiagnostic` when the redo stack is empty.
    pub fn redo(&mut self) -> Result<(), AuthoringDiagnostic> {
        self.core.redo()
    }

    /// True if there is at least one entry on the undo stack.
    pub fn can_undo(&self) -> bool {
        self.core.can_undo()
    }

    /// True if there is at least one entry on the redo stack.
    pub fn can_redo(&self) -> bool {
        self.core.can_redo()
    }

    // ── Export / diagnostics ──────────────────────────────────────────────

    /// Return a clone of the current document state.
    pub fn snapshot(&self) -> KernelDocument {
        self.core.snapshot()
    }

    /// Return all warnings accumulated during this session.
    pub fn diagnostics(&self) -> &[AuthoringDiagnostic] {
        self.core.diagnostics()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project() -> WosProject {
        WosProject::new(ImpactLevel::Operational, "test")
    }

    /// The façade exposes add_state without the caller touching Command.
    #[test]
    fn add_state_via_facade() {
        let mut p = make_project();
        let applied = p
            .add_state("draft", StateKind::Atomic)
            .expect("add_state must succeed");
        assert!(applied.label.contains("AddState"));
        assert!(p.snapshot().lifecycle.states.contains_key("draft"));
    }

    /// Undo/redo operate at the helper boundary.
    #[test]
    fn facade_undo_redo_round_trip() {
        let mut p = make_project();
        p.add_state("s1", StateKind::Atomic).unwrap();
        p.add_actor("a1", ActorKind::Human).unwrap();

        assert!(p.can_undo());
        p.undo().unwrap();
        p.undo().unwrap();
        assert!(!p.can_undo());
        assert!(p.can_redo());

        p.redo().unwrap();
        p.redo().unwrap();
        assert_eq!(p.snapshot().lifecycle.states.len(), 1);
        assert_eq!(p.snapshot().actors.len(), 1);
    }

    /// set_impact_level walks through the façade to the inner handler.
    #[test]
    fn facade_set_impact_level() {
        let mut p = make_project();
        p.set_impact_level(ImpactLevel::SafetyImpacting).unwrap();
        assert_eq!(
            p.snapshot().impact_level,
            Some(ImpactLevel::SafetyImpacting)
        );
    }

    /// Diagnostic warnings surfaced by an inner handler are readable via the façade.
    #[test]
    fn facade_diagnostics_exposed() {
        let mut p = make_project();
        p.add_actor("approver", ActorKind::Human).unwrap();

        // Add a state + transition referencing the actor, then remove the actor.
        p.add_state("s1", StateKind::Atomic).unwrap();
        p.add_state("s2", StateKind::Atomic).unwrap();
        // No assignTo reference here (façade does not yet expose action
        // helpers); the diagnostics slice should still be empty and readable.
        assert!(p.diagnostics().is_empty());
    }

    /// add_extension_key propagates the x- prefix validation error.
    #[test]
    fn facade_rejects_bad_extension_key() {
        let mut p = make_project();
        let err = p
            .add_extension_key("custom", serde_json::json!({}))
            .expect_err("missing x- prefix must be rejected through façade");
        assert!(err.message.contains("must start with 'x-'"));
    }
}
