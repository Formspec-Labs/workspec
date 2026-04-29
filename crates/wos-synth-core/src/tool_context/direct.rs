//! `DirectToolContext` — stopgap that calls `wos-lint` in-process.
//!
//! Until `wos-mcp` exposes an in-process dispatch surface for the lint /
//! conformance tools, the synthesis loop calls them directly through this
//! adapter. The seam is intentional: the day `wos-mcp::dispatch` lands, this
//! file gets a sibling `McpToolContext` and `DirectToolContext` is retained
//! only for offline/no-MCP builds.

use async_trait::async_trait;

use super::{ConformanceVerdict, LintFinding, Severity, ToolContext, ToolError};

/// In-process tool context wrapping `wos-lint`.
///
/// Conformance is a no-op today — see [`Self::run_conformance`].
#[derive(Debug, Default, Clone, Copy)]
pub struct DirectToolContext;

impl DirectToolContext {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolContext for DirectToolContext {
    async fn lint_document(&self, document_json: &str) -> Result<Vec<LintFinding>, ToolError> {
        let diagnostics = wos_lint::lint_document(document_json)
            .map_err(|e| ToolError::Lint(e.to_string()))?;

        Ok(diagnostics.into_iter().map(into_finding).collect())
    }

    async fn run_conformance(
        &self,
        _document_json: &str,
    ) -> Result<Option<ConformanceVerdict>, ToolError> {
        // Conformance requires a fixture; the synthesis loop today produces
        // documents without paired fixtures, so we return `None` to signal
        // "skip the conformance gate." Once fixture-synthesis lands the seam
        // is ready to carry real verdicts.
        Ok(None)
    }
}

fn into_finding(diag: wos_lint::LintDiagnostic) -> LintFinding {
    let path = if diag.path.is_empty() {
        None
    } else {
        Some(diag.path.clone())
    };
    let suggested_fix = diag.suggested_fix.as_ref().map(render_suggested_fix);
    LintFinding {
        rule_id: diag.rule_id.to_string(),
        severity: map_severity(diag.severity),
        message: diag.message.clone(),
        path,
        suggested_fix,
        related_docs: diag.related_docs.clone(),
    }
}

fn render_suggested_fix(fix: &wos_lint::SuggestedFix) -> String {
    match fix {
        wos_lint::SuggestedFix::AddProperty { path, value } => {
            format!("add property at {path}: {value}")
        }
        wos_lint::SuggestedFix::RemoveProperty { path } => {
            format!("remove property at {path}")
        }
        wos_lint::SuggestedFix::ReplaceValue { path, value } => {
            format!("replace value at {path} with {value}")
        }
        wos_lint::SuggestedFix::Rename { from, to } => {
            format!("rename `{from}` → `{to}`")
        }
        wos_lint::SuggestedFix::Custom { hint } => hint.clone(),
    }
}

fn map_severity(severity: wos_lint::LintSeverity) -> Severity {
    match severity {
        wos_lint::LintSeverity::Error => Severity::Error,
        wos_lint::LintSeverity::Warning => Severity::Warning,
        wos_lint::LintSeverity::Info => Severity::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_returns_findings_for_invalid_kernel() {
        // Missing $wosWorkflow marker — should at least be flagged.
        let bad = r#"{"title": "Not a WOS document"}"#;
        let ctx = DirectToolContext::new();
        let result = pollster::block_on(ctx.lint_document(bad));
        // Either an Err (lint pipeline rejects) or non-empty findings is fine;
        // the contract is "this document is not clean."
        match result {
            Ok(findings) => assert!(
                !findings.is_empty(),
                "expected lint findings for a marker-less document"
            ),
            Err(ToolError::Lint(_)) => {}
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn conformance_is_skip_today() {
        let ctx = DirectToolContext::new();
        let verdict = pollster::block_on(ctx.run_conformance(r#"{"$wosWorkflow":"1.0"}"#))
            .expect("no-op should succeed");
        assert!(verdict.is_none(), "DirectToolContext skips conformance");
    }
}
