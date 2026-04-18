//! Assurance Layer (identity facts, subject continuity, assurance upgrade).
//!
//! Every identity fact is recorded once, with an independent
//! `assuranceLevel` (L1–L4) and `disclosurePosture` (open | minimal | none).
//! Invariant 6: `assuranceLevel` and `disclosurePosture` are independent —
//! the server enforces they are separate fields that never conflate.
//! Upgrades are forward-only: a new row with `upgradedFrom` = the prior
//! row's id.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::storage::{IdentityFactRow, StorageHandle};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordFactRequest {
    pub subject_ref: String,
    pub assurance_level: String,
    pub disclosure_posture: String,
    pub fact: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityFactView {
    pub id: String,
    pub instance_id: String,
    pub subject_ref: String,
    pub assurance_level: String,
    pub disclosure_posture: String,
    pub fact: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgraded_from: Option<String>,
    pub created_at: String,
}

impl From<IdentityFactRow> for IdentityFactView {
    fn from(r: IdentityFactRow) -> Self {
        Self {
            id: r.id,
            instance_id: r.instance_id,
            subject_ref: r.subject_ref,
            assurance_level: r.assurance_level,
            disclosure_posture: r.disclosure_posture,
            fact: r.fact_json,
            upgraded_from: r.upgraded_from,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeRequest {
    pub new_assurance_level: String,
    /// Optional new disclosure posture; defaults to the prior row's posture.
    #[serde(default)]
    pub new_disclosure_posture: Option<String>,
    pub basis: serde_json::Value,
}

pub struct AssuranceService;

impl AssuranceService {
    pub async fn record_fact(
        storage: &StorageHandle,
        instance_id: &str,
        req: RecordFactRequest,
    ) -> ApiResult<IdentityFactView> {
        // Invariant 6 guard: the assurance level and disclosure posture must
        // be independent fields and must not collide with a combined "level
        // + posture" string.
        validate_invariant_6(&req.assurance_level, &req.disclosure_posture)?;
        let row = IdentityFactRow {
            id: format!("urn:wos:identity-fact:{}", Uuid::new_v4()),
            instance_id: instance_id.to_string(),
            subject_ref: req.subject_ref,
            assurance_level: req.assurance_level,
            disclosure_posture: req.disclosure_posture,
            fact_json: req.fact,
            upgraded_from: None,
            created_at: Utc::now(),
        };
        storage.insert_identity_fact(&row).await?;
        Ok(row.into())
    }

    pub async fn upgrade(
        storage: &StorageHandle,
        fact_id: &str,
        req: UpgradeRequest,
    ) -> ApiResult<IdentityFactView> {
        let prior = storage
            .get_identity_fact(fact_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let new_posture = req
            .new_disclosure_posture
            .unwrap_or_else(|| prior.disclosure_posture.clone());
        validate_invariant_6(&req.new_assurance_level, &new_posture)?;
        let row = IdentityFactRow {
            id: format!("urn:wos:identity-fact:{}", Uuid::new_v4()),
            instance_id: prior.instance_id.clone(),
            subject_ref: prior.subject_ref.clone(),
            assurance_level: req.new_assurance_level,
            disclosure_posture: new_posture,
            fact_json: req.basis,
            upgraded_from: Some(prior.id.clone()),
            created_at: Utc::now(),
        };
        storage.insert_identity_fact(&row).await?;
        Ok(row.into())
    }

    pub async fn list_for_instance(
        storage: &StorageHandle,
        instance_id: &str,
    ) -> ApiResult<Vec<IdentityFactView>> {
        Ok(storage
            .list_identity_facts(instance_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn assurance_chain(
        storage: &StorageHandle,
        subject_ref: &str,
    ) -> ApiResult<Vec<IdentityFactView>> {
        Ok(storage
            .list_assurance_chain(subject_ref)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

fn validate_invariant_6(level: &str, posture: &str) -> ApiResult<()> {
    // Levels: l1 | l2 | l3 | l4. Postures: open | minimal | none.
    let valid_level = matches!(level.to_ascii_lowercase().as_str(), "l1" | "l2" | "l3" | "l4");
    let valid_posture =
        matches!(posture.to_ascii_lowercase().as_str(), "open" | "minimal" | "none");
    if !valid_level {
        return Err(ApiError::BadRequest(format!(
            "invalid assuranceLevel `{level}` — expected one of L1..L4"
        )));
    }
    if !valid_posture {
        return Err(ApiError::BadRequest(format!(
            "invalid disclosurePosture `{posture}` — expected one of open|minimal|none"
        )));
    }
    Ok(())
}
