# WOS Spec §1 Reference Implementation Blockers — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every checkbox in §1 of `wos-spec/TODO.md` — binding-backed S15 conformance, version-pin enforcement, hierarchical history semantics, milestone firing, business calendar SLA integration, and the six remaining Integration Profile binding kinds — so that WOS can honestly claim "supported in reference runtime" before external engine bindings begin.

**Architecture:** Work happens in the `wos-spec` git submodule (`/Users/mikewolfd/Work/formspec/wos-spec`) across five crates: `wos-core` (typed models, state), `wos-runtime` (execution), `wos-formspec-binding` (coprocessor adapter), `wos-conformance` (fixture harness), `wos-lint` (static analysis). Every slice follows red-green-refactor with new conformance fixtures landing before implementation where the observable behavior is subtle (history, milestones, CloudEvents). Commits land in the submodule; the parent `formspec` repo is bumped once per slice so main-branch bisect stays meaningful.

**Tech Stack:** Rust 2021 workspace, `serde`/`serde_json` for document IO, JSON Schema fixtures under `crates/*/fixtures/` and `wos-spec/fixtures/`, `cargo test -p <crate>` for per-crate runs, `cargo test --workspace` for the full suite.

---

## Ground-Truth Reference (from 2026-04-14 survey)

**Relevant anchor points, verified to exist today:**

- `ConformanceBinding` stub: `crates/wos-conformance/src/engine.rs:478-527` — zero-cost adapter, hardcodes `pin_match: true` and `envelope_valid: true`.
- `StubValidator`: `crates/wos-conformance/src/stubs.rs:86-159` — returns configurable fixture-defined outcomes.
- Binding registry call site: `crates/wos-conformance/src/engine.rs:126-127` (`bindings.register(ConformanceBinding)`).
- Real binding adapter: `crates/wos-formspec-binding/src/lib.rs:42-138` (`FormspecBinding<P>`) + `FormspecProcessor` trait at `lib.rs:11-39`.
- Integration request-response handler: `crates/wos-runtime/src/runtime.rs:1040-1257`; input-mapping helper `1531-1608`; output binding `1692-1876`; JSONPath subset parser `1735-1813`.
- `IntegrationBinding.kind` is a free-form string: `crates/wos-runtime/src/integration.rs:62`.
- Kernel state history schema: `wos-kernel.schema.json:303-307` (`historyState` enum `["shallow","deep"]`).
- `history_store: Option<HashMap<String, Vec<String>>>` already on `CaseInstance`: `crates/wos-core/src/instance.rs:46` — never written today.
- Milestone schema: `crates/wos-core/src/model/kernel.rs:199-202` and `480-482`; lint rule K-013 at `crates/wos-lint/src/rules/fel_analysis.rs:186-198`.
- Business calendar model: `crates/wos-core/src/model/business_calendar.rs` (complete schema, zero runtime integration).
- Timer hook point: `crates/wos-runtime/src/runtime.rs:1447-1491`.
- Provenance enum: `crates/wos-core/src/provenance.rs:13-156` (`ProvenanceKind`).
- Fixture directory: `crates/wos-conformance/fixtures/*.json`.

**Counts to preserve at the end:** 18 specs, 18 schemas. Conformance fixtures today: 41 document fixtures + 102 conformance fixtures (102 green). This plan adds roughly 35 new conformance fixtures.

---

## Slice Map

| # | Slice | TODO bullets | Est. effort |
|---|---|---|---|
| 1 | **S15.1** Wire real binding into conformance | Binding-backed S15 conformance (partial) | 1 day |
| 2 | **S15.2** Author S15 validation fixtures | Binding-backed S15 conformance (complete) | 1 day |
| 3 | **S15.3** Delete stubs + re-validation pin check | Binding-backed S15 conformance / Version-pinned response validation | 0.5 day |
| 4 | **KS.1** History state (Deep + Shallow) | History state semantics | 1.5 days |
| 5 | **KS.2** Milestone firing | Milestone firing | 1 day |
| 6 | **BC.1** Business Calendar SLA runtime | Business Calendar SLA | 1 day |
| 7 | **NB.1** Binding-kind refactor to enum + handler trait | Integration Profile binding coverage (prep) | 0.5 day |
| 8 | **NB.2** Output-binding RFC 9535 profile | Integration Profile output binding | 0.5 day |
| 9 | **NB.3** CloudEvents bindings (emit / consume / callback) | Integration Profile binding coverage | 1.5 days |
| 10 | **NB.4** Arazzo / tool / policy-engine bindings | Integration Profile binding coverage | 1.5 days |
| 11 | **FIN** Counts reconciliation, TODO update, parent repo bump | Closes §1 | 0.5 day |

**Total:** ~10.5 engineer-days of focused work.

**Plan-decay note:** Slice 1 is fully detailed with TDD steps and code. Slices 2–11 carry file paths, test names, acceptance criteria, and the code sketches needed for a skilled engineer to execute — but before each slice is executed, the owner should re-read the prior slice's landed diff and refine this plan's next slice with current line numbers and the actual shape of types introduced by the prior slice. This is a monolithic index, not a script.

---

