// Rust guideline compliant 2026-05-02

//! Tier S1 — Source vault readiness (SV-LINT-001..006).

use std::collections::HashMap;

use serde_json::Value;

use crate::{LintDiagnostic, LintSeverity};
use wos_studio_model::SourceDocument;

use super::studio_diagnostic;

/// Run every source-vault rule against `doc`.
pub fn check(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    sv_lint_001(doc, diagnostics);
    sv_lint_002(doc, diagnostics);
    sv_lint_004(doc, diagnostics);
    sv_lint_005(doc, diagnostics);
    sv_lint_006(doc, diagnostics);
    sv_lint_008(doc, diagnostics);
    sv_lint_009(doc, diagnostics);
    sv_lint_010(doc, diagnostics);
    sv_lint_011(doc, diagnostics);
    sv_lint_012(doc, diagnostics);
    sv_lint_013(doc, diagnostics);
    sv_lint_014(doc, diagnostics);
    // SV-LINT-003 (no PolicyObject relies solely on disputed/superseded) and
    // SV-LINT-007 (versionless SourceDocument cited from elsewhere) are
    // workspace-tier — implemented in `crate::workspace_rules`.
}

/// Index sourceSections by `id` → text. Returns an owned map so callers
/// can iterate citations without re-walking the array.
fn index_sections(doc: &SourceDocument) -> HashMap<String, Option<String>> {
    let Some(sections) = doc.body.get("sourceSections").and_then(Value::as_array) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for s in sections {
        if let Some(id) = s.get("id").and_then(Value::as_str) {
            let text = s
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string);
            out.insert(id.to_string(), text);
        }
    }
    out
}

/// `SV-LINT-001` — every SourceCitation MUST resolve to a real
/// SourceSection in the same document (`SA-MUST-source-020`).
fn sv_lint_001(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(citations) = doc.body.get("sourceCitations").and_then(Value::as_array) else {
        return;
    };
    let sections = index_sections(doc);
    for (i, citation) in citations.iter().enumerate() {
        let Some(target) = citation.get("sectionRef").and_then(Value::as_str) else {
            continue;
        };
        if !sections.contains_key(target) {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-001",
                LintSeverity::Error,
                format!("/sourceCitations/{i}/sectionRef"),
                format!(
                    "SourceCitation references sectionRef '{target}' but no \
                     SourceSection with that id exists in the document.",
                ),
            ));
        }
    }
}

/// `SV-LINT-002` — citation `excerpt` MUST appear within the referenced
/// SourceSection's `text` (`SA-MUST-source-021`). Whitespace-tolerant
/// substring match: collapses runs of whitespace before comparing.
fn sv_lint_002(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(citations) = doc.body.get("sourceCitations").and_then(Value::as_array) else {
        return;
    };
    let sections = index_sections(doc);
    for (i, citation) in citations.iter().enumerate() {
        let (Some(target), Some(excerpt)) = (
            citation.get("sectionRef").and_then(Value::as_str),
            citation.get("excerpt").and_then(Value::as_str),
        ) else {
            continue;
        };
        let Some(Some(section_text)) = sections.get(target) else {
            // SV-LINT-001 already handles a dangling sectionRef; don't
            // double-fire here.
            continue;
        };
        if !whitespace_normalized_contains(section_text, excerpt) {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-002",
                LintSeverity::Error,
                format!("/sourceCitations/{i}/excerpt"),
                format!(
                    "Citation excerpt does not appear in SourceSection '{target}'. \
                     Excerpts MUST be a verbatim substring (whitespace-tolerant) \
                     of the referenced section's text."
                ),
            ));
        }
    }
}

