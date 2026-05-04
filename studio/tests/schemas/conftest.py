"""Shared helpers for WOS Studio (Authoring) schema regression tests.

Carved out of the parent `tests/schemas/conftest.py` (Wave 0.1 of the
Studio decoupling, 2026-05-02): the parent ratchet stops reaching across
the Studio boundary; Studio's quality lives under Studio's own test
runner. Studio schemas are self-contained — no `$ref` into parent
schemas — so the Studio registry doesn't need to load parent files.

Provides:
- `WOS_SPEC_ROOT` / `STUDIO_SCHEMAS_ROOT` for repo-relative path building.
- `MARKER_TO_SCHEMA` — `$wosStudio*` marker → schema file mapping.
- A session-scoped registry keyed by `$id` so cross-Studio `$ref`s
  (e.g., into `wos-studio-common.schema.json#/$defs/OriginClass`) resolve.
- `validators` fixture and `validator_for_def(name)` helper, mirroring
  the parent shape so test files port cleanly.
- `classify(doc)` — first `$wos*` marker key in a document, or None.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012

WOS_SPEC_ROOT = Path(__file__).resolve().parents[3]
STUDIO_SCHEMAS_ROOT = WOS_SPEC_ROOT / "studio" / "schemas"

# `$wosStudio*` marker → schema file (relative to STUDIO_SCHEMAS_ROOT).
MARKER_TO_SCHEMA: dict[str, str] = {
    "$wosStudioPolicyObject": "wos-studio-policy-object.schema.json",
    "$wosStudioBinding": "wos-studio-binding.schema.json",
    "$wosStudioWorkflowIntent": "wos-studio-workflow-intent.schema.json",
    "$wosStudioScenario": "wos-studio-scenario.schema.json",
    "$wosStudioSource": "wos-studio-source.schema.json",
    "$wosStudioWorkspace": "wos-studio-workspace.schema.json",
    "$wosStudioApproval": "wos-studio-approval.schema.json",
    "$wosStudioReadiness": "wos-studio-readiness.schema.json",
    "$wosStudioProvenance": "wos-studio-provenance.schema.json",
    "$wosStudioEffectiveness": "wos-studio-effectiveness.schema.json",
    "$wosStudioIdentitySubject": "wos-studio-identity-subject.schema.json",
    "$wosStudioTerminologyMap": "wos-studio-terminology-map.schema.json",
    "$wosStudioMigrationPath": "wos-studio-migration-path.schema.json",
    "$wosStudioMapping": "wos-studio-mapping.schema.json",
}

# Studio schemas without a document marker (pure $defs libraries
# referenced via $ref from other Studio schemas).
UNMARKED_STUDIO_SCHEMAS: list[str] = [
    "wos-studio-common.schema.json",
]


def _load_schema(rel: str) -> dict[str, Any]:
    return json.loads((STUDIO_SCHEMAS_ROOT / rel).read_text())


def _build_registry() -> tuple[Registry, dict[str, dict[str, Any]]]:
    by_marker: dict[str, dict[str, Any]] = {}
    resources: list[tuple[str, Resource]] = []
    for marker, rel in MARKER_TO_SCHEMA.items():
        schema = _load_schema(rel)
        by_marker[marker] = schema
        schema_id = schema.get("$id")
        if schema_id:
            resources.append((schema_id, DRAFT202012.create_resource(schema)))
            if not schema_id.endswith(".schema.json"):
                file_alias = "https://wos-spec.org/studio/schemas/" + Path(rel).name
                resources.append((file_alias, DRAFT202012.create_resource(schema)))
    for rel in UNMARKED_STUDIO_SCHEMAS:
        schema = _load_schema(rel)
        schema_id = schema.get("$id")
        if schema_id:
            resources.append((schema_id, DRAFT202012.create_resource(schema)))
            if not schema_id.endswith(".schema.json"):
                file_alias = "https://wos-spec.org/studio/schemas/" + Path(rel).name
                resources.append((file_alias, DRAFT202012.create_resource(schema)))
    registry = Registry().with_resources(resources)
    return registry, by_marker


_REGISTRY, _SCHEMAS_BY_MARKER = _build_registry()


@pytest.fixture(scope="session")
def registry() -> Registry:
    return _REGISTRY


@pytest.fixture(scope="session")
def schemas_by_marker() -> dict[str, dict[str, Any]]:
    return _SCHEMAS_BY_MARKER


@pytest.fixture(scope="session")
def validators(
    registry: Registry, schemas_by_marker: dict[str, dict[str, Any]]
) -> dict[str, Draft202012Validator]:
    compiled: dict[str, Draft202012Validator] = {}
    for marker, schema in schemas_by_marker.items():
        compiled[marker] = Draft202012Validator(
            schema,
            registry=registry,
            format_checker=FormatChecker(),
        )
    return compiled


def _find_def(def_name: str) -> tuple[dict[str, Any], dict[str, Any]]:
    for schema in _SCHEMAS_BY_MARKER.values():
        defs = schema.get("$defs") or {}
        if def_name in defs:
            return schema, defs[def_name]
    raise KeyError(
        f"$def {def_name!r} not found in any Studio schema; "
        f"check MARKER_TO_SCHEMA coverage."
    )


def validator_for_def(
    def_name: str, *, registry: Registry | None = None
) -> Draft202012Validator:
    host, target = _find_def(def_name)
    composed = {
        "$schema": host.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{host.get('$id', 'urn:test')}#${def_name}",
        "$defs": host["$defs"],
        **target,
    }
    return Draft202012Validator(
        composed,
        registry=registry or _REGISTRY,
        format_checker=FormatChecker(),
    )


def classify(doc: object) -> str | None:
    if not isinstance(doc, dict):
        return None
    for key in doc:
        if isinstance(key, str) and key.startswith("$wos"):
            return key
    return None
