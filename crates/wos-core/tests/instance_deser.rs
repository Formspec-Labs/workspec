// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS CaseInstance documents.

use wos_core::instance::{ActiveTaskStatus, CaseInstance};

#[test]
fn active_formspec_task_round_trips() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [{
            "taskId": "task-1",
            "taskRef": "complete-intake",
            "status": "claimed",
            "assignedActor": "applicant-123",
            "contractRef": "intakeApplication",
            "binding": "formspec",
            "definitionUrl": "urn:formspec:intake",
            "definitionVersion": "2.0.0",
            "responseMappingRef": "urn:formspec:intake-response:1.0",
            "context": {
                "taskId": "task-1",
                "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
                "contractRef": "intakeApplication",
                "definitionUrl": "urn:formspec:intake",
                "definitionVersion": "2.0.0",
                "binding": "formspec",
                "assignedActor": "applicant-123",
                "responseMappingRef": "urn:formspec:intake-response:1.0"
            },
            "lastValidationOutcome": {
                "envelopeValid": true,
                "pinMatch": true,
                "definitionValid": false,
                "errors": [{ "code": "required", "path": "data.name" }],
                "validationResults": [{ "field": "name", "severity": "error" }]
            },
            "createdAt": "2026-04-11T14:00:00Z",
            "updatedAt": "2026-04-11T14:05:00Z"
        }],
        "status": "active",
        "createdAt": "2026-04-11T14:00:00Z",
        "updatedAt": "2026-04-11T14:05:00Z"
    }"#;

    let instance: CaseInstance = serde_json::from_str(json).unwrap();
    let task = &instance.active_tasks[0];

    assert_eq!(task.task_id, "task-1");
    assert_eq!(task.status, ActiveTaskStatus::Claimed);
    assert_eq!(task.binding.as_deref(), Some("formspec"));
    assert_eq!(task.definition_url.as_deref(), Some("urn:formspec:intake"));
    assert_eq!(
        task.response_mapping_ref.as_deref(),
        Some("urn:formspec:intake-response:1.0")
    );
    assert_eq!(
        task.context.as_ref().unwrap().instance_id,
        "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc"
    );
    assert!(
        !task
            .last_validation_outcome
            .as_ref()
            .unwrap()
            .definition_valid
    );

    let round_trip = serde_json::to_value(&instance).unwrap();
    assert_eq!(
        round_trip
            .pointer("/activeTasks/0/context/instanceId")
            .and_then(serde_json::Value::as_str),
        Some("urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc")
    );
    assert_eq!(
        round_trip
            .pointer("/activeTasks/0/lastValidationOutcome/errors/0/code")
            .and_then(serde_json::Value::as_str),
        Some("required")
    );
}

#[test]
fn missing_active_tasks_is_invalid() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "status": "active",
        "createdAt": "2026-04-11T14:00:00Z",
        "updatedAt": "2026-04-11T14:05:00Z"
    }"#;

    let error = serde_json::from_str::<CaseInstance>(json).unwrap_err();

    assert!(
        error.to_string().contains("activeTasks"),
        "expected missing activeTasks error, got {error}"
    );
}

#[test]
fn terminal_task_statuses_are_not_active_tasks() {
    for terminal_status in ["completed", "failed", "skipped"] {
        let json = format!(
            r#"{{
                "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
                "definitionUrl": "urn:wos:workflow:test",
                "definitionVersion": "1.0.0",
                "configuration": ["intake"],
                "caseState": {{}},
                "provenancePosition": 0,
                "timers": [],
                "activeTasks": [{{
                    "taskId": "task-1",
                    "taskRef": "complete-intake",
                    "status": "{terminal_status}",
                    "createdAt": "2026-04-11T14:00:00Z",
                    "updatedAt": "2026-04-11T14:05:00Z"
                }}],
                "status": "active",
                "createdAt": "2026-04-11T14:00:00Z",
                "updatedAt": "2026-04-11T14:05:00Z"
            }}"#
        );

        let error = serde_json::from_str::<CaseInstance>(&json).unwrap_err();

        assert!(
            error.to_string().contains("unknown variant"),
            "expected terminal status {terminal_status} to be rejected, got {error}"
        );
    }
}

#[test]
fn declined_status_carries_decline_reason() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [],
        "status": "declined",
        "declineReason": "Terms not acceptable",
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }"#;

    let instance: CaseInstance = serde_json::from_str(json).unwrap();
    assert_eq!(
        instance.status,
        wos_core::instance::InstanceStatus::Declined
    );
    assert_eq!(
        instance.decline_reason.as_deref(),
        Some("Terms not acceptable")
    );
    assert!(instance.voided_by.is_none());
    assert!(instance.voided_at.is_none());
    assert!(instance.expired_at.is_none());
}

#[test]
fn voided_status_carries_voided_by_and_voided_at() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [],
        "status": "voided",
        "voidedBy": "actor::supervisor-42",
        "voidedAt": "2026-05-07T12:00:00Z",
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }"#;

    let instance: CaseInstance = serde_json::from_str(json).unwrap();
    assert_eq!(instance.status, wos_core::instance::InstanceStatus::Voided);
    assert_eq!(instance.voided_by.as_deref(), Some("actor::supervisor-42"));
    assert_eq!(instance.voided_at.as_deref(), Some("2026-05-07T12:00:00Z"));
    assert!(instance.decline_reason.is_none());
    assert!(instance.expired_at.is_none());
}

#[test]
fn expired_status_carries_expired_at() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [],
        "status": "expired",
        "expiredAt": "2026-05-07T23:59:59Z",
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }"#;

    let instance: CaseInstance = serde_json::from_str(json).unwrap();
    assert_eq!(instance.status, wos_core::instance::InstanceStatus::Expired);
    assert_eq!(instance.expired_at.as_deref(), Some("2026-05-07T23:59:59Z"));
    assert!(instance.decline_reason.is_none());
    assert!(instance.voided_by.is_none());
    assert!(instance.voided_at.is_none());
}

#[test]
fn stalled_status_still_roundtrips() {
    let json = r#"{
        "instanceId": "urn:wos:test_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        "definitionUrl": "urn:wos:workflow:test",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [],
        "status": "stalled",
        "stalledSince": "2026-05-07T12:00:00Z",
        "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z"
    }"#;

    let instance: CaseInstance = serde_json::from_str(json).unwrap();
    assert_eq!(instance.status, wos_core::instance::InstanceStatus::Stalled);
    assert_eq!(
        instance.stalled_since.as_deref(),
        Some("2026-05-07T12:00:00Z")
    );
    // New fields should all be None for stalled status
    assert!(instance.decline_reason.is_none());
    assert!(instance.voided_by.is_none());
    assert!(instance.voided_at.is_none());
    assert!(instance.expired_at.is_none());
}
