// Rust guideline compliant 2026-02-21

//! `Command` enum — all atomic mutations understood by `RawWosProject`.

use serde::{Deserialize, Serialize};
use wos_core::{ActorKind, ImpactLevel, StateKind, TransitionEvent};

use crate::diagnostics::AuthoringDiagnostic;

/// The result of applying a single command.
///
/// On success, carries an `AppliedCommand` (the inverse for undo) and any
/// auto-generated side-effect identifier.  On failure, carries the diagnostic
/// describing why the command was rejected.
pub type CommandResult = Result<AppliedCommand, AuthoringDiagnostic>;

/// Record of a successfully applied command, used to drive undo.
///
/// Holds whatever information the handler needs to reverse the mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedCommand {
    /// Human-readable label for audit logs.
    pub label: String,

    /// The inverse command. Dispatching this command against the document
    /// restores it to the pre-application state for the limited case of undo.
    ///
    /// `None` for commands that cannot be trivially inverted at this level
    /// (undo for those commands uses full document snapshot restoration).
    ///
    /// `pub(crate)` because `Command` itself is an internal dispatch enum;
    /// the public view of `AppliedCommand` is just its `label` for audit logs.
    pub(crate) inverse: Option<Box<Command>>,
}

impl AppliedCommand {
    /// Construct a labeled `AppliedCommand` with an explicit inverse.
    ///
    /// `pub(crate)` because the argument type is internal.
    pub(crate) fn with_inverse(label: impl Into<String>, inverse: Command) -> Self {
        Self {
            label: label.into(),
            inverse: Some(Box::new(inverse)),
        }
    }

    /// Construct a labeled `AppliedCommand` without an inverse (snapshot-based undo).
    pub fn without_inverse(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            inverse: None,
        }
    }
}

