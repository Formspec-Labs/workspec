"""Facts-tier case-file snapshot schema regression tests.

Validates the runtime provenance-log shape ``wos-provenance-log.schema.json``.
Per ADR 0076 step 5, ``FactsTierRecord`` and ``CaseFileSnapshot`` were
promoted into ``wos-workflow.schema.json``'s ``$defs``; the runtime log
``$ref``s them across schemas. The cross-schema registry in
``conftest`` resolves both the in-document ``$ref`` and the bare ``$def``
lookups used here, so kernel-side promotion is invisible to the tests.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from .conftest import _REGISTRY, validator_for_def

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(PROVENANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Registry-aware ``$def`` lookup. ``schema`` is ignored; def lookup
    spans every classified schema via ``conftest._REGISTRY``.
    """
    return validator_for_def(def_name)


def _document_validator(schema: dict) -> Draft202012Validator:
    """Validate the top-level provenance log document. This is the canonical
    shape the runtime emits, so it -- not the bare ``FactsTierRecord`` $def --
    is what production exports flow through. Registry-aware so cross-schema
    ``$ref``s into ``wos-workflow.schema.json`` resolve.
    """
    from jsonschema import FormatChecker
    return Draft202012Validator(
        schema,
        registry=_REGISTRY,
        format_checker=FormatChecker(),
    )


def _snapshot() -> dict:
    return {
        "value": {"eligible": True, "income": 17500},
        "jcsCanonical": '{"eligible":true,"income":17500}',
        "sha256": "b19f000c0cd497b52c4a78e50641651e4b1e96931a1b1558984d69e722f73f5e",
    }


def _facts_record(record_kind: str, **extra) -> dict:
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordKind": record_kind,
        "timestamp": "2026-04-19T12:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


def test_determination_transition_without_snapshot_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["determination"],
    )

    errors = list(validator.iter_errors(record))

    assert errors, "determination-tagged StateTransition must require caseFileSnapshot"


def test_determination_transition_with_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["determination"],
        caseFileSnapshot=_snapshot(),
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], f"valid determination snapshot rejected: {errors}"


def test_non_determination_transition_without_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["review"],
    )

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
        "$wosProvenanceLog": "1.0",
        "provenanceLog": [
            {
                **_facts_record("stateTransition"),
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
        "$wosProvenanceLog": "1.0",
        "provenanceLog": [
            {
                **_facts_record("stateTransition"),
                "transitionTags": ["determination"],
                "caseFileSnapshot": _snapshot(),
            },
            {
                **_facts_record(
                    "caseStateMutation",
                    id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
                ),
                "transitionTags": [],
            },
        ]
    }

    errors = list(validator.iter_errors(document))

    assert errors == [], (
        f"well-formed provenance log rejected by FactsTierRecord: {errors}"
    )
