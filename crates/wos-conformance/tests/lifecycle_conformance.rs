// Rust guideline compliant 2026-02-21

//! Lifecycle-control conformance (Kernel S11.3 / S11.5): `$suspend`, `$resume` and
//! `$terminate` drive `WosRuntime::control_lifecycle`, the implementation the durable adapter
//! prepares through as well.

use std::path::Path;

use wos_conformance::run_fixture;

fn run(
    name: &str,
) -> Result<wos_conformance::ConformanceResult, wos_conformance::ConformanceError> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixture = std::fs::read_to_string(base.join(name)).expect("lifecycle fixture");
    run_fixture(&fixture, base.to_str().expect("UTF-8 fixture directory"))
}

#[test]
fn life001_suspend_resume_terminate_record_facts() {
    let result = run("LIFE-001-suspend-resume-terminate-record-facts.json")
        .unwrap_or_else(|error| panic!("lifecycle fixture engine error: {error}"));
    assert!(
        result.passed,
        "lifecycle fixture failed:\n{}",
        result.failures.join("\n")
    );
}

#[test]
fn life002_suspended_instance_evaluates_no_event() {
    let error = run("LIFE-002-suspended-instance-evaluates-no-event.json")
        .expect_err("a suspended instance must refuse evaluation");
    assert!(
        error
            .to_string()
            .contains("a `suspended` instance refuses event processing"),
        "{error}"
    );
}

#[test]
fn life003_terminated_instance_admits_no_event() {
    let error = run("LIFE-003-terminated-instance-admits-no-event.json")
        .expect_err("a terminated instance must refuse even queueing");
    assert!(
        error
            .to_string()
            .contains("a `terminated` instance refuses queueing an event"),
        "{error}"
    );
}
