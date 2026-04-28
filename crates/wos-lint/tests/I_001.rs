// Rust guideline compliant 2026-04-14

//! Integration test for lint rule I-001: outputBinding JSONPath profile enforcement.
//!
//! The conformance harness is runtime-only; I-001 is a static lint check applied at
//! definition load time. These tests drive `wos_lint::lint_document` directly with
//! Integration Profile documents and assert the expected diagnostic outcomes.

use serde_json::json;
use wos_lint::{Severity, lint_document};

fn lint(doc: serde_json::Value) -> Vec<wos_lint::Diagnostic> {
    let json_str = serde_json::to_string(&doc).expect("serialization failed");
    lint_document(&json_str).expect("lint_document returned Err")
}

fn has_rule(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> bool {
    diagnostics.iter().any(|d| d.rule_id == rule_id)
}

fn severity_of(diagnostics: &[wos_lint::Diagnostic], rule_id: &str) -> Option<Severity> {
    diagnostics
        .iter()
        .find(|d| d.rule_id == rule_id)
        .map(|d| d.severity)
}

/// Fixture: Integration profile containing a filter expression in outputBinding.
///
/// Corresponds to the conceptual fixture `I-001-outputbinding-filter-rejected`:
/// an Integration Profile with `outputBinding.jsonPath: "$[?(@.ok)]"` must be
/// rejected with an I-001 error diagnostic so that filter expressions are caught
/// at definition load time, not silently accepted as runtime failures.
#[test]
fn i001_outputbinding_filter_rejected() {
    let doc = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": {
            "url": "https://example.gov/workflows/eligibility"
        },
        "bindings": {
            "eligibilityCheck": {
                "type": "request-response",
                "description": "Eligibility service with unsupported filter expression",
                "outputBinding": {
                    "caseFile.result": "$[?(@.ok)]"
                }
            }
        }
    });

    let diags = lint(doc);

    assert!(
        has_rule(&diags, "I-001"),
        "expected I-001 diagnostic for filter expression, got: {diags:?}"
    );
    assert_eq!(
        severity_of(&diags, "I-001"),
        Some(Severity::Error),
        "I-001 must be Error severity so definition load fails: {diags:?}"
    );
}

/// Fixture: Integration profile containing recursive descent in outputBinding.
///
/// Recursive descent (`..`) is excluded from the outputBinding profile for
/// predictability — it can match multiple nodes at unpredictable depths.
#[test]
fn i001_outputbinding_recursive_descent_rejected() {
    let doc = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": {
            "url": "https://example.gov/workflows/audit"
        },
        "bindings": {
            "auditService": {
                "type": "request-response",
                "outputBinding": {
                    "caseFile.deep": "$..value"
                }
            }
        }
    });

    let diags = lint(doc);

    assert!(
        has_rule(&diags, "I-001"),
        "expected I-001 for recursive descent, got: {diags:?}"
    );
    assert_eq!(severity_of(&diags, "I-001"), Some(Severity::Error));
}

/// Fixture: Integration profile using only supported outputBinding features.
///
/// Wildcard (`[*]`), slice (`[0:2]`), member access (`.key`), and index (`[n]`)
/// are all within the RFC 9535 output-binding profile and MUST NOT produce I-001.
#[test]
fn i001_supported_features_pass_without_error() {
    let doc = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": {
            "url": "https://example.gov/workflows/benefits"
        },
        "bindings": {
            "benefitsService": {
                "type": "request-response",
                "outputBinding": {
                    "caseFile.result": "$.result",
                    "caseFile.names": "$.items[*].name",
                    "caseFile.first": "$.items[0]",
                    "caseFile.slice": "$.items[0:2]",
                    "caseFile.quoted": "$['response']['data']"
                }
            }
        }
    });

    let diags = lint(doc);

    assert!(
        !has_rule(&diags, "I-001"),
        "unexpected I-001 on supported features: {diags:?}"
    );
}

/// Multiple bindings: only the binding with unsupported syntax produces I-001.
#[test]
fn i001_only_offending_binding_flagged() {
    let doc = json!({
        "$wosWorkflow": "1.0",
        "targetWorkflow": {
            "url": "https://example.gov/workflows/mixed"
        },
        "bindings": {
            "cleanService": {
                "type": "request-response",
                "outputBinding": {
                    "caseFile.data": "$.data"
                }
            },
            "dirtyService": {
                "type": "request-response",
                "outputBinding": {
                    "caseFile.result": "$[?(@.active == true)]"
                }
            }
        }
    });

    let diags = lint(doc);

    let i001_diags: Vec<_> = diags.iter().filter(|d| d.rule_id == "I-001").collect();
    assert_eq!(
        i001_diags.len(),
        1,
        "expected exactly one I-001 (from dirtyService), got: {i001_diags:?}"
    );
    assert!(
        i001_diags[0].path.contains("dirtyService"),
        "I-001 path should name the offending binding 'dirtyService', got: {}",
        i001_diags[0].path
    );
}
