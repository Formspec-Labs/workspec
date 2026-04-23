"""SignatureAffirmation provenance record schema regression tests (WOS-T4)."""
from __future__ import annotations

import copy
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


def _record() -> dict:
    return {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": "signatureAffirmation",
        "timestamp": "2026-04-22T14:30:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
        "data": {
            "signerId": "applicant",
            "roleId": "applicantSigner",
            "role": "signer",
            "documentId": "benefitsApplication",
            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "documentHashAlgorithm": "sha-256",
            "signedAt": "2026-04-22T14:30:00Z",
            "identityBinding": {
                "method": "email-otp",
                "assuranceLevel": "standard",
                "providerRef": "urn:agency.gov:identity:providers:email-otp",
            },
            "consentReference": {
                "consentTextRef": "urn:agency.gov:consent:esign-benefits:v1",
                "consentVersion": "1.0.0",
                "acceptedAtPath": "response.signature.acceptedAt",
                "affirmationPath": "response.signature.affirmed",
            },
            "signatureProvider": "urn:agency.gov:signature:providers:formspec",
            "ceremonyId": "ceremony-2026-0001",
            "profileRef": "urn:agency.gov:wos:signature-profile:benefits:v1",
            "formspecResponseRef": (
                "urn:agency.gov:formspec:responses:benefits:case-2026-0001"
            ),
            "custodyHookEligible": True,
        },
    }


def test_signature_affirmation_with_required_fields_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()

    errors = list(validator.iter_errors(record))

    assert errors == [], f"valid SignatureAffirmation rejected: {errors}"


def test_signature_affirmation_missing_data_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    del record["data"]

    errors = list(validator.iter_errors(record))

    assert errors, "SignatureAffirmation MUST carry data"


@pytest.mark.parametrize(
    "field",
    [
        "signerId",
        "roleId",
        "role",
        "documentId",
        "documentHash",
        "documentHashAlgorithm",
        "signedAt",
        "identityBinding",
        "consentReference",
        "signatureProvider",
        "ceremonyId",
        "formspecResponseRef",
        "custodyHookEligible",
    ],
)
def test_signature_affirmation_required_data_fields_are_rejected_when_missing(
    schema, field
):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    del record["data"][field]

    errors = list(validator.iter_errors(record))

    assert errors, f"SignatureAffirmation missing {field} must fail"


def test_signature_affirmation_requires_profile_ref_or_key(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    del record["data"]["profileRef"]

    errors = list(validator.iter_errors(record))

    assert errors, "SignatureAffirmation MUST carry profileRef or profileKey"


def test_signature_affirmation_rejects_profile_ref_and_key_together(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    record["data"]["profileKey"] = "benefitsSignature"

    errors = list(validator.iter_errors(record))

    assert errors, "SignatureAffirmation MUST NOT carry both profileRef and profileKey"


def test_signature_affirmation_accepts_profile_key_instead_of_ref(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    del record["data"]["profileRef"]
    record["data"]["profileKey"] = "benefitsSignature"

    errors = list(validator.iter_errors(record))

    assert errors == [], f"profileKey-only SignatureAffirmation rejected: {errors}"


def test_signature_affirmation_custody_hook_eligible_must_be_true(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _record()
    record["data"]["custodyHookEligible"] = False

    errors = list(validator.iter_errors(record))

    assert errors, "SignatureAffirmation custodyHookEligible must be true"


def test_non_signature_record_is_not_forced_into_signature_shape(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = copy.deepcopy(_record())
    record["recordKind"] = "stateTransition"
    record["data"] = {"some": "payload"}

    errors = list(validator.iter_errors(record))

    assert errors == [], f"non-signature records must remain unaffected: {errors}"
