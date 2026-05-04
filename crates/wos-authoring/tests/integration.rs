// Rust guideline compliant 2026-02-21

//! End-to-end authoring session: build → save → load → round-trip.
//!
//! A single test that exercises a realistic multi-command authoring flow
//! against the public `WosProject` façade — no access to `Command`,
//! `RawWosProject`, or `dispatch`. Creates states + transitions +
//! actors + impact level, interleaves undo/redo, serializes the
//! document to JSON (save), parses it back (load), and asserts
//! structural round-trip equality plus a realistic topology shape.

use wos_authoring::{ActorKind, ImpactLevel, StateKind, TransitionEvent, WosProject};
use wos_core::KernelDocument;
use wos_core::model::decision_table::Guard;

/// One long integration scenario. Mirrors the shape of
/// `fixtures/kernel/purchase-order-approval.json` at a high level
/// (actors, guarded-fork transitions) without attempting exact byte
/// equality, which JSON object key ordering does not guarantee.
#[test]
fn authoring_session_round_trip_with_undo_redo() {
    let mut project = WosProject::new(ImpactLevel::Operational, "Purchase Order Approval");

    // ── Actors ────────────────────────────────────────────────────────────
    project
        .add_actor("requester", ActorKind::Human)
        .expect("add requester");
    project
        .add_actor("approver", ActorKind::Human)
        .expect("add approver");
    project
        .add_actor("procurementSystem", ActorKind::System)
        .expect("add system actor");

    // ── States ────────────────────────────────────────────────────────────
    for id in [
        "submitted",
        "pendingDirectorApproval",
        "approved",
        "rejected",
        "cancelled",
        "archived",
    ] {
        project
            .add_state(id, StateKind::Atomic)
            .unwrap_or_else(|err| panic!("add_state({id}) must succeed: {err:?}"));
    }

    // ── Transitions (includes a guarded fork on `approve`) ───────────────
    project
        .add_transition(
            "submitted",
            "approved",
            Some("approve".into()),
            Some("caseFile.amount <= 50000".into()),
        )
        .expect("low-value approval");
    project
        .add_transition(
            "submitted",
            "pendingDirectorApproval",
            Some("approve".into()),
            Some("caseFile.amount > 50000".into()),
        )
        .expect("high-value approval");
    project
        .add_transition("submitted", "rejected", Some("reject".into()), None)
        .expect("rejection");
    project
        .add_transition("submitted", "cancelled", Some("cancel".into()), None)
        .expect("cancellation");
    project
        .add_transition(
            "pendingDirectorApproval",
            "approved",
            Some("directorApprove".into()),
            None,
        )
        .expect("director approval");
    project
        .add_transition(
            "pendingDirectorApproval",
            "rejected",
            Some("directorReject".into()),
            None,
        )
        .expect("director rejection");
    project
        .add_transition_typed(
            "submitted",
            "archived",
            Some(TransitionEvent::Message {
                name: "archive".into(),
                correlation_key: Some("corr-typed-1".into()),
                data: None,
            }),
            None,
        )
        .expect("typed message transition with correlation metadata");

    // ── Governance + milestone + extension surface ────────────────────────
    project
        .add_contract(
            "purchaseOrderForm",
            "formspec",
            "urn:formspec:example.gov:po:1.0",
        )
        .expect("add contract");
    project
        .add_milestone("readyForReview", "caseFile.amount > 0")
        .expect("add milestone");
    project
        .set_timer("approvalDeadline", "P7D")
        .expect("set timer");
    project
        .add_actor_deontic("humanApprovalRequired", "humans must review all orders")
        .expect("add deontic");

    // ── Interleave undo / redo mid-session ────────────────────────────────
    // Undo the deontic + timer, then redo the deontic only. This leaves
    // the timer un-applied and keeps the deontic.
    project.undo().expect("undo deontic");
    project.undo().expect("undo timer");
    assert!(
        project.snapshot().extensions.get("x-wos-timers").is_none(),
        "undo must have removed x-wos-timers"
    );
    project.redo().expect("redo timer");
    assert!(
        project.snapshot().extensions.get("x-wos-timers").is_some(),
        "redo must have restored x-wos-timers"
    );
    // Redo the deontic (LIFO pop from the redo stack, restoring the
    // last-undone operation). The redo stack retains its remaining
    // entries; a later forward dispatch would be what clears it.
    project.redo().expect("redo deontic");

    // ── Upgrade the document to rights-impacting per §S6 ─────────────────
    project
        .set_impact_level(ImpactLevel::RightsImpacting)
        .expect("escalate impact level");

    // ── Save: serialize to JSON ──────────────────────────────────────────
    let exported = project.snapshot();
    let json = serde_json::to_string_pretty(&exported).expect("serialize KernelDocument");

    // ── Load: parse JSON back into a KernelDocument ──────────────────────
    let reloaded: KernelDocument = serde_json::from_str(&json).expect("parse round-tripped JSON");

    // ── Round-trip assertions ────────────────────────────────────────────
    assert_eq!(reloaded.wos_workflow, "1.0");
    assert_eq!(reloaded.title.as_deref(), Some("Purchase Order Approval"));
    assert_eq!(
        reloaded.impact_level,
        Some(ImpactLevel::RightsImpacting),
        "post-escalation impact level must survive round-trip"
    );

    // Actor count + ids preserved.
    assert_eq!(reloaded.actors.len(), 3);
    let actor_ids: Vec<&str> = reloaded.actors.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(actor_ids, ["requester", "approver", "procurementSystem"]);

    // Lifecycle states preserved with insertion order.
    let state_ids: Vec<&str> = reloaded
        .lifecycle
        .states
        .keys()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        state_ids,
        [
            "submitted",
            "pendingDirectorApproval",
            "approved",
            "rejected",
            "cancelled",
            "archived"
        ]
    );

    let archive_typed = reloaded.lifecycle.states["submitted"]
        .transitions
        .iter()
        .find(|t| t.target == "archived")
        .expect("typed archive transition must survive round-trip");
    assert!(
        matches!(
            &archive_typed.event,
            Some(TransitionEvent::Message {
                name,
                correlation_key: Some(ck),
                ..
            }) if name == "archive" && ck == "corr-typed-1"
        ),
        "expected typed Message event with correlationKey, got {:?}",
        archive_typed.event
    );

    // Guarded fork: `submitted` has two `approve` transitions distinguished by guard.
    let submitted_transitions = &reloaded.lifecycle.states["submitted"].transitions;
    let approve_transitions: Vec<&_> = submitted_transitions
        .iter()
        .filter(|t| {
            t.event
                .as_ref()
                .is_some_and(|e| e.runtime_dispatch_label() == "approve")
        })
        .collect();
    assert_eq!(approve_transitions.len(), 2, "guarded fork preserved");
    assert!(
        approve_transitions
            .iter()
            .any(|t| t.guard.as_ref().and_then(Guard::as_fel_str)
                == Some("caseFile.amount <= 50000")),
    );
    assert!(
        approve_transitions
            .iter()
            .any(|t| t.guard.as_ref().and_then(Guard::as_fel_str)
                == Some("caseFile.amount > 50000")),
    );

    // Governance surface.
    assert!(reloaded.contracts.contains_key("purchaseOrderForm"));
    assert_eq!(
        reloaded.lifecycle.milestones["readyForReview"].condition,
        "caseFile.amount > 0"
    );

    // Extension payloads.
    let timers = reloaded
        .extensions
        .get("x-wos-timers")
        .expect("x-wos-timers extension present");
    assert_eq!(timers["approvalDeadline"]["duration"], "P7D");

    let ai = reloaded
        .extensions
        .get("x-wos-ai")
        .expect("x-wos-ai extension present");
    let constraints = ai["deonticConstraints"]
        .as_array()
        .expect("deonticConstraints is an array");
    assert_eq!(constraints.len(), 1);
    assert_eq!(constraints[0]["id"], "humanApprovalRequired");
}
