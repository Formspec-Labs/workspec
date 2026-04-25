// Rust guideline compliant 2026-02-21

//! `ProvenanceSigner` implementations. `NoopSigner` is the default — it
//! produces spec-correct `attestation` blocks with empty signatures so the
//! wire shape is consistent even when cryptographic signing is not deployed.
//! An `Ed25519FileKeySigner` reference impl will ship behind a feature flag
//! (WS-043) when deployment needs externally-verifiable signatures.

use thiserror::Error;
use wos_core::provenance::ProvenanceRecord;
use wos_core::traits::ProvenanceSigner;

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("signer error: {0}")]
    Other(String),
}

/// No-operation signer. Produces an empty signature byte vector and
/// verifies any signature as `true`, matching the reference-server posture
/// where chain-integrity hashes (not cryptographic signatures) are the
/// tamper-evidence mechanism.
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
