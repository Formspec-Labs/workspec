//! Top-level error type produced by the synthesis loop.

use crate::prompter::PrompterError;
use crate::tool_context::ToolError;

/// All failure modes produced by [`crate::synthesize`].
#[derive(Debug, thiserror::Error)]
pub enum SynthError {
    /// The prompter (LLM provider) failed.
    #[error("prompter error: {0}")]
    Prompter(#[from] PrompterError),

    /// A tool call (lint or conformance) failed at the infrastructure layer.
    /// Diagnostic-level failures are NOT errors — they are loop input.
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    /// The LLM produced output that is not parseable as a JSON document.
    #[error("LLM output is not valid JSON after {iterations} iteration(s): {message}")]
    UnparseableOutput {
        /// How many iterations had run.
        iterations: u32,
        /// The parse error message.
        message: String,
        /// The raw text that failed to parse.
        raw: String,
    },
}
