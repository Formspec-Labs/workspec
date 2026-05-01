"""Delivery sidecar schema regression tests.

Guards the `$wosDelivery` sidecar after the correspondence vocabulary rename
from `actorType` to `correspondenceRole`.
"""
from __future__ import annotations

import json

from jsonschema import Draft202012Validator

from tests.schemas.conftest import WOS_SPEC_ROOT, validator_for_def

DELIVERY_SCHEMA = WOS_SPEC_ROOT / "schemas" / "sidecars" / "wos-delivery.schema.json"


def _minimal_delivery() -> dict:
    return {
        "$wosDelivery": "1.0",
        "targetWorkflow": "https://example.test/workflows/delivery",
        "correspondence": {
            "correspondenceField": "caseFile.correspondence",
            "entryTemplates": [
                {
                    "id": "inboundMail",
                    "channel": "mail",
                    "direction": "inbound",
                    "correspondenceRole": "applicant",
                }
            ],
        },
    }


def test_delivery_document_accepts_correspondence_role():
    schema = json.loads(DELIVERY_SCHEMA.read_text())
    errors = list(
        validator_for_def("CorrespondenceBlock").iter_errors(
            _minimal_delivery()["correspondence"]
        )
    )
    assert errors == [], f"valid correspondence block rejected: {errors}"

    validator = Draft202012Validator(schema)
    errors = list(validator.iter_errors(_minimal_delivery()))
    assert errors == [], f"valid delivery document rejected: {errors}"


def test_entry_template_rejects_actor_type():
    validator = validator_for_def("EntryTemplate")
    doc = {
        "id": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "actorType": "applicant",
    }
    errors = list(validator.iter_errors(doc))
    assert errors, "EntryTemplate.actorType should be rejected after rename"


def test_entry_template_rejects_unknown_correspondence_role():
    validator = validator_for_def("EntryTemplate")
    doc = {
        "id": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "correspondenceRole": "customer",
    }
    errors = list(validator.iter_errors(doc))
    assert errors, "EntryTemplate.correspondenceRole should stay closed"


def test_correspondence_entry_accepts_correspondence_role():
    validator = validator_for_def("CorrespondenceEntry")
    doc = {
        "templateRef": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "correspondenceRole": "applicant",
        "contentRef": "s3://agency-docs/cases/2025-001/mail/scan-20250315.pdf",
        "summary": "Applicant submitted income verification documents",
        "timestamp": "2025-03-15T14:30:00Z",
    }
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid correspondence entry rejected: {errors}"


def test_correspondence_entry_rejects_actor_type():
    validator = validator_for_def("CorrespondenceEntry")
    doc = {
        "templateRef": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "actorType": "applicant",
        "contentRef": "s3://agency-docs/cases/2025-001/mail/scan-20250315.pdf",
        "summary": "Applicant submitted income verification documents",
        "timestamp": "2025-03-15T14:30:00Z",
    }
    errors = list(validator.iter_errors(doc))
    assert errors, "CorrespondenceEntry.actorType should be rejected after rename"


def test_correspondence_entry_rejects_unknown_correspondence_role():
    validator = validator_for_def("CorrespondenceEntry")
    doc = {
        "templateRef": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "correspondenceRole": "customer",
        "contentRef": "s3://agency-docs/cases/2025-001/mail/scan-20250315.pdf",
        "summary": "Applicant submitted income verification documents",
        "timestamp": "2025-03-15T14:30:00Z",
    }
    errors = list(validator.iter_errors(doc))
    assert errors, "CorrespondenceEntry.correspondenceRole should stay closed"
