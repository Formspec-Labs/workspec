"""Facts-tier ``outcome`` field schema regression tests.

Validates the optional ``outcome`` property on ``FactsTierRecord``. Canonical
``$defs`` (including ``ProvenanceOutcome``) live in
``schemas/wos-workflow.schema.json`` per ADR 0076; ``validator_for_def`` resolves
them registry-wide (same path as runtime ``wos-provenance-log`` → workflow
``$ref``).

The ``outcome`` field is an open enum (§4.3a #F5a): reserved literals
(``preconditionNotSatisfied`` per AI Integration §3.3.1,
``convergenceCapReached`` per Runtime §10.3) are shipped in-schema; vendor
extensions are admitted via an ``x-``-prefixed fallback branch.
"""

from __future__ import annotations

import pytest
from jsonschema import Draft202012Validator

from .conftest import validator_for_def

_EVENT_BY_KIND = {
    "capabilityInvocation": "wos.ai.capability_invocation",
    "stateTransition": "wos.kernel.state_transition",
}


def _facts_record(record_kind: str, **extra) -> dict:
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "event": _EVENT_BY_KIND.get(record_kind, f"x-test.{record_kind}"),
        "timestamp": "2026-04-19T12:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    if record_kind == "stateTransition" and "data" not in extra:
        record["data"] = {"transitionEvent": "submit"}
    record.update(extra)
    return record


def test_outcome_accepts_precondition_not_satisfied():
    validator = validator_for_def("FactsTierRecord")
    record = _facts_record(
        "capabilityInvocation",
        outcome="preconditionNotSatisfied",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        "Facts-tier record with the AI §3.3.1 reserved outcome must validate: "
        f"{errors}"
    )


def test_outcome_accepts_convergence_cap_reached():
    validator = validator_for_def("FactsTierRecord")
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


def test_outcome_accepts_vendor_extension_prefix():
    validator = validator_for_def("FactsTierRecord")
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


def test_outcome_rejects_unreserved_unprefixed_literal():
    validator = validator_for_def("FactsTierRecord")
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


def test_outcome_is_optional():
    validator = validator_for_def("FactsTierRecord")
    record = _facts_record("stateTransition")

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"outcome is OPTIONAL; a record without it must still validate: {errors}"
    )


def test_outcome_rejects_uppercase_vendor_extension():
    """The vendor-extension regex is aligned with the sibling convention
    (`^x-[a-z][a-z0-9-]*$`) used at ``schemas/wos-workflow.schema.json``
    ``ProvenanceOutcome`` (vendor branch, ca. line 3825). Uppercase-containing
    literals like `x-Acme-Foo` that an earlier (drifted) permissive
    regex `^x-[a-zA-Z][a-zA-Z0-9-]*$` would have accepted MUST be
    rejected so outcome vocabulary stays lowercase-kebab like the rest
    of the WOS vendor-extension surface (§4.3b #F5e)."""
    validator = validator_for_def("FactsTierRecord")
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


def test_outcome_accepts_lowercase_vendor_extension():
    """Lowercase-kebab vendor-extension outcomes continue to validate
    under the aligned regex."""
    validator = validator_for_def("FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        id="sba-poc_prov_01j5b6f5hms4g5c10f0d6qn4v8",
        outcome="x-acme-foo",
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], (
        f"Lowercase-kebab vendor extensions must still be accepted: {errors}"
    )


def test_outcome_rejects_bare_x_prefix():
    """Review B Finding 5: the vendor-extension pattern
    `^x-[a-z][a-z0-9-]*$` requires at least one lowercase letter after
    `x-`. A bare `x-` with no trailing identifier MUST be rejected --
    otherwise the collision-avoidance guarantee collapses into an empty
    sentinel that any future reserved literal could shadow."""
    validator = validator_for_def("FactsTierRecord")
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
