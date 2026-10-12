//! Validates complete native records without changing their original facts.

use std::sync::OnceLock;

use integrity_cbor::{
    Value as CborValue, canonicalize_cbor_value, cbor_value_to_json, decode_cbor_value,
};
use serde_json::Value;

use crate::{ProvenanceKind, ProvenanceRecord};

/// Outer custody event for known kinds without a specialized canonical mapping.
pub const WOS_NATIVE_PROVENANCE_EVENT_LITERAL: &str = "wos.provenance.recorded";

const SCHEMA: &str = include_str!("../../../schemas/wos-native-provenance-event.schema.json");
const MAX_DEPTH: usize = 128;
static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();

/// Refusal to admit an original native provenance record.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeProvenanceError {
    /// The payload violates the closed native record schema or JSON subset.
    #[error("invalid native provenance payload: {0}")]
    InvalidPayload(String),
    /// An explicit, recognized original record kind is required.
    #[error("native provenance requires an explicit recognized recordKind")]
    InvalidRecordKind,
    /// Specialized kinds must retain their existing canonical route.
    #[error("canonical provenance kind cannot use the generic route")]
    CanonicalKind,
    /// The original record identifier must be a native record TypeID.
    #[error("native provenance record identifier is invalid")]
    InvalidRecordId,
    /// The bytes are not an unambiguous value in the owning CBOR subset.
    #[error("invalid native provenance CBOR: {0}")]
    InvalidCbor(String),
    /// The embedded owning schema could not be initialized.
    #[error("native provenance schema initialization failed: {0}")]
    InvalidSchema(String),
}

/// Validated custody payload retaining the complete original JSON record.
///
/// Construction validates raw values before typed parsing can ignore fields.
/// This type preserves JSON values; it does not assert raw HTTP byte identity.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeProvenanceEvent {
    payload: Value,
    kind: ProvenanceKind,
    record_id: String,
}

impl NativeProvenanceEvent {
    /// Validates an original wrapped payload without rewriting its fields.
    ///
    /// # Errors
    /// Refuses malformed fields, unknown kinds, canonical-route bypass and
    /// values outside the owning JSON/CBOR subset.
    pub fn from_json(value: &Value) -> Result<Self, NativeProvenanceError> {
        validate_json_subset(value, 0)?;
        let record = value
            .get("record")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                NativeProvenanceError::InvalidPayload("record must be an object".into())
            })?;
        let kind: ProvenanceKind = serde_json::from_value(
            record
                .get("recordKind")
                .cloned()
                .ok_or(NativeProvenanceError::InvalidRecordKind)?,
        )
        .map_err(|_| NativeProvenanceError::InvalidRecordKind)?;
        if kind.canonical_event_literal().is_some() {
            return Err(NativeProvenanceError::CanonicalKind);
        }
        let record_id = record
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| stack_common_typeid::is_valid_record_type_id(id))
            .ok_or(NativeProvenanceError::InvalidRecordId)?;
        let validator = VALIDATOR
            .get_or_init(|| {
                let schema: Value = serde_json::from_str(SCHEMA)
                    .map_err(|_| "invalid embedded JSON".to_string())?;
                jsonschema::options()
                    .should_validate_formats(true)
                    .build(&schema)
                    .map_err(|_| "invalid embedded schema".to_string())
            })
            .as_ref()
            .map_err(|error| NativeProvenanceError::InvalidSchema(error.clone()))?;
        validator.validate(value).map_err(|error| {
            // Report the location, not an original answer or diagnostic value.
            NativeProvenanceError::InvalidPayload(format!(
                "schema violation at {}",
                error.instance_path()
            ))
        })?;
        Ok(Self {
            payload: value.clone(),
            kind,
            record_id: record_id.to_string(),
        })
    }

    /// Validates a typed source record serialized once into the native payload.
    ///
    /// # Errors
    /// Refuses records that are unstamped, malformed or already canonical.
    pub fn from_record(record: &ProvenanceRecord) -> Result<Self, NativeProvenanceError> {
        let record = serde_json::to_value(record).map_err(|_| {
            NativeProvenanceError::InvalidPayload("record serialization failed".into())
        })?;
        Self::from_json(&serde_json::json!({ "record": record }))
    }

    /// Validates original wrapped archive CBOR before converting an inspection view.
    ///
    /// # Errors
    /// Refuses ambiguous maps, tags, byte strings, trailing bytes and invalid records.
    pub fn from_dcbor_slice(bytes: &[u8]) -> Result<Self, NativeProvenanceError> {
        Self::from_json(&decode_original(bytes, false)?)
    }

    /// Validates bare authored custody bytes before any lossy inspection conversion.
    ///
    /// The owning custody encoder tags only the root timestamp with tag 0.
    /// Original bytes remain the caller's evidence; the timestamp text is retained.
    ///
    /// # Errors
    /// Refuses ambiguous maps, misplaced tags, byte strings and invalid records.
    pub fn from_record_dcbor_slice(bytes: &[u8]) -> Result<Self, NativeProvenanceError> {
        let record = decode_original(bytes, true)?;
        Self::from_json(&serde_json::json!({ "record": record }))
    }

    /// Returns the validated complete append payload.
    #[must_use]
    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// Returns the unchanged original record JSON value.
    #[must_use]
    pub fn record(&self) -> &Value {
        &self.payload["record"]
    }

    /// Returns the recognized original record kind.
    #[must_use]
    pub fn record_kind(&self) -> ProvenanceKind {
        self.kind
    }

    /// Returns the original native record identifier.
    #[must_use]
    pub fn record_id(&self) -> &str {
        &self.record_id
    }
}

