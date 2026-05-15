// Rust guideline compliant 2026-02-21

//! Authored custody append inputs for downstream bindings.
//!
//! This module publishes the WOS-owned append surface from ADR-0061 without
//! embedding any Trellis-, Temporal-, or Restate-specific adapter logic.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use integrity_cbor::{JsonCborError, dcbor_bytes_to_json, json_to_dcbor_bytes_with_limit};
use serde::{Deserialize, Serialize};
use stack_common_typeid as typeid;

use crate::provenance::ProvenanceRecord;

/// WOS authored records are small governance facts and must stay inside the
/// current inline-payload posture. If this bound changes, Trellis and WOS must
/// ratify the new seam contract together.
const DEFAULT_MAX_INLINE_RECORD_BYTES: usize = 64 * 1024;

/// Runtime context for building custody append inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyAppendContext {
    /// Registered `wos.*` prefix used for provenance event types.
    pub event_type_prefix: String,
    /// Optional explicit case identifier override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_id: Option<String>,
    /// Optional cap for authored dCBOR bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_inline_record_bytes: Option<usize>,
    /// Off-wire workflow reference retained for runtime correlation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_ref: Option<String>,
}

impl CustodyAppendContext {
    /// Build metadata for a persisted provenance record.
    ///
    /// # Errors
    /// Returns an error when the case identifier, record identifier, or event
    /// type violate the ADR-0061 authored-wire rules.
    pub fn metadata_for_provenance_record(
        &self,
        process_id: &str,
        _log_position: usize,
        record: &ProvenanceRecord,
    ) -> Result<CustodyAppendMetadata, CustodyAppendError> {
        let case_id = self
            .case_id
            .clone()
            .unwrap_or_else(|| process_id.to_string());
        let metadata = CustodyAppendMetadata {
            case_id,
            record_id: record.id.clone(),
            event_type: provenance_event_type(&self.event_type_prefix, record)?,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    fn max_inline_record_bytes(&self) -> usize {
        self.max_inline_record_bytes
            .unwrap_or(DEFAULT_MAX_INLINE_RECORD_BYTES)
    }
}

/// Narrow authored-wire metadata supplied by the WOS runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyAppendMetadata {
    /// Stable TypeID-structured case identifier.
    pub case_id: String,
    /// Stable TypeID-structured authored-record identifier.
    pub record_id: String,
    /// Outcome-neutral `wos.*` event type admitted into the custody layer.
    pub event_type: String,
}

impl CustodyAppendMetadata {
    /// Validates the ADR-0061 authored-wire metadata.
    ///
    /// # Errors
    /// Returns an error when a required field is empty, malformed, or outside
    /// the reserved WOS identifier namespaces.
    pub fn validate(&self) -> Result<(), CustodyAppendError> {
        validate_required_field("caseId", &self.case_id)?;
        validate_required_field("recordId", &self.record_id)?;
        validate_required_field("eventType", &self.event_type)?;
        if !typeid::is_valid_type_id(&self.case_id, Some(typeid::CASE_PREFIX)) {
            return Err(CustodyAppendError::InvalidTypeId("caseId"));
        }
        if !typeid::is_valid_record_type_id(&self.record_id) {
            return Err(CustodyAppendError::InvalidTypeId("recordId"));
        }
        if !self.event_type.starts_with("wos.") {
            return Err(CustodyAppendError::InvalidEventType(
                self.event_type.clone(),
            ));
        }
        Ok(())
    }
}

/// WOS-authored append input for a custody binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyAppendInput {
    /// Stable TypeID-structured case identifier.
    pub case_id: String,
    /// Stable TypeID-structured authored-record identifier.
    pub record_id: String,
    /// Outcome-neutral `wos.*` event type admitted into custody.
    pub event_type: String,
    /// dCBOR bytes of the authored WOS record.
    #[serde(with = "base64_record_bytes")]
    pub record: Vec<u8>,
}

impl CustodyAppendInput {
    /// Builds a custody append input from a WOS provenance record.
    ///
    /// # Errors
    /// Returns an error when the metadata is malformed or the authored record
    /// cannot be converted to deterministic dCBOR bytes.
    pub fn from_provenance_record(
        record: &ProvenanceRecord,
        context: &CustodyAppendContext,
        metadata: CustodyAppendMetadata,
    ) -> Result<Self, CustodyAppendError> {
        metadata.validate()?;
        let authored = provenance_record_to_custody_json(record)?;
        let string_tags = provenance_string_tags();
        let encoded = json_to_dcbor_bytes_with_limit(
            &authored,
            context.max_inline_record_bytes(),
            &string_tags,
        )
        .map_err(CustodyAppendError::from)?;
        Ok(Self {
            case_id: metadata.case_id,
            record_id: metadata.record_id,
            event_type: metadata.event_type,
            record: encoded,
        })
    }

