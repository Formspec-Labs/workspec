"""Validate Studio (Authoring) example artifacts against their Stage-3 schemas.

Walks `studio/examples/**/*.json`, classifies each document by its
top-level `$wos*` marker, and validates against the schema registered in
`conftest.MARKER_TO_SCHEMA`. Documents without a Studio marker are skipped
(auxiliary artifacts, source documents in markdown-as-JSON, etc.).

The vertical-slice example artifacts in `studio/examples/` are the
truth-of-record for what Studio authors produce. If a schema rejects an
example, the schema is wrong (not the example) — UNLESS the example is itself
known-to-be-stale from v3/v4/bridge-inference churn, in which case the
example is updated.

Wave 3 (review remediation, 2026-05-02) added two coverage paths beyond
the marker-only validation:

1. **Collection-form whole-document validation.** Files like
   `policy-objects/wos-projecting-kinds.json` carry `policyObjects[]` under
   a workspaceId wrapper without a top-level marker. After the Wave-1
   `oneOf` addition to `wos-studio-policy-object.schema.json`, these
   documents validate as whole documents against the collection branch.
   The `_collection_form_schemas` table maps wrapper-shape detection (a
   list-typed field name) to the schema that carries the matching `oneOf`
   branch.
2. **Inner-object expansion.** For collection wrappers whose schemas do
   NOT yet carry collection-form `oneOf` (bindings, identity, mappings,
   sources), each child is validated standalone with its `$wosStudio*`
   marker injected.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator

from .conftest import (  # noqa: TID252 -- relative import inside tests pkg
    MARKER_TO_SCHEMA,
    WOS_SPEC_ROOT,
    classify,
)

EXAMPLES_ROOT = WOS_SPEC_ROOT / "studio" / "examples"

# Collection-form wrappers whose schemas accept the wrapper shape directly
# (via Wave-1 `oneOf` additions). Maps the wrapper-detection field name to
# the schema marker that validates the wrapper.
_COLLECTION_WHOLE_DOC_SCHEMAS: dict[str, str] = {
    "policyObjects": "$wosStudioPolicyObject",
}

# Collection-form wrappers without whole-doc schema support; validate each
# child standalone with the marker injected.
_INNER_EXPAND_SCHEMAS: dict[str, str] = {
    "bindings": "$wosStudioBinding",
    "subjects": "$wosStudioIdentitySubject",
    "sourceVersions": "$wosStudioSource",
    "mappings": "$wosStudioMapping",
}


def _example_files() -> list[Path]:
    """Every JSON file under studio/examples/."""
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
    """Each studio/examples/ artifact carrying a $wosStudio* marker MUST validate.

    Documents without a Studio marker but matching a collection-form wrapper
    pattern are validated via _COLLECTION_WHOLE_DOC_SCHEMAS or
    _INNER_EXPAND_SCHEMAS.
    """
    doc = json.loads(fixture_path.read_text())
    marker = classify(doc)

    # Marker-bearing documents
    if marker is not None:
        if marker not in MARKER_TO_SCHEMA:
            pytest.skip(f"{fixture_path.name}: marker {marker!r} not in MARKER_TO_SCHEMA")
        if not marker.startswith("$wosStudio"):
            pytest.skip(f"{fixture_path.name}: non-Studio marker {marker!r}")
        validator = validators[marker]
        errors = list(validator.iter_errors(doc))
        assert not errors, (
            f"{fixture_path.relative_to(EXAMPLES_ROOT)}: "
            f"{errors[0].message} at {list(errors[0].absolute_path)}"
        )
        return

    # Collection-form whole-document validation
    if isinstance(doc, dict):
        for field, target_marker in _COLLECTION_WHOLE_DOC_SCHEMAS.items():
            if field in doc and isinstance(doc[field], list):
                validator = validators[target_marker]
                errors = list(validator.iter_errors(doc))
                assert not errors, (
                    f"{fixture_path.relative_to(EXAMPLES_ROOT)} (collection-form "
                    f"{field}): {errors[0].message} at "
                    f"{list(errors[0].absolute_path)}"
                )
                return

        # Inner-object expansion
        for field, target_marker in _INNER_EXPAND_SCHEMAS.items():
            if field in doc and isinstance(doc[field], list):
                validator = validators[target_marker]
                for i, child in enumerate(doc[field]):
                    if not isinstance(child, dict):
                        continue
                    # Inject marker if absent (to satisfy schema's required field)
                    enriched = (
                        child if target_marker in child
                        else {target_marker: "1.0", **child}
                    )
                    errors = list(validator.iter_errors(enriched))
                    assert not errors, (
                        f"{fixture_path.relative_to(EXAMPLES_ROOT)}[{field}][{i}]: "
                        f"{errors[0].message} at "
                        f"{list(errors[0].absolute_path)}"
                    )
                return

    # Truly auxiliary — no marker, no recognized collection field
    pytest.skip(f"{fixture_path.name}: no $wos* marker, no collection wrapper")


def test_studio_examples_directory_exists() -> None:
    """Sanity check: the studio/examples/ directory exists."""
    assert EXAMPLES_ROOT.exists(), (
        f"Expected studio/examples/ at {EXAMPLES_ROOT}; "
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
        "No example file under studio/examples/ carries a "
        "$wosStudio* marker. Add markers to at least workflow-intent.json, "
        "scenarios/scenarios.json, and approval-package.json."
    )


def test_studio_every_schema_has_an_example() -> None:
    """Wave-3 coverage gate: every Studio schema (excluding marker-less common)
    MUST have at least one example artifact validating against it.

    Closes Agent 2 finding M5 ('4 of 15 schemas have zero example coverage')
    and ensures the studio/examples/ directory remains a complete
    representation of the Stage-3 schema surface."""
    studio_markers: set[str] = {
        m for m in MARKER_TO_SCHEMA if m.startswith("$wosStudio")
    }
    seen_markers: set[str] = set()
    for f in _example_files():
        try:
            doc = json.loads(f.read_text())
        except json.JSONDecodeError:
            continue
        marker = classify(doc)
        if marker and marker.startswith("$wosStudio"):
            seen_markers.add(marker)
        elif isinstance(doc, dict):
            # Inner-expansion contributes coverage for the inner marker.
            for field, target_marker in _INNER_EXPAND_SCHEMAS.items():
                if field in doc and isinstance(doc[field], list) and doc[field]:
                    seen_markers.add(target_marker)
            for field, target_marker in _COLLECTION_WHOLE_DOC_SCHEMAS.items():
                if field in doc and isinstance(doc[field], list) and doc[field]:
                    seen_markers.add(target_marker)
    missing = studio_markers - seen_markers
    assert not missing, (
        f"{len(missing)} Studio schema(s) without example coverage in "
        f"studio/examples/: {sorted(missing)}. "
        f"Add example artifact(s) carrying these marker(s)."
    )