## Shared Workflow (applies to every slice)

1. **Start inside the submodule.** All work in `/Users/mikewolfd/Work/formspec/wos-spec`. The parent repo `formspec` tracks `wos-spec` as a submodule.
2. **Baseline.** `cd wos-spec && cargo test --workspace` must be green before any slice. If red, stop and investigate — never write new work on top of a red bar.
3. **Per task:** write the failing test, run it, confirm fail-for-right-reason, implement, run green, commit.
4. **Per slice:** once all slice tasks pass, commit a `chore(wos-spec): slice <id> complete` marker and bump the submodule pointer in the parent `formspec` repo with `build: bump wos-spec submodule (<slice id>)`.
5. **Never** commit `ConformanceBinding` or `StubValidator` back into `pub` after Slice 3 — their deletion is a one-way door.

---

## Slice 1 — S15.1: Wire `wos-formspec-binding` into conformance alongside `ConformanceBinding`

**Outcome:** Fixtures can opt into the real `FormspecBinding` by declaring `binding: "formspec"` (or similar) in fixture JSON. At least one existing fixture runs through the real path and remains green.

**Files:**
- Create: `crates/wos-conformance/src/formspec_processor.rs` — test-double `FormspecProcessor` impl that loads a Definition JSON from fixture-embedded data.
- Modify: `crates/wos-conformance/src/engine.rs:88-155` — conditional binding registration based on fixture field.
- Modify: `crates/wos-conformance/src/fixture.rs` — add `binding: Option<String>` field (default `"conformance"`).
- Modify: `crates/wos-conformance/src/lib.rs` — re-export new module internally (not `pub`).
- Test: `crates/wos-conformance/tests/formspec_binding_swap.rs` — new integration test.
- Fixture: port one existing fixture (start with `K-020-provenance-completeness.json` since its behavior is well-understood) — add a new fixture file `K-020-provenance-completeness-real-binding.json` rather than mutating the existing one.

### Task 1.1 — Add `binding` field to fixture schema

**Files:**
- Modify: `crates/wos-conformance/src/fixture.rs:61-78`

- [ ] **Step 1: Write the failing deserialization test**

Add to `crates/wos-conformance/tests/fixture_shape.rs` (create if absent):

```rust
use wos_conformance::fixture::ConformanceFixture;

#[test]
fn fixture_defaults_binding_to_conformance() {
    let json = serde_json::json!({
        "documents": { "kernel": "inline" },
        "initial_case_state": {},
        "events": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    let fx: ConformanceFixture = serde_json::from_value(json).unwrap();
    assert_eq!(fx.binding.as_deref(), Some("conformance"));
}

#[test]
fn fixture_accepts_formspec_binding() {
    let json = serde_json::json!({
        "binding": "formspec",
        "documents": { "kernel": "inline" },
        "initial_case_state": {},
        "events": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    let fx: ConformanceFixture = serde_json::from_value(json).unwrap();
    assert_eq!(fx.binding.as_deref(), Some("formspec"));
}
```

- [ ] **Step 2: Run to verify compile failure**

```bash
cd /Users/mikewolfd/Work/formspec/wos-spec
cargo test -p wos-conformance --test fixture_shape
```
Expected: compile error — field `binding` does not exist on `ConformanceFixture`.

- [ ] **Step 3: Add the field with serde default**

In `crates/wos-conformance/src/fixture.rs`, add to the `ConformanceFixture` struct:

```rust
#[serde(default = "default_binding")]
pub binding: Option<String>,
```

And add the default function (keep it private, near the struct):

```rust
fn default_binding() -> Option<String> {
    Some("conformance".to_string())
}
```

- [ ] **Step 4: Run to verify green**

```bash
cargo test -p wos-conformance --test fixture_shape
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/wos-conformance/src/fixture.rs crates/wos-conformance/tests/fixture_shape.rs
git commit -m "feat(wos-conformance): add binding selector to fixture schema"
```

---

### Task 1.2 — Create test-double `FormspecProcessor`

**Files:**
- Create: `crates/wos-conformance/src/formspec_processor.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/wos-conformance/tests/formspec_processor_double.rs`:

```rust
use wos_conformance::formspec_processor::FixtureFormspecProcessor;
use wos_core::instance::ActiveTask;
use wos_formspec_binding::FormspecProcessor;
use wos_runtime::BindingError;

#[test]
fn processor_validates_pinned_envelope() {
    let proc = FixtureFormspecProcessor::new("urn:fx:form", "1.0.0");
    let envelope = serde_json::json!({
        "status": "complete",
        "definitionUrl": "urn:fx:form",
        "definitionVersion": "1.0.0",
        "data": { "a": 1 }
    });
    let errs = proc.validate_envelope(&envelope).unwrap();
    assert!(errs.is_empty());
}

#[test]
fn processor_rejects_unpinned_envelope() {
    let proc = FixtureFormspecProcessor::new("urn:fx:form", "1.0.0");
    let envelope = serde_json::json!({ "status": "complete", "data": {} });
    let errs = proc.validate_envelope(&envelope).unwrap();
    assert!(errs.iter().any(|e| e["code"] == "envelope_missing_field"));
}
```

