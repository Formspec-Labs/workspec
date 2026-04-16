//! Provenance export to PROV-O JSON-LD, IEEE 1849 XES, and OCEL 2.0.
//!
//! Implements the WOS Semantic Profile §5 (PROV-O) and §6 (XES/OCEL).
//! Takes a [`wos_core::provenance::ProvenanceLog`] and export configuration
//! and produces serialized output in the requested format.

pub mod ocel;
pub mod prov_o;
pub mod xes;

/// Export configuration derived from a Semantic Profile Document.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Base namespace for minting provenance IRIs (PROV-O §5.2).
    pub provenance_namespace: String,
    /// Instance ID used as the XES case identifier / OCEL case reference.
    pub instance_id: String,
}
