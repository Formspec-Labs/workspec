"""Continuation references locate original work without accepting new inputs."""

import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_schema, validator_for_api_def,
)


def validator(name):
    schema = load_schema(API_SCHEMA_DIR / "task.schema.json")
    return validator_for_api_def(schema, name, api_referencing_registry())


def test_continuation_accepts_only_the_original_nonsecret_reference():
    check = validator("TaskProgressionContinuation")
    check.validate({"acceptedCommandRef": "accepted_original_01"})
    for field in ("response", "actor", "scope", "credentialCustodyRef", "apply",
                  "idempotencyKey", "immutableInputDigest"):
        assert not check.is_valid({"acceptedCommandRef": "accepted_original_01", field: "invented"})
    assert not check.is_valid({})


@pytest.mark.parametrize("value", ["", "a" * 129, "../other", "urn:owner:other", None, 42])
def test_continuation_rejects_invalid_reference_syntax(value):
    assert not validator("TaskProgressionContinuation").is_valid({"acceptedCommandRef": value})


def test_reference_and_result_assert_committed_parent_without_child_completion_claim():
    reference = {"acceptedCommandRef": "accepted_original_01", "parentCommitted": True}
    check = validator("TaskProgressionReference")
    check.validate(reference)
    assert not check.is_valid({**reference, "parentCommitted": False})
    assert not check.is_valid({**reference, "childCompleted": True})
    result = {**reference, "applied": True, "outcomeRef": "outcome_original_01"}
    check = validator("TaskProgressionResult")
    check.validate(result)
    for field in result:
        assert not check.is_valid({key: value for key, value in result.items() if key != field})
    assert not check.is_valid({**result, "applied": False})
    assert not check.is_valid({**result, "credentialCustodyRef": "secret_locator"})
