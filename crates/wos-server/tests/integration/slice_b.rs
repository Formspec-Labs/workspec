//! Temp fixture tree + paths for WS-014 slice B (calendar / notifications / integration).

use std::path::PathBuf;

use tempfile::TempDir;

/// Temp fixture tree: kernel `urn:wos:workflow:{slug}:1.0.0` plus business-calendar,
/// notification-template, and integration-profile sidecars keyed by [`slug`].
pub fn slice_b_tempdir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let slug = "ws014sliceb";
    let workflow_url = format!("urn:wos:workflow:{slug}:1.0.0");
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": workflow_url,
        "version": "1.0.0",
        "title": "WS-014 slice B",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{ "id": "sys", "type": "system" }],
        "lifecycle": {
            "initialState": "done",
            "states": { "done": { "type": "final" } }
        },
        "contracts": {}
    });
    std::fs::create_dir_all(root.join("kernel")).unwrap();
    std::fs::write(
        root.join("kernel").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&kernel).unwrap(),
    )
    .unwrap();

    // See `crates/wos-server/tests/common/mod.rs::slice_b_tempdir` for why the
    // legacy `$wosBusinessCalendar` marker is absent. ADR 0076 D-3 collapsed
    // calendars into `$wosDelivery.calendar`; bundle_service still reads the
    // per-sidecar subdirectory layout, but the marker key was always dead
    // decoration and is dropped here.
    let cal = serde_json::json!({
        "targetWorkflow": workflow_url,
        "timezone": "UTC",
        "workWeek": ["monday", "tuesday", "wednesday", "thursday", "friday"],
        "holidays": []
    });
    std::fs::create_dir_all(root.join("business-calendar")).unwrap();
    std::fs::write(
        root.join("business-calendar").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&cal).unwrap(),
    )
    .unwrap();

    let tmpl = serde_json::json!({
        "templates": [
            {
                "id": "notice",
                "subject": "Hello",
                "body": "Hello ${user}",
                "channels": ["email"]
            }
        ]
    });
    std::fs::create_dir_all(root.join("notification-template")).unwrap();
    std::fs::write(
        root.join("notification-template").join(format!("{slug}.json")),
        serde_json::to_vec_pretty(&tmpl).unwrap(),
    )
    .unwrap();

    std::fs::create_dir_all(root.join("integration-profile")).unwrap();
    std::fs::write(
        root.join("integration-profile").join(format!("{slug}.json")),
        br#"{"bindings":[]}"#,
    )
    .unwrap();

    dir
}

pub const SLICE_B_WORKFLOW: &str = "urn:wos:workflow:ws014sliceb:1.0.0";

pub fn slice_b_workflow_path_encoded() -> String {
    SLICE_B_WORKFLOW.replace(':', "%3A")
}

pub fn int_consume_001_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wos-conformance/tests/fixtures/INT-CONSUME-001-happy.json")
}
