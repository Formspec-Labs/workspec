"""Signature Profile schema regression tests (WOS-T4).

Guards the WOS Signature Profile authoring shape before lint/runtime
semantics land. Cross-document reference integrity is intentionally left to
the planned SIG-* Tier 2 lint rules.
"""
from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
SIGNATURE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "profiles" / "wos-signature-profile.schema.json"
)


@pytest.fixture(scope="module")
def validator() -> Draft202012Validator:
    return Draft202012Validator(json.loads(SIGNATURE_SCHEMA.read_text()))


def _minimal_profile() -> dict:
    return {
        "$wosSignatureProfile": "1.0",
        "targetWorkflow": {
            "url": "https://agency.gov/workflows/benefits-adjudication"
        },
        "roles": [
            {
                "id": "applicantSigner",
                "role": "signer",
                "actorId": "applicant",
                "authenticationPolicyKey": "emailOtp",
            }
        ],
        "documents": [
            {
                "id": "benefitsApplication",
                "documentRef": "urn:agency.gov:documents:benefits-application:v1",
                "documentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "documentHashAlgorithm": "sha-256",
            }
        ],
        "authenticationPolicies": [
            {
                "key": "emailOtp",
                "method": "email-otp",
                "assuranceLevel": "standard",
            }
        ],
        "signingFlow": {
            "type": "sequential",
            "steps": [
                {
                    "id": "applicantSigns",
                    "roleId": "applicantSigner",
                    "documentId": "benefitsApplication",
                }
            ],
            "completion": {"type": "all-required"},
        },
        "evidence": {
            "recordKind": "signatureAffirmation",
            "requiredFields": [
                "response.signature.acceptedAt",
                "response.signature.affirmed",
            ],
            "consentReference": {
                "consentTextRef": "urn:agency.gov:consent:esign-benefits:v1",
                "consentVersion": "1.0.0",
                "acceptedAtPath": "response.signature.acceptedAt",
                "affirmationPath": "response.signature.affirmed",
            },
            "identityBinding": {
                "method": "email-otp",
                "assuranceLevel": "standard",
            },
            "custodyHookEligible": True,
        },
    }


def _errors(doc: dict, validator: Draft202012Validator):
    return list(validator.iter_errors(doc))


class TestSignatureProfilePositiveShapes:
    def test_minimal_single_signer_valid(self, validator):
        assert _errors(_minimal_profile(), validator) == []

    def test_sequential_signing_valid(self, validator):
        doc = _minimal_profile()
        doc["signingFlow"]["type"] = "sequential"
        assert _errors(doc, validator) == []

    def test_parallel_signing_valid(self, validator):
        doc = _minimal_profile()
        doc["roles"].append(
            {
                "id": "caseworkerApprover",
                "role": "approver",
                "actorId": "caseworker",
            }
        )
        doc["signingFlow"] = {
            "type": "parallel",
            "steps": [
                {
                    "id": "applicantSigns",
                    "roleId": "applicantSigner",
                    "documentId": "benefitsApplication",
                },
                {
                    "id": "caseworkerApproves",
                    "roleId": "caseworkerApprover",
                    "documentId": "benefitsApplication",
                },
            ],
        }
        assert _errors(doc, validator) == []

    def test_routed_signing_valid(self, validator):
        doc = _minimal_profile()
        doc["signingFlow"]["type"] = "routed"
        doc["signingFlow"]["steps"][0]["guard"] = "caseFile.signature.required == true"
        assert _errors(doc, validator) == []

    def test_witness_countersignature_valid(self, validator):
        doc = _minimal_profile()
        doc["roles"].append(
            {
                "id": "witnessRole",
                "role": "witness",
                "actorId": "witness",
            }
        )
        doc["signingFlow"]["steps"].append(
            {
                "id": "witnessSigns",
                "roleId": "witnessRole",
                "documentId": "benefitsApplication",
                "dependsOn": ["applicantSigns"],
            }
        )
        assert _errors(doc, validator) == []

    def test_notary_in_person_valid(self, validator):
        doc = _minimal_profile()
        doc["roles"][0] = {
            "id": "notaryRole",
            "role": "notary",
            "actorId": "notary",
            "authenticationPolicyKey": "notaryAuth",
        }
        doc["authenticationPolicies"] = [
            {
                "key": "notaryAuth",
                "method": "notary",
                "assuranceLevel": "very-high",
                "requiresInPerson": True,
                "requiresCredentialEvidence": True,
            }
        ]
        doc["signingFlow"]["steps"][0]["roleId"] = "notaryRole"
        doc["evidence"]["identityBinding"] = {
            "method": "notary",
            "assuranceLevel": "very-high",
        }
        assert _errors(doc, validator) == []


class TestSignatureProfileNegativeShapes:
    def test_missing_consent_reference_rejected(self, validator):
        doc = _minimal_profile()
        del doc["evidence"]["consentReference"]
        assert _errors(doc, validator)

    def test_missing_document_hash_rejected(self, validator):
        doc = _minimal_profile()
        del doc["documents"][0]["documentHash"]
        assert _errors(doc, validator)

    def test_invalid_signer_role_rejected(self, validator):
        doc = _minimal_profile()
        doc["roles"][0]["role"] = "rubber-stamp"
        assert _errors(doc, validator)

    def test_invalid_authentication_method_rejected(self, validator):
        doc = _minimal_profile()
        doc["authenticationPolicies"][0]["method"] = "magic-link"
        assert _errors(doc, validator)

    def test_invalid_flow_type_rejected(self, validator):
        doc = _minimal_profile()
        doc["signingFlow"]["type"] = "waterfall"
        assert _errors(doc, validator)

    def test_unknown_root_property_rejected(self, validator):
        doc = _minimal_profile()
        doc["providerBlob"] = {"vendor": "opaque"}
        assert _errors(doc, validator)

    def test_non_x_vendor_extension_rejected(self, validator):
        doc = _minimal_profile()
        doc["extensions"] = {"vendorMode": "opaque"}
        assert _errors(doc, validator)

    def test_x_vendor_extension_accepted(self, validator):
        doc = copy.deepcopy(_minimal_profile())
        doc["extensions"] = {"x-acme-mode": "witnessed"}
        assert _errors(doc, validator) == []
