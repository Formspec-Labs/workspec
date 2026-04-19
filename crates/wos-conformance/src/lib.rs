// Rust guideline compliant 2026-02-21

//! Dynamic conformance test runner for WOS workflows.
//!
//! Executes event sequences against WOS kernel documents using the
//! deterministic evaluation algorithm from the Lifecycle Detail Companion,
//! and asserts on state transitions, provenance records, timer behavior,
//! compensation ordering, and deontic enforcement.
//!
//! This crate covers Tier 3 of the WOS Verification Matrix (98 rules).
//! See `LINT-MATRIX.md` for the complete constraint catalog.

mod engine;
pub mod explain;
mod fixture;
pub mod formspec_processor;
mod meta;
mod provenance;
pub mod rules;
pub mod stubs;
pub mod trace;
pub mod coverage;

pub use engine::WorkflowEngine;
pub use fixture::{
    ConformanceFixture, ContractOutcome, EventEntry, ExpectedTransition, TaskSubmission,
};
pub use meta::{
    observe_delegated_formspec_evaluation, run_profile_against_fixtures,
    validate_ai_family_batch_coverage, verify_processor_manifest, AssistGovernanceProxyEvidence,
    ClaimStatus, ClaimVerification, DelegatedFormspecEvaluationEvidence, ProcessorClaims,
    ProcessorConformanceReport, ProcessorEvidence, ProcessorManifest, AI_CONFIDENCE_BATCHES,
    AI_REGISTRATION_BATCHES, GOVERNANCE_BASIC_RULES,
};
pub use provenance::{ProvenanceKind, ProvenanceRecord};
pub use stubs::{StubService, StubValidator};
pub use explain::{
    diff_traces, render_diff, render_trace, DivergenceCause, TraceDiffResult, TraceDivergence,
};
pub use trace::{
    ConformanceTrace, Delta, Event, GuardEvaluation, Outcome, PolicyApplication, TraceStep,
};
pub use wos_core::proxy::observe_assist_governance_proxy;

/// Run a conformance fixture and return the results.
///
/// Document paths in the fixture are resolved relative to `base_dir`.
/// Pass the directory containing the fixture file so that relative paths
/// like `../../../../fixtures/kernel/example.json` resolve correctly.
///
/// # Examples
///
/// ```no_run
/// use wos_conformance::run_fixture;
///
/// let fixture_json = std::fs::read_to_string("fixture.json").unwrap();
/// let result = run_fixture(&fixture_json, ".").unwrap();
/// assert!(result.passed, "fixture failed: {:?}", result.failures);
/// ```
///
/// # Errors
///
/// Returns `ConformanceError::Parse` if the fixture or documents cannot
/// be parsed, or `ConformanceError::Engine` if the workflow engine
/// encounters an internal error.
pub fn run_fixture(
    fixture_json: &str,
    base_dir: &str,
) -> Result<ConformanceResult, ConformanceError> {
    let mut fixture: ConformanceFixture =
        serde_json::from_str(fixture_json).map_err(|e| ConformanceError::Parse(e.to_string()))?;

    // Resolve file-backed document paths relative to base_dir. The sentinel
    // value "inline" is resolved by `WorkflowEngine` from `inline_documents`.
    for path in fixture.documents.values_mut() {
        if path == "inline" {
            continue;
        }
        if !std::path::Path::new(path).is_absolute() {
            let resolved = std::path::Path::new(base_dir).join(&*path);
            *path = resolved
                .to_str()
                .ok_or_else(|| ConformanceError::Parse("document path is not valid UTF-8".into()))?
                .to_string();
        }
    }

    let mut engine = engine::WorkflowEngine::new(&fixture)?;
    let result = engine.execute(&fixture)?;

    Ok(result)
}