fn validate_json_subset(value: &Value, depth: usize) -> Result<(), NativeProvenanceError> {
    if depth > MAX_DEPTH {
        return Err(NativeProvenanceError::InvalidPayload(
            "maximum depth exceeded".into(),
        ));
    }
    match value {
        Value::Number(number) if number.is_u64() && number.as_i64().is_none() => Err(
            NativeProvenanceError::InvalidPayload("integer outside signed 64-bit range".into()),
        ),
        Value::Number(number) if number.is_f64() => {
            let valid = number
                .as_f64()
                .is_some_and(|n| n.is_finite() && !(n == 0.0 && n.is_sign_negative()));
            if valid {
                Ok(())
            } else {
                Err(NativeProvenanceError::InvalidPayload(
                    "noncanonical float".into(),
                ))
            }
        }
        Value::Array(items) => items
            .iter()
            .try_for_each(|item| validate_json_subset(item, depth + 1)),
        Value::Object(items) => items
            .values()
            .try_for_each(|item| validate_json_subset(item, depth + 1)),
        _ => Ok(()),
    }
}

fn decode_original(bytes: &[u8], bare: bool) -> Result<Value, NativeProvenanceError> {
    let decoded = decode_cbor_value(bytes).map_err(|_| {
        NativeProvenanceError::InvalidCbor("expected exactly one CBOR value".into())
    })?;
    validate_cbor_subset(&decoded, &mut Vec::new(), if bare { 1 } else { 0 }, bare)?;
    // Reuse the maintained duplicate-key/float checks, discarding its sorted clone.
    canonicalize_cbor_value(&decoded).map_err(|_| {
        NativeProvenanceError::InvalidCbor("ambiguous map or noncanonical number".into())
    })?;
    cbor_value_to_json(&decoded)
        .map_err(|_| NativeProvenanceError::InvalidCbor("value outside JSON bridge".into()))
}

fn validate_cbor_subset(
    value: &CborValue,
    path: &mut Vec<String>,
    depth: usize,
    bare: bool,
) -> Result<(), NativeProvenanceError> {
    let invalid = || NativeProvenanceError::InvalidCbor("unsupported value, tag or depth".into());
    if depth > MAX_DEPTH {
        return Err(invalid());
    }
    match value {
        CborValue::Tag(0, child) if bare && path.as_slice() == ["timestamp"] => {
            if matches!(child.as_ref(), CborValue::Text(_)) {
                Ok(())
            } else {
                Err(invalid())
            }
        }
        CborValue::Tag(_, _) | CborValue::Bytes(_) => Err(invalid()),
        CborValue::Array(items) => {
            for item in items {
                path.push(String::new());
                let result = validate_cbor_subset(item, path, depth + 1, bare);
                path.pop();
                result?;
            }
            Ok(())
        }
        CborValue::Map(items) => {
            for (key, value) in items {
                let CborValue::Text(key) = key else {
                    return Err(invalid());
                };
                path.push(key.clone());
                let result = validate_cbor_subset(value, path, depth + 1, bare);
                path.pop();
                result?;
            }
            Ok(())
        }
        CborValue::Null
        | CborValue::Bool(_)
        | CborValue::Integer(_)
        | CborValue::Float(_)
        | CborValue::Text(_) => Ok(()),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod tests;
