// Rust guideline compliant 2026-05-02

//! Published Studio-facing surface of `wos-runtime`.
//!
//! Mirrors the boundary in `wos_core::studio_api` and `wos_lint::studio_api`.
//! Studio authoring crates consume runtime types through this module. The
//! sibling Policy Studio boundary guard lives at
//! `policy-studio/crates/wos-studio-types/tests/api_surface.rs`.
//!
//! [`DurableRuntime`] is the trait implemented by runtime adapters, including
//! in-memory and hosted adapters. Frozen candidate scenario replay is owned by
//! `wos_conformance::studio_api`, which uses the maintained in-memory runtime
//! and returns observed traces and explicit fixture limitations. Runtime does
//! not re-export that API because conformance already depends on runtime.

pub use crate::DurableRuntime;
