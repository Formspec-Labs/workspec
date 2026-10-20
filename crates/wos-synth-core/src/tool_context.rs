//! Candidate-check port and complete evidence retained by the provider-independent loop.
//!
//! Hosts execute native tools and decide whether their requested checks completed.
//! The loop stores those reports before deciding whether to repair or stop.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The single operation the synthesis loop needs from its tool host.
#[async_trait]
pub trait ToolContext: Send + Sync {
    /// Check this exact candidate and retain every requested result.
    ///
    /// Expected candidate/tool failures belong in the returned evidence. Reserve
    /// errors for host failures that cannot produce an assessment.
    ///
    /// # Errors
    /// Returns an error when the host cannot record an assessment.
    async fn check_candidate(&self, document_json: &str) -> Result<CandidateAssessment, ToolError>;
}

/// The host's stopping decision for the requested synthesis checks.
///
/// This is not a publication, hosted-execution, or final readiness decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckDisposition {
    /// Every requested static check and independently supplied replay passed.
    ChecksPassed,
    /// Explicit static-only draft completed; behavior remains unchecked.
    DraftOnly,
    /// A required check failed, refused, errored, or did not run.
    Blocked,
}

/// Complete output or an explicit reason why no native report exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ToolResult {
    /// Untouched native tool output, including failed and refused native reports.
    Completed { report: Value },
    /// Exact available parser, authoring, or dispatch error detail.
    Error { error: Value },
    /// The operation did not execute; no passing report or identity is fabricated.
    NotRun { reason: String },
}

/// One selected operation and its complete result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRecord {
    /// Stable host identifier, available even when preparation refuses.
    pub id: String,
    /// Selected tool arguments without the host-owned ephemeral project identifier.
    pub arguments: Value,
    /// Complete native evidence or explicit absence.
    pub result: ToolResult,
}

/// Evidence and stopping decision for one immutable candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateAssessment {
    /// Derived presentation input for the existing lint repair prompt.
    pub findings: Vec<LintFinding>,
    /// Native intake error when malformed/nonobject input could not be registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intake_error: Option<Value>,
    /// Full canonical static validation tool result.
    pub validation: CheckRecord,
    /// Every selected scenario in request order; absence is explicit not-run.
    pub replays: Vec<CheckRecord>,
    /// Exact shared native decision and reasons, when the native host executed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authoring_decision: Option<Value>,
    /// Narrow requested-check disposition supplied by the host.
    pub disposition: CheckDisposition,
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
    /// Publication blocker; requires repair before convergence.
    Block,
    Error,
    Warning,
    Info,
}

/// A host failure that could not be represented as candidate evidence.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Host configuration or assessment transport failed.
    #[error("candidate-check host failure: {0}")]
    Execution(String),
}
