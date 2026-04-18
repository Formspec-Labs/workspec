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

/// K-011-determinism: single approve event with amount=3000 in case state.
/// Fires submitted → approved; guard `caseFile.amount <= 50000` passes.
#[test]
fn trace_parity_k011_determinism() {
    let trace = run_t3_fixture("K-011-determinism.json");
    assert_trace_matches(&trace, "K-011-determinism");
}

/// §5.3 teaching signal: when a guarded transition's guard evaluates false
/// and another guard fires instead, the trace's Delta must surface
/// `guardFalse` pointing at the blocked transition's guard_id + inputs —
/// not a bare `stateMismatch` — so an LLM can learn which guard it needs
/// to repair.
///
/// K-PO-004 (inline fixture below) expects submitted → approved on
/// `decide`, but amount=75000 sends it to pendingDirectorApproval via the
/// other guard. The expected `approved` transition's guard
/// `caseFile.amount <= 50000` evaluates false; that's the teachable moment.
#[test]
fn guard_false_delta_surfaces_blocking_guard_id() {
    use wos_conformance::{run_fixture_with_trace, Delta};
    let kernel_path = workspace_root()
        .join("fixtures/kernel/purchase-order-approval.json")
        .canonicalize()
        .unwrap();
    let fixture_json = serde_json::json!({
        "id": "K-GUARD-FALSE-DELTA",
        "rule": "K-011",
        "description": "Mismatched expected path triggers GuardFalse delta",
        "documents": { "kernel": kernel_path.to_str().unwrap() },
        "initial_case_state": {
            "amount": 75000,
            "orderId": "PO-BIG",
            "vendor": "Acme"
        },
        "event_sequence": [
            { "event": "approve", "actor": "approver", "data": {} }
        ],
        "expected_transitions": [
            { "from": "submitted", "to": "approved", "event": "approve" }
        ],
        "expected_provenance": [],
        "expected_errors": []
    })
    .to_string();

    let workspace = workspace_root();
    let (_result, trace) =
        run_fixture_with_trace(&fixture_json, workspace.to_str().unwrap()).unwrap();

    let step = trace
        .steps
        .first()
        .expect("at least one transition observed");
    match &step.delta {
        Some(Delta::GuardFalse { guard_id, inputs }) => {
            assert_eq!(guard_id, "submitted->approved:approve");
            assert_eq!(
                inputs,
                &serde_json::json!({ "caseFile": { "amount": 75000 } })
            );
        }
        other => panic!("expected Delta::GuardFalse, got {:?}", other),
    }
}

/// §5.3 teaching signal: guards_evaluated must populate on fixtures that
/// actually exercise guards. K-011-determinism fires through the purchase-
/// order guard `caseFile.amount <= 50000`; the first trace step's
/// guardsEvaluated must surface that evaluation with result=true and
/// inputs subset to `{ caseFile: { amount: 3000 } }`.
///
/// Without this, §5.4's repair prompt has no per-step teaching payload —
/// the trace records only "this transition fired" instead of "this guard
/// evaluated true against these inputs and that's why it fired".
#[test]
fn teaching_signal_populates_guards_evaluated_on_k011_determinism() {
    let trace = run_t3_fixture("K-011-determinism.json");
    assert!(!trace.steps.is_empty(), "trace must have at least one step");
    let step_zero = &trace.steps[0];
    assert!(
        !step_zero.guards_evaluated.is_empty(),
        "step 0 must carry guard evaluations; trace is the teaching signal"
    );
    let passed_guard = step_zero
        .guards_evaluated
        .iter()
        .find(|g| g.target_state == "approved")
        .expect("approved-target guard must be recorded");
    assert!(passed_guard.result, "approved-target guard evaluated true");
    assert_eq!(
        passed_guard.inputs,
        serde_json::json!({ "caseFile": { "amount": 3000 } }),
        "inputs must subset case state to referenced paths"
    );
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

// ── Honest-behavior assertions ──────────────────────────────────────────────
//
// The parity tests above compare against committed goldens; they would pass
// even if every fixture produced `{outcome: fail, steps: []}` because broken
// state equals broken state. These tests pin the stronger property: each
// happy-path T3 fixture must engage the runtime, pass its own assertions,
// and emit exactly the number of steps the fixture declares under
// `expected_transitions`. A fixture that fires zero steps means the
// guard-data path is broken — the bug the 2026-04-18 review flagged and
// that this suite now protects against.

use wos_conformance::{ConformanceFixture, Outcome};

fn read_fixture_json(fixture_filename: &str) -> String {
    let workspace = workspace_root();
    let fixture_path = workspace
        .join("crates/wos-conformance/fixtures")
        .join(fixture_filename);
    std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("could not read fixture '{fixture_filename}': {e}"))
}

fn assert_fixture_engages_runtime(fixture_filename: &str) {
    let json = read_fixture_json(fixture_filename);
    let fixture: ConformanceFixture = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("fixture parse error '{fixture_filename}': {e}"));
    let trace = run_t3_fixture(fixture_filename);

    let expected_count = fixture.expected_transitions.len();
    assert!(
        expected_count > 0,
        "fixture '{fixture_filename}' declares no expected_transitions; not a happy-path fixture"
    );
    assert_eq!(
        trace.steps.len(),
        expected_count,
        "fixture '{fixture_filename}' produced {} steps; expected {} \
         (guard data-path or expected_transitions drift)",
        trace.steps.len(),
        expected_count
    );
    assert_eq!(
        trace.outcome,
        Outcome::Pass,
        "fixture '{fixture_filename}' did not pass; steps were {:?}",
        trace.steps
    );
}

#[test]
fn happy_path_k011_determinism_fires_its_transition() {
    assert_fixture_engages_runtime("K-011-determinism.json");
}

#[test]
fn happy_path_k011_parallel_join_fires_all_transitions() {
    assert_fixture_engages_runtime("K-011-parallel-join.json");
}

#[test]
fn happy_path_k020_provenance_fires_its_transitions() {
    assert_fixture_engages_runtime("K-020-provenance-completeness.json");
}

#[test]
fn happy_path_k033_document_order_fires_first_match() {
    assert_fixture_engages_runtime("K-033-document-order.json");
}

#[test]
fn happy_path_k046_timer_provenance_fires_its_transitions() {
    assert_fixture_engages_runtime("K-046-timer-provenance.json");
}

#[test]
fn happy_path_g030_hold_resume_fires_its_transitions() {
    assert_fixture_engages_runtime("G-030-hold-resume.json");
}
