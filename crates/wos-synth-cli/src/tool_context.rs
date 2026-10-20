//! In-process synthesis adapter using the maintained MCP declarations and handlers.
//!
//! Configuration freezes separate static and runtime selections. Each candidate
//! owns a fresh registry, shared only by its static check and scenario list.

use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wos_mcp::{catalog, dispatch::dispatch, registry::ProjectRegistry};
use wos_synth_core::{
    CandidateAssessment, CheckDisposition, CheckRecord, LintFinding, ToolContext, ToolError,
    ToolResult,
};

/// Independently reviewed replay selection, with a host identity before native preparation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayScenario {
    /// Unique scenario identifier, retained even if native preparation fails.
    pub id: String,
    /// Native `wos_run_conformance` arguments without `project_id`.
    pub arguments: Value,
}

/// Frozen selections for one synthesis run; static and runtime dependencies stay separate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig {
    /// Native `wos_lint` arguments without `project_id`; defaults to project scope.
    #[serde(default = "project_validation")]
    pub validation: Value,
    /// Independent required scenarios, in request order.
    #[serde(default)]
    pub replays: Vec<ReplayScenario>,
}

fn project_validation() -> Value {
    json!({"scope":"project"})
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            validation: project_validation(),
            replays: vec![],
        }
    }
}

/// Host adapter over the same catalog execution used by MCP.
#[derive(Debug, Clone)]
pub struct McpToolContext {
    config: CheckConfig,
    draft_only: bool,
}

