//! Synthesis trace types — record of every loop iteration.
//!
//! The trace is the durable artifact the CLI emits via `wos-synth explain`.
//! Each iteration records the prompt token cost, the document attempt, the
//! lint findings observed, and the conformance verdict if any.

use serde::{Deserialize, Serialize};

use crate::tool_context::{ConformanceVerdict, LintFinding};

/// Full record of a synthesis loop run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SynthTrace {
    /// One entry per loop iteration, in order.
    pub iterations: Vec<IterationRecord>,
    /// Summed across all iterations.
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
}

impl SynthTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, record: IterationRecord) {
        self.total_input_tokens += record.input_tokens;
        self.total_output_tokens += record.output_tokens;
        self.total_cache_read_tokens += record.cache_read_tokens;
        self.iterations.push(record);
    }
}

/// One iteration of the loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationRecord {
    /// Zero-indexed iteration number.
    pub index: u32,
    /// Raw text the LLM returned (post-fence-strip).
    pub attempt: String,
    /// Lint findings observed for this attempt. Empty = clean.
    pub lint_findings: Vec<LintFinding>,
    /// Conformance verdict, if a fixture was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conformance: Option<ConformanceVerdict>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}
