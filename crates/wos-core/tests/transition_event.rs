// Rust guideline compliant 2026-02-21

//! TransitionEvent legacy coercion and dispatch label invariants.

use wos_core::TransitionEvent;
use wos_core::model::kernel::SignalScope;

#[test]
fn legacy_unknown_dollar_prefix_keeps_verbatim_message_name() {
    let ev = TransitionEvent::from_authoring_trigger("$custom");
    assert!(
        matches!(ev, TransitionEvent::Message { ref name, .. } if name == "$custom"),
        "expected verbatim `$custom` message name, got {ev:?}"
    );
}

#[test]
fn error_runtime_label_is_dollar_error_matches_dispatch() {
    let ev = TransitionEvent::Error {
        code: "kernel.validation".into(),
        action_path: Some("/caseFile/total".into()),
    };
    assert_eq!(ev.runtime_dispatch_label(), "$error");
    assert!(ev.matches_runtime_dispatch("$error"));
    assert!(!ev.matches_runtime_dispatch("error:kernel.validation"));
    assert_eq!(ev.authoring_display_label(), "error:kernel.validation");
}

// ── $related.* relationship-event round-trip (regression for prefix-stripping) ──

#[test]
fn related_event_coerces_to_signal_with_full_prefix_preserved() {
    let ev = TransitionEvent::from_authoring_trigger("$related.resolved");
    match ev {
        TransitionEvent::Signal { ref name, scope } => {
            assert_eq!(
                name, "$related.resolved",
                "Signal.name must preserve the full `$related.*` prefix per ADR 0064 / ADR-0063 follow-up; \
                 stripping the prefix made bare-string transitions silently unmatchable against \
                 kernel-emitted relationship events."
            );
            assert_eq!(scope, SignalScope::Related);
        }
        other => panic!("expected Signal scope=Related, got {other:?}"),
    }
}

#[test]
fn related_event_runtime_dispatch_label_includes_prefix() {
    let ev = TransitionEvent::from_authoring_trigger("$related.holdReleased");
    assert_eq!(ev.runtime_dispatch_label(), "$related.holdReleased");
}

#[test]
fn related_event_matches_full_prefixed_runtime_event_name() {
    let ev = TransitionEvent::from_authoring_trigger("$related.stateChanged");
    assert!(
        ev.matches_runtime_dispatch("$related.stateChanged"),
        "Signal coerced from `$related.stateChanged` MUST match a runtime event named \
         `$related.stateChanged`. This was the silent-unmatch bug — coercion stripped the prefix \
         to `stateChanged`, but the runtime dispatches the full `$related.*` name."
    );
    assert!(
        !ev.matches_runtime_dispatch("stateChanged"),
        "Signal coerced from `$related.stateChanged` MUST NOT match the bare `stateChanged` \
         (which would be a different Instance-scope signal)."
    );
}