fn whitespace_normalized_contains(haystack: &str, needle: &str) -> bool {
    fn normalize(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    normalize(haystack).contains(&normalize(needle))
}

/// `SV-LINT-004` — every `current` SourceVersion MUST carry an
/// `effectiveStart` (`SA-MUST-source-004`).
fn sv_lint_004(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    for (i, version) in versions.iter().enumerate() {
        let Some(state) = version.get("lifecycleState").and_then(Value::as_str) else {
            continue;
        };
        if state != "current" && state != "approved" {
            continue;
        }
        if version
            .get("effectiveStart")
            .filter(|v| !v.is_null())
            .is_none()
        {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-004",
                LintSeverity::Error,
                format!("/sourceVersions/{i}/effectiveStart"),
                format!(
                    "SourceVersion in '{state}' lifecycleState MUST carry an \
                     effectiveStart timestamp."
                ),
            ));
        }
    }
}

/// `SV-LINT-005` — section anchors MUST be unique within a SourceVersion
/// (`SA-MUST-source-010`). We approximate "within a SourceVersion" by
/// scoping uniqueness to each SourceSection's `versionRef`.
fn sv_lint_005(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(sections) = doc.body.get("sourceSections").and_then(Value::as_array) else {
        return;
    };
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for (i, section) in sections.iter().enumerate() {
        let (Some(version_ref), Some(anchor)) = (
            section.get("versionRef").and_then(Value::as_str),
            section.get("anchor").and_then(Value::as_str),
        ) else {
            continue;
        };
        let key = (version_ref.to_string(), anchor.to_string());
        if let Some(prior) = seen.get(&key) {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-005",
                LintSeverity::Error,
                format!("/sourceSections/{i}/anchor"),
                format!(
                    "Section anchor '{anchor}' is not unique within \
                     SourceVersion '{version_ref}' (also used by section #{prior})."
                ),
            ));
        } else {
            seen.insert(key, i);
        }
    }
}

/// `SV-LINT-006` — ExtractedClaims with `confidence < 0.5` MUST NOT have
/// `reviewState = approved` (`SA-MUST-pom-010`).
fn sv_lint_006(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(claims) = doc.body.get("extractedClaims").and_then(Value::as_array) else {
        return;
    };
    for (i, claim) in claims.iter().enumerate() {
        let confidence = claim
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let review = claim
            .get("reviewState")
            .and_then(Value::as_str)
            .unwrap_or("candidate");
        if confidence < 0.5 && review == "approved" {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-006",
                LintSeverity::Error,
                format!("/extractedClaims/{i}/reviewState"),
                format!(
                    "ExtractedClaim with confidence {confidence:.2} (< 0.5) \
                     was auto-approved. Low-confidence claims MUST receive \
                     reviewer attention before approval."
                ),
            ));
        }
    }
}

/// `SV-LINT-008` — every SourceVersion at lifecycle `current` /
/// `preliminary` / `disputed` MUST carry a `parsingResult` recording
/// the parse stage's outcome (`SA-MUST-source-002`). Temporal
/// state-machine *progression* through uploaded → parsed → indexed →
/// classified can't be inspected at lint time without history; the
/// presence-of-parsingResult check is the tractable lint-time slice
/// (the field is set during the `parsed` stage, so a downstream
/// state must carry it).
fn sv_lint_008(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    for (i, v) in versions.iter().enumerate() {
        let state = v.get("lifecycleState").and_then(Value::as_str).unwrap_or("");
        if !matches!(state, "current" | "preliminary" | "disputed") {
            continue;
        }
        if v.get("parsingResult").is_none() {
            let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "SV-LINT-008",
                LintSeverity::Error,
                format!("/sourceVersions/{i}/parsingResult"),
                format!(
                    "SourceVersion '{id}' is at lifecycleState='{state}' \
                     (downstream of parsed) but carries no parsingResult \
                     (per SA-MUST-source-002)."
                ),
            ));
        }
    }
}

