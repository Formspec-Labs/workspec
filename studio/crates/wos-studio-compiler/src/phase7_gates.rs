// Rust guideline compliant 2026-05-02

//! Phase 7 — Three external gates per `SA-MUST-cmp-030`.
//!
//! Run in order: schema-pass → lint-pass → conformance-pass. Subsequent
//! gates MUST NOT run if a prior gate fails.
//!
//! - **schema-pass:** focused validator that catches the five
//!   structural invariants the spec relies on (required fields,
//!   `lifecycle.initialState` keyed in `lifecycle.states`,
//!   `actors[*].id` uniqueness, `impactLevel` enum membership,
//!   ADR-0076 conditional-block invariants for `rights-impacting` and
//!   `safety-impacting` workflows). NOT a full Draft-2020-12 validator
//!   — `jsonschema` v0.18 doesn't support 2020-12 yet. Full validation
//!   continues to run via the parent's Python pytest harness against
//!   the schema file.
//! - **lint-pass:** runs `wos_lint::studio_api::lint_workflow_with_project`
//!   (F4.2, 2026-05-02). The compiled workflow JSON is wrapped in an
//!   in-memory `WosProject` and BOTH Tier-1 (single-document) AND
//!   Tier-2 (cross-document resolution / FEL AST analysis) rules
//!   apply. Earlier behavior (T1-only via `lint_workflow`) is kept on
//!   the parent surface for backward compatibility but no longer
//!   used by this gate.
//! - **conformance-pass:** stub-honest — surfaces "stub" status with a
//!   pre-replay "expectations present" check. Real replay lives in
//!   `wos_studio_scenario::run_workspace`. The compiler's gate stays
//!   honest: stub-pass is still pass, but the result records the
//!   stub status in its findings so downstream tooling can distinguish.

use serde_json::Value;

use crate::artifact::EmittedScenario;
use crate::error::{CompileError, FailureKind};
use crate::gates::{ExternalGate, GateResult};
use wos_lint::studio_api;

pub struct GatesResult {
    pub schema_pass: GateResult,
    pub lint_pass: GateResult,
    pub conformance_pass: GateResult,
}

impl GatesResult {
    pub fn all_pass(&self) -> bool {
        self.schema_pass.is_pass()
            && self.lint_pass.is_pass()
            && self.conformance_pass.is_pass()
    }
}

pub fn run(
    wos_workflow: &Value,
    scenarios: &[EmittedScenario],
) -> Result<GatesResult, CompileError> {
    // Gate 1: schema-pass.
    let schema_pass = run_schema_pass(wos_workflow);
    if !schema_pass.is_pass() {
        let detail = schema_pass.findings.clone();
        return Err(CompileError::halt_with(
            7,
            FailureKind::SchemaPassFailed,
            "schema-pass external gate failed",
            detail,
        ));
    }

    // Gate 2: lint-pass.
    let lint_pass = run_lint_pass(wos_workflow);
    if !lint_pass.is_pass() {
        let detail = lint_pass.findings.clone();
        return Err(CompileError::halt_with(
            7,
            FailureKind::LintPassFailed,
            "lint-pass external gate failed",
            detail,
        ));
    }

    // Gate 3: conformance-pass — stub-honest.
    let conformance_pass = run_conformance_pass(scenarios);
    Ok(GatesResult {
        schema_pass,
        lint_pass,
        conformance_pass,
    })
}

fn run_schema_pass(workflow: &Value) -> GateResult {
    // F4.1 (2026-05-02): swapped from a 5-invariant focused validator to
    // full Draft 2020-12 validation via `boon`. The schema's 66
    // conditional rules (`if/then`/`unevaluatedProperties`/
    // `dependentSchemas`/`allOf`) all apply now, including the ADR-0076
    // governance/signature/custody requirements and the F1.* extension
    // shapes. The validator is wrapped in `crate::schema_validator` so
    // a future swap is one file.
    let findings = crate::schema_validator::validate(workflow);
    if findings.is_empty() {
        GateResult::pass(ExternalGate::SchemaPass)
    } else {
        GateResult::fail(ExternalGate::SchemaPass, findings)
    }
}

