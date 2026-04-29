# WOS Facts-Tier Input Snapshot — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make case-file input snapshot MANDATORY (and typed) on every Facts-tier provenance record emitted at a `determination`-tagged transition. The kernel's §8.2 "Facts Tier Record" table currently lists `inputs: object | OPTIONAL`. Today no fixture populates it, no schema enforces it, and no runtime path supplies it. Adverse-decision provenance is therefore reconstructible only via outside tooling — a structural failure mode for the audit chain (the §4.1.2 deterministic adverse-decision notice can't reproduce the exact input set the original determination saw).

**Why this is the prize:** §4.1.2 (Imp 9 deterministic adverse-decision notice) requires the snapshot. Without it, the dual-form notice cannot be deterministic — different runs of the assembly algorithm against the same workflow run can read different "current" case-file values. With it, the notice's "factual basis" section is byte-stable.

**Architecture:** Two complementary changes — one normative (the spec promises the snapshot), one mechanical (the runtime emits it). Migration of ~20 in-tree fixtures (`grep -rln '"determination"' fixtures/` returned 20 paths on 2026-04-18 — TODO's "51" count includes nested fixtures and conformance documents). Schema-level enforcement is via a JSON Schema `if/then` over Facts-tier records when `record_kind` is `StateTransition` AND the carrying transition's tags include `determination`.

**Spec anchor:** Kernel §8.2 (Facts Tier Record). Cross-cutting: §4.1.2 (depends on this), §4.1.23 OverrideRecord (already typed; OverrideRecord references can name this snapshot via `overriddenDecisionRef`), §4.2 #29a Milestone trigger-mode (independent — both this and triggerMode are runtime-policy reifications).

---

## Prerequisites

- `wos-runtime` source for transition emission (`crates/wos-runtime/src/runtime.rs` and `crates/wos-core/src/event_handler.rs`) is the place the snapshot must be inserted. Read `runtime.rs:1-100` to understand the transition emission cycle.
- `crates/wos-core/src/provenance.rs:248-316` — `ProvenanceRecord` already has `inputs: Vec<String>` (PROV-O entity references — different concept). The new snapshot is a STRUCTURED case-file capture, not an entity reference list.
- `wos-core` `KernelDocument` type carries `transitions[*].tags` — confirm via `crates/wos-core/src/model/kernel.rs`.

## Completion criteria

1. Spec §8.2 amended: case-file snapshot REQUIRED on Facts-tier records emitted at transitions tagged `determination`. Snapshot shape is typed (per item 2 below).
2. Typed shape published as a $def in a kernel-tier schema (likely `wos-kernel.schema.json` or a new `wos-provenance.schema.json` if Facts-tier records get their own schema).
3. Runtime change: the transition-emission path inspects the firing transition's tags; when `determination` is present, the runtime captures the current case-file state into `ProvenanceRecord.data.caseFileSnapshot` (or equivalent typed field).
4. New conformance rule (e.g., `K-DET-001` or `K-FACTS-001`) in `crates/wos-conformance/src/rules/`: every StateTransition Facts-tier record emitted at a determination-tagged transition MUST carry a non-empty caseFileSnapshot.
5. Lint rule (Tier 2, e.g., `K-DET-002`) in `crates/wos-lint/src/rules/`: when a workflow declares `tags: ["determination"]` on a transition, the document MAY validate; the runtime-side rule above is the load-bearing check.
6. ~20 in-tree fixtures migrated. Each fixture that contains `"determination"` either:
   - declares `expected_provenance` with `caseFileSnapshot` populated (where the fixture is a conformance fixture), OR
   - is unaffected (where the fixture is a kernel/governance/AI workflow document — no provenance shape is asserted).
7. CI gates green: `schema_doc_zero_regression`, `every_promoted_rule_has_evidence`, workspace tests, Python schema tests.
8. §4.1.2 (the Imp 9 prize) is unblocked — its plan can now reference `caseFileSnapshot` as the deterministic input source.

## File structure

- **Modify:** `specs/kernel/spec.md` — §8.2 prose + table.
- **Modify:** `schemas/kernel/wos-kernel.schema.json` — add `FactsTierRecord` $def OR (cleaner) split into `schemas/kernel/wos-provenance-record.schema.json` + reference from kernel.
- **Modify:** `crates/wos-core/src/provenance.rs` — `ProvenanceRecord` gains `case_file_snapshot: Option<serde_json::Value>` (or typed shape).
- **Modify:** `crates/wos-runtime/src/runtime.rs` (or `event_handler.rs`) — at the transition-emission site, snapshot case state when the firing transition carries `determination`.
- **Create:** `crates/wos-conformance/src/rules/facts_tier_snapshot.rs` (or extend existing rules file).
- **Create:** `crates/wos-conformance/tests/facts_tier_snapshot_test.rs` — happy path + missing-snapshot negative case.
- **Create:** `tests/schemas/test_facts_tier_snapshot.py` — pytest mirror covering schema-level enforcement.
- **Modify:** ~20 fixtures under `fixtures/`. Walk in order: `fixtures/conformance/expected-traces/` first (these are the ones the conformance harness validates against), then `fixtures/governance/` and `fixtures/ai/` for any with embedded expected-provenance arrays.
- **Modify:** `LINT-MATRIX.md` — register the new rule(s).

---

## Task 1: Spec change

**Files:** `specs/kernel/spec.md`

- [ ] **1.1** Update §8.2 table: change `inputs` row to `inputs.caseFileSnapshot` and mark REQUIRED at determination-tagged transitions, OPTIONAL elsewhere. Cite `transitions[*].tags` per Kernel §4 as the trigger.
- [ ] **1.2** New §8.2.1 paragraph: "Snapshot semantics. When a transition tagged `determination` fires, the processor MUST capture the current case-file state into `Facts.inputs.caseFileSnapshot` immediately before any post-transition action runs. The snapshot is byte-stable: identical case state at fire time MUST produce byte-identical snapshots. The snapshot MUST be canonicalized via JCS (RFC 8785) so signed exports remain reproducible."
- [ ] **1.3** Cross-ref §4.1.2 (Governance §3.2 adverse-decision notice) and #4.1.23 OverrideRecord — both consumers of the snapshot.
- [ ] **1.4** Commit. `docs(kernel): Facts-tier input snapshot REQUIRED at determination transitions (§8.2)`.

## Task 2: Schema

**Files:** `schemas/kernel/wos-kernel.schema.json` (or new file)

- [ ] **2.1** Decide: extend kernel schema with `FactsTierRecord` $def, OR split provenance shape into `schemas/kernel/wos-provenance-record.schema.json`. Recommend split — provenance is its own document type for export, and giving it a schema mirrors how `wos-conformance-trace.schema.json` and `wos-lint-diagnostic.schema.json` are structured.
- [ ] **2.2** Schema design: `inputs` is an object with optional `caseFileSnapshot` field; the snapshot is `{ value: <opaque JSON>, jcsCanonical: string, sha256: string }`. JCS + SHA fields make the snapshot tamper-evident and link to PROV-O `inputDigest` (already on `ProvenanceRecord`).
- [ ] **2.3** Add `if/then` (or per-record `oneOf`) so records carrying `record_kind: StateTransition` AND `tags` containing `"determination"` REQUIRE `inputs.caseFileSnapshot`. JSON Schema can express this with conditional subschemas.
- [ ] **2.4** Schema-doc gate: every new field needs description ≥ 60 chars + examples.
- [ ] **2.5** Commit. `feat(kernel): typed FactsTierRecord with required caseFileSnapshot at determinations (§8.2)`.

## Task 3: Rust model

**Files:** `crates/wos-core/src/provenance.rs`

- [ ] **3.1** Extend `ProvenanceRecord` with optional `case_file_snapshot: Option<CaseFileSnapshot>` field. Use `serde(default, skip_serializing_if = "Option::is_none")` so existing fixtures roundtrip.
- [ ] **3.2** Define `CaseFileSnapshot { value: serde_json::Value, jcs_canonical: String, sha256: String }`. Implement constructor `from_case_state(state: &serde_json::Value)` that performs JCS canonicalization + SHA-256.
- [ ] **3.3** `cargo nextest run -p wos-core` green. Backfill the existing `Milestone` `trigger_mode` pattern — every existing ProvenanceRecord constructor needs the new field added with `None`.
- [ ] **3.4** Commit. `feat(wos-core): ProvenanceRecord.case_file_snapshot with JCS canonicalization`.

## Task 4: Runtime emission

**Files:** `crates/wos-runtime/src/runtime.rs` (or `crates/wos-core/src/event_handler.rs` — confirm via dependency direction)

- [ ] **4.1** Locate the transition emission site (search `ProvenanceKind::StateTransition`). Read 100 lines around it to understand pre/post-transition state hooks.
- [ ] **4.2** Add: at emission time, inspect the firing transition's `tags`. If `tags.contains("determination")`, populate `record.case_file_snapshot = Some(CaseFileSnapshot::from_case_state(&pre_transition_case_state))`.
- [ ] **4.3** Pre-transition case state, NOT post-transition. The snapshot represents what the determination was MADE FROM, not what it produced.
- [ ] **4.4** New unit test in `crates/wos-runtime/tests/`: drive a kernel doc with one determination transition, assert the emitted record carries a snapshot whose JCS canonical form matches the expected case state.
- [ ] **4.5** Commit. `feat(wos-runtime): emit case-file snapshot on determination transitions (§8.2)`.

## Task 5: Conformance rule

**Files:** `crates/wos-conformance/src/rules/facts_tier_snapshot.rs`, `crates/wos-conformance/src/rules/mod.rs`

- [ ] **5.1** New rule `K-DET-001` (or whatever id is next available — check the registry): walk all StateTransition records in the conformance trace; for each, look up the firing transition in the workflow; if its tags include `determination`, assert `case_file_snapshot.is_some()`.
- [ ] **5.2** Register in `crates/wos-conformance/src/coverage.rs` evidence map.
- [ ] **5.3** Conformance fixture `K-DET-001-determination-snapshot.json` under `fixtures/conformance/expected-traces/` — happy path: a kernel doc with one determination transition + an `expected_provenance` array containing the snapshot.
- [ ] **5.4** Negative fixture (or unit test) covering: rule fails when snapshot absent.
- [ ] **5.5** `cargo nextest run --workspace` green; `every_promoted_rule_has_evidence` CI gate green.
- [ ] **5.6** Add row to `LINT-MATRIX.md`.
- [ ] **5.7** Commit. `feat(conformance): K-DET-001 — Facts-tier snapshot REQUIRED at determinations`.

## Task 6: Fixture migration

**Files:** ~20 paths under `fixtures/`

- [ ] **6.1** Inventory: `grep -rln '"determination"' fixtures/` (returned 20 paths on 2026-04-18). Classify each:
  - Workflow document (just declares the workflow — no provenance assertion). Migration: NONE.
  - Conformance fixture with `expected_provenance` array. Migration: add `caseFileSnapshot` to every determination-transition record.
  - Other (audit fixture, AI fixture). Investigate per-file.
- [ ] **6.2** Migrate the conformance subset first; run `cargo nextest run -p wos-conformance` after each batch to catch breaking changes early.
- [ ] **6.3** When a fixture's `expected_provenance` array is missing entirely from a determination-bearing fixture, add it (or accept the fixture as a structural-only document).
- [ ] **6.4** Single commit per logical batch (kernel fixtures, governance fixtures, AI fixtures).

## Task 7: Python schema test

**Files:** `tests/schemas/test_facts_tier_snapshot.py`

- [ ] **7.1** Mirror `tests/schemas/test_override_record_shape.py`. Cases:
  - Determination-tagged StateTransition without snapshot → REJECTED.
  - Determination-tagged StateTransition with snapshot → ACCEPTED.
  - Non-determination StateTransition without snapshot → ACCEPTED (snapshot is optional outside determinations).
  - Snapshot with malformed sha256 → REJECTED.
- [ ] **7.2** `python3 -m pytest tests/ -q` green.
- [ ] **7.3** Commit. `test: pytest contract for Facts-tier snapshot enforcement`.

## Task 8: Unblock §4.1.2 + close TODO

- [ ] **8.1** Update TODO.md: §4.1.2's "Dependencies" line drops #24a; only structural blockers (#23 + NoticeTemplate, both done) remain.
- [ ] **8.2** Append an entry to COMPLETED.md.

---

## Self-review checklist

- Spec amendment lands (Task 1).
- Typed shape published with conditional REQUIRED (Task 2).
- Rust model carries the field (Task 3).
- Runtime emits at the right moment, against pre-transition state (Task 4).
- Conformance rule + fixture in place (Task 5).
- ~20 fixtures migrated; grep returns zero "determination" + zero `case_file_snapshot` mismatches (Task 6).
- Python schema contract enforces in addition to runtime (Task 7).
- §4.1.2 plan can now reference `caseFileSnapshot` deterministically.

## Estimated effort

~3–5 engineer-days. Most of the cost is fixture migration; the spec/schema/runtime changes are small but each gates the next.
