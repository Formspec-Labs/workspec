// Rust guideline compliant 2026-02-21
//! Asserts a historical rights-impacting route refuses its declared due process.

use serde_json::Value;
use std::path::Path;
use wos_conformance::ConformanceError;

/// Generic-route refusal of rights-impacting adverse capability (Governance §3).
const REFUSAL: &str = "/governance/dueProcess: unsupported reference operation: \
    Governance §3 notice, explanation, appeal and independent human review; \
    no operation executed";

/// Asserts `run` refuses the canonical due process the fixture's workflow declares.
///
/// The selected workflow must embed a schema-admitted Governance §3 policy with
/// required before-effect notice and an appeal reviewed independently of the
/// original decider, so the refusal is of a declared policy this route cannot
/// execute rather than of a missing or retired one.
pub fn assert_refused<T>(
    name: &str,
    source: &str,
    base: &Path,
    run: impl FnOnce(&str, &str) -> Result<T, ConformanceError>,
) {
    let fixture: Value = serde_json::from_str(source).unwrap();
    let kernel_path = base.join(fixture["documents"]["kernel"].as_str().unwrap());
    let kernel: Value = serde_json::from_str(&std::fs::read_to_string(kernel_path).unwrap())
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let governance = &kernel["governance"];
    wos_core::model::governance::validate_governance(governance)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let policy = &governance["dueProcess"]["adverseDecisionPolicy"];
    assert_eq!(policy["noticeRequired"], true, "{name}: notice is required");
    assert_eq!(policy["noticeTiming"], "beforeEffective", "{name}");
    assert_eq!(
        policy["appealMechanism"]["reviewerConstraint"], "independentFromOriginal",
        "{name}: appeal review is independent"
    );
    match run(source, base.to_str().unwrap()) {
        Err(ConformanceError::UnsupportedOperation(reason)) => {
            assert_eq!(reason, REFUSAL, "{name}");
        }
        Err(error) => panic!("{name}: {error}"),
        Ok(_) => panic!("{name}: the generic route must refuse required due process"),
    }
}
