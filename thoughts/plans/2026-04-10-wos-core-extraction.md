# Implementation Plan: Extract wos-core from wos-conformance

**Date:** 2026-04-10
**Status:** Completed
**Author:** Formspec project

---

## Goal

Extract the domain model and evaluation algorithm from `wos-conformance` into a shared `wos-core` crate. After extraction, `wos-lint` and `wos-conformance` both depend inward on `wos-core`, and future tools (runtime adapters, simulators, migration tools) can do the same.

## Current State

```text
wos-lint ──→ serde_json (raw Value walking, 75 rules)
wos-conformance ──→ serde_json (raw Value walking + evaluation engine)
```

Both crates parse JSON documents independently. The conformance engine reimplements document structure knowledge (state lookup, action execution, timer management) that the linter also needs. Neither uses typed models.

## Target State

```text
wos-core (typed models + evaluation + provenance + timers + context)
   ↑              ↑              ↑
wos-lint       wos-conformance   wos-runtime (future)
```

## Scaffolding (DONE)

`wos-core` has been scaffolded with:

| Module | Contents | Status |
| ------ | -------- | ------ |
| `model::kernel` | `KernelDocument`, `State`, `Transition`, `Action`, `Actor`, `CaseFile`, etc. | Typed, compiles |
| `model::governance` | Placeholder | Stub |
| `model::ai` | Placeholder | Stub |
| `eval` | `Evaluator`, `Configuration`, `EvalError` | Skeleton with `process_event` placeholder |
| `provenance` | `ProvenanceKind` enum, `ProvenanceRecord`, `ProvenanceLog` | Complete |
| `context` | `EvalContext` with `to_fel_environment()` | Complete |
| `timer` | `Timers` with create/cancel/collect_expired | Complete |
| `project` | `Project` with kernel accessor methods | Complete |

## Extraction Steps

### Phase 1: Move evaluation logic from conformance to core

**Step 1.1: Deserialize kernel documents into typed model.**

The conformance engine currently calls `serde_json::from_str` to get a `Value`, then walks it with `.get()` and `.as_str()`. Change to:

```rust
let kernel: KernelDocument = serde_json::from_str(&json)?;
let mut evaluator = Evaluator::new(kernel)?;
```

This requires the `KernelDocument` serde annotations to match the actual JSON structure of the fixtures. Test by deserializing `purchase-order-approval.json`, `benefits-adjudication.json`, and `medicaid-redetermination.json`.

**Estimated effort:** Small. The typed model is already scaffolded. Main risk: serde field naming mismatches (camelCase vs snake_case).

**Step 1.2: Move process_event into Evaluator.**

Extract the `process_event`, `try_fire_transition`, `fire_transition`, `enter_state` methods from `wos-conformance/src/engine.rs` into `wos-core/src/eval.rs`. Adapt from `serde_json::Value` walking to typed `State`, `Transition`, `Action` access.

Key changes:
- `state.get("type").and_then(Value::as_str)` → `state.kind`
- `state.get("transitions").and_then(Value::as_array)` → `state.transitions.iter()`
- `transition.get("target").and_then(Value::as_str)` → `transition.target.as_str()`
- `action.get("action").and_then(Value::as_str)` → `action.action` (typed `ActionKind`)

**Estimated effort:** Medium. The logic is already correct; this is a mechanical translation from Value walking to typed field access. The main complexity is compound/parallel state handling.

**Step 1.3: Move timer and provenance logic.**

Timer management is already scaffolded in `wos-core/src/timer.rs`. Move the timer-related code from `engine.rs` (startTimer, cancelTimer, fire_expired_timers) to use the `Timers` type.

