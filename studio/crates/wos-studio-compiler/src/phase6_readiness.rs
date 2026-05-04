// Rust guideline compliant 2026-05-02

//! Phase 6 — Run Studio readiness checks → tier S1–S6 evaluation.
//!
//! Calls into `wos_studio_lint::lint_workspace` for the full S1..S6 pass.
//! Diagnostics with severity `Error` plus the workspace's `WaivedFindings`
//! configuration determine `halt` vs `emit-with-warnings` per
//! `SA-MUST-cmp-040..043`.

use crate::error::{CompileError, FailureKind};
use wos_studio_lint::{LintDiagnostic, LintSeverity, Workspace};

#[derive(Debug)]
pub struct ReadinessResult {
    pub diagnostics: Vec<LintDiagnostic>,
}

pub fn run(ws: &Workspace, halt_on_error: bool) -> Result<ReadinessResult, CompileError> {
    let diagnostics = wos_studio_lint::lint_workspace(ws);
    let blocking: Vec<&LintDiagnostic> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, LintSeverity::Error))
        .collect();
    if halt_on_error && !blocking.is_empty() {
        let details: Vec<String> = blocking
            .iter()
            .map(|d| format!("{}: {} ({})", d.rule_id, d.message, d.path))
            .collect();
        return Err(CompileError::halt_with(
            6,
            FailureKind::StudioReadinessFailure,
            format!(
                "{} Studio readiness rule(s) at severity `error` block compile",
                blocking.len()
            ),
            details,
        ));
    }
    Ok(ReadinessResult { diagnostics })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    #[test]
    fn passes_when_no_error_diagnostics() {
        let ws = ws_from(vec![]);
        let result = run(&ws, true).expect("ok");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| !matches!(d.severity, LintSeverity::Error))
        );
    }

    #[test]
    fn halts_when_error_diagnostics_present() {
        // PUB-LINT-002 fires on a workspace with required reviewer roles + no approvals.
        let ws = ws_from(vec![(
            "ws.json",
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "w1",
                "reviewerRoles": [{"id": "compliance", "requiredForPublication": true}]
            }),
        )]);
        let err = run(&ws, true).expect_err("halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::StudioReadinessFailure);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn warn_mode_returns_diagnostics_without_halting() {
        let ws = ws_from(vec![(
            "ws.json",
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "w1",
                "reviewerRoles": [{"id": "compliance", "requiredForPublication": true}]
            }),
        )]);
        let result = run(&ws, false).expect("ok");
        assert!(!result.diagnostics.is_empty());
    }
}
