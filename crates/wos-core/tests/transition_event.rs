// Rust guideline compliant 2026-02-21

//! TransitionEvent legacy coercion and dispatch label invariants.

use wos_core::TransitionEvent;

#[test]
fn legacy_unknown_dollar_prefix_keeps_verbatim_message_name() {
    let ev = TransitionEvent::from_legacy_string("$custom");
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
