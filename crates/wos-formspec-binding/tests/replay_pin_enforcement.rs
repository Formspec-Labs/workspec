// Integration tests for pin re-validation on replay / audit / review paths.
//
// Validates that `revalidate_submission` recomputes pin equality from the live
// task definition URL + version, rather than trusting a stored `pin_match: true`
// from a prior validation pass.

use wos_core::instance::ActiveTask;
use wos_formspec_binding::{FormspecBinding, FormspecProcessor};
use wos_runtime::binding::{BindingError, CaseMutationBundle, SubmissionValidation};

/// Minimal stub processor — validates envelope shape, accepts any data.
#[derive(Debug, Clone, Default)]
struct StubProcessor;

impl FormspecProcessor for StubProcessor {
    fn validate_envelope(
        &self,
        _response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError> {
        Ok(Vec::new())
    }

    fn validate_definition(
        &self,
        _definition_url: &str,
        _definition_version: &str,
        _data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
        Ok(None)
    }

    fn compute_prefill(
        &self,
        _mapping_ref: Option<&str>,
        _case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError> {
        Ok(None)
    }

    fn map_response(
        &self,
        _mapping_ref: &str,
        _response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        Ok(None)
    }
}

fn task_with_pin(definition_url: &str, definition_version: &str) -> ActiveTask {
    ActiveTask {
        task_id: "task-replay-1".to_string(),
        task_ref: "review".to_string(),
        status: wos_core::instance::ActiveTaskStatus::Assigned,
        assigned_actor: Some("auditor".to_string()),
        contract_ref: Some("reviewForm".to_string()),
        binding: Some("formspec".to_string()),
        definition_url: Some(definition_url.to_string()),
        definition_version: Some(definition_version.to_string()),
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        deadline: None,
        impact_level: None,
        context: None,
        last_validation_outcome: None,
        created_at: "2024-03-09T00:00:00Z".to_string(),
        updated_at: "2024-03-09T00:00:00Z".to_string(),
        extensions: Default::default(),
    }
}

fn envelope(definition_url: &str, definition_version: &str) -> serde_json::Value {
    serde_json::json!({
        "status": "completed",
        "definitionUrl": definition_url,
        "definitionVersion": definition_version,
        "data": { "approved": true }
    })
}

/// (a) Pin-match happy path: re-validating an envelope whose URL + version match
/// the task pin must yield `pin_match: true`.
#[test]
fn revalidate_matching_pin_reports_pin_match_true() {
    let binding = FormspecBinding::new(StubProcessor);
    let task = task_with_pin("urn:formspec:review", "1.0.0");
    let stored_response = envelope("urn:formspec:review", "1.0.0");

    let result: SubmissionValidation = binding
        .revalidate_submission(&task, &stored_response)
        .expect("revalidate_submission must not error on well-formed input");

    assert!(
        result.validation_outcome.pin_match,
        "envelope with matching URL + version should yield pin_match: true"
    );
    assert!(
        result.validation_outcome.envelope_valid,
        "envelope should be structurally valid"
    );
    assert!(
        result.validation_outcome.errors.is_empty(),
        "no errors expected on pin match: {:#?}",
        result.validation_outcome.errors
    );
}

/// (b) Pin-mismatch detection: an envelope stored with a different `definitionVersion`
/// must yield `pin_match: false` and include a pinMismatch error — even if a caller
/// had previously recorded `pin_match: true` for an older version.
#[test]
fn revalidate_version_mismatch_reports_pin_mismatch() {
    let binding = FormspecBinding::new(StubProcessor);
    let task = task_with_pin("urn:formspec:review", "2.0.0");
    // Envelope was submitted against version 1.0.0; task has since been updated to 2.0.0.
    let stored_response = envelope("urn:formspec:review", "1.0.0");

    let result: SubmissionValidation = binding
        .revalidate_submission(&task, &stored_response)
        .expect("revalidate_submission must not error on well-formed input");

    assert!(
        !result.validation_outcome.pin_match,
        "version mismatch must yield pin_match: false"
    );
    let has_pin_mismatch_error = result
        .validation_outcome
        .errors
        .iter()
        .any(|e| e.get("code") == Some(&serde_json::json!("pinMismatch")));
    assert!(
        has_pin_mismatch_error,
        "errors must include pinMismatch code: {:#?}",
        result.validation_outcome.errors
    );
}

/// (c) URL mismatch: an envelope referencing a different definition URL must also
/// be rejected with pin_match: false, even when the version string matches.
#[test]
fn revalidate_url_mismatch_reports_pin_mismatch() {
    let binding = FormspecBinding::new(StubProcessor);
    let task = task_with_pin("urn:formspec:review", "1.0.0");
    let stored_response = envelope("urn:formspec:OTHER", "1.0.0");

    let result: SubmissionValidation = binding
        .revalidate_submission(&task, &stored_response)
        .expect("revalidate_submission must not error on well-formed input");

    assert!(
        !result.validation_outcome.pin_match,
        "URL mismatch must yield pin_match: false"
    );
    let has_pin_mismatch_error = result
        .validation_outcome
        .errors
        .iter()
        .any(|e| e.get("code") == Some(&serde_json::json!("pinMismatch")));
    assert!(
        has_pin_mismatch_error,
        "errors must include pinMismatch code: {:#?}",
        result.validation_outcome.errors
    );
}

/// (d) Stored `pin_match: true` must NOT be trusted on re-validation.
///
/// A task whose `last_validation_outcome` records `pin_match: true` against an
/// old pin must still produce `pin_match: false` when the live task definition
/// URL or version disagrees with the envelope. Pin equality is always recomputed
/// fresh; the stored outcome is completely ignored.
#[test]
fn revalidate_ignores_stored_pin_match_true() {
    use wos_core::instance::ValidationOutcome;

    // Task currently pinned to form-v2, but the stored last_validation_outcome
    // claims pin_match: true from a previous (now-stale) pass against form-v1.
    let task = ActiveTask {
        task_id: "task-replay-stale".to_string(),
        task_ref: "review".to_string(),
        status: wos_core::instance::ActiveTaskStatus::Assigned,
        assigned_actor: Some("auditor".to_string()),
        contract_ref: Some("reviewForm".to_string()),
        binding: Some("formspec".to_string()),
        definition_url: Some("urn:test:form-v2".to_string()),
        definition_version: Some("2.0.0".to_string()),
        prefill_mapping_ref: None,
        response_mapping_ref: None,
        deadline: None,
        impact_level: None,
        context: None,
        last_validation_outcome: Some(ValidationOutcome {
            pin_match: true, // STALE — must not be trusted
            envelope_valid: true,
            definition_valid: true,
            errors: Vec::new(),
            validation_results: None,
        }),
        created_at: "2024-03-09T00:00:00Z".to_string(),
        updated_at: "2024-03-09T00:00:00Z".to_string(),
        extensions: Default::default(),
    };

    // Envelope was originally submitted against form-v1 — does not match the
    // task's current pin (form-v2).
    let stale_envelope = serde_json::json!({
        "status": "complete",
        "definitionUrl": "urn:test:form-v1",
        "definitionVersion": "1.0.0",
        "data": {}
    });

    let binding = FormspecBinding::new(StubProcessor);
    let result: SubmissionValidation = binding
        .revalidate_submission(&task, &stale_envelope)
        .expect("revalidate_submission must not error on well-formed input");

    assert!(
        !result.validation_outcome.pin_match,
        "pin_match must be recomputed fresh, ignoring stored outcome"
    );
    let has_pin_mismatch_error = result
        .validation_outcome
        .errors
        .iter()
        .any(|e| e.get("code") == Some(&serde_json::json!("pinMismatch")));
    assert!(
        has_pin_mismatch_error,
        "pinMismatch error must be raised: {:#?}",
        result.validation_outcome.errors
    );
}
