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


def test_blocked_invocation_with_correct_outcome_is_accepted(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
    record = {
        "recordKind": "capabilityInvocation",
        "data": {
            "invocationBlocked": True,
            "capabilityId": "documentExtraction",
        },
        "outcome": "preconditionNotSatisfied",
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "A blocked capability invocation with the correct outcome literal must "
        f"validate: {errors}"
    )


def test_blocked_invocation_missing_outcome_is_rejected(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
    record = {
        "recordKind": "capabilityInvocation",
        "data": {"invocationBlocked": True},
    }

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A blocked capability invocation MUST carry outcome; omitting it must "
        "fail schema validation per AI §3.3.1 point 4."
    )


def test_blocked_invocation_with_wrong_outcome_is_rejected(schema):
    validator = _validator_for_def(schema, "CapabilityInvocationRecord")
    record = {
        "recordKind": "capabilityInvocation",
        "data": {"invocationBlocked": True},
        "outcome": "somethingElse",
    }

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
    record = {
        "recordKind": "capabilityInvocation",
        "data": {
            "invocationBlocked": False,
            "capabilityId": "eligibilityScreener",
        },
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"A successful capability invocation must validate without outcome: {errors}"
    )