    /// Returns the WOS-owned semantic idempotency input.
    #[must_use]
    pub fn idempotency_tuple(&self) -> (&str, &str) {
        (&self.case_id, &self.record_id)
    }

    /// Returns the authored dCBOR bytes.
    #[must_use]
    pub fn record_bytes(&self) -> &[u8] {
        &self.record
    }

    /// Decodes the authored record into a JSON inspection view.
    ///
    /// Byte strings are rendered as base64 strings; CBOR tag 0 / 32 values are
    /// rendered as their underlying strings.
    ///
    /// # Errors
    /// Returns an error when the authored bytes are not valid CBOR.
    pub fn record_json_view(&self) -> Result<serde_json::Value, CustodyAppendError> {
        dcbor_bytes_to_json(&self.record).map_err(CustodyAppendError::from)
    }
}

/// Minimum WOS-facing receipt for a successful custody append.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyAppendReceipt {
    /// Trellis `canonical_event_hash` text.
    ///
    /// The service contract currently returns `sha256:<lowercase hex>`, while
    /// older in-process fixtures may still use a bare lowercase hex value.
    pub canonical_event_hash: String,
}

impl CustodyAppendReceipt {
    /// Build a receipt from the canonical event hash returned by the custody
    /// substrate.
    #[must_use]
    pub fn new(canonical_event_hash: impl Into<String>) -> Self {
        Self {
            canonical_event_hash: canonical_event_hash.into(),
        }
    }
}

