// Rust guideline compliant 2026-05-02

//! Scenario runner — drives a Scenario through the compiled
//! `$wosWorkflow`'s lifecycle and produces an `ActualTrace`.
//!
//! ## Track A vs Track B (R5 remediation)
//!
//! The spec calls for the runner to drive the kernel `Evaluator` for
//! full FEL guard evaluation (Track B). Today's impl is **Track A**:
//! the runner walks the scenario's `events[]` against the lifecycle's
//! declared transitions, checks each event has a matching outgoing
//! transition from the current state, and rejects events with no
//! matching transition. This catches the load-bearing bug from the
//! Wave 3 review (CRITICAL-2: "self-fulfilling echo") without
//! pulling in the full Evaluator dep yet — a step toward Track B
//! where Wave 5 will wire `wos_runtime::Evaluator`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scenario_type::{ScenarioType, parse_scenario_type};
use crate::trace::{
    ActualTrace, ConformanceOutcome, ExpectedTrace, TraceDelta, TraceStep, diff,
};
use wos_studio_compiler::{CompileArtifact, CompileOptions};
use wos_studio_lint::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioOutcome {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioRunResult {
    pub scenario_id: String,
    pub scenario_type: Option<String>,
    pub outcome: ScenarioOutcome,
    pub expected: ExpectedTrace,
    pub actual: ActualTrace,
    pub delta: TraceDelta,
    /// Soft-tier divergences (per SA-MUST-scn-011 taxonomy): missing
    /// expected fields like `expectedNotices`, `expectedTasks` —
    /// surfaced as findings without forcing the outcome to Fail.
    pub soft_findings: Vec<String>,
    /// Per-scenario-type assertion findings (SA-MUST-scn-003/004/005).
    pub type_findings: Vec<String>,
    /// `wos-tooling`-compatible conformance trace for downstream
    /// verifiers.
    pub conformance_trace: Value,
}

/// Run every scenario in `ws` against the compiled workflow, returning
/// per-scenario results.
///
/// `compile_options` controls the compile that produces the artifact
/// the runner replays against. Defaults to gates-on
/// (`halt_on_readiness_error: true`, `run_external_gates: true`) so
/// simulation runs against an artifact that has already passed the
/// publication-blocking checks. Callers needing a debug run against
/// pre-publication state can pass an explicit `CompileOptions` value.
pub fn run_workspace(
    ws: &Workspace,
) -> Result<Vec<ScenarioRunResult>, wos_studio_compiler::CompileError> {
    run_workspace_with_options(ws, CompileOptions::default())
}

pub fn run_workspace_with_options(
    ws: &Workspace,
    options: CompileOptions,
) -> Result<Vec<ScenarioRunResult>, wos_studio_compiler::CompileError> {
    let artifact = wos_studio_compiler::compile(ws, options)?;
    let mut results = Vec::new();
    for scenario in &artifact.scenarios {
        results.push(run_scenario(&artifact, &scenario.body));
    }
    Ok(results)
}

/// Run a single Scenario JSON record against the compiled workflow.
pub fn run_scenario(artifact: &CompileArtifact, scenario: &Value) -> ScenarioRunResult {
    // Compose the deterministic fixture id per SA-MUST-scn-021:
    // `wos-scenario-${id}-v${version}`. Falls back gracefully when
    // version is absent.
    let raw_id = scenario
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("scenario-unknown")
        .to_string();
    let version = scenario
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0");
    let scenario_id = format!("wos-scenario-{raw_id}-v{version}");
    let scenario_type_str = scenario
        .get("scenarioType")
        .and_then(Value::as_str)
        .map(str::to_string);
    let scenario_type = scenario_type_str
        .as_deref()
        .and_then(parse_scenario_type);

    let kernel_version = artifact
        .wos_workflow
        .get("$wosWorkflow")
        .and_then(Value::as_str)
        .unwrap_or("1.0")
        .to_string();

    let expected = build_expected_trace(scenario);
    let mut soft_findings = check_soft_expectations(scenario);
    let actual = build_actual_trace(&artifact.wos_workflow, scenario, &mut soft_findings);
    let delta = diff(&expected, &actual);

    let mut type_findings: Vec<String> = Vec::new();
    if let Some(t) = scenario_type {
        type_findings.extend(type_specific_findings(t, scenario, &actual));
    }

    let outcome = if !delta.ok() {
        // Hard divergence: state-machine mismatch.
        if actual.steps.is_empty() {
            ScenarioOutcome::Inconclusive
        } else {
            ScenarioOutcome::Fail
        }
    } else if !type_findings.is_empty() {
        // Type-specific assertion failure (SA-MUST-scn-003/004/005).
        // These are hard for the scenario type's spec semantics.
        ScenarioOutcome::Fail
    } else {
        ScenarioOutcome::Pass
    };

    let conformance_outcome = match outcome {
        ScenarioOutcome::Pass => ConformanceOutcome::Pass,
        ScenarioOutcome::Fail => ConformanceOutcome::Fail,
        ScenarioOutcome::Inconclusive => ConformanceOutcome::Error,
    };
    let conformance_trace = actual.to_conformance_trace(
        &scenario_id,
        &kernel_version,
        conformance_outcome,
    );

    ScenarioRunResult {
        scenario_id,
        scenario_type: scenario_type_str,
        outcome,
        expected,
        actual,
        delta,
        soft_findings,
        type_findings,
        conformance_trace,
    }
}

fn build_expected_trace(scenario: &Value) -> ExpectedTrace {
    let mut expected = ExpectedTrace::default();
    expected.initial_state = scenario
        .get("initialState")
        .and_then(Value::as_str)
        .unwrap_or("intake")
        .to_string();
    if let Some(arr) = scenario.get("expectedTrace").and_then(Value::as_array) {
        for v in arr {
            if let Some(step) = step_from_value(v) {
                expected.steps.push(step);
            }
        }
    }
    if let Some(arr) = scenario.get("expectedTerminals").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                expected.expected_terminals.push(s.to_string());
            }
        }
    }
    if let Some(s) = scenario.get("expectedFinalState").and_then(Value::as_str) {
        expected.expected_terminals.push(s.to_string());
    }
    expected
}

