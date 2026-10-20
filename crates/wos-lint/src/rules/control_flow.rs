//! Presents the shared core structural analysis through authoring diagnostics.
use crate::LintDiagnostic;
use wos_core::KernelDocument;
pub(crate) use wos_core::model::control_flow::CoverageGap;
use wos_core::model::control_flow::{FlowAnalysis, FlowSeverity, analyze};
pub(crate) fn append(
    result: FlowAnalysis,
    diagnostics: &mut Vec<LintDiagnostic>,
    skipped: &mut Vec<CoverageGap>,
) {
    for issue in result.issues {
        diagnostics.push(match issue.severity {
            FlowSeverity::Error => {
                LintDiagnostic::t1_error(issue.rule_id, &issue.path, issue.message)
            }
            FlowSeverity::Warning => {
                LintDiagnostic::t1_warning(issue.rule_id, &issue.path, issue.message)
            }
            FlowSeverity::Info => {
                LintDiagnostic::t1_info(issue.rule_id, &issue.path, issue.message)
            }
        });
    }
    skipped.extend(result.gaps);
}
pub(crate) fn check(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
    skipped: &mut Vec<CoverageGap>,
) {
    append(analyze(kernel), diagnostics, skipped);
}
