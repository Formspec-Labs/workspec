// Rust guideline compliant 2026-05-02

//! Shared types + re-exports consumed by every Studio (Authoring) crate.
//!
//! Today this crate is a thin shim: it re-exports the
//! `wos_core::studio_api` surface under a Studio-local name so downstream
//! Studio crates (`wos-studio-model`, `wos-studio-lint`,
//! `wos-studio-compiler`, `wos-studio-scenario`) all import from one
//! place. As the Studio tier grows (Wave 1.4 onward), Studio-only shared
//! types — workspace identifiers, mapping-state vocabulary, lifecycle
//! state enums — accumulate here.
//!
//! ## Boundary contract
//!
//! Studio crates MUST consume `wos-core` ONLY through `kernel::*`
//! re-exports below (or directly through `wos_core::studio_api` for
//! types not yet re-exported here). Importing
//! `wos_core::model::kernel::*` (for example) directly is a guard-test
//! failure; see `tests/api_surface.rs`.

/// Re-exports of the kernel-tier types Studio code consumes.
///
/// Equivalent to `wos_core::studio_api::*`. Use this name when
/// importing into Studio code so the boundary is visible at every call
/// site. See [`wos_core::studio_api`] for the full surface.
pub mod kernel {
    pub use wos_core::studio_api::*;
}

/// Stage 7 architectural contract: Studio-side port + adapter-seam
/// trait stubs and shared type aliases. See
/// `studio/specs/reference-architecture.md` and ADRs 0086–0091.
pub mod arch;
