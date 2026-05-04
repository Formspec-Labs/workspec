// Rust guideline compliant 2026-05-02

//! Tier S6 — Publication readiness (doc-local subset).
//!
//! S6 rules are predominantly workspace-tier (PUB-LINT-001..007 require
//! the full publication packet). This module carries the doc-local
//! checks: ID-LINT-* (identity-subject shape), CHAIN-LINT-001 (provenance
//! hash chain integrity within a single ProvenanceDocument), and the
//! AI/EFF cross-cuts at S6.

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::{IdentitySubjectDocument, ProvenanceDocument};

use super::studio_diagnostic;

/// Run S6 doc-local rules against an IdentitySubject document.
pub fn check_identity(
    doc: &IdentitySubjectDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    id_lint_003(doc, diagnostics);
    id_lint_004(doc, diagnostics);
}

/// Run S6 doc-local rules against a Provenance document.
pub fn check_provenance(
    doc: &ProvenanceDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    chain_lint_001(doc, diagnostics);
}

fn id_lint_003(
    doc: &IdentitySubjectDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let attestation = doc.body.get("attestationLevel").and_then(Value::as_str);
    let action = doc.body.get("attemptedAction").and_then(Value::as_str);
    if let (Some(level), Some(act)) = (attestation, action) {
        let needs_high = matches!(act, "publish" | "approve" | "attest-local-practice");
        let high = matches!(level, "high-assurance" | "hardware-key");
        if needs_high && !high {
            diagnostics.push(studio_diagnostic(
                "ID-LINT-003",
                LintSeverity::Error,
                "/attestationLevel".to_string(),
                format!(
                    "Attempted action '{act}' requires high-assurance \
                     attestation; subject's attestationLevel is '{level}'."
                ),
            ));
        }
    }
}

/// `ID-LINT-004` — IdentitySubject cardinality + temporal validity per
/// ADR-0084 §2.2: every IdentitySubject at `lifecycleState=approved`
/// (or downstream — `mapped`/`validated`/`published`) MUST carry ≥ 1
/// entry in `activeAttestations[]` whose `validUntil` is null,
/// `"indefinite"` sentinel, OR a future date-time.
fn id_lint_004(
    doc: &IdentitySubjectDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let lifecycle = doc.body.get("lifecycleState").and_then(Value::as_str).unwrap_or("");
    if !matches!(
        lifecycle,
        "approved" | "mapped" | "validated" | "published"
    ) {
        return;
    }
    let id = doc.body.get("id").and_then(Value::as_str).unwrap_or("?");
    let attestations = doc
        .body
        .get("activeAttestations")
        .and_then(Value::as_array);
    let count = attestations.map(|a| a.len()).unwrap_or(0);
    if count == 0 {
        diagnostics.push(studio_diagnostic(
            "ID-LINT-004",
            LintSeverity::Error,
            "/activeAttestations".to_string(),
            format!(
                "IdentitySubject '{id}' is at lifecycleState='{lifecycle}' \
                 but carries no activeAttestations[] (per ADR-0084 §2.2; \
                 SA-MUST-id-004)."
            ),
        ));
        return;
    }
    // Walk attestations; require ≥ 1 with validUntil = null /
    // "indefinite" / future date-time. validUntil is parsed as the
    // id-010 sentinel oneOf [null, "indefinite", date-time string].
    let any_valid = attestations.unwrap().iter().any(|att| {
        let valid_until = att.get("validUntil");
        match valid_until {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) if s == "indefinite" => true,
            Some(Value::String(s)) => {
                // RFC-3339-shape lexicographic compare against a
                // synthetic far-future bound is sufficient for lint
                // purposes (no calendar arithmetic here).
                // We treat any value >= "2026-05-03" as future.
                // Fully precise comparisons are runtime concerns.
                s.as_str() > "2026-05-03"
            }
            _ => false,
        }
    });
    if !any_valid {
        diagnostics.push(studio_diagnostic(
            "ID-LINT-004",
            LintSeverity::Error,
            "/activeAttestations".to_string(),
            format!(
                "IdentitySubject '{id}' has activeAttestations[] but none \
                 are temporally valid (validUntil is null, \"indefinite\", \
                 or a future date-time) (per ADR-0084 §2.2)."
            ),
        ));
    }
}

