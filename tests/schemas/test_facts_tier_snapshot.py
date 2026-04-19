"""Facts-tier case-file snapshot schema regression tests.

Validates the split-out provenance log schema (``wos-provenance-record.schema.json``).
The schema is a real document type -- kernel documents do not embed the provenance
log, so splitting it into its own file means the ``FactsTierRecord`` / ``CaseFileSnapshot``
``$def`` shapes are no longer orphaned: they validate every provenance export.
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


def _document_validator(schema: dict) -> Draft202012Validator:
    """Validate the top-level provenance log document. This is the canonical
    shape the runtime emits, so it -- not the bare ``FactsTierRecord`` $def --
    is what production exports flow through.
    """
    return Draft202012Validator(schema)


def _snapshot() -> dict:
    return {
        "value": {"eligible": True, "income": 17500},
        "jcsCanonical": '{"eligible":true,"income":17500}',
        "sha256": "b19f000c0cd497b52c4a78e50641651e4b1e96931a1b1558984d69e722f73f5e",
    }


def test_determination_transition_without_snapshot_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
        "transitionTags": ["determination"],
    }

    errors = list(validator.iter_errors(record))

    assert errors, "determination-tagged StateTransition must require caseFileSnapshot"


def test_determination_transition_with_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
        "transitionTags": ["determination"],
        "caseFileSnapshot": _snapshot(),
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], f"valid determination snapshot rejected: {errors}"


def test_non_determination_transition_without_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = {
        "recordKind": "stateTransition",
        "transitionTags": ["review"],
    }

    errors = list(validator.iter_errors(record))

    assert errors == [], f"non-determination snapshot should remain optional: {errors}"


def test_snapshot_rejects_malformed_sha256(schema):
    validator = _validator_for_def(schema, "CaseFileSnapshot")
    snapshot = _snapshot()
    snapshot["sha256"] = "not-a-sha"

    errors = list(validator.iter_errors(snapshot))

    assert errors, "caseFileSnapshot.sha256 must be a lowercase 64-character hex digest"


def test_full_document_rejects_determination_record_missing_snapshot(schema):
    """Full-document validation must reject a determination-tagged
    stateTransition that lacks ``caseFileSnapshot``. This is the structural
    bite Finding 2 asks for: the $def must produce errors on realistic
    provenance log exports, not only on the bare $def.
    """
    validator = _document_validator(schema)
    document = {
        "provenanceLog": [
            {
                "recordKind": "stateTransition",
                "transitionTags": ["determination"],
            }
        ]
    }

    errors = list(validator.iter_errors(document))

    assert errors, (
        "Full-document validation must reject a determination-tagged "
        "stateTransition without caseFileSnapshot"
    )


def test_full_document_accepts_determination_record_with_snapshot(schema):
    validator = _document_validator(schema)
    document = {
        "provenanceLog": [
            {
                "recordKind": "stateTransition",
                "transitionTags": ["determination"],
                "caseFileSnapshot": _snapshot(),
            },
            {
                "recordKind": "caseStateMutation",
                "transitionTags": [],
            },
        ]
    }

    errors = list(validator.iter_errors(document))

    assert errors == [], (
        f"well-formed provenance log rejected by FactsTierRecord: {errors}"
    )