- [ ] **Step 2: Run to verify compile failure**

```bash
cargo test -p wos-conformance --test formspec_processor_double
```
Expected: unresolved module `formspec_processor`.

- [ ] **Step 3: Implement the test-double processor**

Create `crates/wos-conformance/src/formspec_processor.rs`:

```rust
//! Test-double FormspecProcessor for conformance fixtures.
//!
//! Implements the four FormspecProcessor methods with deterministic,
//! fixture-driven behaviour: envelope fields are checked by presence,
//! definition validation is a no-op (returns None), prefill is identity
//! over `case_state`, and response mapping returns the envelope `data`
//! verbatim when a mapping_ref is provided.

use wos_core::instance::ActiveTask;
use wos_formspec_binding::FormspecProcessor;
use wos_runtime::{BindingError, CaseMutationBundle};

pub struct FixtureFormspecProcessor {
    pinned_url: String,
    pinned_version: String,
}

impl FixtureFormspecProcessor {
    pub fn new(pinned_url: impl Into<String>, pinned_version: impl Into<String>) -> Self {
        Self {
            pinned_url: pinned_url.into(),
            pinned_version: pinned_version.into(),
        }
    }
}

impl FormspecProcessor for FixtureFormspecProcessor {
    fn validate_envelope(
        &self,
        response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError> {
        let mut errs = Vec::new();
        for field in &["status", "definitionUrl", "definitionVersion", "data"] {
            if response.get(*field).is_none() {
                errs.push(serde_json::json!({
                    "code": "envelope_missing_field",
                    "field": field,
                }));
            }
        }
        Ok(errs)
    }

    fn validate_definition(
        &self,
        _definition_url: &str,
        _definition_version: &str,
        _data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
        Ok(None) // test double treats any data as valid
    }

    fn compute_prefill(
        &self,
        _mapping_ref: Option<&str>,
        case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError> {
        Ok(Some(case_state.clone()))
    }

    fn map_response(
        &self,
        _mapping_ref: &str,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let data = response.get("data").cloned().unwrap_or_default();
        Ok(Some(CaseMutationBundle {
            path_updates: vec![("/mapped".to_string(), data)],
        }))
    }
}
```

Add `pub mod formspec_processor;` to `crates/wos-conformance/src/lib.rs`.

- [ ] **Step 4: Run tests to verify green**

```bash
cargo test -p wos-conformance --test formspec_processor_double
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/wos-conformance/src/formspec_processor.rs \
        crates/wos-conformance/src/lib.rs \
        crates/wos-conformance/tests/formspec_processor_double.rs
git commit -m "feat(wos-conformance): add FixtureFormspecProcessor test double"
```

---

### Task 1.3 — Register real binding when fixture opts in

**Files:**
- Modify: `crates/wos-conformance/src/engine.rs:126-146`

- [ ] **Step 1: Write the failing integration test**

Create `crates/wos-conformance/tests/formspec_binding_swap.rs`:

```rust
use wos_conformance::{WorkflowEngine, fixture::ConformanceFixture};

fn minimal_fixture_with_binding(binding: &str) -> ConformanceFixture {
    let json = serde_json::json!({
        "binding": binding,
        "documents": { "kernel": "inline" },
        "kernel_inline": {
            "url": "urn:test:kernel",
            "version": "1.0.0",
            "states": [{ "id": "s0", "initial": true }],
            "events": []
        },
        "initial_case_state": {},
        "events": [],
        "expected_transitions": [],
        "expected_provenance": [],
    });
    serde_json::from_value(json).unwrap()
}

#[test]
fn engine_accepts_formspec_binding_fixture() {
    let fx = minimal_fixture_with_binding("formspec");
    let mut engine = WorkflowEngine::new(&fx).expect("engine init");
    let result = engine.execute(&fx).expect("execute");
    assert_eq!(result.binding_used.as_deref(), Some("formspec"));
}

#[test]
fn engine_defaults_to_conformance_binding() {
    let fx = minimal_fixture_with_binding("conformance");
    let mut engine = WorkflowEngine::new(&fx).expect("engine init");
    let result = engine.execute(&fx).expect("execute");
    assert_eq!(result.binding_used.as_deref(), Some("conformance"));
}
```

- [ ] **Step 2: Run to verify compile/assertion failure**

```bash
cargo test -p wos-conformance --test formspec_binding_swap
```
Expected: `ConformanceResult` has no `binding_used` field.

- [ ] **Step 3: Add `binding_used` to `ConformanceResult`**

In `crates/wos-conformance/src/lib.rs` (or wherever `ConformanceResult` is defined — confirm with `grep -rn "struct ConformanceResult" crates/wos-conformance/src/`), add:

```rust
pub binding_used: Option<String>,
```

Initialize it in `WorkflowEngine::execute` at the result construction site.

- [ ] **Step 4: Branch registration on `fixture.binding`**

Modify `crates/wos-conformance/src/engine.rs` around line 126-127:

