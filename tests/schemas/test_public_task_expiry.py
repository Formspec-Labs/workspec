"""Public task expiry shape controls; runtime remains the admission authority."""

import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR,
    api_referencing_registry,
    load_schema,
    validator_for_api_def,
)


@pytest.fixture(params=["Task", "TaskListItem"])
def task_shape(request):
    schema = load_schema(API_SCHEMA_DIR / "task.schema.json")
    validator = validator_for_api_def(
        schema, request.param, api_referencing_registry()
    )
    process_id = "urn:wos:default_process_01hw7rm71vfay8vvw14d2pf2db"
    task = {
        "id": f"{process_id}/tasks/respond",
        "processId": process_id,
        "caseLedgerId": "urn:wos:default_case_01hw7rm71vfay8vvw14d2pf2db",
        "taskRevision": "stored-revision",
        "kind": "verification",
        "status": "pending",
        "title": "Provide a response",
        "createdAt": "2026-10-10T12:00:00Z",
        "updatedAt": "2026-10-10T12:00:00Z",
    }
    if request.param == "Task":
        task["binding"] = "freeform"
    return validator, task


def test_public_task_without_recorded_expiry_is_valid(task_shape):
    validator, task = task_shape
    assert list(validator.iter_errors(task)) == []


@pytest.mark.parametrize(
    "expires_at",
    [
        "2026-10-18T12:00:00Z",
        "2026-10-09T12:00:00Z",
        "2026-10-18T08:00:00-04:00",
    ],
)
def test_public_task_paired_expiry_is_valid_even_when_past(task_shape, expires_at):
    validator, task = task_shape
    task.update(expiresAt=expires_at, expiresWithTimer="responseWindow")
    assert list(validator.iter_errors(task)) == []


@pytest.mark.parametrize("only_field", ["expiresAt", "expiresWithTimer"])
def test_public_task_half_expiry_is_rejected(task_shape, only_field):
    validator, task = task_shape
    values = {
        "expiresAt": "2026-10-18T12:00:00Z",
        "expiresWithTimer": "responseWindow",
    }
    task[only_field] = values[only_field]
    errors = list(validator.iter_errors(task))
    assert any(error.validator == "dependentRequired" for error in errors)


@pytest.mark.parametrize(
    "expires_at", [None, False, 123, "not-a-timestamp", "2026-10-18T12:00:00"]
)
def test_public_task_malformed_expiry_is_rejected(task_shape, expires_at):
    validator, task = task_shape
    task.update(expiresAt=expires_at, expiresWithTimer="responseWindow")
    errors = list(validator.iter_errors(task))
    assert any(
        list(error.path) == ["expiresAt"]
        and error.validator in {"type", "format"}
        for error in errors
    )


@pytest.mark.parametrize("timer_id", [None, 123, ""])
def test_public_task_malformed_timer_identifier_is_rejected(task_shape, timer_id):
    validator, task = task_shape
    task.update(expiresAt="2026-10-18T12:00:00Z", expiresWithTimer=timer_id)
    errors = list(validator.iter_errors(task))
    assert any(
        list(error.path) == ["expiresWithTimer"]
        and error.validator in {"type", "minLength"}
        for error in errors
    )


def test_public_task_expiry_does_not_open_other_properties(task_shape):
    validator, task = task_shape
    task.update(
        expiresAt="2026-10-18T12:00:00Z",
        expiresWithTimer="responseWindow",
        inventedExpiryPolicy="extend",
    )
    errors = list(validator.iter_errors(task))
    assert any(error.validator == "additionalProperties" for error in errors)


def test_public_task_list_advisory_deadline_does_not_require_expiry():
    schema = load_schema(API_SCHEMA_DIR / "task.schema.json")
    validator = validator_for_api_def(
        schema, "TaskListItem", api_referencing_registry()
    )
    process_id = "urn:wos:default_process_01hw7rm71vfay8vvw14d2pf2db"
    task = {
        "id": f"{process_id}/tasks/respond",
        "processId": process_id,
        "kind": "verification",
        "status": "pending",
        "title": "Provide a response",
        "createdAt": "2026-10-10T12:00:00Z",
        "deadline": "2026-10-18T12:00:00Z",
    }
    assert list(validator.iter_errors(task)) == []
