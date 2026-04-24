//! Server-specific provenance envelope.
//!
//! The spec's provenance type is `wos_core::provenance::ProvenanceRecord`;
//! this module adds the hash-chain integrity fields the server itself
//! mints (`hash`, `previousHash`, `seq`) without shadowing the record's
//! spec-defined shape.

use serde::{Deserialize, Serialize};
use wos_core::provenance::ProvenanceRecord;

/// `GET /api/instances/:id/provenance[]` element.
///
/// Serializes as a flattened `ProvenanceRecord` plus the server's hash
/// chain metadata at the top level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceResponse {
    #[serde(flatten)]
    pub record: ProvenanceRecord,
    /// Stable server-minted row identifier.
    pub id: String,
    /// Instance identifier (redundant with the path but useful when
    /// records are consumed in isolation).
    pub instance_id: String,
    /// 1-indexed sequence number within the instance's chain.
    pub seq: i64,
    /// Integrity hash: same preimage as [`crate::services::provenance_service::chain_hash`]
    /// — `sha256` over UTF-8 `previous_hash` bytes plus canonical JSON of
    /// `{ instanceId, seq, timestamp, tier, payload }` (not the raw record alone).
    pub hash: String,
    /// Hash of the preceding row, or `sha256:0…` for the genesis record.
    pub previous_hash: String,
}
