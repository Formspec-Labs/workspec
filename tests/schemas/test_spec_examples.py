"""Every fenced `json` code block in specs/ validates against its schema.

Extracts every ```json fenced block from canonical `.md` spec files
(skipping generated `*.llm.md` digests), tries to parse each, and —
for blocks whose JSON carries a `$wos*` marker — validates the
document against the declaring schema. Non-JSON-parseable blocks and
markerless JSON blocks are treated as pseudocode/fragments and skipped;
they are the spec author's prose, not normative data.

Ported from the parent Formspec conformance suite's
`tests/conformance/spec/test_spec_examples.py` pattern.
"""
from __future__ import annotations

import json
import re

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify

SPECS_ROOT = WOS_SPEC_ROOT / "specs"
# Skip generated `*.llm.md` digests — they are rebuilt from the canonical
# `*.md` sources and including them would double-count every example.
SPEC_FILES = sorted(
    p for p in SPECS_ROOT.rglob("*.md") if not p.name.endswith(".llm.md")
)

JSON_BLOCK = re.compile(r"```json\s*\n(.*?)\n```", re.DOTALL)


def _is_pseudocode(doc: dict) -> bool:
    """Illustrative spec examples sometimes use a literal ``"..."`` key or
    value as an "and so on" placeholder. Those blocks parse as JSON but
    are not normative documents; treat them as prose.
    """
    if "..." in doc:
        return True
    for value in doc.values():
        if value == "..." or value == ["..."]:
            return True
    return False


def _extract_cases():
    cases: list[tuple[str, int, str, dict]] = []
    for spec in SPEC_FILES:
        text = spec.read_text()
        rel = spec.relative_to(SPECS_ROOT).as_posix()
        for idx, match in enumerate(JSON_BLOCK.finditer(text)):
            block = match.group(1)
            try:
                doc = json.loads(block)
            except json.JSONDecodeError:
                # Prose fences often contain ellipses, comments, or
                # deliberately-partial snippets. Not an error.
                continue
            marker = classify(doc)
            if marker is None:
                continue
            if _is_pseudocode(doc):
                continue
            cases.append((rel, idx, marker, doc))
    return cases


SPEC_CASES = _extract_cases()


@pytest.mark.parametrize(
    "spec_path,block_index,marker,doc",
    SPEC_CASES,
    ids=[f"{rel}#block{i}:{marker}" for rel, i, marker, _ in SPEC_CASES],
)
def test_spec_example_validates(spec_path, block_index, marker, doc, validators):
    if marker not in validators:
        pytest.fail(
            f"{spec_path} block #{block_index}: marker {marker!r} has no "
            "entry in MARKER_TO_SCHEMA — add it to conftest.py"
        )
    errors = list(validators[marker].iter_errors(doc))
    assert not errors, (
        f"{spec_path} block #{block_index} ({marker}): "
        f"{errors[0].message} at {list(errors[0].absolute_path)}"
    )