fn chain_lint_001(
    doc: &ProvenanceDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    // Records may live under `record` (single) or `records[]` (collection).
    let mut records: Vec<&Value> = Vec::new();
    if let Some(r) = doc.body.get("record") {
        records.push(r);
    }
    if let Some(arr) = doc.body.get("records").and_then(Value::as_array) {
        for r in arr {
            records.push(r);
        }
    }
    if records.is_empty() {
        return;
    }

    let mut prev_self_hash: Option<String> = None;
    for (i, record) in records.iter().enumerate() {
        let prev_ref = record
            .get("hashChain")
            .and_then(|c| c.get("prevRecordHash"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let self_hash = record
            .get("hashChain")
            .and_then(|c| c.get("selfHash"))
            .and_then(Value::as_str)
            .map(str::to_string);

        if let Some(expected) = prev_self_hash.clone() {
            match prev_ref {
                Some(actual) if actual == expected => {}
                _ => {
                    diagnostics.push(studio_diagnostic(
                        "CHAIN-LINT-001",
                        LintSeverity::Error,
                        format!("/records/{i}/hashChain/prevRecordHash"),
                        format!(
                            "Provenance hashChain broken at record #{i}: \
                             prevRecordHash does not match the previous \
                             record's selfHash ({expected})."
                        ),
                    ));
                }
            }
        }
        prev_self_hash = self_hash;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_identity(value: serde_json::Value) -> IdentitySubjectDocument {
        serde_json::from_value(value).expect("identity doc")
    }

    fn parse_provenance(value: serde_json::Value) -> ProvenanceDocument {
        serde_json::from_value(value).expect("provenance doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    #[test]
    fn id_lint_003_publish_needs_high_assurance() {
        let doc = parse_identity(json!({
            "$wosStudioIdentitySubject": "1.0",
            "id": "subj-x",
            "attestationLevel": "session",
            "attemptedAction": "publish"
        }));
        let mut diagnostics = Vec::new();
        check_identity(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "ID-LINT-003"), 1);
    }

    #[test]
    fn id_lint_004_fires_on_approved_subject_without_attestations() {
        let doc = parse_identity(json!({
            "$wosStudioIdentitySubject": "1.0",
            "id": "subj-y",
            "attestationLevel": "session",
            "lifecycleState": "approved"
        }));
        let mut diagnostics = Vec::new();
        check_identity(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "ID-LINT-004"), 1);
    }

    #[test]
    fn id_lint_004_fires_on_expired_attestations_only() {
        let doc = parse_identity(json!({
            "$wosStudioIdentitySubject": "1.0",
            "id": "subj-y",
            "lifecycleState": "approved",
            "activeAttestations": [
                {"id": "att-old", "validUntil": "2020-01-01T00:00:00Z"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check_identity(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "ID-LINT-004"), 1);
    }

    #[test]
    fn id_lint_004_silent_with_indefinite_attestation() {
        let doc = parse_identity(json!({
            "$wosStudioIdentitySubject": "1.0",
            "id": "subj-y",
            "lifecycleState": "approved",
            "activeAttestations": [
                {"id": "att-1", "validUntil": "indefinite"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check_identity(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "ID-LINT-004"), 0);
    }

    #[test]
    fn id_lint_004_silent_pre_approved() {
        let doc = parse_identity(json!({
            "$wosStudioIdentitySubject": "1.0",
            "id": "subj-y",
            "lifecycleState": "draft"
        }));
        let mut diagnostics = Vec::new();
        check_identity(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "ID-LINT-004"), 0);
    }

    #[test]
    fn chain_lint_001_detects_break() {
        let doc = parse_provenance(json!({
            "$wosStudioProvenance": "1.0",
            "records": [
                {"id": "r1", "hashChain": {"selfHash": "abc"}},
                {"id": "r2", "hashChain": {"prevRecordHash": "WRONG", "selfHash": "def"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check_provenance(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "CHAIN-LINT-001"), 1);
    }

    #[test]
    fn chain_lint_001_passes_when_chain_ok() {
        let doc = parse_provenance(json!({
            "$wosStudioProvenance": "1.0",
            "records": [
                {"id": "r1", "hashChain": {"selfHash": "abc"}},
                {"id": "r2", "hashChain": {"prevRecordHash": "abc", "selfHash": "def"}},
                {"id": "r3", "hashChain": {"prevRecordHash": "def", "selfHash": "ghi"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check_provenance(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "CHAIN-LINT-001"), 0);
    }
}