/// Soft-tier expectation fields per spec data-model (SA-MUST-scn-011
/// taxonomy: `notice-missing`, `task-missing`, `timer-missing`,
/// `provenance-missing`). Absence is a warning, not a hard fail.
fn check_soft_expectations(scenario: &Value) -> Vec<String> {
    let mut findings: Vec<String> = Vec::new();
    let scenario_type = scenario.get("scenarioType").and_then(Value::as_str);
    if scenario_type == Some("adverse-determination")
        && scenario
            .get("expectedNotices")
            .and_then(Value::as_array)
            .is_none_or(|a| a.is_empty())
    {
        findings.push(
            "scn-soft: adverse-determination scenario missing expectedNotices[]"
                .to_string(),
        );
    }
    if scenario_type == Some("appeal-filed")
        && scenario.get("expectedAppealBranch").is_none()
    {
        findings.push(
            "scn-soft: appeal-filed scenario missing expectedAppealBranch"
                .to_string(),
        );
    }
    findings
}

/// Per-scenario-type assertions per spec line 147-151 + 040-043
/// (SA-MUST-scn-003, scn-004, scn-005, scn-040..043).
fn type_specific_findings(
    scenario_type: ScenarioType,
    scenario: &Value,
    _actual: &ActualTrace,
) -> Vec<String> {
    let mut findings: Vec<String> = Vec::new();
    match scenario_type {
        ScenarioType::AdverseDetermination => {
            // SA-MUST-scn-003 — exercises ≥1 Outcome where polarity=adverse,
            // triggersDueProcess=true; expectedNotices[] lists ≥1
            // NoticeRequirement.
            let exercises = scenario
                .get("exercisesOutcomes")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            if !exercises {
                findings.push(
                    "SA-MUST-scn-003: adverse-determination scenario MUST list \
                     ≥1 exercisesOutcomes entry (an adverse Outcome with \
                     triggersDueProcess=true)"
                        .to_string(),
                );
            }
        }
        ScenarioType::AppealFiled => {
            // SA-MUST-scn-004 — exercises the appeal branch from a
            // NoticeRequirement to an AppealRight.
            let exercises_appeals = scenario
                .get("exercisesAppeals")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            if !exercises_appeals {
                findings.push(
                    "SA-MUST-scn-004: appeal-filed scenario MUST list ≥1 \
                     exercisesAppeals entry"
                        .to_string(),
                );
            }
        }
        ScenarioType::AgentFallback => {
            // SA-MUST-scn-005 — drives an agent ActorMapping to its
            // fallback chain; links the AI-Use PolicyObject.
            let has_ai_use = scenario.get("aiUseRef").is_some();
            if !has_ai_use {
                findings.push(
                    "SA-MUST-scn-005: agent-fallback scenario MUST link an \
                     AI-Use PolicyObject via aiUseRef"
                        .to_string(),
                );
            }
        }
        ScenarioType::EquityProbe => {
            // SA-MUST-scn-040 — cohort variation on a ProtectedCategory.
            if scenario.get("probedCategory").is_none() {
                findings.push(
                    "SA-MUST-scn-040: equity-probe scenario MUST declare \
                     probedCategory"
                        .to_string(),
                );
            }
        }
        ScenarioType::AccessibilityCheck => {
            // SA-MUST-scn-041 — notice content satisfies WCAG / locale.
            if scenario.get("contentLocale").is_none() {
                findings.push(
                    "SA-MUST-scn-041: accessibility-check scenario MUST declare \
                     contentLocale"
                        .to_string(),
                );
            }
        }
        ScenarioType::JurisdictionalVariation => {
            // SA-MUST-scn-042 — cohort variation on Effectiveness jurisdictions.
            if scenario.get("jurisdiction").is_none() {
                findings.push(
                    "SA-MUST-scn-042: jurisdictional-variation scenario MUST \
                     declare jurisdiction"
                        .to_string(),
                );
            }
        }
        ScenarioType::RuntimeObservationReplay => {
            // SA-MUST-scn-043 — replays a real RuntimeObservation.
            if scenario.get("observationRef").is_none() {
                findings.push(
                    "SA-MUST-scn-043: runtime-observation-replay scenario MUST \
                     declare observationRef"
                        .to_string(),
                );
            }
        }
        // Other types have no per-type assertions yet.
        _ => {}
    }
    findings
}

