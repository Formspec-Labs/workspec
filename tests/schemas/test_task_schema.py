"""Public task identity and signed-source preconditions remain bounded shapes."""
import copy

import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_schema, validator_for_api_def,
)

SCHEMA = load_schema(API_SCHEMA_DIR / "task.schema.json")


def validator(name):
    return validator_for_api_def(SCHEMA, name, api_referencing_registry())


@pytest.mark.parametrize("shape", ["Task", "TaskListItem"])
def test_optional_actual_case_identity_and_task_revision(shape):
    process = "urn:wos:default_process_01hw7rm71vfay8vvw14d2pf2db"
    task = {"id": process + "/tasks/respond", "processId": process,
            "kind": "verification", "status": "pending", "title": "Respond",
            "createdAt": "2026-10-10T12:00:00Z", "updatedAt": "2026-10-10T12:00:00Z"}
    if shape == "Task":
        task["binding"] = "freeform"
    check = validator(shape)
    check.validate(task)
    case = "urn:wos:default_case_01hw7rm71vfay8vvw14d2pf2db"
    check.validate({**task, "caseLedgerId": case, "taskRevision": "stored-revision"})
    for field, value in [("caseLedgerId", "invented"), ("caseLedgerId", None),
                         ("taskRevision", ""), ("taskRevision", None),
                         ("taskRevision", 42)]:
        assert not check.is_valid({**task, field: value})


def submission():
    return {"definitionUrl": "urn:form:response", "definitionVersion": "1.0.0",
            "response": {"answer": "recorded"}}


def signed_preconditions():
    return {"draftId": "source-draft", "expectedSignedDraftVersion": 2,
            "expectedSignedResponseHash": "sha256:" + "a" * 64,
            "associationHash": "sha256:" + "b" * 64,
            "taskBindingHash": "sha256:" + "c" * 64}


def test_signed_source_preconditions_are_optional_complete_and_closed():
    check = validator("TaskSubmission")
    check.validate(submission())
    signed = signed_preconditions()
    check.validate({**submission(), "signedDraft": signed})
    for field in signed:
        incomplete = dict(signed)
        del incomplete[field]
        assert not check.is_valid({**submission(), "signedDraft": incomplete})
    for field in ("actor", "scope", "owner", "token", "workflowUrl"):
        assert not check.is_valid({**submission(), "signedDraft": {**signed, field: "invented"}})
    for value in (None, [], "opaque"):
        assert not check.is_valid({**submission(), "signedDraft": value})


@pytest.mark.parametrize("field,value", [
    ("draftId", ""), ("draftId", None),
    ("expectedSignedDraftVersion", 1), ("expectedSignedDraftVersion", 2.5),
    ("expectedSignedDraftVersion", True), ("expectedSignedDraftVersion", "2"),
])
def test_signed_source_identity_and_version_bounds(field, value):
    signed = signed_preconditions()
    signed[field] = value
    assert not validator("TaskSubmission").is_valid({**submission(), "signedDraft": signed})


@pytest.mark.parametrize("field", ["expectedSignedResponseHash", "associationHash", "taskBindingHash"])
@pytest.mark.parametrize("value", [None, "sha256:" + "A" * 64, "sha256:" + "a" * 63,
                                   "sha512:" + "a" * 64, "a" * 64])
def test_signed_source_hash_preconditions_keep_exact_typed_digest_bounds(field, value):
    signed = copy.deepcopy(signed_preconditions())
    signed[field] = value
    assert not validator("TaskSubmission").is_valid({**submission(), "signedDraft": signed})
