# WOS Typed Event Meta-Vocabulary — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan closes TODO §4.1 #20 — the kernel's last load-bearing openness.

**Goal.** Replace the free-form `Transition.event: string` slot (and its co-dependant `Action.event` for `startTimer`) with a strict, tagged five-kind union: `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. Each kind carries its own typed payload. No `named` wrapper, no escape hatch. Closes the last structural openness in the Kernel spec and normatively adopts the BPMN event-taxonomy subset flagged in `IDEA_SCRATCH.md` / TODO.

**Why this is the prize.** The kernel's lifecycle is a pure function `(active states × event × guards) → next states`. Today the *event* half of that tuple is untyped; every consumer re-derives the taxonomy by prefix-sniffing (`starts_with('$')`). A typed union puts the taxonomy in the schema, in the Rust model, and in every tool that speaks kernel JSON. Cost to land now: ~8–10 engineer-days. Cost after #32 / #51 / #F3b land on untyped events: a week of rework per downstream item.

**Spec anchors.** Kernel §4.5 (`specs/kernel/spec.md:170-179`), §4.6 (`:181-189`), §4.8 fork/join (`:200-206`), §4.9 (`:208-210`), §4.10 kernel-generated events (`:212-232`), §9.7 timeout categories (`:507-520`). BPMN normative-adoption note at `thoughts/specs/2026-04-18-wos-kernel-resilience-sweep.md:77`.

**Architecture.** Reshape is local to `Transition` and `Action` (for `startTimer`). Case-state, actor, milestone, provenance schemas untouched. Taxonomy is closed — no extension on `kind`; per-kind extension lives on payload `x-*` fields. `$join`, `$timeout.*`, `$related.*`, `$error`, `$compensation.complete` become either (a) structured authorships under the right `kind` or (b) engine-synthesized artifacts never authored.

---

## Prerequisites

- Read `specs/kernel/spec.md:170-232` — §4.5 through §4.10 — before writing any code. This is the section the plan normatively amends.
- Read `crates/wos-core/src/model/kernel.rs:296-321` (current `Transition`) and `:323-417` (current `Action`). These are the two types being reshaped.
- Read `crates/wos-core/src/eval.rs:388-584` — the `process_event` + `try_fire_transition` + `try_fire_in_parallel` code path. Every `transition.event != event` comparison changes shape.
- Read `crates/wos-lint/src/rules/tier1.rs:133-169` — K-006/K-007/K-008 transition checks. K-007 (`$` prefix rejection) is demoted to schema validation; K-008 (parallel state requires `$join`) must be rewritten to check `kind == "signal" && name == "$join"` or whatever §4.8 resolves to per Open Question 1.
- Read `crates/wos-lint/src/rules/continuous_mode.rs:766-870` — K-049 treats `"event": "$continuous"` as a cycle sentinel. Confirm this is engine-synthesised-only (never authored) so it needs no schema representation.
- Read `thoughts/plans/2026-04-18-wos-facts-tier-input-snapshot.md` as a style reference for task structure and acceptance criteria.

## Completion criteria

1. `schemas/kernel/wos-kernel.schema.json` defines `$defs/TransitionEvent` as a discriminated union on `kind` with five branches and is referenced from `Transition.event` and `Action.event` (the `startTimer` field).
2. `Transition.event` and `Action.event` normatively use typed objects; the reference deserializer MAY still accept a legacy bare string and coerce it to `TransitionEvent` for migration.
3. `crates/wos-core/src/model/kernel.rs` uses `pub event: Option<TransitionEvent>` on `Transition` (optional) and `Option<TransitionEvent>` on `Action` for `startTimer`, with `#[serde(tag = "kind", rename_all = "lowercase")]` on the enum and explicit `#[serde(rename = "timerId")]` / camelCase field aliases on variant fields so JSON matches the schema.
4. Every in-tree fixture under `fixtures/` and `crates/wos-conformance/{fixtures,tests/fixtures}/` is migrated to the typed form. 185 files; 844 `"event":` occurrences in authored kernel bodies.
5. `cargo test --workspace` and `python3 -m pytest tests/ -q` green.
6. `npm run docs:check` (if applicable for this repo — replace with local equivalent `make docs` / schema-doc checker) green; SCHEMA-DOC-001 green on the new `TransitionEvent` $def.
7. Spec prose amended: §4.5 table (event row now links to `TransitionEvent`), §4.6 (resolution now matches on `(kind, name)` or `(kind, discriminant)`), §4.10 table (kernel-generated events now described in terms of `kind` + named constants rather than `$`-prefixed strings).
8. `LINT-MATRIX.md` updated: K-007 promoted from `draft` to `schema` (the schema now enforces the taxonomy structurally).
9. K-008 (parallel-state transition must use `$join`) rewritten and passing its existing test.

