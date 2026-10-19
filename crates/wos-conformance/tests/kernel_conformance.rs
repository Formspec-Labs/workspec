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

/// Retired external policy/hint fixtures must refuse, never count as implementation proof.
fn assert_retired_fixture_refused(name: &str) {
    let original_base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = std::fs::read_to_string(original_base.join("historical-ai").join(name)).unwrap();
    let error = run_fixture(&source, original_base.to_str().unwrap()).unwrap_err();
    assert!(
        error.to_string().contains("substituted legacy AI document"),
        "{name}: {error}"
    );
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

// ── T3 conformance fixtures (Phase 4) ───────────────────────────
//
// These fixture-backed tests run as part of the normal conformance suite.

// ── Batch 1: Cancel-siblings / fail-fast ────────────────────────

/// K-044: Timer events routed to creating region only (LCD S6.5).
#[test]
fn k044_timer_region_scoping() {
    assert_fixture_passes("k-044-timer-region-scoping.json");
}

/// K-045: Firing a timer far past its tolerance window is a conformance violation (LCD S6.6).
#[test]
fn k045_timer_tolerance_violation() {
    assert_fixture_passes("k-045-timer-tolerance-violation.json");
}

// ── Batch 2: Hold/resume lifecycle ──────────────────────────────

/// G-030: Entering a hold-tagged state starts the hold timer (WG S12.4).
#[test]
fn g030_hold_timer_start() {
    assert_fixture_passes("g-030-hold-timer-start.json");
}

/// G-054: Resume trigger cancels the hold timer before it fires (WG S12.4).
#[test]
fn g054_resume_cancels_hold_timer() {
    assert_fixture_passes("g-054-resume-cancels-hold-timer.json");
}

// ── Batch 3: Deontic enforcement ────────────────────────────────

/// AI-009: Permission bounds evaluated against live agent output (AI S4.2).
#[test]
fn ai009_permission_bounds() {
    assert_fixture_passes("ai-009-permission-bounds.json");
}

/// AI-010: Prohibition condition evaluated against live output (AI S4.3).
#[test]
fn ai010_prohibition_condition() {
    assert_fixture_passes("ai-010-prohibition-condition.json");
}

/// AI-011: Obligation requirement evaluated against live output (AI S4.4).
#[test]
fn ai011_obligation_requirement() {
    assert_fixture_passes("ai-011-obligation-requirement.json");
}

/// AI-012: Rights violation not attributed to agent (AI S4.5).
#[test]
fn ai012_rights_violation_not_attributed() {
    assert_fixture_passes("ai-012-rights-violation-not-attributed.json");
}

/// AI-013: Deontic evaluation order: permissions, prohibitions, obligations, confidence, volume, sampling (AI S4.6).
#[test]
fn ai013_evaluation_order() {
    assert_fixture_passes("ai-013-evaluation-order.json");
}

/// AI-014: Most restrictive enforcement action wins (AI S4.6).
#[test]
fn ai014_most_restrictive_wins() {
    assert_fixture_passes("ai-014-most-restrictive-wins.json");
}

/// AI-015: All constraints at all three composition levels evaluated (AI S4.7).
#[test]
fn ai015_multi_level_evaluation() {
    assert_fixture_passes("ai-015-multi-level-evaluation.json");
}

/// AI-016: Same-level most-restrictive resolution — reject wins over escalateToHuman (AI S4.6).
#[test]
fn ai016_cross_level_most_restrictive() {
    assert_fixture_passes("ai-016-cross-level-most-restrictive.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai017_null_escalation_retired_input_refused() {
    assert_retired_fixture_refused("ai-017-null-escalation.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai027_escalation_deontic_not_bypassed_retired_input_refused() {
    assert_retired_fixture_refused("ai-027-escalation-deontic-not-bypassed.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai051_proxy_deontic_retired_input_refused() {
    assert_retired_fixture_refused("ai-051-proxy-deontic.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai054_bypass_single_invocation_retired_input_refused() {
    assert_retired_fixture_refused("ai-054-bypass-single-invocation.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai055_consistency_contradiction_retired_input_refused() {
    assert_retired_fixture_refused("ai-055-consistency-contradiction.json");
}

// ── Batch 4: Autonomy caps ──────────────────────────────────────

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai005_no_override_human_retired_input_refused() {
    assert_retired_fixture_refused("ai-005-no-override-human.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai019_assistive_creates_human_task_retired_input_refused() {
    assert_retired_fixture_refused("ai-019-assistive-creates-human-task.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai021_impact_level_cap_retired_input_refused() {
    assert_retired_fixture_refused("ai-021-impact-level-cap.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai022_effective_autonomy_minimum_retired_input_refused() {
    assert_retired_fixture_refused("ai-022-effective-autonomy-minimum.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai025_escalation_requires_approval_retired_input_refused() {
    assert_retired_fixture_refused("ai-025-escalation-requires-approval.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai028_demotion_next_invocation_retired_input_refused() {
    assert_retired_fixture_refused("ai-028-demotion-next-invocation.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai029_pending_recalibration_retired_input_refused() {
    assert_retired_fixture_refused("ai-029-pending-recalibration.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai030_dynamic_autonomy_cap_retired_input_refused() {
    assert_retired_fixture_refused("ai-030-dynamic-autonomy-cap.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ac001_expired_calibration_cap_retired_input_refused() {
    assert_retired_fixture_refused("ac-001-expired-calibration-cap.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ac002_max_autonomy_minimum_retired_input_refused() {
    assert_retired_fixture_refused("ac-002-max-autonomy-minimum.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag005_tool_not_permitted_retired_input_refused() {
    assert_retired_fixture_refused("ag-005-tool-not-permitted.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag006_no_direct_case_write_retired_input_refused() {
    assert_retired_fixture_refused("ag-006-no-direct-case-write.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag007_tool_rate_limit_retired_input_refused() {
    assert_retired_fixture_refused("ag-007-tool-rate-limit.json");
}

// ── Batch 5: Confidence framework ───────────────────────────────

/// AI-034: Every agent output MUST have a ConfidenceReport (AI S7.1).
#[test]
fn ai034_confidence_report_required() {
    assert_fixture_passes("ai-034-confidence-report-required.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai035_calibrated_confidence_retired_input_refused() {
    assert_retired_fixture_refused("ai-035-calibrated-confidence.json");
}

/// AI-036: Confidence below floor invalidates output (AI S7.4).
#[test]
fn ai036_confidence_below_floor() {
    assert_fixture_passes("ai-036-confidence-below-floor.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai037_decay_trigger_retired_input_refused() {
    assert_retired_fixture_refused("ai-037-decay-trigger.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai038_cumulative_confidence_pause_retired_input_refused() {
    assert_retired_fixture_refused("ai-038-cumulative-confidence-pause.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag004_session_pause_retired_input_refused() {
    assert_retired_fixture_refused("ag-004-session-pause.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag016_review_ground_truth_retired_input_refused() {
    assert_retired_fixture_refused("ag-016-review-ground-truth.json");
}

// ── Batch 6: Due process runtime ────────────────────────────────

/// G-002: Notice before adverse decision takes effect (WG S3.2).
#[test]
fn g002_notice_before_adverse() {
    assert_fixture_passes("g-002-notice-before-adverse.json");
}

/// G-006: Appeal reviewed by independent adjudicator (WG S3.5).
#[test]
fn g006_appeal_independent_reviewer() {
    assert_fixture_passes("g-006-appeal-independent-reviewer.json");
}

/// G-007: Appeal filing produces provenance record (WG S3.5).
#[test]
fn g007_appeal_provenance() {
    assert_fixture_passes("g-007-appeal-provenance.json");
}

/// G-010: independentFirst enforces recording before recommendation visible (WG S4.2).
#[test]
fn g010_independent_first() {
    assert_fixture_passes("g-010-independent-first.json");
}

/// G-016: Configurable percentage randomly selected for quality review (WG S7.1).
#[test]
fn g016_review_sampling() {
    assert_fixture_passes("g-016-review-sampling.json");
}

/// G-017: Reviewer MUST NOT be original decision-maker (WG S7.2).
#[test]
fn g017_reviewer_separation() {
    assert_fixture_passes("g-017-reviewer-separation.json");
}

/// G-018: Override requires structured rationale, authority, evidence (WG S7.3).
#[test]
fn g018_override_rationale() {
    assert_fixture_passes("g-018-override-rationale.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai045_independent_first_suppression_retired_input_refused() {
    assert_retired_fixture_refused("ai-045-independent-first-suppression.json");
}

// ── Batch 7: Pipeline execution ─────────────────────────────────

/// G-012: Pipeline stage records inputs, outputs, gate results in provenance (WG S5.5).
#[test]
fn g012_pipeline_stage_provenance() {
    assert_fixture_passes("g-012-pipeline-stage-provenance.json");
}

/// G-013: Pipeline risk profile determined by weakest gate (WG S5.5).
#[test]
fn g013_weakest_link_risk() {
    assert_fixture_passes("g-013-weakest-link-risk.json");
}

/// G-019: Override records are immutable provenance entries (WG S7.3).
#[test]
fn g019_override_immutable() {
    assert_fixture_passes("g-019-override-immutable.json");
}

/// G-020: Rejection records gate, input, threshold, what would pass (WG S8.2).
#[test]
fn g020_rejection_detail() {
    assert_fixture_passes("g-020-rejection-detail.json");
}

/// G-021: All task state transitions recorded in provenance (WG S10.1).
#[test]
fn g021_task_provenance() {
    assert_fixture_passes("g-021-task-provenance.json");
}

/// G-032: Temporal resolution selects most recent entry before resolution date (WG S13.2).
#[test]
fn g032_temporal_resolution() {
    assert_fixture_passes("g-032-temporal-resolution.json");
}

/// G-049: Processor MUST NOT alter resolution based on bindingType (PP S1.5.4).
#[test]
fn g049_binding_type_neutral() {
    assert_fixture_passes("g-049-binding-type-neutral.json");
}

// ── Batch 8: Compensation ───────────────────────────────────────

/// K-027: Compensation log is append-only (Kernel S9.5, LCD S5.2).
#[test]
fn k027_compensation_log_append_only() {
    assert_fixture_passes("k-027-compensation-log-append-only.json");
}

/// K-039: Compensation in reverse of forward completion order (LCD S5.4).
#[test]
fn k039_compensation_reverse_order() {
    assert_fixture_passes("k-039-compensation-reverse-order.json");
}

/// K-040: Pivot step excluded from compensation (LCD S5.5).
#[test]
fn k040_pivot_no_compensation() {
    assert_fixture_passes("k-040-pivot-no-compensation.json");
}

/// K-041: Inner scope compensation does not trigger outer (LCD S5.8).
#[test]
fn k041_inner_scope_boundary() {
    assert_fixture_passes("k-041-inner-scope-boundary.json");
}

/// K-042: $compensation.complete event processed like any event (LCD S5.9).
#[test]
fn k042_compensation_complete_event() {
    assert_fixture_passes("k-042-compensation-complete-event.json");
}

// ── Batch 9: Delegation runtime ─────────────────────────────────

/// G-025: Determinations without valid delegation are conformance errors (WG S11.4).
#[test]
fn g025_delegation_required() {
    assert_fixture_passes("g-025-delegation-required.json");
}

/// G-026: Delegation used referenced in provenance record (WG S11.4).
#[test]
fn g026_delegation_in_provenance() {
    assert_fixture_passes("g-026-delegation-in-provenance.json");
}

// ── Batch 10: Agent provenance + fallback ───────────────────────

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai006_agent_provenance_fields_retired_input_refused() {
    assert_retired_fixture_refused("ai-006-agent-provenance-fields.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai008_actor_type_immutable_retired_input_refused() {
    assert_retired_fixture_refused("ai-008-actor-type-immutable.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai033_agent_touched_annotation_retired_input_refused() {
    assert_retired_fixture_refused("ai-033-agent-touched-annotation.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai044_drift_reclassification_retired_input_refused() {
    assert_retired_fixture_refused("ai-044-drift-reclassification.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai047_narrative_non_authoritative_retired_input_refused() {
    assert_retired_fixture_refused("ai-047-narrative-non-authoritative.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai052_proxy_provenance_retired_input_refused() {
    assert_retired_fixture_refused("ai-052-proxy-provenance.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai053_version_change_provenance_retired_input_refused() {
    assert_retired_fixture_refused("ai-053-version-change-provenance.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ag009_agent_state_provenance_retired_input_refused() {
    assert_retired_fixture_refused("ag-009-agent-state-provenance.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai057_processor_enforces_constraints_retired_input_refused() {
    assert_retired_fixture_refused("ai-057-processor-enforces-constraints.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai032_validation_triggers_fallback_retired_input_refused() {
    assert_retired_fixture_refused("ai-032-validation-triggers-fallback.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai039_fallback_provenance_retired_input_refused() {
    assert_retired_fixture_refused("ai-039-fallback-provenance.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai040_terminal_fallback_retired_input_refused() {
    assert_retired_fixture_refused("ai-040-terminal-fallback.json");
}

// ── Batch 11: Crash recovery / durability ────────────────────────

/// K-023: Non-terminal instances resume after crash (Kernel S9.1, G1).
#[test]
fn k023_crash_recovery() {
    assert_fixture_passes("k-023-crash-recovery.json");
}

/// K-024: Non-deterministic output persisted before advancing state (Kernel S9.1, G3).
#[test]
fn k024_persist_before_advance() {
    assert_fixture_passes("k-024-persist-before-advance.json");
}

/// K-026: IdempotencyKey deduplicates invocations (Kernel S9.3).
#[test]
fn k026_idempotency_dedup() {
    assert_fixture_passes("k-026-idempotency-dedup.json");
}

/// K-028: Instance migration produces provenance (Kernel S9.6).
#[test]
fn k028_migration_provenance() {
    assert_fixture_passes("k-028-migration-provenance.json");
}

/// K-031: Contract validation produces structured results (Kernel S11.1).
#[test]
fn k031_contract_structured_results() {
    assert_fixture_passes("k-031-contract-structured-results.json");
}

/// K-032: Lifecycle state separated from case state (Kernel S12).
#[test]
fn k032_lifecycle_case_separation() {
    assert_fixture_passes("k-032-lifecycle-case-separation.json");
}

/// K-035: History cleared on parent exit or region cancellation (LCD S3.4).
#[test]
fn k035_history_cleared_on_exit() {
    assert_fixture_passes("k-035-history-cleared-on-exit.json");
}

// ── Batch 12: DCR marking state ─────────────────────────────────

/// AG-001: Equity guardrails do not block individual actions (AdvGov S3.3).
#[test]
fn ag001_equity_no_block_individual() {
    assert_fixture_passes("ag-001-equity-no-block-individual.json");
}

/// AG-002: Excluding a pending activity raises resolution error (AdvGov S4.4).
#[test]
fn ag002_exclude_pending_error() {
    assert_fixture_passes("ag-002-exclude-pending-error.json");
}

/// AG-003: Zone satisfied when all pending executed (AdvGov S4.5).
#[test]
fn ag003_zone_satisfaction() {
    assert_fixture_passes("ag-003-zone-satisfaction.json");
}

// ── Batch 13: Provenance completeness ───────────────────────────

/// K-018: Case relationship changes produce provenance (Kernel S5.5).
#[test]
fn k018_relationship_change_provenance() {
    assert_fixture_passes("k-018-relationship-change-provenance.json");
}

/// Historical external-policy input refuses; the original operation remains unsupported.
#[test]
fn ai048_narrative_not_dispositive_retired_input_refused() {
    assert_retired_fixture_refused("ai-048-narrative-not-dispositive.json");
}

// ── Batch 14: Verification reports ──────────────────────────────

/// VR-001: Verification report is immutable once produced (VerifReport S1).
#[test]
fn vr001_report_immutable() {
    assert_fixture_passes("vr-001-report-immutable.json");
}

/// VR-002: Proven-unsafe prevents workflow activation (VerifReport S1).
#[test]
fn vr002_proven_unsafe_blocks_activation() {
    assert_fixture_passes("vr-002-proven-unsafe-blocks-activation.json");
}

/// AG-015: Proven-unsafe constraint blocks workflow activation (AdvGov S8.3).
#[test]
fn ag015_proven_unsafe_blocks_active() {
    assert_fixture_passes("ag-015-proven-unsafe-blocks-active.json");
}

// ── Batch 15: Sidecar runtime ───────────────────────────────────

/// G-061: Processor ignores expired calendar, falls back to wall-clock (BC S8.1).
#[test]
fn g061_expired_calendar_ignored() {
    assert_fixture_passes("g-061-expired-calendar-ignored.json");
}

/// G-064: Processor does not send notification when required variables missing (NT S5.3).
#[test]
fn g064_notification_missing_variables() {
    assert_fixture_passes("g-064-notification-missing-variables.json");
}

// ── Guard evaluation unit tests ──────────────────────────────────

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

// ── Batch 17: Edge case behavioral rules ────────────────────────

/// K-016: Every field mutation MUST produce a CaseStateMutation record.
#[test]
fn k016_mutation_history() {
    assert_fixture_passes("k-016-mutation-history.json");
}

/// K-025: Timers MUST preserve original duration and deadline metadata.
#[test]
fn k025_timer_metadata_preservation() {
    assert_fixture_passes("k-025-timer-metadata-preservation.json");
}

/// K-034: Compound state entry MUST enter initialState recursively.
#[test]
fn k034_compound_entry_logic() {
    assert_fixture_passes("k-034-compound-entry-logic.json");
}

/// K-036: Parallel state entry MUST initialize all regions atomically.
#[test]
fn k036_parallel_initialization() {
    assert_fixture_passes("k-036-parallel-initialization.json");
}

/// K-038: When a region is cancelled, all timers created within that region MUST be cancelled.
#[test]
fn k038_cancelled_region_timers() {
    assert_fixture_passes("k-038-cancelled-region-timers.json");
}

/// K-043: When a state is re-entered, existing timer MUST be cancelled and recreated.
#[test]
fn k043_reentered_timer_reset() {
    assert_fixture_passes("k-043-reentered-timer-reset.json");
}

/// K-047: Case relationships MUST NOT affect lifecycle evaluation.
#[test]
fn k047_relationship_isolation() {
    assert_fixture_passes("k-047-relationship-isolation.json");
}

/// K-DET-001: Determination transitions carry the pre-transition case-file snapshot.
#[test]
fn kdet001_determination_snapshot() {
    assert_fixture_passes("k-det-001-determination-snapshot.json");
}

// ── Fixture parse validation ────────────────────────────────────

/// Validate that all fixture JSON files parse as `ConformanceFixture` and that
/// their `documents` paths resolve to existing files.
///
/// This catches structural issues early without running fixtures through the
/// engine (which requires Phase 5 capabilities for most rules).
#[test]
fn all_fixtures_parse_and_resolve() {
    let fixtures_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(&fixtures_dir).expect("fixtures dir exists") {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        // Only loose `*.json` files at the fixtures root are standard
        // `ConformanceFixture` entries. The `export/` subdirectory holds
        // export-conformance fixtures with a distinct envelope — they are
        // owned by `tests/export_conformance.rs` and must not be parsed
        // as `ConformanceFixture`. `read_dir` does not recurse, so the
        // `.json` extension guard below excludes the subdirectory itself.
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let fixture: wos_conformance::ConformanceFixture =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        // Verify document paths resolve relative to the fixture directory.
        let base = path.parent().unwrap().to_str().unwrap();
        for (role, doc_path) in &fixture.documents {
            if doc_path == "inline" {
                assert!(
                    fixture.inline_documents.contains_key(role),
                    "fixture {} declares inline {role} document but omits inline_documents.{role}",
                    path.display()
                );
                continue;
            }
            let full = format!("{base}/{doc_path}");
            assert!(
                std::path::Path::new(&full).exists(),
                "fixture {} references non-existent {role} document: {doc_path}",
                path.display()
            );
        }
    }
}

// ── S15: FormspecBinding task lifecycle ──────────────────────────

/// S15-001: Task creation through real FormspecBinding produces provenance.
#[test]
fn s15_001_task_draft_prefill() {
    assert_fixture_passes("S15-001-task-draft-prefill.json");
}

/// S15-002: Happy-path submit through real FormspecBinding.
#[test]
fn s15_002_submit_valid() {
    assert_fixture_passes("S15-002-submit-valid.json");
}

/// S15-003: Missing envelope field causes validation failure.
#[test]
fn s15_003_submit_missing_envelope_field() {
    assert_fixture_passes("S15-003-submit-missing-envelope-field.json");
}

/// S15-004: Pin mismatch causes validation failure.
#[test]
fn s15_004_pin_mismatch() {
    assert_fixture_passes("S15-004-pin-mismatch.json");
}

/// S15-005: Definition validation failure via canned errors.
#[test]
fn s15_005_submit_definition_invalid() {
    assert_fixture_passes("S15-005-submit-definition-invalid.json");
}

/// S15-006: Response mapping applies field_updates to case state.
#[test]
fn s15_006_response_mapping() {
    assert_fixture_passes("S15-006-response-mapping.json");
}

// ── History state (KS.1) ────────────────────────────────────────

/// K-H-D1: Shallow history re-entry at depth 1.
#[test]
fn kh_d1_shallow_normal_reentry() {
    assert_fixture_passes("K-H-D1-shallow-normal-reentry.json");
}

/// K-H-D1: Deep history re-entry at depth 1.
#[test]
fn kh_d1_deep_normal_reentry() {
    assert_fixture_passes("K-H-D1-deep-normal-reentry.json");
}

/// K-H-D2: Shallow history re-entry at depth 2 — only direct substate restored.
#[test]
fn kh_d2_shallow_normal_reentry() {
    assert_fixture_passes("K-H-D2-shallow-normal-reentry.json");
}

/// K-H-D2: Deep history re-entry at depth 2 — full nested config restored.
#[test]
fn kh_d2_deep_normal_reentry() {
    assert_fixture_passes("K-H-D2-deep-normal-reentry.json");
}

/// K-H-D2: Deep history capture after parallel-region exit.
#[test]
fn kh_d2_deep_after_parallel_exit() {
    assert_fixture_passes("K-H-D2-deep-after-parallel-exit.json");
}

/// K-H-D2: Shallow history capture after parallel-region exit.
#[test]
fn kh_d2_shallow_after_parallel_exit() {
    assert_fixture_passes("K-H-D2-shallow-after-parallel-exit.json");
}

/// K-H-D3: Deep history re-entry at depth 3 — three levels of nesting restored.
#[test]
fn kh_d3_deep_normal_reentry() {
    assert_fixture_passes("K-H-D3-deep-normal-reentry.json");
}

/// K-H-D2: Deep history across-boundary — history cleared after restore; fresh entry uses initialState.
///
/// Two interrupt-resume cycles each clear history on exit, so two `HistoryCleared`
/// records must be present (one per cycle).  The any-match assertion in the fixture
/// only asserts at least one exists; this test enforces the exact count.
#[test]
fn kh_d2_deep_across_boundary() {
    let path = fixture_path("K-H-D2-deep-across-boundary.json");
    let fixture_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture '{path}': {e}"));
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!(
            "K-H-D2-deep-across-boundary FAILED:\n{}",
            result.failures.join("\n")
        );
    }

    let cleared_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::HistoryCleared)
        .count();

    assert_eq!(
        cleared_count, 2,
        "two interrupt-resume cycles must each produce a HistoryCleared record (got {cleared_count})"
    );
}

/// K-H-D2: Shallow history across-boundary — history cleared after restore; fresh entry uses initialState.
///
/// Two interrupt-resume cycles each clear history on exit, so two `HistoryCleared`
/// records must be present (one per cycle).  The any-match assertion in the fixture
/// only asserts at least one exists; this test enforces the exact count.
#[test]
fn kh_d2_shallow_across_boundary() {
    let path = fixture_path("K-H-D2-shallow-across-boundary.json");
    let fixture_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture '{path}': {e}"));
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!(
            "K-H-D2-shallow-across-boundary FAILED:\n{}",
            result.failures.join("\n")
        );
    }

    let cleared_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::HistoryCleared)
        .count();

    assert_eq!(
        cleared_count, 2,
        "two interrupt-resume cycles must each produce a HistoryCleared record (got {cleared_count})"
    );
}

// ── Milestones (KS.2) ────────────────────────────────────────────

/// K-M-001: A setData action makes a milestone condition true; exactly one MilestoneFired is emitted.
#[test]
fn km_001_single_fire() {
    assert_fixture_passes("K-M-001-single-fire.json");
}

/// K-M-003: Milestone-inside-repeat is not supported by the kernel model; fixture verifies a
/// lifecycle-level milestone evaluating a scalar aggregate condition (reviewCount >= 2) that fires
/// after the second setData write but not the first.
#[test]
fn km_003_aggregate_condition() {
    let path = fixture_path("K-M-003-aggregate-condition.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!("K-M-003 FAILED:\n{}", result.failures.join("\n"));
    }

    let fire_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::MilestoneFired)
        .count();

    // Must fire exactly once — after the second write sets reviewCount=2.
    assert_eq!(
        fire_count, 1,
        "dualReviewComplete milestone must fire exactly once (after reviewCount reaches 2), got {fire_count}"
    );
}

/// K-M-004: Provenance ordering — CaseStateMutation precedes MilestoneFired, which precedes
/// StateTransition from the reactive completion event.  The expected_provenance any-match field
/// cannot enforce ordering, so this test checks absolute index positions.
#[test]
fn km_004_ordering_with_transition() {
    let path = fixture_path("K-M-004-ordering-with-transition.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!("K-M-004 FAILED:\n{}", result.failures.join("\n"));
    }

    let provenance = &result.provenance;

    // Locate the key records.
    let mutation_idx = provenance
        .iter()
        .position(|p| p.record_kind == wos_conformance::ProvenanceKind::CaseStateMutation)
        .expect("CaseStateMutation record must be present");

    let milestone_idx = provenance
        .iter()
        .position(|p| p.record_kind == wos_conformance::ProvenanceKind::MilestoneFired)
        .expect("MilestoneFired record must be present");

    // The StateTransition from review→approved fires after the task completion event drains.
    let transition_idx = provenance
        .iter()
        .rposition(|p| {
            p.record_kind == wos_conformance::ProvenanceKind::StateTransition
                && p.from_state.as_deref() == Some("review")
                && p.to_state.as_deref() == Some("approved")
        })
        .expect("StateTransition review→approved must be present");

    assert!(
        mutation_idx < milestone_idx,
        "CaseStateMutation (idx={mutation_idx}) must precede MilestoneFired (idx={milestone_idx})"
    );
    assert!(
        milestone_idx < transition_idx,
        "MilestoneFired (idx={milestone_idx}) must precede StateTransition review→approved (idx={transition_idx})"
    );
}

/// K-M-005: A milestone whose condition depends on a case-state field populated by an integration
/// output binding fires after the output binding applies (Kernel S4.13).
#[test]
fn km_005_milestone_fires_on_integration_response() {
    assert_fixture_passes("K-M-005-milestone-fires-on-integration-response.json");
}

// ── Business Calendar SLA runtime (BC.1) ─────────────────────────

/// G-S10-001: Fixed holiday on the naive deadline day causes the deadline to
/// skip to the next non-holiday work day (BC S10).
#[test]
fn gs10_001_holiday_shift() {
    assert_fixture_passes("G-S10-001-holiday-shift.json");
}

/// G-S10-002: Naive deadline past operating-hours end carries excess time to the
/// next business day's operating-hours start (BC S10).
#[test]
fn gs10_002_operating_hours_cutoff() {
    assert_fixture_passes("G-S10-002-operating-hours-cutoff.json");
}

/// G-S10-003: Calendar timezone is used when computing the deadline — naive
/// deadline on a weekend in the configured TZ snaps to Monday local time (BC S10).
#[test]
fn gs10_003_timezone_boundary() {
    assert_fixture_passes("G-S10-003-timezone-boundary.json");
}

/// G-S10-004: Deadlines are computed lazily at drain time; attaching calendar v2
/// (with a new holiday) shifts the deadline compared to v1 (BC S10).
#[test]
fn gs10_004_calendar_update_shifts_future_deadline() {
    assert_fixture_passes("G-S10-004-calendar-update-shifts-future-deadline.json");
}

/// G-S10-005: Business-calendar deadline does not drift across multiple drains.
///
/// A P1D delay forces a first drain that persists the snapped deadline.  A second
/// drain then fires the timer at the same snapped deadline computed from the original
/// `created_at_ms`, not from the previously-snapped `deadline_ms`.  Without the fix
/// the second drain recomputes from `snapped_1 - duration`, producing a drifted start
/// and a different (wrong) snapped deadline (BC S10).
#[test]
fn gs10_005_no_deadline_drift_across_drains() {
    assert_fixture_passes("G-S10-005-no-deadline-drift-across-drains.json");
}

/// K-M-002: A milestone that fired on the first data write does not re-fire on subsequent events
/// even when the condition remains true.  Exactly one MilestoneFired appears across the full sequence.
#[test]
fn km_002_no_refire() {
    let path = fixture_path("K-M-002-no-refire.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!("K-M-002 FAILED:\n{}", result.failures.join("\n"));
    }

    let fire_count = result
        .provenance
        .iter()
        .filter(|p| p.record_kind == wos_conformance::ProvenanceKind::MilestoneFired)
        .count();

    assert_eq!(
        fire_count, 1,
        "applicationApproved milestone must fire exactly once across all three events, got {fire_count}"
    );
}

// ── NB.2: RFC 9535 outputBinding profile ─────────────────────────

/// I-002: Wildcard (`[*]`) plus member access (`.name`) fans out over all array
/// elements and writes the extracted array to the case state (RFC 9535 profile).
#[test]
fn i002_outputbinding_wildcard_extracts_array() {
    let path = fixture_path("I-002-outputbinding-wildcard-extracts-array.json");
    let fixture_json = std::fs::read_to_string(&path).unwrap();
    let base_dir = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let result = run_fixture(&fixture_json, &base_dir).expect("run_fixture failed");

    if !result.passed {
        panic!("I-002 FAILED:\n{}", result.failures.join("\n"));
    }

    // Verify the dataMapping provenance record carries the correct updatedPaths.
    let data_mapping = result
        .provenance
        .iter()
        .find(|p| p.record_kind == wos_conformance::ProvenanceKind::DataMapping)
        .expect("expected a dataMapping provenance record");

    let updated_paths = data_mapping
        .data
        .as_ref()
        .and_then(|d| d.get("updatedPaths"))
        .and_then(serde_json::Value::as_array)
        .expect("dataMapping provenance must carry updatedPaths");

    assert!(
        updated_paths
            .iter()
            .any(|p| p.as_str() == Some("caseFile.items")),
        "updatedPaths must include 'caseFile.items', got: {updated_paths:?}"
    );
}
