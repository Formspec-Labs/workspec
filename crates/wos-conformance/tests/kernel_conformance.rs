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

/// AI-017: Null deontic expression in rights-impacting workflow escalates to human (AI S4.9).
#[test]
fn ai017_null_escalation() {
    assert_fixture_passes("ai-017-null-escalation.json");
}

/// AI-027: Escalation does NOT bypass deontic constraints (AI S5.4).
#[test]
fn ai027_escalation_deontic_not_bypassed() {
    assert_fixture_passes("ai-027-escalation-deontic-not-bypassed.json");
}

/// AI-051: Assist Governance Proxy applies deontic constraints to tool invocations (AI S14.2).
#[test]
fn ai051_proxy_deontic() {
    assert_fixture_passes("ai-051-proxy-deontic.json");
}

/// AI-054: Deontic bypass applies to single invocation only (AI S4.7).
#[test]
fn ai054_bypass_single_invocation() {
    assert_fixture_passes("ai-054-bypass-single-invocation.json");
}

/// AI-055: Consistency constraints detect contradictions between output and case data (AI S4.7).
#[test]
fn ai055_consistency_contradiction() {
    assert_fixture_passes("ai-055-consistency-contradiction.json");
}

// ── Batch 4: Autonomy caps ──────────────────────────────────────

/// AI-005: Agents MUST NOT override human decisions (AI S3.7).
#[test]
fn ai005_no_override_human() {
    assert_fixture_passes("ai-005-no-override-human.json");
}

/// AI-019: Assistive actions MUST create a human task for confirmation (AI S5.3).
#[test]
fn ai019_assistive_creates_human_task() {
    assert_fixture_passes("ai-019-assistive-creates-human-task.json");
}

/// AI-021: Effective autonomy MUST NOT exceed impact-level cap (AI S5.3).
#[test]
fn ai021_impact_level_cap() {
    assert_fixture_passes("ai-021-impact-level-cap.json");
}

/// AI-022: Effective autonomy = minimum of 4 sources (AI S5.3).
#[test]
fn ai022_effective_autonomy_minimum() {
    assert_fixture_passes("ai-022-effective-autonomy-minimum.json");
}

/// AI-025: Human approval required for escalation (AI S5.4).
#[test]
fn ai025_escalation_requires_approval() {
    assert_fixture_passes("ai-025-escalation-requires-approval.json");
}

/// AI-028: Demotion takes effect for next invocation (AI S5.5).
#[test]
fn ai028_demotion_next_invocation() {
    assert_fixture_passes("ai-028-demotion-next-invocation.json");
}

/// AI-029: pendingRecalibration keeps demoted level (AI S5.5).
#[test]
fn ai029_pending_recalibration() {
    assert_fixture_passes("ai-029-pending-recalibration.json");
}

/// AI-030: Dynamic autonomy MUST NOT exceed effective cap (AI S5.6).
#[test]
fn ai030_dynamic_autonomy_cap() {
    assert_fixture_passes("ai-030-dynamic-autonomy-cap.json");
}

/// AC-001: Expired calibration caps autonomy at assistive (AgentConfig S1.3).
#[test]
fn ac001_expired_calibration_cap() {
    assert_fixture_passes("ac-001-expired-calibration-cap.json");
}

/// AC-002: maxAutonomy participates in cross-document minimum (AgentConfig S1.4).
#[test]
fn ac002_max_autonomy_minimum() {
    assert_fixture_passes("ac-002-max-autonomy-minimum.json");
}

/// AG-005: Agent MUST NOT invoke tools not in permitted list (AdvGov S6.1).
#[test]
fn ag005_tool_not_permitted() {
    assert_fixture_passes("ag-005-tool-not-permitted.json");
}

/// AG-006: Agent MUST NOT write to case file directly (AdvGov S6.1).
#[test]
fn ag006_no_direct_case_write() {
    assert_fixture_passes("ag-006-no-direct-case-write.json");
}

/// AG-007: Tool invocations MUST respect rate limits (AdvGov S6.1).
#[test]
fn ag007_tool_rate_limit() {
    assert_fixture_passes("ag-007-tool-rate-limit.json");
}

// ── Batch 5: Confidence framework ───────────────────────────────

/// AI-034: Every agent output MUST have a ConfidenceReport (AI S7.1).
#[test]
fn ai034_confidence_report_required() {
    assert_fixture_passes("ai-034-confidence-report-required.json");
}

/// AI-035: modelNative confidence MUST be calibrated (AI S7.2).
#[test]
fn ai035_calibrated_confidence() {
    assert_fixture_passes("ai-035-calibrated-confidence.json");
}

