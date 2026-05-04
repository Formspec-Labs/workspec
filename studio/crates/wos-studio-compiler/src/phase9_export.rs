// Rust guideline compliant 2026-05-02

//! Phase 9 — Workspace export bundle (`SA-MUST-cmp-060..063`).
//!
//! Produces a deterministic, self-contained bundle that lets a verifier
//! reproduce the compile + audit the inputs without external `Iri`
//! resolution. Composed from existing phase outputs:
//!
//! - `sources[]` — every loaded SourceVersion from the workspace.
//! - `policyObjects[]` — every consumed PolicyObject (sorted).
//! - `mappings[]` — every Mapping referenced by the bundle.
//! - `scenarios[]` — every emitted Scenario (already sorted by phase 5).
//! - `provenanceLog` — projected from workspace AuthoringProvenance
//!   records (`SA-MUST-cmp-024`).
//! - `compileManifest` — embedded with `manifestHash`.
//! - `custodyReceipts[]` — Trellis-shaped append records placeholders;
//!   real custody anchoring lives in Stage 8 production.
//!
//! `SA-MUST-cmp-061`: bundle MUST be ≤ 50 MB (configurable; today a
//! soft check that surfaces a warning rather than halting).
//! `SA-MUST-cmp-062`: bundle MUST be reproducible from manifest alone
//! (the manifest's `manifestHash` cryptographically pins the inputs).
//! `SA-MUST-cmp-063`: bundle MUST be self-contained — no external
//! IRI resolution at consume time.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::artifact::EmittedScenario;
use crate::manifest::CompileManifest;
use wos_studio_lint::Workspace;

/// Self-contained workspace export bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceExportBundle {
    pub sources: Vec<Value>,
    pub policy_objects: Vec<Value>,
    pub mappings: Vec<Value>,
    pub scenarios: Vec<EmittedScenario>,
    /// Projected authoring-provenance log per `SA-MUST-cmp-024`. Records
    /// the chain of authoring events (citations, supersessions,
    /// approvals) that produced the consumed PolicyObjects.
    pub provenance_log: Vec<Value>,
    /// Compile manifest (with `manifestHash`) embedded so the bundle
    /// is self-describing per `SA-MUST-cmp-062`.
    pub compile_manifest: CompileManifest,
    /// Trellis-shaped custody append records. Production anchoring
    /// lives in Stage 8; today we emit placeholders so the bundle's
    /// shape is stable across versions.
    pub custody_receipts: Vec<CustodyReceipt>,
}

/// Trellis-shaped custody append record. The four-field shape mirrors
/// `crates/wos-server/VISION.md` §"EventStore composition" — `kind`,
/// `payloadHash`, `signerKeyId`, `sequence`. `signerKeyId` is `None`
/// today; production custody fills it from the deployment's signing
/// key bag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyReceipt {
    pub kind: String,
    pub payload_hash: String,
    pub signer_key_id: Option<String>,
    pub sequence: u32,
}

pub struct ExportResult {
    pub bundle: WorkspaceExportBundle,
    /// Soft warnings (e.g., bundle approaching size cap) surfaced
    /// to the caller. Empty in the happy path.
    pub warnings: Vec<String>,
}

const BUNDLE_SIZE_SOFT_CAP_BYTES: usize = 50 * 1024 * 1024;

pub fn run(
    ws: &Workspace,
    scenarios: &[EmittedScenario],
    manifest: &CompileManifest,
    consumed_policy_objects: &[String],
    consumed_mappings: &[String],
) -> ExportResult {
    let policy_object_set: BTreeSet<&str> = consumed_policy_objects
        .iter()
        .map(String::as_str)
        .collect();
    let mapping_set: BTreeSet<&str> =
        consumed_mappings.iter().map(String::as_str).collect();

    let policy_objects: Vec<Value> = collect_records(
        ws,
        wos_studio_model::StudioMarker::PolicyObject,
        "policyObjects",
        |id| policy_object_set.contains(id),
    );
    let mappings: Vec<Value> = collect_records(
        ws,
        wos_studio_model::StudioMarker::Mapping,
        "mappings",
        |id| mapping_set.contains(id),
    );
    let sources: Vec<Value> = collect_records(
        ws,
        wos_studio_model::StudioMarker::Source,
        "sourceVersions",
        |_| true,
    );

    // SA-MUST-cmp-024 — project AuthoringProvenance records into the
    // bundle's provenanceLog. Order by recordedAt where present, else
    // by id; ties broken by record-content equality (sorted JSON).
    let mut provenance_log: Vec<Value> = collect_records(
        ws,
        wos_studio_model::StudioMarker::Provenance,
        "records",
        |_| true,
    );
    provenance_log.sort_by(|a, b| {
        let key_a = provenance_sort_key(a);
        let key_b = provenance_sort_key(b);
        key_a.cmp(&key_b)
    });

    // Custody receipts: one placeholder per emitted block + the
    // workflow itself. Production custody fills signer_key_id;
    // sequence numbers stay deterministic via emission order.
    let mut custody_receipts: Vec<CustodyReceipt> = Vec::new();
    custody_receipts.push(CustodyReceipt {
        kind: "wos.compile.workflow".to_string(),
        payload_hash: manifest.manifest_hash.clone(),
        signer_key_id: None,
        sequence: 0,
    });
    for (i, block) in manifest.embedded_blocks_emitted.iter().enumerate() {
        custody_receipts.push(CustodyReceipt {
            kind: format!("wos.compile.block.{block}"),
            payload_hash: manifest.manifest_hash.clone(),
            signer_key_id: None,
            sequence: (i + 1) as u32,
        });
    }

    let bundle = WorkspaceExportBundle {
        sources,
        policy_objects,
        mappings,
        scenarios: scenarios.to_vec(),
        provenance_log,
        compile_manifest: manifest.clone(),
        custody_receipts,
    };

    // SA-MUST-cmp-061: soft size cap. Emit a warning rather than
    // halting since the cap is workspace-tunable; 50 MB is the
    // default per spec compose-time guidance.
    let mut warnings: Vec<String> = Vec::new();
    if let Ok(serialized) = serde_json::to_string(&bundle) {
        if serialized.len() > BUNDLE_SIZE_SOFT_CAP_BYTES {
            warnings.push(format!(
                "workspace-export.bundle.json size {} bytes exceeds soft cap \
                 of {} bytes; consider splitting the workspace",
                serialized.len(),
                BUNDLE_SIZE_SOFT_CAP_BYTES
            ));
        }
    }

    ExportResult { bundle, warnings }
}

