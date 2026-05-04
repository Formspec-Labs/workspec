// Rust guideline compliant 2026-05-02

//! Published Studio-facing surface of `wos-runtime`.
//!
//! Mirrors the contract laid out in `wos_core::studio_api` /
//! `wos_lint::studio_api`. Studio (Authoring) crates may consume
//! `wos-runtime` ONLY through this module. Boundary is enforced by
//! one workspace-wide guard test at
//! `studio/crates/wos-studio-types/tests/api_surface.rs`.
//!
//! ## What lives here
//!
//! - [`DurableRuntime`] — the trait every adapter implements (in-memory,
//!   Restate, Temporal). Studio's scenario simulator (Wave 3 of the
//!   decoupling plan) takes a `&dyn DurableRuntime` and replays scenario
//!   step events against it.
//!
//! ## What does NOT live here (yet)
//!
//! Wave 3 will add a Studio-specific `replay(&KernelDocument, &[Event])
//! -> ActualTrace` surface — a thin wrapper over the in-memory runtime
//! that returns scenarios in the conformance-trace shape. Held until
//! Wave 3 has a concrete consumer to design against (the Studio scenario
//! runner). See `studio/specs/scenario-authoring.md` for the contract.

pub use crate::DurableRuntime;
