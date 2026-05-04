// Rust guideline compliant 2026-05-02

//! Workspace-tier lint surface.
//!
//! Many readiness rules cross-cut multiple documents — a Mapping
//! references a PolicyObject; a Scenario references an Outcome which
//! lives inside a WorkflowIntent's elements; a Supersession chain spans
//! multiple PolicyObjects. Single-document `lint_document` can't catch
//! these. The [`Workspace`] view is a collection of `StudioDocument`s
//! plus a path-based index so cross-document rules can resolve refs
//! cheaply.

use indexmap::IndexMap;
use serde_json::Value;

use wos_studio_model::{StudioDocument, StudioMarker, classify};

/// A bundle of Studio documents materialising one authoring workspace.
///
/// Constructed from a sequence of (path, JSON) pairs; each JSON document
/// is classified and indexed by id where possible. Documents that carry
/// neither an `id` nor a marker the workspace knows about are silently
/// skipped — they're auxiliary artifacts, not workspace state.
#[derive(Debug, Default)]
pub struct Workspace {
    /// Every parsed document in arrival order. Path is the relative
    /// filesystem path the doc was loaded from (used in diagnostics).
    pub documents: Vec<WorkspaceDocument>,
    /// Index from `id` → position in `documents`. Used for fast
    /// cross-document refs. Some marker types (Source, Mapping in
    /// collection form, etc.) carry multiple ids per file; this map
    /// records the *first* id observed.
    pub by_id: IndexMap<String, usize>,
}

/// One entry in a [`Workspace`].
#[derive(Debug)]
pub struct WorkspaceDocument {
    pub path: String,
    pub marker: StudioMarker,
    pub document: StudioDocument,
    /// Raw JSON for fields the typed model doesn't promote.
    pub raw: Value,
}

impl WorkspaceDocument {
    /// Studio `id` field (most documents carry one). Wraps the
    /// per-variant body access so lint rules don't bypass into
    /// `WorkspaceDocument.raw` for the common case.
    pub fn id(&self) -> Option<&str> {
        self.document.body().get("id").and_then(Value::as_str)
    }

    /// PolicyObject / Approval `kind` discriminator. Returns None when
    /// the document doesn't carry one (e.g., WorkflowIntent).
    pub fn kind(&self) -> Option<&str> {
        self.document.body().get("kind").and_then(Value::as_str)
    }

    /// `lifecycleState` as a raw `&str`. Some document types expose a
    /// typed enum via per-variant accessors; use this when the lint
    /// rule only needs string equality.
    pub fn lifecycle_state_str(&self) -> Option<&str> {
        self.document
            .body()
            .get("lifecycleState")
            .and_then(Value::as_str)
    }

    /// `idpRole` — Approval / IdentitySubject body field.
    pub fn idp_role(&self) -> Option<&str> {
        self.document.body().get("idpRole").and_then(Value::as_str)
    }

    /// `entries[]` — TerminologyMap body collection.
    pub fn entries(&self) -> Option<&Vec<Value>> {
        self.document.body().get("entries").and_then(Value::as_array)
    }

    /// `sourceVersions[]` — Source body collection (collection-form
    /// SourceVault wrapper carries this; single-form Source documents
    /// don't, hence the Option).
    pub fn source_versions(&self) -> Option<&Vec<Value>> {
        self.document
            .body()
            .get("sourceVersions")
            .and_then(Value::as_array)
    }

    /// `elements[]` — WorkflowIntent body collection.
    pub fn elements(&self) -> Option<&Vec<Value>> {
        self.document
            .body()
            .get("elements")
            .and_then(Value::as_array)
    }
}

impl Workspace {
    /// Build a workspace from `(path, raw_json)` pairs. Documents whose
    /// JSON does not parse or carries no `$wosStudio*` marker are
    /// silently skipped.
    pub fn from_iter<I, P>(items: I) -> Self
    where
        I: IntoIterator<Item = (P, String)>,
        P: Into<String>,
    {
        let mut ws = Workspace::default();
        for (path, json) in items {
            let path = path.into();
            let raw: Value = match serde_json::from_str(&json) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(marker) = classify(&raw) else { continue };
            let document: StudioDocument =
                match serde_json::from_value(raw.clone()) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
            let position = ws.documents.len();
            if let Some(id) = raw.get("id").and_then(Value::as_str) {
                ws.by_id.entry(id.to_string()).or_insert(position);
            }
            ws.documents.push(WorkspaceDocument {
                path,
                marker,
                document,
                raw,
            });
        }
        ws
    }