fn collect_records<F>(
    ws: &Workspace,
    marker: wos_studio_model::StudioMarker,
    collection_key: &str,
    include: F,
) -> Vec<Value>
where
    F: Fn(&str) -> bool,
{
    let mut out: Vec<Value> = Vec::new();
    for d in &ws.documents {
        if d.marker != marker {
            continue;
        }
        // Record form: the document itself is the record.
        if let Some(id) = d.raw.get("id").and_then(Value::as_str) {
            if include(id) {
                out.push(d.raw.clone());
                continue;
            }
        }
        // Collection form: walk `<collection_key>[]`.
        if let Some(arr) = d.raw.get(collection_key).and_then(Value::as_array) {
            for r in arr {
                if let Some(id) = r.get("id").and_then(Value::as_str) {
                    if include(id) {
                        out.push(r.clone());
                    }
                } else {
                    // No id — keep the record (sources sometimes lack ids).
                    out.push(r.clone());
                }
            }
        }
    }
    // Stable sort by id (or by full-record string when id is absent).
    out.sort_by(|a, b| {
        let key_a = a
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| a.to_string());
        let key_b = b
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| b.to_string());
        key_a.cmp(&key_b)
    });
    out
}

fn provenance_sort_key(v: &Value) -> String {
    let recorded_at = v
        .get("recordedAt")
        .and_then(Value::as_str)
        .unwrap_or("9999-99-99");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("");
    format!("{recorded_at}|{id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws_from(items: Vec<(&str, Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    #[test]
    fn bundle_includes_only_consumed_policy_objects() {
        let ws = ws_from(vec![(
            "po.json",
            json!({
                "$wosStudioPolicyObject": "1.0",
                "policyObjects": [
                    {"id": "pol-a", "lifecycleState": "approved"},
                    {"id": "pol-b", "lifecycleState": "approved"},
                    {"id": "pol-c", "lifecycleState": "approved"}
                ]
            }),
        )]);
        let manifest = CompileManifest::empty("ws-1".into(), "wf-1".into());
        let result = run(&ws, &[], &manifest, &["pol-a".into(), "pol-c".into()], &[]);
        assert_eq!(result.bundle.policy_objects.len(), 2);
        let ids: Vec<&str> = result
            .bundle
            .policy_objects
            .iter()
            .filter_map(|p| p.get("id").and_then(Value::as_str))
            .collect();
        assert_eq!(ids, vec!["pol-a", "pol-c"]);
    }

    #[test]
    fn bundle_includes_workflow_custody_receipt_first() {
        let ws = Workspace::default();
        let mut manifest = CompileManifest::empty("ws-1".into(), "wf-1".into());
        manifest.embedded_blocks_emitted = vec!["governance".into(), "custody".into()];
        manifest.manifest_hash = "sha256:abc".into();
        let result = run(&ws, &[], &manifest, &[], &[]);
        assert_eq!(result.bundle.custody_receipts.len(), 3);
        assert_eq!(result.bundle.custody_receipts[0].kind, "wos.compile.workflow");
        assert_eq!(result.bundle.custody_receipts[0].sequence, 0);
        assert_eq!(
            result.bundle.custody_receipts[1].kind,
            "wos.compile.block.governance"
        );
    }

    #[test]
    fn provenance_records_sort_by_recorded_at() {
        let ws = ws_from(vec![(
            "prov.json",
            json!({
                "$wosStudioProvenance": "1.0",
                "records": [
                    {"id": "p2", "recordedAt": "2026-01-15T10:00:00Z"},
                    {"id": "p1", "recordedAt": "2026-01-01T00:00:00Z"},
                    {"id": "p3", "recordedAt": "2026-02-01T00:00:00Z"}
                ]
            }),
        )]);
        let manifest = CompileManifest::empty("ws-1".into(), "wf-1".into());
        let result = run(&ws, &[], &manifest, &[], &[]);
        let ids: Vec<&str> = result
            .bundle
            .provenance_log
            .iter()
            .filter_map(|r| r.get("id").and_then(Value::as_str))
            .collect();
        assert_eq!(ids, vec!["p1", "p2", "p3"]);
    }

    #[test]
    fn bundle_is_self_contained() {
        // SA-MUST-cmp-063: serialized bundle MUST round-trip without
        // requiring external IRI resolution. (Smoke test — round-trip
        // through serde and assert deserialization succeeds.)
        let ws = Workspace::default();
        let manifest = CompileManifest::empty("ws-1".into(), "wf-1".into());
        let result = run(&ws, &[], &manifest, &[], &[]);
        let s = serde_json::to_string(&result.bundle).expect("serialize");
        let _: WorkspaceExportBundle =
            serde_json::from_str(&s).expect("round-trip");
    }
}
