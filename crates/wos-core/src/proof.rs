//! Cross-stack proof attestation domain model for WOS-T4.
//!
//! Ticket: fs-w0li (WOS-T4 cross-stack proof + signature-attestation closeout)
//! Related: ADR 0083, ADR 0085, ADR 0086, ADR 0110, ADR 0111
//!
//! This module defines the typed domain representation for cross-stack proof
//! attestations across Formspec (response & signatures), WOS (case governance
//! & lifecycle acts), and Trellis (custody ledger & export verification).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Status of the cross-stack proof attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofAttestationStatus {
    /// Proof fully satisfied with all signatures valid and ledger unbroken
    Attested,
    /// Cryptographic invalidity or broken chain detected
    Defective,
    /// Missing required signature acts, receipts, or unanchored ledger
    Incomplete,
}

/// Errors detected during cross-stack proof validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProofValidationError {
    #[error("missing required participant role: {role}")]
    MissingRequiredRole { role: String },
    #[error("case file digest mismatch: expected {expected}, found {actual}")]
    CaseDigestMismatch { expected: String, actual: String },
    #[error("signing act {act_id} failed cryptographic verification")]
    InvalidSigningAct { act_id: String },
    #[error("ledger stream sequence head is 0 or uncommitted")]
    EmptyLedgerStream,
    #[error("missing receipt commitment for signing act {act_id}")]
    MissingReceiptCommitment { act_id: String },
}

/// Attestation of a verified signing act within the governed workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningActAttestation {
    pub act_id: String,
    pub actor_id: String,
    pub role: String,
    pub payload_digest: String,
    pub signed_at: DateTime<Utc>,
    pub intent: String,
    pub signature_valid: bool,
}

/// Attestation of the immutable ledger stream checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerCheckpointAttestation {
    pub stream_id: String,
    pub merkle_root: String,
    pub sequence_head: u64,
}

/// Attestation of a deployment-class verification receipt commitment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptCommitmentAttestation {
    pub receipt_id: String,
    pub act_id: String,
    pub issuer_key_class: String,
    pub receipt_digest: String,
}

/// Canonical Cross-Stack Proof Attestation container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossStackProofAttestation {
    pub case_id: String,
    pub case_file_digest: String,
    pub signing_acts: Vec<SigningActAttestation>,
    pub ledger_checkpoint: LedgerCheckpointAttestation,
    pub receipt_commitments: Vec<ReceiptCommitmentAttestation>,
    pub verified_at: DateTime<Utc>,
    pub status: ProofAttestationStatus,
}