    /// Iterate all PolicyObject documents in the workspace.
    pub fn policy_objects(&self) -> impl Iterator<Item = &WorkspaceDocument> {
        self.documents
            .iter()
            .filter(|d| matches!(d.marker, StudioMarker::PolicyObject))
    }

    /// Iterate every flat PolicyObject record. Studio policy-object
    /// collections may carry a single object OR a `policyObjects[]`
    /// array under a workspaceId wrapper (per the schema's collection
    /// `oneOf`). This iterator yields both shapes uniformly.
    ///
    /// The `&doc.raw` fallback below is one of the two infrastructure
    /// uses of `WorkspaceDocument.raw` that intentionally stay (along
    /// with the analogous fallbacks in mapping/scenario record
    /// iteration) — see DEFERRED.md::STUDIO-DEFER-001 closeout. The
    /// single-object form needs `&Value` shape (with the marker key)
    /// to feed downstream rules; the typed body map omits the marker.
    pub fn policy_object_records(&self) -> Vec<(&WorkspaceDocument, &Value)> {
        let mut out: Vec<(&WorkspaceDocument, &Value)> = Vec::new();
        for doc in self.policy_objects() {
            if let Some(inner) = doc
                .document
                .body()
                .get("policyObjects")
                .and_then(Value::as_array)
            {
                for v in inner {
                    out.push((doc, v));
                }
            } else {
                out.push((doc, &doc.raw));
            }
        }
        out
    }

    /// Iterate every Mapping record (single + collection form). See
    /// `policy_object_records` for the `&doc.raw` fallback note.
    pub fn mapping_records(&self) -> Vec<(&WorkspaceDocument, &Value)> {
        let mut out: Vec<(&WorkspaceDocument, &Value)> = Vec::new();
        for doc in self.documents.iter().filter(|d| matches!(d.marker, StudioMarker::Mapping)) {
            if let Some(inner) = doc.document.body().get("mappings").and_then(Value::as_array) {
                for v in inner {
                    out.push((doc, v));
                }
            } else {
                out.push((doc, &doc.raw));
            }
        }
        out
    }

    /// Iterate every Scenario record (single + collection form). See
    /// `policy_object_records` for the `&doc.raw` fallback note.
    pub fn scenario_records(&self) -> Vec<(&WorkspaceDocument, &Value)> {
        let mut out: Vec<(&WorkspaceDocument, &Value)> = Vec::new();
        for doc in self.documents.iter().filter(|d| matches!(d.marker, StudioMarker::Scenario)) {
            if let Some(inner) = doc.document.body().get("scenarios").and_then(Value::as_array) {
                for v in inner {
                    out.push((doc, v));
                }
            } else {
                out.push((doc, &doc.raw));
            }
        }
        out
    }

    /// First Workflow-intent document, if any.
    pub fn workflow_intent(&self) -> Option<&WorkspaceDocument> {
        self.documents
            .iter()
            .find(|d| matches!(d.marker, StudioMarker::WorkflowIntent))
    }

    /// First Workspace document, if any. Used by WF-LINT-006 to read
    /// `policy.retentionPolicies` defaults during retention-policy
    /// resolution (per ADR-0083 r2).
    pub fn workspace_document(&self) -> Option<&WorkspaceDocument> {
        self.documents
            .iter()
            .find(|d| matches!(d.marker, StudioMarker::Workspace))
    }

    /// Walk every WorkflowElement across all WorkflowIntent documents.
    pub fn workflow_elements(&self) -> Vec<(&WorkspaceDocument, usize, &Value)> {
        let mut out = Vec::new();
        for doc in self.documents.iter().filter(|d| matches!(d.marker, StudioMarker::WorkflowIntent)) {
            if let Some(elements) = doc.elements() {
                for (i, elem) in elements.iter().enumerate() {
                    out.push((doc, i, elem));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_indexes_documents() {
        let ws = Workspace::from_iter(vec![
            (
                "wf.json".to_string(),
                r#"{"$wosStudioWorkspace": "1.0", "id": "ws-1", "title": "T"}"#
                    .to_string(),
            ),
            (
                "po.json".to_string(),
                r#"{"$wosStudioPolicyObject": "1.0", "id": "pol-1",
                    "workspaceId": "ws-1", "kind": "Outcome"}"#
                    .to_string(),
            ),
        ]);
        assert_eq!(ws.documents.len(), 2);
        assert_eq!(ws.by_id.get("ws-1"), Some(&0));
        assert_eq!(ws.by_id.get("pol-1"), Some(&1));
    }
}
