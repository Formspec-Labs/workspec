// Rust guideline compliant 2026-02-21

//! Integration tests for the WOS conformance engine against real kernel documents.
//!
//! Each test loads a fixture from `tests/fixtures/`, runs it through `run_fixture`,
//! and asserts the result passes.  Fixtures exercise flat lifecycle, guard evaluation,
//! parallel dual-blind review, compound state, and timer-based transitions.

use wos_conformance::run_fixture;

// ── Helpers ──────────────────────────────────────────────────────

/// Resolve a fixture path relative to the manifest directory.
///
/// This ensures the test works regardless of the working directory.
fn fixture_path(name: &str) -> String {
    // CARGO_MANIFEST_DIR is set by Cargo during test compilation to the crate root.
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/tests/fixtures/{name}")
}

/// Read a fixture file and run it through the conformance engine.
///
/// Resolves document paths relative to the fixture file's directory.
/// On failure, prints the failure list and panics with a descriptive message.
fn assert_fixture_passes(fixture_filename: &str) {
    let path = fixture_path(fixture_filename);
    let fixture_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture '{path}': {e}"));

    // Resolve document paths relative to the directory containing the fixture.
    let base_dir = std::path::Path::new(&path)
        .parent()
        .expect("fixture path has no parent directory")
        .to_str()
        .expect("fixture directory is not valid UTF-8");

    let result = run_fixture(&fixture_json, base_dir)
        .unwrap_or_else(|e| panic!("fixture '{fixture_filename}' engine error: {e}"));

    if !result.passed {
        panic!(
            "fixture '{}' FAILED:\n{}",
            fixture_filename,
            result.failures.join("\n")
        );
    }
}

// ── Purchase Order Approval (flat lifecycle, guard evaluation) ───

/// Simple approval path: amount under $50k guard passes, order completes.
#[test]
fn purchase_order_simple_approval() {
    assert_fixture_passes("purchase-order-simple.json");
}

/// Amount over $50k routes to director approval via guard, then completes.
#[test]
fn purchase_order_director_approval() {
    assert_fixture_passes("purchase-order-director.json");
}

/// Manager rejects, requester resubmits, then approval completes normally.
#[test]
fn purchase_order_reject_then_resubmit() {
    assert_fixture_passes("purchase-order-reject-resubmit.json");
}

// ── Benefits Adjudication (parallel dual-blind review) ──────────

/// Both reviewers reach the same decision; $join fires directly to determination.
#[test]
fn benefits_parallel_reviewers_agree() {
    assert_fixture_passes("benefits-parallel-agree.json");
}

/// Reviewers disagree; $join fires to reconciliation before determination.
#[test]
fn benefits_parallel_reviewers_disagree() {
    assert_fixture_passes("benefits-parallel-disagree.json");
}

// ── Medicaid Redetermination (compound state, timers) ────────────

/// Complete intake through compound eligibilityReview to activeBenefits.
#[test]
fn medicaid_happy_path() {
    assert_fixture_passes("medicaid-happy-path.json");
}

/// Non-response timer fires after 30 days and routes to deniedForNonResponse.
#[test]
fn medicaid_timer_nonresponse() {
    assert_fixture_passes("medicaid-timer-nonresponse.json");
}

// ── Guard evaluation unit tests ──────────────────────────────────

/// Unmatched events are silently ignored (Kernel S4.9).
#[test]
fn unmatched_event_is_recorded_in_provenance_not_error() {
    let path = fixture_path("purchase-order-simple.json");
    let base_fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Inject an unmatched event before the normal sequence.
    let mut fixture = base_fixture.clone();
    let seq = fixture["event_sequence"].as_array_mut().unwrap();
    seq.insert(
        0,
        serde_json::json!({ "event": "completelySurprising", "actor": "unknown" }),
    );

    // Remove expected transitions — we only care it doesn't error.
    fixture["expected_transitions"] = serde_json::json!([]);

    let result = run_fixture(&serde_json::to_string(&fixture).unwrap(), &base_dir)
        .expect("run_fixture must not return an error for unmatched events");

    let unmatched_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::UnmatchedEvent)
        .count();

    assert!(
        unmatched_count >= 1,
        "expected at least one unmatchedEvent provenance record, got {unmatched_count}"
    );
}

/// `setData` actions on entry fire and produce caseStateMutation provenance records.
#[test]
fn set_data_produces_case_state_mutation_provenance() {
    // The purchase order `approved` state has two setData onEntry actions.
    let path = fixture_path("purchase-order-simple.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    let mutations: Vec<_> = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::CaseStateMutation)
        .collect();

    // The approved state onEntry has two setData actions: approvedBy and approvedAt.
    assert!(
        mutations.len() >= 2,
        "expected at least 2 caseStateMutation records, got {}",
        mutations.len()
    );
}

/// Timer provenance records are emitted when a timer is created.
#[test]
fn timer_created_provenance_on_entry() {
    let path = fixture_path("medicaid-timer-nonresponse.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    let timer_created = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerCreated)
        .count();

    assert!(
        timer_created >= 1,
        "expected at least 1 timerCreated provenance record, got {timer_created}"
    );

    let timer_fired = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::TimerFired)
        .count();

    assert!(
        timer_fired >= 1,
        "expected at least 1 timerFired provenance record, got {timer_fired}"
    );
}
