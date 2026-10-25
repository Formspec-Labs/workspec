#!/usr/bin/env python3
"""Enforce schema↔enum `recordKind` parity for `ProvenanceKind`.

Closes the failure mode that produced audit Gap 1 (2026-04-28): a JSON
Schema `$def` shipped with a fixed `recordKind` literal but no matching
`ProvenanceKind` variant in Rust, leaving the schema MUST unfulfillable
from the typed Rust path.

**Forward direction (enforced).** For every subschema, at any depth, of any
schema in `work-spec/schemas/` (`$defs`, `properties`, `allOf`, `if`/`then`,
...) whose `properties.recordKind.const` (or single-element `enum`) pins a
literal string, that literal MUST equal the camelCase serialization of some
`ProvenanceKind` variant in
`work-spec/crates/wos-events/src/provenance/kind.rs`. Mismatch fails CI.

**Required pins (enforced).** Every registry kind marked `schemaValidated`
with no `eventLiteral` MUST be pinned by at least one schema. Records
dispatch on `event` (registry `admissionContract.dispatchField`; `recordKind`
is the legacy field), so a kind with an event literal keys its overlay on
`event` and needs no pin, but a kind without one can only key on
`recordKind`. Zero pins is therefore legitimate only when the registry lists
no such kind. This keeps the forward direction from going silently vacuous,
as it did from a318d12d, when overlays moved to `event` dispatch and the
old `$defs`-only walk stopped finding any pin.

**D26 event-literal direction (enforced).** Every `eventLiteral` row in
`schemas/record-kind-registry.json` MUST equal what
`ProvenanceKind::canonical_event_literal()` returns, and every kind it
returns a literal for MUST appear in the registry. Both directions of that
mapping read the `define_canonical_substrate_events!` table in `kind.rs`,
so the table is the Rust source of truth this script parses. This keeps the
registry's admission/event-type surface as a checked mirror of it.

**Reverse direction (informational, intentionally not enforced).** The
script also lists variants that have *no* schema pin binding them. Most
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
Default (no `--strict`) enforces every direction except reverse.

Exits 0 on clean tree, 1 on any enforced violation (or any unbound variant
if `--strict`), 2 on usage error.
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

# The `define_canonical_substrate_events! { ... }` invocation (not its
# `macro_rules!` definition) and its `"wos.layer.event_literal" => Variant,`
# rows, one per line.
CANONICAL_EVENT_TABLE_START = re.compile(
    r"^\s*define_canonical_substrate_events!\s*\{\s*$"
)
CANONICAL_EVENT_TABLE_ROW = re.compile(
    r'^\s*"([^"]+)"\s*=>\s*([A-Z][A-Za-z0-9]*)\s*,\s*$'
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
    """Return camelCase recordKind -> event literal from the
    `define_canonical_substrate_events!` table behind
    `ProvenanceKind::canonical_event_literal()`.

    Every line inside the table must be a row, blank, or a `//` comment. A
    missing, empty, or reshaped table exits here as parser drift, so it is
    never misreported as every registry row being stale.
    """
    lines = kind_rs.read_text(encoding="utf-8").splitlines()
    start = next(
        (i for i, line in enumerate(lines) if CANONICAL_EVENT_TABLE_START.match(line)),
        None,
    )
    if start is None:
        raise SystemExit(
            f"error: no `define_canonical_substrate_events! {{` table in {kind_rs}; "
            "point this parser at whatever canonical_event_literal() reads now."
        )
    mappings: dict[str, str] = {}
    for line in lines[start + 1 :]:
        stripped = line.strip()
        if stripped == "}":
            break
        if not stripped or stripped.startswith("//"):
            continue
        match = CANONICAL_EVENT_TABLE_ROW.match(line)
        if not match:
            raise SystemExit(
                f"error: unparsed line in define_canonical_substrate_events! "
                f"in {kind_rs}: {stripped!r}; expected one "
                '`"wos.layer.event_literal" => Variant,` row per line.'
            )
        event_literal, variant = match.groups()
        mappings[to_camel_case(variant)] = event_literal
    if not mappings:
        raise SystemExit(
            f"error: parsed 0 rows from define_canonical_substrate_events! in {kind_rs}."
        )
    return mappings


def extract_registry_event_literals(registry: dict) -> dict[str, str]:
    """Return recordKind literal -> eventLiteral from the registry JSON."""
    event_literals: dict[str, str] = {}
    for entry in registry.get("recordKinds", []):
        literal = entry.get("literal")
        event_literal = entry.get("eventLiteral")
        if isinstance(literal, str) and isinstance(event_literal, str):
            event_literals[literal] = event_literal
    return event_literals


def extract_registry_pin_required(registry: dict) -> set[str]:
    """Return recordKind literals the registry marks `schemaValidated` with
    no `eventLiteral`. Records dispatch on `event`
    (`admissionContract.dispatchField`), so a kind without an event literal
    can key its payload overlay only on `recordKind`: each needs a pin."""
    return {
        entry["literal"]
        for entry in registry.get("recordKinds", [])
        if entry.get("schemaValidated") is True and "eventLiteral" not in entry
    }


# JSON Schema keywords whose values are instance data, not subschemas.
INSTANCE_VALUE_KEYWORDS = frozenset({"const", "enum", "default", "examples"})


def pinned_record_kind(subschema: dict) -> str | None:
    """Return the literal `subschema` pins `properties.recordKind` to through
    `const` or a one-value `enum`; both are a hard wire-shape pin."""
    properties = subschema.get("properties")
    record_kind = properties.get("recordKind") if isinstance(properties, dict) else None
    if not isinstance(record_kind, dict):
        return None
    const = record_kind.get("const")
    if isinstance(const, str):
        return const
    enum = record_kind.get("enum")
    if isinstance(enum, list) and len(enum) == 1 and isinstance(enum[0], str):
        return enum[0]
    return None


def iter_record_kind_pins(node: object, pointer: str = "") -> Iterable[tuple[str, str]]:
    """Yield (JSON Pointer, literal) for every subschema at any depth that
    pins `recordKind`: under `$defs`, `properties`, `allOf`/`anyOf`/`oneOf`,
    `if`/`then`/`else`, `items`, and the rest. Instance-valued keywords are
    not descended into, because their contents are data, not schemas."""
    if isinstance(node, dict):
        literal = pinned_record_kind(node)
        if literal is not None:
            yield (pointer or "/", literal)
        for key, child in node.items():
            if key not in INSTANCE_VALUE_KEYWORDS:
                escaped = key.replace("~", "~0").replace("/", "~1")
                yield from iter_record_kind_pins(child, f"{pointer}/{escaped}")
    elif isinstance(node, list):
        for index, child in enumerate(node):
            yield from iter_record_kind_pins(child, f"{pointer}/{index}")


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
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    registry_event_literals = extract_registry_event_literals(registry)

    # Collect every (schema_path, JSON Pointer, literal) pin.
    bindings: list[tuple[Path, str, str]] = []
    for schema_path in sorted(schemas_root.rglob("*.json")):
        try:
            schema = json.loads(schema_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"warning: could not parse {schema_path}: {exc}", file=sys.stderr)
            continue
        for pointer, literal in iter_record_kind_pins(schema):
            bindings.append((schema_path, pointer, literal))

    # Forward-direction check: every literal must have a matching variant.
    forward_violations = [
        (schema_path, pointer, literal)
        for schema_path, pointer, literal in bindings
        if literal not in variant_camel
    ]

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

    bound_literals = {literal for (_, _, literal) in bindings}
    unpinned_required = sorted(extract_registry_pin_required(registry) - bound_literals)

    # Reverse-direction report (informational by default).
    unbound_variants = sorted(v for v in variants if to_camel_case(v) not in bound_literals)

    # Print findings.
    if forward_violations:
        print(
            "FAIL: schema `recordKind` literals without a matching ProvenanceKind variant:",
            file=sys.stderr,
        )
        for schema_path, pointer, literal in forward_violations:
            rel = schema_path.relative_to(root)
            print(
                f"  {rel}#{pointer}: literal {literal!r} is not a "
                f"ProvenanceKind variant under serde(rename_all=\"camelCase\"). "
                f"Add the variant to {kind_rs.relative_to(root)}, classify "
                f"it in audit_tier.rs, or remove the `recordKind` pin.",
                file=sys.stderr,
            )

    if unpinned_required:
        print(
            f"FAIL: {len(bindings)} schema `recordKind` pins found, but these "
            "registry kinds are schemaValidated with no eventLiteral, so their "
            "payload overlay can only key on `recordKind` and no schema pins it:",
            file=sys.stderr,
        )
        for literal in unpinned_required:
            print(
                f"  {literal}: restore its schema `recordKind` pin, or correct "
                "its registry schemaValidated/eventLiteral fields.",
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
            "variants have no schema pinning `recordKind` to their "
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
        or unpinned_required
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
        "canonical_event_literal() rows."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
