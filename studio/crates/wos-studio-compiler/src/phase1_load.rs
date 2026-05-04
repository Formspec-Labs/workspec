// Rust guideline compliant 2026-05-02

//! Phase 1 — Select approved policy objects → assemble compile-input.
//!
//! `SA-MUST-cmp-010`: every `policyObjectRefs[]` entry of the
//! WorkflowIntent's elements MUST be in lifecycle state `approved` or
//! later. Missing or unapproved inputs halt with `unapproved-input`.

use std::collections::HashSet;

use serde_json::Value;

use crate::error::{CompileError, FailureKind};
use wos_studio_lint::{Workspace, WorkspaceDocument};

#[derive(Debug)]
pub struct LoadResult<'a> {
    /// The single subject WorkflowIntent.
    pub workflow_intent: &'a WorkspaceDocument,
    /// PolicyObject ids referenced by the WorkflowIntent (sorted).
    pub referenced_policy_objects: Vec<String>,
    /// PolicyObject records (record-form, after collection-form unrolling).
    pub policy_object_records: Vec<&'a Value>,
}

const APPROVED_OR_LATER: &[&str] = &[
    "approved",
    "mapped",
    "validated",
    "published",
    "superseded",
    "deprecated",
];

pub fn run(ws: &Workspace) -> Result<LoadResult<'_>, CompileError> {
    let workflow_intent = ws.workflow_intent().ok_or_else(|| {
        CompileError::halt(
            1,
            FailureKind::MissingInput,
            "no $wosStudioWorkflowIntent document found in workspace",
        )
    })?;

    // wosVersionPin enforcement (`SA-MUST-cmp-052`). Two paths:
    //
    // 1. **Studio string-form pin** — current Studio
    //    `wos-studio-workflow-intent.schema.json` carries `wosVersionPin`
    //    as a free-form claims string (`kernel@1.0, governance@1.0, ...`).
    //    We parse it into stream@version pairs and verify the
    //    `kernel@X.Y` claim matches the envelope version compiled into
    //    this binary. Other stream claims pass through advisory until
    //    the parent RELEASE-STREAMS catalog moves out of markdown.
    // 2. **Kernel typed-form pin** (post-F1.3) — the typed
    //    `{envelopeVersion, includedBlocks[]}` shape is the kernel
    //    `wos-workflow.schema.json#/$defs/WosVersionPin`. Studio docs
    //    don't carry it directly today; the compiler emits it to the
    //    manifest. When a Studio doc DOES carry the typed form,
    //    `envelopeVersion` is enforced the same way.
    //
    // The compiled-in envelope version is "1.0" today (the $wosWorkflow
    // const at schemas/wos-workflow.schema.json:20). Hardcoding it here
    // is fine until ADR-0076 moves the envelope from 1.0; the build
    // script (build.rs) feeds SCHEMA_VERSION which is the content hash
    // of the schema, so a real bump produces a new hash + manifest
    // record.
    const SUPPORTED_ENVELOPE: &str = "1.0";
    if let Some(pin_value) = workflow_intent.raw.get("wosVersionPin") {
        match pin_value {
            Value::String(pin) => check_string_pin(pin, SUPPORTED_ENVELOPE)?,
            Value::Object(_) => check_typed_pin(pin_value, SUPPORTED_ENVELOPE)?,
            _ => {
                return Err(CompileError::halt(
                    1,
                    FailureKind::PinMismatch,
                    "WorkflowIntent.wosVersionPin must be a string or an \
                     object (per kernel $defs/WosVersionPin)",
                ));
            }
        }
    }

    // Collect every policyObjectRef referenced by elements.
    let mut referenced: HashSet<String> = HashSet::new();
    for (_doc, _i, elem) in ws.workflow_elements() {
        for r in collect_refs(elem) {
            referenced.insert(r);
        }
    }

    // Index PolicyObject records by id.
    let mut by_id: indexmap::IndexMap<&str, &Value> = indexmap::IndexMap::new();
    for (_doc, record) in ws.policy_object_records() {
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            by_id.insert(id, record);
        }
    }

    let mut missing: Vec<String> = Vec::new();
    let mut unapproved: Vec<String> = Vec::new();
    let mut consumed_records: Vec<&Value> = Vec::new();

    let mut sorted_refs: Vec<String> = referenced.iter().cloned().collect();
    sorted_refs.sort();

    for r in &sorted_refs {
        let Some(record) = by_id.get(r.as_str()) else {
            missing.push(r.clone());
            continue;
        };
        let state = record
            .get("lifecycleState")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !APPROVED_OR_LATER.contains(&state) {
            unapproved.push(format!("{r} (lifecycleState={state})"));
            continue;
        }
        consumed_records.push(record);
    }

    if !missing.is_empty() {
        return Err(CompileError::halt_with(
            1,
            FailureKind::MissingInput,
            format!("WorkflowIntent references {} unknown PolicyObject(s)", missing.len()),
            missing,
        ));
    }
    if !unapproved.is_empty() {
        return Err(CompileError::halt_with(
            1,
            FailureKind::UnapprovedInput,
            format!(
                "WorkflowIntent references {} PolicyObject(s) not in \
                 approved-or-later lifecycleState",
                unapproved.len()
            ),
            unapproved,
        ));
    }

    Ok(LoadResult {
        workflow_intent,
        referenced_policy_objects: sorted_refs,
        policy_object_records: consumed_records,
    })
}

