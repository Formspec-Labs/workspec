// Rust guideline compliant 2026-05-02

//! Tier S4 — Workflow readiness (WF-LINT-001..008, EQ-LINT-001 doc-local).
//!
//! Many WF rules cross-cut PolicyObjects (Outcomes, NoticeRequirements,
//! AppealRights, EvidenceRequirements). The doc-local subset captured
//! here checks structural shape; cross-document resolution lives in
//! `crate::workspace_rules`.

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::WorkflowIntentDocument;

use super::studio_diagnostic;

pub fn check(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // WF-LINT-001 (Outcome → Notice + Appeal linkage) is workspace-tier
    // — it lives in `crate::workspace_rules::wf_lint_001`. The
    // doc-local "phase needs body" check moved to WFI-SHAPE-001
    // (a structural-shape rule, not a readiness rule).
    wfi_shape_001(doc, diagnostics);
    wf_lint_003(doc, diagnostics);
    wf_lint_007(doc, diagnostics);
    wf_lint_008(doc, diagnostics);
    eq_lint_001(doc, diagnostics);
}

fn elements<'a>(doc: &'a WorkflowIntentDocument) -> Option<&'a Vec<Value>> {
    doc.body.get("elements").and_then(Value::as_array)
}

/// `WFI-SHAPE-001` — WorkflowElement of kind `phase` MUST carry a body
/// block declaring contained steps. Doc-local structural shape check
/// (was mislabelled `WF-LINT-001`; that rule has different spec
/// semantics — see workspace_rules::wf_lint_001).
fn wfi_shape_001(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(elements) = elements(doc) else { return };
    for (i, elem) in elements.iter().enumerate() {
        let Some(kind) = elem.get("kind").and_then(Value::as_str) else { continue };
        if kind == "phase" && elem.get("body").is_none() {
            diagnostics.push(studio_diagnostic(
                "WFI-SHAPE-001",
                LintSeverity::Error,
                format!("/elements/{i}/body"),
                "WorkflowElement of kind 'phase' MUST carry a body block \
                 declaring contained steps."
                    .to_string(),
            ));
        }
    }
}

fn wf_lint_003(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every Deadline element MUST have a TimerMapping reference OR an
    // explicit reviewObligation.
    let Some(elements) = elements(doc) else { return };
    for (i, elem) in elements.iter().enumerate() {
        if elem.get("kind").and_then(Value::as_str) != Some("deadline") {
            continue;
        }
        let has_timer = elem.get("timerMappingRef").is_some()
            || elem
                .get("body")
                .and_then(|b| b.get("timerMappingRef"))
                .is_some();
        let has_review_obligation = elem
            .get("body")
            .and_then(|b| b.get("reviewObligation"))
            .is_some();
        if !has_timer && !has_review_obligation {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-003",
                LintSeverity::Error,
                format!("/elements/{i}/timerMappingRef"),
                "Deadline element MUST carry either a timerMappingRef or \
                 a reviewObligation; otherwise the deadline cannot fire."
                    .to_string(),
            ));
        }
    }
}

