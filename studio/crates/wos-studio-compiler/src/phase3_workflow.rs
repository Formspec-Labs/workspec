// Rust guideline compliant 2026-05-02

//! Phase 3 — Generate workflow intent → walk the WorkflowIntent's elements.
//!
//! `SA-MUST-cmp-012`: every WorkflowElement's `bridge` MUST be
//! well-formed for its `kind`. Malformed bridges halt with
//! `malformed-bridge`.

use serde_json::Value;

use crate::error::{CompileError, FailureKind};
use wos_studio_lint::WorkspaceDocument;

#[derive(Debug)]
pub struct WorkflowResult<'a> {
    pub workflow_intent: &'a WorkspaceDocument,
    pub elements: Vec<&'a Value>,
}

pub fn run<'a>(
    workflow_intent: &'a WorkspaceDocument,
) -> Result<WorkflowResult<'a>, CompileError> {
    let elements = workflow_intent
        .raw
        .get("elements")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut malformed: Vec<String> = Vec::new();
    for (i, elem) in elements.iter().enumerate() {
        let kind = elem.get("kind").and_then(Value::as_str).unwrap_or("");
        match kind {
            "step" | "system-check" => {
                // step / system-check require a bridge with kernelKind.
                let bridge = elem.get("bridge");
                let kernel_kind =
                    bridge.and_then(|b| b.get("kernelKind")).and_then(Value::as_str);
                if kernel_kind.is_none() {
                    let id = elem
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    malformed.push(format!(
                        "/elements/{i} ({id}): {kind} requires bridge.kernelKind"
                    ));
                }
            }
            "phase" => {
                if elem.get("body").is_none() {
                    let id = elem
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    malformed.push(format!(
                        "/elements/{i} ({id}): phase requires body block"
                    ));
                }
            }
            // Other kinds either have no bridge requirement or are
            // structurally unconstrained at this phase.
            _ => {}
        }
    }

    if !malformed.is_empty() {
        return Err(CompileError::halt_with(
            3,
            FailureKind::MalformedBridge,
            format!(
                "{} WorkflowElement(s) carry malformed or missing bridge",
                malformed.len()
            ),
            malformed,
        ));
    }

    Ok(WorkflowResult {
        workflow_intent,
        elements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wos_studio_model::{StudioDocument, StudioMarker};

    fn ws_doc(raw: serde_json::Value) -> WorkspaceDocument {
        let document: StudioDocument = serde_json::from_value(raw.clone()).unwrap();
        WorkspaceDocument {
            path: "wfi.json".to_string(),
            marker: StudioMarker::WorkflowIntent,
            document,
            raw,
        }
    }

    #[test]
    fn halts_on_step_without_bridge_kernel_kind() {
        let doc = ws_doc(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "elements": [{"id": "s1", "kind": "step"}]
        }));
        let err = run(&doc).expect_err("halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::MalformedBridge);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn passes_when_step_has_kernel_kind() {
        let doc = ws_doc(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "elements": [{
                "id": "s1", "kind": "step",
                "bridge": {"kernelKind": "transition"}
            }]
        }));
        let result = run(&doc).expect("ok");
        assert_eq!(result.elements.len(), 1);
    }
}
