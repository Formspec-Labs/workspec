// Rust guideline compliant 2026-04-18

//! Golden-trace regression tests for T3 conformance fixtures.
//!
//! Each test runs a T3 fixture from `crates/wos-conformance/fixtures/` via
//! `run_fixture_with_trace` and compares the result against the committed
//! golden baseline in `fixtures/conformance/expected-traces/<slug>.json`.
//!
//! When a spec change legitimately alters a trace (different state sequence,
//! new provenance shape, etc.), the commit MUST update both the implementation
//! and the golden file — the test failure is the signal that an update is due.
//!
//! AI-041-negative-fallback-cycle is excluded because it has no `kernel`
//! document; it is a lint-only fixture that the runtime engine cannot execute.

use wos_conformance::{ConformanceTrace, run_fixture_with_trace, slugify};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/wos-conformance; workspace root is ../../
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Run one T3 fixture and return its trace.
///
/// `base_dir` must be the workspace root because the T3 fixture set
/// references kernel documents via paths like `fixtures/kernel/…`.
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

/// Load the committed golden trace for a fixture.
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

/// Assert that actual trace steps match the golden baseline.
///
/// Timestamps, if any are ever added, would be excluded here. For now
/// all fields in TraceStep are deterministic and fully comparable.
fn assert_trace_matches(actual: &ConformanceTrace, fixture_id: &str) {
    let expected = load_golden(fixture_id);
    assert_eq!(
        actual.fixture_id, expected.fixture_id,
        "fixture_id mismatch for {fixture_id}"
    );
    assert_eq!(
        actual.outcome, expected.outcome,
        "outcome mismatch for {fixture_id}: actual={:?} expected={:?}",
        actual.outcome, expected.outcome
    );
    assert_eq!(
        actual.steps.len(),
        expected.steps.len(),
        "step count mismatch for {fixture_id}: actual={} expected={}",
        actual.steps.len(),
        expected.steps.len()
    );
    assert_eq!(
        actual.steps, expected.steps,
        "trace steps drifted on {fixture_id}"
    );
}

// ── Regression tests ─────────────────────────────────────────────────────────

/// K-001: Negative lint fixture — final state with outgoing transition.
///
/// Engine cannot execute; expected_errors assertion fails → outcome=fail, 0 steps.
#[test]
fn trace_parity_k001_negative_final_transitions() {
    let trace = run_t3_fixture("K-001-negative-final-transitions.json");
    assert_trace_matches(&trace, "K-001-negative-final-transitions");
}

/// K-011-determinism: single approve event.
///
/// Without initial_case_state.amount the guard evaluates to false → no
/// transition fires → outcome=fail, 0 steps (current engine behavior).
#[test]
fn trace_parity_k011_determinism() {
    let trace = run_t3_fixture("K-011-determinism.json");
    assert_trace_matches(&trace, "K-011-determinism");
}

/// K-011-parallel-join: benefits adjudication parallel regions.
#[test]
fn trace_parity_k011_parallel_join() {
    let trace = run_t3_fixture("K-011-parallel-join.json");
    assert_trace_matches(&trace, "K-011-parallel-join");
}

/// K-020-provenance-completeness: full happy path with provenance.
#[test]
fn trace_parity_k020_provenance_completeness() {
    let trace = run_t3_fixture("K-020-provenance-completeness.json");
    assert_trace_matches(&trace, "K-020-provenance-completeness");
}

/// K-033-document-order: first-match-wins guard evaluation.
#[test]
fn trace_parity_k033_document_order() {
    let trace = run_t3_fixture("K-033-document-order.json");
    assert_trace_matches(&trace, "K-033-document-order");
}

/// K-046-timer-provenance: timer lifecycle provenance records.
#[test]
fn trace_parity_k046_timer_provenance() {
    let trace = run_t3_fixture("K-046-timer-provenance.json");
    assert_trace_matches(&trace, "K-046-timer-provenance");
}

/// G-030-hold-resume: hold timer started and cancelled on resume.
#[test]
fn trace_parity_g030_hold_resume() {
    let trace = run_t3_fixture("G-030-hold-resume.json");
    assert_trace_matches(&trace, "G-030-hold-resume");
}
