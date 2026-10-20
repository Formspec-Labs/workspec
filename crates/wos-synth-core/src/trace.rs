//! Synthesis trace types — record of every loop iteration.
//!
//! The trace is the durable artifact the CLI emits via `wos-synth explain`.
//! Each iteration records the prompt token cost, the document attempt, the
//! native validation/replay reports, explicit absence/errors, and the host disposition.

use serde::{Deserialize, Serialize};

use crate::tool_context::CandidateAssessment;

/// Full record of a synthesis loop run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SynthTrace {
    /// One entry per loop iteration, in order.
    pub iterations: Vec<IterationRecord>,
    /// Summed across all iterations.
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    /// Actual provider failure after completed check evidence, without a fabricated iteration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interruption: Option<RunInterruption>,
}

/// A run interruption distinct from native candidate-check results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInterruption {
    /// Operation that failed; currently `repair_provider`.
    pub operation: String,
    /// Original available error text, including its provider error classification.
    pub error: String,
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
    /// Complete requested-check evidence, retained before any loop decision.
    pub assessment: CandidateAssessment,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}
