//! Deontic-violation listing — a filtered view over the provenance chain.
//!
//! The runtime already writes deontic/autonomy/confidence records with
//! the right `ProvenanceKind` values (DeonticViolation, RightsViolation,
//! ConsistencyViolation, DeonticBypass, AutonomyViolation,
//! AutonomyCapped, ConfidenceViolation, ToolViolation, ...). This service
//! pulls them out of the chain, grouped by kind, so consumers don't have
//! to scan the whole provenance log.

use serde::Serialize;
use wos_core::provenance::ProvenanceKind;

use crate::error::ApiResult;
use crate::services::provenance_service::ProvenanceService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViolationView {
    pub seq: i64,
    pub timestamp: String,
    pub kind: String,
    pub event: Option<String>,
    pub actor_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub hash: String,
}

pub struct DeonticService;

impl DeonticService {
    pub async fn list(
        provenance: &ProvenanceService,
        instance_id: &str,
    ) -> ApiResult<Vec<ViolationView>> {
        let rows = provenance.list(instance_id).await?;
        Ok(rows
            .into_iter()
            .filter(|r| is_violation(r.record.record_kind))
            .map(|r| ViolationView {
                seq: r.seq,
                timestamp: r.record.timestamp.clone(),
                kind: serde_json::to_value(r.record.record_kind)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                event: r.record.event.clone(),
                actor_id: r.record.actor_id.clone(),
                data: r.record.data.clone(),
                hash: r.hash.clone(),
            })
            .collect())
    }
}

fn is_violation(kind: ProvenanceKind) -> bool {
    use ProvenanceKind as K;
    matches!(
        kind,
        K::DeonticViolation
            | K::RightsViolation
            | K::ConsistencyViolation
            | K::DeonticBypass
            | K::AutonomyViolation
            | K::AutonomyCapped
            | K::ConfidenceViolation
            | K::ToolViolation
            | K::ToleranceViolation
    )
}
