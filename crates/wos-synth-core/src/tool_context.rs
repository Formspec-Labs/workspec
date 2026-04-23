//! [`ToolContext`] trait — the seam between the loop and the lint / conformance tools.
//!
//! Default production implementation lives in [`crate::tool_context::direct`]
//! (a thin wrapper over `wos-lint`). The trait exists so callers like
//! `wos-bench` can wrap it for instrumentation, caching, or remoting without
//! forking the loop.

use async_trait::async_trait;

pub mod direct;

pub use direct::DirectToolContext;

/// The set of tools the synthesis loop may invoke between LLM calls.
///
/// Conformance is intentionally an `Option` return today: most synthesized
/// documents do not have an associated fixture, and conformance only becomes
/// relevant when the caller supplies one. The seam is reserved for the
/// future when fixture synthesis is automated.
#[async_trait]
pub trait ToolContext: Send + Sync {
    /// Lint the document (raw JSON) and return structured findings.
    ///
    /// Empty vec = clean. Non-empty = repairable problems for the loop to
    /// feed back into the next prompt.
    async fn lint_document(&self, document_json: &str) -> Result<Vec<LintFinding>, ToolError>;

    /// Run conformance for the document if a fixture is available.
    ///
    /// Returning `Ok(None)` means "no fixture supplied, skip"; the loop will
    /// converge on lint cleanliness alone. `Ok(Some(verdict))` carries the
    /// pass/fail and any divergence detail.
    async fn run_conformance(
        &self,
        document_json: &str,
    ) -> Result<Option<ConformanceVerdict>, ToolError>;
}

/// A single lint finding the loop can ask the LLM to repair.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LintFinding {
    /// Stable rule id (e.g., `K-001`, `G-027`).
    pub rule_id: String,
    /// Severity of the finding.
    pub severity: Severity,
    /// Human-readable message; this is what the LLM sees in the repair prompt.
    pub message: String,
    /// Optional JSON pointer / path the finding is anchored to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Pre-rendered suggested fix (`add property at /foo`, `rename X → Y`, etc.).
    /// Surfaced in the repair prompt so the LLM gets the rule's machine-readable
    /// remediation, not just the human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
    /// Spec sections or matrix entries that explain this rule. Joined into
    /// the repair prompt so the model can ground its fix.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_docs: Vec<String>,
}

/// Severity buckets the loop distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Conformance result the loop can use to decide whether to keep iterating.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConformanceVerdict {
    /// True if the fixture passed end-to-end.
    pub passed: bool,
    /// Human-readable summary suitable for inclusion in a repair prompt.
    pub summary: String,
}

/// Failures the tool surface itself can suffer (distinct from lint findings).
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Lint pipeline crashed (not the same as a lint finding).
    #[error("lint pipeline failure: {0}")]
    Lint(String),

    /// Conformance harness crashed.
    #[error("conformance harness failure: {0}")]
    Conformance(String),
}
