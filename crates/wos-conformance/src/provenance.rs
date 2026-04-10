// Rust guideline compliant 2026-02-21

//! Provenance records produced during conformance test execution.
//!
//! Re-exports from [`wos_core::provenance`] — the canonical provenance
//! types live in wos-core; this module provides the public API surface
//! that conformance tests import.

pub use wos_core::provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};
