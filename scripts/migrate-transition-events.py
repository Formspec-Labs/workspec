#!/usr/bin/env python3
"""Migrate kernel Transition.event and startTimer Action.event from strings to typed TransitionEvent objects (TODO #20).

Walks lifecycle.states recursively (including regions) and transition.actions.
Idempotent: leaves already-typed objects unchanged.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def transition_event_from_string(s: str) -> dict:
    s = s.strip()
    if s == "$join":
        return {"kind": "signal", "name": "$join", "scope": "instance"}
    if s == "$error":
        return {"kind": "error", "code": "kernel.error"}
    if s.startswith("$timeout."):
        rest = s.removeprefix("$timeout.")
        fixed = ("task", "service", "state", "signal", "workflow")
        if rest in fixed:
            return {"kind": "timer", "timerId": rest, "source": rest}
        return {"kind": "timer", "timerId": rest, "source": "custom"}
    if s.startswith("$related."):
        name = s.removeprefix("$related.")
        return {"kind": "signal", "name": name, "scope": "related"}
    if s == "$compensation.complete":
        return {"kind": "signal", "name": "$compensation.complete", "scope": "instance"}
    if s.startswith("$"):
        return {"kind": "message", "name": s.lstrip("$")}
    return {"kind": "message", "name": s}


def start_timer_event_from_string(s: str, timer_id: str | None) -> dict:
    s = s.strip()
    tid = (timer_id or "timer").strip()
    if s.startswith("$timeout."):
        rest = s.removeprefix("$timeout.")
        mapping = {"task", "service", "state", "signal", "workflow"}
        if rest in mapping:
            return {"kind": "timer", "timerId": rest, "source": rest}
        return {"kind": "timer", "timerId": rest, "source": "custom"}
    return {"kind": "timer", "timerId": tid, "source": "custom", "firesAs": s}


def migrate_actions(actions: object) -> None:
    if not isinstance(actions, list):
        return
    for a in actions:
        if not isinstance(a, dict):
            continue
        if a.get("action") == "startTimer" and isinstance(a.get("event"), str):
            a["event"] = start_timer_event_from_string(a["event"], a.get("timerId"))
        ca = a.get("compensatingAction")
        if isinstance(ca, dict):
            migrate_actions([ca])


def migrate_state(state: dict) -> None:
    migrate_actions(state.get("onEntry"))
    migrate_actions(state.get("onExit"))
    for t in state.get("transitions") or []:
        if not isinstance(t, dict):
            continue
        ev = t.get("event")
        if isinstance(ev, str):
            st = ev.strip()
            if not st:
                del t["event"]
            else:
                t["event"] = transition_event_from_string(st)
        migrate_actions(t.get("actions"))
    for sub in (state.get("states") or {}).values():
        if isinstance(sub, dict):
            migrate_state(sub)
    for reg in (state.get("regions") or {}).values():
        if not isinstance(reg, dict):
            continue
        for st in (reg.get("states") or {}).values():
            if isinstance(st, dict):
                migrate_state(st)


def migrate_document(root: dict) -> bool:
    """Return True if the document looks like a kernel doc with lifecycle."""
    if not isinstance(root, dict):
        return False
    if root.get("$wosKernel") != "1.0" and "$wosKernel" not in root:
        # Some fixtures wrap kernel under "kernel" — skip unless lifecycle at top
        pass
    lc = root.get("lifecycle")
    if not isinstance(lc, dict):
        return False
    for st in (lc.get("states") or {}).values():
        if isinstance(st, dict):
            migrate_state(st)
    return True


def migrate_file(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    try:
        data = json.loads(text)
    except json.JSONDecodeError as e:
        raise SystemExit(f"{path}: invalid JSON: {e}") from e
    if not isinstance(data, dict):
        return
    if not migrate_document(data):
        return
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    roots = [Path(p) for p in sys.argv[1:]] if len(sys.argv) > 1 else []
    if not roots:
        print("usage: migrate-transition-events.py <dir|file>...", file=sys.stderr)
        raise SystemExit(2)
    for root in roots:
        if root.is_file():
            migrate_file(root)
        else:
            for path in sorted(root.rglob("*.json")):
                migrate_file(path)


if __name__ == "__main__":
    main()
