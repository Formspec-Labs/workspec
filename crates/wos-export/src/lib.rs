// Rust guideline compliant 2026-04-16

//! Provenance export to PROV-O JSON-LD, IEEE 1849 XES, and OCEL 2.0.
//!
//! Implements the WOS Semantic Profile §5 (PROV-O) and §6 (XES/OCEL).
//! Takes a [`wos_core::provenance::ProvenanceLog`] and export configuration
//! and produces serialized output in the requested format.
//!
//! # Empty-timestamp policy (intentionally divergent)
//!
//! The three serializers handle `ProvenanceRecord.timestamp == ""` (see
//! [`wos_core::provenance::ProvenanceRecord::timestamp`]) differently, by
//! design. Each policy is driven by its target format's schema and must
//! not be "harmonized" by a future maintainer:
//!
//! - **PROV-O** omits `prov:atTime` entirely. An empty string is not a
//!   valid `xsd:dateTime` literal and would poison any JSON-LD consumer.
//! - **XES** omits the `<date key="time:timestamp" .../>` element. An
//!   empty string is not a valid `xs:dateTime` and IEEE 1849 consumers
//!   treat the missing element as "timestamp unknown".
//! - **OCEL** emits `"time": ""` verbatim. OCEL 2.0 **requires** a `time`
//!   field on every event; omission produces an invalid document. The
//!   empty string surfaces the missed stamping site to downstream tools
//!   rather than papering over it.
//!
//! If a future change introduces a uniform policy (e.g. populating all
//! three with a sentinel), it MUST update all three serializers together
//! and remove this note.

use serde_json::Value;

use wos_core::provenance::ProvenanceRecord;

pub mod ocel;
pub mod prov_o;
pub mod xes;

/// Export configuration derived from a Semantic Profile Document.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Base namespace for minting provenance IRIs (PROV-O §5.2).
    ///
    /// This string is concatenated directly with record identifiers to form
    /// activity IRIs (`{provenance_namespace}{index}`) and agent IRIs
    /// (`{provenance_namespace}agent/{actor_id}`). Callers are therefore
    /// responsible for including the IRI-path separator: the namespace
    /// MUST end with either `:` (for URN-style bases) or `/` (for
    /// HTTP-style bases). Omitting the separator produces malformed IRIs
    /// that PROV-O validators will reject.
    ///
    /// Examples of well-formed values:
    /// - `"urn:wos:prov:grant-2026:"` (URN)
    /// - `"https://example.org/prov/grant-2026/"` (HTTP)
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

/// §6.5 scope predicate shared by every exporter.
///
/// A record is included in the default Facts-tier export iff its
/// `audit_layer` is absent (legacy records, pre-extension runtimes) or
/// explicitly `"facts"`. Higher-tier records (`"narrative"`, `"reasoning"`,
/// `"counterfactual"`) are routed to Bundle exports, not the default
/// Facts-tier surface.
///
/// Kept in one place so any future refinement of "what counts as Facts
/// tier" applies uniformly to PROV-O, XES, and OCEL outputs.
pub(crate) fn is_facts_tier(record: &ProvenanceRecord) -> bool {
    matches!(record.audit_layer.as_deref(), None | Some("facts"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::provenance::ProvenanceRecord;

    fn record_with_layer(layer: Option<&str>) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
        record.audit_layer = layer.map(String::from);
        record
    }

    #[test]
    fn is_facts_tier_accepts_none_and_facts() {
        assert!(is_facts_tier(&record_with_layer(None)));
        assert!(is_facts_tier(&record_with_layer(Some("facts"))));
    }

    #[test]
    fn is_facts_tier_rejects_higher_tiers() {
        for higher in ["narrative", "reasoning", "counterfactual"] {
            assert!(
                !is_facts_tier(&record_with_layer(Some(higher))),
                "{higher}-tier records must be excluded from the default facts export"
            );
        }
    }
}
