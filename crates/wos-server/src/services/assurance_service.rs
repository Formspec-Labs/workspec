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

/// NIST 800-63 style assurance level. Invariant 6: independent of
/// `DisclosurePosture` — the two fields are distinct dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AssuranceLevel {
    L1,
    L2,
    L3,
    L4,
}

impl AssuranceLevel {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::L1 => "l1",
            Self::L2 => "l2",
            Self::L3 => "l3",
            Self::L4 => "l4",
        }
    }
}

/// Subject-disclosure posture. Invariant 6: independent of `AssuranceLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DisclosurePosture {
    Open,
    Minimal,
    None,
}

impl DisclosurePosture {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Minimal => "minimal",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordFactRequest {
    pub subject_ref: String,
    pub assurance_level: AssuranceLevel,
    pub disclosure_posture: DisclosurePosture,
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
    pub new_assurance_level: AssuranceLevel,
    /// Optional new disclosure posture; defaults to the prior row's posture.
    #[serde(default)]
    pub new_disclosure_posture: Option<DisclosurePosture>,
    pub basis: serde_json::Value,
}

/// Response for `GET /api/subjects/{ref}/assurance-chain` (WS-037). Wraps
/// the assurance-chain list with chain-continuity metadata so callers can
/// detect a broken `upgradedFrom` link without re-walking the chain.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssuranceChainResponse {
    pub facts: Vec<IdentityFactView>,
    pub chain_valid: bool,
    /// **Polarity:** the **child** fact id whose `upgraded_from` link is
    /// dangling — i.e. the fact in `facts` whose `upgraded_from` does not
    /// resolve to any id present in the returned set. Callers seeking the
    /// dangling reference itself must consult
    /// `facts.iter().find(|f| f.id == broken_at).and_then(|f| f.upgraded_from.clone())`.
    /// `None` when the chain is intact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broken_at: Option<String>,
}

pub struct AssuranceService;

impl AssuranceService {
    pub async fn record_fact(
        storage: &StorageHandle,
        instance_id: &str,
        req: RecordFactRequest,
    ) -> ApiResult<IdentityFactView> {
        let row = IdentityFactRow {
            id: format!("urn:wos:identity-fact:{}", Uuid::now_v7()),
            instance_id: instance_id.to_string(),
            subject_ref: req.subject_ref,
            assurance_level: req.assurance_level.as_wire().to_string(),
            disclosure_posture: req.disclosure_posture.as_wire().to_string(),
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
            .map(|p| p.as_wire().to_string())
            .unwrap_or_else(|| prior.disclosure_posture.clone());
        let row = IdentityFactRow {
            id: format!("urn:wos:identity-fact:{}", Uuid::now_v7()),
            instance_id: prior.instance_id.clone(),
            subject_ref: prior.subject_ref.clone(),
            assurance_level: req.new_assurance_level.as_wire().to_string(),
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

    /// Walks the assurance chain for `subject_ref` and reports whether each
    /// `upgraded_from` reference resolves within the returned set. Used by
    /// WS-037 to surface broken upgrade links without forcing the caller to
    /// re-walk the chain client-side.
    pub async fn assurance_chain_with_validation(
        storage: &StorageHandle,
        subject_ref: &str,
    ) -> ApiResult<AssuranceChainResponse> {
        let facts: Vec<IdentityFactView> = storage
            .list_assurance_chain(subject_ref)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        let known: std::collections::HashSet<&str> =
            facts.iter().map(|f| f.id.as_str()).collect();
        let broken_at = facts
            .iter()
            .find(|f| {
                f.upgraded_from
                    .as_deref()
                    .is_some_and(|prior| !known.contains(prior))
            })
            .map(|f| f.id.clone());
        Ok(AssuranceChainResponse {
            chain_valid: broken_at.is_none(),
            broken_at,
            facts,
        })
    }
}

