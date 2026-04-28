"""Signature block schema regression tests (WOS-T4).

Guards the WOS Signature embedded block shape inside a $wosWorkflow document.
The signature block lives at `workflow.signature` per ADR 0076; the standalone
$wosSignatureProfile marker is retired. Cross-document reference integrity is
intentionally left to the planned SIG-* Tier 2 lint rules.
"""
from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_SCHEMA = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"


@pytest.fixture(scope="module")
def validator() -> Draft202012Validator:
    return Draft202012Validator(json.loads(WORKFLOW_SCHEMA.read_text()))


def _minimal_profile() -> dict:
    """Minimal $wosWorkflow document with a signature block."""
    return {
        "$wosWorkflow": "1.0",
        "url": "https://example.test/sig-test",
        "version": "1.0.0",
        "title": "Signature Profile Shape Test",
        "impactLevel": "operational",
        "actors": [
            {"id": "applicant", "type": "human"},
            {"id": "agency", "type": "system"},
        ],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic"},
                "done": {"type": "final"},
            },
        },
        "signature": {
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
        },
    }


def _errors(doc: dict, validator: Draft202012Validator):
    return list(validator.iter_errors(doc))


class TestSignatureProfilePositiveShapes:
    def test_minimal_single_signer_valid(self, validator):
        assert _errors(_minimal_profile(), validator) == []

    def test_sequential_signing_valid(self, validator):
        doc = _minimal_profile()
        doc["signature"]["signingFlow"]["type"] = "sequential"
        assert _errors(doc, validator) == []

    def test_parallel_signing_valid(self, validator):
        doc = _minimal_profile()
        doc["signature"]["roles"].append(
            {
                "id": "caseworkerApprover",
                "role": "approver",
                "actorId": "agency",
            }
        )
        doc["signature"]["signingFlow"] = {
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
        doc["signature"]["signingFlow"]["type"] = "routed"
        doc["signature"]["signingFlow"]["steps"][0]["guard"] = (
            "caseFile.signature.required == true"
        )
        assert _errors(doc, validator) == []

    def test_witness_countersignature_valid(self, validator):
        doc = _minimal_profile()
        doc["actors"].append({"id": "witness", "type": "human"})
        doc["signature"]["roles"].append(
            {
                "id": "witnessRole",
                "role": "witness",
                "actorId": "witness",
            }
        )
        doc["signature"]["signingFlow"]["steps"].append(
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
        doc["actors"] = [{"id": "notary", "type": "human"}]
        doc["signature"]["roles"][0] = {
            "id": "notaryRole",
            "role": "notary",
            "actorId": "notary",
            "authenticationPolicyKey": "notaryAuth",
        }
        doc["signature"]["authenticationPolicies"] = [
            {
                "key": "notaryAuth",
                "method": "notary",
                "assuranceLevel": "very-high",
                "requiresInPerson": True,
                "requiresCredentialEvidence": True,
            }
        ]
        doc["signature"]["signingFlow"]["steps"][0]["roleId"] = "notaryRole"
        doc["signature"]["evidence"]["identityBinding"] = {
            "method": "notary",
            "assuranceLevel": "very-high",
        }
        assert _errors(doc, validator) == []


class TestSignatureProfileNegativeShapes:
    def test_missing_consent_reference_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["evidence"]["consentReference"]
        assert _errors(doc, validator)

    def test_missing_document_hash_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["documents"][0]["documentHash"]
        assert _errors(doc, validator)

    def test_invalid_flow_type_rejected(self, validator):
        doc = _minimal_profile()
        doc["signature"]["signingFlow"]["type"] = "waterfall"
        assert _errors(doc, validator)

    def test_unknown_signature_property_rejected(self, validator):
        doc = _minimal_profile()
        doc["signature"]["providerBlob"] = {"vendor": "opaque"}
        assert _errors(doc, validator)

    def test_x_vendor_extension_on_signature_accepted(self, validator):
        doc = copy.deepcopy(_minimal_profile())
        doc["signature"]["x-acme-mode"] = "witnessed"
        assert _errors(doc, validator) == []

    def test_missing_signature_roles_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["roles"]
        assert _errors(doc, validator)

    def test_missing_signature_documents_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["documents"]
        assert _errors(doc, validator)

    def test_missing_signing_flow_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["signingFlow"]
        assert _errors(doc, validator)

    def test_missing_evidence_rejected(self, validator):
        doc = _minimal_profile()
        del doc["signature"]["evidence"]
        assert _errors(doc, validator)
