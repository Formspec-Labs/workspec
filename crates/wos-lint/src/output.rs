// Rust guideline compliant 2026-02-21

//! Output formatters for [`LintDiagnostic`] streams.
//!
//! Three formats are available:
//!
//! - **`text`** — one line per diagnostic, human-readable, `cargo check`-style.
//! - **`json`** — pretty-printed JSON array of [`LintDiagnostic`] objects.
//! - **`sarif`** — SARIF 2.1.0 for GitHub code scanning integration.
//!
//! All formatters are pure functions over `&[LintDiagnostic]` and produce
//! a `String`. They never fail — if serialization of a field is impossible
//! (which cannot currently happen given the stable types), the implementation
//! panics rather than silently dropping diagnostics.

use crate::diagnostic::{LintDiagnostic, LintSeverity};

// ---------------------------------------------------------------------------
// Text formatter
// ---------------------------------------------------------------------------

/// Format diagnostics as single-line human-readable text, one per line.
///
/// Output shape: `[RULE-ID] severity at path: message`
///
/// This mirrors the format of `cargo check` output and is the default
/// presentation for the `wos-lint` CLI.
///
/// # Examples
///
/// ```
/// use wos_lint::output::format_text;
/// use wos_lint::{LintDiagnostic, LintSeverity, Tier};
///
/// let diag = LintDiagnostic {
///     rule_id: "K-001",
///     severity: LintSeverity::Error,
///     tier: Tier::T1,
///     path: "/lifecycle/states/done".to_string(),
///     message: "final state must not have outgoing transitions".to_string(),
///     suggested_fix: None,
///     related_docs: vec![],
///     source: None,
/// };
/// let text = format_text(&[diag]);
/// assert!(text.contains("[K-001]"));
/// assert!(text.contains("error"));
/// ```
pub fn format_text(diagnostics: &[LintDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| {
            let sev = match d.severity {
                LintSeverity::Error => "error",
                LintSeverity::Warning => "warning",
                LintSeverity::Info => "info",
            };
            format!("[{}] {} at {}: {}", d.rule_id, sev, d.path, d.message)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// JSON formatter
// ---------------------------------------------------------------------------

/// Format diagnostics as a pretty-printed JSON array.
///
/// Each diagnostic serializes to a `camelCase` JSON object matching the
/// published `schemas/lint/wos-lint-diagnostic.schema.json`.
///
/// # Examples
///
/// ```
/// use wos_lint::output::format_json;
/// use wos_lint::{LintDiagnostic, LintSeverity, Tier};
///
/// let diag = LintDiagnostic {
///     rule_id: "K-001",
///     severity: LintSeverity::Error,
///     tier: Tier::T1,
///     path: "/lifecycle/states/done".to_string(),
///     message: "final state must not have outgoing transitions".to_string(),
///     suggested_fix: None,
///     related_docs: vec![],
///     source: None,
/// };
/// let json = format_json(&[diag]);
/// assert!(json.contains("\"ruleId\""));
/// assert!(json.contains("\"K-001\""));
/// ```
pub fn format_json(diagnostics: &[LintDiagnostic]) -> String {
    serde_json::to_string_pretty(diagnostics).expect("LintDiagnostic serialization must not fail")
}

// ---------------------------------------------------------------------------
// SARIF formatter
// ---------------------------------------------------------------------------

/// Format diagnostics as a SARIF 2.1.0 document for GitHub code scanning.
///
/// The output is a complete SARIF run object with a `wos-lint` tool entry,
/// one rule per unique `rule_id`, and one result per diagnostic. Location
/// information is included when `source` is present; otherwise a dummy
/// `physicalLocation` with `uri: "unknown"` is emitted (SARIF requires
/// a location on every result).
///
/// SARIF spec: <https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html>
///
/// # Examples
///
/// ```
/// use wos_lint::output::format_sarif;
/// use wos_lint::{LintDiagnostic, LintSeverity, Tier};
///
/// let diag = LintDiagnostic {
///     rule_id: "K-001",
///     severity: LintSeverity::Error,
///     tier: Tier::T1,
///     path: "/lifecycle/states/done".to_string(),
///     message: "final state must not have outgoing transitions".to_string(),
///     suggested_fix: None,
///     related_docs: vec![],
///     source: None,
/// };
/// let sarif = format_sarif(&[diag]);
/// assert!(sarif.contains("\"$schema\""));
/// assert!(sarif.contains("\"wos-lint\""));
/// ```
pub fn format_sarif(diagnostics: &[LintDiagnostic]) -> String {
    // Collect unique rule IDs for the rules array.
    let mut rule_ids: Vec<&'static str> = Vec::new();
    for d in diagnostics {
        if !rule_ids.contains(&d.rule_id) {
            rule_ids.push(d.rule_id);
        }
    }

    let rules: Vec<serde_json::Value> = rule_ids
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "name": id,
                "shortDescription": {
                    "text": format!("WOS lint rule {id}")
                }
            })
        })
        .collect();

    let results: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(|d| {
            let level = match d.severity {
                LintSeverity::Error => "error",
                LintSeverity::Warning => "warning",
                LintSeverity::Info => "note",
            };

            let location = if let Some(src) = &d.source {
                serde_json::json!({
                    "physicalLocation": {
                        "artifactLocation": { "uri": src.document },
                        "region": {
                            "startLine": src.line,
                            "startColumn": src.column
                        }
                    },
                    "logicalLocations": [{ "name": d.path }]
                })
            } else {
                serde_json::json!({
                    "physicalLocation": {
                        "artifactLocation": { "uri": "unknown" }
                    },
                    "logicalLocations": [{ "name": d.path }]
                })
            };

            serde_json::json!({
                "ruleId": d.rule_id,
                "level": level,
                "message": { "text": d.message },
                "locations": [location]
            })
        })
        .collect();

    let sarif = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "wos-lint",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/formspec/wos-spec",
                    "rules": rules
                }
            },
            "results": results
        }]
    });

    serde_json::to_string_pretty(&sarif).expect("SARIF serialization must not fail")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Tier;

    fn sample_diagnostic() -> LintDiagnostic {
        LintDiagnostic {
            rule_id: "K-001",
            severity: LintSeverity::Error,
            tier: Tier::T1,
            path: "/lifecycle/states/done".to_string(),
            message: "final state must not have outgoing transitions".to_string(),
            suggested_fix: None,
            related_docs: vec![],
            source: None,
        }
    }

    #[test]
    fn format_text_produces_correct_line() {
        let text = format_text(&[sample_diagnostic()]);
        assert_eq!(
            text,
            "[K-001] error at /lifecycle/states/done: final state must not have outgoing transitions"
        );
    }

    #[test]
    fn format_text_empty_produces_empty_string() {
        assert_eq!(format_text(&[]), "");
    }

    #[test]
    fn format_text_multiple_diagnostics_separated_by_newlines() {
        let d1 = sample_diagnostic();
        let mut d2 = sample_diagnostic();
        d2.rule_id = "K-002";
        d2.path = "/other".to_string();
        let text = format_text(&[d1, d2]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("[K-001]"));
        assert!(lines[1].contains("[K-002]"));
    }

    #[test]
    fn format_json_produces_valid_json_array() {
        let json = format_json(&[sample_diagnostic()]);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");
        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["ruleId"], "K-001");
        assert_eq!(arr[0]["severity"], "error");
        assert_eq!(arr[0]["tier"], "T1");
        assert_eq!(arr[0]["path"], "/lifecycle/states/done");
    }

    #[test]
    fn format_json_empty_produces_empty_array() {
        let json = format_json(&[]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.as_array().unwrap().is_empty());
    }

    #[test]
    fn format_sarif_produces_valid_sarif_shape() {
        let sarif_str = format_sarif(&[sample_diagnostic()]);
        let sarif: serde_json::Value =
            serde_json::from_str(&sarif_str).expect("must be valid JSON");
        assert_eq!(sarif["version"], "2.1.0");
        let runs = sarif["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 1);
        let driver = &runs[0]["tool"]["driver"];
        assert_eq!(driver["name"], "wos-lint");
        let results = runs[0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["ruleId"], "K-001");
        assert_eq!(results[0]["level"], "error");
    }

    #[test]
    fn format_sarif_warning_maps_to_warning_level() {
        let mut d = sample_diagnostic();
        d.severity = LintSeverity::Warning;
        let sarif: serde_json::Value = serde_json::from_str(&format_sarif(&[d])).unwrap();
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "warning");
    }

    #[test]
    fn format_sarif_info_maps_to_note_level() {
        let mut d = sample_diagnostic();
        d.severity = LintSeverity::Info;
        let sarif: serde_json::Value = serde_json::from_str(&format_sarif(&[d])).unwrap();
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "note");
    }
}
