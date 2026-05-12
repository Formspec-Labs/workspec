"""SignatureAffirmation provenance record schema regression tests (WOS-T4)."""
from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker

from .conftest import _REGISTRY

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(PROVENANCE_SCHEMA.read_text())


def _document_validator(schema: dict) -> Draft202012Validator:
    """Validate export-shaped documents; composes workflow ``FactsTierRecord``
    with provenance-local ``SignatureAffirmationRecord`` via ``items.allOf``."""
    return Draft202012Validator(
        schema,
        registry=_REGISTRY,
        format_checker=FormatChecker(),
    )


def _record() -> dict:
    return {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": "signatureAffirmation",
        "timestamp": "2026-04-22T14:30:00Z",
        "auditLayer": "facts",
        "event": "wos.kernel.signature_affirmation",
        "definitionVersion": "1.0.0",
        "data": {
            "caseLedgerId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
            "processId": "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd",
            "signerId": "applicant",
            "roleId": "applicantSigner",
            "role": "signer",
            "documentId": "benefitsApplication",
            "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "documentHashAlgorithm": "sha-256",
            "sourceSignatureSystem": "formspec",
            "sourceSignatureId": "sig-2026-0001",
            "signedPayloadDigest": "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "signedPayloadDigestAlgorithm": "sha-256",
            "signingIntent": "urn:wos:signing-intent:applicant-signature",
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
            "sourceResponseRef": (
                "urn:agency.gov:formspec:responses:benefits:case-2026-0001"
            ),
            "custodyHookEligible": True,
            "primitiveVerification": {
                "status": "deferredPendingHelper",
                "reason": "formspec-signing-helper-pending",
            },
        },
    }


def _log(record: dict) -> dict:
    return {"$wosProvenanceLog": "1.0", "provenanceLog": [record]}


def test_signature_affirmation_with_required_fields_is_accepted(schema):
    validator = _document_validator(schema)
    errors = list(validator.iter_errors(_log(_record())))

    assert errors == [], f"valid SignatureAffirmation rejected: {errors}"


def test_signature_affirmation_missing_data_is_rejected(schema):
    validator = _document_validator(schema)
    record = _record()
    del record["data"]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "SignatureAffirmation MUST carry data"


@pytest.mark.parametrize(
    "field",
    [
        "caseLedgerId",
        "signerId",
        "roleId",
        "role",
        "documentId",
        "documentHash",
        "documentHashAlgorithm",
        "sourceSignatureSystem",
        "sourceSignatureId",
        "signedPayloadDigest",
        "signedPayloadDigestAlgorithm",
        "signingIntent",
        "signedAt",
        "identityBinding",
        "consentReference",
        "signatureProvider",
        "ceremonyId",
        "sourceResponseRef",
        "custodyHookEligible",
        "primitiveVerification",
    ],
)
def test_signature_affirmation_required_data_fields_are_rejected_when_missing(
    schema, field
):
    validator = _document_validator(schema)
    record = _record()
    del record["data"][field]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, f"SignatureAffirmation missing {field} must fail"


def test_signature_affirmation_rejects_swapped_identity_families(schema):
    validator = _document_validator(schema)
    record = _record()
    record["data"]["caseLedgerId"] = "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd"
    record["data"]["processId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc"

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "signature decision identity fields must use their reserved families"


def test_signature_affirmation_requires_profile_ref_or_key(schema):
    validator = _document_validator(schema)
    record = _record()
    del record["data"]["profileRef"]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "SignatureAffirmation MUST carry profileRef or profileKey"


def test_signature_affirmation_rejects_profile_ref_and_key_together(schema):
    validator = _document_validator(schema)
    record = _record()
    record["data"]["profileKey"] = "benefitsSignature"

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "SignatureAffirmation MUST NOT carry both profileRef and profileKey"


def test_signature_affirmation_accepts_profile_key_instead_of_ref(schema):
    validator = _document_validator(schema)
    record = _record()
    del record["data"]["profileRef"]
    record["data"]["profileKey"] = "benefitsSignature"

    errors = list(validator.iter_errors(_log(record)))

    assert errors == [], f"profileKey-only SignatureAffirmation rejected: {errors}"


def test_signature_affirmation_custody_hook_eligible_must_be_true(schema):
    validator = _document_validator(schema)
    record = _record()
    record["data"]["custodyHookEligible"] = False

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "SignatureAffirmation custodyHookEligible must be true"


def test_non_signature_record_is_not_forced_into_signature_shape(schema):
    validator = _document_validator(schema)
    record = copy.deepcopy(_record())
    record["recordKind"] = "stateTransition"
    record["event"] = "decide"
    record["data"] = {"some": "payload"}

    errors = list(validator.iter_errors(_log(record)))

    assert errors == [], f"non-signature records must remain unaffected: {errors}"


def test_signature_affirmation_event_selects_signature_shape(schema):
    validator = _document_validator(schema)
    record = copy.deepcopy(_record())
    record["recordKind"] = "stateTransition"
    del record["data"]["signerId"]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "wos.kernel.signature_affirmation must select SignatureAffirmation data"


def _admission_failed_record() -> dict:
    return {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": "signatureAdmissionFailed",
        "timestamp": "2026-05-08T10:30:00Z",
        "auditLayer": "facts",
        "event": "wos.kernel.signature_admission_failed",
        "definitionVersion": "1.0.0",
        "data": {
            "caseLedgerId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc",
            "processId": "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd",
            "reason": "primitive_verification_failed",
            "evidenceBindings": {
                "responseId": "resp-2026-0001",
                "signedPayloadDigest": "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
                "signatureId": "sig-2026-0001",
                "signingIntent": "urn:wos:signing-intent:applicant-signature",
            },
            "emittedAt": "2026-05-08T10:30:00Z",
        },
    }


def test_signature_admission_failed_accepts_f11_identity(schema):
    validator = _document_validator(schema)
    errors = list(validator.iter_errors(_log(_admission_failed_record())))

    assert errors == [], f"valid SignatureAdmissionFailed rejected: {errors}"


def test_signature_admission_failed_event_selects_failure_shape(schema):
    validator = _document_validator(schema)
    record = _admission_failed_record()
    record["recordKind"] = "stateTransition"
    del record["data"]["reason"]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, (
        "wos.kernel.signature_admission_failed must select "
        "SignatureAdmissionFailed data"
    )


def test_signature_admission_failed_requires_case_ledger_id(schema):
    validator = _document_validator(schema)
    record = _admission_failed_record()
    del record["data"]["caseLedgerId"]

    errors = list(validator.iter_errors(_log(record)))

    assert errors, "SignatureAdmissionFailed MUST carry caseLedgerId"
