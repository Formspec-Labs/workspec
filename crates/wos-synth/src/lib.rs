//! wos-synth — reference LLM-authoring harness for WOS documents.
//!
//! This crate is scaffolded at Task 1 of the wos-synth plan. Provider trait,
//! prompt templates, loop orchestrator, and CLI land in subsequent tasks.
//!
//! Provider dependencies (reqwest, tokio, anthropic-sdk) are gated behind
//! the non-default `synth` Cargo feature. See README.md for the boundary
//! with `wos-bench`, the extraction trigger, and the benchmark-causality
//! guardrail.

pub mod types;

#[cfg(feature = "synth")]
pub mod provider_gated {
    //! Placeholder — Task 2 adds the provider trait module here, gated by feature.
}
