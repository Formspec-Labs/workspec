//! Exercise the sha256 hash chain built by `ProvenanceService::prepare_next`
//! and verified by `provenance_service::verify_chain`.
//!
//! The test stays inside `wos-server`'s public surface — no network sockets
//! or external services required.

use wos_server::services::provenance_service::{chain_hash, verify_chain};
use wos_server::storage::ProvenanceRow;

fn row(seq: i64, previous: &str, tier: &str, payload: serde_json::Value) -> ProvenanceRow {
    let ts = chrono::DateTime::parse_from_rfc3339("2026-04-17T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
        + chrono::Duration::seconds(seq);
    let hash = chain_hash(previous, "inst-1", seq, &ts, tier, &payload);
    ProvenanceRow {
        id: format!("rec-{seq}"),
        instance_id: "inst-1".into(),
        seq,
        timestamp: ts,
        tier: tier.into(),
        payload,
        hash,
        previous_hash: previous.to_string(),
    }
}

#[test]
fn chain_of_three_verifies() {
    const ZERO: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let a = row(1, ZERO, "facts", serde_json::json!({ "event": "submitted" }));
    let b = row(2, &a.hash, "facts", serde_json::json!({ "event": "approved" }));
    let c = row(3, &b.hash, "facts", serde_json::json!({ "event": "issued" }));
    assert!(verify_chain(&[a, b, c]).is_ok());
}

#[test]
fn tampering_breaks_the_chain() {
    const ZERO: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let a = row(1, ZERO, "facts", serde_json::json!({ "event": "submitted" }));
    let mut b = row(2, &a.hash, "facts", serde_json::json!({ "event": "approved" }));
    // Mutate the payload without recomputing the hash — should fail.
    b.payload = serde_json::json!({ "event": "denied" });
    match verify_chain(&[a, b]) {
        Err(idx) => assert_eq!(idx, 1, "expected failure at row 1 (the tampered one)"),
        Ok(_) => panic!("tampered chain should not verify"),
    }
}

#[test]
fn broken_previous_hash_is_detected() {
    const ZERO: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let a = row(1, ZERO, "facts", serde_json::json!({ "event": "submitted" }));
    // Point `previous_hash` at an unrelated value.
    let mut b = row(2, &a.hash, "facts", serde_json::json!({ "event": "approved" }));
    b.previous_hash = "sha256:deadbeef".into();
    assert!(verify_chain(&[a, b]).is_err());
}
