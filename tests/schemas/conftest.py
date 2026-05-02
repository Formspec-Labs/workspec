"""Shared helpers for WOS schema regression tests.

Provides:
- WOS_SPEC_ROOT / SCHEMAS_ROOT constants for repo-relative path building.
- MARKER_TO_SCHEMA — the authoritative mapping from each `$wos*` document
  marker to its declaring schema file. When a new schema is added to
  `schemas/`, add its `$wos*` marker here and every fixture + spec code
  block carrying that marker becomes part of the regression suite
  automatically.
- A session-scoped registry of every classified schema, keyed by `$id`
  URL, so any schema (including cross-schema `$ref`s such as
  `wos-provenance-log.schema.json`'s reference into
  `wos-workflow.schema.json#/$defs/FactsTierRecord`) resolves cleanly
  under Draft 2020-12.
- `validators` fixture — registry-aware compiled validators per marker.
- `validator_for_def(def_name)` — registry-aware bare-`$def` validator
  for any def in any classified schema. Replaces per-file
  `_validator_for_def` helpers; spans cross-schema $defs (e.g.
  `AuthorityBasis` lives in `wos-workflow` but is exercised by tests
  that load `wos-provenance-log`).
- `classify(doc)` — returns the `$wos*` marker key present in a
  document, or None if the document is unmarked (e.g. negative-fixture
  catalogs or non-WOS auxiliary data).
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
SCHEMAS_ROOT = WOS_SPEC_ROOT / "schemas"

# Marker → schema path. Post-ADR-0076 (D-6) the schema family collapsed to 6
# files — one author-time core ($wosWorkflow), two sidecars ($wosDelivery,
# $wosOntologyAlignment), two runtime artifacts ($wosCaseInstance,
# $wosProvenanceLog), one tooling ($wosTooling). Legacy per-block schemas have
# been deleted; their normative content is absorbed under embedded blocks of
# wos-workflow.schema.json. Spec example blocks and fixtures use the canonical
# six markers — no legacy compat shims.
MARKER_TO_SCHEMA: dict[str, str] = {
    "$wosWorkflow": "wos-workflow.schema.json",
    "$wosDelivery": "sidecars/wos-delivery.schema.json",
    "$wosOntologyAlignment": "sidecars/wos-ontology-alignment.schema.json",
    "$wosCaseInstance": "wos-case-instance.schema.json",
    "$wosProvenanceLog": "wos-provenance-log.schema.json",
    "$wosTooling": "wos-tooling.schema.json",
    # Studio (Authoring) Stage-3 schemas — see studio-authoring/CONCEPT-MODEL.md
    # §6.1 composition strategy. wos-studio-common.schema.json carries shared
    # $defs (StudioMetadataEnvelope etc.) and has no document marker; it's
    # registered below via UNMARKED_STUDIO_SCHEMAS so cross-schema $refs resolve.
    "$wosStudioPolicyObject": "studio/wos-studio-policy-object.schema.json",
    "$wosStudioBinding": "studio/wos-studio-binding.schema.json",
    "$wosStudioWorkflowIntent": "studio/wos-studio-workflow-intent.schema.json",
    "$wosStudioScenario": "studio/wos-studio-scenario.schema.json",
    "$wosStudioSource": "studio/wos-studio-source.schema.json",
    "$wosStudioWorkspace": "studio/wos-studio-workspace.schema.json",
    "$wosStudioApproval": "studio/wos-studio-approval.schema.json",
    "$wosStudioReadiness": "studio/wos-studio-readiness.schema.json",
    "$wosStudioProvenance": "studio/wos-studio-provenance.schema.json",
    "$wosStudioEffectiveness": "studio/wos-studio-effectiveness.schema.json",
    "$wosStudioIdentitySubject": "studio/wos-studio-identity-subject.schema.json",
    "$wosStudioTerminologyMap": "studio/wos-studio-terminology-map.schema.json",
    "$wosStudioMigrationPath": "studio/wos-studio-migration-path.schema.json",
}

# Studio schemas that have NO document marker (they are pure $defs libraries
# referenced via $ref from other schemas). Registered with the resolver so
# cross-schema $refs work, but not attached to a marker.
UNMARKED_STUDIO_SCHEMAS: list[str] = [
    "studio/wos-studio-common.schema.json",
]


def _load_schema(rel: str) -> dict[str, Any]:
    return json.loads((SCHEMAS_ROOT / rel).read_text())


def _build_registry() -> tuple[Registry, dict[str, dict[str, Any]]]:
    """Load every classified schema, key by `$id`, build a Registry that
    resolves cross-schema `$ref` URLs. Returns the registry alongside a
    by-marker schema dict so callers can both validate and crawl `$defs`.
    """
    by_marker: dict[str, dict[str, Any]] = {}
    resources: list[tuple[str, Resource]] = []
    for marker, rel in MARKER_TO_SCHEMA.items():
        schema = _load_schema(rel)
        by_marker[marker] = schema
        schema_id = schema.get("$id")
        if schema_id:
            resources.append((schema_id, DRAFT202012.create_resource(schema)))
            # The provenance-log schema (and others) carry `$ref`s that point at
            # the workflow schema's *legacy* file-name URL
            # (`wos-workflow.schema.json`) rather than its canonical short URL
            # (`workflow/1.0`). Register both forms when a schema declares a
            # short `$id` so the file-name form resolves too.
            if not schema_id.endswith(".schema.json"):
                file_alias = (
                    "https://wos-spec.org/schemas/"
                    + Path(rel).name
                )
                resources.append(
                    (file_alias, DRAFT202012.create_resource(schema))
                )
    # Register Studio schemas that have no document marker but ARE referenced
    # via $ref (notably wos-studio-common.schema.json carrying shared $defs).
    for rel in UNMARKED_STUDIO_SCHEMAS:
        schema = _load_schema(rel)
        schema_id = schema.get("$id")
        if schema_id:
            resources.append((schema_id, DRAFT202012.create_resource(schema)))
            if not schema_id.endswith(".schema.json"):
                file_alias = (
                    "https://wos-spec.org/schemas/"
                    + Path(rel).name
                )
                resources.append(
                    (file_alias, DRAFT202012.create_resource(schema))
                )
    registry = Registry().with_resources(resources)
    return registry, by_marker


_REGISTRY, _SCHEMAS_BY_MARKER = _build_registry()


@pytest.fixture(scope="session")
def registry() -> Registry:
    """Session-wide referencing.Registry resolving every classified schema."""
    return _REGISTRY


@pytest.fixture(scope="session")
def schemas_by_marker() -> dict[str, dict[str, Any]]:
    """Session-wide map: `$wos*` marker → loaded schema dict."""
    return _SCHEMAS_BY_MARKER


@pytest.fixture(scope="session")
def validators(registry: Registry, schemas_by_marker: dict[str, dict[str, Any]]) -> dict[str, Draft202012Validator]:
    """Compile every classified schema once per session, registry-aware."""
    compiled: dict[str, Draft202012Validator] = {}
    for marker, schema in schemas_by_marker.items():
        compiled[marker] = Draft202012Validator(
            schema,
            registry=registry,
            format_checker=FormatChecker(),
        )
    return compiled


def _find_def(def_name: str) -> tuple[dict[str, Any], dict[str, Any]]:
    """Locate `def_name` across every classified schema's `$defs`. Returns
    ``(host_schema, target_def)``. Raises KeyError if no schema declares it.

    **Duplicate names:** iteration follows ``MARKER_TO_SCHEMA`` insertion order
    (see ``_build_registry``). The **first** schema that declares ``def_name``
    wins. Do not reuse the same ``$def`` name across classified files unless
    intentionally identical; otherwise ``validator_for_def`` may bind to the
    wrong host's ``$defs`` for composed validation.
    """
    for schema in _SCHEMAS_BY_MARKER.values():
        defs = schema.get("$defs") or {}
        if def_name in defs:
            return schema, defs[def_name]
    raise KeyError(
        f"$def {def_name!r} not found in any classified schema; "
        f"check MARKER_TO_SCHEMA coverage."
    )


def validator_for_def(
    def_name: str,
    *,
    registry: Registry | None = None,
) -> Draft202012Validator:
    """Build a registry-aware Draft-2020-12 validator over a single `$def`.

    Spans every classified schema — callers no longer need to know which
    file declares `def_name`. Cross-schema `$ref`s (e.g. provenance-log
    referencing workflow's `FactsTierRecord`) resolve through the shared
    registry. Format-checking is enabled so `format: uri` and friends bite.
    """
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
    """Return the first `$wos*` marker key in a document, or None.

    Documents without a marker are auxiliary artifacts (negative-fixture
    catalogs, scenario transcripts) that the regression suite skips.
    """
    if not isinstance(doc, dict):
        return None
    for key in doc:
        if isinstance(key, str) and key.startswith("$wos"):
            return key
    return None
