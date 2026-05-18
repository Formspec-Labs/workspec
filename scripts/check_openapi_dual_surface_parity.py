#!/usr/bin/env python3
"""Wave 7 architecture-review Finding A1 — OpenAPI dual-surface parity gate.

The WOS public API ships two OpenAPI documents that must stay coherent:

  1. `api/wos-public-api.openapi.json` — hand-maintained, broader surface
     (~71 operations). The contract authors ratchet here as new endpoints
     promote into the public release stream.

  2. `api/wos-public-api.registry.openapi.json` — utoipa-emitted from
     `WosPublicApi::paths(...)` in `workspec-server/`. A subset that
     reflects what is currently wired into Rust handlers (~35 operations).

Wave 7 added 423 Locked / `WOS-1423` (case-source lock contention) and
503 Service Unavailable to both files for `submit_case_event` and other
substrate-touching operations. Without a CI gate, the next code-emitted
addition (new response code, new error class) can drift between the two
surfaces — the registry says "this operation returns 423" while the
broader hand-maintained spec still claims only 4xx classes.

This script enforces the parity assertion:

    For every operation that appears in BOTH files (matched by METHOD+PATH,
    not operationId — utoipa emits snake_case while the broader file uses
    camelCase), every response code present in the registry MUST also be
    present in the broader file.

The assertion is one-way (registry => broader): the broader file MAY
declare codes the registry does not (the registry is a subset surface,
and the broader file is the contract). Operations present only in the
broader file are informational — they are scoped out of the current
utoipa registration set.

Operations present only in the REGISTRY but missing from the broader
file ARE flagged: the registry is code-truth; if the implementation
exposes an endpoint, the contract must declare it.

Prior-art note: `oasdiff` (Go, MIT, actively maintained) is the mature
external tool for OpenAPI parity checks and would cover this case via
`oasdiff diff --check-breaking`. We keep this assertion in pure Python
stdlib to avoid a Go install + binary vendor in CI for a closed, ~100-line
assertion set. The script's parity shape mirrors what `oasdiff` would
report for the specific drift class we care about (added/removed response
codes per operation). If the assertion set grows materially, migrate to
`oasdiff` and delete this script.

Match shape: METHOD + PATH (operationId convention differs between files).
Assertion: registry response-code set ⊆ broader response-code set per overlapping op.
Tolerated divergence: description prose (utoipa emits short prose, broader
file emits `$ref: #/components/responses/Problem`); broader-only codes; broader-only ops.

# Known limitation — path-version drift

If the broader file's (METHOD, PATH) evolves (e.g., versioned `/v2/cases/...`)
while the registry retains the prior path, the overlap set drops the operation
entirely and this gate silently loses response-code coverage for that op. Migrate
the registry path in lockstep when bumping the broader file, OR add an
`operationId`-bridge assertion class to this script (currently the assertion is
(METHOD, PATH)-keyed only). When `operationId` conventions converge across the
two surfaces (broader is camelCase, registry is snake_case as of HEAD), the
path-drift bridge becomes a one-liner.

Usage:
    python3 scripts/check_openapi_dual_surface_parity.py [--root WORK_SPEC_DIR]

Exit codes:
    0 — every overlapping operation passes the parity assertion
    1 — at least one operation drifts (registry declares a code the broader
        file does not, or an operation appears in registry but not broader)
    2 — invocation error (missing files, malformed OpenAPI)
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

HTTP_METHODS = frozenset(
    {"get", "post", "put", "delete", "patch", "options", "head", "trace"}
)


def load_ops(path: Path) -> dict[tuple[str, str], dict[str, Any]]:
    """Return `{(METHOD, PATH): operation_object}` for every operation."""
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"error: could not load {path}: {exc}") from exc
    if not isinstance(doc, dict):
        raise SystemExit(f"error: {path} is not a JSON object")
    paths = doc.get("paths")
    if not isinstance(paths, dict):
        raise SystemExit(f"error: {path} has no `paths` object")

    out: dict[tuple[str, str], dict[str, Any]] = {}
    for url, methods in paths.items():
        if not isinstance(methods, dict):
            continue
        for method, body in methods.items():
            if method.lower() not in HTTP_METHODS:
                continue
            if not isinstance(body, dict):
                continue
            out[(method.upper(), url)] = body
    return out


def response_codes(op: dict[str, Any]) -> set[str]:
    """Set of response keys declared on the operation (e.g. `{"200","423","default"}`)."""
    resps = op.get("responses")
    if not isinstance(resps, dict):
        return set()
    return {str(k) for k in resps.keys()}


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
    broader_path = root / "api" / "wos-public-api.openapi.json"
    registry_path = root / "api" / "wos-public-api.registry.openapi.json"

    for p in (broader_path, registry_path):
        if not p.is_file():
            print(f"error: {p} not found", file=sys.stderr)
            return 2

    broader = load_ops(broader_path)
    registry = load_ops(registry_path)

    overlap = sorted(set(broader.keys()) & set(registry.keys()))
    only_registry = sorted(set(registry.keys()) - set(broader.keys()))
    only_broader = sorted(set(broader.keys()) - set(registry.keys()))

    drift_blocks: list[str] = []

    # Drift class 1: operation in registry but absent from broader.
    # Registry = code-truth. If a handler exposes the endpoint, the
    # contract document must declare it.
    for method, url in only_registry:
        reg_op = registry[(method, url)]
        reg_id = reg_op.get("operationId", "<no-operationId>")
        drift_blocks.append(
            f"REGISTRY-ONLY: {method} {url} (operationId={reg_id}) "
            "is wired in WosPublicApi::paths but is missing from the "
            "hand-maintained broader OpenAPI surface."
        )

    # Drift class 2: registry declares a response code the broader file
    # does not. One-way assertion: registry => broader.
    for key in overlap:
        method, url = key
        reg_codes = response_codes(registry[key])
        broad_codes = response_codes(broader[key])
        missing = sorted(reg_codes - broad_codes)
        if missing:
            reg_id = registry[key].get("operationId", "<no-operationId>")
            broad_id = broader[key].get("operationId", "<no-operationId>")
            drift_blocks.append(
                f"CODE-DRIFT: {method} {url} "
                f"(registry={reg_id}, broader={broad_id}) "
                f"declares response codes {missing} in registry that are "
                f"absent from the broader file. Registry codes: "
                f"{sorted(reg_codes)}; broader codes: {sorted(broad_codes)}."
            )

    # Report.
    print(
        f"OpenAPI dual-surface parity — Wave 7 Finding A1 gate\n"
        f"  broader  ({broader_path.name}): {len(broader)} operations\n"
        f"  registry ({registry_path.name}): {len(registry)} operations\n"
        f"  overlap (METHOD+PATH):           {len(overlap)} operations\n"
        f"  broader-only (informational):    {len(only_broader)} operations\n"
        f"  registry-only (DRIFT):           {len(only_registry)} operations\n"
    )

    if drift_blocks:
        print(
            f"FAIL — {len(drift_blocks)} drift finding(s):\n",
            file=sys.stderr,
        )
        for block in drift_blocks:
            print(f"  - {block}", file=sys.stderr)
            print("", file=sys.stderr)
        print(
            "Fix: either remove the code/operation from the registry "
            "(handler change) or add the code/operation to the broader "
            "hand-maintained file (contract change). Do NOT silence the "
            "gate.",
            file=sys.stderr,
        )
        return 1

    print("OK — every registry response code is mirrored in the broader file.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
