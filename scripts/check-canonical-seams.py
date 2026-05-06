#!/usr/bin/env python3
"""Enforce ADR 0077 D-3: reject invented seam identifiers in normative WOS surfaces.

Per ADR 0077, the canonical six kernel extension seams are:
  - actorExtension
  - contractHook
  - provenanceLayer
  - lifecycleHook
  - custodyHook
  - extensions (and `x-` keys)

Five names that previously appeared in `work-spec/CLAUDE.md` were freestanding
fictions with no Kernel §10 backing and have been retired (ADR 0077 D-3):
  - attachmentExtension
  - caseFieldExtension
  - eventExtension
  - outcomeExtension
  - sidecarExtension

This script scans normative WOS surfaces for any reappearance of those names
and fails CI when found. Scope per ADR 0077 lint-rule-candidate section:
  - Markdown files under `work-spec/specs/**`
  - JSON Schema files under `work-spec/schemas/**` (any text content; in
    practice only `description` and `$comment` strings would carry seam
    names, but flagging property keys catches schema-shape regressions too).

Out of scope (intentionally not scanned):
  - `thoughts/adr/0077-*.md` — the ADR itself enumerates and disposes of the
    invented names; that is the authoritative discussion.
  - `work-spec/counter-proposal-disposition.md` — disposition prose
    references invented names in its `Seam vocabulary drift` section
    (resolved-by-ADR-0077 marker).
  - `work-spec/CLAUDE.md` history; only the current `work-spec/specs/**` and
    `work-spec/schemas/**` text counts.

Usage:
  python3 work-spec/scripts/check-canonical-seams.py [--root WOS_SPEC_DIR]

Exits 0 on clean tree, 1 on any violation. Prints `file:line: identifier`
diagnostics on stderr; clean run prints a single OK line on stdout.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

INVENTED_SEAM_NAMES = (
    "attachmentExtension",
    "caseFieldExtension",
    "eventExtension",
    "outcomeExtension",
    "sidecarExtension",
)

# Single regex with word-boundary anchors so e.g. `caseFieldExtensionHistory`
# would not match. We expect identifier-shaped occurrences only.
PATTERN = re.compile(r"\b(" + "|".join(INVENTED_SEAM_NAMES) + r")\b")


def scan_file(path: Path) -> list[tuple[int, str, str]]:
    """Return [(line_number, identifier, line_text), ...] for any hits."""
    hits: list[tuple[int, str, str]] = []
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        print(f"warning: could not read {path}: {exc}", file=sys.stderr)
        return hits
    for lineno, line in enumerate(text.splitlines(), start=1):
        match = PATTERN.search(line)
        if match:
            hits.append((lineno, match.group(1), line.rstrip()))
    return hits


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Root of the wos-spec tree (default: parent of scripts/).",
    )
    args = parser.parse_args()

    root: Path = args.root
    spec_root = root / "specs"
    schema_root = root / "schemas"

    if not spec_root.is_dir() or not schema_root.is_dir():
        print(
            f"error: expected specs/ and schemas/ under {root}",
            file=sys.stderr,
        )
        return 2

    targets: list[Path] = []
    targets.extend(sorted(spec_root.rglob("*.md")))
    targets.extend(sorted(schema_root.rglob("*.json")))

    any_violation = False
    for path in targets:
        for lineno, ident, snippet in scan_file(path):
            any_violation = True
            rel = path.relative_to(root)
            print(
                f"{rel}:{lineno}: invented seam '{ident}' (ADR 0077 D-3): {snippet}",
                file=sys.stderr,
            )

    if any_violation:
        print(
            "ADR 0077 violation: one or more invented seam identifiers appear in "
            "normative wos-spec surfaces. The canonical six are actorExtension, "
            "contractHook, provenanceLayer, lifecycleHook, custodyHook, and "
            "extensions (with `x-` keys). Rewrite the offending lines or, if a new "
            "seam is genuinely needed, ratify it via a follow-up ADR amending Kernel §10 "
            "before adding the name back.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: scanned {len(targets)} files under {spec_root.relative_to(root)} + "
        f"{schema_root.relative_to(root)}; canonical six seams hold."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