fn run_lint_pass(workflow: &Value) -> GateResult {
    let json = match serde_json::to_string(workflow) {
        Ok(s) => s,
        Err(e) => {
            return GateResult::fail(
                ExternalGate::LintPass,
                vec![format!("serialization error: {e}")],
            );
        }
    };
    match studio_api::lint_workflow_with_project(&json) {
        Ok(diagnostics) => {
            let mut blocking: Vec<String> = diagnostics
                .iter()
                .filter(|d| {
                    matches!(
                        d.severity,
                        studio_api::LintSeverity::Error | studio_api::LintSeverity::Block
                    )
                })
                .map(|d| format!("{}: {} ({})", d.rule_id, d.message, d.path))
                .collect();
            blocking.sort();
            if blocking.is_empty() {
                GateResult::pass(ExternalGate::LintPass)
            } else {
                GateResult::fail(ExternalGate::LintPass, blocking)
            }
        }
        Err(e) => GateResult::fail(
            ExternalGate::LintPass,
            vec![format!("lint pipeline error: {e}")],
        ),
    }
}

fn run_conformance_pass(scenarios: &[EmittedScenario]) -> GateResult {
    // Stub-honest: surface the gate as pass with a clear "stub" sentinel
    // so downstream tooling can distinguish stub-pass from real-pass.
    // Real replay lives in wos_studio_scenario::run_workspace which
    // does NOT run from this gate (running it would invert the dep
    // graph: compiler depending on simulator).
    let mut missing_expectations: Vec<String> = scenarios
        .iter()
        .filter(|s| {
            s.body.get("expectedOutcome").is_none()
                && s.body.get("expectedTrace").is_none()
                && s.body.get("expectedTerminals").is_none()
        })
        .map(|s| {
            format!(
                "scenario '{}' has no expectedOutcome/expectedTrace block",
                s.id
            )
        })
        .collect();
    if !missing_expectations.is_empty() {
        missing_expectations.sort();
        // These are real failures (scenarios without expectations
        // can't ever match an actual trace). Fail the gate.
        return GateResult::fail(
            ExternalGate::ConformancePass,
            missing_expectations,
        );
    }
    // Pre-replay checks pass; the gate itself is stub. Surface that
    // status as a finding so the manifest records it.
    let mut result = GateResult::pass(ExternalGate::ConformancePass);
    result.findings.push(
        "status: stub — real conformance replay lives in \
         wos_studio_scenario::run_workspace; this gate validates only \
         that scenarios carry an expectedOutcome / expectedTrace block"
            .to_string(),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_valid_envelope() -> Value {
        // Updated 2026-05-02 (F4.1): the full Draft 2020-12 validator
        // is stricter than the previous focused validator. The State
        // type discriminator is `type`, not `kind` (Rust-side
        // #[serde(rename = "type")]); version follows SemVer; the
        // schema's `additionalProperties: false` rejects unmodelled
        // top-level keys.
        json!({
            "$wosWorkflow": "1.0",
            "url": "https://example.org/wf-1",
            "version": "1.0.0",
            "title": "Test",
            "impactLevel": "operational",
            "actors": [
                {"id": "actor-1", "type": "human", "name": "Test actor"}
            ],
            "lifecycle": {
                "initialState": "intake",
                "states": {
                    "intake": {"type": "atomic", "transitions": []}
                }
            }
        })
    }

    #[test]
    fn schema_pass_passes_minimal_valid_envelope() {
        let result = run_schema_pass(&minimal_valid_envelope());
        assert!(result.is_pass(), "{result:?}");
    }

    #[test]
    fn schema_pass_fires_on_empty_envelope() {
        let result = run_schema_pass(&json!({}));
        assert!(!result.is_pass());
        // Empty envelope misses every required top-level field.
        // boon surfaces one finding per missing required key plus the
        // root-level "required" finding; the exact count depends on
        // boon's error-tree shape but MUST be > 1.
        assert!(result.findings.len() > 1, "{:?}", result.findings);
    }

    #[test]
    fn schema_pass_silent_on_unknown_initial_state_lint_catches_it() {
        // JSON Schema Draft 2020-12 cannot natively express
        // "lifecycle.initialState ∈ lifecycle.states.keys()" — there is
        // no value-references-key keyword. Parent-tier lint rule K-016
        // catches the case (see `lint_pass_xref.rs::
        // lint_catches_unknown_initial_state`). This test asserts the
        // schema-pass remains silent so the layered-defense contract
        // stays observable: schema does NOT catch, lint DOES catch.
        // (DEFER-003 Tranche B closed via lint K-016, 2026-05-03.)
        let mut wf = minimal_valid_envelope();
        wf["lifecycle"]["initialState"] = json!("nonexistent");
        let result = run_schema_pass(&wf);
        assert!(
            result.is_pass(),
            "schema-pass passes for unknown initialState by design (lint K-016 catches): {result:?}"
        );
    }

    #[test]
    fn schema_pass_silent_on_actor_id_collision_lint_catches_it() {
        // JSON Schema Draft 2020-12 has no native "uniqueItems by
        // property" keyword; a schema-side catch would require
        // reshaping `actors: Array<Actor>` → `actors: Map<id, Actor>`
        // so JSON object key uniqueness enforces the invariant. That
        // reshape costs ~225 consumer migrations across the workspace
        // (typed `KernelDocument::actors: Vec<Actor>`, every
        // `actors[i]` index, every `"actors": [...]` test fixture and
        // sample workflow) for redundant coverage with parent-tier
        // lint rule K-009. (STUDIO-DEFER-003 Tranche C closed via
        // "lint-covered, no schema action" 2026-05-03.)
        //
        // This test asserts the schema-pass remains silent so the
        // layered-defense contract stays observable: schema does NOT
        // catch, lint K-009 DOES catch (see
        // `lint_pass_xref.rs::lint_catches_actor_id_collision`).
        let mut wf = minimal_valid_envelope();
        wf["actors"] = json!([
            {"id": "a1", "type": "human", "name": "X"},
            {"id": "a1", "type": "human", "name": "Y"}
        ]);
        let result = run_schema_pass(&wf);
        assert!(
            result.is_pass(),
            "schema-pass passes for duplicate actor ids by design (lint K-009 catches): {result:?}"
        );
    }

    #[test]
    fn schema_pass_fires_on_impact_level_outside_enum() {
        let mut wf = minimal_valid_envelope();
        wf["impactLevel"] = json!("imaginary-level");
        let result = run_schema_pass(&wf);
        assert!(!result.is_pass());
        // boon's enum-failure message format mentions the enum
        // keyword and lists the value; we just assert non-empty
        // findings reference impactLevel.
        assert!(
            result.findings.iter().any(|f| f.contains("impactLevel"))
                || result.findings.iter().any(|f| f.contains("enum")),
            "{:?}",
            result.findings
        );
    }

    #[test]
    fn schema_pass_fires_on_rights_impacting_without_governance() {
        let mut wf = minimal_valid_envelope();
        wf["impactLevel"] = json!("rights-impacting");
        let result = run_schema_pass(&wf);
        assert!(!result.is_pass());
        // The ADR-0076 + F1.6 conditionals require governance,
        // signature, and custody for rights-impacting workflows.
        // boon surfaces each missing-required as a separate finding;
        // at least one of the three MUST appear.
        assert!(
            result.findings.iter().any(|f| {
                f.contains("governance") || f.contains("signature") || f.contains("custody")
            }),
            "{:?}",
            result.findings
        );
    }

    #[test]
    fn conformance_pass_in_stub_mode_passes_when_expectations_present() {
        let scenarios = vec![EmittedScenario {
            id: "s1".to_string(),
            scenario_type: None,
            status: Some("expected".to_string()),
            body: json!({"expectedOutcome": "approved"}),
        }];
        let result = run_conformance_pass(&scenarios);
        assert!(result.is_pass());
        // Gate still surfaces stub status in findings.
        assert!(result.findings.iter().any(|f| f.starts_with("status: stub")));
    }

    #[test]
    fn conformance_pass_fails_when_scenario_lacks_expectations() {
        let scenarios = vec![EmittedScenario {
            id: "s1".to_string(),
            scenario_type: None,
            status: Some("expected".to_string()),
            body: json!({}),
        }];
        let result = run_conformance_pass(&scenarios);
        assert!(!result.is_pass());
    }
}