/// Run a conformance fixture and return both the pass/fail result and a
/// structured [`ConformanceTrace`].
///
/// The trace is built from the observed state transitions, the per-drain
/// guard evaluations and the provenance-derived policy applications
/// captured during execution. Each [`TraceStep`] carries:
///   - state before / after and the triggering event
///   - every guard expression tested during the step's drain (including
///     short-circuited `false` guards on competing transitions), with
///     inputs subset to the paths the guard referenced
///   - every policy / rule that applied during the step's drain (governance
///     deontic resolutions, autonomy computations, override records, etc.),
///     with parameter bindings preserved from the underlying provenance
///   - a [`Delta`] when the actual target diverges from
///     [`ConformanceFixture::expected_transitions`] — enriched to
///     [`Delta::GuardFalse`] when the expected transition's guard
///     evaluated false, so repair prompts see the blocking expression
///     directly rather than a bare state mismatch.
///
/// The trace is also written to
/// `target/conformance-traces/<fixture-slug>.json` on every run (pass or fail).
/// The output directory is created automatically if it does not exist.
///
/// # Errors
///
/// Same as [`run_fixture`].
pub fn run_fixture_with_trace(
    fixture_json: &str,
    base_dir: &str,
) -> Result<(ConformanceResult, ConformanceTrace), ConformanceError> {
    let result = run_fixture(fixture_json, base_dir)?;

    // Parse fixture again to extract id and event_sequence metadata.
    let fixture: ConformanceFixture =
        serde_json::from_str(fixture_json).map_err(|e| ConformanceError::Parse(e.to_string()))?;

    let trace = build_trace_from_result(&fixture, &result);
    emit_trace_to_disk(&fixture.id, &trace);

    Ok((result, trace))
}

/// Build a [`ConformanceTrace`] from a completed [`ConformanceResult`].
///
/// One [`TraceStep`] is emitted per observed transition. For fixtures that
/// produce no transitions (error/lint-negative fixtures), the trace records
/// the expected zero-transition outcome directly.
///
/// Guard evaluations from [`ConformanceResult::guard_evaluations`] are
/// attached to trace steps using the heuristic that guards sharing the
/// step's `(event, source_state)` pair participated in producing that step
/// or one of its short-circuited siblings. This matches the evaluator's
/// in-drain ordering (guards fire strictly before the transition they
/// gate) and gives the §5.3 teaching signal per-step granularity.
///
/// The `kernel_version` is extracted from the fixture's event sequence
/// metadata. Because the conformance harness does not re-expose the parsed
/// kernel document after `run_fixture` completes, the version defaults to
/// `"unknown"` — callers that need the exact kernel version should read it
/// directly from the kernel document before calling this function.
fn build_trace_from_result(fixture: &ConformanceFixture, result: &ConformanceResult) -> ConformanceTrace {
    // Use a synthetic kernel version derived from fixture metadata. The engine
    // does not return kernel metadata through `ConformanceResult`, so we use
    // "1.0" as the stable conformance-harness sentinel. Real version info is
    // available by parsing the kernel document independently.
    let kernel_version = "1.0".to_string();

    let outcome = if result.passed {
        Outcome::Pass
    } else {
        Outcome::Fail
    };

    let mut trace = ConformanceTrace::new(fixture.id.clone(), kernel_version);
    trace.outcome = outcome;

    // Map each observed transition to a TraceStep. The fixture's
    // expected_transitions list provides the expected target state for each
    // step index. If the fixture has fewer expected transitions than observed
    // ones, the extra steps carry no expected state.
    for (idx, transition) in result.transitions.iter().enumerate() {
        let expected_state_after = fixture
            .expected_transitions
            .get(idx)
            .map(|exp| exp.to.clone());

        // Compute delta when actual state differs from expected.
        //
        // If a guard on the expected transition evaluated false, surface
        // Delta::GuardFalse instead of a bare StateMismatch. This is the
        // §5.3 teaching signal: an LLM reading the trace learns exactly
        // which guard blocked its intended path and what inputs it saw.
        let delta = expected_state_after.as_ref().and_then(|expected| {
            if *expected == transition.to {
                return None;
            }
            let blocking_guard = result.guard_evaluations.iter().find(|g| {
                !g.result
                    && g.event == transition.event
                    && g.source_state == transition.from
                    && g.target_state == *expected
            });
            if let Some(guard) = blocking_guard {
                Some(Delta::GuardFalse {
                    guard_id: guard.guard_id.clone(),
                    expression: guard.expression.clone(),
                    inputs: guard.inputs.clone(),
                })
            } else {
                Some(Delta::StateMismatch {
                    expected: expected.clone(),
                    actual: transition.to.clone(),
                    cause: None,
                })
            }
        });

        let step = TraceStep {
            step_index: idx as u32,
            event: Event {
                name: transition.event.clone(),
                source_actor: fixture
                    .event_sequence
                    .get(idx)
                    .and_then(|e| e.actor.clone()),
                payload: fixture
                    .event_sequence
                    .get(idx)
                    .and_then(|e| e.data.clone()),
            },
            state_before: transition.from.clone(),
            state_after: transition.to.clone(),
            expected_state_after,
            guards_evaluated: result
                .guard_evaluations
                .iter()
                .filter(|g| g.event == transition.event && g.source_state == transition.from)
                .cloned()
                .collect(),
            policies_applied: policy_applications_for_event(&result.provenance, &transition.event),
            delta,
        };

        trace.push_step(step);
    }

    trace
}

