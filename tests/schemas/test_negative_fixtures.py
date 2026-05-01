"""Negative fixtures MUST fail schema validation.

`fixtures/kernel/invalid-documents.json` catalogs intentionally-broken
WOS workflow documents (`$wosWorkflow` envelope). Each entry pairs a minimal invalid document with
the human-readable error a conformant structural processor SHOULD
produce. This suite guards against a schema accidentally becoming too
permissive: if an invalid document starts validating, the test fails
and forces the author to tighten the schema (or move the case to the
semantic-only linter).

Some catalog entries describe violations that pure JSON Schema cannot
express (e.g. "final states must not have outgoing transitions" — that
requires cross-property reasoning the Rust lint pass owns). Those cases
are strict xfails: if schema coverage later catches one, XPASS fails the
suite and forces this classification to be updated.
"""
from __future__ import annotations

import json

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT

NEGATIVE_CATALOG = WOS_SPEC_ROOT / "fixtures" / "kernel" / "invalid-documents.json"

# Cases whose violation is NOT expressible in JSON Schema alone; the Rust
# semantic lint pass is the authoritative check. Kept here as xfail so
# the catalog stays the single source of truth, and so we notice
# immediately if schema expressivity changes (a surprise pass flips this
# to XPASS and fails the build).
SEMANTIC_ONLY_CASES: set[str] = {
    # Final states MUST NOT have outgoing transitions — cross-property rule
    # not expressed as a single JSON Schema constraint on this schema.
    "final-state-with-transitions",
}


def _load_cases():
    catalog = json.loads(NEGATIVE_CATALOG.read_text())
    return [(case["id"], case) for case in catalog["invalidDocuments"]]


NEGATIVE_CASES = _load_cases()


def _semantic_only_reason(case_id: str, case: dict) -> str:
    return (
        f"{case_id}: violation requires semantic checks beyond JSON Schema "
        f"(expected error: {case.get('expectedError', 'unspecified')})"
    )


def _case_params():
    params = []
    for case_id, case in NEGATIVE_CASES:
        marks = []
        if case_id in SEMANTIC_ONLY_CASES:
            marks.append(
                pytest.mark.xfail(
                    reason=_semantic_only_reason(case_id, case),
                    strict=True,
                )
            )
        params.append(pytest.param(case_id, case, id=case_id, marks=marks))
    return params


@pytest.mark.parametrize(
    "case_id,case",
    _case_params(),
)
def test_invalid_workflow_document_is_rejected(case_id, case, validators):
    errors = list(validators["$wosWorkflow"].iter_errors(case["document"]))
    assert errors, (
        f"{case_id} was expected to fail validation but passed. "
        f"Expected error: {case.get('expectedError', 'unspecified')}"
    )