```rust
let mut bindings = BindingRegistry::new();
let binding_used = match fixture.binding.as_deref() {
    Some("formspec") => {
        let processor = crate::formspec_processor::FixtureFormspecProcessor::new(
            definition_url.clone(),
            definition_version.clone(),
        );
        bindings.register(wos_formspec_binding::FormspecBinding::new(processor));
        "formspec".to_string()
    }
    _ => {
        bindings.register(ConformanceBinding);
        "conformance".to_string()
    }
};
```

Persist `binding_used` on the engine and emit it in `ConformanceResult`.

- [ ] **Step 5: Wire `wos-formspec-binding` as a dev/test dependency of `wos-conformance`**

Add to `crates/wos-conformance/Cargo.toml` under `[dependencies]` (not dev, since `engine.rs` imports it):

```toml
wos-formspec-binding = { path = "../wos-formspec-binding" }
```

- [ ] **Step 6: Run to verify green**

```bash
cargo test -p wos-conformance --test formspec_binding_swap
cargo test -p wos-conformance
```
Expected: 2 new passes, no regression in existing tests.

- [ ] **Step 7: Commit**

```bash
git add crates/wos-conformance/src/engine.rs \
        crates/wos-conformance/src/lib.rs \
        crates/wos-conformance/Cargo.toml \
        crates/wos-conformance/tests/formspec_binding_swap.rs
git commit -m "feat(wos-conformance): register FormspecBinding when fixture opts in"
```

---

### Task 1.4 — Port one existing fixture to the real binding

**Files:**
- Create: `crates/wos-conformance/fixtures/K-020-provenance-completeness-real-binding.json`

- [ ] **Step 1: Copy and modify the fixture**

```bash
cp crates/wos-conformance/fixtures/K-020-provenance-completeness.json \
   crates/wos-conformance/fixtures/K-020-provenance-completeness-real-binding.json
```

Add `"binding": "formspec"` to the new file. Keep every other field identical.

- [ ] **Step 2: Run all fixtures (both paths)**

```bash
cargo test -p wos-conformance
```
Expected: the ported fixture is green through the real binding; the original is still green through the stub; total fixture count increments by 1 (102 → 103).

- [ ] **Step 3: Commit**

```bash
git add crates/wos-conformance/fixtures/K-020-provenance-completeness-real-binding.json
git commit -m "test(wos-conformance): port K-020 fixture to real FormspecBinding"
```

---

### Task 1.5 — Slice boundary

- [ ] **Step 1: Workspace green**

```bash
cargo test --workspace
```

- [ ] **Step 2: Slice commit marker**

```bash
git commit --allow-empty -m "chore(wos-spec): S15.1 slice complete"
```

- [ ] **Step 3: Parent repo bump**

```bash
cd /Users/mikewolfd/Work/formspec
git add wos-spec
git commit -m "build: bump wos-spec submodule (S15.1)"
```

---

## Slice 2 — S15.2: Author S15 validation fixtures through real binding

**Outcome:** Six new fixtures exercise the task lifecycle through `FormspecBinding` and assert on `contract_outcomes`. No existing stub-backed fixture is removed.

**Files:**
- Create fixtures:
  - `crates/wos-conformance/fixtures/S15-001-task-draft-prefill.json`
  - `crates/wos-conformance/fixtures/S15-002-submit-valid.json`
  - `crates/wos-conformance/fixtures/S15-003-submit-missing-envelope-field.json`
  - `crates/wos-conformance/fixtures/S15-004-pin-mismatch.json`
  - `crates/wos-conformance/fixtures/S15-005-submit-definition-invalid.json`
  - `crates/wos-conformance/fixtures/S15-006-response-mapping.json`
- Modify: `crates/wos-conformance/src/formspec_processor.rs` — extend `FixtureFormspecProcessor` so it can be *told* (via fixture data) to report a definition-invalid outcome, for S15-005 only. Add a `definition_errors: Vec<Value>` field populated from fixture input.

**Tasks:**

