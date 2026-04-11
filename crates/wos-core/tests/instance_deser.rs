// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS CaseInstance documents.

use wos_core::instance::{ActiveTaskStatus, CaseInstance};

#[test]
fn active_formspec_task_round_trips() {
    let json = r#"{
        "instanceId": "urn:wos:instance:test:1",
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
                "instanceId": "urn:wos:instance:test:1",
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
        "urn:wos:instance:test:1"
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
        Some("urn:wos:instance:test:1")
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
        "instanceId": "urn:wos:instance:test:1",
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
                "instanceId": "urn:wos:instance:test:1",
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
