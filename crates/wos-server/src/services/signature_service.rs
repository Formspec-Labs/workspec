//! Signature-affirmation listing — a filtered view over the provenance chain.
//!
//! The runtime writes `SignatureAffirmation` provenance records when a signing
//! act occurs. This service filters those records from the hash-chained
//! provenance log and returns them as standard `ProvenanceResponse` envelopes
//! in sequence order.

use wos_core::provenance::ProvenanceKind;

use crate::domain::provenance::ProvenanceResponse;
use crate::error::ApiResult;
use crate::services::provenance_service::ProvenanceService;

pub struct SignatureService;

impl SignatureService {
    pub async fn list(
        provenance: &ProvenanceService,
        instance_id: &str,
    ) -> ApiResult<Vec<ProvenanceResponse>> {
        let rows = provenance.list(instance_id).await?;
        Ok(rows
            .into_iter()
            .filter(|r| matches!(r.record.record_kind, ProvenanceKind::SignatureAffirmation))
            .collect())
    }
}