- [ ] **2.1** Extend `FixtureFormspecProcessor` to accept a list of canned definition-validation errors via a new constructor. Test covers the new constructor (red → green → commit).
- [ ] **2.2** Author `S15-001` (task draft with prefill from case state). Fixture asserts: one `TaskPresented` provenance record, `prefill_data` non-empty, transition `s0 → s1` on `task.completed`.
- [ ] **2.3** Author `S15-002` (happy-path submit). Assert `contract_outcomes["task-1"]` is `{ envelope_valid: true, pin_match: true, definition_valid: true }`.
- [ ] **2.4** Author `S15-003` (envelope missing `definitionUrl`). Assert `envelope_valid: false` with error code `envelope_missing_field`.
- [ ] **2.5** Author `S15-004` (submitted envelope carries wrong version). Assert `pin_match: false`.
- [ ] **2.6** Author `S15-005` (definition-validation failure via processor's canned errors). Assert `definition_valid: false`.
- [ ] **2.7** Author `S15-006` (response mapping — a completed task applies `/mapped` update to case state). Assert the resulting `DataMapping` provenance record.
- [ ] **2.8** Slice commit + parent bump.

**Acceptance:** fixture count 103 → 109. All green. Each new fixture includes a top-of-file `description` field explaining the behaviour under test.

---

## Slice 3 — S15.3: Delete `ConformanceBinding` and `StubValidator`, enforce pin at re-validation

**Outcome:** Every conformance fixture uses `FormspecBinding`. `ConformanceBinding`, `StubValidator`, and their `pub` exports are gone. Pin equality is asserted on re-validation paths, not only initial submit.

**Files:**
- Modify every fixture under `crates/wos-conformance/fixtures/*.json` — add `"binding": "formspec"` (or delete the field so the default becomes `"formspec"` after the default flip).
- Modify: `crates/wos-conformance/src/fixture.rs` — flip `default_binding()` to return `"formspec"`.
- Modify: `crates/wos-conformance/src/engine.rs` — drop the `ConformanceBinding` arm of the match.
- Delete: `crates/wos-conformance/src/engine.rs` lines `478-527` (the `ConformanceBinding` impl).
- Delete: `crates/wos-conformance/src/stubs.rs` entirely (confirm no fixture outside `S15-*` depends on `StubValidator.contract_outcomes` — if any do, migrate to the processor-canned-errors mechanism from Task 2.1).
- Modify: `crates/wos-conformance/src/lib.rs` — remove the `pub mod stubs;` and `pub use stubs::*;` lines.
- Modify: `crates/wos-runtime/src/runtime.rs` — find any path that re-validates an envelope post-submission (e.g., review/replay) and add pin-equality assertion there. If no such path exists, add one as part of the slice (call site: wherever `ValidationOutcome` is re-examined).
- Test: `crates/wos-formspec-binding/tests/replay_pin_enforcement.rs` (new).

**Tasks:**

- [ ] **3.1** Migrate any existing fixture that relied on `StubValidator` fixture-driven outcomes to the canned-error processor mechanism. Red: identify fixtures by `grep -l contract_outcomes crates/wos-conformance/fixtures/`. Green: each migrated fixture still passes.
- [ ] **3.2** Flip `default_binding()` to `"formspec"`. Run fixtures — every remaining stub-reliant fixture fails. Fix by migration, not by stub fallback.
- [ ] **3.3** Delete `ConformanceBinding`. Delete `StubValidator` / `stubs.rs`. Remove `pub` surface. Run fixtures — must stay green.
- [ ] **3.4** Write a new test `replay_pin_enforcement.rs`: simulate re-loading a completed response with a different pinned version than case-instance; assert `BindingError::PinMismatch`.
- [ ] **3.5** Implement pin-equality on re-validation paths (exact hook depends on runtime code; survey points to `runtime.rs` `ValidationOutcome` construction). Red → green.
- [ ] **3.6** Slice commit + parent bump.

**Acceptance:** `grep -rn "ConformanceBinding\|StubValidator" crates/` returns nothing. `cargo test --workspace` green.

---

## Slice 4 — KS.1: History state (Deep + Shallow) — fixtures first

**Outcome:** `historyState: "deep"` on a compound state causes re-entry after external interruption to restore the full nested configuration; `historyState: "shallow"` restores only the direct substate. `history_store` is written on exit of any state whose parent has a `historyState` declaration.

**Files:**
- Create fixtures (9):
  - `K-H-D1-deep-normal-reentry.json` (depth 1, deep, event interrupts → re-enter, full path restored)
  - `K-H-D1-shallow-normal-reentry.json`
  - `K-H-D2-deep-normal-reentry.json`
  - `K-H-D2-shallow-normal-reentry.json`
  - `K-H-D2-deep-after-parallel-exit.json` (exit from a parallel region triggers history capture)
  - `K-H-D2-shallow-after-parallel-exit.json`
  - `K-H-D3-deep-normal-reentry.json`
  - `K-H-D2-deep-across-boundary.json` (a transition's target crosses the history-bearing state's boundary — history MUST NOT be used)
  - `K-H-D2-shallow-across-boundary.json`
- Modify: `crates/wos-core/src/instance.rs:46` — no schema change; add helper methods `record_history(&mut self, compound_id: &str, configuration: &[String])` and `take_history(&mut self, compound_id: &str) -> Option<Vec<String>>`.
- Modify: `crates/wos-core/src/evaluator.rs` (or wherever state exit/entry is driven — confirm with `grep -rn "fn on_exit\|fn enter_state"`) — write history on exit of the *deepest* configuration whose parent has `historyState`; read history on entry into a state whose `historyState` is declared.

**Tasks:**

- [ ] **4.1** Author the 9 fixtures first. Each fixture asserts expected state-configuration provenance after re-entry. Run them — **all 9 should fail**. That's the baseline.
- [ ] **4.2** Implement `CaseInstance::record_history` and `take_history` on `instance.rs`. Unit test in `crates/wos-core/tests/history_store_ops.rs`. Red → green.
- [ ] **4.3** Hook exit handler: when exiting a state, walk up parent chain to find nearest ancestor with `historyState`; record the substate configuration at that ancestor's entry in `history_store`. Test: `K-H-D1-shallow-normal-reentry` and `K-H-D1-deep-normal-reentry` pass.
- [ ] **4.4** Hook entry handler: when entering a state with `historyState`, consult `history_store` — if populated, restore per Deep/Shallow semantics instead of `initialState`. Test: remaining 7 fixtures pass.
- [ ] **4.5** Boundary check: when a transition's target is outside the history-bearing state, skip history capture *and* entry-restoration. Fixture `K-H-D2-*-across-boundary` covers this. Red → green.
- [ ] **4.6** Slice commit + parent bump.

**Acceptance:** all 9 new fixtures green. Existing `k-035-history-cleared-on-exit.json` still green. Fixture count 109 → 118 (approximately; subtract any replaced).

---

## Slice 5 — KS.2: Milestone firing

**Outcome:** Milestone conditions are evaluated after each durable data write. Newly-true milestones emit a `MilestoneFired` provenance record, then reactive transitions drain. Milestones do not re-fire once fired within a given case-instance until the underlying state changes.

**Files:**
- Modify: `crates/wos-core/src/provenance.rs:13-156` — add variant `MilestoneFired { milestone_id: String }` to `ProvenanceKind`.
- Modify: `crates/wos-core/src/instance.rs` — add `fired_milestones: HashSet<String>` to `CaseInstance` (serde default empty).
- Create: `crates/wos-runtime/src/milestones.rs` — `evaluate_milestones(instance, kernel, pre_state, post_state) -> Vec<ProvenanceRecord>`. Evaluates each milestone's FEL condition against post-state; for any newly-true (not in `fired_milestones`), records the ID and returns a provenance record.
- Modify: `crates/wos-runtime/src/runtime.rs` — insert milestone evaluation **after** data write completes durably, **before** reactive-transition drain. Exact line identified during execution — look near every `CaseMutationBundle` application.
- Create fixtures:
  - `K-M-001-single-fire.json`
  - `K-M-002-no-refire.json`
  - `K-M-003-per-repeat-instance.json` (milestone on a repeatable group fires per instance)
  - `K-M-004-ordering-with-transition.json` (milestone → transition; asserts provenance order in expected_provenance)

**Tasks:**

- [ ] **5.1** Add `MilestoneFired` variant; `fired_milestones` field. Unit tests in `wos-core`.
- [ ] **5.2** Author the 4 fixtures. Run — all fail.
- [ ] **5.3** Implement `milestones.rs` evaluator. Unit-test in `crates/wos-runtime/tests/milestones.rs` (pure function, no runtime needed). Red → green.
- [ ] **5.4** Hook into runtime after durable write + before reactive-transition drain. Fixture `K-M-001` passes.
- [ ] **5.5** Implement `no-refire` via `fired_milestones` insert. `K-M-002` passes.
- [ ] **5.6** Implement per-repeat-instance evaluation: milestone inside a repeat group evaluates per index; track `fired_milestones` with instance-scoped keys like `"milestoneId#index"`. `K-M-003` passes.
- [ ] **5.7** Confirm ordering: `DataMapping` → `MilestoneFired` → transition-resulting records. `K-M-004` passes.
- [ ] **5.8** Slice commit + parent bump.

**Acceptance:** K-013 lint rule still green (no regressions). Fixture count +4.

---

## Slice 6 — BC.1: Business Calendar SLA runtime

**Outcome:** SLA deadlines computed against an attached `BusinessCalendarDocument` — holidays skipped, operating-hours enforced, timezone-correct. Deadlines computed lazily at check time. `calendarVersion` appears in provenance.

**Files:**
- Create: `crates/wos-core/src/business_calendar/evaluator.rs` (split from the model module; add `next_business_moment(start: DateTime, duration: Duration, calendar: &BusinessCalendarDocument) -> DateTime`).
- Modify: `crates/wos-runtime/src/runtime.rs:1447-1491` — replace literal deadline comparison with calendar-aware call when a calendar is attached.
- Modify: `crates/wos-core/src/provenance.rs` — add `calendar_version: Option<String>` to whatever provenance record covers deadline evaluations (likely `TimerFired` or a new `DeadlineEvaluated` variant — decide during execution).
- Create fixtures:
  - `G-S10-001-holiday-shift.json`
  - `G-S10-002-operating-hours-cutoff.json`
  - `G-S10-003-timezone-boundary.json`
  - `G-S10-004-calendar-update-shifts-future-deadline.json` (case created, calendar updated mid-flight; unchecked future deadline shifts)

**Tasks:**

- [ ] **6.1** Unit tests for `next_business_moment` in `crates/wos-core/tests/business_calendar_eval.rs`: holidays, operating hours, weekends, timezone edges (DST transitions).
- [ ] **6.2** Implement evaluator. TDD through unit tests.
- [ ] **6.3** Author the 4 fixtures. Run — all fail.
- [ ] **6.4** Hook into timer handler at `runtime.rs:1447-1491`. Each fixture passes as hook lands.
- [ ] **6.5** Add `calendar_version` to deadline provenance. Assert in fixtures.
- [ ] **6.6** Slice commit + parent bump.

**Acceptance:** Fixture count +4. Existing timer-fixture (`K-046-timer-provenance.json`) still green, with a `calendar_version: null` tolerated when no calendar is attached.

---

## Slice 7 — NB.1: Binding-kind refactor (enum + handler trait)

**Outcome:** `IntegrationBinding.kind` becomes a typed enum. Each kind's execution is delegated to an `IntegrationBindingHandler` trait implementation. Request-response becomes the first handler. No behavior change.

**Files:**
- Modify: `crates/wos-runtime/src/integration.rs:62` — replace `pub kind: String` with `pub kind: IntegrationBindingKind`.
- Add to same file: the `IntegrationBindingKind` enum with variants `RequestResponse`, `EventEmit`, `EventConsume`, `Callback`, `ArazzoSequence`, `Tool`, `PolicyEngine`. `#[serde(rename_all = "kebab-case")]`.
- Create: `crates/wos-runtime/src/integration_handlers/mod.rs` — `trait IntegrationBindingHandler` with `fn execute(&self, ctx: &mut InvocationCtx, binding: &IntegrationBinding) -> Result<InvocationOutcome, RuntimeError>;`.
- Create: `crates/wos-runtime/src/integration_handlers/request_response.rs` — move the body of the request-response arm currently at `runtime.rs:1049-1257` into this handler.
- Modify: `crates/wos-runtime/src/runtime.rs:1040-1060` — dispatch via handler registry keyed on `IntegrationBindingKind`.

**Tasks:**

- [ ] **7.1** Add enum + serde, keep string field as alias during migration (one commit).
- [ ] **7.2** Extract `IntegrationBindingHandler` trait + `RequestResponseHandler`. All existing integration tests stay green.
- [ ] **7.3** Remove the string alias; drop the match arm for non-request-response (they now return `RuntimeError::UnsupportedBindingKind(kind)`).
- [ ] **7.4** Slice commit + parent bump.

**Acceptance:** no fixture regressions. Workspace green.

---

## Slice 8 — NB.2: Output-binding RFC 9535 profile

**Outcome:** `outputBinding` JSONPath supports member access, index, wildcard, slice. Filter expressions (`[?()]`) and recursive descent (`..`) are rejected at definition load with a clear diagnostic. Profile documented in `specs/profiles/integration.md` §3.3.

**Files:**
- Modify: `crates/wos-runtime/src/runtime.rs:1735-1813` — extend `parse_json_path` with `Wildcard` and `Slice(start, end, step)` segments.
- Modify: `crates/wos-runtime/src/runtime.rs:1709-1733` — extend `resolve_json_path` to emit arrays for wildcard/slice matches.
- Modify: `crates/wos-lint/src/rules/` — new rule `I-001-outputbinding-profile`, rejects `..` and `[?(...)]` in any `outputBinding.jsonPath` field.
- Modify: `specs/profiles/integration.md` §3.3 — document the profile explicitly.

**Tasks:**

- [ ] **8.1** Unit tests in `crates/wos-runtime/tests/jsonpath_profile.rs` for wildcard, slice, escape interactions with wildcards.
- [ ] **8.2** Implement wildcard + slice in parser + resolver.
- [ ] **8.3** Lint rule I-001. Red → green. Add entry to `LINT-MATRIX.md`.
- [ ] **8.4** Fixture: `I-001-jsonpath-filter-rejected.json` (lint must fail). Fixture: `I-002-wildcard-extracts-array.json` (runtime behavior).
- [ ] **8.5** Update spec prose.
- [ ] **8.6** Slice commit + parent bump.

**Acceptance:** lint rule count 196 → 197. `LINT-MATRIX.md` reflects the new rule.

---

## Slice 9 — NB.3: CloudEvents bindings (emit / consume / callback)

**Outcome:** `event-emit`, `event-consume`, `callback` handlers execute, emit CloudEvents 1.0 envelopes, correlate callbacks via `subject`, and produce per-kind provenance records. Events missing `id` or `source` are rejected at ingress.

**Files:**
- Create: `crates/wos-runtime/src/integration_handlers/event_emit.rs`
- Create: `crates/wos-runtime/src/integration_handlers/event_consume.rs`
- Create: `crates/wos-runtime/src/integration_handlers/callback.rs`
- Create: `crates/wos-runtime/src/cloudevents.rs` — `struct CloudEvent { id, source, spec_version, type, subject, time, data, ... }` + `fn validate_ingress(&self) -> Result<(), CloudEventError>`.
- Modify: `crates/wos-core/src/provenance.rs` — variants `EventEmitted { subject, id, source }`, `EventConsumed { subject, id, source }`, `CallbackReceived { subject, id, correlated_invocation_id }`, `CallbackPending { subject, expected_until }`.
- Modify: `schemas/profiles/wos-integration-profile.schema.json` — add `subject` template field to callback binding kind (if not already implied).
- Create fixtures (6–8): `INT-EMIT-001-happy.json`, `INT-EMIT-002-full-envelope-in-provenance.json`, `INT-CONSUME-001-happy.json`, `INT-CONSUME-002-missing-id-rejected.json`, `INT-CALLBACK-001-correlation.json`, `INT-CALLBACK-002-pending-to-received.json`, `INT-CALLBACK-003-uncorrelated-drop.json`.

**Tasks:**

- [ ] **9.1** CloudEvents struct + ingress validation tests (`tests/cloudevents.rs`).
- [ ] **9.2** Provenance variants + serde tests.
- [ ] **9.3** `event-emit` handler — produces envelope, records `EventEmitted`. Fixture `INT-EMIT-001` and `INT-EMIT-002` pass.
- [ ] **9.4** `event-consume` handler — validates ingress, records `EventConsumed` on success, rejects with `BindingError::EventIngressInvalid` on missing fields. Fixtures `INT-CONSUME-001`, `INT-CONSUME-002` pass.
- [ ] **9.5** Correlation table (in-memory store addition): `pending_callbacks: HashMap<subject, InvocationId>`. `callback` handler awaits; receiver resolves by subject. Fixtures `INT-CALLBACK-001`, `INT-CALLBACK-002`, `INT-CALLBACK-003` pass.
- [ ] **9.6** Slice commit + parent bump.

**Acceptance:** 6–8 new fixtures green; existing request-response fixtures untouched.

---

## Slice 10 — NB.4: Arazzo, tool, policy-engine bindings

**Outcome:** Each remaining binding kind executes; Arazzo steps produce per-step provenance; tool reuses the request-response contract; policy-engine decisions normalize to a pinned shape.

**Files:**
- Create: `crates/wos-runtime/src/integration_handlers/arazzo_sequence.rs`
- Create: `crates/wos-runtime/src/integration_handlers/tool.rs`
- Create: `crates/wos-runtime/src/integration_handlers/policy_engine.rs`
- Modify: `crates/wos-core/src/provenance.rs` — `ArazzoStep { step_id, outcome }`, `ToolInvoked { tool_id, outcome }`, `PolicyDecision { decision, reasons_count }`.
- Create: `crates/wos-runtime/src/policy_decision.rs` — canonical decision struct `PolicyDecision { decision: Allow|Deny|Indeterminate, reasons: Vec<Reason>, obligations: Vec<Obligation> }` + adapters for common engines (OPA result shape, Cedar result shape).
- Fixtures per kind (3–4 each): happy, failure, indeterminate-for-policy, per-step-failure-for-arazzo.

**Tasks:**

- [ ] **10.1** Arazzo handler — each step dispatches through the request-response handler internally; step outputs available to subsequent steps via a scoped context. Fixtures cover happy, mid-sequence failure, per-step provenance.
- [ ] **10.2** Tool handler — reuse request-response with a tool-identity check. Fixtures cover happy + pin-mismatch.
- [ ] **10.3** Policy-engine handler — adapter per engine type; normalize to `PolicyDecision`. Fixtures cover allow, deny, indeterminate, and malformed-response rejection.
- [ ] **10.4** Slice commit + parent bump.

**Acceptance:** all 6 non-request-response binding kinds green. `IntegrationBindingKind` enum has no `TODO` in match arms.

---

## Slice 11 — FIN: Counts reconciliation, TODO update, parent bump

**Outcome:** `wos-spec/TODO.md` §1 checkboxes all marked `[x]`. Implementation Status matrix updated. Counts in `TODO.md` header match reality.

**Tasks:**

- [ ] **11.1** Count fixtures: `ls crates/wos-conformance/fixtures/*.json | wc -l` and update the TODO header.
- [ ] **11.2** Count lint rules: `grep -c "^-" wos-spec/LINT-MATRIX.md` approach, or the existing counting script. Update TODO header (196 → new total).
- [ ] **11.3** Move every §1 checkbox to `[x]` and push each completed bullet to the "Completed" section at the bottom of the TODO.
- [ ] **11.4** Update `WOS-IMPLEMENTATION-STATUS.md` with a row per completed slice.
- [ ] **11.5** Update `WOS-FEATURE-MATRIX.md` cells that now flip green.
- [ ] **11.6** Final commit in submodule + final bump in parent.

**Acceptance:** `cargo test --workspace` green. `npm run docs:check` (in parent) passes. TODO §1 is empty of open checkboxes.

---

## Self-Review Checklist (author ran 2026-04-14)

- **Spec coverage:** every §1 bullet in the TODO maps to a slice (S15.1–3 → slices 1–3, KS.1–2 → slices 4–5, BC.1 → slice 6, NB.1–4 → slices 7–10, finalisation → slice 11). No bullet unmapped.
- **Placeholder scan:** no `TODO`/`TBD` inside tasks; no "handle edge cases" or "similar to Task N" without repeated code. Where a later slice's code can't be written without earlier slice's shape, the plan says so explicitly (plan-decay note + per-slice "File paths confirmed at execution time" mention).
- **Type consistency:** `FixtureFormspecProcessor`, `FormspecBinding`, `ConformanceFixture.binding`, `IntegrationBindingKind`, `IntegrationBindingHandler`, `PolicyDecision` used consistently across slices that reference them.
- **Scope check:** plan covers one section (§1); §§2–5 of the TODO are deliberately out of scope. Engine bindings (§2) cannot honestly begin until this plan is complete.

---

## Execution Handoff

Plan complete and saved to `wos-spec/thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration. Good for Slice 1 where TDD steps are tight.
2. **Inline Execution** — execute tasks in this session with checkpoints. Good when you want to feel the shape of the code land.

Which approach?
