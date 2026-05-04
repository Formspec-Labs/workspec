// Rust guideline compliant 2026-05-02

//! Top-level pipeline — runs all eight phases in order.

use std::path::Path;

use crate::artifact::CompileArtifact;
use crate::error::{CompileError, Disposition, FailureKind};
use crate::events::EventBuffer;
use crate::{
    phase1_load, phase2_mapping, phase3_workflow, phase4_emit, phase5_scenarios,
    phase6_readiness, phase7_gates, phase8_package, phase9_export,
};
use serde_json::Value;
use wos_studio_lint::{LintSeverity, Workspace};

/// Run a phase: emit `phase-started`, run, emit `phase-completed` (or
/// `phase-halted` on error). Used by the compile loop so each phase
/// transition surfaces in the event stream.
fn run_phase<F, T>(
    events: &mut EventBuffer,
    phase: u8,
    body: F,
) -> Result<T, CompileError>
where
    F: FnOnce() -> Result<T, CompileError>,
{
    events.phase_started(phase);
    match body() {
        Ok(result) => {
            events.phase_completed(phase);
            Ok(result)
        }
        Err(err) => {
            if let CompileError::Halt { kind, message, .. } = &err {
                events.phase_halted(phase, kind.as_str(), message);
            } else {
                events.phase_halted(phase, "io-or-json-error", &err.to_string());
            }
            Err(err)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompileOptions {
    /// If false, phase-6 readiness errors do not halt the compile (they
    /// flow through to release notes instead).
    pub halt_on_readiness_error: bool,
    /// If false, the three external gates skip (used for pre-validate
    /// dry-runs where gate failures are surfaced rather than blocking).
    pub run_external_gates: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            halt_on_readiness_error: true,
            run_external_gates: true,
        }
    }
}

/// Compile a workspace already loaded into memory.
pub fn compile(
    ws: &Workspace,
    options: CompileOptions,
) -> Result<CompileArtifact, CompileError> {
    let mut events = EventBuffer::new();

    let result = compile_inner(ws, options, &mut events);
    match &result {
        Ok(artifact) => events.compile_succeeded(&artifact.manifest.manifest_hash),
        Err(err) => events.compile_failed(&err.to_string()),
    }
    let mut artifact = result?;
    artifact.events = events;
    Ok(artifact)
}

fn compile_inner(
    ws: &Workspace,
    options: CompileOptions,
    events: &mut EventBuffer,
) -> Result<CompileArtifact, CompileError> {
    // Phase 1
    let load_result = run_phase(events, 1, || phase1_load::run(ws))?;

    // Phase 2
    let mapping_result = run_phase(events, 2, || {
        phase2_mapping::run(ws, &load_result.referenced_policy_objects)
    })?;

    // Phase 3
    let workflow_result = run_phase(events, 3, || {
        phase3_workflow::run(load_result.workflow_intent)
    })?;

    // Phase 4
    let emit_result = run_phase(events, 4, || {
        phase4_emit::run(ws, &workflow_result, &mapping_result)
    })?;

    // Phase 5 (infallible — phase5_scenarios::run returns Vec, not Result)
    events.phase_started(5);
    let emitted_scenarios = phase5_scenarios::run(ws);
    events.phase_completed(5);

    // Phase 6
    let mut readiness_result = run_phase(events, 6, || {
        phase6_readiness::run(ws, options.halt_on_readiness_error)
    })?;

    // Phase 7 — three external gates. Per `SA-MUST-cmp-031`, gate
    // failures lift into tier-S6 findings AND halt the pipeline.
    // halt_on_gate_failure mirrors halt_on_readiness_error: when
    // false, gates still run and findings flow to the artifact, but
    // the pipeline does not halt — caller inspects via
    // CompileArtifact.disposition + readiness_findings.
    if options.run_external_gates {
        events.phase_started(7);
        match phase7_gates::run(&emit_result.wos_workflow, &emitted_scenarios) {
            Ok(gates) => {
                // Emit gate-passed events for each gate that returned Pass.
                events.gate(
                    "schema-pass",
                    gates.schema_pass.is_pass(),
                    gates.schema_pass.findings.len(),
                );
                events.gate(
                    "lint-pass",
                    gates.lint_pass.is_pass(),
                    gates.lint_pass.findings.len(),
                );
                events.gate(
                    "conformance-pass",
                    gates.conformance_pass.is_pass(),
                    gates.conformance_pass.findings.len(),
                );
                events.phase_completed(7);
                // Lift the conformance-pass stub status (a Pass with
                // findings) into a PUB-LINT-007 info-tier finding so
                // downstream tooling sees it.
                if !gates.conformance_pass.findings.is_empty()
                    && gates.conformance_pass.is_pass()
                {
                    readiness_result.diagnostics.push(
                        wos_studio_lint::LintDiagnostic {
                            rule_id: "PUB-LINT-007",
                            severity: wos_studio_lint::LintSeverity::Info,
                            tier: wos_studio_lint::Tier::T1,
                            path: "/gates/conformance-pass".to_string(),
                            message: gates.conformance_pass.findings.join("; "),
                            suggested_fix: None,
                            related_docs: Vec::new(),
                            source: None,
                        },
                    );
                }
            }
            Err(err) => {
                // Convert gate failure into a tier-S6 finding before
                // re-raising, so callers that catch the error still
                // see structured diagnostics in the readiness stream.
                if let CompileError::Halt {
                    kind, message, details, ..
                } = &err
                {
                    let rule_id = match kind {
                        FailureKind::SchemaPassFailed => "PUB-LINT-003",
                        FailureKind::LintPassFailed => "PUB-LINT-004",
                        FailureKind::ConformancePassFailed => "PUB-LINT-007",
                        _ => "PUB-LINT-001",
                    };
                    readiness_result.diagnostics.push(
                        wos_studio_lint::LintDiagnostic {
                            rule_id,
                            severity: wos_studio_lint::LintSeverity::Block,
                            tier: wos_studio_lint::Tier::T1,
                            path: format!("/gates/{}", kind.as_str()),
                            message: format!("{message}: {}", details.join("; ")),
                            suggested_fix: None,
                            related_docs: Vec::new(),
                            source: None,
                        },
                    );
                    let gate_name = match kind {
                        FailureKind::SchemaPassFailed => "schema-pass",
                        FailureKind::LintPassFailed => "lint-pass",
                        FailureKind::ConformancePassFailed => "conformance-pass",
                        _ => "unknown",
                    };
                    events.gate(gate_name, false, details.len());
                    events.phase_halted(7, kind.as_str(), message);
                }
                return Err(err);
            }
        }
    }

    // Phase 8
    let package = run_phase(events, 8, || {
        Ok::<_, CompileError>(phase8_package::run(
            ws,
            load_result.workflow_intent,
            &mapping_result,
            &emitted_scenarios,
            emit_result.embedded_blocks_emitted,
            &load_result.referenced_policy_objects,
            &readiness_result.diagnostics,
        ))
    })?;

    let disposition = derive_disposition(&mapping_result, &readiness_result.diagnostics);

    // Phase 9: workspace export bundle per SA-MUST-cmp-060..063.
    let export_result = phase9_export::run(
        ws,
        &emitted_scenarios,
        &package.manifest,
        &load_result.referenced_policy_objects,
        &package.manifest.mappings_consumed,
    );

    Ok(CompileArtifact {
        wos_workflow: emit_result.wos_workflow,
        scenarios: emitted_scenarios,
        approval_package: package.approval_package,
        release_notes: package.release_notes,
        manifest: package.manifest,
        disposition,
        readiness_findings: readiness_result.diagnostics,
        events: EventBuffer::new(), // overwritten by compile() outer
        export_bundle: export_result.bundle,
    })
}

/// Compute the artifact's `Disposition` per `SA-MUST-cmp-040..043`.
///
/// - `EmitWithBlockers` if any readiness diagnostic is `LintSeverity::Error`
///   (only reachable when `halt_on_readiness_error: false`; the default
///   path halts before reaching this point).
/// - `EmitWithWarnings` if `unmappedButApproved` mappings exist OR if
///   any readiness diagnostic is `Warning`.
/// - `Compiled` otherwise.
fn derive_disposition(
    mapping: &phase2_mapping::MappingResult<'_>,
    diagnostics: &[wos_studio_lint::LintDiagnostic],
) -> Disposition {
    if diagnostics
        .iter()
        .any(|d| matches!(d.severity, LintSeverity::Error))
    {
        return Disposition::EmitWithBlockers;
    }
    let has_unmapped = mapping.by_subject.values().any(|m| {
        m.get("mappingState").and_then(Value::as_str)
            == Some("unmappedButApproved")
    });
    let has_warning = diagnostics
        .iter()
        .any(|d| matches!(d.severity, LintSeverity::Warning));
    if has_unmapped || has_warning {
        Disposition::EmitWithWarnings
    } else {
        Disposition::Compiled
    }
}

/// Compile a workspace loaded from a directory.
pub fn compile_workspace(
    dir: &Path,
    options: CompileOptions,
) -> Result<CompileArtifact, CompileError> {
    let ws = crate::load_workspace_from_dir(dir)?;
    compile(&ws, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    fn minimal_clean_workspace() -> Workspace {
        // Note (F4.1): the compiled artifact must pass full
        // Draft-2020-12 schema-pass. The schema requires:
        //   - `version` follows SemVer X.Y.Z
        //   - `actors` array contains ≥1 entry
        //   - top-level keys are limited per the kernel envelope
        //     (no `id` at the workflow root)
        ws_from(vec![
            (
                "wfi.json",
                json!({
                    "$wosStudioWorkflowIntent": "1.0",
                    "id": "wfi-1",
                    "workspaceId": "ws-1",
                    "version": "0.1.0",
                    "title": "Demo workflow",
                    "impactLevel": "operational",
                    "publicationUrl": "https://example.org/demo",
                    "actors": [
                        {"id": "system", "type": "system"}
                    ],
                    "elements": [
                        {"id": "intake", "kind": "step",
                         "policyObjectRefs": ["pol-x"],
                         "bridge": {"kernelKind": "transition"},
                         "derivedFrom": ["pol-x"]}
                    ]
                }),
            ),
            (
                "po.json",
                json!({
                    "$wosStudioPolicyObject": "1.0",
                    "policyObjects": [
                        {
                            "id": "pol-x", "workspaceId": "ws-1",
                            "kind": "DecisionRule", "lifecycleState": "approved",
                            "originClass": "source",
                            "citations": [{"sourceCitationRef": "c-1"}]
                        },
                        {
                            "id": "pol-auth-system", "workspaceId": "ws-1",
                            "kind": "AuthorityGrant", "lifecycleState": "approved",
                            "originClass": "source",
                            "actor": "system",
                            "citations": [{"sourceCitationRef": "c-1"}]
                        }
                    ]
                }),
            ),
            (
                "map.json",
                json!({
                    "$wosStudioMapping": "1.0",
                    "mappings": [{
                        "id": "m-1", "policyObjectRef": "pol-x",
                        "mappingState": "mapsToWos",
                        "targets": [{
                            "wosConceptId": "DecisionRule",
                            "wosJsonPath": "$.governance.policyObjects[0]"
                        }]
                    }]
                }),
            ),
            (
                "ws.json",
                json!({
                    "$wosStudioWorkspace": "1.0",
                    "id": "ws-1", "title": "Test",
                    "reviewerRoles": []
                }),
            ),
        ])
    }

    #[test]
    fn compiles_minimal_workspace_end_to_end() {
        let ws = minimal_clean_workspace();
        // Use the dry-run shape so readiness + external gates don't
        // halt on minor fixture gaps that aren't relevant to the
        // pipeline-shape assertions below. The pipeline still runs
        // every phase 1-9; only the publication gates relax.
        let options = CompileOptions {
            halt_on_readiness_error: false,
            run_external_gates: false,
        };
        let artifact = compile(&ws, options).expect("clean compile");
        // Top-level `id` is no longer emitted (per ADR-0076 the
        // envelope's identity is `url + version`); the manifest
        // carries the intent id instead.
        assert_eq!(artifact.wos_workflow["$wosWorkflow"], json!("1.0"));
        assert_eq!(artifact.wos_workflow["version"], json!("0.1.0"));
        assert!(
            artifact.wos_workflow.get("governance").is_some(),
            "governance block should emit"
        );
        assert_eq!(artifact.manifest.workflow_intent_id, "wfi-1");
        assert!(
            !artifact.manifest.policy_objects_consumed.is_empty(),
            "manifest should record consumed policy objects"
        );
    }

    #[test]
    fn deterministic_compile_byte_identical_in_process_repeats() {
        // SA-MUST-cmp-001: identical input → byte-identical output.
        // In-process repetition catches HashSet `RandomState` jitter
        // within one process; the cross-process variant lives in the
        // determinism integration test (`tests/determinism.rs`).
        let ws = minimal_clean_workspace();
        // See CompileOptions doc; we disable readiness halt + gates here
        // to isolate the determinism assertion from rule churn.
        let options = CompileOptions {
            halt_on_readiness_error: false,
            run_external_gates: false,
        };
        let mut hashes: Vec<(String, String, String, String)> = Vec::new();
        for _ in 0..10 {
            let artifact = compile(&ws, options).expect("ok");
            hashes.push((
                serde_json::to_string_pretty(&artifact.wos_workflow).unwrap(),
                // manifest_hash MUST be stable (SA-MUST-cmp-073).
                artifact.manifest.manifest_hash.clone(),
                serde_json::to_string_pretty(&artifact.approval_package).unwrap(),
                serde_json::to_string_pretty(&artifact.scenarios).unwrap(),
            ));
        }
        let baseline = &hashes[0];
        for (i, h) in hashes.iter().enumerate().skip(1) {
            assert_eq!(h.0, baseline.0, "iteration {i}: wos_workflow drift");
            assert_eq!(h.1, baseline.1, "iteration {i}: manifest_hash drift");
            assert_eq!(h.2, baseline.2, "iteration {i}: approval_package drift");
            assert_eq!(h.3, baseline.3, "iteration {i}: scenarios drift");
        }
        assert!(
            baseline.1.starts_with("sha256:"),
            "manifest_hash format: {}",
            baseline.1
        );
    }

    #[test]
    fn pipeline_emits_phase_lifecycle_events() {
        let ws = minimal_clean_workspace();
        // Same flags as compiles_minimal_workspace_end_to_end — the
        // event-emission contract is what we're testing, not
        // publication readiness.
        let options = CompileOptions {
            halt_on_readiness_error: false,
            run_external_gates: false,
        };
        let artifact = compile(&ws, options).expect("ok");
        let kinds: Vec<&str> = artifact
            .events
            .events()
            .iter()
            .map(|e| e.kind.as_str())
            .collect();
        // Each phase emits one started + one completed event.
        // Phase 7 (external gates) is skipped because options
        // disables them — only phases 1-6 + 8 emit.
        let started: Vec<u8> = artifact
            .events
            .events()
            .iter()
            .filter(|e| e.kind == "wos.compiler.phase-started")
            .map(|e| e.phase)
            .collect();
        assert_eq!(started, vec![1, 2, 3, 4, 5, 6, 8]);
        // Final event is compile-succeeded.
        assert_eq!(
            artifact.events.events().last().unwrap().kind,
            "wos.compiler.compile-succeeded"
        );
        // Sequence numbers are dense from 0.
        for (i, e) in artifact.events.events().iter().enumerate() {
            assert_eq!(e.sequence, i as u32);
        }
        // gate events: 0 when run_external_gates is false (this
        // test path), 3 when they're enabled.
        let gates: Vec<&str> = kinds
            .iter()
            .filter(|k| k.starts_with("wos.compiler.gate"))
            .copied()
            .collect();
        assert_eq!(gates.len(), 0, "external gates disabled in this test");
    }

    #[test]
    fn halts_on_unapproved_input() {
        let ws = ws_from(vec![
            (
                "wfi.json",
                json!({
                    "$wosStudioWorkflowIntent": "1.0",
                    "id": "wfi-1", "workspaceId": "ws-1",
                    "version": "0.1.0", "title": "T",
                    "impactLevel": "operational",
                    "publicationUrl": "https://example.org/x",
                    "actors": [],
                    "elements": [{"id": "s", "kind": "step",
                                  "policyObjectRefs": ["pol-x"],
                                  "bridge": {"kernelKind": "transition"}}]
                }),
            ),
            (
                "po.json",
                json!({
                    "$wosStudioPolicyObject": "1.0",
                    "policyObjects": [{"id": "pol-x", "lifecycleState": "draft"}]
                }),
            ),
        ]);
        let err = compile(&ws, CompileOptions::default()).expect_err("halt");
        assert!(matches!(
            err,
            CompileError::Halt {
                kind: crate::error::FailureKind::UnapprovedInput,
                phase: 1,
                ..
            }
        ));
    }
}