/// Extract structured [`PolicyApplication`] records from a flat provenance log.
///
/// Scans records in drain order and synthesizes one `PolicyApplication`
/// per record whose `record_kind` returns true from
/// [`ProvenanceKind::is_policy_application`]. The synthesized
/// `parameter_bindings` is the whole `data` object so downstream consumers
/// (CLI `explain`, LLM repair prompts) can surface rule parameters verbatim.
///
/// `policy_id` resolution order:
///   1. `data.ruleId` (if governance rules adopt a canonical rule-id key)
///   2. `data.policyId` (if AI integration docs adopt a policy-id key)
///   3. `data.constraintId` (what governance actually emits today — see
///      `crates/wos-core/src/deontic.rs::DeonticBypass` construction)
///   4. `data.id` or `data.tool` (autonomy / tool-governance fallback)
///   5. The record_kind's camelCase name (`deonticResolution`,
///      `autonomyComputed`, …) — used for aggregate records that carry
///      no single identifier but whose kind IS the teaching signal
///      (e.g. `DeonticResolution` with `{effectiveAction, reason}`).
///
/// The `event_filter` restricts output to records whose `event` field
/// matches. Governance records are stamped with the drain's event by
/// `wos-runtime::drain_once` (see the stamping loop there); kernel-layer
/// records set `event` directly at construction. Records with no event
/// are skipped — they belong to a broader scope (instance or case-wide)
/// and would mis-attach if forced onto a specific step.
///
/// This is a heuristic: the runtime does not yet expose a canonical
/// "policy application" seam on its companion-policy return type. When
/// that seam lands, replace the extractor body with the structured
/// pass-through and keep the `PolicyApplication` shape stable.
fn policy_applications_for_event(
    provenance: &[ProvenanceRecord],
    event_filter: &str,
) -> Vec<PolicyApplication> {
    provenance
        .iter()
        .filter(|record| record.record_kind.is_policy_application())
        .filter(|record| {
            record
                .event
                .as_deref()
                .map(|e| e == event_filter)
                .unwrap_or(false)
        })
        .map(|record| {
            let data = record
                .data
                .clone()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let policy_id = data
                .get("ruleId")
                .or_else(|| data.get("policyId"))
                .or_else(|| data.get("constraintId"))
                .or_else(|| data.get("id"))
                .or_else(|| data.get("tool"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| policy_kind_id(&record.record_kind));
            PolicyApplication {
                policy_id,
                parameter_bindings: data,
            }
        })
        .collect()
}

/// Fallback policy id for aggregate records that carry no explicit
/// identifier — uses the record_kind's camelCase serde name.
fn policy_kind_id(kind: &ProvenanceKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{:?}", kind))
}

/// Write a [`ConformanceTrace`] to `target/conformance-traces/<slug>.json`.
///
/// Silently no-ops if the directory cannot be created or the file cannot be
/// written — trace emission must never break the test suite.
fn emit_trace_to_disk(fixture_id: &str, trace: &ConformanceTrace) {
    let slug = slugify(fixture_id);
    let dir = std::path::Path::new("target/conformance-traces");
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let path = dir.join(format!("{slug}.json"));
    if let Ok(json) = serde_json::to_string_pretty(trace) {
        let _ = std::fs::write(path, json);
    }
}

/// Convert a fixture id to a filesystem-safe slug.
///
/// Lowercases the string and replaces any character that is not alphanumeric
/// or a hyphen with a hyphen, then collapses consecutive hyphens.
pub fn slugify(id: &str) -> String {
    let mut slug = String::with_capacity(id.len());
    let mut prev_hyphen = false;
    for ch in id.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    slug.trim_matches('-').to_string()
}

/// Result of running a conformance fixture.
#[derive(Debug)]
pub struct ConformanceResult {
    /// Whether all assertions passed.
    pub passed: bool,

    /// Failures, if any.
    pub failures: Vec<String>,

    /// Actual state transitions observed.
    pub transitions: Vec<engine::Transition>,

    /// Actual provenance records produced.
    pub provenance: Vec<ProvenanceRecord>,

    /// Guard expressions evaluated, in observation order (§5.3 teaching signal).
    pub guard_evaluations: Vec<GuardEvaluation>,

    /// Binding discriminator used during execution.
    pub binding_used: Option<String>,
}

/// Errors from the conformance runner.
#[derive(Debug, thiserror::Error)]
pub enum ConformanceError {
    /// Fixture or document parsing failed.
    #[error("parse error: {0}")]
    Parse(String),

    /// Workflow engine internal error.
    #[error("engine error: {0}")]
    Engine(String),

    /// Referenced document file not found.
    #[error("document not found: {0}")]
    DocumentNotFound(String),
}

#[cfg(test)]
mod runner_trace_tests {
    use super::*;

    /// Resolve a fixture from `tests/fixtures/` and return (json, base_dir).
    /// The `tests/fixtures/` directory is already wired for `run_fixture`.
    fn read_test_fixture(name: &str) -> (String, String) {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let path = format!("{manifest}/tests/fixtures/{name}");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("could not read fixture '{path}': {e}"));
        let base_dir = std::path::Path::new(&path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        (json, base_dir)
    }

    /// purchase-order-simple: approve then orderProcessed produces two transitions
    /// (submitted → approved → completed). Verify step count, state progression,
    /// and that all steps have correct expected_state_after (no delta on pass).
    #[test]
    fn trace_purchase_order_simple_two_transitions() {
        let (json, base_dir) = read_test_fixture("purchase-order-simple.json");
        let (result, trace) = run_fixture_with_trace(&json, &base_dir)
            .expect("run_fixture_with_trace failed");

        assert!(result.passed, "fixture must pass: {:?}", result.failures);
        assert_eq!(trace.steps.len(), 2, "expected exactly 2 trace steps");

        let step0 = &trace.steps[0];
        assert_eq!(step0.step_index, 0);
        assert_eq!(step0.state_before, "submitted");
        assert_eq!(step0.state_after, "approved");
        assert_eq!(step0.event.name, "approve");
        assert_eq!(
            step0.expected_state_after.as_deref(),
            Some("approved"),
            "step0 expected_state_after must reflect fixture expectation"
        );
        assert!(step0.delta.is_none(), "no delta for a passing step");

        let step1 = &trace.steps[1];
        assert_eq!(step1.step_index, 1);
        assert_eq!(step1.state_before, "approved");
        assert_eq!(step1.state_after, "completed");
        assert_eq!(step1.event.name, "orderProcessed");
        assert!(step1.delta.is_none(), "no delta for a passing step");

        assert_eq!(trace.outcome, Outcome::Pass);
    }

    /// purchase-order-reject-resubmit: four transitions in sequence.
    /// Verify step count, indices, and that all steps are delta-free (all pass).
    #[test]
    fn trace_purchase_order_reject_resubmit_four_transitions() {
        let (json, base_dir) = read_test_fixture("purchase-order-reject-resubmit.json");
        let (result, trace) = run_fixture_with_trace(&json, &base_dir)
            .expect("run_fixture_with_trace failed");

        assert!(result.passed, "fixture must pass: {:?}", result.failures);
        assert_eq!(trace.steps.len(), 4, "expected exactly 4 trace steps");

        for (i, step) in trace.steps.iter().enumerate() {
            assert_eq!(step.step_index, i as u32, "step index must be sequential");
            assert!(
                step.delta.is_none(),
                "step {i} has unexpected delta: {:?}",
                step.delta
            );
        }
        assert_eq!(trace.outcome, Outcome::Pass);
    }

    /// medicaid-happy-path: multi-step compound-state fixture.
    /// Verify step count matches expected_transitions length and all pass.
    #[test]
    fn trace_medicaid_happy_path_step_count_matches_expected_transitions() {
        let (json, base_dir) = read_test_fixture("medicaid-happy-path.json");
        let fixture: ConformanceFixture = serde_json::from_str(&json).unwrap();

        let (result, trace) = run_fixture_with_trace(&json, &base_dir)
            .expect("run_fixture_with_trace failed");

        assert!(result.passed, "fixture must pass: {:?}", result.failures);
        assert_eq!(
            trace.steps.len(),
            fixture.expected_transitions.len(),
            "trace step count must match expected_transitions count"
        );
        assert_eq!(trace.outcome, Outcome::Pass);
    }

    /// §5.3 policy-application extractor: DeonticEvaluation with a
    /// ruleId data field surfaces as a structured PolicyApplication on
    /// the step whose event matches. Non-policy provenance kinds and
    /// unmatched events are filtered out.
    #[test]
    fn policy_applications_synthesized_from_deontic_evaluation_records() {
        let matching = ProvenanceRecord {
            record_kind: ProvenanceKind::DeonticEvaluation,
            timestamp: "2026-04-18T00:00:00Z".to_string(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: Some("approve".to_string()),
            data: Some(serde_json::json!({
                "ruleId": "P-income-threshold",
                "version": "v2",
                "threshold": 50_000
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        };
        // Non-policy kind: must be filtered out.
        let state_transition = ProvenanceRecord {
            record_kind: ProvenanceKind::StateTransition,
            timestamp: "2026-04-18T00:00:00Z".to_string(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: Some("approve".to_string()),
            data: Some(serde_json::json!({ "ruleId": "not-a-policy" })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        };
        // Different event: must be filtered out.
        let other_event = ProvenanceRecord {
            record_kind: ProvenanceKind::AutonomyComputed,
            timestamp: "2026-04-18T00:00:00Z".to_string(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: Some("other".to_string()),
            data: Some(serde_json::json!({ "policyId": "P-other" })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        };

        let provenance = vec![matching, state_transition, other_event];
        let applied = policy_applications_for_event(&provenance, "approve");

        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].policy_id, "P-income-threshold");
        assert_eq!(
            applied[0].parameter_bindings,
            serde_json::json!({
                "ruleId": "P-income-threshold",
                "version": "v2",
                "threshold": 50_000
            })
        );
    }

    /// §5.3 policy extractor: governance records carry `constraintId` in
    /// `data` (see `crates/wos-core/src/deontic.rs`) — the extractor must
    /// recognize that key, not just `ruleId` / `policyId`. Without this,
    /// every real governance fixture produced empty `policies_applied`.
    #[test]
    fn policy_applications_recognize_constraint_id_on_deontic_bypass() {
        let bypass = ProvenanceRecord {
            record_kind: ProvenanceKind::DeonticBypass,
            timestamp: "2026-04-18T00:00:00Z".to_string(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: Some("approve".to_string()),
            data: Some(serde_json::json!({
                "constraintId": "perm-income-range",
                "constraintType": "permission",
                "rationale": "emergency override"
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        };
        let applied = policy_applications_for_event(&[bypass], "approve");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].policy_id, "perm-income-range");
    }

    /// §5.3 policy extractor: aggregate records with no identifier
    /// (e.g. `DeonticResolution` which carries `{effectiveAction, reason}`)
    /// fall back to the record_kind's camelCase name so they still
    /// contribute to the teaching signal.
    #[test]
    fn policy_applications_fall_back_to_kind_name_when_no_identifier() {
        let resolution = ProvenanceRecord {
            record_kind: ProvenanceKind::DeonticResolution,
            timestamp: "2026-04-18T00:00:00Z".to_string(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: Some("determined".to_string()),
            data: Some(serde_json::json!({
                "effectiveAction": "reject",
                "reason": "most-restrictive"
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
        };
        let applied = policy_applications_for_event(&[resolution], "determined");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].policy_id, "deonticResolution");
        assert_eq!(
            applied[0].parameter_bindings["effectiveAction"],
            serde_json::json!("reject")
        );
    }

    /// slugify: verifies fixture id → safe slug conversion.
    #[test]
    fn slugify_converts_ids_to_safe_slugs() {
        assert_eq!(slugify("K-011-determinism"), "k-011-determinism");
        assert_eq!(slugify("AI-041-negative-fallback-cycle"), "ai-041-negative-fallback-cycle");
        assert_eq!(slugify("K-011-parallel-join"), "k-011-parallel-join");
        // Non-alphanumeric characters other than hyphen collapse to a single hyphen.
        assert_eq!(slugify("foo_bar.baz"), "foo-bar-baz");
    }
}