## File structure

- **Modify:** `schemas/kernel/wos-kernel.schema.json` — add `$defs/TransitionEvent` + 5 branch defs; reshape `$defs/Transition.properties.event` and `$defs/Action.properties.event`.
- **Modify:** `specs/kernel/spec.md` — §4.5 table, §4.6 resolution, §4.8 $join framing, §4.10 kernel-generated events table.
- **Modify:** `crates/wos-core/src/model/kernel.rs` — introduce `TransitionEvent` + `TimerEventSource` + `SignalScope`; replace `Transition.event` and `Action.event`.
- **Modify:** `crates/wos-core/src/eval.rs:388-584`, `:269`, `:814` — match on typed form.
- **Modify:** `crates/wos-core/src/event_handler.rs:20,274`; `crates/wos-core/src/project.rs:92`; `crates/wos-runtime/src/runtime.rs:574,602,688,1118-1128`; `crates/wos-runtime/src/companion.rs:155-272,400,511` — call-site migration.
- **Modify:** `crates/wos-lint/src/rules/tier1.rs:149-168` (K-007 delete, K-008 rewrite); `crates/wos-lint/src/rules/tier2.rs:157`; `crates/wos-lint/src/rules/continuous_mode.rs:766-870` (verify `$continuous` synthesized-only).
- **Create:** `scripts/migrate-transition-events.py` (landed name).
- **Modify:** 185 fixtures across `fixtures/` and `crates/wos-conformance/{fixtures,tests/fixtures}/`.
- **Modify:** `LINT-MATRIX.md` — K-007 summary updated (JSON Schema **plus** Tier 1 on the typed model for `$` misuse); K-008 note updated.

---

## Section 1 — Goal + architecture

The kernel lifecycle rests on `(active states, event, guards) → next states` (§4.2). The event half is today a free-form string; four things are un-pinned: **taxonomy** (what kinds trigger transitions — `$`-prefix is convention, not schema), **payload** (`$timeout.slaTimer` and `submit` are the same shape), **separation-of-concerns** (K-007 lint is the only guardrail between authored and kernel-synthesized names), and **forward extensibility** into #32, #51, and continuous-mode #F3b condition triggers.

This plan lands a closed five-kind union — `timer | message | signal | condition | error` — co-located in schema and Rust model. No `named` fallback; no `kind: "custom"`; no `x-kind` prefix. BPMN 2.0's broader catalog (escalation, compensation, cancel, terminate, link, conditional, multiple) maps *into* this taxonomy without adding kinds: compensation is an action, terminate is a cancellation policy, link is a signal, conditional is our `condition`, multiple is an authoring shortcut. BPMN topology is rejected (Harel remains); BPMN event-semantics is adopted normatively via this item.

## Section 2 — Taxonomy design

Four reference taxonomies informed this cut:

- **BPMN 2.0 event catalog** — 11 types. WOS maps 1:1 for `timer / message / signal / error / conditional`; rejects `terminate` (cancellation policy), `compensation` (action), `cancel` (policy), `link` (a signal), `multiple` (authoring shortcut), `escalation` (re-authored as a tagged signal).
- **SCXML event model** — flat, dot-namespaced. No structural help; WOS keeps dots-as-characters inside `name` (OQ2).
- **Current WOS §4.10 kernel-generated events** — distribution in fixtures (from `grep '"event":\s*"\$'`): 19× `$timeout.state`, 18× `$join`, 15× `$timeout.slaTimer`, 3× `$compensation.complete`, 3× `$timeout.regionA`, 2× `$activate`, 2× `$timeout.regionB`, and 1 each of `$migrate`, `$restart`, `$verificationReportProduced`, `$verificationReportModified`. Zero `$related.*` in fixtures — spec'd but never exercised.
- **Current author-defined events** — 127 unique non-`$` names, 844 total occurrences. Message-like: `submit`, `approve`, `analyze`, `verificationComplete`, `appealDenied`. A small slice uses dotted names (`appeal.filed`, `review.completed`).

