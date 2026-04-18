//! Deontic-violation listing — a filtered view over the provenance chain.
//!
//! The runtime writes deontic/autonomy/confidence records with the
//! corresponding `ProvenanceKind` values. This service filters those
//! kinds out of the hash-chained provenance log and returns them as
//! standard `ProvenanceResponse` envelopes so the client sees exactly
//! the same shape as `/provenance` — just pre-filtered.

use wos_core::provenance::ProvenanceKind;

use crate::domain::provenance::ProvenanceResponse;
use crate::error::ApiResult;
use crate::services::provenance_service::ProvenanceService;

pub struct DeonticService;

impl DeonticService {
    pub async fn list(
        provenance: &ProvenanceService,
        instance_id: &str,
    ) -> ApiResult<Vec<ProvenanceResponse>> {
        let rows = provenance.list(instance_id).await?;
        Ok(rows
            .into_iter()
            .filter(|r| is_violation(r.record.record_kind))
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
