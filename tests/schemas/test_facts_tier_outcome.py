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


def _facts_record(record_kind: str, **extra) -> dict:
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": record_kind,
        "timestamp": "2026-04-19T12:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


def test_outcome_accepts_precondition_not_satisfied(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "capabilityInvocation",
        outcome="preconditionNotSatisfied",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "Facts-tier record with the AI §3.3.1 reserved outcome must validate: "
        f"{errors}"
    )


def test_outcome_accepts_convergence_cap_reached(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "caseStateMutation",
        id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
        outcome="convergenceCapReached",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "Facts-tier record with the Runtime §10.3 reserved outcome must "
        f"validate: {errors}"
    )


def test_outcome_accepts_vendor_extension_prefix(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01j5b6f5hms4g5c10f0d6qn4v8",
        outcome="x-vendor-specific",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "x-prefixed vendor extension outcomes must be accepted by the open "
        f"enum: {errors}"
    )


def test_outcome_rejects_unreserved_unprefixed_literal(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01j8dy7g3h36y8s3z5j4h3j7cw",
        outcome="arbitrary",
    )

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A literal that is neither reserved nor x-prefixed must be rejected -- "
        "otherwise vendor extensions lose their collision-avoidance guarantee."
    )


def test_outcome_is_optional(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record("stateTransition")

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"outcome is OPTIONAL; a record without it must still validate: {errors}"
    )


def test_outcome_rejects_uppercase_vendor_extension(schema):
    """The vendor-extension regex is aligned with the sibling convention
    (`^x-[a-z][a-z0-9-]*$`) used at `wos-kernel.schema.json:816` and
    `wos-workflow-governance.schema.json:1527`. Uppercase-containing
    literals like `x-Acme-Foo` that an earlier (drifted) permissive
    regex `^x-[a-zA-Z][a-zA-Z0-9-]*$` would have accepted MUST be
    rejected so outcome vocabulary stays lowercase-kebab like the rest
    of the WOS vendor-extension surface (§4.3b #F5e)."""
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
        outcome="x-Acme-Foo",
    )

    errors = list(validator.iter_errors(record))

    assert errors, (
        "Uppercase-containing vendor-extension outcomes must be rejected "
        "so the outcome literal vocabulary stays lowercase-kebab."
    )


def test_outcome_accepts_lowercase_vendor_extension(schema):
    """Lowercase-kebab vendor-extension outcomes continue to validate
    under the aligned regex."""
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01j5b6f5hms4g5c10f0d6qn4v8",
        outcome="x-acme-foo",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"Lowercase-kebab vendor extensions must still be accepted: {errors}"
    )


def test_outcome_rejects_bare_x_prefix(schema):
    """Review B Finding 5: the vendor-extension pattern
    `^x-[a-z][a-z0-9-]*$` requires at least one lowercase letter after
    `x-`. A bare `x-` with no trailing identifier MUST be rejected --
    otherwise the collision-avoidance guarantee collapses into an empty
    sentinel that any future reserved literal could shadow."""
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01j8dy7g3h36y8s3z5j4h3j7cw",
        outcome="x-",
    )

    errors = list(validator.iter_errors(record))

    assert errors, (
        "A bare `x-` prefix with no vendor-specific suffix must be rejected "
        "so the collision-avoidance guarantee stays meaningful."
    )
