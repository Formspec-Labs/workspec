use sha2::{Digest, Sha256};

use crate::domain::{
    ActorRef, CounterfactualView, CriteriaCheckView, FactsView, IntegrityView,
    ProvenanceRecordView, ReasoningView,
};
use crate::storage::{ProvenanceRow, StorageHandle, StorageResult};

pub struct ProvenanceService {
    storage: StorageHandle,
}

impl ProvenanceService {
    pub fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }

    pub async fn list(&self, instance_id: &str) -> StorageResult<Vec<ProvenanceRecordView>> {
        let rows = self.storage.list_provenance(instance_id).await?;
        Ok(rows.iter().map(row_to_view).collect())
    }

    /// Build the next [`ProvenanceRow`] in the chain for `instance_id`,
    /// computing `previous_hash` from the last stored row (or a genesis
    /// zero-hash when the chain is empty).
    pub async fn prepare_next(
        &self,
        instance_id: &str,
        tier: &str,
        payload: serde_json::Value,
    ) -> StorageResult<ProvenanceRow> {
        let last = self.storage.last_provenance(instance_id).await?;
        let (seq, previous_hash) = match last {
            Some(r) => (r.seq + 1, r.hash),
            None => (1, ZERO_HASH.to_string()),
        };
        let timestamp = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        let hash = chain_hash(&previous_hash, instance_id, seq, &timestamp, tier, &payload);
        Ok(ProvenanceRow {
            id,
            instance_id: instance_id.to_string(),
            seq,
            timestamp,
            tier: tier.to_string(),
            payload,
            hash,
            previous_hash,
        })
    }
}

const ZERO_HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Canonical hash: `sha256(previous_hash || canonical_json(record))`.
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

/// Verify the hash chain. Returns `Err(index)` of the first broken link.
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

fn row_to_view(r: &ProvenanceRow) -> ProvenanceRecordView {
    // The payload is free-form; try to project known shapes.
    fn s(v: &serde_json::Value, k: &str) -> Option<String> {
        v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
    }

    let actor = r
        .payload
        .get("actor")
        .map(|a| ActorRef {
            id: s(a, "id").unwrap_or_else(|| "system".into()),
            actor_type: s(a, "type").unwrap_or_else(|| "system".into()),
            name: s(a, "name").unwrap_or_else(|| "system".into()),
        })
        .unwrap_or(ActorRef {
            id: "system".into(),
            actor_type: "system".into(),
            name: "System".into(),
        });

    let facts = r
        .payload
        .get("facts")
        .map(|f| FactsView {
            inputs: f.get("inputs").cloned().unwrap_or(serde_json::json!({})),
            outputs: f.get("outputs").cloned().unwrap_or(serde_json::json!({})),
            metadata: f.get("metadata").cloned().unwrap_or(serde_json::json!({})),
        })
        .unwrap_or(FactsView {
            inputs: serde_json::json!({}),
            outputs: serde_json::json!({}),
            metadata: serde_json::json!({}),
        });

    let reasoning = r.payload.get("reasoning").map(|x| ReasoningView {
        rules_applied: x
            .get("rulesApplied")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        criteria_checked: x
            .get("criteriaChecked")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|c| CriteriaCheckView {
                        label: s(c, "label").unwrap_or_default(),
                        passed: c.get("passed").and_then(|v| v.as_bool()).unwrap_or(false),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        explanation: s(x, "explanation"),
        source_authority: s(x, "sourceAuthority"),
    });

    let counterfactual = r.payload.get("counterfactual").map(|x| CounterfactualView {
        positive: x
            .get("positive")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        negative: x
            .get("negative")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
    });

    ProvenanceRecordView {
        id: r.id.clone(),
        instance_id: r.instance_id.clone(),
        timestamp: r.timestamp.to_rfc3339(),
        tier: r.tier.clone(),
        actor,
        event: s(&r.payload, "event").unwrap_or_default(),
        source_state: s(&r.payload, "sourceState").unwrap_or_default(),
        target_state: s(&r.payload, "targetState").unwrap_or_default(),
        facts,
        reasoning,
        ai_narrative: None,
        counterfactual,
        authority_chain: None,
        integrity: IntegrityView {
            hash: r.hash.clone(),
            previous_hash: r.previous_hash.clone(),
        },
    }
}
