// Rust guideline compliant 2026-05-02

//! Tier S2 — Policy-object readiness.
//!
//! Doc-local rules:
//! - POM-LINT-001 — approved PolicyObject MUST carry citations.
//! - POM-LINT-002 — `originClass = approved-interpretation` MUST carry a
//!                  ReviewerResolution (PROV-LINT-003 dual).
//! - POM-LINT-003 — every approved PolicyObject MUST carry an `originClass`.
//! - PROV-LINT-002 — chain resolves to citation/assumption/attestation.
//! - PROV-LINT-003 — origin = approved-interpretation needs ReviewerResolution.
//! - PROV-LINT-004 — origin = local-practice needs an attestation.
//! - EFF-LINT-001 — redundant effectiveness duplicate (verbatim).
//! - EFF-LINT-003 — `enjoined` MUST be paired with `enjoinedScope`.
//! - AI-LINT-001 — AI-extracted claim missing aiLineage block.
//! - AI-LINT-002 — AI-extracted PolicyObject promoted past `extracted`
//!                 without `humanApprover`.
//! - EQ-LINT-002 — every ProtectedCategory cites a SourceCitation.
//! - TERM-LINT-002 — DataElement canonicalTermRef = manual-pending.
//! - TERM-LINT-003 — DataElement uses legacy `sensitivity` alias.
//!
//! Workspace-tier rules (POM-LINT-007 supersession-cycle, POM-LINT-008
//! conflict-resolution-required, EFF-LINT-002 widening-disallowed,
//! TERM-LINT-001 deprecated-canonicalTerm-target) live in
//! `crate::workspace_rules`.

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::PolicyObjectDocument;

use super::studio_diagnostic;

/// Run every doc-local policy-object readiness rule against `doc`.
pub fn check(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    pom_lint_001(doc, diagnostics);
    pom_lint_002(doc, diagnostics);
    pom_lint_003(doc, diagnostics);
    prov_lint_002(doc, diagnostics);
    prov_lint_004(doc, diagnostics);
    eff_lint_001(doc, diagnostics);
    eff_lint_003(doc, diagnostics);
    ai_lint_001(doc, diagnostics);
    ai_lint_002(doc, diagnostics);
    eq_lint_002(doc, diagnostics);
    term_lint_002(doc, diagnostics);
    term_lint_003(doc, diagnostics);
    pom_lint_dpv_001(doc, diagnostics);
}

/// `POM-LINT-DPV-001` — DataElement carrying `dpvSensitivity` MUST
/// also carry `canonicalTermRef`. Per F5.6 (2026-05-02). Pairs with
/// the F1.5 kernel FieldDeclaration extension that admits both
/// fields. Workspace-tier check on retentionPolicy resolution lives
/// in `WF-LINT-006` (already covered).
fn pom_lint_dpv_001(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Fires only on DataElement-kind PolicyObjects.
    if doc.kind() != Some("DataElement") {
        return;
    }
    if doc.dpv_sensitivity().is_none() {
        return;
    }
    if doc.canonical_term_ref().is_none() {
        let id = doc.id().unwrap_or("?");
        diagnostics.push(studio_diagnostic(
            "POM-LINT-DPV-001",
            LintSeverity::Error,
            format!("/policyObjects/{id}/canonicalTermRef"),
            format!(
                "DataElement '{id}' carries dpvSensitivity='{sensitivity}' \
                 but no canonicalTermRef. DPV-classified data MUST be \
                 vocabulary-aligned for cross-workflow review.",
                sensitivity = doc.dpv_sensitivity().unwrap_or("?")
            ),
        ));
    }
}

fn lifecycle_state(doc: &PolicyObjectDocument) -> Option<&str> {
    doc.body.get("lifecycleState").and_then(Value::as_str)
}

fn is_approved_or_later(state: Option<&str>) -> bool {
    matches!(
        state,
        Some("approved" | "mapped" | "validated" | "published" | "superseded" | "deprecated")
    )
}

fn pom_lint_001(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if !is_approved_or_later(lifecycle_state(doc)) {
        return;
    }
    let citations = doc.body.get("citations").and_then(Value::as_array);
    let basis_assumption = doc.body.get("basisAssumption").is_some();
    let has_citation = citations.is_some_and(|c| !c.is_empty());

    if !has_citation && !basis_assumption {
        diagnostics.push(studio_diagnostic(
            "POM-LINT-001",
            LintSeverity::Error,
            "/citations".to_string(),
            "Approved PolicyObject MUST carry at least one citation OR a \
             basisAssumption. Hold the object in 'draft' / 'reviewed' \
             until evidence lands."
                .to_string(),
        ));
    }
}

