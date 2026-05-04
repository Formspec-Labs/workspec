// Rust guideline compliant 2026-05-02

//! External-gate types per `SA-MUST-cmp-030..032`.
//!
//! The three gate names — `schema-pass`, `lint-pass`, `conformance-pass`
//! — are spec-stable and MUST be preserved verbatim across spec versions
//! and compiler versions (the contract by which technical implementers
//! triage publication failures).

use serde::{Deserialize, Serialize};

/// Spec-stable gate name. Display + serde representations are the
/// kebab-case wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalGate {
    SchemaPass,
    LintPass,
    ConformancePass,
}

impl ExternalGate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SchemaPass => "schema-pass",
            Self::LintPass => "lint-pass",
            Self::ConformancePass => "conformance-pass",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateOutcome {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: ExternalGate,
    pub outcome: GateOutcome,
    /// Diagnostic strings — empty on pass, populated on fail.
    pub findings: Vec<String>,
}

impl GateResult {
    pub fn pass(gate: ExternalGate) -> Self {
        Self {
            gate,
            outcome: GateOutcome::Pass,
            findings: Vec::new(),
        }
    }

    pub fn fail(gate: ExternalGate, findings: Vec<String>) -> Self {
        Self {
            gate,
            outcome: GateOutcome::Fail,
            findings,
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self.outcome, GateOutcome::Pass)
    }
}
