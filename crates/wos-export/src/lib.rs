//! Provenance export to PROV-O JSON-LD, IEEE 1849 XES, and OCEL 2.0.
//!
//! Implements the WOS Semantic Profile §5 (PROV-O) and §6 (XES/OCEL).
//! Takes a [`wos_core::provenance::ProvenanceLog`] and export configuration
//! and produces serialized output in the requested format.

use serde_json::Value;

use wos_core::provenance::ProvenanceRecord;

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

/// Render `record_kind` in camelCase by reusing the serde rename already
/// attached to [`wos_core::provenance::ProvenanceKind`]. Keeping a single
/// source of truth means PROV-O, XES, and OCEL all emit the identical
/// on-the-wire string (§5.3, §6.3, §6.4 examples: `"stateTransition"`).
pub(crate) fn camel_case_record_kind(record: &ProvenanceRecord) -> String {
    match serde_json::to_value(record.record_kind) {
        Ok(Value::String(name)) => name,
        // `ProvenanceKind` is `#[serde(rename_all = "camelCase")]` over plain
        // unit variants, so serialization cannot fail or yield a non-string.
        // Any surprise here is a bug in the enum definition or serde.
        other => unreachable!("ProvenanceKind must serialize as a string, got {other:?}"),
    }
}
