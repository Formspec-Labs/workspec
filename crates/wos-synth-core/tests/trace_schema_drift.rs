//! Drift test — keep `schemas/synth/wos-synth-trace.schema.json` in sync with
//! the Rust types in `wos-synth-core::trace` and `wos-synth-core::synth_loop`.
//!
//! Mechanism (the simplest correct check — round-trip + validate):
//!
//!   1. Construct representative `SynthTrace` and `SynthOutcome` (both
//!      `Converged` and `Unconverged` variants) values that exercise every
//!      schema property, including the optional `conformance`,
//!      `path`, `suggested_fix`, and `related_docs` fields.
//!   2. Serialize each to JSON via `serde_json`.
//!   3. Validate the JSON against the published schema using the `jsonschema`
//!      crate. Any drift between Rust serde output and the schema surfaces
//!      as a validation error: a renamed field violates `additionalProperties:
//!      false`; a removed field violates the `required` array; a changed type
//!      violates `type`.
//!
//! When this test fails, the fix is one of:
//!   - The schema is wrong → update `schemas/synth/wos-synth-trace.schema.json`.
//!   - The Rust types changed and the schema needs the matching change.
//!   - Both are right but the test data is stale → update the test fixtures.

use std::path::PathBuf;

use serde_json::Value;

use wos_synth_core::synth_loop::SynthOutcome;
use wos_synth_core::tool_context::{ConformanceVerdict, LintFinding, Severity};
use wos_synth_core::trace::{IterationRecord, SynthTrace};

/// Build a `SynthTrace` that exercises every schema-defined property.
///
/// Two iterations:
///   - index 0: a "dirty" attempt with one error finding, full optional fields
///     populated (path, suggested_fix, related_docs, conformance).
///   - index 1: a "clean" attempt with zero findings and no conformance.
fn representative_trace() -> SynthTrace {
    let mut trace = SynthTrace::new();

    trace.push(IterationRecord {
        index: 0,
        attempt: r#"{"$wosKernel":"1.0","broken":true}"#.to_string(),
        lint_findings: vec![LintFinding {
            rule_id: "K-001".into(),
            severity: Severity::Error,
            message: "state 'approved' has no outbound transition".into(),
            path: Some("/states/approved".into()),
            suggested_fix: Some(
                "add property at /states/approved/type with value 'terminal'".into(),
            ),
            related_docs: vec![
                "specs/kernel/spec.md#S4.2".into(),
                "LINT-MATRIX.md#K-001".into(),
            ],
        }],
        conformance: Some(ConformanceVerdict {
            passed: false,
            summary: "step 1: expected state 'approved', got 'rejected'".into(),
        }),
        input_tokens: 1500,
        output_tokens: 450,
        cache_read_tokens: 0,
    });

    trace.push(IterationRecord {
        index: 1,
        attempt: r#"{"$wosKernel":"1.0","clean":true}"#.to_string(),
        lint_findings: vec![],
        conformance: None,
        input_tokens: 1620,
        output_tokens: 380,
        cache_read_tokens: 1500,
    });

    trace
}

/// Path to the published schema, relative to the workspace root.
fn schema_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|crates| crates.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .join("schemas/synth/wos-synth-trace.schema.json")
}

/// Load the published schema as `serde_json::Value`.
fn load_published_schema() -> Value {
    let path = schema_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

/// Compile the published schema into a `jsonschema::Validator`.
fn compile_schema() -> jsonschema::Validator {
    let schema = load_published_schema();
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .expect("published schema compiles as Draft 2020-12")
}

/// Format every validation error against `instance` for a panic message.
fn format_errors(validator: &jsonschema::Validator, instance: &Value) -> String {
    validator
        .iter_errors(instance)
        .map(|e| format!("  - at {}: {e}", e.instance_path()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn representative_trace_validates_against_published_schema() {
    let trace = representative_trace();
    let json = serde_json::to_value(&trace).expect("trace serializes");

    let validator = compile_schema();
    if !validator.is_valid(&json) {
        panic!(
            "SynthTrace serialization failed schema validation:\n{}\n\nserialized JSON:\n{}",
            format_errors(&validator, &json),
            serde_json::to_string_pretty(&json).unwrap(),
        );
    }
}

/// Build a validator that targets `/$defs/SynthOutcome` while keeping the
/// rest of the schema (including sibling `$defs`) reachable for `$ref`
/// resolution. We construct a synthetic root schema that hoists the
/// published `$defs` block and `$ref`s into the SynthOutcome definition.
fn synth_outcome_validator() -> jsonschema::Validator {
    let published = load_published_schema();
    let defs = published
        .get("$defs")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let wrapper = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": defs,
        "$ref": "#/$defs/SynthOutcome"
    });

    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&wrapper)
        .expect("SynthOutcome wrapper schema compiles")
}

#[test]
fn converged_outcome_validates_against_outcome_schema() {
    let outcome = SynthOutcome::Converged {
        document: r#"{"$wosKernel":"1.0","clean":true}"#.to_string(),
        trace: representative_trace(),
    };
    let json = serde_json::to_value(&outcome).expect("outcome serializes");

    let validator = synth_outcome_validator();
    if !validator.is_valid(&json) {
        panic!(
            "Converged outcome failed schema validation:\n{}\n\nserialized JSON:\n{}",
            format_errors(&validator, &json),
            serde_json::to_string_pretty(&json).unwrap(),
        );
    }
}

#[test]
fn unconverged_outcome_validates_against_outcome_schema() {
    let outcome = SynthOutcome::Unconverged {
        last_attempt: r#"{"$wosKernel":"1.0","still_broken":true}"#.to_string(),
        last_findings: vec![LintFinding {
            rule_id: "K-001".into(),
            severity: Severity::Error,
            message: "state 'approved' has no outbound transition".into(),
            path: None,
            suggested_fix: None,
            related_docs: vec![],
        }],
        trace: representative_trace(),
    };
    let json = serde_json::to_value(&outcome).expect("outcome serializes");

    let validator = synth_outcome_validator();
    if !validator.is_valid(&json) {
        panic!(
            "Unconverged outcome failed schema validation:\n{}\n\nserialized JSON:\n{}",
            format_errors(&validator, &json),
            serde_json::to_string_pretty(&json).unwrap(),
        );
    }
}
