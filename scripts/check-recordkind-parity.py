#!/usr/bin/env python3
"""Enforce schema↔enum `recordKind` parity for `ProvenanceKind`.

Closes the failure mode that produced audit Gap 1 (2026-04-28): a JSON
Schema `$def` shipped with a fixed `recordKind` literal but no matching
`ProvenanceKind` variant in Rust, leaving the schema MUST unfulfillable
from the typed Rust path.

**Forward direction (enforced).** For every `$def` under any schema in
`work-spec/schemas/` whose `properties.recordKind.const` (or single-element
`enum`) pins a literal string, that literal MUST equal the camelCase
serialization of some `ProvenanceKind` variant in
`work-spec/crates/wos-events/src/provenance/kind.rs`. Mismatch fails CI.

**D26 event-literal direction (enforced).** Every `eventLiteral` row in
`schemas/record-kind-registry.json` MUST equal the matching
`ProvenanceKind::canonical_event_literal()` match arm, and every match arm
MUST appear in the registry. This keeps the registry's admission/event-type
surface as a checked mirror of the Rust source of truth.

**Reverse direction (informational, intentionally not enforced).** The
script also lists variants that have *no* schema `$def` binding them. Most
variants are runtime-emitted without a schema-pinned record shape today, so
enforcing reverse-direction parity would require a per-kind `$def` build-out
that is out of scope for this lint. The list is printed for visibility so
future schema work can incrementally close the surface; cf. TODO.md
Hygiene #68's deferred scope note.

Mirrors the surface of `scripts/check-canonical-seams.py` (ADR 0077).

Usage:
  python3 work-spec/scripts/check-recordkind-parity.py [--root WOS_SPEC_DIR]
                                                       [--strict]

`--strict` upgrades reverse-direction informational warnings to errors.
Default (no `--strict`) only enforces forward direction.

Exits 0 on clean tree, 1 on any forward-direction violation (or any
violation if `--strict`), 2 on usage error.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Iterable

# Match `Variant,` lines inside `pub enum ProvenanceKind { ... }`. Doc
# comments and tier-section markers begin with `///` or `//` and are
# skipped. Variants are PascalCase identifiers ending in `,`.
VARIANT_LINE = re.compile(r"^\s*([A-Z][A-Za-z0-9]*)\s*,\s*$")

# Match `Self::Variant => Some("wos.layer.event_literal"),` lines inside
# `ProvenanceKind::canonical_event_literal`.
CANONICAL_EVENT_LITERAL_LINE = re.compile(
    r'^\s*Self::([A-Z][A-Za-z0-9]*)\s*=>\s*Some\("([^"]+)"\),\s*$'
)

# `pub enum ProvenanceKind {` start line.
ENUM_START = re.compile(r"^\s*pub\s+enum\s+ProvenanceKind\s*\{")


def to_camel_case(pascal: str) -> str:
    """Apply `serde(rename_all = "camelCase")`: lowercase the first letter,
    preserve subsequent characters. Mirrors serde's rule for unit variants:
    `StateTransition` → `stateTransition`, `IntakeAccepted` → `intakeAccepted`.
    """
    if not pascal:
        return pascal
    return pascal[0].lower() + pascal[1:]


def extract_variants(kind_rs: Path) -> list[str]:
    """Return all `ProvenanceKind` variant names in declaration order."""
    text = kind_rs.read_text(encoding="utf-8")
    variants: list[str] = []
    in_enum = False
    depth = 0
    for line in text.splitlines():
        if not in_enum:
            if ENUM_START.search(line):
                in_enum = True
                depth = line.count("{") - line.count("}")
            continue
        depth += line.count("{") - line.count("}")
        if depth <= 0:
            break
        match = VARIANT_LINE.match(line)
        if match:
            variants.append(match.group(1))
    if not variants:
        raise SystemExit(
            f"error: parsed 0 variants from {kind_rs}; the regex assumes "
            "`pub enum ProvenanceKind { ... }` with one PascalCase identifier "
            "per line. Fix the parser or restructure the enum."
        )
    return variants


def extract_canonical_event_literals(kind_rs: Path) -> dict[str, str]:
    """Return camelCase recordKind -> event literal from Rust source."""
    text = kind_rs.read_text(encoding="utf-8")
    mappings: dict[str, str] = {}
    for line in text.splitlines():
        match = CANONICAL_EVENT_LITERAL_LINE.match(line)
        if match:
            variant, event_literal = match.groups()
            mappings[to_camel_case(variant)] = event_literal
    return mappings


def extract_registry_event_literals(registry_path: Path) -> dict[str, str]:
    """Return recordKind literal -> eventLiteral from the registry JSON."""
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    event_literals: dict[str, str] = {}
    for entry in registry.get("recordKinds", []):
        literal = entry.get("literal")
        event_literal = entry.get("eventLiteral")
        if isinstance(literal, str) and isinstance(event_literal, str):
            event_literals[literal] = event_literal
    return event_literals


def iter_def_record_kind_consts(
    schema: dict, schema_path: Path
) -> Iterable[tuple[str, str]]:
    """Yield (def_name, literal) for each `$def` that pins `recordKind` to a
    fixed string via `properties.recordKind.const` or single-element
    `enum`. Walks `$defs` recursively.

    A `$def` may pin the literal in either:
      - `properties.recordKind.const = "literal"`
      - `properties.recordKind.enum = ["literal"]` (single element)

    Both are treated as a hard wire-shape pin.
    """

    def walk(defs: dict, prefix: str = "") -> Iterable[tuple[str, str]]:
        for name, body in defs.items():
            if not isinstance(body, dict):
                continue
            qualified = f"{prefix}{name}" if prefix else name

            # Dive into nested `$defs` (rare, but valid JSON Schema).
            nested = body.get("$defs")
            if isinstance(nested, dict):
                yield from walk(nested, prefix=f"{qualified}/")

            properties = body.get("properties")
            if not isinstance(properties, dict):
                continue
            record_kind = properties.get("recordKind")
            if not isinstance(record_kind, dict):
                continue

            # Direct const.
            const_val = record_kind.get("const")
            if isinstance(const_val, str):
                yield (qualified, const_val)
                continue

            # Single-element enum.
            enum_val = record_kind.get("enum")
            if isinstance(enum_val, list) and len(enum_val) == 1:
                literal = enum_val[0]
                if isinstance(literal, str):
                    yield (qualified, literal)
                    continue

            # if/then guards: many schemas pin `recordKind` inside
            # `allOf[].if.properties.recordKind.const` to fire conditional
            # `then` requirements. Capture those too — they're equivalent
            # to declaring a record-shape $def with that recordKind, just
            # expressed as a constraint instead of a positional $def.
        # Walk allOf branches at this level for if/then recordKind pins.

    # Walk allOf entries that pin recordKind via if-blocks. This catches the
    # `CapabilityInvocationRecord`-style `allOf[0].if.properties.recordKind.const`
    # pattern even when the discriminator is hidden inside a guard.
    def walk_if_pins(
        node: dict, anchor: str = "<root>"
    ) -> Iterable[tuple[str, str]]:
        if not isinstance(node, dict):
            return
        all_of = node.get("allOf")
        if isinstance(all_of, list):
            for branch in all_of:
                if not isinstance(branch, dict):
                    continue
                if_block = branch.get("if")
                if isinstance(if_block, dict):
                    if_props = if_block.get("properties")
                    if isinstance(if_props, dict):
                        rk = if_props.get("recordKind")
                        if isinstance(rk, dict):
                            const_val = rk.get("const")
                            if isinstance(const_val, str):
                                yield (anchor, const_val)
                            enum_val = rk.get("enum")
                            if (
                                isinstance(enum_val, list)
                                and len(enum_val) == 1
                                and isinstance(enum_val[0], str)
                            ):
                                yield (anchor, enum_val[0])

    defs = schema.get("$defs")
    if isinstance(defs, dict):
        yield from walk(defs)
        for def_name, def_body in defs.items():
            if isinstance(def_body, dict):
                yield from walk_if_pins(def_body, anchor=def_name)

    # Top-level if/then is rare but possible.
    yield from walk_if_pins(schema, anchor="<top>")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Root of the wos-spec tree (default: parent of scripts/).",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help=(
            "Upgrade reverse-direction warnings (variants without schema "
            "binding) to errors. Default lints forward direction only."
        ),
    )
    args = parser.parse_args()

    root: Path = args.root
    schemas_root = root / "schemas"
    kind_rs = root / "crates" / "wos-events" / "src" / "provenance" / "kind.rs"
    registry_path = schemas_root / "record-kind-registry.json"

    if not schemas_root.is_dir():
        print(f"error: schemas/ not found under {root}", file=sys.stderr)
        return 2
    if not kind_rs.is_file():
        print(f"error: kind.rs not found at {kind_rs}", file=sys.stderr)
        return 2
    if not registry_path.is_file():
        print(f"error: record-kind registry not found at {registry_path}", file=sys.stderr)
        return 2

    variants = extract_variants(kind_rs)
    variant_camel = {to_camel_case(v) for v in variants}
    canonical_event_literals = extract_canonical_event_literals(kind_rs)
    registry_event_literals = extract_registry_event_literals(registry_path)

    # Collect every (schema_path, def_name, literal) triple.
    bindings: list[tuple[Path, str, str]] = []
    for schema_path in sorted(schemas_root.rglob("*.json")):
        try:
            schema = json.loads(schema_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"warning: could not parse {schema_path}: {exc}", file=sys.stderr)
            continue
        if not isinstance(schema, dict):
            continue
        for def_name, literal in iter_def_record_kind_consts(schema, schema_path):
            bindings.append((schema_path, def_name, literal))

    # Forward-direction check: every literal must have a matching variant.
    forward_violations: list[tuple[Path, str, str]] = []
    for schema_path, def_name, literal in bindings:
        if literal not in variant_camel:
            forward_violations.append((schema_path, def_name, literal))

    missing_registry_events = sorted(
        (literal, event_literal)
        for literal, event_literal in canonical_event_literals.items()
        if literal not in registry_event_literals
    )
    stale_registry_events = sorted(
        (literal, event_literal)
        for literal, event_literal in registry_event_literals.items()
        if literal not in canonical_event_literals
    )
    mismatched_registry_events = sorted(
        (literal, event_literal, canonical_event_literals[literal])
        for literal, event_literal in registry_event_literals.items()
        if literal in canonical_event_literals
        and canonical_event_literals[literal] != event_literal
    )

    # Reverse-direction report (informational by default).
    bound_literals = {literal for (_, _, literal) in bindings}
    unbound_variants = sorted(v for v in variants if to_camel_case(v) not in bound_literals)

    # Print findings.
    if forward_violations:
        print(
            "FAIL: schema `recordKind` literals without a matching ProvenanceKind variant:",
            file=sys.stderr,
        )
        for schema_path, def_name, literal in forward_violations:
            rel = schema_path.relative_to(root)
            print(
                f"  {rel} $defs/{def_name}: literal {literal!r} is not a "
                f"ProvenanceKind variant under serde(rename_all=\"camelCase\"). "
                f"Add the variant to {kind_rs.relative_to(root)}, classify "
                f"it in audit_tier.rs, or remove the $def's `recordKind` pin.",
                file=sys.stderr,
            )

    if missing_registry_events:
        print(
            "FAIL: canonical_event_literal() rows missing from record-kind registry:",
            file=sys.stderr,
        )
        for literal, event_literal in missing_registry_events:
            actual = registry_event_literals.get(literal)
            print(
                f"  {literal}: Rust expects {event_literal!r}, registry has {actual!r}",
                file=sys.stderr,
            )

    if stale_registry_events:
        print(
            "FAIL: record-kind registry eventLiteral rows not sourced by canonical_event_literal():",
            file=sys.stderr,
        )
        for literal, event_literal in stale_registry_events:
            print(f"  {literal}: registry eventLiteral {event_literal!r}", file=sys.stderr)

    if mismatched_registry_events:
        print(
            "FAIL: record-kind registry eventLiteral rows disagree with canonical_event_literal():",
            file=sys.stderr,
        )
        for literal, actual, expected in mismatched_registry_events:
            print(
                f"  {literal}: registry has {actual!r}, Rust expects {expected!r}",
                file=sys.stderr,
            )

    if unbound_variants:
        msg = (
            f"INFO: {len(unbound_variants)} of {len(variants)} ProvenanceKind "
            "variants have no schema $def pinning `recordKind` to their "
            "camelCase literal. Most variants are runtime-emitted without a "
            "schema-pinned record shape — this is informational, not a "
            "failure. Variants without binding (sample of up to 10):"
        )
        sample = unbound_variants[:10]
        details = "\n".join(f"  {v}" for v in sample)
        suffix = (
            f"\n  ...and {len(unbound_variants) - 10} more"
            if len(unbound_variants) > 10
            else ""
        )
        print(f"{msg}\n{details}{suffix}")

    if (
        forward_violations
        or missing_registry_events
        or stale_registry_events
        or mismatched_registry_events
    ):
        return 1
    if args.strict and unbound_variants:
        print(
            "FAIL (--strict): reverse-direction parity requires every "
            f"ProvenanceKind variant to have a schema binding; "
            f"{len(unbound_variants)} variants are unbound.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: scanned {len(bindings)} schema recordKind bindings against "
        f"{len(variants)} ProvenanceKind variants under "
        f"{schemas_root.relative_to(root)}; forward-direction parity holds. "
        f"Registry event literals match {len(canonical_event_literals)} "
        "canonical_event_literal() arms."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
