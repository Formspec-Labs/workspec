"""Kernel-schema intake-acceptance provenance regression tests.

Validates the ADR-0073 record-kind constraints layered into
``schemas/kernel/wos-provenance-record.schema.json`` for the WOS-owned
intake boundary: ``intakeAccepted``, ``intakeRejected``, ``intakeDeferred``,
and ``caseCreated``.

These tests intentionally pin only the kernel-owned minimum:

- canonical event literals,
- intake identity / binding / intent data for rejected/deferred intake decisions,
- accepted-intake disposition plus canonical governed-case reference,
- created governed-case reference plus governed-case outputs.

They do not freeze binding-specific ``data`` keys such as Formspec evidence
references, which remain owned by the binding seam.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
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


def _facts_record(record_kind: str, record_id: str | None = None, **extra) -> dict:
    record = {
        "id": record_id or "sba-poc_prov_01jqt0f0wm8f4b7n1j6m2r3k4p",
        "recordKind": record_kind,
        "timestamp": "2026-04-23T12:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


def test_intake_accepted_requires_event_data_and_outputs(schema):
    validator = _validator_for_def(schema, "IntakeAcceptedRecord")
    record = _facts_record(
        "intakeAccepted",
        event="case.intake.accepted",
        inputs=["handoff-public-2026-0001"],
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
            "caseDisposition": "createGovernedCase",
            "caseRef": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        },
    )

    assert list(validator.iter_errors(record)) == []


def test_intake_accepted_rejects_missing_outputs(schema):
    validator = _validator_for_def(schema, "IntakeAcceptedRecord")
    record = _facts_record(
        "intakeAccepted",
        event="case.intake.accepted",
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors, "accepted intake without governed-case outputs must fail"


def test_intake_accepted_rejects_missing_case_disposition(schema):
    validator = _validator_for_def(schema, "IntakeAcceptedRecord")
    record = _facts_record(
        "intakeAccepted",
        event="case.intake.accepted",
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
            "caseRef": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors, "accepted intake without caseDisposition must fail"


def test_intake_accepted_rejects_missing_case_ref(schema):
    validator = _validator_for_def(schema, "IntakeAcceptedRecord")
    record = _facts_record(
        "intakeAccepted",
        event="case.intake.accepted",
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
            "caseDisposition": "createGovernedCase",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors, "accepted intake without caseRef must fail"


def test_intake_rejected_requires_code_and_rejection_event(schema):
    validator = _validator_for_def(schema, "IntakeRejectedRecord")
    record = _facts_record(
        "intakeRejected",
        event="case.intake.rejected",
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
            "code": "publicIntakeDisabled",
        },
    )

    assert list(validator.iter_errors(record)) == []


def test_intake_deferred_requires_code_and_deferral_event(schema):
    validator = _validator_for_def(schema, "IntakeDeferredRecord")
    record = _facts_record(
        "intakeDeferred",
        event="case.intake.deferred",
        data={
            "binding": "formspec",
            "intakeId": "handoff-public-2026-0001",
            "caseIntent": "requestGovernedCaseCreation",
            "code": "manualReviewRequired",
        },
    )

    assert list(validator.iter_errors(record)) == []


def test_case_created_requires_case_created_event_and_outputs(schema):
    validator = _validator_for_def(schema, "CaseCreatedRecord")
    record = _facts_record(
        "caseCreated",
        event="case.created",
        inputs=["handoff-public-2026-0001"],
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
        data={
            "caseRef": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
            "initiationMode": "publicIntake",
        },
    )

    assert list(validator.iter_errors(record)) == []


def test_case_created_rejects_wrong_event_literal(schema):
    validator = _validator_for_def(schema, "CaseCreatedRecord")
    record = _facts_record(
        "caseCreated",
        event="case.intake.accepted",
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
    )

    errors = list(validator.iter_errors(record))

    assert errors, "caseCreated must pin the case.created event literal"


def test_case_created_rejects_missing_case_ref(schema):
    validator = _validator_for_def(schema, "CaseCreatedRecord")
    record = _facts_record(
        "caseCreated",
        event="case.created",
        outputs=["sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"],
        data={
            "initiationMode": "publicIntake",
        },
    )

    errors = list(validator.iter_errors(record))

    assert errors, "caseCreated must carry data.caseRef"
