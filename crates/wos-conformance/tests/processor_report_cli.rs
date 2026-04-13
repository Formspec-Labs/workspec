// Rust guideline compliant 2026-02-21

//! CLI tests for processor conformance reporting.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn json_report_succeeds_for_verified_claims() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let manifest_path = temp_dir.path().join("processor-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "processorName": "reference-processor",
            "claims": {
                "governanceBasic": true,
                "agentRegistration": true
            }
        }))
        .expect("serialize manifest"),
    )
    .expect("write manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_wos-conformance-report"))
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--fixtures-dir")
        .arg(fixtures_dir())
        .arg("--format")
        .arg("json")
        .output()
        .expect("run report CLI");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse JSON report");
    assert_eq!(report["processor_name"], "reference-processor");
    let claims = report["claims"].as_array().expect("claims array");
    assert!(
        claims
            .iter()
            .any(|claim| { claim["rule_id"] == "G-051" && claim["status"] == "Verified" })
    );
    assert!(
        claims
            .iter()
            .any(|claim| { claim["rule_id"] == "AI-001" && claim["status"] == "Verified" })
    );
}

#[test]
fn text_report_exits_nonzero_for_failed_claims() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let manifest_path = temp_dir.path().join("processor-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "processorName": "reference-processor",
            "claims": {
                "delegatesFormspecEvaluation": true
            }
        }))
        .expect("serialize manifest"),
    )
    .expect("write manifest");

    let output = Command::new(env!("CARGO_BIN_EXE_wos-conformance-report"))
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--fixtures-dir")
        .arg(fixtures_dir())
        .arg("--format")
        .arg("text")
        .output()
        .expect("run report CLI");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("utf8 text report");
    assert!(stdout.contains("processor: reference-processor"));
    assert!(stdout.contains("AI-004: failed"));
    assert!(stdout.contains("missing delegated Formspec evaluation evidence"));
}