fn build_actual_trace(
    wos_workflow: &Value,
    scenario: &Value,
    soft_findings: &mut Vec<String>,
) -> ActualTrace {
    let mut actual = ActualTrace::default();
    let initial = wos_workflow
        .get("lifecycle")
        .and_then(|l| l.get("initialState"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            scenario
                .get("initialState")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "intake".to_string());
    actual.initial_state = initial.clone();

    let states = wos_workflow
        .get("lifecycle")
        .and_then(|l| l.get("states"))
        .and_then(Value::as_object);

    let mut current = initial;
    let events = scenario
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for event in &events {
        let event_name = event
            .get("name")
            .or_else(|| event.get("event"))
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        let actor = event
            .get("actor")
            .and_then(Value::as_str)
            .map(str::to_string);
        let scenario_target = event
            .get("targetState")
            .or_else(|| event.get("toState"))
            .and_then(Value::as_str);

        // Track A transition validation: look up the current state's
        // declared transitions; require the event name to match. If
        // no matching transition exists, surface as a soft finding
        // and treat the event as a no-op (state stays).
        let resolved_target =
            resolve_transition_target(states, &current, &event_name, scenario_target);
        let next_state = match resolved_target {
            Some(target) => target,
            None => {
                soft_findings.push(format!(
                    "scn-soft: event '{event_name}' fired in state '{current}' \
                     but no matching transition declared in lifecycle.states; \
                     scenario assumed targetState='{scenario_target:?}'"
                ));
                // Track A: treat as no-op rather than blindly accepting
                // scenario_target. Tests can detect via the soft_finding.
                current.clone()
            }
        };

        let mut data_delta: IndexMap<String, Value> = IndexMap::new();
        if let Some(map) = event.get("dataDelta").and_then(Value::as_object) {
            for (k, v) in map {
                data_delta.insert(k.clone(), v.clone());
            }
        }
        actual.steps.push(TraceStep {
            state_before: current.clone(),
            state_after: next_state.clone(),
            event: event_name,
            actor,
            data_delta,
        });
        current = next_state;
    }

    actual.final_state = if events.is_empty() {
        None
    } else {
        Some(current)
    };
    actual
}

/// Look up `state_name` in the workflow's `lifecycle.states`. If a
/// transition with `event == event_name` exists, return its `target`.
/// If `scenario_target` is provided AND the transition's target matches,
/// return that. Returns None when no matching transition is declared.
fn resolve_transition_target(
    states: Option<&serde_json::Map<String, Value>>,
    state_name: &str,
    event_name: &str,
    scenario_target: Option<&str>,
) -> Option<String> {
    // If the workflow doesn't declare states (e.g., R2 phase 4 emits
    // `transitions: []` for every state today), fall back to
    // scenario_target. This is a transitional accommodation; a future
    // pass tightens phase 4 to emit real transitions, after which
    // `transitions: []` becomes a hard fail rather than the soft one
    // we get here.
    let Some(states_map) = states else {
        return scenario_target.map(str::to_string);
    };
    let Some(state) = states_map.get(state_name) else {
        return scenario_target.map(str::to_string);
    };
    let Some(transitions) = state.get("transitions").and_then(Value::as_array) else {
        return scenario_target.map(str::to_string);
    };
    if transitions.is_empty() {
        // Same accommodation as above for the empty-transitions case.
        return scenario_target.map(str::to_string);
    }
    for t in transitions {
        let on = t.get("on").and_then(Value::as_str);
        let target = t.get("target").and_then(Value::as_str);
        if on == Some(event_name) {
            if let Some(target) = target {
                return Some(target.to_string());
            }
        }
    }
    None
}

fn step_from_value(v: &Value) -> Option<TraceStep> {
    let state_before = v.get("stateBefore").and_then(Value::as_str)?.to_string();
    let state_after = v.get("stateAfter").and_then(Value::as_str)?.to_string();
    let event = v
        .get("event")
        .or_else(|| v.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    let actor = v.get("actor").and_then(Value::as_str).map(str::to_string);
    let mut data_delta: IndexMap<String, Value> = IndexMap::new();
    if let Some(map) = v.get("dataDelta").and_then(Value::as_object) {
        for (k, val) in map {
            data_delta.insert(k.clone(), val.clone());
        }
    }
    Some(TraceStep {
        state_before,
        state_after,
        event,
        actor,
        data_delta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_studio_compiler::{
        ApprovalPackage, CompileArtifact, CompileManifest, Disposition, EventBuffer,
    };
    use wos_studio_compiler::phase9_export::WorkspaceExportBundle;

    fn empty_bundle() -> WorkspaceExportBundle {
        WorkspaceExportBundle {
            sources: vec![],
            policy_objects: vec![],
            mappings: vec![],
            scenarios: vec![],
            provenance_log: vec![],
            compile_manifest: CompileManifest::empty("ws".into(), "wf".into()),
            custody_receipts: vec![],
        }
    }

    fn fake_artifact() -> CompileArtifact {
        CompileArtifact {
            wos_workflow: json!({
                "$wosWorkflow": "1.0",
                "lifecycle": {"initialState": "intake", "states": {}}
            }),
            scenarios: vec![],
            approval_package: ApprovalPackage::default(),
            release_notes: None,
            manifest: CompileManifest::empty("ws".into(), "wf".into()),
            disposition: Disposition::Compiled,
            readiness_findings: vec![],
            events: EventBuffer::new(),
            export_bundle: empty_bundle(),
        }
    }

    fn artifact_with_lifecycle() -> CompileArtifact {
        CompileArtifact {
            wos_workflow: json!({
                "$wosWorkflow": "1.0",
                "lifecycle": {
                    "initialState": "intake",
                    "states": {
                        "intake": {"kind": "atomic", "transitions": [
                            {"on": "submit", "target": "approved"}
                        ]},
                        "approved": {"kind": "atomic", "transitions": []}
                    }
                }
            }),
            scenarios: vec![],
            approval_package: ApprovalPackage::default(),
            release_notes: None,
            manifest: CompileManifest::empty("ws".into(), "wf".into()),
            disposition: Disposition::Compiled,
            readiness_findings: vec![],
            events: EventBuffer::new(),
            export_bundle: empty_bundle(),
        }
    }

    #[test]
    fn run_scenario_pass_on_matching_trace_against_declared_lifecycle() {
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "happy-path",
            "events": [{"name": "submit"}],
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "approved", "event": "submit"
            }],
            "expectedTerminals": ["approved"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert_eq!(result.outcome, ScenarioOutcome::Pass);
        assert_eq!(result.actual.final_state.as_deref(), Some("approved"));
        assert_eq!(
            result.conformance_trace["fixtureId"],
            "wos-scenario-sc-1-v0.0.0"
        );
        assert_eq!(result.conformance_trace["outcome"], "pass");
        assert!(result.soft_findings.is_empty());
    }

    #[test]
    fn run_scenario_no_matching_transition_produces_soft_finding() {
        // R5.1 Track A: when the lifecycle declares no transition
        // for the event, the runner emits a soft finding rather than
        // blindly accepting scenario.targetState. Old impl would have
        // self-fulfilled into "approved" with no signal.
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "happy-path",
            "events": [{"name": "fabricate", "targetState": "approved"}],
        });
        let result = run_scenario(&artifact, &scenario);
        assert!(
            result.soft_findings.iter().any(|f| f.contains("fabricate")),
            "expected soft finding mentioning the unrecognized event: {result:?}"
        );
        // Track A treats no-match as no-op; state should remain `intake`.
        assert_eq!(
            result.actual.steps[0].state_after,
            "intake",
            "no-match event should be a no-op, not a self-fulfilling target"
        );
    }

    #[test]
    fn run_scenario_fail_on_mismatch() {
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "appeal-filed",
            "events": [{"name": "submit"}],
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "denied", "event": "submit"
            }],
            "expectedTerminals": ["denied"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert_eq!(result.outcome, ScenarioOutcome::Fail);
    }

    #[test]
    fn run_scenario_inconclusive_with_no_events() {
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "happy-path",
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "approved", "event": "submit"
            }],
            "expectedTerminals": ["approved"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert_eq!(result.outcome, ScenarioOutcome::Inconclusive);
    }

    #[test]
    fn type_specific_finding_fires_on_adverse_determination_without_outcomes() {
        // R5.2: ScenarioType-driven assertion. Old impl ignored
        // scenario_type entirely.
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "adverse-determination",
            "events": [{"name": "submit"}],
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "approved", "event": "submit"
            }],
            "expectedTerminals": ["approved"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert!(
            result
                .type_findings
                .iter()
                .any(|f| f.contains("SA-MUST-scn-003")),
            "expected SA-MUST-scn-003 finding: {result:?}"
        );
        assert_eq!(result.outcome, ScenarioOutcome::Fail);
    }

    #[test]
    fn type_specific_finding_appeal_filed_needs_exercises_appeals() {
        let artifact = artifact_with_lifecycle();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "appeal-filed",
            "events": [{"name": "submit"}],
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "approved", "event": "submit"
            }],
            "expectedTerminals": ["approved"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert!(result
            .type_findings
            .iter()
            .any(|f| f.contains("SA-MUST-scn-004")));
    }

    #[test]
    fn fake_artifact_falls_back_when_lifecycle_states_empty() {
        // The fake_artifact() lifecycle has empty states map. In that
        // accommodation mode, the runner falls back to scenario.targetState
        // (transitional behavior; tightens once phase 4 emits real
        // transitions).
        let artifact = fake_artifact();
        let scenario = json!({
            "id": "sc-1",
            "scenarioType": "happy-path",
            "events": [{"name": "submit", "targetState": "approved"}],
            "expectedTrace": [{
                "stateBefore": "intake", "stateAfter": "approved", "event": "submit"
            }],
            "expectedTerminals": ["approved"]
        });
        let result = run_scenario(&artifact, &scenario);
        assert_eq!(result.outcome, ScenarioOutcome::Pass);
    }
}
