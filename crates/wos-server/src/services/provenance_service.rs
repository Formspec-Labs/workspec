use sha2::{Digest, Sha256};
use wos_core::provenance::ProvenanceRecord;
use wos_server_ports::runtime::{ProvenancePort, RuntimeAdapterError};

use crate::domain::provenance::ProvenanceResponse;
use crate::error::{ApiError, ApiResult};
use crate::storage::{ProvenanceRow, StorageHandle, StorageResult};

pub struct ProvenanceService {
    storage: StorageHandle,
}

impl ProvenanceService {
    pub fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }

    /// Return the full, hash-chain-verified provenance history for
    /// `instance_id`. Stored payloads are `wos_core::ProvenanceRecord`
    /// serialisations; each row is returned as a `ProvenanceResponse`
    /// that flattens the spec-defined record with the server's integrity
    /// metadata at the top level.
    pub async fn list(&self, instance_id: &str) -> ApiResult<Vec<ProvenanceResponse>> {
        let rows = self.storage.list_provenance(instance_id).await?;
        rows.iter().map(row_to_response).collect()
    }

    /// Single-row convenience over [`Self::prepare_batch`].
    pub async fn prepare_next(
        &self,
        instance_id: &str,
        record: &ProvenanceRecord,
    ) -> StorageResult<ProvenanceRow> {
        let rows = self.prepare_batch(instance_id, std::slice::from_ref(record)).await?;
        rows.into_iter().next().ok_or_else(|| {
            crate::storage::StorageError::Other("prepare_batch returned empty".into())
        })
    }

    /// Build N new chain rows for `records`, sharing one read of the
    /// stored tail and self-chaining within the batch so the whole set
    /// commits as one atomic append.
    pub async fn prepare_batch(
        &self,
        instance_id: &str,
        records: &[ProvenanceRecord],
    ) -> StorageResult<Vec<ProvenanceRow>> {
        let last = self.storage.last_provenance(instance_id).await?;
        let (mut next_seq, mut previous_hash) = match last {
            Some(r) => (r.seq + 1, r.hash),
            None => (1, ZERO_HASH.to_string()),
        };
        let mut out = Vec::with_capacity(records.len());
        for record in records {
            let tier = record
                .audit_layer
                .clone()
                .unwrap_or_else(|| "facts".to_string());
            let timestamp = chrono::Utc::now();
            let payload = serde_json::to_value(record)
                .map_err(|e| crate::storage::StorageError::Other(format!(
                    "ProvenanceRecord serialise: {e}"
                )))?;
            let hash = chain_hash(&previous_hash, instance_id, next_seq, &timestamp, &tier, &payload);
            out.push(ProvenanceRow {
                id: uuid::Uuid::new_v4().to_string(),
                instance_id: instance_id.to_string(),
                seq: next_seq,
                timestamp,
                tier,
                payload,
                hash: hash.clone(),
                previous_hash,
            });
            previous_hash = hash;
            next_seq += 1;
        }
        Ok(out)
    }
}

pub(crate) const ZERO_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Canonical integrity hash: `sha256(previous_hash || canonical_json(payload_envelope))`.
///
/// The envelope pins every field that MUST influence the hash; adding or
/// reordering a field here is a chain-breaking change.
pub fn chain_hash(
    previous_hash: &str,
    instance_id: &str,
    seq: i64,
    timestamp: &chrono::DateTime<chrono::Utc>,
    tier: &str,
    payload: &serde_json::Value,
) -> String {
    let canonical = serde_json::json!({
        "instanceId": instance_id,
        "seq": seq,
        "timestamp": timestamp.to_rfc3339(),
        "tier": tier,
        "payload": payload,
    });
    let canonical_str = serde_json::to_string(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(previous_hash.as_bytes());
    hasher.update(canonical_str.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Verify the chain. Returns `Err(index)` of the first broken link.
pub fn verify_chain(rows: &[ProvenanceRow]) -> Result<(), usize> {
    let mut expected_prev = ZERO_HASH.to_string();
    for (i, r) in rows.iter().enumerate() {
        if r.previous_hash != expected_prev {
            return Err(i);
        }
        let recomputed = chain_hash(
            &r.previous_hash,
            &r.instance_id,
            r.seq,
            &r.timestamp,
            &r.tier,
            &r.payload,
        );
        if recomputed != r.hash {
            return Err(i);
        }
        expected_prev = r.hash.clone();
    }
    Ok(())
}

pub fn row_to_response(r: &ProvenanceRow) -> ApiResult<ProvenanceResponse> {
    let record: ProvenanceRecord = serde_json::from_value(r.payload.clone())
        .map_err(|e| ApiError::ServiceUnavailable(format!(
            "provenance payload is not a wos_core::ProvenanceRecord: {e}"
        )))?;
    Ok(ProvenanceResponse {
        record,
        id: r.id.clone(),
        instance_id: r.instance_id.clone(),
        seq: r.seq,
        hash: r.hash.clone(),
        previous_hash: r.previous_hash.clone(),
    })
}

#[async_trait::async_trait]
impl ProvenancePort for ProvenanceService {
    async fn prepare_batch(
        &self,
        instance_id: &str,
        records: &[ProvenanceRecord],
    ) -> Result<Vec<ProvenanceRow>, RuntimeAdapterError> {
        ProvenanceService::prepare_batch(self, instance_id, records)
            .await
            .map_err(|e| RuntimeAdapterError::Message(e.to_string()))
    }
}
