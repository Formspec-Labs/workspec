#!/usr/bin/env python3
"""Annotate open string leaves reported by schema_string_leaf_report with x-wos.openStringKind (+ minLength).

Reads CSV from: cargo run -p wos-lint --example schema_string_leaf_report -- <schema> --csv
Mutates the JSON schema in place. Run from wos-spec/ (workspace root for wos-lint)."""

from __future__ import annotations

import csv
import io
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def workspace_root() -> Path:
    return Path(__file__).resolve().parents[1]


def leaf_report_csv(rel_schema: str) -> list[tuple[str, str]]:
    root = workspace_root()
    out = subprocess.check_output(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "wos-lint",
            "--example",
            "schema_string_leaf_report",
            "--",
            rel_schema,
            "--csv",
        ],
        cwd=root,
        text=True,
    )
    lines = out.strip().splitlines()
    if not lines:
        return []
    header_idx = next(i for i, ln in enumerate(lines) if ln.startswith("pointer,"))
    r = csv.reader(io.StringIO("\n".join(lines[header_idx:])))
    header = next(r)
    pi = header.index("pointer")
    si = header.index("description_snippet")
    rows: list[tuple[str, str]] = []
    for row in r:
        if len(row) <= max(pi, si):
            continue
        rows.append((row[pi], row[si]))
    return rows


def resolve_ptr(doc: Any, pointer: str) -> Any:
    if pointer in ("", "/"):
        return doc
    parts = pointer.lstrip("/").split("/")
    cur: Any = doc
    for raw in parts:
        tok = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(cur, list):
            cur = cur[int(tok)]
        else:
            cur = cur[tok]
    return cur


def infer_kind(pointer: str, snippet: str) -> str:
    s = snippet.lower()
    tail = pointer.split("/")[-1].replace("~1", "/").replace("~0", "~")

    if "fel" in s or tail == "expression":
        return "fel"
    if "$schema" in pointer or tail in ("schemaRef", "specRef") or "json schema uri" in s:
        return "uri"
    if "jsonpath" in s or "json pointer" in s or "jsonpath" in tail.lower():
        return "pathExpression"
    if tail == "path" and ("jsonpath" in s or "starts with '$.'" in s):
        return "pathExpression"
    if "sha256" in s or "merkle" in s:
        return "hash"
    if "iso 8601" in s or "timestamp" in s or "time-keyed" in s:
        return "timestamp"
    if tail in (
        "stateBefore",
        "stateAfter",
        "expectedStateAfter",
        "sourceState",
        "targetState",
        "kernelVersion",
        "since",
        "deprecatedSince",
    ):
        return "tagLabel"
    if tail == "event" and "guard" in pointer:
        return "tagLabel"
    if tail == "name" and ("event" in pointer or "tool" in pointer):
        return "tagLabel"
    if tail in (
        "message",
        "description",
        "summary",
        "hint",
        "attempt",
        "document",
        "last_attempt",
        "suggested_fix",
    ):
        return "prose"
    if tail in ("from", "to", "ruleId", "rule_id", "guardId", "policyId", "sourceActor", "fixtureId", "seam"):
        return "identifier"
    if "identifier" in s and "human" not in s:
        return "identifier"
    if "uri" in s and ("http" in s or "https://" in s or "namespace" in s):
        return "uri"
    if "free-form" in s or "human-readable" in s or "prose" in s:
        return "prose"
    return "identifier"


def annotate_node(node: dict[str, Any], kind: str) -> None:
    if node.get("type") != "string":
        return
    xw = node.get("x-wos")
    if not isinstance(xw, dict):
        xw = {}
    if xw.get("openStringKind"):
        return
    xw = {**xw, "openStringKind": kind}
    node["x-wos"] = xw
    node.setdefault("minLength", 1)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: annotate_open_string_kinds.py <workspace-relative schema path>", file=sys.stderr)
        return 2
    rel = sys.argv[1]
    root = workspace_root()
    path = root / rel
    rows = leaf_report_csv(rel)
    with path.open(encoding="utf-8") as f:
        doc = json.load(f)
    for ptr, snip in rows:
        node = resolve_ptr(doc, ptr)
        if not isinstance(node, dict):
            continue
        kind = infer_kind(ptr, snip)
        annotate_node(node, kind)
    with path.open("w", encoding="utf-8") as f:
        json.dump(doc, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
