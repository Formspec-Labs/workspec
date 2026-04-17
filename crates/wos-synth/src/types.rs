//! Public types for wos-synth.
//!
//! This module is intentionally empty at Task 1. Subsequent tasks in the
//! wos-synth plan populate it with the public surface consumed by
//! `wos-bench` and the CLI:
//!
//! - `Outcome` / `SynthOutcome` (Task 4): converged vs unconverged run results.
//! - `CacheAnchor` (Task 2): named prompt prefixes eligible for provider-side
//!   prompt caching.
//! - `SynthTrace` (Task 4, schema in Task 7): per-iteration record of the
//!   generate → lint → conformance → repair loop.
//! - Error types (`SynthError`, `ProviderError`): Tasks 2–4.
//!
//! Keeping these in one module (rather than scattering across `loop.rs`,
//! `provider/mod.rs`, etc.) gives downstream crates a single stable import
//! path once the surface stabilises.
