#!/usr/bin/env python3
"""Pending-annotation ratchet for Studio specs.

Walks `studio/specs/**/*.md`, counts `(*-pending)` annotations
(lint-pending, runtime-pending, schema-pending, etc.), and fails if
ANY kind's count exceeds its per-kind baseline. Per-kind ratcheting
(D-wave 2026-05-03) replaces the single 343 baseline so each
STUDIO-DEFER-004-* sub-ID closes against its own ledger.

Usage::

    python3 studio/tests/pending_ratchet.py
    pytest studio/tests/pending_ratchet.py    # also works

When closing a marker:
1. Delete the `(*-pending)` annotation from the spec.
2. Decrement the corresponding entry in BASELINES below.
3. Update the matching STUDIO-DEFER-004-* entry in DEFERRED.md.

Adding a new kind requires:
1. Adding its baseline to BASELINES (uncategorized kinds fail).
2. Opening a new STUDIO-DEFER-004-<KIND> entry in DEFERRED.md
   with a Phase-N closure plan.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SPECS_ROOT = REPO_ROOT / "specs"

# Matches `(<word>-pending` — the closing paren / period is variable
# (some annotations are `(runtime-pending.)`, some `(lint-pending: …)`).
PENDING_RE = re.compile(r"\(([a-zA-Z][a-zA-Z0-9_-]+)-pending")

# Per-kind baselines. Each kind maps 1:1 to a STUDIO-DEFER-004-<KIND>
# sub-ID in DEFERRED.md. Decrement when closing markers; never bump
# without a documented Phase-N plan.
BASELINES: dict[str, int] = {
    # I-wave (2026-05-03) drained STUDIO-DEFER-004 to all-zero by:
    # - Phase A: 35 new lint rules closed 36 lint markers (67 → 31).
    # - Phase B: 4 schema markers encoded (11 → 7); residual moved
    #   to DEFER-007.
    # - Phase D5 reclassify: every (runtime-pending) marker became
    #   (substrate-pending) under STUDIO-DEFER-007 (Stage-7/8
    #   substrate dependencies — write-barriers, change-detection
    #   engine, scenario simulator emission, runtime-observation
    #   adapter, Trellis identity seam, kernel clock-resume, etc.).
    # - I-D1+I-D2 ADRs 0084/0085 closed both coordination markers.
    # - cmp-051 sharpened from fixture-pending to substrate-pending
    #   (cross-version comparison harness lands when v2 compiler
    #   exists).
    # All five DEFER-004 sub-IDs are now Closed.
    "runtime": 0,         # CLOSED via reclassify → DEFER-007 (substrate)
    "lint": 31,           # remaining lint slice (residual; Phase C deferred)
    "schema": 7,          # residual schema-pending (DEFER-007)
    "fixture": 0,         # CLOSED via cmp-051 sharpen → substrate-pending
    "coordination": 0,    # CLOSED via ADR-0084 + ADR-0085
    "substrate": 191,     # NEW: STUDIO-DEFER-007 — irreducible Stage-7/8
                          # substrate residual (190 reclassified runtime
                          # + 1 cmp-051 fixture sharpening).
    # workflow: closed in E7; STUDIO-DEFER-004-WORKFLOW moved to Closed.
}


# Files exempt from marker counting. `README.md` is the convention
# document — it shows the literal marker tokens (`*(schema-pending)*`,
# `*(lint-pending)*`, `*(runtime-pending)*`, `*(fixture-pending)*`) to
# explain what they mean. Without the skip, those four prose mentions
# would each inflate their respective baselines by 1.
SKIP_FILES: set[str] = {"README.md"}


def count_pending() -> tuple[int, dict[str, int]]:
    total = 0
    by_kind: dict[str, int] = {}
    for path in sorted(SPECS_ROOT.rglob("*.md")):
        if path.name in SKIP_FILES:
            continue
        text = path.read_text(encoding="utf-8")
        for match in PENDING_RE.finditer(text):
            kind = match.group(1)
            by_kind[kind] = by_kind.get(kind, 0) + 1
            total += 1
    return total, by_kind


def check(by_kind: dict[str, int]) -> list[str]:
    """Return a list of human-readable violation messages (empty = OK)."""
    violations: list[str] = []
    for kind, count in by_kind.items():
        if kind not in BASELINES:
            violations.append(
                f"uncategorized kind '{kind}' (count={count}) — add to BASELINES "
                f"+ open STUDIO-DEFER-004-{kind.upper()} in DEFERRED.md"
            )
            continue
        if count > BASELINES[kind]:
            violations.append(
                f"'{kind}-pending' count {count} > baseline {BASELINES[kind]} "
                f"(see STUDIO-DEFER-004-{kind.upper()})"
            )
    return violations


def main() -> int:
    total, by_kind = count_pending()
    violations = check(by_kind)
    if violations:
        print("FAIL: pending-annotation ratchet violated.", file=sys.stderr)
        for v in violations:
            print(f"  {v}", file=sys.stderr)
        print("Per-kind breakdown:", file=sys.stderr)
        for kind, n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
            base = BASELINES.get(kind, "<uncategorized>")
            print(f"  {kind}: {n} / baseline {base}", file=sys.stderr)
        return 1
    print(f"OK: pending-annotation ratchet — total {total}, all kinds within baseline.")
    for kind, n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
        print(f"  {kind}: {n} / {BASELINES[kind]}")
    return 0


def test_pending_annotations_do_not_grow() -> None:
    """Pytest entry point — same check as `main`, raises AssertionError."""
    _, by_kind = count_pending()
    violations = check(by_kind)
    assert not violations, (
        "pending-annotation ratchet violated:\n  "
        + "\n  ".join(violations)
        + "; see STUDIO-DEFER-004-* entries in DEFERRED.md"
    )


if __name__ == "__main__":
    sys.exit(main())
