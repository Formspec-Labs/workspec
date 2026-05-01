// Rust guideline compliant 2026-05-01

//! Migration conformance fixtures that execute `WosRuntime::migrate` directly.

use std::path::Path;

use wos_conformance::run_fixture;

fn fixture_path(name: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/tests/fixtures/{name}")
}

fn fixture_base_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .to_str()
        .expect("fixture directory is valid UTF-8")
        .to_string()
}

fn assert_fixture_passes(name: &str) {
    let path = fixture_path(name);
    let fixture_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read fixture '{path}': {error}"));
    let result = run_fixture(&fixture_json, &fixture_base_dir())
        .unwrap_or_else(|error| panic!("fixture '{name}' engine error: {error}"));

    assert!(
        result.passed,
        "fixture '{name}' failed:\n{}",
        result.failures.join("\n")
    );
}

#[test]
fn mig001_migrate_version_bump() {
    assert_fixture_passes("MIG-001-migrate-version-bump.json");
}

#[test]
fn mig002_migrate_state_not_found_rejects() {
    let path = fixture_path("MIG-002-migrate-state-not-found.json");
    let fixture_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read fixture '{path}': {error}"));
    let error = run_fixture(&fixture_json, &fixture_base_dir())
        .expect_err("state-not-found migration must reject");
    assert!(
        error.to_string().contains("state not found"),
        "unexpected migration rejection error: {error}"
    );
}