Provenance is already scaffolded in `wos-core/src/provenance.rs` with the `ProvenanceKind` enum (code-scout Finding #13). Move all provenance recording to use `ProvenanceLog`.

**Estimated effort:** Small. These are already independent modules in the conformance engine.

**Step 1.4: Move FEL context building.**

`EvalContext` is already scaffolded in `wos-core/src/context.rs`. Move `build_fel_context` from `engine.rs` to `EvalContext::from_case_state()` or similar. The `to_fel_environment()` method already handles the FEL bridge.

**Estimated effort:** Small.

### Phase 2: Adapt wos-conformance to use wos-core

**Step 2.1: Make wos-conformance a thin test harness.**

After extraction, `wos-conformance` becomes:
- Fixture loading (JSON → typed fixture struct)
- Document loading (JSON → `KernelDocument` via `wos-core`)
- Evaluator construction (`Evaluator::new(kernel)`)
- Event sequence replay (`evaluator.process_event()` in a loop)
- Assertion matching (compare actual transitions/provenance against expectations)

The engine logic, state management, timer tracking, and provenance recording all live in `wos-core`.

**Step 2.2: Add contract validator trait.**

```rust
// In wos-core
pub trait ContractValidator {
    fn validate(&self, contract_ref: &str, data: &serde_json::Value) -> ValidationResult;
}

// In wos-conformance
struct StubValidator { outcomes: HashMap<String, bool> }
impl ContractValidator for StubValidator { ... }
```

The `Evaluator` accepts a `&dyn ContractValidator` at `invokeService` and `contractHook` points. The conformance engine provides a stub. A future runtime provides a real Formspec validator.

**Estimated effort:** Small. The trait is simple; the stub reads from fixture `contract_outcomes`.

### Phase 3: Adapt wos-lint to use wos-core typed models

**Step 3.1: Deserialize documents into typed models in wos-lint.**

Currently `wos-lint` walks raw `serde_json::Value` trees. Change the document loading to deserialize into `wos-core` typed models where possible. The `DocumentKind` detection stays (it's format-specific), but after detection, the linter can deserialize into the appropriate typed model.

**Step 3.2: Replace `collect_kernel_tags` etc. with Project methods.**

`wos-core::Project` already has `kernel_tags()`, `kernel_events()`, `kernel_actor_ids()`, `kernel_case_fields()`. These replace the 6x-called collection builders in `tier2.rs` (code-scout Finding #1).

**Step 3.3: Replace Value walking in tier1 rules with typed access.**

Most T1 rules walk JSON objects to check state types, transitions, actions. With typed models, these become pattern matches on enums:

```rust
// Before:
if state.get("type").and_then(Value::as_str) == Some("final") {
    if state.get("transitions").is_some_and(|t| ...) { ... }
}

// After:
if state.kind == StateKind::Final && !state.transitions.is_empty() { ... }
```

**Estimated effort:** Medium-large. The T1 rules are the bulk of the codebase. The typed access makes each rule simpler but there are 32 rules to convert.

### Phase 4: Add governance and AI typed models

**Step 4.1: Define GovernanceDocument, DueProcessConfig, etc.**

Extract from the governance schema structure. The T2-xdoc rules in `tier2.rs` already know what fields they need — the typed model formalizes those accesses.

**Step 4.2: Define AIIntegrationDocument, AgentConfig, etc.**

Same pattern. The T2-xdoc AI rules define the shape.

**Step 4.3: Update Project to hold all document types.**

```rust
pub struct Project {
    kernel: Option<KernelDocument>,
    governance: Vec<GovernanceDocument>,
    ai_integration: Vec<AIIntegrationDocument>,
    advanced: Vec<AdvancedGovernanceDocument>,
    policy_parameters: Vec<PolicyParametersDocument>,
    // ... sidecars
}
```

**Estimated effort:** Large but mechanical. Each document type follows the same serde pattern.

## Dependency Graph After Extraction

```text
                    fel-core
                       ↑
                    wos-core
                   ↗    ↑    ↘
            wos-lint  wos-conformance  (future: wos-runtime, wos-wasm)
```

- `wos-core` depends on `fel-core` (FEL evaluation) and `serde` (typed deserialization)
- `wos-lint` depends on `wos-core` (typed models, project, tags/events/actors) and `fel-core` (AST analysis)
- `wos-conformance` depends on `wos-core` (evaluator, typed models) and `wos-lint` (document detection, fixture loading)
- `wos-conformance`'s dependency on `wos-lint` may become optional — if document detection moves to `wos-core`

## Risks

1. **Serde naming mismatches.** The JSON fixtures use camelCase; the Rust types use snake_case with `#[serde(rename_all = "camelCase")]`. Any mismatch means deserialization fails silently (field defaults to None/empty). Mitigation: test deserialization against every existing fixture.

2. **Partial extraction temptation.** If we extract only the kernel model but leave governance/AI as raw JSON, we end up with a mixed codebase. Mitigation: extract all document types in one phase, even if governance/AI models start as thin wrappers.

3. **Breaking conformance tests.** The 37 existing tests are the correctness baseline. Every extraction step must keep them passing. Mitigation: run `cargo test` after every change.

## Success Criteria

- [ ] `KernelDocument` deserializes all 8 kernel fixtures without error
- [ ] `Evaluator::process_event()` passes all 10 conformance integration tests
- [ ] `wos-lint` T1 rules operate on typed models, not raw JSON
- [ ] `wos-lint` T2 rules use `Project` methods instead of repeated collection building
- [ ] `collect_kernel_tags` called 0 times (replaced by `Project::kernel_tags()`)
- [ ] No `serde_json::Value` walking in the evaluation hot path
- [ ] `ContractValidator` trait defined with stub and real implementations
- [ ] `cargo test` passes with >= 37 tests after extraction
