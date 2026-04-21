// Rust guideline compliant 2026-04-14

//! CloudEvents 1.0 envelope types and ingress validation.
//!
//! Provides the `CloudEvent` struct that all three NB.3 binding handlers
//! (event-emit, event-consume, callback) use, and an ingress validator that
//! rejects events missing or emptying required CloudEvents 1.0 attributes.

use serde::{Deserialize, Serialize};

/// A CloudEvents 1.0 envelope.
///
/// Required attributes (`id`, `source`, `specversion`, `type`) are
/// deserialized as non-optional strings so serde will return a parse
/// error if they are absent from the wire representation.
///
/// Optional attributes (`subject`, `time`, `datacontenttype`, `data`)
/// are wrapped in `Option`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEvent {
    /// Globally unique event identifier.
    pub id: String,

    /// URI identifying the context that produced the event.
    pub source: String,

    /// CloudEvents specification version. Must be `"1.0"`.
    #[serde(rename = "specversion")]
    pub spec_version: String,

    /// Event type string (reverse-DNS recommended).
    #[serde(rename = "type")]
    pub event_type: String,

    /// Identifies the subject of the event within the context of the event producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Timestamp of the occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<chrono::DateTime<chrono::Utc>>,

    /// Content type of the `data` value.
    #[serde(
        rename = "datacontenttype",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub data_content_type: Option<String>,

    /// Event payload. May be any JSON value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Errors produced by ingress validation.
#[derive(Debug, thiserror::Error)]
pub enum CloudEventError {
    /// A required attribute is present but empty.
    #[error("CloudEvent has empty required field: {0}")]
    EmptyField(&'static str),

    /// The `specversion` attribute is not `"1.0"`.
    #[error("CloudEvent spec version '{0}' not supported; expected 1.0")]
    UnsupportedSpecVersion(String),
}

impl CloudEvent {
    /// Validate the envelope for ingress acceptance.
    ///
    /// Rejects events with empty required attributes or an unsupported spec
    /// version. Serde already handles the case where required attributes are
    /// absent from the wire representation (they become deserialization errors
    /// before this method is even called).
    pub fn validate_ingress(&self) -> Result<(), CloudEventError> {
        if self.id.is_empty() {
            return Err(CloudEventError::EmptyField("id"));
        }
        if self.source.is_empty() {
            return Err(CloudEventError::EmptyField("source"));
        }
        if self.event_type.is_empty() {
            return Err(CloudEventError::EmptyField("type"));
        }
        if self.spec_version != "1.0" {
            return Err(CloudEventError::UnsupportedSpecVersion(
                self.spec_version.clone(),
            ));
        }
        Ok(())
    }

    /// Serialize the full envelope to a JSON value for provenance capture.
    ///
    /// All CE attributes are included; `None` fields are omitted (via
    /// `skip_serializing_if`).
    pub fn to_provenance_data(&self) -> serde_json::Value {
        // CloudEvent fields are all primitives or Option<primitive> — serialization
        // is infallible by construction. The expect communicates that invariant.
        serde_json::to_value(self).expect(
            "CloudEvent serialization is infallible: all fields are primitive or Option<Value>",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_event() -> CloudEvent {
        CloudEvent {
            id: "evt-001".to_string(),
            source: "https://example.com/producer".to_string(),
            spec_version: "1.0".to_string(),
            event_type: "com.example.order.placed".to_string(),
            subject: None,
            time: None,
            data_content_type: None,
            data: Some(serde_json::json!({"orderId": "42"})),
        }
    }

    #[test]
    fn valid_1_0_envelope_passes_ingress() {
        assert!(valid_event().validate_ingress().is_ok());
    }

    #[test]
    fn missing_id_causes_deser_error() {
        let json = r#"{
            "source": "https://example.com",
            "specversion": "1.0",
            "type": "com.example.test"
        }"#;
        let result = serde_json::from_str::<CloudEvent>(json);
        assert!(
            result.is_err(),
            "expected deserialization error for missing id"
        );
    }

    #[test]
    fn empty_id_causes_empty_field_error() {
        let mut event = valid_event();
        event.id = String::new();
        let err = event.validate_ingress().unwrap_err();
        assert!(
            matches!(err, CloudEventError::EmptyField("id")),
            "expected EmptyField(id), got: {err}"
        );
    }

    #[test]
    fn empty_source_causes_empty_field_error() {
        let mut event = valid_event();
        event.source = String::new();
        let err = event.validate_ingress().unwrap_err();
        assert!(
            matches!(err, CloudEventError::EmptyField("source")),
            "expected EmptyField(source), got: {err}"
        );
    }

    #[test]
    fn unsupported_spec_version_is_rejected() {
        let mut event = valid_event();
        event.spec_version = "0.3".to_string();
        let err = event.validate_ingress().unwrap_err();
        assert!(
            matches!(err, CloudEventError::UnsupportedSpecVersion(ref v) if v == "0.3"),
            "expected UnsupportedSpecVersion(0.3), got: {err}"
        );
    }
}
