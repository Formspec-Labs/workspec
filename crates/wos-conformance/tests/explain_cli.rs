// Rust guideline compliant 2026-04-18

//! Integration tests for the `wos-conformance-explain` and `wos-conformance-diff`
//! CLI binaries.
//!
//! Tests invoke the library functions directly rather than spawning subprocesses
//! so they run fast and remain hermetic. The binary entry points are thin wrappers
//! around the same library calls tested here.

use wos_conformance::{
    diff_traces, render_diff, render_trace, run_fixture_with_trace, slugify, ConformanceTrace,
    DivergenceCause, Outcome, TraceDiffResult,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_golden(fixture_id: &str) -> ConformanceTrace {
    let slug = slugify(fixture_id);
    let path = workspace_root()
        .join("fixtures/conformance/expected-traces")
        .join(format!("{slug}.json"));
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden trace missing for '{fixture_id}': {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("golden trace parse error for '{fixture_id}': {e}"))
}

fn run_t3_fixture(fixture_filename: &str) -> ConformanceTrace {
    let workspace = workspace_root();
    let fixture_path = workspace
        .join("crates/wos-conformance/fixtures")
        .join(fixture_filename);
    let fixture_json = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("could not read fixture '{fixture_filename}': {e}"));
    let base_dir = workspace.to_str().unwrap();
    let (_result, trace) = run_fixture_with_trace(&fixture_json, base_dir)
        .unwrap_or_else(|e| panic!("engine error on '{fixture_filename}': {e}"));
    trace
}

// ── explain render_trace tests ────────────────────────────────────────────────

/// Happy path: K-011-determinism passes; render_trace includes the fixture name,
/// the transition (submitted → approved), the guard id, and "PASS" in the summary.
#[test]
fn explain_passing_fixture_shows_transition_guard_and_pass_summary() {
    let trace = run_t3_fixture("K-011-determinism.json");
    let output = render_trace(&trace);

    assert!(
        output.contains("K-011-determinism"),
        "fixture id missing: {output}"
    );
    assert!(
        output.contains("submitted"),
        "source state missing: {output}"
    );
    assert!(
        output.contains("approved"),
        "target state missing: {output}"
    );
    // Guard id should appear since K-011 fires through the guard.
    assert!(
        output.contains("submitted->approved:approve"),
        "guard_id missing: {output}"
    );
    // Guard expression should appear.
    assert!(
        output.contains("caseFile.amount <= 50000"),
        "guard expression missing: {output}"
    );
    assert!(output.contains("PASS"), "summary PASS missing: {output}");
    assert!(!output.contains("FAIL"), "no FAIL on passing fixture: {output}");
}

/// Fail path: K-001 is a lint-negative fixture (0 transitions, outcome=fail).
/// render_trace must surface "(no transitions recorded)" and a FAIL summary
/// with no divergence step (lint failures have no step-level divergence).
#[test]
fn explain_failing_lint_fixture_shows_no_transitions_and_fail_summary() {
    let trace = run_t3_fixture("K-001-negative-final-transitions.json");
    let output = render_trace(&trace);

    assert!(
        output.contains("(no transitions recorded)"),
        "empty-trace message missing: {output}"
    );
    assert!(output.contains("FAIL"), "FAIL missing: {output}");
    // Lint failures have no step-level divergence to report.
    assert!(
        !output.contains("first divergence at step"),
        "unexpected step divergence on lint fixture: {output}"
    );
}

/// render_trace for a multi-step fixture surfaces all step lines.
#[test]
fn explain_multi_step_fixture_shows_all_steps() {
    let trace = run_t3_fixture("K-020-provenance-completeness.json");
    let output = render_trace(&trace);

    assert!(output.contains("step 1:"), "step 1 missing: {output}");
    assert!(output.contains("step 2:"), "step 2 missing: {output}");
    assert!(output.contains("PASS"), "PASS missing: {output}");
    assert!(output.contains("2 steps"), "step count missing: {output}");
}

/// render_trace --json flag path: the trace JSON round-trips from the golden.
#[test]
fn explain_json_flag_round_trips_golden_trace() {
    let golden = load_golden("K-011-determinism");
    let json = serde_json::to_string_pretty(&golden).expect("serialize");
    let reparsed: ConformanceTrace = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(golden, reparsed);
    // Also verify the fixture id is present in the JSON string.
    assert!(json.contains("K-011-determinism"), "fixture id missing in JSON: {json}");
}

// ── diff_traces + render_diff tests ───────────────────────────────────────────

/// Diff match: golden trace vs freshly-run fixture → OK.
#[test]
fn diff_match_golden_vs_fresh_k011_determinism_returns_ok() {
    let golden = load_golden("K-011-determinism");
    let fresh = run_t3_fixture("K-011-determinism.json");
    let result = diff_traces(&golden, &fresh);
    assert_eq!(
        result,
        TraceDiffResult::Match,
        "expected Match; got: {result:?}"
    );
    let rendered = render_diff(&result);
    assert_eq!(rendered, "OK\n", "render_diff on Match must print OK");
}