fn pom_lint_002(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if !is_approved_or_later(lifecycle_state(doc)) {
        return;
    }
    let origin = doc.body.get("originClass").and_then(Value::as_str);
    if origin == Some("approved-interpretation")
        && doc.body.get("reviewerResolution").is_none()
    {
        diagnostics.push(studio_diagnostic(
            "POM-LINT-002",
            LintSeverity::Error,
            "/reviewerResolution".to_string(),
            "PolicyObject with originClass='approved-interpretation' MUST \
             carry a reviewerResolution block (PROV-LINT-003 dual)."
                .to_string(),
        ));
    }
}

fn pom_lint_003(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if !is_approved_or_later(lifecycle_state(doc)) {
        return;
    }
    if doc.body.get("originClass").is_none() {
        diagnostics.push(studio_diagnostic(
            "POM-LINT-003",
            LintSeverity::Error,
            "/originClass".to_string(),
            "Approved PolicyObject MUST declare an originClass (one of \
             source / approved-interpretation / local-practice / \
             assumption / runtime-observed)."
                .to_string(),
        ));
    }
}

fn prov_lint_002(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if !is_approved_or_later(lifecycle_state(doc)) {
        return;
    }
    let citations = doc.body.get("citations").and_then(Value::as_array);
    let has_citation = citations.is_some_and(|c| !c.is_empty());
    let has_assumption = doc.body.get("basisAssumption").is_some();
    let has_attestation = doc.body.get("attestation").is_some();

    if !has_citation && !has_assumption && !has_attestation {
        diagnostics.push(studio_diagnostic(
            "PROV-LINT-002",
            LintSeverity::Error,
            "/citations".to_string(),
            "Approved PolicyObject's provenance chain MUST resolve to a \
             citation, assumption, or attestation. None present."
                .to_string(),
        ));
    }
}

fn prov_lint_004(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let origin = doc.body.get("originClass").and_then(Value::as_str);
    if origin == Some("local-practice") && doc.body.get("attestation").is_none() {
        diagnostics.push(studio_diagnostic(
            "PROV-LINT-004",
            LintSeverity::Error,
            "/attestation".to_string(),
            "PolicyObject with originClass='local-practice' MUST carry an \
             attestation block."
                .to_string(),
        ));
    }
}

fn eff_lint_001(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(eff) = doc.body.get("effectiveness") else {
        return;
    };
    // Redundant duplicate: object inlines effectiveness AND carries
    // `effectivenessRef` pointing somewhere — pick one, not both.
    if doc.body.get("effectivenessRef").is_some() && eff.is_object() {
        diagnostics.push(studio_diagnostic(
            "EFF-LINT-001",
            LintSeverity::Warning,
            "/effectiveness".to_string(),
            "PolicyObject inlines effectiveness AND declares \
             effectivenessRef. Pick one — inheritance via ref is preferred \
             unless this object scopes effectiveness more narrowly."
                .to_string(),
        ));
    }
}

fn eff_lint_003(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(eff) = doc.body.get("effectiveness").and_then(Value::as_object) else {
        return;
    };
    let appellate = eff.get("appellateState").and_then(Value::as_object);
    let Some(appellate) = appellate else { return };
    let enjoined = appellate
        .get("enjoined")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if enjoined && !appellate.contains_key("enjoinedScope") {
        diagnostics.push(studio_diagnostic(
            "EFF-LINT-003",
            LintSeverity::Error,
            "/effectiveness/appellateState/enjoinedScope".to_string(),
            "Effectiveness with enjoined=true MUST carry enjoinedScope \
             documenting which jurisdictions / time-windows are affected."
                .to_string(),
        ));
    }
}

fn ai_lint_001(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let extracted_by_ai = doc
        .body
        .get("extractedBy")
        .and_then(Value::as_str)
        .is_some_and(|s| s.starts_with("subj-ai-") || s.contains("agent"));
    let extraction_subtype = doc
        .body
        .get("eventSubtype")
        .and_then(Value::as_str)
        .is_some_and(|s| s.contains("ai"));
    if !(extracted_by_ai || extraction_subtype) {
        return;
    }
    if doc.body.get("aiLineage").is_none() {
        diagnostics.push(studio_diagnostic(
            "AI-LINT-001",
            LintSeverity::Error,
            "/aiLineage".to_string(),
            "AI-extracted PolicyObject MUST carry an aiLineage block \
             (modelId / modelVersion / promptTemplateRef / temperature / \
             seed / inputContextHash / confidence)."
                .to_string(),
        ));
    }
}

