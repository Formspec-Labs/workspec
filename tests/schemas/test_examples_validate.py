"""Validate every `examples` entry against the schema fragment that owns it.

ADR 0063 §2 makes schema `examples` load-bearing for the LLM authoring loop:
authors and authoring agents copy from examples, so a drifted example becomes
a factory for invalid documents. This test walks every classified schema,
collects every `examples` array under any sub-schema, and validates each entry
against that sub-schema. Failures list the (schema, json-pointer, index) tuple
so authors can fix the offending example.

The test uses the session-wide referencing registry from `conftest.py` so
examples buried under nested `$ref`s resolve cleanly.
"""
from __future__ import annotations

from typing import Any, Iterator

import pytest
from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry


_MAP_SUBSCHEMAS = {"properties", "patternProperties", "$defs", "definitions", "dependentSchemas"}
_DIRECT_SUBSCHEMAS = {
    "items", "additionalProperties", "if", "then", "else", "not", "propertyNames", "contains",
}
_LIST_SUBSCHEMAS = {"oneOf", "anyOf", "allOf", "prefixItems", "items"}


def _validator_for_fragment(
    marker: str,
    pointer: str,
    sub_schema: dict[str, Any],
    registry: Registry,
    schemas_by_marker: dict[str, dict[str, Any]],
) -> Draft202012Validator:
    """Build a validator for a sub-schema that may use ``#/$defs/...`` refs.

    Fragment nodes under ``properties`` / ``$defs`` / ``items`` often use
    ``$ref`` or nested ``items.$ref`` to ``#/$defs/X``. A bare fragment root
    cannot resolve those pointers (``#`` is the fragment, which has no
    ``$defs``). Re-anchor every **non-root** sub-schema by merging the host
    resource's ``$defs`` plus a synthetic ``$id`` so internal pointers resolve
    the same way as validating against the full schema document.
    """
    host = schemas_by_marker.get(marker)
    definitions = {key: host[key] for key in ("$defs", "definitions") if host and key in host}
    if (
        host
        and pointer
        and definitions
        and id(sub_schema) != id(host)
    ):
        safe = pointer.replace("/", "_").replace("~", "_") or "sub"
        composed: dict[str, Any] = {
            "$schema": host.get(
                "$schema", "https://json-schema.org/draft/2020-12/schema"
            ),
            "$id": f"{host.get('$id', 'urn:wos:fragment')}#fragment{safe}",
            **definitions,
        }
        composed.update(sub_schema)
        return Draft202012Validator(
            composed,
            registry=registry,
            format_checker=FormatChecker(),
        )
    return Draft202012Validator(
        sub_schema,
        registry=registry,
        format_checker=FormatChecker(),
    )


def _walk_schema_with_pointer(
    node: Any, pointer: str = ""
) -> Iterator[tuple[str, dict[str, Any]]]:
    """Yield (json_pointer, sub_schema) for every object node in a schema.

    Uses the same authored subschema keywords as the Rust schema-doc lint.
    Pointers are relative to the host schema root.
    """
    if isinstance(node, dict):
        yield pointer, node
        for key, child in node.items():
            child_pointer = f"{pointer}/{_escape_pointer(key)}"
            if key in _MAP_SUBSCHEMAS and isinstance(child, dict):
                for sub_key, sub_child in child.items():
                    yield from _walk_schema_with_pointer(
                        sub_child, f"{child_pointer}/{_escape_pointer(sub_key)}"
                    )
            elif key in _DIRECT_SUBSCHEMAS and isinstance(child, dict):
                yield from _walk_schema_with_pointer(child, child_pointer)
            elif key in _LIST_SUBSCHEMAS and isinstance(child, list):
                for i, sub_child in enumerate(child):
                    yield from _walk_schema_with_pointer(sub_child, f"{child_pointer}/{i}")

def _escape_pointer(token: str) -> str:
    return token.replace("~", "~0").replace("/", "~1")


def _is_validatable_subschema(node: dict[str, Any]) -> bool:
    """A schema-shaped object that can be turned into a validator.

    Skip metadata-only nodes that happen to carry an `examples` array but no
    constraint surface (rare in practice but defensive).
    """
    return bool(node.keys() & {
        "type", "enum", "const", "$ref", "pattern",
        *(_MAP_SUBSCHEMAS - {"$defs", "definitions"}), *_DIRECT_SUBSCHEMAS, *_LIST_SUBSCHEMAS,
    })


@pytest.mark.parametrize(("schema", "expected"), [
    ({"definitions": {"a/b": {"type": "string"}}}, "/definitions/a~1b"),
    ({"dependentSchemas": {"a/b": {"type": "string"}}}, "/dependentSchemas/a~1b"),
    ({"propertyNames": {"type": "string"}}, "/propertyNames"),
    ({"contains": {"type": "string"}}, "/contains"),
    ({"prefixItems": [True, {"type": "string"}]}, "/prefixItems/1"),
    ({"items": [True, {"type": "string"}]}, "/items/1"),
])
def test_example_walk_reaches_schema_keywords(schema, expected):
    assert [pointer for pointer, _ in _walk_schema_with_pointer(schema)] == ["", expected]


def test_every_schema_example_validates_against_its_fragment(
    registry: Registry,
    schemas_for_examples: dict[str, dict[str, Any]],
) -> None:
    """Every `examples[i]` MUST validate against the sub-schema that declares it.

    Collects (schema_marker, json_pointer, example_index, example_value,
    sub_schema) tuples across every classified schema. Builds a validator
    per sub-schema (registry-aware so cross-schema $refs resolve) and asserts
    each example is valid. Failures aggregate so a drifted schema reports all
    drifted examples at once, not just the first.
    """
    failures: list[str] = []

    for marker, schema in schemas_for_examples.items():
        for pointer, sub_schema in _walk_schema_with_pointer(schema):
            examples = sub_schema.get("examples")
            if not isinstance(examples, list) or not examples:
                continue
            if not _is_validatable_subschema(sub_schema):
                continue
            try:
                validator = _validator_for_fragment(
                    marker, pointer, sub_schema, registry, schemas_for_examples
                )
            except Exception as e:
                failures.append(
                    f"{marker} {pointer or '(root)'}: validator construction failed: {e}"
                )
                continue
            for i, example in enumerate(examples):
                errors = sorted(validator.iter_errors(example), key=lambda e: e.path)
                if errors:
                    err_msgs = "; ".join(
                        f"{'/'.join(str(p) for p in e.absolute_path) or '(root)'}: {e.message}"
                        for e in errors
                    )
                    failures.append(
                        f"{marker} {pointer or '(root)'} examples[{i}]: {err_msgs}"
                    )

    if failures:
        msg = "\n  - ".join(["schema examples failed validation:"] + failures)
        pytest.fail(msg)