/// Errors building authored custody append inputs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustodyAppendError {
    /// A required ADR-0061 field was empty.
    #[error("custody append field must not be empty: {0}")]
    EmptyField(&'static str),

    /// A TypeID field was malformed.
    #[error("custody append TypeID field is invalid: {0}")]
    InvalidTypeId(&'static str),

    /// The binding event type was outside the `wos.*` namespace.
    #[error("custody event type must start with 'wos.': {0}")]
    InvalidEventType(String),

    /// A registry-seeded D26 record omitted or contradicted its event literal.
    #[error("custody record event must match registry literal {expected}: {actual:?}")]
    EventLiteralMismatch {
        /// The required registry event literal.
        expected: &'static str,
        /// The record's supplied `event` value.
        actual: Option<String>,
    },

    /// The authored record could not be serialized to JSON first.
    #[error("failed to serialize authored custody record to JSON: {0}")]
    JsonSerialization(String),

    /// The authored record could not be rendered as deterministic dCBOR.
    #[error("failed to encode authored custody record as dCBOR: {0}")]
    Dcbor(String),

    /// The authored record exceeded the inline payload posture.
    #[error("authored custody record exceeds inline payload posture: {actual} > {max} bytes")]
    OversizedRecord { actual: usize, max: usize },

    /// The record contained a JSON number outside the permitted range.
    #[error("custody record integer is outside the supported signed 64-bit range")]
    IntegerOutOfRange,

    /// The record contained a float outside deterministic dCBOR JSON rules (non-finite or `-0.0`).
    #[error("custody record floating-point numbers must be finite and use canonical +0")]
    FloatNotDcborCanonical,

    /// The record contained an unsupported tagged value on decode.
    #[error("custody record contains unsupported CBOR content: {0}")]
    UnsupportedCbor(String),
}

impl From<JsonCborError> for CustodyAppendError {
    fn from(error: JsonCborError) -> Self {
        match error {
            JsonCborError::Cbor(message) => Self::Dcbor(message),
            JsonCborError::Oversized { actual, max } => Self::OversizedRecord { actual, max },
            JsonCborError::IntegerOutOfRange => Self::IntegerOutOfRange,
            JsonCborError::FloatNotDcborCanonical => Self::FloatNotDcborCanonical,
            JsonCborError::UnsupportedCbor(message) => Self::UnsupportedCbor(message),
        }
    }
}

fn validate_required_field(name: &'static str, value: &str) -> Result<(), CustodyAppendError> {
    if value.trim().is_empty() {
        return Err(CustodyAppendError::EmptyField(name));
    }
    Ok(())
}

fn provenance_event_type(
    event_type_prefix: &str,
    record: &ProvenanceRecord,
) -> Result<String, CustodyAppendError> {
    let event_type_prefix = event_type_prefix.trim_end_matches('.');
    validate_required_field("eventTypePrefix", event_type_prefix)?;

    if let Some(expected) = record.record_kind.canonical_event_literal() {
        return match record.event.as_deref() {
            Some(actual) if actual == expected => Ok(expected.to_string()),
            _ => Err(CustodyAppendError::EventLiteralMismatch {
                expected,
                actual: record.event.clone(),
            }),
        };
    }

    let kind = serde_json::to_value(record.record_kind)
        .map_err(|error| CustodyAppendError::JsonSerialization(error.to_string()))?;
    let Some(kind) = kind.as_str() else {
        return Err(CustodyAppendError::JsonSerialization(
            "provenance kind did not serialize to a string".to_string(),
        ));
    };
    Ok(format!(
        "{event_type_prefix}.{}",
        camel_case_record_kind_to_event_tail(kind)
    ))
}

fn provenance_record_to_custody_json(
    record: &ProvenanceRecord,
) -> Result<serde_json::Value, CustodyAppendError> {
    let mut authored = serde_json::to_value(record)
        .map_err(|error| CustodyAppendError::JsonSerialization(error.to_string()))?;

    if let Some(expected) = record.record_kind.canonical_event_literal() {
        match record.event.as_deref() {
            Some(actual) if actual == expected => {
                let Some(authored) = authored.as_object_mut() else {
                    return Err(CustodyAppendError::JsonSerialization(
                        "provenance record did not serialize to an object".to_string(),
                    ));
                };
                authored.remove("recordKind");
            }
            _ => {
                return Err(CustodyAppendError::EventLiteralMismatch {
                    expected,
                    actual: record.event.clone(),
                });
            }
        }
    }

    Ok(authored)
}

fn camel_case_record_kind_to_event_tail(kind: &str) -> String {
    let mut event_tail = String::with_capacity(kind.len());
    for character in kind.chars() {
        if character.is_ascii_uppercase() {
            if !event_tail.is_empty() {
                event_tail.push('_');
            }
            event_tail.push(character.to_ascii_lowercase());
        } else {
            event_tail.push(character);
        }
    }
    event_tail
}

fn provenance_string_tags() -> Vec<(Vec<String>, u64)> {
    vec![(vec!["timestamp".to_string()], 0u64)]
}

mod base64_record_bytes {
    use super::*;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::provenance::{ProvenanceKind, SignatureAffirmationInput};
    use sha2::{Digest, Sha256};

    fn metadata() -> CustodyAppendMetadata {
        CustodyAppendMetadata {
            case_id: typeid::mint_case_ledger_id(),
            record_id: typeid::mint_provenance_id(),
            event_type: "wos.kernel.state_transition".to_string(),
        }
    }

    fn context() -> CustodyAppendContext {
        CustodyAppendContext {
            event_type_prefix: "wos.kernel".to_string(),
            case_id: None,
            max_inline_record_bytes: None,
            workflow_ref: Some("https://example.com/workflows/intake-review.json".to_string()),
        }
    }

    #[test]
    fn provenance_record_becomes_four_field_append_input() {
        let mut record =
            ProvenanceRecord::state_transition("intake", "review", "submitted", Some("worker"));
        record.timestamp = "2026-04-21T14:30:00Z".to_string();
        let input = CustodyAppendInput::from_provenance_record(&record, &context(), metadata())
            .expect("build input");

        assert_eq!(input.event_type, "wos.kernel.state_transition");
        assert_eq!(input.idempotency_tuple().0, input.case_id);
        assert_eq!(input.idempotency_tuple().1, input.record_id);
        let view = input.record_json_view().expect("decode json view");
        assert_eq!(view["id"], record.id);
        assert_eq!(view.get("recordKind"), None);
        assert_eq!(view["event"], "wos.kernel.state_transition");
        assert_eq!(view["timestamp"], "2026-04-21T14:30:00Z");
    }

    #[test]
    fn provenance_event_type_uses_snake_case_record_tail() {
        assert_eq!(
            camel_case_record_kind_to_event_tail("signatureAffirmation"),
            "signature_affirmation"
        );
        assert_eq!(
            camel_case_record_kind_to_event_tail("stateTransition"),
            "state_transition"
        );
        assert_eq!(
            camel_case_record_kind_to_event_tail("dcrActivityExecuted"),
            "dcr_activity_executed"
        );

        let record =
            ProvenanceRecord::state_transition("intake", "review", "submitted", Some("worker"));
        assert_eq!(
            provenance_event_type("wos.kernel", &record).expect("event type"),
            "wos.kernel.state_transition"
        );
    }

    fn record_with_seeded_kind(kind: ProvenanceKind, event: Option<&str>) -> ProvenanceRecord {
        let mut record =
            ProvenanceRecord::state_transition("intake", "review", "submitted", Some("worker"));
        record.record_kind = kind;
        record.event = event.map(str::to_string);
        record
    }

    #[test]
    fn provenance_event_type_uses_seeded_event_literal() {
        let seeded = [
            (
                ProvenanceKind::StateTransition,
                "wos.kernel.state_transition",
            ),
            (ProvenanceKind::CaseCreated, "wos.kernel.case_created"),
            (ProvenanceKind::IntakeAccepted, "wos.kernel.intake_accepted"),
            (ProvenanceKind::IntakeRejected, "wos.kernel.intake_rejected"),
            (ProvenanceKind::IntakeDeferred, "wos.kernel.intake_deferred"),
            (
                ProvenanceKind::CapabilityInvocation,
                "wos.ai.capability_invocation",
            ),
            (
                ProvenanceKind::ForEachIterationStarted,
                "wos.kernel.for_each_iteration_started",
            ),
            (
                ProvenanceKind::ForEachIterationCompleted,
                "wos.kernel.for_each_iteration_completed",
            ),
            (
                ProvenanceKind::ForEachCompleted,
                "wos.kernel.for_each_completed",
            ),
            (
                ProvenanceKind::SignatureAffirmation,
                "wos.kernel.signature_affirmation",
            ),
            (
                ProvenanceKind::SignatureAdmissionFailed,
                "wos.kernel.signature_admission_failed",
            ),
            (
                ProvenanceKind::CorrectionAuthorized,
                "wos.governance.correction_authorized",
            ),
            (
                ProvenanceKind::AmendmentAuthorized,
                "wos.governance.amendment_authorized",
            ),
            (
                ProvenanceKind::DeterminationAmended,
                "wos.governance.determination_amended",
            ),
            (
                ProvenanceKind::RescissionAuthorized,
                "wos.governance.rescission_authorized",
            ),
            (
                ProvenanceKind::DeterminationRescinded,
                "wos.governance.determination_rescinded",
            ),
            (ProvenanceKind::Reinstated, "wos.governance.reinstated"),
            (
                ProvenanceKind::AuthorizationAttestation,
                "wos.governance.authorization_attestation",
            ),
            (ProvenanceKind::ClockStarted, "wos.governance.clock_started"),
            (
                ProvenanceKind::ClockResolved,
                "wos.governance.clock_resolved",
            ),
            (
                ProvenanceKind::IdentityAttestation,
                "wos.assurance.identity_attestation",
            ),
            (ProvenanceKind::KeyRebind, "wos.assurance.key_rebind"),
            (
                ProvenanceKind::ClockSkewObserved,
                "wos.governance.clock_skew_observed",
            ),
            (
                ProvenanceKind::CommitAttemptFailure,
                "wos.kernel.commit_attempt_failure",
            ),
            (
                ProvenanceKind::AuthorizationRejected,
                "wos.governance.authorization_rejected",
            ),
            (
                ProvenanceKind::MigrationPinChanged,
                "wos.kernel.migration_pin_changed",
            ),
        ];

        for (kind, event_literal) in seeded {
            let record = record_with_seeded_kind(kind, Some(event_literal));

            assert_eq!(
                provenance_event_type("wos.kernel", &record).expect("event type"),
                event_literal
            );
        }
    }

    #[test]
    fn provenance_event_type_rejects_missing_seeded_event() {
        let record = record_with_seeded_kind(ProvenanceKind::CaseCreated, None);

        assert_eq!(
            provenance_event_type("wos.kernel", &record).expect_err("reject"),
            CustodyAppendError::EventLiteralMismatch {
                expected: "wos.kernel.case_created",
                actual: None,
            }
        );
    }

    #[test]
    fn provenance_event_type_rejects_mismatched_seeded_event() {
        let record =
            record_with_seeded_kind(ProvenanceKind::CaseCreated, Some("wos.kernel.caseCreated"));

        assert_eq!(
            provenance_event_type("wos.kernel", &record).expect_err("reject"),
            CustodyAppendError::EventLiteralMismatch {
                expected: "wos.kernel.case_created",
                actual: Some("wos.kernel.caseCreated".to_string()),
            }
        );
    }

    #[test]
    fn provenance_event_type_rejects_missing_seeded_event_for_for_each() {
        let record = record_with_seeded_kind(ProvenanceKind::ForEachIterationStarted, None);

        assert_eq!(
            provenance_event_type("wos.kernel", &record).expect_err("reject"),
            CustodyAppendError::EventLiteralMismatch {
                expected: "wos.kernel.for_each_iteration_started",
                actual: None,
            }
        );
    }

    #[test]
    fn provenance_event_type_rejects_mismatched_seeded_event_for_for_each() {
        let record = record_with_seeded_kind(
            ProvenanceKind::ForEachIterationStarted,
            Some("wos.kernel.for_each_iteration_started_WRONG"),
        );

        assert_eq!(
            provenance_event_type("wos.kernel", &record).expect_err("reject"),
            CustodyAppendError::EventLiteralMismatch {
                expected: "wos.kernel.for_each_iteration_started",
                actual: Some("wos.kernel.for_each_iteration_started_WRONG".to_string()),
            }
        );
    }

    #[test]
    fn metadata_rejects_non_wos_event_type() {
        let error = CustodyAppendMetadata {
            case_id: typeid::mint_case_ledger_id(),
            record_id: typeid::mint_provenance_id(),
            event_type: "trellis.appended".to_string(),
        }
        .validate()
        .expect_err("reject");

        assert_eq!(
            error,
            CustodyAppendError::InvalidEventType("trellis.appended".to_string())
        );
    }

    #[test]
    fn metadata_rejects_non_case_type_id() {
        let error = CustodyAppendMetadata {
            case_id: typeid::mint_provenance_id(),
            record_id: typeid::mint_provenance_id(),
            event_type: "wos.kernel.state_transition".to_string(),
        }
        .validate()
        .expect_err("reject");

        assert_eq!(error, CustodyAppendError::InvalidTypeId("caseId"));
    }

    #[test]
    fn metadata_rejects_case_family_record_id() {
        let error = CustodyAppendMetadata {
            case_id: typeid::mint_case_ledger_id(),
            record_id: typeid::mint_case_ledger_id(),
            event_type: "wos.kernel.state_transition".to_string(),
        }
        .validate()
        .expect_err("reject");

        assert_eq!(error, CustodyAppendError::InvalidTypeId("recordId"));
    }

    #[test]
    fn metadata_rejects_unknown_record_family() {
        let prov = typeid::mint_provenance_id();
        let tail = prov.rsplit_once('_').expect("typeid").1;
        let error = CustodyAppendMetadata {
            case_id: typeid::mint_case_ledger_id(),
            record_id: format!("default_custom_{tail}"),
            event_type: "wos.kernel.state_transition".to_string(),
        }
        .validate()
        .expect_err("reject");

        assert_eq!(error, CustodyAppendError::InvalidTypeId("recordId"));
    }

    #[test]
    fn oversized_records_fail_loudly() {
        let mut record =
            ProvenanceRecord::state_transition("intake", "review", "submitted", Some("worker"));
        record.data = Some(serde_json::json!({ "blob": "x".repeat(8_192) }));
        let mut context = context();
        context.max_inline_record_bytes = Some(128);

        let error = CustodyAppendInput::from_provenance_record(&record, &context, metadata())
            .expect_err("oversize rejection");

        assert!(matches!(error, CustodyAppendError::OversizedRecord { .. }));
    }

    #[test]
    fn context_uses_case_ledger_id_as_default_case_id() {
        let record = ProvenanceRecord::unmatched_event("submitted", Some("worker"));
        let case_ledger_id = typeid::mint_case_ledger_id();
        let metadata = context()
            .metadata_for_provenance_record(&case_ledger_id, 0, &record)
            .expect("metadata");

        assert_eq!(metadata.case_id, case_ledger_id);
        assert_eq!(metadata.record_id, record.id);
    }

    #[test]
    fn encoded_json_representation_uses_base64() {
        let record = ProvenanceRecord::unmatched_event("submitted", Some("worker"));
        let input = CustodyAppendInput::from_provenance_record(&record, &context(), metadata())
            .expect("append input");

        let json = serde_json::to_value(&input).expect("serialize");
        let encoded = json["record"].as_str().expect("base64 record");
        assert_eq!(STANDARD.decode(encoded).expect("decode"), input.record);
    }

    #[test]
    fn provenance_fixture_corpus_matches_rust_authority() {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/kernel/custody-hook/provenance-state-transition");
        let authored: ProvenanceRecord = serde_json::from_str(
            &std::fs::read_to_string(fixture_dir.join("record.json")).expect("fixture json"),
        )
        .expect("deserialize provenance fixture");
        let metadata = CustodyAppendMetadata {
            case_id: "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd".to_string(),
            record_id: authored.id.clone(),
            event_type: "wos.kernel.state_transition".to_string(),
        };
        let expected_bytes =
            std::fs::read(fixture_dir.join("record.dcbor")).expect("fixture dcbor");
        let expected_sha256 = std::fs::read_to_string(fixture_dir.join("record.sha256"))
            .expect("fixture sha256")
            .trim()
            .to_string();

        let input = CustodyAppendInput::from_provenance_record(&authored, &context(), metadata)
            .expect("append input");
        let digest = Sha256::digest(input.record_bytes());
        let actual_sha256 = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        assert_eq!(input.record_bytes(), expected_bytes);
        assert_eq!(actual_sha256, expected_sha256);
        let expected_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture_dir.join("record.json")).expect("fixture json"),
        )
        .expect("fixture json value");
        assert_eq!(
            input.record_json_view().expect("decode json view"),
            expected_json
        );
    }

    #[test]
    fn signature_affirmation_enters_custody_append_window() {
        let record = ProvenanceRecord::signature_affirmation(SignatureAffirmationInput {
            signer_id: "applicant",
            role_id: "applicantSigner",
            role: "signer",
            document_id: "benefitsApplication",
            signing_act_id: "01JQRPD32JF8XT9QXKKV3RQSD1",
            document_ref: serde_json::json!({
                "documentId": "benefitsApplication",
                "locale": "en-US",
            }),
            document_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            presentation_hash: "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedc",
            document_hash_algorithm: "sha-256",
            source_signature_system: "formspec",
            source_signature_id: "sig-2026-0001",
            signed_payload_digest: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            signed_payload_digest_algorithm: "sha-256",
            signing_intent: "urn:wos:signing-intent:applicant-signature",
            signed_at: "2026-04-22T14:30:00Z",
            identity_binding: serde_json::json!({
                "method": "email-otp",
                "assuranceLevel": "standard",
                "providerRef": "urn:agency.gov:identity:providers:email-otp"
            }),
            consent_reference: serde_json::json!({
                "consentTextRef": "urn:agency.gov:consent:esign-benefits:v1",
                "consentVersion": "1.0.0",
                "acceptedAtPath": "response.signature.acceptedAt",
                "affirmationPath": "response.signature.affirmed"
            }),
            signature_provider: "urn:agency.gov:signature:providers:formspec",
            ceremony_id: "ceremony-2026-0001",
            profile_ref: Some("urn:agency.gov:wos:signature-profile:benefits:v1"),
            profile_key: None,
            source_response_ref: "urn:agency.gov:formspec:responses:benefits:case-2026-0001",
            signer_authority: None,
            custody_hook_eligible: true,
            primitive_verification: serde_json::json!({
                "status": "deferredPendingHelper",
                "reason": "formspec-signing-helper-pending",
            }),
            verification_receipt: None,
            witnessed_signature_ref: None,
        });
        let metadata = context()
            .metadata_for_provenance_record(&typeid::mint_case_ledger_id(), 0, &record)
            .expect("metadata");

        let input = CustodyAppendInput::from_provenance_record(&record, &context(), metadata)
            .expect("append input");
        let view = input.record_json_view().expect("decode json view");

        assert_eq!(input.event_type, "wos.kernel.signature_affirmation");
        assert_eq!(input.record_id, record.id);
        assert_eq!(view.get("recordKind"), None);
        assert_eq!(view["event"], "wos.kernel.signature_affirmation");
        assert_eq!(view["data"]["signerId"], "applicant");
        assert_eq!(view["data"]["signingActId"], "01JQRPD32JF8XT9QXKKV3RQSD1");
        assert_eq!(
            view["data"]["documentRef"]["documentId"],
            "benefitsApplication"
        );
        assert_eq!(view["data"]["documentRef"]["locale"], "en-US");
        assert_eq!(
            view["data"]["presentationHash"],
            "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedc"
        );
        assert_eq!(
            view["data"]["witnessedSignatureRef"],
            serde_json::Value::Null
        );
        assert_eq!(view["data"]["custodyHookEligible"], true);
    }
}