/// `SV-LINT-009` — `parsingResult.status = ok` MUST be present for a
/// SourceVersion to leave `uploaded` (`SA-MUST-source-003`).
/// `parsingResult.status = partial` MAY proceed only with a recorded
/// reviewer waiver (`parsingWaiverRef`).
fn sv_lint_009(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    for (i, v) in versions.iter().enumerate() {
        let state = v.get("lifecycleState").and_then(Value::as_str).unwrap_or("");
        // Only versions past uploaded participate.
        if matches!(state, "uploaded" | "" ) {
            continue;
        }
        let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
        let status = v
            .get("parsingResult")
            .and_then(|p| p.get("status"))
            .and_then(Value::as_str);
        match status {
            Some("ok") => {}
            Some("partial") => {
                if v.get("parsingWaiverRef").is_none() {
                    diagnostics.push(studio_diagnostic(
                        "SV-LINT-009",
                        LintSeverity::Error,
                        format!("/sourceVersions/{i}/parsingWaiverRef"),
                        format!(
                            "SourceVersion '{id}' has parsingResult.status='partial' \
                             without a recorded parsingWaiverRef \
                             (per SA-MUST-source-003)."
                        ),
                    ));
                }
            }
            // status absent or non-{ok,partial} on a downstream state →
            // already flagged by SV-LINT-008's presence check (or fall
            // through to a status-shape error).
            _ => {
                if v.get("parsingResult").is_some() {
                    diagnostics.push(studio_diagnostic(
                        "SV-LINT-009",
                        LintSeverity::Error,
                        format!("/sourceVersions/{i}/parsingResult/status"),
                        format!(
                            "SourceVersion '{id}' has parsingResult but \
                             status is not 'ok' or 'partial' \
                             (per SA-MUST-source-003)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `SV-LINT-010` — at most one SourceVersion per SourceDocument MAY
/// hold the `current` lifecycle state at any moment
/// (`SA-MUST-source-005`).
fn sv_lint_010(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    let current_indices: Vec<usize> = versions
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            (v.get("lifecycleState").and_then(Value::as_str) == Some("current")).then_some(i)
        })
        .collect();
    if current_indices.len() > 1 {
        // Map every conflicting index to an id (fall back to "?" if a
        // version unexpectedly lacks one). The list reflects every
        // member of the conflict set — never silently truncates
        // because of a missing id.
        let ids: Vec<&str> = current_indices
            .iter()
            .map(|i| versions[*i].get("id").and_then(Value::as_str).unwrap_or("?"))
            .collect();
        // Anchor on the first; one diagnostic per extra to make
        // the total count visible.
        for i in current_indices.iter().skip(1) {
            diagnostics.push(studio_diagnostic(
                "SV-LINT-010",
                LintSeverity::Error,
                format!("/sourceVersions/{i}/lifecycleState"),
                format!(
                    "Multiple SourceVersions hold lifecycleState='current' \
                     within the same SourceDocument: {} (per SA-MUST-source-005). \
                     Promoting a new version to 'current' MUST atomically \
                     supersede the prior current version.",
                    ids.join(", ")
                ),
            ));
        }
    }
}

/// `SV-LINT-011` — when a SourceVersion's `pageable` flag is `true`,
/// every SourceSection in that version MUST carry a `pageRange`
/// (`SA-MUST-source-011`).
fn sv_lint_011(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    let pageable_versions: std::collections::HashSet<&str> = versions
        .iter()
        .filter(|v| v.get("pageable").and_then(Value::as_bool) == Some(true))
        .filter_map(|v| v.get("id").and_then(Value::as_str))
        .collect();
    if pageable_versions.is_empty() {
        return;
    }
    let Some(sections) = doc.body.get("sourceSections").and_then(Value::as_array) else {
        return;
    };
    for (i, section) in sections.iter().enumerate() {
        let Some(version_ref) = section.get("versionRef").and_then(Value::as_str) else {
            continue;
        };
        if !pageable_versions.contains(version_ref) {
            continue;
        }
        if section.get("pageRange").is_none() {
            let sid = section.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "SV-LINT-011",
                LintSeverity::Error,
                format!("/sourceSections/{i}/pageRange"),
                format!(
                    "SourceSection '{sid}' is in a pageable SourceVersion \
                     ('{version_ref}') but carries no pageRange \
                     (per SA-MUST-source-011)."
                ),
            ));
        }
    }
}

/// `SV-LINT-012` — JSON-LD `@context` drift across consecutive
/// *comparable* SourceVersions surfaces a tier-S1 finding
/// (`SA-MUST-source-052`). Detection rules (J5 hardening):
///
/// 1. Only `ingestFormat = "json-ld"` versions participate.
/// 2. Only `lifecycleState ∈ {current, superseded}` versions are
///    comparable. `disputed` versions are not baselines (their
///    contexts may already be questionable); `uploaded`/`parsed`/
///    `indexed`/`classified` are pre-publication intermediates.
/// 3. A non-json-ld SourceVersion appearing in document order
///    *between* two json-ld versions resets the comparison chain
///    (the baseline is the prior json-ld of the comparable lifecycle
///    that was NOT broken by a non-json-ld gap).
fn sv_lint_012(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    let mut prior_ctx: Option<&Value> = None;
    let mut prior_id: Option<&str> = None;
    for (i, v) in versions.iter().enumerate() {
        let format = v.get("ingestFormat").and_then(Value::as_str);
        // Non-json-ld version breaks the comparison chain.
        if format != Some("json-ld") {
            prior_ctx = None;
            prior_id = None;
            continue;
        }
        // Only compare against versions of comparable lifecycle.
        let lifecycle = v
            .get("lifecycleState")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !matches!(lifecycle, "current" | "superseded") {
            // Non-baseline lifecycle: don't compare against prior,
            // and don't update the prior baseline either.
            continue;
        }
        let ctx = v.get("jsonLdContext");
        let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
        if let (Some(prev), Some(curr)) = (prior_ctx, ctx) {
            if prev != curr {
                diagnostics.push(studio_diagnostic(
                    "SV-LINT-012",
                    LintSeverity::Error,
                    format!("/sourceVersions/{i}/jsonLdContext"),
                    format!(
                        "JSON-LD @context drift in SourceVersion '{id}' \
                         relative to prior comparable version '{}' \
                         (per SA-MUST-source-052; review semantic continuity).",
                        prior_id.unwrap_or("?")
                    ),
                ));
            }
        }
        if ctx.is_some() {
            prior_ctx = ctx;
            prior_id = Some(id);
        }
    }
}

/// `SV-LINT-013` — a SourceVersion with `ingestFormat = "akoma-ntoso"`
/// MUST carry `effectiveStart` (the FRBRdate-derived value) on the
/// version envelope. Sources without an extracted FRBRdate surface
/// this finding so reviewers supply the missing temporal scope
/// (`SA-MUST-source-081`).
fn sv_lint_013(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    for (i, v) in versions.iter().enumerate() {
        if v.get("ingestFormat").and_then(Value::as_str) != Some("akoma-ntoso") {
            continue;
        }
        if v.get("effectiveStart").is_none() {
            let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "SV-LINT-013",
                LintSeverity::Error,
                format!("/sourceVersions/{i}/effectiveStart"),
                format!(
                    "SourceVersion '{id}' has ingestFormat='akoma-ntoso' but \
                     no effectiveStart extracted from <FRBRdate> \
                     (per SA-MUST-source-081)."
                ),
            ));
        }
    }
}

/// `SV-LINT-014` — when a SourceVersion has multiple `contentLocales`,
/// every SourceSection in that version MUST carry text for the FIRST
/// (authoritative) locale; missing translations of OTHER locales are
/// permitted and surface as informational findings only
/// (`SA-MUST-source-060`). The spec's "per-locale block" surface is
/// represented as `text` for the authoritative locale + a `translations`
/// map keyed by locale code.
fn sv_lint_014(doc: &SourceDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(versions) = doc.body.get("sourceVersions").and_then(Value::as_array) else {
        return;
    };
    // Map versionId → authoritative-locale code.
    let auth_locale: std::collections::HashMap<&str, &str> = versions
        .iter()
        .filter_map(|v| {
            let id = v.get("id").and_then(Value::as_str)?;
            let locales = v.get("contentLocales").and_then(Value::as_array)?;
            if locales.len() < 2 {
                return None;
            }
            let first = locales.first()?.as_str()?;
            Some((id, first))
        })
        .collect();
    if auth_locale.is_empty() {
        return;
    }
    let Some(sections) = doc.body.get("sourceSections").and_then(Value::as_array) else {
        return;
    };
    for (i, section) in sections.iter().enumerate() {
        let Some(version_ref) = section.get("versionRef").and_then(Value::as_str) else {
            continue;
        };
        let Some(locale) = auth_locale.get(version_ref) else {
            continue;
        };
        // Authoritative locale text MUST be present (either inline `text`
        // or under `translations[<auth-locale>]`).
        let has_auth_text = section
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
            || section
                .get("translations")
                .and_then(Value::as_object)
                .and_then(|m| m.get(*locale))
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
        if !has_auth_text {
            let sid = section.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "SV-LINT-014",
                LintSeverity::Error,
                format!("/sourceSections/{i}/text"),
                format!(
                    "SourceSection '{sid}' (multilingual SourceVersion \
                     '{version_ref}', authoritative locale '{locale}') is \
                     missing text for the authoritative locale \
                     (per SA-MUST-source-060)."
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(json: serde_json::Value) -> SourceDocument {
        serde_json::from_value(json).expect("source doc")
    }

    fn rule_count(diagnostics: &[LintDiagnostic], rule: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule_id == rule).count()
    }

    #[test]
    fn sv_lint_001_dangling_section_ref() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceSections": [{"id": "sec-1"}],
            "sourceCitations": [{"id": "cite-1", "sectionRef": "sec-missing"}]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-001"), 1);
    }

    #[test]
    fn sv_lint_002_excerpt_substring_check() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceSections": [{"id": "sec-1", "text": "Notice MUST be in writing."}],
            "sourceCitations": [
                {"id": "cite-good", "sectionRef": "sec-1",
                 "excerpt": "MUST be in writing"},
                {"id": "cite-bad", "sectionRef": "sec-1",
                 "excerpt": "fabricated quote"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        let sv2 = diagnostics
            .iter()
            .filter(|d| d.rule_id == "SV-LINT-002")
            .collect::<Vec<_>>();
        assert_eq!(sv2.len(), 1);
        assert!(sv2[0].path.ends_with("/1/excerpt"));
    }

    #[test]
    fn sv_lint_004_current_version_needs_effective_start() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current"},
                {"id": "v-2", "lifecycleState": "current",
                 "effectiveStart": "2026-01-01"},
                {"id": "v-3", "lifecycleState": "ingested"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-004"), 1);
    }

    #[test]
    fn sv_lint_005_anchor_uniqueness() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceSections": [
                {"id": "s1", "versionRef": "v-1", "anchor": "§1"},
                {"id": "s2", "versionRef": "v-1", "anchor": "§1"},
                {"id": "s3", "versionRef": "v-2", "anchor": "§1"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        // Only the v-1/§1 collision fires; v-2 has its own scope.
        assert_eq!(rule_count(&diagnostics, "SV-LINT-005"), 1);
    }

    #[test]
    fn sv_lint_006_low_confidence_auto_approval() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "extractedClaims": [
                {"id": "c1", "confidence": 0.3, "reviewState": "approved"},
                {"id": "c2", "confidence": 0.95, "reviewState": "approved"},
                {"id": "c3", "confidence": 0.2, "reviewState": "needsReview"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-006"), 1);
    }

    #[test]
    fn whitespace_tolerance_in_excerpt_match() {
        assert!(whitespace_normalized_contains(
            "Notice  MUST   be in\nwriting.",
            "MUST be in writing"
        ));
        assert!(!whitespace_normalized_contains(
            "Notice MUST be in writing.",
            "MUST not be"
        ));
    }

    // ---- I-A1: SV-LINT-008..014 (doc-local) ----

    #[test]
    fn sv_lint_008_current_version_needs_parsing_result() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current"},
                {"id": "v-2", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"}},
                {"id": "v-3", "lifecycleState": "uploaded"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-008"), 1);
    }

    #[test]
    fn sv_lint_009_partial_status_requires_waiver() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-bad", "lifecycleState": "indexed",
                 "parsingResult": {"status": "partial"}},
                {"id": "v-ok", "lifecycleState": "indexed",
                 "parsingResult": {"status": "partial"},
                 "parsingWaiverRef": "rr-1"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-009"), 1);
    }

    #[test]
    fn sv_lint_010_at_most_one_current_version() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"}},
                {"id": "v-2", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"}},
                {"id": "v-3", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        // Two extras (v-2, v-3) flagged; v-1 is the canonical "first".
        assert_eq!(rule_count(&diagnostics, "SV-LINT-010"), 2);
    }

    #[test]
    fn sv_lint_011_pageable_section_needs_page_range() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"},
                 "pageable": true}
            ],
            "sourceSections": [
                {"id": "s1", "versionRef": "v-1", "anchor": "§1",
                 "pageRange": "1-2"},
                {"id": "s2", "versionRef": "v-1", "anchor": "§2"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-011"), 1);
    }

    #[test]
    fn sv_lint_012_jsonld_context_drift() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "superseded",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "1"}},
                {"id": "v-2", "lifecycleState": "current",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "2"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-012"), 1);
    }

    #[test]
    fn sv_lint_012_silent_when_non_jsonld_breaks_chain() {
        // J5 regression: v1(json-ld, superseded) → v2(pdf, superseded)
        // → v3(json-ld, current). Without the J5 fix, v3 would fire
        // against v1's context. Post-fix, v2 (non-json-ld) resets the
        // chain so v3 has no comparable baseline.
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "superseded",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "1"}},
                {"id": "v-2", "lifecycleState": "superseded",
                 "ingestFormat": "pdf",
                 "parsingResult": {"status": "ok"}},
                {"id": "v-3", "lifecycleState": "current",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "2"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-012"), 0);
    }

    #[test]
    fn sv_lint_012_silent_when_disputed_lifecycle_present() {
        // J5 regression: a `disputed` json-ld version is not a
        // comparable baseline. v1(json-ld, current) ≠ v2(json-ld,
        // disputed) ≠ v3(json-ld, current) — only v1 vs v3
        // participates; if their contexts agree, no fire.
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "superseded",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "stable"}},
                {"id": "v-2", "lifecycleState": "disputed",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "questionable"}},
                {"id": "v-3", "lifecycleState": "current",
                 "ingestFormat": "json-ld",
                 "parsingResult": {"status": "ok"},
                 "jsonLdContext": {"v": "stable"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-012"), 0);
    }

    #[test]
    fn sv_lint_013_akoma_ntoso_needs_frbr_date() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"},
                 "ingestFormat": "akoma-ntoso"}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        assert_eq!(rule_count(&diagnostics, "SV-LINT-013"), 1);
    }

    #[test]
    fn sv_lint_014_multilingual_needs_authoritative_text() {
        let doc = parse(json!({
            "$wosStudioSource": "1.0",
            "sourceVersions": [
                {"id": "v-1", "lifecycleState": "current",
                 "parsingResult": {"status": "ok"},
                 "contentLocales": ["en", "es"]}
            ],
            "sourceSections": [
                {"id": "s1", "versionRef": "v-1", "anchor": "§1",
                 "text": "English authoritative text."},
                {"id": "s2", "versionRef": "v-1", "anchor": "§2",
                 "translations": {"es": "Spanish only"}}
            ]
        }));
        let mut diagnostics = Vec::new();
        check(&doc, &mut diagnostics);
        // s2 lacks authoritative-locale (en) text.
        assert_eq!(rule_count(&diagnostics, "SV-LINT-014"), 1);
    }
}
