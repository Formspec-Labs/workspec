"""Every positive WOS fixture validates against its classified schema.

Positive fixtures are JSON files under `fixtures/` with a top-level
`$wos*` marker. The marker identifies which schema the document claims
to conform to; this test parametrizes over only those files.

Unmarked fixture JSON is allowed only when explicitly classified as an
auxiliary artifact: negative-case catalogs, scenario transcripts, or
standalone examples owned by another targeted test.
"""
from __future__ import annotations

import json

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify, validator_for_def

FIXTURES_ROOT = WOS_SPEC_ROOT / "fixtures"
ALL_FIXTURES = sorted(p for p in FIXTURES_ROOT.rglob("*.json") if p.is_file())
AUXILIARY_FIXTURES = frozenset(
    {
        "conformance/expected-traces/ai-auto-001-escalation-expiry-revocation.json",
        "conformance/expected-traces/ai-auto-002-drift-alert-demotion.json",
        "conformance/expected-traces/g-030-hold-resume.json",
        "conformance/expected-traces/k-001-negative-final-transitions.json",
        "conformance/expected-traces/k-011-determinism.json",
        "conformance/expected-traces/k-011-parallel-join.json",
        "conformance/expected-traces/k-020-provenance-completeness.json",
        "conformance/expected-traces/k-033-document-order.json",
        "conformance/expected-traces/k-046-timer-provenance.json",
        "kernel/custody-hook/provenance-state-transition/record.json",
        "kernel/invalid-documents.json",
        "kernel/purchase-order-provenance.json",
    }
)
FIXTURE_MARKERS = {
    path: classify(json.loads(path.read_text()))
    for path in ALL_FIXTURES
}
SCHEMA_FIXTURES = [
    path
    for path, marker in FIXTURE_MARKERS.items()
    if marker is not None
]
UNMARKED_FIXTURES = frozenset(
    path.relative_to(FIXTURES_ROOT).as_posix()
    for path, marker in FIXTURE_MARKERS.items()
    if marker is None
)


def test_unmarked_fixture_inventory_is_explicit():
    assert UNMARKED_FIXTURES == AUXILIARY_FIXTURES


@pytest.mark.parametrize(
    "fixture_path",
    SCHEMA_FIXTURES,
    ids=[p.relative_to(FIXTURES_ROOT).as_posix() for p in SCHEMA_FIXTURES],
)
def test_fixture_validates_against_its_schema(fixture_path, validators):
    doc = json.loads(fixture_path.read_text())
    marker = FIXTURE_MARKERS[fixture_path]
    # Standalone signature profiles use `$wosSignatureProfile` as a document
    # tag; the normative JSON Schema is `wos-workflow` `$defs.Signature`, which
    # does not list that tag (additionalProperties: false). Strip the tag and
    # validate the payload shape.
    if marker == "$wosSignatureProfile":
        # Standalone profile transport may carry legacy envelope keys that the
        # embedded Signature $def explicitly retired (ADR 0063 §2.1).
        _strip = frozenset(
            {
                "$wosSignatureProfile",
                "targetWorkflow",
                "title",
                "version",
            }
        )
        body = {k: v for k, v in doc.items() if k not in _strip}
        errors = list(validator_for_def("Signature").iter_errors(body))
    elif marker not in validators:
        pytest.fail(
            f"{fixture_path.relative_to(FIXTURES_ROOT)}: marker {marker!r} is "
            "not in MARKER_TO_SCHEMA — add it to conftest.py"
        )
    else:
        errors = list(validators[marker].iter_errors(doc))
    assert not errors, (
        f"{fixture_path.relative_to(FIXTURES_ROOT)}: "
        f"{errors[0].message} at {list(errors[0].absolute_path)}"
    )