fn ai_lint_002(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let state = lifecycle_state(doc).unwrap_or("");
    let promoted_past_extracted = matches!(
        state,
        "approved" | "mapped" | "validated" | "published"
    );
    if !promoted_past_extracted {
        return;
    }
    let was_ai_extracted = doc.body.get("aiLineage").is_some();
    if !was_ai_extracted {
        return;
    }
    let has_human_approver = doc
        .body
        .get("aiLineage")
        .and_then(|l| l.get("humanApprover"))
        .is_some()
        || doc.body.get("humanApprover").is_some();
    if !has_human_approver {
        diagnostics.push(studio_diagnostic(
            "AI-LINT-002",
            LintSeverity::Error,
            "/aiLineage/humanApprover".to_string(),
            "AI-extracted PolicyObject promoted past 'extracted' MUST \
             record a humanApprover. AI extraction proposes; humans approve."
                .to_string(),
        ));
    }
}

fn eq_lint_002(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if doc.body.get("kind").and_then(Value::as_str) != Some("ProtectedCategory") {
        return;
    }
    let legal_basis = doc.body.get("legalBasis").and_then(Value::as_array);
    let has_basis = legal_basis.is_some_and(|c| !c.is_empty());
    let has_citation = doc
        .body
        .get("citations")
        .and_then(Value::as_array)
        .is_some_and(|c| !c.is_empty());
    if !has_basis && !has_citation {
        diagnostics.push(studio_diagnostic(
            "EQ-LINT-002",
            LintSeverity::Error,
            "/legalBasis".to_string(),
            "Every ProtectedCategory MUST cite a SourceCitation as legalBasis."
                .to_string(),
        ));
    }
}

fn term_lint_002(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if doc.body.get("kind").and_then(Value::as_str) != Some("DataElement") {
        return;
    }
    let canonical = doc
        .body
        .get("canonicalTermRef")
        .and_then(Value::as_str);
    if canonical == Some("manual-pending") {
        diagnostics.push(studio_diagnostic(
            "TERM-LINT-002",
            LintSeverity::Warning,
            "/canonicalTermRef".to_string(),
            "DataElement canonicalTermRef='manual-pending' awaits reviewer \
             attestation. Resolve before approval."
                .to_string(),
        ));
    }
}

