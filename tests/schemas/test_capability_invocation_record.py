"""Kernel-schema ``CapabilityInvocationRecord`` if/then regression tests.

Validates the §4.3b #F5d if/then branch in
``schemas/kernel/wos-provenance-record.schema.json`` that enforces
``outcome: "preconditionNotSatisfied"`` whenever a capability-invocation
record carries ``data.invocationBlocked: true`` (AI Integration §3.3.1).

The $def was relocated from the AI schema into the kernel provenance
schema so every conformant provenance log participates in the MUST via
``FactsTierRecord.allOf``, not only logs from workflows that separately
attach an AI Integration document.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "kernel" / "wos-provenance-record.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(PROVENANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    target = schema["$defs"][def_name]
    composed = {
        "$schema": schema.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{schema.get('$id', 'urn:test')}#${def_name}",
        "$defs": schema["$defs"],
        **target,
    }
    return Draft202012Validator(composed)


def _facts_record(record_kind: str, **extra) -> dict:
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": record_kind,
        "timestamp": "2026-04-22T14:30:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


def test_blocked_invocation_with_correct_outcome_is_accepted(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
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


def test_blocked_invocation_missing_outcome_is_rejected(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
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


def test_blocked_invocation_with_wrong_outcome_is_rejected(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
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


def test_unblocked_invocation_without_outcome_is_accepted(schema):
    """When `invocationBlocked` is false (or absent), the if branch does not
    match, so the then-required outcome is NOT mandated. This keeps the
    happy-path record shape unconstrained.
    """
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
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


def test_absent_invocation_blocked_not_required_outcome(schema):
    """Review B Finding 5: when `data.invocationBlocked` is absent, the
    if branch's `required: ["invocationBlocked"]` on the `data` subschema
    does not match, so the then branch does not apply. A capability-
    invocation record with no `invocationBlocked` flag MUST therefore
    validate without carrying an `outcome` -- otherwise the MUST would
    over-fire on records that predate the precondition gate."""
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
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


def test_non_capability_record_kind_with_blocked_flag_not_required_outcome(schema):
    """Review B Finding 5: the if-guard is keyed on
    `recordKind == "capabilityInvocation"` AND `data.invocationBlocked == true`.
    A record with a DIFFERENT recordKind that happens to carry
    `data.invocationBlocked: true` MUST NOT be forced to
    `outcome = "preconditionNotSatisfied"` -- the MUST is scoped to the
    AI §3.3.1 capability-invocation path, not every provenance record
    whose payload reuses the field name."""
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
        data={
            "invocationBlocked": True,
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "A record with a non-`capabilityInvocation` recordKind must "
        f"validate without `outcome` even when its data reuses the "
        f"invocationBlocked field: {errors}"
    )
