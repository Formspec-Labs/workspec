// Rust guideline compliant 2026-05-02

//! Phase 8 — Produce review package: ApprovalPackage + manifest +
//! release notes.
//!
//! `SA-MUST-cmp-005..006`: emitted blocks are recorded in the manifest
//! for audit. `SA-MUST-cmp-023`: unmappedButApproved mappings flow into
//! release notes.

use serde_json::Value;

use crate::artifact::{ApprovalPackage, EmittedScenario};
use crate::manifest::CompileManifest;
use crate::phase2_mapping::MappingResult;
use wos_studio_lint::{LintDiagnostic, Workspace, WorkspaceDocument};

pub struct PackageResult {
    pub approval_package: ApprovalPackage,
    pub manifest: CompileManifest,
    pub release_notes: Option<String>,
}

pub fn run(
    ws: &Workspace,
    workflow_intent: &WorkspaceDocument,
    mapping: &MappingResult<'_>,
    scenarios: &[EmittedScenario],
    embedded_blocks_emitted: Vec<String>,
    referenced_policy_objects: &[String],
    readiness_findings: &[LintDiagnostic],
) -> PackageResult {
    // workspaceId / version have no typed accessor on
    // WorkflowIntentDocument today; route through `.document.body()`
    // (typed-dispatch into per-variant body) rather than `.raw`. If a
    // dedicated accessor lands later, migrate again.
    let workspace_id = workflow_intent
        .document
        .body()
        .get("workspaceId")
        .and_then(Value::as_str)
        .unwrap_or("ws-?")
        .to_string();
    let intent_id = workflow_intent.id().unwrap_or("wfi-?").to_string();
    let intent_version = workflow_intent
        .document
        .body()
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.1.0")
        .to_string();
    // Use the typed accessor when present (string form). The
    // typed-object form (`wos_version_pin_typed`) is not yet a
    // compiler concern; falls through to None as the prior `.raw.get`
    // also did for object values.
    let pin = workflow_intent
        .document
        .body()
        .get("wosVersionPin")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut manifest = CompileManifest::empty(workspace_id.clone(), intent_id.clone());
    manifest.workflow_intent_version = intent_version.clone();
    manifest.wos_version_pin = pin;
    manifest.embedded_blocks_emitted = embedded_blocks_emitted;

    // Record consumed inputs (sorted). Per `SA-MUST-cmp-050`, list only
    // PolicyObjects actually referenced by the WorkflowIntent — not the
    // entire workspace catalog.
    let mut consumed: Vec<String> = referenced_policy_objects.to_vec();
    consumed.sort();
    consumed.dedup();
    manifest.policy_objects_consumed = consumed;
    manifest.mappings_consumed = mapping
        .by_subject
        .values()
        .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    manifest.bindings_consumed = sorted_ids(ws, |d| {
        matches!(d.marker, wos_studio_model::StudioMarker::Binding)
    });
    manifest.scenarios_consumed =
        scenarios.iter().map(|s| s.id.clone()).collect();
    manifest.scenarios_consumed.sort();
    manifest.source_versions_consumed = ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Source))
        .flat_map(|d| {
            d.source_versions()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Approval package — collect ApprovalDecisions referencing this workflow.
    let approvals: Vec<Value> = ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Approval))
        .map(|d| d.raw.clone())
        .collect();
    // Manifest hash MUST be computed before binding into the ApprovalPackage.
    // Fields can still be appended above this point; once compute_hash runs
    // the hash reflects the final manifest content.
    manifest.compute_hash();
    let approval_package = ApprovalPackage {
        workflow_intent_id: intent_id,
        workflow_intent_version: intent_version,
        approvals,
        compliance_attestations: Vec::new(),
        bound_manifest_hash: manifest.manifest_hash.clone(),
        extensions: Default::default(),
    };

    // Release notes — list unmappedButApproved mappings + readiness warnings.
    let mut notes = String::new();
    let unmapped: Vec<&Value> = mapping
        .by_subject
        .values()
        .filter(|m| {
            m.get("mappingState").and_then(Value::as_str)
                == Some("unmappedButApproved")
        })
        .copied()
        .collect();
    if !unmapped.is_empty() {
        notes.push_str("## Unmapped-but-approved\n\n");
        for m in unmapped {
            let id = m.get("id").and_then(Value::as_str).unwrap_or("?");
            let rationale = m
                .get("unmappedRationale")
                .and_then(Value::as_str)
                .unwrap_or("(no rationale)");
            notes.push_str(&format!("- `{id}` — {rationale}\n"));
        }
        notes.push('\n');
    }
    let warnings: Vec<&LintDiagnostic> = readiness_findings
        .iter()
        .filter(|d| matches!(d.severity, wos_studio_lint::LintSeverity::Warning))
        .collect();
    if !warnings.is_empty() {
        notes.push_str("## Readiness warnings\n\n");
        for d in warnings {
            notes.push_str(&format!("- `{}` — {}\n", d.rule_id, d.message));
        }
    }
    let release_notes = if notes.is_empty() { None } else { Some(notes) };

    PackageResult {
        approval_package,
        manifest,
        release_notes,
    }
}

fn sorted_ids<P>(ws: &Workspace, predicate: P) -> Vec<String>
where
    P: Fn(&WorkspaceDocument) -> bool,
{
    let mut ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for d in &ws.documents {
        if !predicate(d) {
            continue;
        }
        if let Some(id) = d.id() {
            ids.insert(id.to_string());
        }
        // Collection-form PolicyObject wrappers carry per-child `id`s
        // under `body.policyObjects[]`; route through `.document.body()`
        // typed dispatch (no dedicated accessor exists for the wrapper
        // collection today).
        if let Some(arr) = d
            .document
            .body()
            .get("policyObjects")
            .and_then(Value::as_array)
        {
            for v in arr {
                if let Some(id) = v.get("id").and_then(Value::as_str) {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    ids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_studio_model::StudioMarker;

    fn intent_doc() -> WorkspaceDocument {
        let raw = json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1", "workspaceId": "ws-1", "version": "0.2.0"
        });
        WorkspaceDocument {
            path: "wfi.json".to_string(),
            marker: StudioMarker::WorkflowIntent,
            document: serde_json::from_value(raw.clone()).unwrap(),
            raw,
        }
    }

    #[test]
    fn manifest_records_intent_metadata() {
        let ws = Workspace::default();
        let intent = intent_doc();
        let mapping = MappingResult {
            by_subject: std::collections::BTreeMap::new(),
        };
        let result = run(&ws, &intent, &mapping, &[], vec![], &[], &[]);
        assert_eq!(result.manifest.workflow_intent_id, "wfi-1");
        assert_eq!(result.manifest.workflow_intent_version, "0.2.0");
    }
}
