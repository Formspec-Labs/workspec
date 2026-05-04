// Rust guideline compliant 2026-05-02

//! Stage-6 Studio scenario simulator.
//!
//! Replays Studio Scenarios against a compiled `$wosWorkflow` and
//! produces an `ActualTrace` plus a `TraceDelta` (expected vs actual).
//! 16 canonical scenario types per `studio/specs/scenario-authoring.md`.
//!
//! ## Composition
//!
//! 1. Compile the Studio workspace via `wos_studio_compiler::compile`.
//! 2. For each emitted Scenario, build an `ActualTrace` by walking the
//!    scenario's `events[]` against the compiled `lifecycle.states`.
//! 3. Diff `ExpectedTrace` (declared) vs `ActualTrace` (computed) and
//!    return a `TraceDelta`.
//!
//! ## Conformance-trace bridge
//!
//! `ActualTrace` round-trips through the conformance-trace shape so
//! Trellis (downstream) can verify WOS-published traces. The
//! `to_conformance_trace` helper produces a `serde_json::Value`
//! schema-compatible with `wos-tooling.schema.json#/$defs/ConformanceTrace`.

pub mod runner;
pub mod scenario_type;
pub mod trace;

pub use runner::{
    ScenarioOutcome, ScenarioRunResult, run_scenario, run_workspace,
    run_workspace_with_options,
};
pub use scenario_type::{ScenarioType, parse_scenario_type};
pub use trace::{ActualTrace, ExpectedTrace, TraceDelta, TraceStep};
