// Rust guideline compliant 2026-02-21

//! Provenance records produced during conformance test execution.
//!
//! Re-exports of the WOS event vocabulary used by conformance tests.
//!
//! The canonical module lives in `wos-events`; `wos-core` re-exports the
//! public item types at its crate root because evaluator APIs produce
//! provenance records.

pub use wos_core::{ProvenanceKind, ProvenanceRecord};
