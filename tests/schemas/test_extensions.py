"""F1 — five WOS extensions + custody allOf regression tests.

Covers the kernel surface added 2026-05-02 per the F1 finishing wave:

- ``ApplicabilityScope`` ($def) — programs/jurisdictions/dateRanges scope.
- ``EffectivePeriod`` ($def) — name/intervals/retroactive/sunset.
- ``DeonticConstraint`` ($def) — OASIS LegalRuleML §3 expansion of the
  previous ``{kind, expression}`` agents[].deonticConstraints shape.
- ``WosVersionPin`` ($def) + top-level ``wosVersionPin`` property.
- ``FieldDeclaration.canonicalTermRef`` + ``dpvSensitivity``.
- The custody conditional ``allOf`` clause: rights-impacting and
  safety-impacting workflows MUST embed ``custody``.

Each surface gets:
- A positive case (well-formed values validate).
- A negative case (malformed value or required-property violation
  fails the appropriate schema rule).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker

from .conftest import validator_for_def, validators

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_SCHEMA_PATH = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"


def _minimal_rights_impacting_doc() -> dict:
    """Minimal $wosWorkflow document at impactLevel=rights-impacting,
    carrying the four post-F1.6 conditionally-required blocks
    (governance, signature is optional unless transitions gate on it,
    custody MUST be present after F1.6).
    """
    return {
        "$wosWorkflow": "1.0",
        "url": "https://example.gov/workflows/test",
        "version": "1.0.0",
        "title": "Test workflow",
        "impactLevel": "rights-impacting",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic"},
                "done": {"type": "final"},
            },
        },
        "governance": {},
        "custody": {"trustProfileRef": "urn:trellis:trust-profile/dev"},
    }


# --- F1.6 custody conditional ---------------------------------------------

class TestCustodyConditional:
    def test_rights_impacting_without_custody_fails(self, validators):
        doc = _minimal_rights_impacting_doc()
        del doc["custody"]
        errors = list(validators["$wosWorkflow"].iter_errors(doc))
        assert any(
            "'custody' is a required property" in str(e.message) for e in errors
        ), "rights-impacting workflows MUST embed custody (ADR-0076)"

    def test_safety_impacting_without_custody_fails(self, validators):
        doc = _minimal_rights_impacting_doc()
        doc["impactLevel"] = "safety-impacting"
        del doc["custody"]
        errors = list(validators["$wosWorkflow"].iter_errors(doc))
        assert any(
            "'custody' is a required property" in str(e.message) for e in errors
        ), "safety-impacting workflows MUST embed custody (ADR-0076)"

    def test_operational_without_custody_passes(self, validators):
        doc = _minimal_rights_impacting_doc()
        doc["impactLevel"] = "operational"
        del doc["custody"]
        del doc["governance"]
        errors = list(validators["$wosWorkflow"].iter_errors(doc))
        # Custody is NOT required at operational tier.
        custody_errors = [
            e for e in errors if "custody" in str(e.message)
        ]
        assert custody_errors == [], (
            "operational workflows do NOT need custody"
        )

    def test_rights_impacting_with_custody_passes(self, validators):
        doc = _minimal_rights_impacting_doc()
        errors = list(validators["$wosWorkflow"].iter_errors(doc))
        assert errors == [], f"baseline rights-impacting+custody must validate: {errors}"


# --- F1.1 ApplicabilityScope ---------------------------------------------

class TestApplicabilityScope:
    def test_minimal_scope_validates(self):
        validator = validator_for_def("ApplicabilityScope")
        doc = {
            "programs": ["SNAP"],
            "jurisdictions": ["US-CA"],
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"minimal applicability scope must validate: {errors}"

    def test_full_scope_validates(self):
        validator = validator_for_def("ApplicabilityScope")
        doc = {
            "programs": ["SNAP", "MEDICAID"],
            "jurisdictions": ["US-CA", "US-NY"],
            "dateRanges": [
                {"start": "2024-10-01T00:00:00Z", "end": "2025-09-30T23:59:59Z"}
            ],
            "caseFilters": "caseFile.householdSize >= 4",
            "effectivenessRef": "urn:wos:effectiveness/snap-2024-fy",
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"full applicability scope must validate: {errors}"

    def test_unknown_property_rejected(self):
        validator = validator_for_def("ApplicabilityScope")
        doc = {"programs": ["SNAP"], "unknownKey": "x"}
        errors = list(validator.iter_errors(doc))
        assert errors, "additionalProperties=false MUST reject unknown keys"

    def test_dateRange_without_start_rejected(self):
        validator = validator_for_def("ApplicabilityScope")
        doc = {"dateRanges": [{"end": "2025-01-01T00:00:00Z"}]}
        errors = list(validator.iter_errors(doc))
        assert errors, "dateRanges[*].start is required"


# --- F1.2 EffectivePeriod -------------------------------------------------

class TestEffectivePeriod:
    def test_minimal_period_validates(self):
        validator = validator_for_def("EffectivePeriod")
        doc = {
            "intervals": [
                {"start": "2020-03-18T00:00:00Z", "end": "2023-04-01T00:00:00Z"}
            ]
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"minimal effective period must validate: {errors}"

    def test_retroactive_and_sunset_validate(self):
        validator = validator_for_def("EffectivePeriod")
        doc = {
            "name": "Test PHE",
            "intervals": [{"start": "2020-03-18T00:00:00Z"}],
            "retroactiveFrom": "2020-01-01T00:00:00Z",
            "sunsetAt": "2030-01-01T00:00:00Z",
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"retroactive/sunset shape must validate: {errors}"

    def test_unknown_property_rejected(self):
        validator = validator_for_def("EffectivePeriod")
        doc = {"name": "x", "intervals": [], "extra": True}
        errors = list(validator.iter_errors(doc))
        assert errors, "additionalProperties=false MUST reject unknown keys"


# --- F1.3 WosVersionPin ---------------------------------------------------

class TestWosVersionPin:
    def test_minimal_pin_validates(self):
        validator = validator_for_def("WosVersionPin")
        doc = {"envelopeVersion": "1.0"}
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"minimal version pin must validate: {errors}"

    def test_full_pin_validates(self):
        validator = validator_for_def("WosVersionPin")
        doc = {
            "envelopeVersion": "1.0",
            "includedBlocks": ["governance", "signature", "custody"],
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"full version pin must validate: {errors}"

    def test_missing_envelope_version_rejected(self):
        validator = validator_for_def("WosVersionPin")
        doc = {"includedBlocks": ["governance"]}
        errors = list(validator.iter_errors(doc))
        assert errors, "envelopeVersion is required"

    def test_malformed_envelope_version_rejected(self):
        validator = validator_for_def("WosVersionPin")
        doc = {"envelopeVersion": "1"}  # Pattern requires major.minor.
        errors = list(validator.iter_errors(doc))
        assert errors, "envelopeVersion pattern is ^\\d+\\.\\d+$"

    def test_unknown_block_rejected(self):
        validator = validator_for_def("WosVersionPin")
        doc = {"envelopeVersion": "1.0", "includedBlocks": ["notARealBlock"]}
        errors = list(validator.iter_errors(doc))
        assert errors, "includedBlocks enum MUST reject unknown values"

    def test_top_level_property_validates_in_workflow(self, validators):
        doc = _minimal_rights_impacting_doc()
        doc["wosVersionPin"] = {
            "envelopeVersion": "1.0",
            "includedBlocks": ["governance", "custody"],
        }
        errors = list(validators["$wosWorkflow"].iter_errors(doc))
        assert errors == [], f"workflow with wosVersionPin must validate: {errors}"


# --- F1.4 DeonticConstraint expansion -------------------------------------

class TestDeonticConstraint:
    def test_minimal_legacy_shape_still_validates(self):
        # The pre-F1 shape was {kind, expression}. The new shape MUST
        # remain backward-compatible.
        validator = validator_for_def("DeonticConstraint")
        doc = {"kind": "prohibition", "expression": "true"}
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"legacy shape must round-trip: {errors}"

    def test_full_legalruleml_shape_validates(self):
        validator = validator_for_def("DeonticConstraint")
        doc = {
            "id": "no-deny-without-notice",
            "kind": "prohibition",
            "actor": "caseworker",
            "expression": "outcome == 'denied' && notice == null",
            "condition": "outcome.adverse == true",
            "defeasible": False,
            "citationRefs": ["42 CFR § 431.211"],
            "x-legalruleml-iri": "urn:legalruleml:42cfr.431.211",
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"full LegalRuleML shape must validate: {errors}"

    def test_unknown_kind_rejected(self):
        validator = validator_for_def("DeonticConstraint")
        doc = {"kind": "wish", "expression": "true"}
        errors = list(validator.iter_errors(doc))
        assert errors, "kind enum MUST reject unknown values"

    def test_missing_expression_rejected(self):
        validator = validator_for_def("DeonticConstraint")
        doc = {"kind": "obligation"}
        errors = list(validator.iter_errors(doc))
        assert errors, "expression is required"

    def test_unknown_property_rejected_but_x_extensions_pass(self):
        # patternProperties ^x- allows extension keys; bare unknown keys fail.
        validator = validator_for_def("DeonticConstraint")
        bad = {"kind": "right", "expression": "true", "garbage": 1}
        good = {"kind": "right", "expression": "true", "x-vendor-extra": 1}
        assert list(validator.iter_errors(bad)), "unknown keys MUST be rejected"
        assert list(validator.iter_errors(good)) == [], "x- keys MUST be accepted"


# --- F1.5 FieldDeclaration extensions --------------------------------------

class TestFieldDeclarationExtensions:
    def test_minimal_field_validates(self):
        validator = validator_for_def("FieldDeclaration")
        doc = {"type": "string"}
        errors = list(validator.iter_errors(doc))
        assert errors == [], "baseline field declaration must validate"

    def test_canonical_term_ref_accepted(self):
        validator = validator_for_def("FieldDeclaration")
        doc = {"type": "number", "canonicalTermRef": "urn:wos:vocab:income:monthly"}
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"canonicalTermRef must be accepted: {errors}"

    def test_dpv_sensitivity_accepted(self):
        validator = validator_for_def("FieldDeclaration")
        doc = {
            "type": "string",
            "dpvSensitivity": "dpv:HealthData",
            "canonicalTermRef": "urn:wos:vocab:health:diagnosis",
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"dpvSensitivity + canonicalTermRef must validate: {errors}"
