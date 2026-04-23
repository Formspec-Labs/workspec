"""OverrideRecord and EvidenceReference $def shape regression tests.

Guards the contract documented in `wos-workflow-governance.schema.json`
§OverrideRecord (Governance §7.3): every override emits one OverrideRecord
with rationale + authorityVerification + supportingEvidence; every
supportingEvidence entry MUST be locatable (carry a `caseFieldPath` or
`uri`).

The descriptive prose alone is not enforcement — these tests confirm the
schema actually rejects malformed records.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
GOVERNANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "governance" / "wos-workflow-governance.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(GOVERNANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Build a Draft 2020-12 validator that resolves $defs from the parent schema."""
    target = schema["$defs"][def_name]
    composed = {
        "$schema": schema.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{schema.get('$id', 'urn:test')}#${def_name}",
        "$defs": schema["$defs"],
        **target,
    }
    return Draft202012Validator(composed)


class TestEvidenceReferenceLocatableContract:
    def test_empty_object_is_rejected(self, schema):
        v = _validator_for_def(schema, "EvidenceReference")
        errors = list(v.iter_errors({}))
        assert errors, "EvidenceReference {} must fail validation; the schema cannot enforce 'evidence is locatable' if it accepts an empty record"

    def test_summary_only_is_rejected(self, schema):
        """A summary without any locator (caseFieldPath or uri) is unauditable."""
        v = _validator_for_def(schema, "EvidenceReference")
        errors = list(v.iter_errors({"summary": "Updated medical assessment"}))
        assert errors, "EvidenceReference with only `summary` and no locator must fail"

    def test_kind_only_is_rejected(self, schema):
        """`kind` alone is metadata about the locator type — but no locator is provided."""
        v = _validator_for_def(schema, "EvidenceReference")
        errors = list(v.iter_errors({"kind": "document"}))
        assert errors, "EvidenceReference with only `kind` and no locator must fail"

    def test_case_field_locator_is_accepted(self, schema):
        v = _validator_for_def(schema, "EvidenceReference")
        valid = {
            "kind": "caseField",
            "caseFieldPath": "/medical/recentEvidence/2026-03-14",
            "summary": "New medical assessment",
        }
        errors = list(v.iter_errors(valid))
        assert errors == [], f"valid case-field reference rejected: {errors}"

    def test_uri_locator_is_accepted(self, schema):
        v = _validator_for_def(schema, "EvidenceReference")
        valid = {
            "kind": "document",
            "uri": "https://agency.gov/cases/12345/attachments/MED-007.pdf",
            "summary": "Treating physician statement.",
        }
        errors = list(v.iter_errors(valid))
        assert errors == [], f"valid URI reference rejected: {errors}"

    def test_both_locators_is_accepted(self, schema):
        """Belt-and-suspenders: both case-field and URI present should validate."""
        v = _validator_for_def(schema, "EvidenceReference")
        valid = {
            "kind": "document",
            "caseFieldPath": "/uploads/MED-007",
            "uri": "https://agency.gov/cases/12345/attachments/MED-007.pdf",
        }
        errors = list(v.iter_errors(valid))
        assert errors == [], f"reference with both locators rejected: {errors}"


class TestOverrideRecordRequiredFields:
    """OverrideRecord requires rationale + authorityVerification + supportingEvidence."""

    def _minimal_valid_record(self) -> dict:
        return {
            "id": "sba-poc_gov_01jqrpd32jf8xtx9qxkkv3rqsd",
            "caseId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
            "rationale": {"summary": "New evidence invalidates prior denial."},
            "authorityVerification": {
                "actorId": "reviewer-alice",
                "method": "roleAssignment",
                "verifiedAt": "2026-03-14T15:42:00Z",
            },
            "supportingEvidence": [
                {
                    "kind": "caseField",
                    "caseFieldPath": "/medical/recentEvidence",
                    "summary": "Updated medical evidence.",
                }
            ],
        }

    def test_minimal_record_validates(self, schema):
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(self._minimal_valid_record()))
        assert errors == [], f"minimal valid OverrideRecord rejected: {errors}"

    @pytest.mark.parametrize(
        "missing_field", ["id", "caseId", "rationale", "authorityVerification", "supportingEvidence"]
    )
    def test_missing_required_field_rejected(self, schema, missing_field):
        record = self._minimal_valid_record()
        del record[missing_field]
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(record))
        assert errors, f"OverrideRecord missing `{missing_field}` must fail"

    def test_supporting_evidence_with_empty_reference_rejected(self, schema):
        """Supporting evidence array elements must each be locatable (Finding 1)."""
        record = self._minimal_valid_record()
        record["supportingEvidence"] = [{}]
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(record))
        assert errors, (
            "OverrideRecord with `supportingEvidence: [{}]` must fail — "
            "an empty reference provides no auditable provenance"
        )

    def test_authority_verification_method_enum_enforced(self, schema):
        record = self._minimal_valid_record()
        record["authorityVerification"]["method"] = "vibesBased"
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(record))
        assert errors, "authorityVerification.method must reject unknown values"

    def test_override_record_rejects_non_gov_id_prefix(self, schema):
        record = self._minimal_valid_record()
        record["id"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(record))
        assert errors, "OverrideRecord.id must use the reserved `gov` family prefix"

    def test_override_record_rejects_non_case_case_id_prefix(self, schema):
        record = self._minimal_valid_record()
        record["caseId"] = "sba-poc_gov_01jqrpd32jf8xtx9qxkkv3rqsd"
        v = _validator_for_def(schema, "OverrideRecord")
        errors = list(v.iter_errors(record))
        assert errors, "OverrideRecord.caseId must use the reserved `case` family prefix"