fn term_lint_003(doc: &PolicyObjectDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if doc.body.get("kind").and_then(Value::as_str) != Some("DataElement") {
        return;
    }
    let sensitivity = doc.body.get("sensitivity").and_then(Value::as_str);
    if matches!(sensitivity, Some("pii" | "phi" | "restricted")) {
        diagnostics.push(studio_diagnostic(
            "TERM-LINT-003",
            LintSeverity::Warning,
            "/sensitivity".to_string(),
            "DataElement uses legacy sensitivity alias. Migrate to a DPV IRI \
             via canonicalTermRef for portable classification."
                .to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> PolicyObjectDocument {
        serde_json::from_value(value).expect("policy-object doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    fn run(doc: PolicyObjectDocument) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn pom_lint_001_approved_needs_citation_or_assumption() {
        let no_citation = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-x", "workspaceId": "ws-x", "kind": "NoticeRequirement",
            "lifecycleState": "approved", "originClass": "source"
        }));
        assert_eq!(rule_count(&run(no_citation), "POM-LINT-001"), 1);

        let with_assumption = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-x", "workspaceId": "ws-x", "kind": "NoticeRequirement",
            "lifecycleState": "approved", "originClass": "assumption",
            "basisAssumption": {"text": "Local practice"}
        }));
        assert_eq!(rule_count(&run(with_assumption), "POM-LINT-001"), 0);
    }

    #[test]
    fn pom_lint_002_approved_interpretation_needs_resolution() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "lifecycleState": "approved",
            "originClass": "approved-interpretation",
            "citations": [{"sourceCitationRef": "cite-1"}]
        }));
        assert_eq!(rule_count(&run(doc), "POM-LINT-002"), 1);
    }

    #[test]
    fn pom_lint_003_approved_needs_origin_class() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "lifecycleState": "approved",
            "citations": [{"sourceCitationRef": "cite-1"}]
        }));
        assert_eq!(rule_count(&run(doc), "POM-LINT-003"), 1);
    }

    #[test]
    fn prov_lint_004_local_practice_needs_attestation() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "originClass": "local-practice"
        }));
        assert_eq!(rule_count(&run(doc), "PROV-LINT-004"), 1);
    }

    #[test]
    fn eff_lint_003_enjoined_needs_scope() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "effectiveness": {
                "appellateState": {"enjoined": true}
            }
        }));
        assert_eq!(rule_count(&run(doc), "EFF-LINT-003"), 1);
    }

    #[test]
    fn ai_lint_001_extracted_needs_lineage() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "extractedBy": "subj-ai-claim-extractor"
        }));
        assert_eq!(rule_count(&run(doc), "AI-LINT-001"), 1);
    }

    #[test]
    fn ai_lint_002_promoted_needs_human_approver() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "p1", "workspaceId": "ws-x", "kind": "Outcome",
            "lifecycleState": "approved",
            "originClass": "source",
            "citations": [{"sourceCitationRef": "c1"}],
            "aiLineage": {"modelId": "claude-opus-4-7", "confidence": 0.9}
        }));
        assert_eq!(rule_count(&run(doc), "AI-LINT-002"), 1);
    }

    #[test]
    fn eq_lint_002_protected_category_needs_legal_basis() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pc1", "workspaceId": "ws-x", "kind": "ProtectedCategory"
        }));
        assert_eq!(rule_count(&run(doc), "EQ-LINT-002"), 1);
    }

    #[test]
    fn term_lint_002_canonical_pending_warns() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "de1", "workspaceId": "ws-x", "kind": "DataElement",
            "canonicalTermRef": "manual-pending"
        }));
        let diagnostics = run(doc);
        assert_eq!(rule_count(&diagnostics, "TERM-LINT-002"), 1);
        assert_eq!(diagnostics[0].severity, LintSeverity::Warning);
    }

    #[test]
    fn term_lint_003_legacy_sensitivity_alias_warns() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "de1", "workspaceId": "ws-x", "kind": "DataElement",
            "sensitivity": "pii"
        }));
        let diagnostics = run(doc);
        assert_eq!(rule_count(&diagnostics, "TERM-LINT-003"), 1);
        assert_eq!(diagnostics[0].severity, LintSeverity::Warning);
    }

    #[test]
    fn prov_lint_002_fires_on_approved_with_no_provenance_chain() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-1", "workspaceId": "ws-x",
            "kind": "NoticeRequirement",
            "lifecycleState": "approved",
            "originClass": "approved-interpretation"
            // No citations, no basisAssumption, no attestation.
        }));
        let diagnostics = run(doc);
        assert!(
            rule_count(&diagnostics, "PROV-LINT-002") >= 1,
            "expected PROV-LINT-002 (no provenance chain); got {diagnostics:?}"
        );
    }

    #[test]
    fn prov_lint_002_silent_when_assumption_present() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-1", "workspaceId": "ws-x",
            "kind": "NoticeRequirement",
            "lifecycleState": "approved",
            "originClass": "approved-interpretation",
            "basisAssumption": "Authoring assumption: 30-day notice window."
        }));
        let diagnostics = run(doc);
        assert_eq!(
            rule_count(&diagnostics, "PROV-LINT-002"),
            0,
            "PROV-LINT-002 must not fire when basisAssumption is present"
        );
    }

    #[test]
    fn pom_lint_dpv_001_fires_on_sensitivity_without_canonical_term() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "data-ssn", "workspaceId": "ws-x",
            "kind": "DataElement",
            "dpvSensitivity": "dpv:SpecialCategoryData"
            // No canonicalTermRef.
        }));
        let diagnostics = run(doc);
        assert!(
            rule_count(&diagnostics, "POM-LINT-DPV-001") >= 1,
            "expected POM-LINT-DPV-001; got {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_dpv_001_silent_when_canonical_term_present() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "data-ssn", "workspaceId": "ws-x",
            "kind": "DataElement",
            "dpvSensitivity": "dpv:SpecialCategoryData",
            "canonicalTermRef": "urn:wos:vocab:identity:ssn"
        }));
        let diagnostics = run(doc);
        assert_eq!(
            rule_count(&diagnostics, "POM-LINT-DPV-001"),
            0,
            "POM-LINT-DPV-001 must not fire when canonicalTermRef is present"
        );
    }

    #[test]
    fn pom_lint_dpv_001_silent_on_non_data_element() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "out-1", "workspaceId": "ws-x",
            "kind": "Outcome",
            "dpvSensitivity": "dpv:HealthData"
            // Outcome with dpvSensitivity is unusual but the rule
            // scopes to DataElement only.
        }));
        let diagnostics = run(doc);
        assert_eq!(
            rule_count(&diagnostics, "POM-LINT-DPV-001"),
            0,
            "POM-LINT-DPV-001 scopes to DataElement; got {diagnostics:?}"
        );
    }

    #[test]
    fn eff_lint_001_warns_on_redundant_inline_plus_ref() {
        let doc = parse(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-1", "workspaceId": "ws-x",
            "kind": "NoticeRequirement",
            "lifecycleState": "approved",
            "originClass": "source",
            "citations": [{"sourceCitationRef": "c-1"}],
            "effectivenessRef": "eff-1",
            "effectiveness": {
                "jurisdictions": ["US-TX"]
            }
        }));
        let diagnostics = run(doc);
        assert!(
            rule_count(&diagnostics, "EFF-LINT-001") >= 1,
            "expected EFF-LINT-001 (redundant inline + ref); got {diagnostics:?}"
        );
    }
}
