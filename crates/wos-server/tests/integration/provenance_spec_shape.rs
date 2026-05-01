//! Verify the server's provenance surface exposes `wos_core::ProvenanceRecord`
//! unchanged (spec-first posture). Stored payloads must deserialise back to
//! the same record, and the response envelope must flatten the record's
//! fields at the top level alongside the server's integrity metadata.

use wos_core::provenance::{ProvenanceKind, ProvenanceRecord};
use wos_server::domain::provenance::ProvenanceResponse;
use wos_server::services::provenance_service::row_to_response;
use wos_server::storage::ProvenanceRow;

fn row_with_record(record: &ProvenanceRecord) -> ProvenanceRow {
    let ts = chrono::DateTime::parse_from_rfc3339("2026-04-17T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    ProvenanceRow {
        id: "rec-1".into(),
        instance_id: "inst-1".into(),
        seq: 1,
        timestamp: ts,
        tier: "facts".into(),
        payload: serde_json::to_value(record).unwrap(),
        hash: "sha256:abc".into(),
        previous_hash: "sha256:0000".into(),
    }
}

#[test]
fn stored_payload_roundtrips_to_wos_core_record() {
    let mut record = ProvenanceRecord::state_transition(
        "intake",
        "eligibilityReview",
        "submit",
        Some("jane.doe"),
    );
    record.actor_type = Some("human".into());
    record.audit_layer = Some("facts".into());
    record.data = Some(serde_json::json!({ "applicationId": "app-1" }));

    let row = row_with_record(&record);
    let response = row_to_response(&row).unwrap();

    // The server's envelope preserves the record's fields verbatim.
    assert!(matches!(
        response.record.record_kind,
        ProvenanceKind::StateTransition
    ));
    assert_eq!(response.record.from_state.as_deref(), Some("intake"));
    assert_eq!(
        response.record.to_state.as_deref(),
        Some("eligibilityReview")
    );
    assert_eq!(response.record.event.as_deref(), Some("submit"));
    assert_eq!(response.record.actor_id.as_deref(), Some("jane.doe"));
    assert_eq!(response.record.actor_type.as_deref(), Some("human"));
    assert_eq!(response.record.audit_layer.as_deref(), Some("facts"));

    // Integrity metadata is the server's addition.
    assert_eq!(response.hash, "sha256:abc");
    assert_eq!(response.previous_hash, "sha256:0000");
    assert_eq!(response.seq, 1);
}

#[test]
fn response_serialises_as_flattened_record_plus_integrity() {
    let record = ProvenanceRecord::state_transition("a", "b", "ev", Some("actor"));
    let response = ProvenanceResponse {
        record,
        id: "rec-1".into(),
        instance_id: "inst-1".into(),
        seq: 7,
        hash: "sha256:cafe".into(),
        previous_hash: "sha256:babe".into(),
    };
    let serialised = serde_json::to_value(&response).unwrap();
    // Flattened wos-core fields at the top level.
    assert_eq!(
        serialised.get("recordKind").and_then(|v| v.as_str()),
        Some("stateTransition")
    );
    assert_eq!(serialised.get("event").and_then(|v| v.as_str()), Some("ev"));
    assert_eq!(
        serialised.get("fromState").and_then(|v| v.as_str()),
        Some("a")
    );
    // Server integrity metadata at the same level.
    assert_eq!(
        serialised.get("hash").and_then(|v| v.as_str()),
        Some("sha256:cafe")
    );
    assert_eq!(serialised.get("seq").and_then(|v| v.as_i64()), Some(7));
}
