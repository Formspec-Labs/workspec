"""Facts-tier ``outcome`` field schema regression tests.

Validates the open-enum ``ProvenanceOutcome`` ``$def`` and the optional
``outcome`` property on ``FactsTierRecord`` in
``schemas/kernel/wos-provenance-record.schema.json``.

The ``outcome`` field is an open enum (§4.3a #F5a): reserved literals
(``preconditionNotSatisfied`` per AI Integration §3.3.1,
``convergenceCapReached`` per Runtime §10.3) are shipped in-schema; vendor
extensions are admitted via an ``x-``-prefixed fallback branch.
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


def test_outcome_accepts_precondition_not_satisfied(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "capabilityInvocation",
        "outcome": "preconditionNotSatisfied",
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "Facts-tier record with the AI §3.3.1 reserved outcome must validate: "
        f"{errors}"
    )


def test_outcome_accepts_convergence_cap_reached(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "caseStateMutation",
        "outcome": "convergenceCapReached",
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "Facts-tier record with the Runtime §10.3 reserved outcome must "
        f"validate: {errors}"
    )


def test_outcome_accepts_vendor_extension_prefix(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
        "outcome": "x-vendor-specific",
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "x-prefixed vendor extension outcomes must be accepted by the open "
        f"enum: {errors}"
    )


def test_outcome_rejects_unreserved_unprefixed_literal(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
        "outcome": "arbitrary",
    }

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A literal that is neither reserved nor x-prefixed must be rejected -- "
        "otherwise vendor extensions lose their collision-avoidance guarantee."
    )


def test_outcome_is_optional(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"outcome is OPTIONAL; a record without it must still validate: {errors}"
    )
