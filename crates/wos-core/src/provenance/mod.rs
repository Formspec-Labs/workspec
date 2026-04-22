// Rust guideline compliant 2026-02-21

//! Provenance recording for workflow execution.
//!
//! Every action that changes lifecycle or case state produces a provenance
//! record (Kernel S8). The provenance log is append-only.
//!
//! Split across `snapshot`, `kind`, `audit_tier`, `record`, and `log` so
//! `ProvenanceKind` growth does not monopolize a single merge-bottleneck file.

mod audit_tier;
mod kind;
mod log;
mod record;
mod snapshot;

#[cfg(test)]
mod tests;

pub use audit_tier::{ProvenanceAuditTier, audit_layer_for_kind};
pub use kind::ProvenanceKind;
pub use log::ProvenanceLog;
pub use record::{ProvenanceRecord, SignatureAffirmationInput};
pub use snapshot::CaseFileSnapshot;
