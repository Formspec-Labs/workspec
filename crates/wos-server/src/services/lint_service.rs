//! Thin wrap around `wos-lint`. Exposes single-document linting plus the
//! rule-metadata catalog (`all_lint_rules`) as HTTP-friendly shapes.

use serde::Serialize;
use wos_lint::{Diagnostic, Severity};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticView {
    pub rule_id: &'static str,
    pub path: String,
    pub message: String,
    pub severity: &'static str,
}

impl From<Diagnostic> for DiagnosticView {
    fn from(d: Diagnostic) -> Self {
        Self {
            rule_id: d.rule_id,
            path: d.path,
            message: d.message,
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintResult {
    pub is_valid: bool,
    pub diagnostics: Vec<DiagnosticView>,
}

/// `POST /api/lint/document` — lint a single WOS document (Tier 1).
pub fn lint_document(body: &serde_json::Value) -> LintResult {
    let json = serde_json::to_string(body).unwrap_or_default();
    match wos_lint::lint_document(&json) {
        Ok(diags) => LintResult {
            is_valid: !diags.iter().any(|d| d.severity == Severity::Error),
            diagnostics: diags.into_iter().map(Into::into).collect(),
        },
        Err(e) => LintResult {
            is_valid: false,
            diagnostics: vec![DiagnosticView {
                rule_id: "PARSE-001",
                path: String::new(),
                message: e.to_string(),
                severity: "error",
            }],
        },
    }
}

/// `POST /api/lint/schema` — lint a JSON Schema doc for SCHEMA-DOC-001.
pub fn lint_schema(body: &serde_json::Value) -> LintResult {
    let json = serde_json::to_string(body).unwrap_or_default();
    match wos_lint::lint_schema(&json) {
        Ok(diags) => LintResult {
            is_valid: !diags.iter().any(|d| d.severity == Severity::Error),
            diagnostics: diags.into_iter().map(Into::into).collect(),
        },
        Err(e) => LintResult {
            is_valid: false,
            diagnostics: vec![DiagnosticView {
                rule_id: "PARSE-001",
                path: String::new(),
                message: e.to_string(),
                severity: "error",
            }],
        },
    }
}

/// `GET /api/lint/rules` — rule metadata catalog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMetadataView {
    pub id: &'static str,
    pub tier: &'static str,
    pub severity: &'static str,
    pub summary: &'static str,
    pub graduation: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<&'static str>,
    pub fixtures: &'static [&'static str],
}

pub fn list_rules() -> Vec<RuleMetadataView> {
    wos_lint::all_lint_rules()
        .iter()
        .map(|r| RuleMetadataView {
            id: r.id,
            tier: match r.tier {
                wos_lint::rules::Tier::T1 => "T1",
                wos_lint::rules::Tier::T2 => "T2",
                wos_lint::rules::Tier::T3 => "T3",
            },
            severity: match r.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
            summary: r.summary,
            graduation: match r.graduation {
                wos_lint::Graduation::Draft => "draft",
                wos_lint::Graduation::Tested => "tested",
                wos_lint::Graduation::Stable => "stable",
                wos_lint::Graduation::LoadBearing => "load-bearing",
            },
            spec_ref: r.spec_ref,
            suggested_fix: r.suggested_fix,
            fixtures: r.fixtures,
        })
        .collect()
}
