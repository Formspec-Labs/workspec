// Rust guideline compliant 2026-02-21

//! Authored custody append inputs for downstream bindings.
//!
//! This module publishes the WOS-owned append surface from ADR-0061 without
//! embedding any Trellis-, Temporal-, or Restate-specific adapter logic.
//! Runtime bindings can take one authored WOS record, canonicalize it with
//! JCS, and forward the resulting append input to their own durable backend.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wos_core::provenance::ProvenanceRecord;

/// Custody append metadata supplied by the WOS runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyAppendMetadata {
    /// Stable identifier for the authored WOS record.
    pub record_id: String,
    /// Outcome-neutral `wos.*` event type admitted into the custody layer.
    pub event_type: String,
    /// WOS version governing the authored record semantics.
    pub wos_spec_version: String,
    /// URI for the normative record schema or document surface.
    pub record_schema_ref: String,
    /// URI for the governing workflow or kernel document.
    pub workflow_ref: String,
    /// Stable deployment case identifier.
    pub case_ref: String,
    /// Stable workflow instance identifier.
    pub instance_ref: String,
    /// Optional governance-envelope or sidecar document URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_envelope_ref: Option<String>,
    /// Optional structured pointer to the runtime moment that emitted the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_ref: Option<CustodyLifecycleRef>,
}

impl CustodyAppendMetadata {
    /// Validate required ADR-0061 metadata before building a custody append input.
    ///
    /// # Errors
    /// Returns [`CustodyAppendError::EmptyField`] when a required string is
    /// empty or whitespace-only, or [`CustodyAppendError::InvalidEventType`]
    /// when `event_type` is not in the `wos.*` namespace.
    pub fn validate(&self) -> Result<(), CustodyAppendError> {
        validate_required_field("recordId", &self.record_id)?;
        validate_required_field("eventType", &self.event_type)?;
        validate_required_field("wosSpecVersion", &self.wos_spec_version)?;
        validate_required_field("recordSchemaRef", &self.record_schema_ref)?;
        validate_required_field("workflowRef", &self.workflow_ref)?;
        validate_required_field("caseRef", &self.case_ref)?;
        validate_required_field("instanceRef", &self.instance_ref)?;
        if !self.event_type.starts_with("wos.") {
            return Err(CustodyAppendError::InvalidEventType(
                self.event_type.clone(),
            ));
        }
        Ok(())
    }
}

/// Structured pointer to the runtime moment that produced a custody record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustodyLifecycleRef {
    /// Kernel transition identifier, when the record came from a transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<String>,
    /// Lifecycle state active at record creation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    /// Triggering event name, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Task pattern identifier, when a task-driven runtime moment applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_pattern: Option<String>,
    /// Runtime task identifier, when the record is task-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// WOS-authored append input for a custody binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodyAppendInput {
    /// Stable identifier for the authored WOS record.
    pub record_id: String,
    /// Outcome-neutral `wos.*` event type admitted into custody.
    pub event_type: String,
    /// WOS-native record family discriminator.
    pub wos_record_kind: String,
    /// WOS version governing the authored record semantics.
    pub wos_spec_version: String,
    /// URI for the normative record schema or document surface.
    pub record_schema_ref: String,
    /// URI for the governing workflow or kernel document.
    pub workflow_ref: String,
    /// Stable deployment case identifier.
    pub case_ref: String,
    /// Stable workflow instance identifier.
    pub instance_ref: String,
    /// Optional governance-envelope or sidecar document URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_envelope_ref: Option<String>,
    /// Optional structured pointer to the runtime moment that emitted the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_ref: Option<CustodyLifecycleRef>,
    /// JCS-canonical UTF-8 JSON for the authored WOS record.
    pub record_canonical_json: String,
    /// Lowercase SHA-256 hex digest of `record_canonical_json`.
    pub record_digest_sha256: String,
}

impl CustodyAppendInput {
    /// Build a custody append input from an authored record.
    ///
    /// # Errors
    /// Returns an error when required metadata is empty, the event type does
    /// not live in the `wos.*` namespace, or canonical JSON generation fails.
    pub fn from_authored_record<T>(
        record: &T,
        wos_record_kind: impl Into<String>,
        metadata: CustodyAppendMetadata,
    ) -> Result<Self, CustodyAppendError>
    where
        T: Serialize,
    {
        metadata.validate()?;
        let wos_record_kind = wos_record_kind.into();
        validate_required_field("wosRecordKind", &wos_record_kind)?;
        let record_canonical_json = serde_json_canonicalizer::to_string(record)
            .map_err(|error| CustodyAppendError::CanonicalJson(error.to_string()))?;
        let record_digest_sha256 =
            format!("{:x}", Sha256::digest(record_canonical_json.as_bytes()));

        Ok(Self {
            record_id: metadata.record_id,
            event_type: metadata.event_type,
            wos_record_kind,
            wos_spec_version: metadata.wos_spec_version,
            record_schema_ref: metadata.record_schema_ref,
            workflow_ref: metadata.workflow_ref,
            case_ref: metadata.case_ref,
            instance_ref: metadata.instance_ref,
            governance_envelope_ref: metadata.governance_envelope_ref,
            lifecycle_ref: metadata.lifecycle_ref,
            record_canonical_json,
            record_digest_sha256,
        })
    }

    /// Build a custody append input from a WOS provenance record.
    ///
    /// # Errors
    /// Returns an error when metadata validation fails or the provenance
    /// record cannot be canonicalized.
    pub fn from_provenance_record(
        record: &ProvenanceRecord,
        metadata: CustodyAppendMetadata,
    ) -> Result<Self, CustodyAppendError> {
        Self::from_authored_record(record, provenance_kind_label(record)?, metadata)
    }