/// Diff match renders "OK" for every T3 golden trace.
#[test]
fn diff_match_all_golden_traces_match_fresh_runs() {
    let fixtures = [
        ("K-011-determinism", "K-011-determinism.json"),
        ("K-020-provenance-completeness", "K-020-provenance-completeness.json"),
        ("K-033-document-order", "K-033-document-order.json"),
    ];
    for (fixture_id, fixture_file) in fixtures {
        let golden = load_golden(fixture_id);
        let fresh = run_t3_fixture(fixture_file);
        let result = diff_traces(&golden, &fresh);
        assert_eq!(
            result,
            TraceDiffResult::Match,
            "diff mismatch for '{fixture_id}'"
        );
    }
}

/// Diff mismatch: mutate one step's state_after → non-zero divergence with cause.
#[test]
fn diff_mismatch_mutated_step_surfaces_divergence_and_exits_nonzero() {
    let mut expected = load_golden("K-011-determinism");
    // Mutate expected so it claims the step should land in "rejected" instead of "approved".
    expected.steps[0].state_after = "rejected".to_string();
    if let Some(ref mut exp) = expected.steps[0].expected_state_after {
        *exp = "rejected".to_string();
    }

    let actual = run_t3_fixture("K-011-determinism.json");
    let result = diff_traces(&expected, &actual);

    assert!(
        matches!(result, TraceDiffResult::Divergence(_)),
        "expected Divergence, got Match"
    );

    let rendered = render_diff(&result);
    assert!(rendered.contains("DIVERGENCE"), "DIVERGENCE missing: {rendered}");
    assert!(rendered.contains("at step: 1"), "step number missing: {rendered}");
    // expected state in the mutated golden is "rejected"; actual from runner is "approved"
    assert!(rendered.contains("rejected"), "expected state missing: {rendered}");
    assert!(rendered.contains("approved"), "actual state missing: {rendered}");
    assert!(rendered.contains("hypothesis:"), "hypothesis missing: {rendered}");

    // Verify the exit code semantic: Divergence → exit 1.
    let exit_code: u8 = match &result {
        TraceDiffResult::Match => 0,
        TraceDiffResult::Divergence(_) => 1,
    };
    assert_eq!(exit_code, 1, "divergence must map to exit code 1");
}

/// Diff --json flag: divergence JSON includes the expected cause fields.
#[test]
fn diff_json_flag_divergence_includes_cause_fields() {
    let mut expected = load_golden("K-011-determinism");
    expected.steps[0].state_after = "rejected".to_string();
    if let Some(ref mut exp) = expected.steps[0].expected_state_after {
        *exp = "rejected".to_string();
    }

    let actual = run_t3_fixture("K-011-determinism.json");
    let result = diff_traces(&expected, &actual);

    let json = serde_json::to_string_pretty(&result).expect("serialize diff result");
    // JSON should contain divergence key and differsAtStep.
    assert!(
        json.contains("Divergence") || json.contains("divergence") || json.contains("differsAtStep"),
        "diff JSON missing divergence content: {json}"
    );
}

/// Step-count mismatch: add a phantom step to expected → StepCountMismatch cause.
#[test]
fn diff_step_count_mismatch_surfaces_step_count_cause() {
    let mut expected = load_golden("K-011-determinism");
    // Push a phantom extra step.
    let phantom = expected.steps[0].clone();
    expected.steps.push(phantom);

    let actual = run_t3_fixture("K-011-determinism.json");
    let result = diff_traces(&expected, &actual);

    match result {
        TraceDiffResult::Divergence(div) => {
            assert!(
                matches!(div.cause, Some(DivergenceCause::StepCountMismatch { .. })),
                "expected StepCountMismatch cause: {:?}",
                div.cause
            );
        }
        TraceDiffResult::Match => panic!("expected Divergence"),
    }
}

/// Outcome mismatch: flip expected outcome to Error → OutcomeMismatch cause.
#[test]
fn diff_outcome_mismatch_surfaces_outcome_mismatch_cause() {
    let mut expected = load_golden("K-011-determinism");
    expected.outcome = Outcome::Error;

    let actual = run_t3_fixture("K-011-determinism.json");
    let result = diff_traces(&expected, &actual);

    match result {
        TraceDiffResult::Divergence(div) => {
            assert!(
                matches!(div.cause, Some(DivergenceCause::OutcomeMismatch { .. })),
                "expected OutcomeMismatch cause: {:?}",
                div.cause
            );
        }
        TraceDiffResult::Match => panic!("expected Divergence on outcome flip"),
    }
}
