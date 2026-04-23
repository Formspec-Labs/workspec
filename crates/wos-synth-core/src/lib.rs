//! WOS LLM-authoring loop core.
//!
//! Owns the synthesis loop, the [`Prompter`] trait that abstracts over LLM
//! providers, and the [`ToolContext`] trait that abstracts over the lint /
//! conformance tool surface. No network dependencies, no provider-specific
//! code, no feature flags. Provider crates (`wos-synth-anthropic`,
//! `wos-synth-mock`) depend on this crate; this crate depends on neither.
//!
//! # Boundary
//!
//! Loop and abstractions live here. Concrete providers live in sibling crates.
//! Tool dispatch is delegated through [`ToolContext`] to keep the loop
//! testable without `wos-lint` / `wos-conformance` execution side effects.

pub mod errors;
pub mod prompter;
pub mod prompts;
pub mod synth_loop;
pub mod tool_context;
pub mod trace;

pub use errors::SynthError;
pub use prompter::{CacheAnchor, Completion, Prompter, PrompterError};
pub use prompts::Layer;
pub use synth_loop::{SynthOutcome, synthesize};
pub use tool_context::{
    ConformanceVerdict, DirectToolContext, LintFinding, Severity, ToolContext, ToolError,
};
pub use trace::{IterationRecord, SynthTrace};
