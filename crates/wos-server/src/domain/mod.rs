//! Server-side view types.
//!
//! Design: the WOS spec *does not* mandate an HTTP surface, so the REST
//! paths here are a server convention. The **data types** flowing through
//! those paths, however, are the spec — so runtime types like
//! `CaseInstance`, `ProvenanceRecord`, and `KernelDocument` come straight
//! from `wos_core` and are returned by serde round-trip without shadowing.
//!
//! This module is therefore only the glue that the server itself adds on
//! top of `wos-core`: auth tokens, pagination, server-composed bundles,
//! and UX-shaped projections (dashboard, applicant, governance) that
//! don't exist in the spec core.

pub mod applicant;
pub mod auth;
pub mod bundle;
pub mod dashboard;
pub mod governance;
pub mod instance;
pub mod provenance;

pub use applicant::*;
pub use auth::*;
pub use bundle::*;
pub use dashboard::*;
pub use governance::*;
pub use instance::*;
pub use provenance::*;