    /// Return the ADR-0061 idempotency source tuple.
    pub fn idempotency_tuple(&self) -> (&str, &str, &str) {
        (&self.case_ref, &self.event_type, &self.record_id)
    }

    /// Return the authored canonical JSON as UTF-8 bytes.
    pub fn record_canonical_json_bytes(&self) -> &[u8] {
        self.record_canonical_json.as_bytes()
    }
}

/// Errors building authored custody append inputs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustodyAppendError {
    /// A required ADR-0061 field was empty.
    #[error("custody append field must not be empty: {0}")]
    EmptyField(&'static str),

    /// The binding event type was outside the `wos.*` namespace.
    #[error("custody event type must start with 'wos.': {0}")]
    InvalidEventType(String),

    /// The authored record could not be rendered as JCS JSON.
    #[error("failed to canonicalize authored custody record: {0}")]
    CanonicalJson(String),
}

fn validate_required_field(name: &'static str, value: &str) -> Result<(), CustodyAppendError> {
    if value.trim().is_empty() {
        return Err(CustodyAppendError::EmptyField(name));
    }
    Ok(())
}

fn provenance_kind_label(record: &ProvenanceRecord) -> Result<String, CustodyAppendError> {
    let kind = serde_json::to_value(record.record_kind)
        .map_err(|error| CustodyAppendError::CanonicalJson(error.to_string()))?;
    let Some(kind) = kind.as_str() else {
        return Err(CustodyAppendError::CanonicalJson(
            "provenance kind did not serialize to a string".to_string(),
        ));
    };
    Ok(kind.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> CustodyAppendMetadata {
        CustodyAppendMetadata {
            record_id: "prov-0001".to_string(),
            event_type: "wos.kernel.stateTransition".to_string(),
            wos_spec_version: "0.1.0".to_string(),
            record_schema_ref: "https://example.com/schemas/wos-provenance-record.json".to_string(),
            workflow_ref: "https://example.com/workflows/intake-review.json".to_string(),
            case_ref: "case-123".to_string(),
            instance_ref: "instance-456".to_string(),
            governance_envelope_ref: Some("https://example.com/governance/defaults.json".into()),
            lifecycle_ref: Some(CustodyLifecycleRef {
                transition_id: Some("submit".to_string()),
                state_id: Some("intake".to_string()),
                event_name: Some("submitted".to_string()),
                task_pattern: None,
                task_id: None,
            }),
        }
    }

    #[test]
    fn provenance_record_becomes_custody_append_input() {
        let mut record =
            ProvenanceRecord::state_transition("intake", "review", "submitted", Some("worker"));
        record.timestamp = "2026-04-21T14:30:00Z".to_string();

        let input =
            CustodyAppendInput::from_provenance_record(&record, metadata()).expect("build input");

        assert_eq!(input.wos_record_kind, "stateTransition");
        assert_eq!(
            input.idempotency_tuple(),
            ("case-123", "wos.kernel.stateTransition", "prov-0001")
        );
        assert_eq!(
            input.record_digest_sha256,
            format!("{:x}", Sha256::digest(input.record_canonical_json_bytes()))
        );
        assert_eq!(
            input.record_canonical_json,
            serde_json_canonicalizer::to_string(&record).expect("canonical record"),
        );
    }

    #[test]
    fn non_wos_event_type_is_rejected() {
        let record = ProvenanceRecord::unmatched_event("submitted", Some("worker"));
        let mut metadata = metadata();
        metadata.event_type = "trellis.record.appended".to_string();

        let error =
            CustodyAppendInput::from_provenance_record(&record, metadata).expect_err("reject");

        assert_eq!(
            error,
            CustodyAppendError::InvalidEventType("trellis.record.appended".to_string())
        );
    }

    #[test]
    fn empty_record_id_in_metadata_is_rejected() {
        let record = ProvenanceRecord::unmatched_event("submitted", Some("worker"));
        let mut metadata = metadata();
        metadata.record_id = String::new();

        let error =
            CustodyAppendInput::from_provenance_record(&record, metadata).expect_err("reject");
        assert_eq!(error, CustodyAppendError::EmptyField("recordId"));
    }

    #[test]
    fn whitespace_only_case_ref_is_rejected() {
        let record = ProvenanceRecord::unmatched_event("submitted", Some("worker"));
        let mut metadata = metadata();
        metadata.case_ref = "   ".to_string();

        let error =
            CustodyAppendInput::from_provenance_record(&record, metadata).expect_err("reject");
        assert_eq!(error, CustodyAppendError::EmptyField("caseRef"));
    }

    #[test]
    fn metadata_validate_surfaces_empty_field_before_digest() {
        let mut metadata = metadata();
        metadata.wos_spec_version = " \t ".to_string();
        let err = metadata.validate().expect_err("reject");
        assert_eq!(err, CustodyAppendError::EmptyField("wosSpecVersion"));
    }

    #[test]
    fn from_authored_record_accepts_generic_serialize_value() {
        let record = serde_json::json!({"alpha": 1, "beta": "two"});
        let input = CustodyAppendInput::from_authored_record(
            &record,
            "customRecordKind",
            metadata(),
        )
        .expect("generic path");

        assert_eq!(input.wos_record_kind, "customRecordKind");
        assert_eq!(
            input.record_canonical_json,
            serde_json_canonicalizer::to_string(&record).expect("canonical"),
        );
        assert_eq!(
            input.record_digest_sha256,
            format!("{:x}", Sha256::digest(input.record_canonical_json_bytes()))
        );
    }
}
