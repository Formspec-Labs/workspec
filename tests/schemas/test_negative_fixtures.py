"""Negative fixtures MUST fail schema validation.

`fixtures/kernel/invalid-documents.json` catalogs intentionally-broken
WOS Kernel documents. Each entry pairs a minimal invalid document with
the human-readable error a conformant structural processor SHOULD
produce. This suite guards against a schema accidentally becoming too
permissive: if an invalid document starts validating, the test fails
and forces the author to tighten the schema (or move the case to the
semantic-only linter).

Some catalog entries describe violations that pure JSON Schema cannot
express (e.g. "final states must not have outgoing transitions" — that
requires cross-property reasoning the Rust lint pass owns). Those are
marked xfail with a reason so the file remains a single source of
truth without silently losing coverage.
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
    # Final states MUST NOT have outgoing transitions — requires joining
    # `lifecycle.states[*].final` with `lifecycle.transitions[*].from`.
    "final-state-with-transitions",
}


def _load_cases():
    catalog = json.loads(NEGATIVE_CATALOG.read_text())
    return [(case["id"], case) for case in catalog["invalidDocuments"]]


NEGATIVE_CASES = _load_cases()


@pytest.mark.parametrize(
    "case_id,case",
    NEGATIVE_CASES,
    ids=[cid for cid, _ in NEGATIVE_CASES],
)
def test_invalid_kernel_document_is_rejected(case_id, case, validators):
    if case_id in SEMANTIC_ONLY_CASES:
        pytest.xfail(
            f"{case_id}: violation requires semantic checks beyond JSON Schema "
            f"(expected error: {case.get('expectedError', 'unspecified')})"
        )
    errors = list(validators["$wosWorkflow"].iter_errors(case["document"]))
    assert errors, (
        f"{case_id} was expected to fail validation but passed. "
        f"Expected error: {case.get('expectedError', 'unspecified')}"
    )
