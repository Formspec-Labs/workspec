"""Shared helpers for WOS schema regression tests.

Provides:
- WOS_SPEC_ROOT / SCHEMAS_ROOT constants for repo-relative path building.
- MARKER_TO_SCHEMA — the authoritative mapping from each `$wos*` document
  marker to its declaring schema file. When a new schema is added to
  `schemas/`, add its `$wos*` marker here and every fixture + spec code
  block carrying that marker becomes part of the regression suite
  automatically.
- A session-scoped `validators` fixture that compiles every schema once.
- `classify(doc)` — returns the `$wos*` marker key present in a document,
  or None if the document is unmarked (e.g. negative-fixture catalogs or
  non-WOS auxiliary data).
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
SCHEMAS_ROOT = WOS_SPEC_ROOT / "schemas"

# Marker → schema path. Post-ADR-0076 (D-6) the schema family collapsed to 6
# files — one author-time core ($wosWorkflow), two sidecars ($wosDelivery,
# $wosOntologyAlignment), two runtime artifacts ($wosCaseInstance,
# $wosProvenanceLog), one tooling ($wosTooling). Legacy per-block schemas have
# been deleted; their normative content is absorbed under embedded blocks of
# wos-workflow.schema.json. Spec example blocks and fixtures use the canonical
# six markers — no legacy compat shims.
MARKER_TO_SCHEMA: dict[str, str] = {
    "$wosWorkflow": "wos-workflow.schema.json",
    "$wosDelivery": "sidecars/wos-delivery.schema.json",
    "$wosOntologyAlignment": "sidecars/wos-ontology-alignment.schema.json",
    "$wosCaseInstance": "wos-case-instance.schema.json",
    "$wosProvenanceLog": "wos-provenance-log.schema.json",
    "$wosTooling": "wos-tooling.schema.json",
}


@pytest.fixture(scope="session")
def validators() -> dict[str, Draft202012Validator]:
    """Load and compile every classified schema once per test session."""
    compiled: dict[str, Draft202012Validator] = {}
    for marker, rel in MARKER_TO_SCHEMA.items():
        schema = json.loads((SCHEMAS_ROOT / rel).read_text())
        compiled[marker] = Draft202012Validator(schema)
    return compiled


def classify(doc: object) -> str | None:
    """Return the first `$wos*` marker key in a document, or None.

    Documents without a marker are auxiliary artifacts (negative-fixture
    catalogs, scenario transcripts) that the regression suite skips.
    """
    if not isinstance(doc, dict):
        return None
    for key in doc:
        if isinstance(key, str) and key.startswith("$wos"):
            return key
    return None