/// Parse a Studio-form claims string and verify the `kernel@X.Y` claim
/// matches the compiled-in envelope version. Other stream claims pass
/// advisory.
fn check_string_pin(pin: &str, supported_envelope: &str) -> Result<(), CompileError> {
    let trimmed = pin.trim();
    if trimmed.is_empty() {
        return Err(CompileError::halt(
            1,
            FailureKind::PinMismatch,
            "WorkflowIntent.wosVersionPin is empty; declare an explicit \
             stream@version pin or remove the field",
        ));
    }
    if !trimmed.contains('@') {
        return Err(CompileError::halt_with(
            1,
            FailureKind::PinMismatch,
            "WorkflowIntent.wosVersionPin lacks the stream@version \
             format (e.g., 'kernel@1.0, governance@1.0')",
            vec![trimmed.to_string()],
        ));
    }
    // Find the `kernel@X.Y` claim; if absent that's an advisory state
    // (older docs may pin only optional streams). If present, verify.
    for claim in trimmed.split(',').map(str::trim) {
        let mut parts = claim.splitn(2, '@');
        let stream = parts.next().unwrap_or("").trim();
        let version = parts.next().unwrap_or("").trim();
        if stream == "kernel" && version != supported_envelope {
            return Err(CompileError::halt_with(
                1,
                FailureKind::PinMismatch,
                format!(
                    "WorkflowIntent.wosVersionPin claims kernel@{version} but \
                     this compiler supports kernel@{supported_envelope}"
                ),
                vec![trimmed.to_string()],
            ));
        }
    }
    Ok(())
}

/// Verify a kernel-typed `{envelopeVersion, includedBlocks[]}` pin.
fn check_typed_pin(pin: &Value, supported_envelope: &str) -> Result<(), CompileError> {
    let envelope = pin
        .get("envelopeVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CompileError::halt(
                1,
                FailureKind::PinMismatch,
                "WorkflowIntent.wosVersionPin (typed form) lacks the required \
                 envelopeVersion field",
            )
        })?;
    if envelope != supported_envelope {
        return Err(CompileError::halt_with(
            1,
            FailureKind::PinMismatch,
            format!(
                "WorkflowIntent.wosVersionPin.envelopeVersion is `{envelope}` \
                 but this compiler supports envelope `{supported_envelope}`"
            ),
            vec![envelope.to_string()],
        ));
    }
    Ok(())
}

fn collect_refs(elem: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = elem.get("policyObjectRefs").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                out.push(s.to_string());
            }
        }
    }
    if let Some(s) = elem.get("policyObjectRef").and_then(Value::as_str) {
        out.push(s.to_string());
    }
    out
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

    #[test]
    fn halts_on_missing_workflow_intent() {
        let ws = ws_from(vec![]);
        let err = run(&ws).expect_err("should halt");
        match err {
            CompileError::Halt { kind, phase, .. } => {
                assert_eq!(kind, FailureKind::MissingInput);
                assert_eq!(phase, 1);
            }
            _ => panic!("expected Halt"),
        }
    }

    #[test]
    fn halts_on_unapproved_policy_object() {
        let ws = ws_from(vec![
            (
                "wfi.json",
                json!({
                    "$wosStudioWorkflowIntent": "1.0",
                    "id": "wfi-1",
                    "elements": [{"id": "e", "policyObjectRefs": ["pol-x"]}]
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
        let err = run(&ws).expect_err("should halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::UnapprovedInput);
            }
            _ => panic!("expected Halt"),
        }
    }

    #[test]
    fn halts_on_pin_mismatch_kernel_envelope() {
        // `kernel@2.0` doesn't match the compiler's compiled-in
        // envelope `1.0` — SA-MUST-cmp-052 demands a halt.
        let ws = ws_from(vec![(
            "wfi.json",
            json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-1",
                "wosVersionPin": "kernel@2.0, governance@1.0",
                "elements": []
            }),
        )]);
        let err = run(&ws).expect_err("should halt on pin mismatch");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::PinMismatch);
            }
            _ => panic!("expected Halt"),
        }
    }

    #[test]
    fn accepts_string_pin_with_matching_kernel_envelope() {
        let ws = ws_from(vec![(
            "wfi.json",
            json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-1",
                "wosVersionPin": "kernel@1.0, governance@1.0",
                "elements": []
            }),
        )]);
        run(&ws).expect("matching kernel pin must pass");
    }

    #[test]
    fn accepts_typed_pin_with_matching_envelope() {
        let ws = ws_from(vec![(
            "wfi.json",
            json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-1",
                "wosVersionPin": {
                    "envelopeVersion": "1.0",
                    "includedBlocks": ["governance"]
                },
                "elements": []
            }),
        )]);
        run(&ws).expect("matching typed pin must pass");
    }

    #[test]
    fn halts_on_typed_pin_envelope_mismatch() {
        let ws = ws_from(vec![(
            "wfi.json",
            json!({
                "$wosStudioWorkflowIntent": "1.0",
                "id": "wfi-1",
                "wosVersionPin": {"envelopeVersion": "2.0"},
                "elements": []
            }),
        )]);
        let err = run(&ws).expect_err("should halt on typed pin mismatch");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::PinMismatch);
            }
            _ => panic!("expected Halt"),
        }
    }

    #[test]
    fn loads_clean_workspace() {
        let ws = ws_from(vec![
            (
                "wfi.json",
                json!({
                    "$wosStudioWorkflowIntent": "1.0",
                    "id": "wfi-1",
                    "elements": [{"id": "e", "policyObjectRefs": ["pol-x"]}]
                }),
            ),
            (
                "po.json",
                json!({
                    "$wosStudioPolicyObject": "1.0",
                    "policyObjects": [{"id": "pol-x", "lifecycleState": "approved"}]
                }),
            ),
        ]);
        let result = run(&ws).expect("loads");
        assert_eq!(result.referenced_policy_objects, vec!["pol-x".to_string()]);
        assert_eq!(result.policy_object_records.len(), 1);
    }
}