The five kinds, with payload and rationale:

**`timer`** — `{ kind, timerId, expiresAt?, duration?, source }`. `source: "task" | "service" | "state" | "signal" | "workflow" | "custom"` maps §9.7 timeout categories; `custom` covers author-authored `startTimer` targets like `slaTimer`. Example: `{ "kind": "timer", "timerId": "slaTimer", "duration": "PT4H", "source": "custom" }`. Subsumes `$timeout.*` (all variants), hand-authored timer names. Category collapses into `source` enum; no reserved-name-not-a-kind. Extension: `source: "custom"` + payload `x-*`.

**`message`** — `{ kind, name, correlationKey?, data? }`. `name` is required, MUST NOT start with `$`, MAY be dotted (OQ2). `correlationKey` routes externally-delivered messages. `data` open. Example: `{ "kind": "message", "name": "submit", "correlationKey": "case-123" }`. Subsumes every current author-defined event (127 unique names × 779 non-`$` occurrences).

**`signal`** — `{ kind, name, scope }`. `scope: "instance" | "related" | "broadcast"`. `instance` affects only the current workflow instance; `related` routes to related cases (subsumes today's `$related.*`); `broadcast` delivers to all instances subscribed to the name. Example: `{ "kind": "signal", "name": "stateChanged", "scope": "related" }`. Subsumes `$related.{stateChanged,resolved,holdReleased}`, `$compensation.complete` (as `{ "kind": "signal", "name": "$compensation.complete", "scope": "instance" }` — the runtime `process_event` name includes the `$` prefix). `$join` disposition under OQ1.

**`condition`** — `{ kind, expression }`. `expression` is FEL; evaluated when the processor is in continuous mode and re-evaluated on case-file mutation (§4.3a #F3b). Example: `{ "kind": "condition", "expression": "caseFile.amount > 10000" }`. New capability — no current fixture shape. In event-driven mode, `condition` transitions are inert.

**`error`** — `{ kind, code, actionPath? }`. `code` is dot-namespaced (`contract.violation`, `timeout.exceeded`). `actionPath` is a JSON Pointer populated by the runtime. Example: `{ "kind": "error", "code": "contract.violation", "actionPath": "/states/draft/onEntry/0" }`. Subsumes `$error`. Forward-looking — no current `$error` authorings in fixtures.

**Subsumes:** `$error` (§4.10). Existing fixtures contain zero `$error` authorings (grep returns nothing), so this is schema-forward-looking; the runtime emits it synthetically.

**Reserved-name-not-a-kind:** None.

**Extension:** `code` is an open string; the registry tier (§spec-tier registry, TODO #21) MAY constrain it further.

### Rejected options

- **Six kinds with `compensation`** — rejected. Compensation is an *action* (§9.5); `$compensation.complete` becomes an engine-emitted `signal`.
- **`named` escape hatch / open `kind: "other"`** — rejected per TODO §4.1 #20. Closed discriminant is the whole point.
- **Flat SCXML-style hierarchical strings** — defeats schema enforcement.
- **Vendor-specific `kind` values prefixed `x-`** — see OQ4; default is closed.

## Section 3 — Schema migration

### Before (`schemas/kernel/wos-kernel.schema.json:508-517`)

```json
"event": {
  "type": "string",
  "minLength": 1,
  "description": "Identifier of the event that triggers this transition (Kernel §4.5, §4.6). … Names prefixed with `$` are reserved for kernel-generated events … workflow authors MUST NOT define `$`-prefixed event names.",
  "examples": ["submit", "approve", "reject", "task.completed", "$timeout.task"],
  "x-lm": { "critical": true, "intent": "The event that triggers this transition" }
}
```

### After

```json
"event": { "$ref": "#/$defs/TransitionEvent" }
```

### New `$defs/TransitionEvent`

A closed discriminated union. Top-level shape (abbreviated):

- `type: object`, `required: ["kind"]`.
- `properties.kind` = string enum `["timer", "message", "signal", "condition", "error"]`, `x-lm.critical: true`, description ≥140 chars spanning what each kind means.
- `oneOf` referencing five branch $defs: `TransitionEventTimer / Message / Signal / Condition / Error`.

Each branch is `type: object`, `additionalProperties: false` (except `message.data` which is open), `properties.kind: { const: "<kind>" }`, plus the kind's required payload fields. Example for `TransitionEventTimer`:

- `required: ["kind", "timerId", "source"]`.
- `timerId`: string, minLength 1, description cites §9.2/§9.7, examples `["slaTimer", "reviewDeadline"]`.
- `expiresAt`: string/date-time, mutually exclusive with `duration` via top-level `oneOf`.
- `duration`: ISO 8601 duration.
- `source`: enum `["task", "service", "state", "signal", "workflow", "custom"]`.

`TransitionEventMessage`: `required ["kind", "name"]`, `name` pattern forbidding leading `$` (closes K-007 structurally), optional `correlationKey`, open `data`. `TransitionEventSignal`: `required ["kind", "name", "scope"]`, same `name` pattern, `scope` enum. `TransitionEventCondition`: `required ["kind", "expression"]`, `expression` is FEL. `TransitionEventError`: `required ["kind", "code"]`, optional `actionPath` (JSON Pointer). Every leaf ≥140-char description + ≥2 `examples` per SCHEMA-DOC-001.

### `Action.event` (co-change)

`Action.event` (`schemas/kernel/wos-kernel.schema.json:652-657`) is the event the `startTimer` action fires *when its timer expires*. It becomes a `TransitionEventTimer` — the only sane kind a timer can fire. Simpler: it can become a `$ref` to `TransitionEventTimer` directly, dropping the `kind` field because it's fixed. Either shape is acceptable; the plan proposes full `TransitionEvent` for symmetry and future-proofing (allowing `startTimer` to fire a `signal` or `message` is a small future capability that doesn't cost anything to admit here).

## Section 4 — Rust model migration

### New types in `crates/wos-core/src/model/kernel.rs`

`TransitionEvent` is an internally tagged enum: `#[serde(tag = "kind", rename_all = "lowercase")]` so JSON uses `"kind": "timer"` / `"message"` / etc. Variant fields use Rust `snake_case` with `#[serde(rename = "timerId")]` (and similar) where JSON is camelCase. Supporting enums `TimerEventSource` and `SignalScope` use `rename_all = "camelCase"` on their **variant** names in JSON (`task`, `instance`, …). Helpers: `from_legacy_string`, `runtime_dispatch_label`, `matches_runtime_dispatch`, `start_timer_fires_string`.

`Transition.event` is `Option<TransitionEvent>` (optional transition trigger). `Action.event` is `Option<TransitionEvent>` for `startTimer`, practically always the `Timer` variant.

### Call-site migration list (grep-verified)

| File | Line(s) | Current | After |
|---|---|---|---|
| `crates/wos-core/src/model/kernel.rs` | 301, 380 | `event: String`, `event: Option<String>` | typed enum |
| `crates/wos-core/src/eval.rs` | 458, 529 | `if transition.event != event { continue; }` | `if !transition.event.matches_runtime(event) { continue; }` — see Runtime Event Bus note below |
| `crates/wos-core/src/eval.rs` | 269 | `fires_event: timer.event.clone()` | `fires_event: timer.event.clone()` (still `TransitionEvent`; downstream readers access `.timer_id`) |
| `crates/wos-core/src/eval.rs` | 814 | `let fires_event = action.event.as_deref().unwrap_or("")` | match on `action.event` → `Timer { timer_id, .. }` |
| `crates/wos-core/src/event_handler.rs` | 20, 274 | `pub event: String` on `AdverseDecisionNoticeInput` | keep `String`; this is a *runtime observation*, not an authored shape. Document the boundary. |
| `crates/wos-core/src/project.rs` | 92 | `events.insert(transition.event.as_str())` | `events.insert(transition.event.runtime_dispatch_label())` (collection uses `String` set) |
| `crates/wos-runtime/src/runtime.rs` | 574, 602, 688, 1118-1128 | `event.event` (inbound-event struct name) | inbound-event struct keeps `String`; boundary is clear (authored schema is typed, runtime inbox is still name-string until a later item reshapes it) |
| `crates/wos-runtime/src/companion.rs` | … | `transition.event == event_name` string compare | `matches_runtime_dispatch` / typed comparison |
| `crates/wos-lint/src/rules/tier1.rs` | 149-168 | K-007 `$`-prefix check; K-008 parallel-state transition shape | K-007 deleted (now schema); K-008 rewritten against typed form |
| `crates/wos-lint/src/rules/tier2.rs` | 157 | `events.insert(transition.event.clone())` | `runtime_dispatch_label()` |
| `crates/wos-lint/src/rules/continuous_mode.rs` | 766-870 | K-049 `$continuous` sentinel | no change if `$continuous` is engine-synthesized-only and never authored; verify and add a test ensuring the schema rejects an authored `"event": "$continuous"` |

**Runtime Event Bus boundary.** The runtime receives inbound events as names (strings); the kernel JSON is typed. Authored form is typed; the runtime's event-inbox struct (`InboundEvent { event: String, ... }`) stays untyped for now. Deliberate minimal blast radius — reshape authored schema here, reshape the inbox in a follow-up plan.

## Section 5 — Fixture migration

**Scale.** Running `grep -rl '"event"' fixtures/ crates/wos-conformance/{fixtures,tests/fixtures}/` returns **185 files** holding **844** `"event":` string occurrences (with 964 across conformance + docs dirs). Of the 844, **65** are `$`-prefixed (distribution in §2); the remaining **779** are author-defined names.

### Per-category migration rule

| Current shape | New shape | Notes |
|---|---|---|
| `"event": "submit"` (or any non-`$` name) | `{ "kind": "message", "name": "submit" }` | 779 occurrences. Mechanical. |
| `"event": "$timeout.task"` | `{ "kind": "timer", "timerId": "<synthesized>", "source": "task" }` | `timerId` recovered from the `startTimer` action pairing in the same document, or synthesized as `"task-timeout-${stateId}"` if no pairing is found (must be reviewed manually — there are few). |
| `"event": "$timeout.state"` (19) | `{ "kind": "timer", "timerId": "<paired>", "source": "state" }` | Pair with the authoring state's `stateTimeout`. |
| `"event": "$timeout.slaTimer"` (15) | `{ "kind": "timer", "timerId": "slaTimer", "source": "custom" }` | The suffix *is* the timerId; the category is `custom`. |
| `"event": "$timeout.regionA"` / `regionB` (5) | `{ "kind": "timer", "timerId": "regionA", "source": "custom" }` | Same pattern. |
| `"event": "$join"` | `{ "kind": "signal", "scope": "instance", "name": "$join" }` OR dropped from authored form — see Open Question 1. | 18 occurrences. |
| `"event": "$compensation.complete"` (3) | `{ "kind": "signal", "scope": "instance", "name": "$compensation.complete" }` | Keep the `$` prefix on `name` so the typed transition matches the processor-delivered event string after §9.5 compensation completes. |
| `"event": "$related.stateChanged"` (0 in fixtures) | `{ "kind": "signal", "scope": "related", "name": "stateChanged" }` | Forward-looking; runtime path not yet exercised. |
| `"event": "$activate"` / `$restart` / `$migrate` (4 total) | case-by-case; likely `signal` | Out-of-spec names not listed in §4.10. Flag for human review during migration. |
| `"event": "$verificationReportProduced"` / `Modified` (2) | `{ "kind": "message", "name": "verificationReportProduced" }` | These are author-defined but misuse the `$` prefix (K-007 violations that slipped in). Drop prefix. |
| `"event": "$continuous"` | engine-synthesized only; never in authored fixtures | Verify the 3 occurrences in `continuous_mode.rs` tests are in *test harness* JSON, not authored documents. |

### Migration script sketch (`scripts/migrate-events-to-typed.py`)

~50 LOC Python. Walks every JSON file under the given roots, recursively descends all dict/list nodes, and for each `{"event": "<string>"}` occurrence calls `classify(event) -> dict` per the table above. Unclassifiable events (`$activate`, `$restart`, `$migrate`, `$verificationReport*`) cause the script to exit non-zero and leave the file untouched for manual migration. Emits `migration-manifest-{before,after}.json` with every `(path, json-pointer, old-name, new-shape)` triple for diff review. Re-serialises each file with stable JSON formatting (2-space indent, preserving key order via `sort_keys=False`). Owns no state beyond the classification table; idempotent on re-runs if the fixture is already in the typed form.

**Manual review checkpoints (estimated 10–15 files):**

- All 3 `$compensation.complete` occurrences (verify the `signal` scope call).
- All 4 non-§4.10 `$`-prefixed names (`$activate`, `$restart`, `$migrate`) — decide kind.
- All 2 misuse cases (`$verificationReport*`) — verify re-authorship as `message`.
- Every parallel-state (`kind: Parallel`) transition with `"event": "$join"` — verify K-008 still passes after the rewrite (`has > 10` such sites per fixture survey).

**Before/after digest manifest.** Before the script runs, emit `migration-manifest-before.json` mapping every fixture path → list of every `"event": "<name>"` occurrence with its JSON pointer. After the script runs, emit `migration-manifest-after.json` with the new shapes. Diff the two to verify no event silently disappeared or duplicated. This is the load-bearing check for Section 9's "silent semantic drift" risk.

## Section 6 — Lint rule migration

### K-007 — schema + Tier 1 on typed `TransitionEvent`

The schema's `message` `name` pattern forbids a leading `$`, and `signal` `name` allows only `$join` and `$compensation.complete` with a `$` prefix. Tier 1 additionally flags typed `TransitionEvent::Message` whose `name` starts with `$`, and typed `TransitionEvent::Signal` whose `name` starts with `$` but is not one of those two sentinels — so invalid shapes that bypass JSON Schema (e.g. hand-edited documents) still surface as K-007. `crates/wos-lint/tests/tier1_rules.rs` uses typed `{ "kind": "message", "name": "$…" }` fixtures for the negative case (a legacy bare string coerces to `message` without `$` and would not trip K-007).

### K-008 — parallel-state transition shape

Currently: `if state.kind == StateKind::Parallel && transition.event != "$join"` → K-008 error. After migration, depending on Open Question 1:

- **If `$join` remains in authored form** as `{kind: "signal", scope: "instance", name: "$join"}`: K-008 becomes `if state.kind == StateKind::Parallel && !matches!(&transition.event, TransitionEvent::Signal { name, .. } if name == "$join")`.
- **If `$join` is engine-synthesized-only** and never appears in authored documents: K-008 is deleted or inverted — the lint now asserts that a `Parallel` state MAY NOT declare any outgoing transition (the processor synthesizes the join signal and fires its own transition). This is the cleaner end-state and is the plan's recommended resolution of OQ1.

### K-049 — continuous-mode cycle detector

No changes needed. K-049 matches on transition *shape* (guards referencing the same case-file fields each transition writes). Event kind is irrelevant to the detection logic. The existing `$continuous` engine-synthesized sentinel stays an engine-only concept; after this change the sentinel is synthesized as `TransitionEvent::Condition { expression: ... }` when matching continuous-mode transitions, or as a special-cased runtime tag. Confirm no authored document ever uses `$continuous` (grep fixtures: none).

### K-017 / K-019 — FEL guard-reference rules

No changes. They analyse guard expressions, not events.

### New schema-tier guard (replaces K-007 bunching)

Add a JSON Schema test in `tests/schemas/test_transition_event_typed.py`: for each of ~15 invalid shapes (untagged union; wrong kind; missing required payload fields; $-prefixed `name` in `message` / `signal`; unknown `source` on `timer`), assert `jsonschema.validate` raises.

## Section 7 — Ordered task list

Ten tasks, ~1 engineer-day each (~8–10 engineer-days total).

### Task 1 — Schema reshape (schema-only)

- **Files:** `schemas/kernel/wos-kernel.schema.json`.
- **LOC:** +220 / −12.
- **Acceptance:** SCHEMA-DOC-001 green (every leaf has description ≥140 chars + ≥2 examples). No Rust code changes yet — the Rust model still takes `String`, so this task alone will break `cargo test` (expected). Land it in a commit that is explicitly labelled "schema-only; Rust migration follows in Task 2."
- **Dependencies:** Open Questions 1 + 4 resolved.

### Task 2 — Rust `TransitionEvent` enum

- **Files:** `crates/wos-core/src/model/kernel.rs`.
- **LOC:** +80 / −2.
- **Acceptance:** `cargo build -p wos-core` succeeds. Compilation errors cascade through `eval.rs`, `runtime.rs`, `companion.rs`, lint rules — these are the next tasks. Commit in a single unit with a header explaining the cascade.
- **Dependencies:** Task 1.

### Task 3 — `eval.rs` + `event_handler.rs` migration

- **Files:** `crates/wos-core/src/eval.rs:388-584,269,814,1011`; `crates/wos-core/src/event_handler.rs:20,274`; `crates/wos-core/src/project.rs:92`.
- **LOC:** +120 / −60. (Dominated by matching on 5 kinds per call site.)
- **Acceptance:** `cargo test -p wos-core` green (after fixtures are migrated in Task 6, or with migrated test fixtures inlined in the test module in this task).
- **Dependencies:** Task 2.

### Task 4 — `wos-runtime` migration

- **Files:** `crates/wos-runtime/src/runtime.rs:574,602,688,1118-1128`; `crates/wos-runtime/src/companion.rs:155-272,400,511`.
- **LOC:** +140 / −50.
- **Acceptance:** `cargo test -p wos-runtime` green (with fixtures migrated in Task 6).
- **Dependencies:** Task 3.

### Task 5 — `wos-lint` migration

- **Files:** `crates/wos-lint/src/rules/tier1.rs:149-168`; `crates/wos-lint/src/rules/tier2.rs:157`; `crates/wos-lint/src/rules/continuous_mode.rs` (tests); `crates/wos-lint/tests/tier1_rules.rs:443-507,1894-1906`.
- **LOC:** +40 / −60 (K-007 deletion, K-008 rewrite).
- **Acceptance:** `cargo test -p wos-lint` green.
- **Dependencies:** Task 3.

### Task 6 — Fixture migration (scripted)

- **Files:** 185 fixture files; new `scripts/migrate-events-to-typed.py`.
- **LOC:** ~50 in the script; fixture diff is mechanical (~1600 line changes total across 185 files).
- **Acceptance:** `migration-manifest-before.json` vs `migration-manifest-after.json` diff shows only shape rewrites (no name deletions or additions). 10–15 fixtures manually reviewed and hand-tweaked (log each in the commit body). Commit in ≤5 grouped commits by tier: (a) `fixtures/kernel`, (b) `fixtures/conformance`, (c) `fixtures/governance + fixtures/ai`, (d) `fixtures/validation + fixtures/profiles + fixtures/sidecars`, (e) `crates/wos-conformance/{fixtures,tests/fixtures}`.
- **Dependencies:** Task 1 (schema must accept the new shape before fixtures adopt it).

### Task 7 — Full test suite

- `cargo test --workspace` and `python3 -m pytest tests/ -q`.
- **Acceptance:** Green across both. Any Python schema regression tests that assert the old string shape are updated.
- **Dependencies:** Tasks 1–6.

### Task 8 — LINT-MATRIX regen

- **Files:** `LINT-MATRIX.md`.
- **Acceptance:** K-007 row status → `schema`; K-008 row note updated.

### Task 9 — K-007 promotion in registry

- **Files:** `crates/wos-lint/src/rules/registry.rs:857`; `LINT-MATRIX.md`.
- **Acceptance:** Registry record reflects K-007 as `Tested` (the schema is the authority; registry records the transition from draft).

### Task 10 — Spec prose

- **Files:** `specs/kernel/spec.md`.
- **Sections:** §4.5 (Transitions table — `event` row now references `TransitionEvent`); §4.6 (Transition resolution — match now on `(kind, discriminant)` rather than string); §4.8 ($join framing — depends on OQ1); §4.10 (Kernel-Generated Events table — each row restated in terms of `{kind, ...}` rather than `$`-prefixed string).
- **LOC:** +90 / −40.
- **Acceptance:** `specs/kernel/spec.llm.md` regenerates cleanly. No dangling `$`-prefix references in normative prose; any remaining mentions are historical / migration-note only.
- **Dependencies:** Task 6.

## Section 8 — Open questions

1. **$join disposition** — Does `$join` remain in authored form (as `{kind: "signal", scope: "instance", name: "$join"}` or `"join"` — a reserved signal name) or is it engine-synthesized-only (never appears in any authored document; the processor synthesizes and fires its own transition when a parallel join condition is met)? The cleaner answer is engine-synthesized-only, which means the schema rejects any `Parallel` state with outgoing transitions (K-008 inverts); the author writes no join machinery and the processor handles it. 18 fixtures today author `$join` explicitly and would change shape. **REQUIRED TO RESOLVE BEFORE TASK 1** — the schema shape depends on it.

2. **Dotted `signal.name` and `message.name`** — Should names support SCXML-style hierarchy (`order.received.urgent`) with structural meaning (prefix-subscribe), or is a dotted name just a naming convention with no structural semantics? Current fixtures use both flat (`submit`) and dotted (`appeal.filed`, `review.completed`). The plan proposes: dotted names are *allowed* as a convention, with no structural prefix-matching semantics at the kernel tier; Registry-tier constructs (§spec-tier registry, TODO #21) MAY add prefix-subscribe behavior. **CAN DEFER** — the default is "dots are just characters"; registry-tier sugar can be added later without re-shaping.

3. **`condition.expression` evaluation context** — Does the expression evaluate against the full current evaluation context (§7) including `caseFile`, `event`, `actor`, `instance`, `now`? Or a narrower "data-change only" context that excludes `event` (since there *is* no triggering event)? The plan proposes: full §7 context minus `event` (the `event` slot is undefined — the trigger is the data mutation, not an event). **CAN DEFER** — can be resolved during §4.3a #F3b implementation; the schema doesn't need to encode the context shape.

4. **Vendor-specific `kind` values** — Do vendor extensions use `x-`-prefixed kinds (e.g., `kind: "x-mycorp-escalation"`), or does the `kind` discriminant stay closed and vendors extend via payload `x-*` fields (e.g., `{kind: "signal", name: "escalate", "x-mycorp-policy": "..."}` — payload extension on a spec-defined kind)? The plan proposes: **closed `kind` discriminant; no vendor kinds ever**. Vendor extension lives on payload via `x-*` patternProperties, just like Action's existing `^x-` extension point. **REQUIRED TO RESOLVE BEFORE TASK 1** — affects the schema's `kind` enum openness.

## Section 9 — Risk register

- **Fixture migration silent semantic drift.** A name could be rewritten to the wrong kind (e.g., `$compensation.complete` → `message` instead of `signal`), breaking a lifecycle expectation that no test currently asserts. **Mitigation:** the before/after manifest diff (§5). Every name is listed with its old shape and new shape; the diff is code-reviewed before the commit is merged. Additionally, Task 7 runs `cargo test --workspace` — any fixture-dependent behaviour test catches a mis-rewrite.
- **BPMN-export compatibility** (`wos-bpmn-export`, TODO §4). The reshape *aligns* with BPMN's native event taxonomy; it does not misalign. If anything it makes the export simpler (no need to re-infer kinds from `$`-prefixes). Risk: low; no blocker. Flag a recheck when the BPMN-export crate is sketched.
- **Rollback cost.** One-way door at the fixture and Rust-type level. Before Task 6 lands, rollback is a 1-hour revert. After Task 6 lands with migrated fixtures, rollback is a ~1-week hand-migration back to strings. After any downstream consumer (studio, authoring UI, synth crate) adopts the typed form, rollback is ~1-person-month. **Implication:** commit the *schema + model + eval* (Tasks 1–3) on a feature branch, run migration (Task 6) on the branch, validate (Task 7), then merge. Do not stage partial landings to main.
- **Runtime event inbox boundary.** The plan deliberately leaves the runtime's inbound-event struct (`event: String`) untyped — matching happens via the typed `Transition.event` on one side and a name-string on the other. This is a known migration seam that a future plan closes; it must be clearly documented in §4.5 prose and in the companion runtime §S11 so consumers don't assume full end-to-end typing.
- **Studio authoring UI.** The WOS studio's transition editor today edits `event` as a text field. The studio must learn the typed form. Out of scope for this plan (studio is a separate crate tier), but flag as a follow-up item for `#studio` triage.

---

## Decision checklist for user

Please answer before any agent starts Task 1:

- [ ] **OQ1 ($join disposition):** engine-synthesized-only (preferred) OR authored as a `signal`?
- [ ] **OQ2 (dotted names):** flat naming convention (no structural semantics) OR registry-tier prefix-subscribe?
- [ ] **OQ3 (condition context):** full §7 context minus `event` (preferred) OR narrower data-only context?
- [ ] **OQ4 (vendor kinds):** closed `kind`, vendor extension via payload `x-*` (preferred) OR open `kind` with `x-` prefix?

**GO / NO-GO:** this plan is executable once OQ1 and OQ4 are resolved. OQ2 and OQ3 can be deferred to implementation-time. The fixture migration (Task 6) is the heaviest step (185 files, ~1600-line diff) and has the sharpest one-way-door quality; land it last among the Rust-model tasks and gate it behind a green `cargo test --workspace` on a feature branch.