/// All atomic document mutations.
///
/// `pub(crate)` so in-crate handlers and tests construct variants directly
/// while external consumers (including `wos-mcp`) go through the intent-driven
/// `WosProject` / `IWosProjectCore` seam — they never see `Command` itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Command {
    // ── Lifecycle ──────────────────────────────────────────────────────────
    /// Add a new top-level state.
    AddState {
        /// Unique state identifier within `lifecycle.states`.
        id: String,
        /// Atomic, compound, parallel, or final.
        kind: StateKind,
        /// Optional human-readable description stored on the state.
        description: Option<String>,
        /// Optional metadata stored under `state.extensions.x-meta`.
        metadata: Option<serde_json::Value>,
    },

    /// Remove a top-level state by identifier.
    ///
    /// Also removes any outgoing transitions from other states that target
    /// the removed state, preventing dangling references.
    RemoveState {
        /// Identifier of the state to remove.
        id: String,
    },

    /// Set the document-level initial state identifier.
    SetInitialState {
        /// State identifier to make the initial state.
        state_id: String,
    },

    /// Rename a top-level state.
    RenameState {
        /// Current identifier.
        old_id: String,
        /// Desired new identifier.
        new_id: String,
    },

    // ── Transitions ────────────────────────────────────────────────────────
    /// Add a transition from one state to another.
    AddTransition {
        /// Source state identifier.
        from_state: String,
        /// Target state identifier.
        to_state: String,
        /// Optional guard FEL expression.
        guard: Option<String>,
        /// Legacy string event (trimmed, parsed via [`TransitionEvent::from_legacy_string`]).
        event: Option<String>,
        /// Typed event; when set, must be the only event source (leave `event` empty).
        #[serde(default)]
        event_typed: Option<TransitionEvent>,
    },

    // ── Actors ─────────────────────────────────────────────────────────────
    /// Add an actor declaration.
    ///
    /// AI agents are NOT actors — they live in `x-wos-ai.agents`. Custom
    /// actor categories go through the `actorExtension` extension seam
    /// (kernel §10.6), not new `ActorKind` variants.
    AddActor {
        /// Unique actor identifier.
        id: String,
        /// Actor kind (human or system).
        kind: ActorKind,
    },

    /// Remove an actor declaration by identifier.
    ///
    /// Emits a warning if the actor is referenced by any transition's
    /// `assignTo` action; does not error (authoring may be mid-flight).
    RemoveActor {
        /// Identifier of the actor to remove.
        id: String,
    },

    /// Set an extension key on a specific actor (kernel §10.6 actorExtension).
    AddActorExtension {
        /// Identifier of the actor to annotate.
        actor_id: String,
        /// Extension key; must begin with `x-`.
        key: String,
        /// JSON value to store.
        value: serde_json::Value,
    },

    // ── Governance ─────────────────────────────────────────────────────────
    /// Set the document-level impact classification.
    SetImpactLevel {
        /// New impact level.
        level: ImpactLevel,
    },

    /// Add a named contract reference.
    AddContract {
        /// Contract name (map key in `contracts`).
        name: String,
        /// Binding type string (e.g., `"formspec"`).
        binding: String,
        /// Reference URI.
        ref_uri: String,
    },

    // ── AI integration ─────────────────────────────────────────────────────
    /// Append a deontic constraint to `x-wos-ai`.
    AddActorDeontic {
        /// Constraint identifier.
        constraint_id: String,
        /// Deontic rule description (stored verbatim in the extension object).
        rule: String,
    },

    // ── Milestones ─────────────────────────────────────────────────────────
    /// Add a named milestone condition under `lifecycle.milestones`.
    AddMilestone {
        /// Unique milestone identifier (map key under `lifecycle.milestones`).
        milestone_id: String,
        /// FEL condition string evaluated against the case state (kernel §S4.13).
        condition: String,
    },

    /// Remove a milestone by identifier.
    RemoveMilestone {
        /// Identifier of the milestone to remove.
        milestone_id: String,
    },

    // ── Timers ─────────────────────────────────────────────────────────────
    /// Record a timer configuration in `x-wos-timers`.
    SetTimer {
        /// Timer identifier.
        timer_id: String,
        /// ISO 8601 duration string.
        duration: String,
    },

    // ── Governance ─────────────────────────────────────────────────────────
    /// Record a due-process path under `x-wos-governance.dueProcessPaths`.
    AddDueProcessPath {
        /// Unique path identifier.
        path_id: String,
        /// Human-readable description of the due-process path.
        description: String,
        /// Ordered list of step identifiers.
        steps: Vec<String>,
    },

    /// Add an assertion gate to `x-wos-governance.assertionGates`.
    AddAssertionGate {
        /// Unique gate identifier.
        gate_id: String,
        /// FEL assertion that must hold.
        assertion: String,
        /// Lifecycle transition event this gate guards.
        transition: String,
    },

    // ── AI integration ─────────────────────────────────────────────────────
    /// Register an AI agent under `x-wos-ai.agents`.
    AddAiAgent {
        /// Unique agent identifier.
        agent_id: String,
        /// Role description.
        role: String,
        /// Model identifier string (e.g. "claude-3-5-sonnet").
        model: String,
        /// Capability strings (e.g. ["read_case_file", "submit_review"]).
        capabilities: Vec<String>,
    },

    /// Append a structured deontic constraint under `x-wos-ai.deonticConstraints`.
    ///
    /// Replaces `AddActorDeontic` for new MCP tooling. `modality` ∈ `must | must_not | may`.
    AddDeonticConstraint {
        /// Constraint identifier.
        constraint_id: String,
        /// Actor or scope this constraint targets.
        target: String,
        /// Deontic modality: "must", "must_not", or "may".
        modality: String,
        /// Action description.
        action: String,
    },

    // ── Extensions ─────────────────────────────────────────────────────────
    /// Set an `x-`-prefixed extension key on the document.
    AddExtensionKey {
        /// Extension key; must begin with `x-`.
        key: String,
        /// JSON value to store.
        value: serde_json::Value,
    },
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that every Command variant can be constructed.
    #[test]
    fn construct_all_variants() {
        let _add_state = Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        };

        let _set_initial_state = Command::SetInitialState {
            state_id: "draft".into(),
        };

        let _remove_state = Command::RemoveState { id: "draft".into() };

        let _rename_state = Command::RenameState {
            old_id: "draft".into(),
            new_id: "submitted".into(),
        };

        let _add_transition = Command::AddTransition {
            from_state: "submitted".into(),
            to_state: "approved".into(),
            guard: Some("caseFile.amount <= 50000".into()),
            event: Some("approve".into()),
            event_typed: None,
        };

        let _add_actor = Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        };

        let _remove_actor = Command::RemoveActor {
            id: "approver".into(),
        };

        let _set_impact = Command::SetImpactLevel {
            level: ImpactLevel::Operational,
        };

        let _add_contract = Command::AddContract {
            name: "purchaseOrderForm".into(),
            binding: "formspec".into(),
            ref_uri: "urn:formspec:example.gov:po:1.0".into(),
        };

        let _add_deontic = Command::AddActorDeontic {
            constraint_id: "noAutoApprove".into(),
            rule: "humans must review all orders".into(),
        };

        let _set_timer = Command::SetTimer {
            timer_id: "approvalDeadline".into(),
            duration: "P7D".into(),
        };

        let _add_milestone = Command::AddMilestone {
            milestone_id: "readyForReview".into(),
            condition: "caseFile.amount > 0".into(),
        };

        let _remove_milestone = Command::RemoveMilestone {
            milestone_id: "readyForReview".into(),
        };

        let _add_ext = Command::AddExtensionKey {
            key: "x-custom-meta".into(),
            value: serde_json::json!({ "owner": "procurement" }),
        };

        let _add_actor_ext = Command::AddActorExtension {
            actor_id: "reviewer".into(),
            key: "x-department".into(),
            value: serde_json::json!("finance"),
        };

        let _add_due_process = Command::AddDueProcessPath {
            path_id: "appealPath".into(),
            description: "Standard appeal process".into(),
            steps: vec!["review".into(), "hearing".into()],
        };

        let _add_gate = Command::AddAssertionGate {
            gate_id: "incomeCheck".into(),
            assertion: "caseFile.income > 0".into(),
            transition: "approve".into(),
        };

        let _add_ai_agent = Command::AddAiAgent {
            agent_id: "reviewBot".into(),
            role: "Automated reviewer".into(),
            model: "claude-3-5-sonnet".into(),
            capabilities: vec!["read_case_file".into()],
        };

        let _add_deontic = Command::AddDeonticConstraint {
            constraint_id: "mustNotAutoApprove".into(),
            target: "ai-agents".into(),
            modality: "must_not".into(),
            action: "auto-approve".into(),
        };
    }

    /// Verify that a Command serializes to the expected JSON structure.
    #[test]
    fn add_state_json_structure() {
        let cmd = Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        };

        let json = serde_json::to_value(&cmd).expect("serialization must succeed");

        assert_eq!(json["AddState"]["id"], "draft");
        // StateKind::Atomic serializes as the camelCase discriminant.
        assert_eq!(json["AddState"]["kind"], "atomic");
    }

    /// Verify that `AppliedCommand` round-trips through JSON.
    #[test]
    fn applied_command_round_trips() {
        let applied = AppliedCommand::with_inverse(
            "AddState(draft)",
            Command::RemoveState { id: "draft".into() },
        );

        let json = serde_json::to_string(&applied).expect("serialization");
        let back: AppliedCommand = serde_json::from_str(&json).expect("deserialization");
        assert_eq!(back.label, "AddState(draft)");
        assert!(back.inverse.is_some());
    }
}
