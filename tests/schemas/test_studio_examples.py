"""Validate Studio (Authoring) example artifacts against their Stage-3 schemas.

Walks `studio-authoring/examples/**/*.json`, classifies each document by its
top-level `$wos*` marker, and validates against the schema registered in
`conftest.MARKER_TO_SCHEMA`. Documents without a Studio marker are skipped
(auxiliary artifacts, source documents in markdown-as-JSON, etc.).

The vertical-slice example artifacts in `studio-authoring/examples/` are the
truth-of-record for what Studio authors produce. If a schema rejects an
example, the schema is wrong (not the example) — UNLESS the example is itself
known-to-be-stale from v3/v4/bridge-inference churn, in which case the
example is updated.

Pre-stage corrections to CM §6.1 (2026-05-01) softened the `$ref` claim;
collection-form policy-objects files (with `policyObjects[]` wrapper) are
NOT validated here because they aren't the canonical Studio document shape;
each PolicyObject inside is a separate `$wosStudioPolicyObject` document
when serialized standalone.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker

from .conftest import (  # noqa: TID252 -- relative import inside tests pkg
    MARKER_TO_SCHEMA,
    WOS_SPEC_ROOT,
    classify,
)

EXAMPLES_ROOT = WOS_SPEC_ROOT / "studio-authoring" / "examples"


def _example_files() -> list[Path]:
    """Every JSON file under studio-authoring/examples/."""
    if not EXAMPLES_ROOT.exists():
        return []
    return sorted(EXAMPLES_ROOT.rglob("*.json"))


@pytest.mark.parametrize(
    "fixture_path",
    _example_files(),
    ids=lambda p: str(p.relative_to(EXAMPLES_ROOT)),
)
def test_studio_example_validates_by_marker(
    fixture_path: Path,
    validators: dict[str, Draft202012Validator],
) -> None:
    """Each studio-authoring example carrying a $wosStudio* marker MUST validate.

    Skips:
    - Documents without any $wos* marker (auxiliary artifacts).
    - Documents whose marker is registered but is a non-Studio marker (e.g.,
      `$wosWorkflow` for the compiled artifact — validated by the existing
      test_fixture_validity.py harness against the parent schema).
    """
    doc = json.loads(fixture_path.read_text())
    marker = classify(doc)
    if marker is None:
        pytest.skip(f"{fixture_path.name}: no $wos* marker (auxiliary artifact)")
    if marker not in MARKER_TO_SCHEMA:
        pytest.skip(f"{fixture_path.name}: marker {marker!r} not in MARKER_TO_SCHEMA")
    if not marker.startswith("$wosStudio"):
        # Non-Studio markers (parent $wosWorkflow, $wosTooling) are validated
        # by the parent test harness; skip here.
        pytest.skip(f"{fixture_path.name}: non-Studio marker {marker!r}")
    validator = validators[marker]
    errors = list(validator.iter_errors(doc))
    assert not errors, (
        f"{fixture_path.relative_to(EXAMPLES_ROOT)}: "
        f"{errors[0].message} at {list(errors[0].absolute_path)}"
    )


def test_studio_examples_directory_exists() -> None:
    """Sanity check: the studio-authoring/examples/ directory exists."""
    assert EXAMPLES_ROOT.exists(), (
        f"Expected studio-authoring/examples/ at {EXAMPLES_ROOT}; "
        f"vertical-slice examples drive the Studio Stage-3 validation."
    )


def test_studio_at_least_one_example_carries_studio_marker() -> None:
    """At least one example file MUST carry a $wosStudio* marker so the
    parametrized validation above is non-vacuous."""
    studio_marked = []
    for f in _example_files():
        try:
            doc = json.loads(f.read_text())
        except json.JSONDecodeError:
            continue
        marker = classify(doc)
        if marker and marker.startswith("$wosStudio"):
            studio_marked.append(f.relative_to(EXAMPLES_ROOT))
    assert studio_marked, (
        "No example file under studio-authoring/examples/ carries a "
        "$wosStudio* marker. Add markers to at least workflow-intent.json, "
        "scenarios/scenarios.json, and approval-package.json."
    )
