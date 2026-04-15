// Tests for binding selector dispatch in WorkflowEngine::new.
//
// Verifies that "formspec", "conformance", and absent `binding` fields all
// route to FormspecBinding, and that unknown binding names are rejected with
// a ConformanceError::Parse rather than silently falling through.

use wos_conformance::{run_fixture, ConformanceError};

/// Minimal inline fixture JSON with the given binding field (or None to omit it).
fn inline_fixture(binding: Option<&str>) -> String {
    let binding_field = match binding {
        Some(b) => format!(r#""binding": "{b}","#),
        None => String::new(),
    };
    format!(
        r#"{{
            {binding_field}
            "id": "binding-dispatch-test",
            "rule": "K-0.0",
            "description": "Binding dispatch test",
            "documents": {{"kernel": "inline"}},
            "inline_documents": {{
                "kernel": {{
                    "wos": "1.0",
                    "url": "urn:test:dispatch",
                    "version": "1.0.0",
                    "states": {{
                        "initial": {{
                            "type": "final"
                        }}
                    }},
                    "initialState": "initial",
                    "tasks": []
                }}
            }},
            "event_sequence": [],
            "expected_transitions": []
        }}"#
    )
}

#[test]
fn binding_formspec_succeeds_with_binding_used_formspec() {
    let fixture_json = inline_fixture(Some("formspec"));
    let result = run_fixture(&fixture_json, ".").expect("run_fixture must succeed");
    assert!(
        result.passed,
        "fixture with binding='formspec' should pass: {:?}",
        result.failures
    );
    assert_eq!(
        result.binding_used.as_deref(),
        Some("formspec"),
        "binding_used must be 'formspec'"
    );
}

#[test]
fn binding_conformance_alias_succeeds_with_binding_used_formspec() {
    let fixture_json = inline_fixture(Some("conformance"));
    let result = run_fixture(&fixture_json, ".").expect("run_fixture must succeed");
    assert!(
        result.passed,
        "fixture with binding='conformance' should pass: {:?}",
        result.failures
    );
    assert_eq!(
        result.binding_used.as_deref(),
        Some("formspec"),
        "binding_used must be 'formspec' even when selector is 'conformance'"
    );
}

#[test]
fn binding_omitted_succeeds_with_binding_used_formspec() {
    let fixture_json = inline_fixture(None);
    let result = run_fixture(&fixture_json, ".").expect("run_fixture must succeed");
    assert!(
        result.passed,
        "fixture with binding omitted should pass: {:?}",
        result.failures
    );
    assert_eq!(
        result.binding_used.as_deref(),
        Some("formspec"),
        "binding_used must be 'formspec' when binding is omitted"
    );
}

#[test]
fn binding_unknown_returns_parse_error() {
    let fixture_json = inline_fixture(Some("unknown"));
    let error = run_fixture(&fixture_json, ".").expect_err("unknown binding must return an error");
    assert!(
        matches!(error, ConformanceError::Parse(_)),
        "error must be ConformanceError::Parse, got: {:?}",
        error
    );
    let ConformanceError::Parse(message) = error else {
        unreachable!()
    };
    assert!(
        message.contains("unknown"),
        "error message must mention 'unknown', got: {message}"
    );
}
