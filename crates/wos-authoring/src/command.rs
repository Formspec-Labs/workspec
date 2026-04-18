// Rust guideline compliant 2026-02-21

//! `Command` enum — all atomic mutations understood by `RawWosProject`.

use serde::{Deserialize, Serialize};
use wos_core::{ActorKind, ImpactLevel, StateKind};

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

    /// The inverse command.  Dispatching this command against the document
    /// restores it to the pre-application state for the limited case of undo.
    ///
    /// `None` for commands that cannot be trivially inverted at this level
    /// (undo for those commands uses full document snapshot restoration).
    pub inverse: Option<Box<Command>>,
}

impl AppliedCommand {
    /// Construct a labeled `AppliedCommand` with an explicit inverse.
    pub fn with_inverse(label: impl Into<String>, inverse: Command) -> Self {
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
/// The enum is `pub` so that tests can construct variants directly.  It is not
/// re-exported from the crate root; consumers interact via `RawWosProject`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Command {
    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Add a new top-level state.
    AddState {
        /// Unique state identifier within `lifecycle.states`.
        id: String,
        /// Atomic, compound, parallel, or final.
        kind: StateKind,
    },

    /// Remove a top-level state by identifier.
    RemoveState {
        /// Identifier of the state to remove.
        id: String,
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
        /// Event name that triggers the transition.
        event: Option<String>,
    },

    // ── Actors ─────────────────────────────────────────────────────────────

    /// Add an actor declaration.
    AddActor {
        /// Unique actor identifier.
        id: String,
        /// Actor kind (human or system).
        kind: ActorKind,
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

    // ── Timers ─────────────────────────────────────────────────────────────

    /// Record a timer configuration in `x-wos-timers`.
    SetTimer {
        /// Timer identifier.
        timer_id: String,
        /// ISO 8601 duration string.
        duration: String,
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

    /// Verify that each of the 10 Command variants can be constructed.
    #[test]
    fn construct_all_ten_variants() {
        let _add_state = Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
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
        };

        let _add_actor = Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
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

        let _add_ext = Command::AddExtensionKey {
            key: "x-custom-meta".into(),
            value: serde_json::json!({ "owner": "procurement" }),
        };
    }

    /// Verify that a Command serializes to the expected JSON structure.
    #[test]
    fn add_state_json_structure() {
        let cmd = Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
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