impl McpToolContext {
    /// Validate frozen selections using the owning catalog declarations.
    ///
    /// Draft mode requests static checks only and cannot carry replay selections.
    /// Missing scenarios in checked mode remain representable for saved-candidate checks.
    ///
    /// # Errors
    /// Rejects invalid catalog arguments, duplicate/empty scenario IDs, host-owned
    /// project IDs, or replay selections combined with static-only draft mode.
    pub fn new(mut config: CheckConfig, draft_only: bool) -> Result<Self, ToolError> {
        config.validation = normalized_arguments("wos_lint", config.validation)?;
        if !draft_only && config.validation["scope"] != "project" {
            return Err(ToolError::Execution(
                "checked synthesis requires project validation scope".into(),
            ));
        }
        if draft_only && !config.replays.is_empty() {
            return Err(ToolError::Execution(
                "draft-only mode requests static checks without replay scenarios".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for scenario in &mut config.replays {
            if scenario.id.trim().is_empty() || !ids.insert(scenario.id.clone()) {
                return Err(ToolError::Execution(
                    "scenario IDs must be nonempty and unique".into(),
                ));
            }
            scenario.arguments =
                normalized_arguments("wos_run_conformance", scenario.arguments.clone())?;
        }
        Ok(Self { config, draft_only })
    }

    /// Reject incomplete checked-generation configuration before calling a provider.
    ///
    /// # Errors
    /// Returns an error when checked generation has no independent scenario.
    pub fn require_generation_checks(&self) -> Result<(), ToolError> {
        if !self.draft_only && self.config.replays.is_empty() {
            return Err(ToolError::Execution("checked generation requires at least one replay scenario in --checks; use --draft-only for a static-only draft".into()));
        }
        Ok(())
    }

    fn not_run_replays(&self, reason: &str) -> Vec<CheckRecord> {
        if self.config.replays.is_empty() {
            return vec![CheckRecord {
                id: "replay-not-requested".into(),
                arguments: json!({}),
                result: ToolResult::NotRun {
                    reason: reason.into(),
                },
            }];
        }
        self.config
            .replays
            .iter()
            .map(|scenario| CheckRecord {
                id: scenario.id.clone(),
                arguments: scenario.arguments.clone(),
                result: ToolResult::NotRun {
                    reason: reason.into(),
                },
            })
            .collect()
    }

    fn intake_failure(&self, error: Value) -> CandidateAssessment {
        CandidateAssessment {
            findings: vec![],
            validation: CheckRecord {
                id: "static".into(),
                arguments: self.config.validation.clone(),
                result: ToolResult::NotRun {
                    reason: "candidate intake failed before native project registration".into(),
                },
            },
            intake_error: Some(error),
            replays: self
                .not_run_replays("candidate intake failed before native project registration"),
            authoring_decision: None,
            disposition: CheckDisposition::Blocked,
        }
    }
}

fn normalized_arguments(tool: &str, mut arguments: Value) -> Result<Value, ToolError> {
    if let Some(object) = arguments.as_object_mut() {
        if object.contains_key("project_id") {
            return Err(ToolError::Execution(
                "project_id belongs to the current synthesis candidate".into(),
            ));
        }
        // Only catalog shape validation runs here, so this placeholder is never looked up.
        object.insert("project_id".into(), json!("current-synthesis-candidate"));
    }
    let mut normalized = catalog::validate_arguments(tool, &arguments)
        .map_err(|error| ToolError::Execution(error.to_string()))?;
    normalized
        .as_object_mut()
        .expect("catalog requires an object")
        .remove("project_id");
    Ok(normalized)
}

#[async_trait]
impl ToolContext for McpToolContext {
    async fn check_candidate(&self, document_json: &str) -> Result<CandidateAssessment, ToolError> {
        let candidate: Value = match serde_json::from_str(document_json) {
            Ok(candidate) => candidate,
            Err(error) => return Ok(self.intake_failure(json!({"operation":"parse-candidate","message":error.to_string(),"line":error.line(),"column":error.column(),"category":format!("{:?}",error.classify())}))),
        };
        let project = match wos_authoring::WosProject::from_native_document(candidate) {
            Ok(project) => project,
            Err(diagnostic) => {
                return Ok(self.intake_failure(
                    json!({"operation":"register-native-candidate","diagnostic":diagnostic}),
                ));
            }
        };
        // Lifetime is one immutable candidate, including all configured scenarios.
        let mut registry = ProjectRegistry::new();
        let project_id = match registry.insert(project) {
            Ok(project_id) => project_id.to_string(),
            Err(error) => return Ok(self.intake_failure(json!({"operation":"register-native-candidate","message":error.to_string(),"detail":format!("{error:?}")}))),
        };
        let combined = dispatch(&mut registry,"wos_check_authoring",&project_id,
            json!({"project_id":project_id,"validation":self.config.validation,"replays":self.config.replays,"draft_only":self.draft_only}))
            .await.map_err(|error|ToolError::Execution(error.to_string()))?;
        let validation: CheckRecord = serde_json::from_value(combined["validation"].clone())
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        let mut replays: Vec<CheckRecord> = serde_json::from_value(combined["replays"].clone())
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        if replays.is_empty() {
            replays = self.not_run_replays(if self.draft_only {
                "explicit static-only draft; replay not requested"
            } else {
                "required replay unavailable: no independent scenario supplied"
            });
        }
        // Presentation maps the shared result; no native pass fields are aggregated here.
        let disposition = match combined["decision"]["status"].as_str() {
            Some("verified") => CheckDisposition::ChecksPassed,
            Some("draft") => CheckDisposition::DraftOnly,
            Some("blocked") => CheckDisposition::Blocked,
            _ => {
                return Err(ToolError::Execution(
                    "shared authoring decision status is absent or unsupported".into(),
                ));
            }
        };
        let findings = match &validation.result {
            ToolResult::Completed { report } => presentation_findings(&report["diagnostics"]),
            _ => vec![],
        };
        Ok(CandidateAssessment {
            findings,
            validation,
            replays,
            intake_error: None,
            authoring_decision: Some(combined["decision"].clone()),
            disposition,
        })
    }
}

fn presentation_findings(diagnostics: &Value) -> Vec<LintFinding> {
    // The catalog has already validated the owning output. This is presentation
    // only: neither severity counts nor this view decide whether checks passed.
    diagnostics
        .as_array()
        .into_iter()
        .flatten()
        .map(|diagnostic| LintFinding {
            rule_id: diagnostic["ruleId"]
                .as_str()
                .expect("validated diagnostic ruleId")
                .into(),
            severity: serde_json::from_value(diagnostic["severity"].clone())
                .expect("validated diagnostic severity"),
            message: diagnostic["message"]
                .as_str()
                .expect("validated diagnostic message")
                .into(),
            path: diagnostic["path"].as_str().map(str::to_string),
            suggested_fix: diagnostic.get("suggestedFix").map(Value::to_string),
            related_docs: diagnostic["relatedDocs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_block_and_repair_metadata_survive_presentation_without_becoming_a_pass_predicate() {
        // No current rule emits Block; exercise the native diagnostic type at
        // this presentation boundary rather than inventing a production rule.
        let diagnostic = wos_lint::LintDiagnostic {
            rule_id: "PUB-LINT-001",
            severity: wos_lint::LintSeverity::Block,
            tier: wos_lint::Tier::T2,
            path: "".into(),
            message: "Resolve the publication blocker.".into(),
            suggested_fix: Some(wos_lint::SuggestedFix::RemoveProperty {
                path: "/governance/findings/0".into(),
            }),
            related_docs: vec!["LINT-MATRIX.md#PUB-LINT-001".into()],
            source: None,
        };
        let native = serde_json::to_value(&diagnostic).unwrap();
        let findings = presentation_findings(&json!([native]));
        assert_eq!(findings[0].severity, wos_synth_core::Severity::Block);
        assert_eq!(findings[0].path.as_deref(), Some(""));
        assert_eq!(
            serde_json::from_str::<Value>(findings[0].suggested_fix.as_ref().unwrap()).unwrap(),
            native["suggestedFix"]
        );
        assert_eq!(findings[0].related_docs, diagnostic.related_docs);
        let assessment = CandidateAssessment {
            findings,
            intake_error: None,
            validation: CheckRecord {
                id: "static".into(),
                arguments: json!({}),
                result: ToolResult::Completed { report: native },
            },
            replays: vec![],
            authoring_decision: None,
            disposition: CheckDisposition::Blocked,
        };
        let (_, prompt, _) = wos_synth_core::prompts::build_repair_prompt(
            "{}",
            &assessment,
            wos_synth_core::Layer::Kernel,
        );
        assert!(prompt.contains("PUB-LINT-001"));
        assert!(prompt.contains("/governance/findings/0"));
        assert!(prompt.contains("LINT-MATRIX.md#PUB-LINT-001"));
    }
}
