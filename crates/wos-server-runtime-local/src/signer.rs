// Rust guideline compliant 2026-02-21

use thiserror::Error;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::ProvenanceSigner;

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("signer error: {0}")]
    Other(String),
}

#[derive(Debug, Default)]
pub struct NoopSigner;

impl ProvenanceSigner for NoopSigner {
    type Error = SignerError;

    fn sign(&self, _record: &ProvenanceRecord) -> Result<Vec<u8>, Self::Error> {
        Ok(Vec::new())
    }

    fn verify(&self, _record: &ProvenanceRecord, _signature: &[u8]) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