impl CrossStackProofAttestation {
    /// Validates completeness of the proof attestation against required participant roles
    /// and expected case file digest.
    pub fn validate_completeness(
        &self,
        required_roles: &[&str],
        expected_case_digest: &str,
    ) -> Result<(), ProofValidationError> {
        // 1. Check case file digest continuity
        if self.case_file_digest != expected_case_digest {
            return Err(ProofValidationError::CaseDigestMismatch {
                expected: expected_case_digest.to_string(),
                actual: self.case_file_digest.clone(),
            });
        }

        // 2. Check ledger checkpoint continuity
        if self.ledger_checkpoint.sequence_head == 0 {
            return Err(ProofValidationError::EmptyLedgerStream);
        }

        // 3. Check that all required roles have at least one valid signing act
        for role in required_roles {
            let role_present = self.signing_acts.iter().any(|act| {
                act.role == *role && act.signature_valid
            });
            if !role_present {
                return Err(ProofValidationError::MissingRequiredRole {
                    role: role.to_string(),
                });
            }
        }

        // 4. Verify all signing acts are marked valid
        for act in &self.signing_acts {
            if !act.signature_valid {
                return Err(ProofValidationError::InvalidSigningAct {
                    act_id: act.act_id.clone(),
                });
            }

            // 5. Ensure each signing act has an associated receipt commitment
            let has_receipt = self
                .receipt_commitments
                .iter()
                .any(|r| r.act_id == act.act_id);
            if !has_receipt {
                return Err(ProofValidationError::MissingReceiptCommitment {
                    act_id: act.act_id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Evaluates and sets the overall attestation status.
    pub fn evaluate_status(&mut self, required_roles: &[&str], expected_case_digest: &str) {
        match self.validate_completeness(required_roles, expected_case_digest) {
            Ok(()) => self.status = ProofAttestationStatus::Attested,
            Err(ProofValidationError::InvalidSigningAct { .. })
            | Err(ProofValidationError::CaseDigestMismatch { .. }) => {
                self.status = ProofAttestationStatus::Defective;
            }
            Err(ProofValidationError::MissingRequiredRole { .. })
            | Err(ProofValidationError::EmptyLedgerStream)
            | Err(ProofValidationError::MissingReceiptCommitment { .. }) => {
                self.status = ProofAttestationStatus::Incomplete;
            }
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_attestation_completeness() {
        let attestation = CrossStackProofAttestation {
            case_id: "case_abc".into(),
            case_file_digest: "sha256:1111".into(),
            signing_acts: vec![
                SigningActAttestation {
                    act_id: "act_1".into(),
                    actor_id: "usr_alice".into(),
                    role: "claimant".into(),
                    payload_digest: "sha256:2222".into(),
                    signed_at: Utc::now(),
                    intent: "submit_claim".into(),
                    signature_valid: true,
                },
                SigningActAttestation {
                    act_id: "act_2".into(),
                    actor_id: "usr_bob".into(),
                    role: "approver".into(),
                    payload_digest: "sha256:3333".into(),
                    signed_at: Utc::now(),
                    intent: "approve_claim".into(),
                    signature_valid: true,
                },
            ],
            ledger_checkpoint: LedgerCheckpointAttestation {
                stream_id: "case_abc".into(),
                merkle_root: "sha256:root99".into(),
                sequence_head: 12,
            },
            receipt_commitments: vec![
                ReceiptCommitmentAttestation {
                    receipt_id: "rec_1".into(),
                    act_id: "act_1".into(),
                    issuer_key_class: "deployment_receipt_key".into(),
                    receipt_digest: "sha256:rec_dig_1".into(),
                },
                ReceiptCommitmentAttestation {
                    receipt_id: "rec_2".into(),
                    act_id: "act_2".into(),
                    issuer_key_class: "deployment_receipt_key".into(),
                    receipt_digest: "sha256:rec_dig_2".into(),
                },
            ],
            verified_at: Utc::now(),
            status: ProofAttestationStatus::Attested,
        };

        let result = attestation.validate_completeness(&["claimant", "approver"], "sha256:1111");
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_role_detected() {
        let attestation = CrossStackProofAttestation {
            case_id: "case_abc".into(),
            case_file_digest: "sha256:1111".into(),
            signing_acts: vec![SigningActAttestation {
                act_id: "act_1".into(),
                actor_id: "usr_alice".into(),
                role: "claimant".into(),
                payload_digest: "sha256:2222".into(),
                signed_at: Utc::now(),
                intent: "submit_claim".into(),
                signature_valid: true,
            }],
            ledger_checkpoint: LedgerCheckpointAttestation {
                stream_id: "case_abc".into(),
                merkle_root: "sha256:root99".into(),
                sequence_head: 5,
            },
            receipt_commitments: vec![ReceiptCommitmentAttestation {
                receipt_id: "rec_1".into(),
                act_id: "act_1".into(),
                issuer_key_class: "deployment_receipt_key".into(),
                receipt_digest: "sha256:rec_dig_1".into(),
            }],
            verified_at: Utc::now(),
            status: ProofAttestationStatus::Attested,
        };

        let result = attestation.validate_completeness(&["claimant", "supervisor"], "sha256:1111");
        assert_eq!(
            result,
            Err(ProofValidationError::MissingRequiredRole {
                role: "supervisor".into()
            })
        );
    }

    #[test]
    fn test_case_digest_mismatch_detected() {
        let attestation = CrossStackProofAttestation {
            case_id: "case_abc".into(),
            case_file_digest: "sha256:wrong".into(),
            signing_acts: vec![],
            ledger_checkpoint: LedgerCheckpointAttestation {
                stream_id: "case_abc".into(),
                merkle_root: "sha256:root99".into(),
                sequence_head: 5,
            },
            receipt_commitments: vec![],
            verified_at: Utc::now(),
            status: ProofAttestationStatus::Attested,
        };

        let result = attestation.validate_completeness(&[], "sha256:expected");
        assert_eq!(
            result,
            Err(ProofValidationError::CaseDigestMismatch {
                expected: "sha256:expected".into(),
                actual: "sha256:wrong".into(),
            })
        );
    }
}
