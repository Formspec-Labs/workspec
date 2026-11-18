"""Record-lifecycle reason/statement classes copy Formspec's AccessClass constraint.

`reasonField` / `statementField` `accessControl.class` carry a Formspec
Access-Class Registry token (formspec extension-registry.md §3.2.3). Both slots
reference one WOS `$defs/AccessClass`, which copies the enum and pattern of
formspec common.schema.json `$defs/AccessClass`; this test pins the copy against
the sibling checkout when present. Branch examples are validated by
test_examples_validate.py, and their presence by wos-lint SCHEMA-DOC-001.
"""
from __future__ import annotations

import json

import pytest
from jsonschema import Draft202012Validator

from tests.schemas.conftest import WOS_SPEC_ROOT

WORKFLOW = json.loads((WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json").read_text("utf-8"))
FORMSPEC_COMMON = WOS_SPEC_ROOT.parent / "formspec" / "schemas" / "common.schema.json"


def _class_nodes(node, out):
    if isinstance(node, dict):
        props = node.get("properties", {})
        ac = props.get("accessControl", {})
        if isinstance(ac, dict) and "class" in ac.get("properties", {}):
            out.append(ac["properties"]["class"])
        for value in node.values():
            _class_nodes(value, out)
    elif isinstance(node, list):
        for value in node:
            _class_nodes(value, out)
    return out


CLASS_NODES = _class_nodes(WORKFLOW, [])


def test_both_lifecycle_class_slots_are_constrained():
    assert len(CLASS_NODES) == 2
    for node in CLASS_NODES:
        assert node == {"$ref": "#/$defs/AccessClass"}


@pytest.mark.parametrize("value", ["unclassified", "protected", "cui.privacy", "hipaa.phi"])
def test_registry_tokens_accepted(value):
    for node in CLASS_NODES:
        assert Draft202012Validator({"$defs": WORKFLOW["$defs"], **node}).is_valid(value)


@pytest.mark.parametrize("value", ["safe-address", "standard", "wos.case", "cui"])
def test_unregistered_tokens_refused(value):
    for node in CLASS_NODES:
        assert not Draft202012Validator({"$defs": WORKFLOW["$defs"], **node}).is_valid(value)


@pytest.mark.skipif(not FORMSPEC_COMMON.exists(), reason="formspec sibling not checked out")
def test_copy_matches_formspec_common_access_class():
    common = json.loads(FORMSPEC_COMMON.read_text("utf-8"))["$defs"]["AccessClass"]["anyOf"]
    # WOS documents each typed leaf. Compare every other branch keyword,
    # including the explicit string types consumed by code generators.
    def constraints(branches):
        return [
            {key: value for key, value in branch.items() if key not in {"description", "examples"}}
            for branch in branches
        ]

    assert constraints(WORKFLOW["$defs"]["AccessClass"]["anyOf"]) == constraints(common)
