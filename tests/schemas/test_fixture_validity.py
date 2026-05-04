"""Every positive WOS fixture validates against its classified schema.

Positive fixtures are JSON files under `fixtures/` whose top-level
contains a `$wos*` marker. The marker identifies which schema the
document claims to conform to; this test parametrizes over every such
file and asserts validation succeeds.

Fixtures without a `$wos*` marker are auxiliary artifacts (negative-case
catalogs, scenario transcripts) and are skipped — the
`test_negative_fixtures.py` suite covers the catalog in
`fixtures/kernel/invalid-documents.json` explicitly.
"""
from __future__ import annotations

import json

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify, validator_for_def

FIXTURES_ROOT = WOS_SPEC_ROOT / "fixtures"
ALL_FIXTURES = sorted(p for p in FIXTURES_ROOT.rglob("*.json") if p.is_file())


@pytest.mark.parametrize(
    "fixture_path",
    ALL_FIXTURES,
    ids=[p.relative_to(FIXTURES_ROOT).as_posix() for p in ALL_FIXTURES],
)
def test_fixture_validates_against_its_schema(fixture_path, validators):
    doc = json.loads(fixture_path.read_text())
    marker = classify(doc)
    if marker is None:
        pytest.skip(f"no $wos* marker in {fixture_path.name}")
    # Standalone signature profiles use `$wosSignatureProfile` as a document
    # tag; the normative JSON Schema is `wos-workflow` `$defs.Signature`, which
    # does not list that tag (additionalProperties: false). Strip the tag and
    # validate the payload shape.
    if marker == "$wosSignatureProfile":
        body = {k: v for k, v in doc.items() if k != "$wosSignatureProfile"}
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
