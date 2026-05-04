// Rust guideline compliant 2026-05-02

//! Tier S3 — Mapping readiness (MAP-LINT-002, 003, 004, 008,
//!                              EFF-LINT-004 doc-local subset).
//!
//! Workspace-tier rules (MAP-LINT-001 every-approved-PolicyObject-has-mapping,
//! MAP-LINT-005 collision-detection, MAP-LINT-006 workflow-bearing-not-
//! unmapped, MAP-LINT-007 no-open-extension-record-blocks, EFF-LINT-004
//! cross-mapping-collision) live in `crate::workspace_rules`.

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::MappingDocument;

use super::studio_diagnostic;

pub fn check(doc: &MappingDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Walk the single mapping or the collection of mappings and apply
    // each doc-local rule to each.
    if let Some(records) = doc.body.get("mappings").and_then(Value::as_array) {
        for (i, record) in records.iter().enumerate() {
            check_record(record, &format!("/mappings/{i}"), diagnostics);
        }
    } else {
        check_record(&serde_json::to_value(doc).unwrap_or(Value::Null), "", diagnostics);
    }
}

fn check_record(record: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    map_lint_002(record, base, diagnostics);
    map_lint_003(record, base, diagnostics);
    map_lint_004(record, base, diagnostics);
    map_lint_008(record, base, diagnostics);
}

fn map_lint_002(record: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if record.get("mappingState").and_then(Value::as_str) != Some("mapsToWos") {
        return;
    }
    let targets = record.get("targets").and_then(Value::as_array);
    let has_target = targets.is_some_and(|t| !t.is_empty());
    if !has_target {
        diagnostics.push(studio_diagnostic(
            "MAP-LINT-002",
            LintSeverity::Error,
            format!("{base}/targets"),
            "mapsToWos mapping MUST carry at least one target with valid \
             wosConceptId / wosJsonPath."
                .to_string(),
        ));
        return;
    }
    for (i, target) in targets.unwrap().iter().enumerate() {
        let path = format!("{base}/targets/{i}");
        let has_concept = target
            .get("wosConceptId")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty());
        let has_jsonpath = target
            .get("wosJsonPath")
            .and_then(Value::as_str)
            .is_some_and(|s| s.starts_with('$') || s.starts_with('/'));
        if !has_concept || !has_jsonpath {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-002",
                LintSeverity::Error,
                path,
                "mapsToWos target MUST carry both a non-empty wosConceptId \
                 and a wosJsonPath that begins with `$` or `/`."
                    .to_string(),
            ));
        }
    }
}

fn map_lint_003(record: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if record.get("mappingState").and_then(Value::as_str)
        != Some("requiresSpecExtension")
    {
        return;
    }
    let extension = record
        .get("extensionRecord")
        .or_else(|| record.get("extensionRecordRef"));
    if extension.is_none() {
        diagnostics.push(studio_diagnostic(
            "MAP-LINT-003",
            LintSeverity::Error,
            format!("{base}/extensionRecord"),
            "requiresSpecExtension mapping MUST carry an extensionRecord \
             (or extensionRecordRef) describing the proposed WOS extension."
                .to_string(),
        ));
        return;
    }
    if let Some(extension) = record.get("extensionRecord") {
        let proposal_present = extension
            .get("proposal")
            .and_then(Value::as_str)
            .is_some_and(|s| s.len() >= 50);
        if !proposal_present {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-003",
                LintSeverity::Error,
                format!("{base}/extensionRecord/proposal"),
                "extensionRecord proposal MUST be substantive (≥50 characters \
                 of rationale)."
                    .to_string(),
            ));
        }
    }
}

fn map_lint_004(record: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    if record.get("mappingState").and_then(Value::as_str)
        != Some("unmappedButApproved")
    {
        return;
    }
    let rationale = record
        .get("unmappedRationale")
        .and_then(Value::as_str)
        .unwrap_or("");
    if rationale.len() < 50 {
        diagnostics.push(studio_diagnostic(
            "MAP-LINT-004",
            LintSeverity::Warning,
            format!("{base}/unmappedRationale"),
            "unmappedButApproved mapping MUST carry a substantive \
             unmappedRationale (≥50 characters); this finding stays at \
             warn perpetually as a noisy reminder."
                .to_string(),
        ));
    }
}

fn map_lint_008(record: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(targets) = record.get("targets").and_then(Value::as_array) else {
        return;
    };
    for (i, target) in targets.iter().enumerate() {
        let path = target.get("wosJsonPath").and_then(Value::as_str);
        let registry_entry = target.get("extensionRegistryEntry");
        let starts_with_x =
            path.is_some_and(|p| p.contains("x-") || p.contains("/x-"));
        if starts_with_x && registry_entry.is_none() {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-008",
                LintSeverity::Error,
                format!("{base}/targets/{i}/extensionRegistryEntry"),
                "Mapping target with `x-` extension path MUST carry an \
                 extensionRegistryEntry referencing the workspace's \
                 extension registry."
                    .to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> MappingDocument {
        serde_json::from_value(value).expect("mapping doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    fn run(doc: MappingDocument) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn map_lint_002_requires_concept_and_path() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "mappingState": "mapsToWos",
            "targets": [{"wosJsonPath": "$.governance"}]
        }));
        assert_eq!(rule_count(&run(doc), "MAP-LINT-002"), 1);
    }

    #[test]
    fn map_lint_002_passes_with_valid_target() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "mappingState": "mapsToWos",
            "targets": [{
                "wosConceptId": "PolicyObject",
                "wosJsonPath": "$.governance.policyObjects[0]"
            }]
        }));
        assert_eq!(rule_count(&run(doc), "MAP-LINT-002"), 0);
    }

    #[test]
    fn map_lint_003_requires_substantive_proposal() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "mappingState": "requiresSpecExtension",
            "extensionRecord": {"proposal": "TBD"}
        }));
        assert_eq!(rule_count(&run(doc), "MAP-LINT-003"), 1);
    }

    #[test]
    fn map_lint_004_warns_on_thin_rationale() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "mappingState": "unmappedButApproved",
            "unmappedRationale": "Workspace-only artifact."
        }));
        assert_eq!(rule_count(&run(doc), "MAP-LINT-004"), 1);
    }

    #[test]
    fn map_lint_008_x_target_needs_registry_entry() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "mappingState": "mapsToWos",
            "targets": [{
                "wosConceptId": "Whatever",
                "wosJsonPath": "$.x-custom-thing"
            }]
        }));
        assert_eq!(rule_count(&run(doc), "MAP-LINT-008"), 1);
    }

    #[test]
    fn collection_form_walks_each_record() {
        let doc = parse(json!({
            "$wosStudioMapping": "1.0",
            "mappings": [
                {"id": "m1", "mappingState": "mapsToWos", "targets": []},
                {"id": "m2", "mappingState": "unmappedButApproved",
                 "unmappedRationale": "short"}
            ]
        }));
        let diagnostics = run(doc);
        assert_eq!(rule_count(&diagnostics, "MAP-LINT-002"), 1);
        assert_eq!(rule_count(&diagnostics, "MAP-LINT-004"), 1);
    }
}