fn wf_lint_007(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every required EvidenceRequirement element MUST have at least one
    // collection step in the workflow.
    let Some(elements) = elements(doc) else { return };

    let evidence_ids: Vec<&str> = elements
        .iter()
        .filter(|e| e.get("kind").and_then(Value::as_str) == Some("evidence"))
        .filter(|e| {
            e.get("body")
                .and_then(|b| b.get("required"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|e| e.get("id").and_then(Value::as_str))
        .collect();

    let collected_evidence: std::collections::HashSet<&str> = elements
        .iter()
        .filter(|e| e.get("kind").and_then(Value::as_str) == Some("step"))
        .flat_map(|e| {
            e.get("body")
                .and_then(|b| b.get("collectsEvidence"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect();

    for (i, elem) in elements.iter().enumerate() {
        let Some(id) = elem.get("id").and_then(Value::as_str) else { continue };
        if evidence_ids.contains(&id) && !collected_evidence.contains(id) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-007",
                LintSeverity::Error,
                format!("/elements/{i}"),
                format!(
                    "Required EvidenceRequirement '{id}' has no workflow \
                     step that collects it. Add a step with collectsEvidence \
                     listing this id."
                ),
            ));
        }
    }
}

fn wf_lint_008(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every workflow step MUST carry a `derivedFrom` citation chain entry.
    let Some(elements) = elements(doc) else { return };
    for (i, elem) in elements.iter().enumerate() {
        if elem.get("kind").and_then(Value::as_str) != Some("step") {
            continue;
        }
        let derived = elem.get("derivedFrom").and_then(Value::as_array);
        if derived.is_none_or(|c| c.is_empty()) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-008",
                LintSeverity::Error,
                format!("/elements/{i}/derivedFrom"),
                "Workflow step MUST carry a derivedFrom citation chain \
                 (PolicyObject id or SourceCitation ref). Authoring \
                 provenance §SA-MUST-prov-005 requires this for audit."
                    .to_string(),
            ));
        }
    }
}

fn eq_lint_001(doc: &WorkflowIntentDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let impact_level = doc.body.get("impactLevel").and_then(Value::as_str);
    if impact_level != Some("rights-impacting") {
        return;
    }
    let categories = doc
        .body
        .get("protectedCategoryRefs")
        .and_then(Value::as_array);
    let count = categories.map(|c| c.len()).unwrap_or(0);
    if count < 3 {
        diagnostics.push(studio_diagnostic(
            "EQ-LINT-001",
            LintSeverity::Error,
            "/protectedCategoryRefs".to_string(),
            format!(
                "Rights-impacting workflows MUST declare at least 3 \
                 ProtectedCategories per workspace policy default; found {count}."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> WorkflowIntentDocument {
        serde_json::from_value(value).expect("workflow-intent doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    fn run(doc: WorkflowIntentDocument) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn wfi_shape_001_phase_needs_body() {
        let doc = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "elements": [{"id": "phase-1", "kind": "phase"}]
        }));
        assert_eq!(rule_count(&run(doc), "WFI-SHAPE-001"), 1);
    }

    #[test]
    fn wf_lint_003_deadline_needs_timer_or_review() {
        let doc = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "elements": [{"id": "d-1", "kind": "deadline"}]
        }));
        assert_eq!(rule_count(&run(doc), "WF-LINT-003"), 1);
    }

    #[test]
    fn wf_lint_007_evidence_must_be_collected() {
        let doc = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "elements": [
                {"id": "ev-pay-stub", "kind": "evidence",
                 "body": {"required": true}},
                {"id": "step-intake", "kind": "step",
                 "body": {"collectsEvidence": ["ev-other"]},
                 "derivedFrom": ["pol-1"]}
            ]
        }));
        assert_eq!(rule_count(&run(doc), "WF-LINT-007"), 1);
    }

    #[test]
    fn wf_lint_008_step_needs_derived_from() {
        let doc = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "elements": [{"id": "s-1", "kind": "step"}]
        }));
        assert_eq!(rule_count(&run(doc), "WF-LINT-008"), 1);
    }

    #[test]
    fn eq_lint_001_rights_impacting_needs_three_categories() {
        let too_few = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "impactLevel": "rights-impacting",
            "protectedCategoryRefs": ["pc-race", "pc-disability"]
        }));
        assert_eq!(rule_count(&run(too_few), "EQ-LINT-001"), 1);

        let enough = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "impactLevel": "rights-impacting",
            "protectedCategoryRefs": ["pc-race", "pc-disability", "pc-age"]
        }));
        assert_eq!(rule_count(&run(enough), "EQ-LINT-001"), 0);
    }

    #[test]
    fn eq_lint_001_skips_non_rights_impacting() {
        let doc = parse(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "version": "0.1.0",
            "impactLevel": "operational"
        }));
        assert_eq!(rule_count(&run(doc), "EQ-LINT-001"), 0);
    }
}
