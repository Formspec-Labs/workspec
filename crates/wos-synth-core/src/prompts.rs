//! Prompt templates and the [`Layer`] enum.
//!
//! Prompts are pure functions returning `(system, user, cache_anchors)`. They
//! never touch the network or read the environment.

use crate::prompter::CacheAnchor;
use crate::tool_context::LintFinding;

/// Which WOS layer the loop is generating.
///
/// Today only [`Layer::Kernel`] is wired; other layers are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Kernel,
    Governance,
    Ai,
    Advanced,
}

impl Layer {
    pub fn marker(self) -> &'static str {
        match self {
            Layer::Kernel => "$wosWorkflow",
            Layer::Governance => "$wosGovernance",
            Layer::Ai => "$wosAiIntegration",
            Layer::Advanced => "$wosAdvancedGovernance",
        }
    }
}

/// Schemas baked into the binary at compile time.
///
/// We embed the workflow schema so generation works offline and the binary is
/// self-contained. Other layers will be added as their plans land.
const WORKFLOW_SCHEMA: &str = include_str!("../../../schemas/wos-workflow.schema.json");
const KERNEL_SPEC_SUMMARY: &str = include_str!("../../../specs/kernel/spec.llm.md");

/// Build the initial generation prompt.
///
/// Returns `(system, user, cache_anchors)`. Cache anchors are ordered most
/// stable first so prompt-caching providers can serve the schema and spec
/// from cache across iterations.
pub fn build_generate_prompt(problem: &str, layer: Layer) -> (String, String, Vec<CacheAnchor>) {
    let system = format!(
        "You are a workflow modelling expert producing WOS ({layer:?}) documents. \
         Output ONLY a single valid JSON object — no markdown fences, no prose. \
         Every document MUST include the marker `\"{marker}\": \"1.0\"`.",
        layer = layer,
        marker = layer.marker(),
    );

    let cache_anchors = match layer {
        Layer::Kernel => vec![
            CacheAnchor {
                name: "kernel-schema",
                content: WORKFLOW_SCHEMA.to_string(),
            },
            CacheAnchor {
                name: "kernel-spec-summary",
                content: KERNEL_SPEC_SUMMARY.to_string(),
            },
        ],
        // Other layers TBD — until then the model relies on the system prompt
        // alone. Lint will surface gaps and the repair loop will close them.
        _ => Vec::new(),
    };

    let user = format!(
        "## Problem statement\n\n{problem}\n\n\
         ## Requirements\n\n\
         - Output ONLY the JSON object.\n\
         - Include `\"{marker}\": \"1.0\"`.\n\
         - Use only fields permitted by the schema above.\n\n\
         Produce the document now.",
        problem = problem,
        marker = layer.marker(),
    );

    (system, user, cache_anchors)
}

/// Build a repair prompt from the prior attempt and lint findings.
pub fn build_repair_prompt(
    prior_attempt: &str,
    findings: &[LintFinding],
    layer: Layer,
) -> (String, String, Vec<CacheAnchor>) {
    let system = format!(
        "You are repairing a WOS ({layer:?}) document. Apply the minimum diff \
         that resolves every listed lint finding. Output ONLY the corrected \
         JSON — no markdown fences, no prose.",
        layer = layer,
    );

    let numbered = findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let path = f.path.as_deref().unwrap_or("(no path)");
            let mut entry = format!(
                "{idx}. [{rule}] {sev:?} at {path}: {msg}",
                idx = i + 1,
                rule = f.rule_id,
                sev = f.severity,
                path = path,
                msg = f.message,
            );
            if let Some(fix) = &f.suggested_fix {
                entry.push_str(&format!("\n   suggested fix: {fix}"));
            }
            if !f.related_docs.is_empty() {
                entry.push_str(&format!("\n   related: {}", f.related_docs.join(", ")));
            }
            entry
        })
        .collect::<Vec<_>>()
        .join("\n");

    let user = format!(
        "## Lint findings to repair\n\n{numbered}\n\n\
         ## Prior attempt\n\n```json\n{prior_attempt}\n```\n\n\
         Produce the corrected JSON now.",
        numbered = numbered,
        prior_attempt = prior_attempt,
    );

    // Repair re-uses the same cache anchors as generation so the provider
    // keeps serving them from cache across the loop's iterations.
    let cache_anchors = match layer {
        Layer::Kernel => vec![
            CacheAnchor {
                name: "kernel-schema",
                content: WORKFLOW_SCHEMA.to_string(),
            },
            CacheAnchor {
                name: "kernel-spec-summary",
                content: KERNEL_SPEC_SUMMARY.to_string(),
            },
        ],
        _ => Vec::new(),
    };

    (system, user, cache_anchors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_context::Severity;

    #[test]
    fn generate_prompt_embeds_problem_and_marker() {
        let (system, user, anchors) = build_generate_prompt("unique-sentinel-42", Layer::Kernel);
        assert!(system.contains("$wosWorkflow"));
        assert!(user.contains("unique-sentinel-42"));
        assert!(user.contains("$wosWorkflow"));
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].name, "kernel-schema");
    }

    #[test]
    fn repair_prompt_numbers_findings_and_embeds_attempt() {
        let findings = vec![
            LintFinding {
                rule_id: "K-001".into(),
                severity: Severity::Error,
                message: "missing initialState".into(),
                path: Some("/lifecycle".into()),
                suggested_fix: Some("add property at /lifecycle/initialState: \"start\"".into()),
                related_docs: vec!["Kernel §4.2".into()],
            },
            LintFinding {
                rule_id: "K-007".into(),
                severity: Severity::Error,
                message: "transition missing event".into(),
                path: None,
                suggested_fix: None,
                related_docs: vec![],
            },
        ];
        let (_sys, user, _) =
            build_repair_prompt(r#"{"$wosWorkflow":"1.0"}"#, &findings, Layer::Kernel);
        assert!(user.contains("1. [K-001]"));
        assert!(user.contains("2. [K-007]"));
        assert!(user.contains("missing initialState"));
        assert!(user.contains("$wosWorkflow"));
        // Suggested fix and related docs reach the prompt (Finding 5).
        assert!(user.contains("suggested fix: add property at /lifecycle/initialState"));
        assert!(user.contains("related: Kernel §4.2"));
    }
}
