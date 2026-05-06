#!/usr/bin/env python3
"""ADR 0082 D-13 Gate 6 — API-mirror byte-parity check.

ADR 0082 D-14 says: "`$ref` kernel/governance/AI types from existing schemas
in `work-spec/schemas/wos-*.schema.json` instead of redefining. The API layer
is a projection, not an alternative reality." Today (PLN-0403) the
provenance API schema inline-mirrors several kernel types (`RuleReference`,
`EvidenceReference`, `CaseFileSnapshot`, `FactsRecordKind`) because typify's
$ref resolver does not traverse cross-schema absolute-URL refs out of the
box. PLN-0403 path (a) wires the resolver and removes the mirrors; path (b)
keeps the mirrors but enforces structural parity at CI time.

This script lands path (b). It walks every `work-spec/schemas/api/*.schema.json`,
finds every `$def` carrying an `x-wos.mirror` pointer, resolves the named
source `$def` in `work-spec/schemas/wos-*.schema.json`, and compares the
two on a curated set of load-bearing keywords. Divergence on any of:

    type, pattern, format, enum, const,
    minLength, maxLength, minimum, maximum,
    required, additionalProperties,
    oneOf, anyOf, allOf,
    x-wos.openStringKind

…fails the gate. Documentation-shaped keys (`description`, `$comment`,
`examples`, `default`, `x-lm`, `patternProperties`, `title`) are allowed to
diverge — those are projection prose, not contract.

`$ref` resolution within the API or kernel schema is performed before
comparison: if one side uses `$ref: "#/$defs/X"` and the other inlines the
shape, the script resolves the `$ref` against the same schema's `$defs`
and compares the resolved load-bearing keys. This lets the API project
shared sub-shapes (e.g. `SourceAuthority`, `EvidenceKind`) into named
$defs while the kernel inlines them — without false-positive divergences.

Mirror declaration in the API schema:

    "x-wos": {
      "mirror": {
        "source": "wos-workflow.schema.json",
        "path": "$defs/RuleReference"
      }
    }

The block is read in addition to (not instead of) the existing `description`
prose — the structured pointer is the contract; the prose is documentation.

Cite: ADR 0082 D-13 gate 6, PLN-0403 path (b),
`work-spec/scripts/check-recordkind-parity.py` (precedent pattern).

Usage:
    python3 scripts/check-api-mirror-parity.py [--root WORK_SPEC_DIR]

Exit codes:
    0 — every annotated mirror matches its source
    1 — at least one mirror diverges
    2 — invocation error (missing files, malformed mirror pointer)
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Iterable

# Load-bearing keywords that MUST byte-match between API mirror and kernel
# source. Any divergence on these is a contract drift and fails the gate.
LOAD_BEARING_KEYS: frozenset[str] = frozenset(
    {
        "type",
        "pattern",
        "format",
        "enum",
        "const",
        "minLength",
        "maxLength",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "required",
        "additionalProperties",
        "oneOf",
        "anyOf",
        "allOf",
        "items",
    }
)

# Documentation/styling keys that are allowed to diverge between mirror and
# source. The mirror is a projection; these carry projection prose only.
ALLOWED_DIVERGENT_KEYS: frozenset[str] = frozenset(
    {
        "description",
        "$comment",
        "examples",
        "default",
        "title",
        "x-lm",
        "patternProperties",
    }
)


class MirrorViolation(Exception):
    """Raised internally to short-circuit comparison with a structured diff."""

    def __init__(self, path: str, message: str) -> None:
        super().__init__(message)
        self.path = path
        self.message = message


def canonical(value: Any) -> Any:
    """Return a sort-stable canonical form for comparison. Lists keep order
    (order of `oneOf`/`anyOf`/`enum` entries can be load-bearing; if both
    sides agree on order, fine; if they don't, surface the divergence)."""
    if isinstance(value, dict):
        return {k: canonical(value[k]) for k in sorted(value.keys())}
    if isinstance(value, list):
        return [canonical(v) for v in value]
    return value


def resolve_ref(node: dict, root: dict) -> dict:
    """Resolve a single `$ref` node against `root`. Only handles
    fragment-relative refs of the form `#/$defs/<name>` (the shape used by
    every API and kernel schema in this tree). Returns the dereferenced
    target dict; raises `MirrorViolation` if the ref is unresolvable.

    Multi-hop refs are followed transitively. External absolute-URL refs
    are NOT resolved (PLN-0403 path (a) is the future home for that).
    """
    seen: set[str] = set()
    current = node
    while isinstance(current, dict) and "$ref" in current:
        ref = current["$ref"]
        if not isinstance(ref, str) or not ref.startswith("#/"):
            # External / absolute refs — PLN-0403 path (a) wires these.
            # For path (b) we treat them as opaque values: the comparison
            # falls back to byte-equal of the raw `$ref` strings.
            return current
        if ref in seen:
            raise MirrorViolation(
                "<ref-cycle>",
                f"$ref cycle detected: {ref}",
            )
        seen.add(ref)
        # Walk fragment path: "#/$defs/RuleReference" → root["$defs"]["RuleReference"]
        parts = ref[2:].split("/")
        target: Any = root
        for part in parts:
            if not isinstance(target, dict) or part not in target:
                raise MirrorViolation(
                    "<unresolved-ref>",
                    f"could not resolve $ref {ref!r} in source schema",
                )
            target = target[part]
        if not isinstance(target, dict):
            return current
        current = target
    return current


def compare(
    mirror_node: Any,
    source_node: Any,
    *,
    mirror_root: dict,
    source_root: dict,
    path: str = "",
) -> Iterable[str]:
    """Yield human-readable divergence lines comparing `mirror_node`
    against `source_node` on the load-bearing key set. Recurses into
    `properties` and the sub-schemas under load-bearing combinator keys.

    Empty iterable means parity holds.
    """
    # Resolve `$ref` on either side first so the comparison runs over
    # equivalent inline shapes. Accept `$ref` accompanied by allowed-divergent
    # keys (`description`, `examples`, etc.) — those are projection prose and
    # do not block resolution per JSON Schema 2020-12 (where adjacent
    # keywords MAY co-exist with `$ref`).
    if isinstance(mirror_node, dict) and "$ref" in mirror_node:
        non_doc_keys = set(mirror_node.keys()) - ALLOWED_DIVERGENT_KEYS - {"$ref"}
        if not non_doc_keys:
            try:
                mirror_node = resolve_ref(mirror_node, mirror_root)
            except MirrorViolation as exc:
                yield f"{path or '<root>'}: {exc.message}"
                return
    if isinstance(source_node, dict) and "$ref" in source_node:
        non_doc_keys = set(source_node.keys()) - ALLOWED_DIVERGENT_KEYS - {"$ref"}
        if not non_doc_keys:
            try:
                source_node = resolve_ref(source_node, source_root)
            except MirrorViolation as exc:
                yield f"{path or '<root>'}: {exc.message}"
                return

    # Type-shape sanity: if one side is a dict and the other isn't, that's
    # a load-bearing divergence.
    if isinstance(mirror_node, dict) != isinstance(source_node, dict):
        yield (
            f"{path or '<root>'}: structural type mismatch — "
            f"mirror is {type(mirror_node).__name__}, "
            f"source is {type(source_node).__name__}"
        )
        return

    if not isinstance(mirror_node, dict):
        # Leaf scalar / list — compare canonical form directly.
        if canonical(mirror_node) != canonical(source_node):
            yield (
                f"{path or '<root>'}: value divergence\n"
                f"    mirror: {json.dumps(canonical(mirror_node), sort_keys=True)}\n"
                f"    source: {json.dumps(canonical(source_node), sort_keys=True)}"
            )
        return

    # Both dicts. Walk the load-bearing keys and the special structural
    # keys (`properties`, `items`, `x-wos`).

    # Compare each load-bearing scalar/list key.
    for key in LOAD_BEARING_KEYS:
        if key in {"oneOf", "anyOf", "allOf", "items"}:
            continue  # handled below with sub-schema recursion
        if key in mirror_node or key in source_node:
            mirror_val = mirror_node.get(key, "<absent>")
            source_val = source_node.get(key, "<absent>")
            if canonical(mirror_val) != canonical(source_val):
                yield (
                    f"{path or '<root>'}/{key}: divergence\n"
                    f"    mirror: {json.dumps(canonical(mirror_val), sort_keys=True)}\n"
                    f"    source: {json.dumps(canonical(source_val), sort_keys=True)}"
                )

    # `x-wos.openStringKind` is the dropped-pin class — load-bearing.
    mirror_xwos = mirror_node.get("x-wos") if isinstance(mirror_node.get("x-wos"), dict) else {}
    source_xwos = source_node.get("x-wos") if isinstance(source_node.get("x-wos"), dict) else {}
    mirror_open = mirror_xwos.get("openStringKind", "<absent>") if isinstance(mirror_xwos, dict) else "<absent>"
    source_open = source_xwos.get("openStringKind", "<absent>") if isinstance(source_xwos, dict) else "<absent>"
    # Skip comparison when neither side declares openStringKind. The
    # `x-wos.mirror` pointer block itself lives under `x-wos` on the
    # mirror side only — exclude it from the comparison.
    if mirror_open != "<absent>" or source_open != "<absent>":
        if canonical(mirror_open) != canonical(source_open):
            yield (
                f"{path or '<root>'}/x-wos.openStringKind: divergence "
                f"(dropped-pin class — ADR 0082 D-14)\n"
                f"    mirror: {json.dumps(canonical(mirror_open))}\n"
                f"    source: {json.dumps(canonical(source_open))}"
            )

    # Recurse into combinator sub-schema lists (`oneOf`, `anyOf`, `allOf`).
    for combinator in ("oneOf", "anyOf", "allOf"):
        m_list = mirror_node.get(combinator)
        s_list = source_node.get(combinator)
        if m_list is None and s_list is None:
            continue
        if (m_list is None) != (s_list is None):
            yield (
                f"{path or '<root>'}/{combinator}: presence divergence — "
                f"mirror has {'present' if m_list is not None else 'absent'}, "
                f"source has {'present' if s_list is not None else 'absent'}"
            )
            continue
        if not isinstance(m_list, list) or not isinstance(s_list, list):
            continue
        if len(m_list) != len(s_list):
            yield (
                f"{path or '<root>'}/{combinator}: length divergence — "
                f"mirror has {len(m_list)}, source has {len(s_list)}"
            )
            continue
        for idx, (m_item, s_item) in enumerate(zip(m_list, s_list)):
            yield from compare(
                m_item,
                s_item,
                mirror_root=mirror_root,
                source_root=source_root,
                path=f"{path}/{combinator}[{idx}]",
            )

    # Recurse into `items` (array element schema).
    if "items" in mirror_node or "items" in source_node:
        yield from compare(
            mirror_node.get("items"),
            source_node.get("items"),
            mirror_root=mirror_root,
            source_root=source_root,
            path=f"{path}/items",
        )

    # Recurse into `properties` — compare key sets, then each property's schema.
    mirror_props = mirror_node.get("properties")
    source_props = source_node.get("properties")
    if isinstance(mirror_props, dict) or isinstance(source_props, dict):
        if not isinstance(mirror_props, dict):
            mirror_props = {}
        if not isinstance(source_props, dict):
            source_props = {}
        mirror_keys = set(mirror_props.keys())
        source_keys = set(source_props.keys())
        only_mirror = sorted(mirror_keys - source_keys)
        only_source = sorted(source_keys - mirror_keys)
        if only_mirror:
            yield (
                f"{path or '<root>'}/properties: keys present only in mirror: "
                f"{only_mirror}"
            )
        if only_source:
            yield (
                f"{path or '<root>'}/properties: keys present only in source: "
                f"{only_source}"
            )
        for prop_name in sorted(mirror_keys & source_keys):
            yield from compare(
                mirror_props[prop_name],
                source_props[prop_name],
                mirror_root=mirror_root,
                source_root=source_root,
                path=f"{path}/properties/{prop_name}",
            )


def _parse_string_mirror(mirror_str: str) -> dict | None:
    """Parse a string-form mirror declaration `"<file>#/<path>"` into the
    canonical dict form `{"source": <file>, "path": <path>}`.

    The fragment is stripped of its leading `/` so the result feeds directly
    into `resolve_source_def`, which splits on `/` and walks `source_root`.
    Returns `None` when the string does not contain a `#/...` fragment.
    """
    if "#/" not in mirror_str:
        return None
    source, _, fragment = mirror_str.partition("#/")
    if not source or not fragment:
        return None
    return {"source": source, "path": fragment}


def find_mirrors(api_schema: dict) -> Iterable[tuple[str, dict, dict]]:
    """Yield `(def_name, def_body, mirror_pointer)` for every `$def` in
    `api_schema` that carries an `x-wos.mirror` declaration.

    Two forms are accepted:
    - dict form: `{"source": "wos-X.schema.json", "path": "$defs/Foo"}`
    - string form: `"wos-X.schema.json#/$defs/Foo"` (parsed into the dict
      form before yielding).

    Both forms appear in the tree today; the string form is the older one
    (e.g. `task.schema.json#/$defs/TaskBinding`, `instance.schema.json#/$defs/LifecycleState`).
    """
    defs = api_schema.get("$defs")
    if not isinstance(defs, dict):
        return
    for def_name, def_body in defs.items():
        if not isinstance(def_body, dict):
            continue
        x_wos = def_body.get("x-wos")
        if not isinstance(x_wos, dict):
            continue
        mirror = x_wos.get("mirror")
        if isinstance(mirror, str):
            parsed = _parse_string_mirror(mirror)
            if parsed is None:
                yield def_name, def_body, {"source": mirror, "path": ""}  # malformed
                continue
            yield def_name, def_body, parsed
            continue
        if not isinstance(mirror, dict):
            continue
        if not isinstance(mirror.get("source"), str) or not isinstance(
            mirror.get("path"), str
        ):
            yield def_name, def_body, mirror  # malformed; reported below
            continue
        yield def_name, def_body, mirror


def resolve_source_def(
    source_root: dict, ref_path: str
) -> dict:
    """Resolve a `$defs/Name` (or deeper) path string against `source_root`.
    Raises `MirrorViolation` with `<unresolved-mirror>` path on miss."""
    parts = ref_path.split("/")
    target: Any = source_root
    for part in parts:
        if not isinstance(target, dict) or part not in target:
            raise MirrorViolation(
                "<unresolved-mirror>",
                f"mirror path {ref_path!r} did not resolve in source schema",
            )
        target = target[part]
    if not isinstance(target, dict):
        raise MirrorViolation(
            "<unresolved-mirror>",
            f"mirror path {ref_path!r} resolved to non-object {type(target).__name__}",
        )
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Root of the work-spec tree (default: parent of scripts/).",
    )
    args = parser.parse_args()

    root: Path = args.root
    api_dir = root / "schemas" / "api"
    schemas_dir = root / "schemas"

    if not api_dir.is_dir():
        print(f"error: {api_dir} not found", file=sys.stderr)
        return 2
    if not schemas_dir.is_dir():
        print(f"error: {schemas_dir} not found", file=sys.stderr)
        return 2

    # Pre-load every wos-*.schema.json (kernel/governance/AI/runtime sources)
    # by filename so mirror pointers can resolve cheaply.
    source_schemas: dict[str, dict] = {}
    for schema_path in sorted(schemas_dir.glob("wos-*.schema.json")):
        try:
            source_schemas[schema_path.name] = json.loads(
                schema_path.read_text(encoding="utf-8")
            )
        except (OSError, json.JSONDecodeError) as exc:
            print(
                f"error: could not load source schema {schema_path}: {exc}",
                file=sys.stderr,
            )
            return 2

    # Walk every API schema; for each annotated mirror, compare.
    total_mirrors = 0
    violation_blocks: list[str] = []
    invocation_errors: list[str] = []

    for api_path in sorted(api_dir.glob("*.schema.json")):
        try:
            api_schema = json.loads(api_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"error: could not load {api_path}: {exc}", file=sys.stderr)
            return 2
        if not isinstance(api_schema, dict):
            continue

        api_rel = api_path.relative_to(root)

        for def_name, def_body, mirror in find_mirrors(api_schema):
            total_mirrors += 1
            source_file = mirror.get("source")
            source_path = mirror.get("path")
            if not isinstance(source_file, str) or not isinstance(source_path, str):
                invocation_errors.append(
                    f"{api_rel} $defs/{def_name}: malformed x-wos.mirror block "
                    f"(expected {{source: str, path: str}}, got {mirror!r})"
                )
                continue
            if source_file not in source_schemas:
                invocation_errors.append(
                    f"{api_rel} $defs/{def_name}: x-wos.mirror.source "
                    f"{source_file!r} is not a wos-*.schema.json under "
                    f"{schemas_dir.relative_to(root)}/"
                )
                continue
            source_root = source_schemas[source_file]
            try:
                source_def = resolve_source_def(source_root, source_path)
            except MirrorViolation as exc:
                invocation_errors.append(
                    f"{api_rel} $defs/{def_name}: {exc.message}"
                )
                continue

            divergences = list(
                compare(
                    def_body,
                    source_def,
                    mirror_root=api_schema,
                    source_root=source_root,
                    path="",
                )
            )
            if divergences:
                header = (
                    f"{api_rel} $defs/{def_name}  (mirrors "
                    f"{source_file}#/{source_path})"
                )
                indented = "\n".join(f"    {line}" for line in divergences)
                violation_blocks.append(f"{header}\n{indented}")

    if invocation_errors:
        print(
            "ADR 0082 D-13 gate 6 invocation error(s):",
            file=sys.stderr,
        )
        for err in invocation_errors:
            print(f"  {err}", file=sys.stderr)
        return 2

    if violation_blocks:
        print(
            "ADR 0082 D-13 gate 6 FAIL — API mirror divergence(s) detected.\n"
            "These are real bugs: either the API mirror dropped a pin from "
            "the kernel/governance/AI source (fix the API), or the source "
            "shifted under a settled mirror (raise as a kernel-source bug).\n",
            file=sys.stderr,
        )
        for block in violation_blocks:
            print(block, file=sys.stderr)
            print("", file=sys.stderr)
        print(
            f"\n{len(violation_blocks)} mirror(s) diverged out of "
            f"{total_mirrors} annotated.",
            file=sys.stderr,
        )
        return 1

    if total_mirrors == 0:
        print(
            "ADR 0082 D-13 gate 6: no `x-wos.mirror` annotations found under "
            f"{api_dir.relative_to(root)}. Gate is a no-op until at least one "
            "API $def declares a kernel-source mirror pointer."
        )
        return 0

    print(
        f"ADR 0082 D-13 gate 6 OK: {total_mirrors} API mirror(s) byte-aligned "
        f"with kernel/governance/AI sources under "
        f"{schemas_dir.relative_to(root)}/wos-*.schema.json."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
