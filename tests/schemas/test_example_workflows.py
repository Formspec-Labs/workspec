"""Every maintained `examples/` document validates against its classified schema.

The examples define the product-tier ladder (ADR 0076 D-9) that authors and
authoring agents copy from, so a drifted example is a factory for invalid
workflows. Each `*.workflow.json` must carry `$wosWorkflow`, and every
`$wos*`-marked example (workflows and their delivery sidecars) validates.
Retained historical evidence is exempt only by name: its bytes are pinned, and
the targeted test named beside each entry asserts its precise refusal.
"""
from __future__ import annotations

import json

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify

EXAMPLES_ROOT = WOS_SPEC_ROOT / "examples"
HISTORICAL_EXAMPLES = frozenset(
    {
        "d1-passport.workflow.json",  # test_d1_workflow.py
        "d1-passport-complete-deferred.workflow.json",  # test_deferred_callback_shape.py
    }
)
EXAMPLE_MARKERS = {
    path: classify(json.loads(path.read_text())) for path in sorted(EXAMPLES_ROOT.glob("*.json"))
}
MAINTAINED_EXAMPLES = [
    path
    for path, marker in EXAMPLE_MARKERS.items()
    if marker is not None and path.name not in HISTORICAL_EXAMPLES
]


def test_example_inventory_is_explicit():
    names = {path.name for path in EXAMPLE_MARKERS}
    assert HISTORICAL_EXAMPLES <= names
    unmarked = [
        path.name
        for path, marker in EXAMPLE_MARKERS.items()
        if path.name.endswith(".workflow.json") and marker != "$wosWorkflow"
    ]
    assert not unmarked, f"workflow examples without $wosWorkflow: {unmarked}"


@pytest.mark.parametrize("path", MAINTAINED_EXAMPLES, ids=[p.name for p in MAINTAINED_EXAMPLES])
def test_example_validates_against_its_schema(path, validators):
    errors = sorted(
        validators[EXAMPLE_MARKERS[path]].iter_errors(json.loads(path.read_text())),
        key=lambda error: list(error.absolute_path),
    )
    assert not errors, "\n".join(
        f"{path.name} {list(error.absolute_path)}: {error.message}" for error in errors
    )
