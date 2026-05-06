#!/usr/bin/env python3
"""ADR 0082 D-13 Gate 2 — OpenAPI staleness + reference-discipline check.

Walks `work-spec/api/wos-public-api.openapi.json` and asserts:

1. Every `$ref` in the document either:
   a. is a fragment-relative ref (`#/...`) into the same document, OR
   b. resolves to the schema family `work-spec/schemas/api/*.schema.json`
      (matched by `$id` URL — ADR D-1 fixes the canonical
      `https://schemas.formspec.io/wos-api/<resource>/v<major>` family), OR
   c. is a small allowlist of non-WOS standards refs (OpenAPI meta-schema
      and JSON Schema dialect). The allowlist is closed and explicit.

2. No inline schemas under `components.schemas` — every entry must be a
   pure `{"$ref": "..."}` object pointing into the registered schema family.

3. The committed snapshot is not stale relative to the server build, when
   a server-emitted snapshot is available at `work-spec/api/.openapi-emitted.json`.
   The current ADR Notes flow commits the doc by hand; the utoipa emission
   path is tracked as ADR 0082 follow-up. When the emitted file is absent
   (current state), this check is skipped with a printed warning so CI
   does not block on infrastructure that does not yet exist.

Exit codes:
  0 — OK
  1 — gate violation
  2 — invocation error (paths missing, etc.)

Cite: ADR 0082 D-13 (gate 2).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Iterable

WOS_SPEC_ROOT = Path(__file__).resolve().parents[1]
OPENAPI_PATH = WOS_SPEC_ROOT / "api" / "wos-public-api.openapi.json"
SCHEMAS_DIR = WOS_SPEC_ROOT / "schemas"
API_SCHEMAS_DIR = SCHEMAS_DIR / "api"
EMITTED_SNAPSHOT_PATH = WOS_SPEC_ROOT / "api" / ".openapi-emitted.json"

# ADR D-1 canonical schema family. Every API schema declares an `$id` under
# this prefix; gate 2 asserts every non-meta `$ref` in the OpenAPI doc points
# at a `$id` URL discovered via filesystem walk of `schemas/api/*.schema.json`.
WOS_API_SCHEMA_ID_PREFIX = "https://schemas.formspec.io/wos-api/"

# Closed allowlist for non-schema-family `$ref` URLs the OpenAPI doc may use.
# Keep small. New entries require an ADR amendment.
ALLOWED_NON_FAMILY_REFS = {
    # OpenAPI 3.1 meta-schema reference (used by the GET /api/openapi.json
    # response shape declaration). ADR D-1 acknowledges OpenAPI 3.1.
    "https://spec.openapis.org/oas/3.1/schema/2022-10-07",
}


def _walk_refs(node: object, trail: tuple[str, ...] = ()) -> Iterable[tuple[tuple[str, ...], str]]:
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "$ref" and isinstance(value, str):
                yield trail, value
            else:
                yield from _walk_refs(value, trail + (str(key),))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from _walk_refs(value, trail + (str(index),))


def _walk_objects(node: object, trail: tuple[str, ...] = ()) -> Iterable[tuple[tuple[str, ...], dict]]:
    if isinstance(node, dict):
        yield trail, node
        for key, value in node.items():
            yield from _walk_objects(value, trail + (str(key),))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from _walk_objects(value, trail + (str(index),))


def _load_registered_schema_ids() -> set[str]:
    ids: set[str] = set()
    for path in sorted(API_SCHEMAS_DIR.glob("*.schema.json")):
        data = json.loads(path.read_text())
        schema_id = data.get("$id")
        if not isinstance(schema_id, str):
            print(
                f"::error file={path}::missing or non-string $id; ADR D-14 requires stable $id URLs",
                file=sys.stderr,
            )
            continue
        if not schema_id.startswith(WOS_API_SCHEMA_ID_PREFIX):
            print(
                f"::error file={path}::$id {schema_id!r} not under canonical family prefix "
                f"{WOS_API_SCHEMA_ID_PREFIX!r} (ADR 0082 D-1)",
                file=sys.stderr,
            )
            continue
        ids.add(schema_id)
    return ids


def _check_components_schemas_are_pure_refs(doc: dict) -> list[str]:
    violations: list[str] = []
    components = doc.get("components", {})
    if not isinstance(components, dict):
        return violations
    schemas = components.get("schemas", {})
    if not isinstance(schemas, dict):
        return violations
    for name, value in schemas.items():
        if not isinstance(value, dict):
            violations.append(
                f"components.schemas.{name}: must be a pure $ref object (ADR 0082 D-1: no inline schemas)"
            )
            continue
        if set(value) != {"$ref"}:
            violations.append(
                f"components.schemas.{name}: must be exactly {{\"$ref\": \"...\"}} — found keys {sorted(value)} "
                f"(ADR 0082 D-1: components.schemas entries are always $refs into work-spec/schemas/api/)"
            )
    return violations


def _check_no_inline_schemas_in_paths(doc: dict) -> list[str]:
    """Schemas under `paths.*.responses.*.content.*.schema` and request bodies
    must always be a `$ref`. Inline `type:` / `properties:` shapes are forbidden
    by ADR D-1.
    """
    violations: list[str] = []
    paths = doc.get("paths", {})
    if not isinstance(paths, dict):
        return violations
    for trail, node in _walk_objects(paths):
        # The `schema` key inside content/parameters is the binding seam.
        if not trail or trail[-1] != "schema":
            continue
        if not isinstance(node, dict):
            continue
        if "$ref" in node:
            continue
        # Some schema slots use `type: string` + format/pattern as primitive
        # leaves (e.g. headers like X-WOS-API-Version). These are not "inline
        # resource schemas" — they are primitive types that have no business
        # being resource refs. Allow them only when they are pure primitives.
        keys = set(node)
        if keys <= {"type", "format", "pattern", "description", "example", "enum", "const", "minimum", "maximum"}:
            continue
        if "type" in node and node.get("type") in {"string", "integer", "number", "boolean"}:
            continue
        violations.append(
            f"paths.{'/'.join(trail)}: inline schema not allowed — must $ref into "
            f"work-spec/schemas/api/*.schema.json (ADR 0082 D-1)"
        )
    return violations


def _check_refs_resolve_to_family(doc: dict, registered_ids: set[str]) -> list[str]:
    violations: list[str] = []
    for trail, ref in _walk_refs(doc):
        if ref.startswith("#/"):
            # internal fragment ref — the OpenAPI components system handles it;
            # not in scope for the schema-family check.
            continue
        # Strip fragment to get the bare schema $id URL
        schema_id = ref.split("#", 1)[0]
        if schema_id in ALLOWED_NON_FAMILY_REFS:
            continue
        if schema_id.startswith(WOS_API_SCHEMA_ID_PREFIX):
            if schema_id not in registered_ids:
                violations.append(
                    f"$ref at {'/'.join(trail)}: {ref!r} points at schema family but no "
                    f"work-spec/schemas/api/*.schema.json declares $id={schema_id!r} "
                    f"(ADR 0082 D-13 gate 2)"
                )
            continue
        violations.append(
            f"$ref at {'/'.join(trail)}: {ref!r} is neither a fragment ref, a registered "
            f"api schema, nor an allowlisted standards ref (ADR 0082 D-13 gate 2)"
        )
    return violations


def _check_emitted_snapshot_freshness(committed: dict) -> list[str]:
    """When `work-spec/api/.openapi-emitted.json` exists, assert byte-for-byte
    equality (after canonical JSON normalization) against the committed doc.
    The emitted file is produced by the server build and is never committed;
    CI writes it before invoking this script. When absent, skip with warning —
    utoipa emission is an ADR 0082 follow-up.
    """
    violations: list[str] = []
    if not EMITTED_SNAPSHOT_PATH.exists():
        print(
            "::notice::ADR 0082 D-13 gate 2: server-emitted OpenAPI snapshot at "
            f"{EMITTED_SNAPSHOT_PATH.relative_to(WOS_SPEC_ROOT)} not present; skipping "
            "staleness comparison. Wire utoipa emission per ADR D-13 to enable.",
            file=sys.stderr,
        )
        return violations
    emitted = json.loads(EMITTED_SNAPSHOT_PATH.read_text())
    if json.dumps(emitted, sort_keys=True) != json.dumps(committed, sort_keys=True):
        violations.append(
            f"committed {OPENAPI_PATH.relative_to(WOS_SPEC_ROOT)} is stale relative to "
            f"server-emitted snapshot {EMITTED_SNAPSHOT_PATH.relative_to(WOS_SPEC_ROOT)} "
            f"(ADR 0082 D-13 gate 2). Run server build and commit the regenerated file."
        )
    return violations


def main() -> int:
    if not OPENAPI_PATH.exists():
        print(f"::error::{OPENAPI_PATH} missing", file=sys.stderr)
        return 2
    if not SCHEMAS_DIR.exists():
        print(f"::error::{SCHEMAS_DIR} missing", file=sys.stderr)
        return 2

    doc = json.loads(OPENAPI_PATH.read_text())
    registered_ids = _load_registered_schema_ids()
    if not registered_ids:
        print(
            "::error::no work-spec/schemas/api/*.schema.json files declare $id under "
            f"{WOS_API_SCHEMA_ID_PREFIX!r}",
            file=sys.stderr,
        )
        return 1

    violations: list[str] = []
    violations += _check_components_schemas_are_pure_refs(doc)
    violations += _check_no_inline_schemas_in_paths(doc)
    violations += _check_refs_resolve_to_family(doc, registered_ids)
    violations += _check_emitted_snapshot_freshness(doc)

    if violations:
        for v in violations:
            print(f"::error::{v}", file=sys.stderr)
        print(
            f"\nADR 0082 D-13 gate 2 failed: {len(violations)} violation(s).",
            file=sys.stderr,
        )
        return 1

    print(
        f"ADR 0082 D-13 gate 2 passed: {OPENAPI_PATH.relative_to(WOS_SPEC_ROOT)} "
        f"references {len(registered_ids)} registered api schema(s) cleanly."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