/// AI-036: Confidence below floor invalidates output (AI S7.4).
#[test]
fn ai036_confidence_below_floor() {
    assert_fixture_passes("ai-036-confidence-below-floor.json");
}

/// AI-037: DecayTrigger multiplies confidence; below floor triggers escalation (AI S7.5).
#[test]
fn ai037_decay_trigger() {
    assert_fixture_passes("ai-037-decay-trigger.json");
}

/// AI-038: Cumulative confidence below floor pauses for human review (AI S7.7).
#[test]
fn ai038_cumulative_confidence_pause() {
    assert_fixture_passes("ai-038-cumulative-confidence-pause.json");
}

/// AG-004: Session pause at checkpoint when cumulative confidence drops (AdvGov S5.4).
#[test]
fn ag004_session_pause() {
    assert_fixture_passes("ag-004-session-pause.json");
}

/// AG-016: Every review provides ground-truth label (AdvGov S9.3).
#[test]
fn ag016_review_ground_truth() {
    assert_fixture_passes("ag-016-review-ground-truth.json");
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

/// AI-045: independentFirst suppression hides agent output until independent assessment (AI S10.2).
#[test]
fn ai045_independent_first_suppression() {
    assert_fixture_passes("ai-045-independent-first-suppression.json");
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

/// AI-006: Agent provenance includes model ID, version, confidence, input summary (AI S3.7).
#[test]
fn ai006_agent_provenance_fields() {
    assert_fixture_passes("ai-006-agent-provenance-fields.json");
}

/// AI-008: Actor type is immutable for a given action (AI S3.7).
#[test]
fn ai008_actor_type_immutable() {
    assert_fixture_passes("ai-008-actor-type-immutable.json");
}

/// AI-033: Agent-touched fields annotated with agentProvenance (AI S6.2).
#[test]
fn ai033_agent_touched_annotation() {
    assert_fixture_passes("ai-033-agent-touched-annotation.json");
}

/// AI-044: Training data contamination triggers reclassification (AI S9.3).
#[test]
fn ai044_drift_reclassification() {
    assert_fixture_passes("ai-044-drift-reclassification.json");
}

/// AI-047: Narrative tier provenance labeled non-authoritative (AI S13.2).
#[test]
fn ai047_narrative_non_authoritative() {
    assert_fixture_passes("ai-047-narrative-non-authoritative.json");
}

/// AI-052: Proxy produces provenance per governed invocation (AI S14.2).
#[test]
fn ai052_proxy_provenance() {
    assert_fixture_passes("ai-052-proxy-provenance.json");
}

/// AI-053: Version change emits agentVersionChange provenance (AI S3.4).
#[test]
fn ai053_version_change_provenance() {
    assert_fixture_passes("ai-053-version-change-provenance.json");
}

/// AG-009: Agent state transitions produce provenance (AdvGov S7.2).
#[test]
fn ag009_agent_state_provenance() {
    assert_fixture_passes("ag-009-agent-state-provenance.json");
}

/// AI-057: Processor enforces constraints; agent cannot weaken its own (AI S3.5).
#[test]
fn ai057_processor_enforces_constraints() {
    assert_fixture_passes("ai-057-processor-enforces-constraints.json");
}

/// AI-032: Validation failures trigger fallback, not silent acceptance (AI S6.2).
#[test]
fn ai032_validation_triggers_fallback() {
    assert_fixture_passes("ai-032-validation-triggers-fallback.json");
}

/// AI-039: Every fallback attempt produces provenance (AI S8.2).
#[test]
fn ai039_fallback_provenance() {
    assert_fixture_passes("ai-039-fallback-provenance.json");
}

/// AI-040: Terminal fallback produces result or human task (AI S8.2).
#[test]
fn ai040_terminal_fallback() {
    assert_fixture_passes("ai-040-terminal-fallback.json");
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

/// AI-048: Narrative tier not treated as dispositive evidence (AI S13.2).
#[test]
fn ai048_narrative_not_dispositive() {
    assert_fixture_passes("ai-048-narrative-not-dispositive.json");
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
        if path.extension().map_or(false, |e| e == "json") {
            let json = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let fixture: wos_conformance::ConformanceFixture = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
            // Verify document paths resolve relative to the fixture directory.
            let base = path.parent().unwrap().to_str().unwrap();
            for (role, doc_path) in &fixture.documents {
                let full = format!("{base}/{doc_path}");
                assert!(
                    std::path::Path::new(&full).exists(),
                    "fixture {} references non-existent {role} document: {doc_path}",
                    path.display()
                );
            }
        }
    }
}
