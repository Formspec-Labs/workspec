"""Kernel-schema ``CapabilityInvocationRecord`` if/then regression tests.

Validates the §4.3b #F5d if/then branch on the ``CapabilityInvocationRecord``
``$def`` in ``schemas/wos-workflow.schema.json`` (ADR 0076 step 5 promotion).
It is composed into ``FactsTierRecord.allOf`` there; runtime
``wos-provenance-log.schema.json`` items ``$ref`` that ``FactsTierRecord``, so
the same MUST applies to exported logs.
"""

from __future__ import annotations

import pytest
from jsonschema import Draft202012Validator

from .conftest import validator_for_def

_EVENT_BY_KIND = {
    "capabilityInvocation": "wos.ai.capability_invocation",
    "stateTransition": "wos.kernel.state_transition",
}


@pytest.fixture(scope="module")
def cap_validator() -> Draft202012Validator:
    return validator_for_def("CapabilityInvocationRecord")


def _facts_record(record_kind: str, **extra) -> dict:
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "event": _EVENT_BY_KIND[record_kind],
        "timestamp": "2026-04-22T14:30:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


def test_blocked_invocation_with_correct_outcome_is_accepted(cap_validator):
    validator = cap_validator
    record = _facts_record(
        "capabilityInvocation",
        data={
            "invocationBlocked": True,
            "capabilityId": "documentExtraction",
        },
        outcome="preconditionNotSatisfied",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "A blocked capability invocation with the correct outcome literal must "
        f"validate: {errors}"
    )


def test_blocked_invocation_missing_outcome_is_rejected(cap_validator):
    validator = cap_validator
    record = _facts_record(
        "capabilityInvocation",
        id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
        data={"invocationBlocked": True},
    )

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A blocked capability invocation MUST carry outcome; omitting it must "
        "fail schema validation per AI §3.3.1 point 4."
    )


def test_blocked_invocation_with_wrong_outcome_is_rejected(cap_validator):
    validator = cap_validator
    record = _facts_record(
        "capabilityInvocation",
        id="sba-poc_prov_01j5b6f5hms4g5c10f0d6qn4v8",
        data={"invocationBlocked": True},
        outcome="somethingElse",
    )

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A blocked capability invocation with a non-reserved outcome literal "
        "must fail -- the if/then branch pins outcome to "
        "`preconditionNotSatisfied`."
    )


def test_unblocked_invocation_without_outcome_is_accepted(cap_validator):
    """When `invocationBlocked` is false (or absent), the if branch does not
    match, so the then-required outcome is NOT mandated. This keeps the
    happy-path record shape unconstrained.
    """
    validator = cap_validator
    record = _facts_record(
        "capabilityInvocation",
        id="sba-poc_prov_01j8dy7g3h36y8s3z5j4h3j7cw",
        data={
            "invocationBlocked": False,
            "capabilityId": "eligibilityScreener",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"A successful capability invocation must validate without outcome: {errors}"
    )


def test_absent_invocation_blocked_not_required_outcome(cap_validator):
    """Review B Finding 5: when `data.invocationBlocked` is absent, the
    if branch's `required: ["invocationBlocked"]` on the `data` subschema
    does not match, so the then branch does not apply. A capability-
    invocation record with no `invocationBlocked` flag MUST therefore
    validate without carrying an `outcome` -- otherwise the MUST would
    over-fire on records that predate the precondition gate."""
    validator = cap_validator
    record = _facts_record(
        "capabilityInvocation",
        data={
            "capabilityId": "documentExtraction",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "A capability-invocation record without `data.invocationBlocked` "
        f"must validate without `outcome`: {errors}"
    )


def test_non_capability_event_with_blocked_flag_not_required_outcome(cap_validator):
    """Review B Finding 5: the if-guard is keyed on
    `event == "wos.ai.capability_invocation"` AND
    `data.invocationBlocked == true`. A record with a DIFFERENT event that
    happens to carry `data.invocationBlocked: true` MUST NOT be forced to
    `outcome = "preconditionNotSatisfied"` -- the MUST is scoped to the
    AI §3.3.1 capability-invocation path, not every provenance record
    whose payload reuses the field name."""
    validator = cap_validator
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
        data={
            "invocationBlocked": True,
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "A record with a non-capability-invocation event must "
        f"validate without `outcome` even when its data reuses the "
        f"invocationBlocked field: {errors}"
    )


def test_standalone_def_rejects_vacuous_empty_object(cap_validator):
    """The $def carries an explicit `not` guard so standalone validation
    cannot accept the vacuous empty object `{}` (SCHEMA review: fragments
    must not trivially validate with no asserted shape)."""
    validator = cap_validator
    errors = list(validator.iter_errors({}))
    assert errors, (
        "CapabilityInvocationRecord MUST reject an empty object at the "
        f"$def root: {errors}"
    )
    assert any("maxProperties" in e.message for e in errors), (
        f"expected vacuity rejection via `not`+maxProperties, got: {errors}"
    )
