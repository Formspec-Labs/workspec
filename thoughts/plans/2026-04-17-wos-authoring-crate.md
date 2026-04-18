# WOS Authoring Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce `wos-authoring` — an intent-driven authoring API over `wos-core` — that gives tool handlers, LLM harnesses, and human-authored scripts a clean, atomic, undo-capable way to build WOS documents without touching raw types directly.

**Architecture:** `WosProject` wraps `wos-core`'s typed model via a private command pipeline (dispatch → handler → state update → diagnostic collection → undo stack). Public API is 28 intent-level helper methods; no `dispatch()`, no `.raw` accessor, no command enum is ever exposed. This mirrors the `formspec-studio-core` pattern (`Project` over `IProjectCore`) exactly one layer down — `wos-core` plays the role of `formspec-core`, and `wos-authoring` plays the role of `formspec-studio-core`. Consumers (principally `wos-mcp`) import from `wos-authoring` and never from `wos-core` directly for mutation operations.

**Tech Stack:** Rust, `wos-core`, `serde`, `serde_json`, `thiserror`, `indexmap`.

**Spec anchor:** Open questions Q6 (resolved in [architecture-review-open-questions.md](../archive/reviews/2026-04-16-architecture-review-open-questions.md#q6)) establishes the three-crate layering (`wos-authoring` → `wos-mcp` → `wos-synth-core`) and confirms `wos-authoring` as the seam where intent-to-primitives translation lives. The Formspec pattern this mirrors is at `packages/formspec-studio-core/` (TypeScript) — the composition model, the helper-result return type, and the no-raw-dispatch discipline all come from there.

**Related:**
- `2026-04-17-wos-mcp-crate.md` — `wos-mcp` consumes `wos-authoring`; each MCP tool handler calls one or more `WosProject` helper methods rather than assembling raw model mutations itself.
- `2026-04-16-wos-synth-crate.md` — `wos-synth-core`'s `ToolContext` production implementation delegates to `wos-mcp`, which in turn calls `wos-authoring`. `wos-authoring` is the lowest-level authoring primitive in that chain.

---

## Prerequisites

- `wos-core` exists with typed model (`KernelDocument`, `State`, `Transition`, `Actor`, `ActorKind`, `ImpactLevel`, `Lifecycle`, `CaseFile`, `GovernanceDocument`, `AIIntegrationDocument`, etc.) and a parser that deserializes from JSON. No work needed there.
- Understanding of how `formspec-studio-core` composes over `formspec-core`: `Project` holds a private `IProjectCore`, all mutations go through helper methods, every helper returns `HelperResult`, and undo reverses one helper call as an atomic unit even if that call internally dispatched multiple commands. See `packages/formspec-studio-core/src/project.ts` for the template.

---

## Completion Criteria

1. `crates/wos-authoring/` scaffolded with `Cargo.toml`, `src/lib.rs`, `src/project.rs`, and a `src/handlers/` module containing one file per concern area.
2. Public API: a `WosProject` struct with **28 intent-driven helper methods** across six domain areas (lifecycle, actors, correspondence, governance, AI integration, provenance/metadata). No `dispatch()`, no `raw` accessor, no public command-level API.
3. A private command pipeline: `dispatch_internal(cmd)` → handler function → apply to owned document state → collect diagnostics → push snapshot to undo stack.
4. Undo/redo operates at the helper-call granularity: `undo()` reverses one `add_state()` call even if it internally dispatched multiple internal commands.
5. Round-trip fidelity: `WosProject::from_document(doc) -> project; project.export() -> doc'` produces a document equal to the input (modulo key ordering within objects, which JSON does not guarantee).
6. Unit tests for every helper method: at minimum one happy-path test and one error-path test per helper.
7. Integration test: compose a 10+ helper sequence starting from `WosProject::new()` to build the purchase-order-approval workflow (`fixtures/kernel/purchase-order-approval.json`) from scratch, then assert the exported document matches the shipped fixture field-by-field.

---

## File Structure

```
crates/wos-authoring/
├── Cargo.toml              # deps: wos-core, serde, serde_json, thiserror, indexmap
├── src/
│   ├── lib.rs              # public exports: WosProject, ProjectError, AuthoringResult
│   ├── project.rs          # WosProject struct + all 28 helper methods
│   ├── pipeline.rs         # private dispatch, undo/redo, diagnostic collection
│   ├── commands.rs         # private Command enum (never exposed publicly)
│   ├── history.rs          # undo/redo stack: Vec<DocumentSnapshot>, cursor
│   └── handlers/
│       ├── mod.rs          # aggregates all handler submodules
│       ├── lifecycle.rs    # state + transition + region + initial-state handlers
│       ├── actors.rs       # actor + actor-extension handlers
│       ├── transitions.rs  # fine-grained transition property handlers
│       ├── governance.rs   # due-process, assertion gate, impact level, hold policy
│       ├── ai.rs           # AI agent, deontic constraint, autonomy, fallback chain
│       └── provenance.rs   # provenance config, metadata, correspondence fields
└── tests/
    ├── round_trip.rs       # from_document → export equality for all fixture files
    ├── lifecycle_tests.rs  # per-helper unit tests for lifecycle domain
    ├── actors_tests.rs     # per-helper unit tests for actors domain
    ├── governance_tests.rs # per-helper unit tests for governance domain
    ├── ai_tests.rs         # per-helper unit tests for AI domain
    └── integration.rs      # purchase-order-approval composition end-to-end
```

**`Cargo.toml` dependency block:**

```toml
[dependencies]
wos-core  = { path = "../wos-core" }
serde     = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
indexmap  = { version = "2", features = ["serde"] }
```

---

## Public API Surface

### `WosProject`

```rust
/// Intent-driven authoring facade for WOS Kernel Documents.
/// All mutations go through helper methods; no raw command access is exposed.
pub struct WosProject { /* private */ }
```

**Construction:**

| Method | Description |
|--------|-------------|
| `WosProject::new() -> Self` | Empty kernel document with mandatory `$wosKernel: "1.0"`. |
| `WosProject::from_document(doc: KernelDocument) -> Self` | Wrap an existing parsed document; initializes history with the document as baseline. |

**Lifecycle helpers (9):**

| Method | Description |
|--------|-------------|
| `add_state(id, kind) -> AuthoringResult` | Add a new `StateKind::Atomic` or `Compound` state. Error if `id` already exists. |
| `add_nested_state(parent_id, id, kind) -> AuthoringResult` | Add a child state inside a compound parent. Error if parent does not exist or is not compound. |
| `remove_state(id) -> AuthoringResult` | Remove a state. Warns if other states' transitions target it. |
| `set_initial_state(state_id) -> AuthoringResult` | Set `lifecycle.initialState`. Error if state does not exist. |
| `add_on_entry_action(state_id, action) -> AuthoringResult` | Append an `Action` to a state's `onEntry` list. |
| `add_on_exit_action(state_id, action) -> AuthoringResult` | Append an `Action` to a state's `onExit` list. |
| `add_tag_to_state(state_id, tag) -> AuthoringResult` | Append a tag string to a state's `tags` list. |
| `add_region(state_id, region_id) -> AuthoringResult` | Add a named orthogonal region inside a compound state. |
| `set_state_description(state_id, description) -> AuthoringResult` | Set the human-readable description field on a state. |

**Transition helpers (5):**

| Method | Description |
|--------|-------------|
| `add_transition(from_state, to_state, event) -> AuthoringResult` | Add a transition from `from_state` to `to_state` triggered by `event`. Both states must exist. |
| `set_transition_guard(from_state, event, guard_expr) -> AuthoringResult` | Set or replace the guard expression on a named transition. Error if transition not found. |
| `set_transition_description(from_state, event, description) -> AuthoringResult` | Set the description on a named transition. |
| `add_tag_to_transition(from_state, event, tag) -> AuthoringResult` | Append a tag to a transition's `tags` list. |
| `remove_transition(from_state, event) -> AuthoringResult` | Remove a transition by source state and event name. |

**Actor helpers (4):**

| Method | Description |
|--------|-------------|
| `add_actor(id, kind) -> AuthoringResult` | Add an actor declaration. `kind` is `ActorKind::Human`, `System`, or `Agent`. Error if `id` already exists. |
| `set_actor_description(actor_id, description) -> AuthoringResult` | Set the `description` field on an actor. Error if actor not found. |
| `add_actor_extension(actor_id, key, value) -> AuthoringResult` | Set an `x-`prefixed extension key on an actor's extension map. Error if key does not start with `x-`. |
| `remove_actor(actor_id) -> AuthoringResult` | Remove an actor. Warns (does not error) if the actor ID is referenced in any transition `assignTo` field. |

**Case file helpers (3):**

| Method | Description |
|--------|-------------|
| `add_case_field(name, schema) -> AuthoringResult` | Add a field to `caseFile.fields`. `schema` is a `serde_json::Value` JSON Schema fragment. Error if field name already exists. |
| `remove_case_field(name) -> AuthoringResult` | Remove a case file field. Warns if the field name appears in any guard expression. |
| `set_correspondence_metadata(field, value) -> AuthoringResult` | Set a top-level metadata field (`title`, `description`, `version`, `status`, `url`). |

**Governance helpers (4):**

| Method | Description |
|--------|-------------|
| `set_impact_level(level) -> AuthoringResult` | Set `impactLevel` on the document. `level` is `ImpactLevel::Operational`, `Significant`, `High`, or `Critical`. |
| `add_contract_reference(name, binding, ref_uri) -> AuthoringResult` | Insert a named entry into `contracts`. Error if `name` already exists. |
| `add_due_process_path(state_id, path_config) -> AuthoringResult` | Set `x-wos-governance.dueProcess` on the document's extensions, scoped to a state. Appends; does not replace existing paths. |
| `add_governance_assertion(assertion_id, condition, message) -> AuthoringResult` | Append an assertion to `x-wos-governance.assertionGates`. Error if `assertion_id` already exists. |

**AI integration helpers (3):**

| Method | Description |
|--------|-------------|
| `add_ai_agent(agent_id, capabilities) -> AuthoringResult` | Append an agent declaration to `x-wos-ai.agents`. Error if `agent_id` already exists. |
| `add_deontic_constraint(constraint_id, rule) -> AuthoringResult` | Append a deontic constraint to `x-wos-ai.deonticConstraints`. Error if `constraint_id` already exists. |
| `set_autonomy_level(agent_id, level) -> AuthoringResult` | Set the autonomy level on a declared AI agent. Error if `agent_id` not found. |

**History helpers:**

| Method | Description |
|--------|-------------|
| `undo() -> bool` | Reverse the last helper call. Returns `true` if there was something to undo. |
| `redo() -> bool` | Re-apply a reversed helper call. Returns `true` if there was something to redo. |
| `can_undo() -> bool` | True if the undo stack has entries. |
| `can_redo() -> bool` | True if the redo stack has entries. |

**Export / diagnostics:**

| Method | Description |
|--------|-------------|
| `export() -> KernelDocument` | Return the current document state as a cloned `KernelDocument`. |
| `export_json() -> serde_json::Value` | Serialize the document to a JSON value (round-trip safe). |
| `diagnostics() -> Vec<AuthoringDiagnostic>` | Return all diagnostics collected during the current session. |

### `AuthoringResult`

```rust
pub struct AuthoringResultOk {
    /// Human-readable summary of what was done.
    pub summary: String,
    /// Which helper was called and with what parameters (for audit log).
    pub helper: String,
    /// Non-fatal warnings (e.g., dangling actor reference on remove_actor).
    pub warnings: Vec<AuthoringWarning>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("state '{id}' not found")]
    StateNotFound { id: String },
    #[error("state '{id}' already exists")]
    StateAlreadyExists { id: String },
    #[error("actor '{id}' not found")]
    ActorNotFound { id: String },
    #[error("actor '{id}' already exists")]
    ActorAlreadyExists { id: String },
    #[error("transition from '{from_state}' on event '{event}' not found")]
    TransitionNotFound { from_state: String, event: String },
    #[error("extension key '{key}' must start with 'x-'")]
    InvalidExtensionKey { key: String },
    #[error("case field '{name}' already exists")]
    CaseFieldAlreadyExists { name: String },
    #[error("contract '{name}' already exists")]
    ContractAlreadyExists { name: String },
    #[error("agent '{agent_id}' already exists")]
    AgentAlreadyExists { agent_id: String },
    #[error("constraint '{constraint_id}' already exists")]
    ConstraintAlreadyExists { constraint_id: String },
    #[error("parent state '{id}' is not a compound state")]
    InvalidParentState { id: String },
}

pub type AuthoringResult = Result<AuthoringResultOk, ProjectError>;
```

---

## Tasks

### Task 1: Scaffold Crate + `WosProject::new()` + Empty-Document Round-Trip

**Files:**
- Create: `crates/wos-authoring/Cargo.toml`
- Create: `crates/wos-authoring/src/lib.rs`
- Create: `crates/wos-authoring/src/project.rs`
- Create: `crates/wos-authoring/src/pipeline.rs` (stub)
- Create: `crates/wos-authoring/src/commands.rs` (stub)
- Create: `crates/wos-authoring/src/history.rs` (stub)
- Create: `crates/wos-authoring/src/handlers/mod.rs` (stub)
- Modify: `wos-spec/Cargo.toml` workspace members

- [ ] **Step 1.1:** Add `crates/wos-authoring` to workspace `members` in the root `Cargo.toml`.

- [ ] **Step 1.2:** Create `Cargo.toml` with the dependency block listed in File Structure above.

- [ ] **Step 1.3:** Implement `WosProject::new()` in `project.rs`. It must produce a minimal valid `KernelDocument` with only `$wosKernel: "1.0"` and an empty `lifecycle.states`. The `WosProject` struct holds the document as its sole field (plus the pipeline/history which are stubbed as no-ops at this stage):

  ```rust
  pub struct WosProject {
      document: KernelDocument,
      history: History,        // stub: empty Vec
      diagnostics: Vec<AuthoringDiagnostic>,
  }
  ```

- [ ] **Step 1.4:** Implement `export() -> KernelDocument` as a `.clone()` of the internal document.

- [ ] **Step 1.5:** Write the first failing test in `tests/round_trip.rs`:

  ```rust
  #[test]
  fn new_document_round_trips() {
      let project = WosProject::new();
      let doc = project.export();
      assert_eq!(doc.wos_kernel, "1.0");
      assert!(doc.lifecycle.states.is_empty());
  }
  ```

  Run it, confirm it fails (crate does not compile yet), implement until it passes.

- [ ] **Step 1.6:** Extend `tests/round_trip.rs` with a `from_document_round_trips` test that loads `fixtures/kernel/purchase-order-approval.json` via `serde_json::from_str`, wraps it in `WosProject::from_document(doc)`, exports, and asserts the `$wosKernel` version and actor count match the source.

- [ ] **Step 1.7:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 1.8:** Commit:
  ```
  feat(wos-authoring): scaffold crate with empty-document round-trip
  ```

---

### Task 2: Command Pipeline + Undo/Redo Foundation

**Files:**
- Implement: `src/commands.rs`
- Implement: `src/history.rs`
- Implement: `src/pipeline.rs`
- Extend: `src/project.rs` — wire `undo()` / `redo()` / `can_undo()` / `can_redo()`

The pipeline is the beating heart of this crate. Every helper method (Tasks 3–5) calls `self.pipeline.dispatch(cmd, &mut self.document)` which applies the command, saves a snapshot for undo, and returns diagnostics. Because helpers are the unit of undo — not commands — each helper takes a pre-dispatch snapshot and registers it as one undo entry regardless of how many internal commands it dispatches.

- [ ] **Step 2.1:** Define the private `Command` enum in `commands.rs`. This enum is `pub(crate)` — never re-exported. Start with stubs for the domains implemented in Tasks 3–5:

  ```rust
  pub(crate) enum Command {
      AddState { id: String, kind: StateKind },
      SetInitialState { state_id: String },
      AddTransition { from: String, to: String, event: String },
      AddActor { id: String, kind: ActorKind },
      SetImpactLevel { level: ImpactLevel },
      // ... one variant per internal operation
  }
  ```

  Keep it flat — no nested command types. The handler module dispatches on these variants.

- [ ] **Step 2.2:** Implement `History` in `history.rs`:

  ```rust
  pub(crate) struct History {
      undo_stack: Vec<KernelDocument>,   // snapshots before each helper call
      redo_stack: Vec<KernelDocument>,
  }

  impl History {
      pub fn push(&mut self, snapshot: KernelDocument) { ... }
      pub fn undo(&mut self, current: KernelDocument) -> Option<KernelDocument> { ... }
      pub fn redo(&mut self, current: KernelDocument) -> Option<KernelDocument> { ... }
  }
  ```

  `push` clears the redo stack. Undo returns the most recent pre-call snapshot; redo returns the most recent reverted snapshot. Cap depth at 100 entries.

- [ ] **Step 2.3:** Implement `Pipeline` in `pipeline.rs` with a single public(crate) method:

  ```rust
  pub(crate) fn execute(
      cmd: Command,
      document: &mut KernelDocument,
  ) -> Result<(), ProjectError> {
      handlers::apply(cmd, document)
  }
  ```

  This is thin on purpose — the `handlers` module does the real work.

- [ ] **Step 2.4:** Wire `WosProject::undo()` / `redo()` using the `History` pair. Each helper method (Task 3 onward) will follow this discipline:

  ```rust
  fn add_state(&mut self, id: &str, kind: StateKind) -> AuthoringResult {
      // 1. Validate before touching state
      if self.document.lifecycle.states.contains_key(id) {
          return Err(ProjectError::StateAlreadyExists { id: id.into() });
      }
      // 2. Snapshot for undo
      let snapshot = self.document.clone();
      // 3. Apply
      Pipeline::execute(Command::AddState { id: id.into(), kind }, &mut self.document)?;
      // 4. Register undo entry
      self.history.push(snapshot);
      Ok(AuthoringResultOk { summary: format!("Added state '{id}'"), ... })
  }
  ```

- [ ] **Step 2.5:** Write failing test in `tests/round_trip.rs`:

  ```rust
  #[test]
  fn undo_reverses_add_state() {
      let mut p = WosProject::new();
      p.add_state("draft", StateKind::Atomic).unwrap();
      assert!(p.export().lifecycle.states.contains_key("draft"));
      p.undo();
      assert!(!p.export().lifecycle.states.contains_key("draft"));
  }
  ```

  Implement until passing.

- [ ] **Step 2.6:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 2.7:** Commit:
  ```
  feat(wos-authoring): command pipeline with undo/redo
  ```

---

### Task 3: Lifecycle + Transition Helpers

**Files:**
- Implement: `src/handlers/lifecycle.rs`
- Implement: `src/handlers/transitions.rs`
- Extend: `src/handlers/mod.rs`
- Extend: `src/project.rs`
- Create: `tests/lifecycle_tests.rs`

Implement the 9 lifecycle helpers and 5 transition helpers from the API surface table. Each handler function has the signature:

```rust
pub(crate) fn apply_add_state(
    id: &str,
    kind: StateKind,
    document: &mut KernelDocument,
) -> Result<(), ProjectError> {
    let state = State {
        kind,
        states: IndexMap::new(),
        transitions: vec![],
        tags: vec![],
        on_entry: vec![],
        on_exit: vec![],
        regions: IndexMap::new(),
        description: None,
        // ... other fields at their zero values
    };
    document.lifecycle.states.insert(id.to_owned(), state);
    Ok(())
}
```

The analogous `formspec-studio-core` pattern: `addField` dispatches `definition.addItem` which calls the `addItem` handler in `packages/formspec-core/src/handlers/definition-items.ts`. Same separation here: `project.rs` validates and coordinates; `handlers/lifecycle.rs` mutates the document.

- [ ] **Step 3.1:** Implement `add_state`, `set_initial_state`, `add_on_entry_action`, `add_on_exit_action`, `add_tag_to_state`, `set_state_description`, `add_region` in `handlers/lifecycle.rs`. Wire each through `Command` + `Pipeline::execute`.

- [ ] **Step 3.2:** Implement `add_nested_state` and `remove_state`. `remove_state` must scan all states' `transitions` to check for references targeting the removed state; emit `AuthoringWarning` if any are found (do not error — the caller may be building a partial document).

- [ ] **Step 3.3:** Implement `add_transition`, `set_transition_guard`, `set_transition_description`, `add_tag_to_transition`, `remove_transition` in `handlers/transitions.rs`. A transition is identified by `(from_state, event)`. When multiple transitions share the same event from the same state (valid for guarded forks, as in the purchase-order fixture), `set_transition_guard` targets the first matching event name that does not yet have a guard; document this behavior in the function's doc comment.

- [ ] **Step 3.4:** Write `tests/lifecycle_tests.rs` with at minimum:
  - `add_state_happy_path` — state appears in export
  - `add_state_duplicate_errors` — returns `StateAlreadyExists`
  - `add_nested_state_inside_compound` — child state appears under parent
  - `add_nested_state_into_atomic_errors` — returns `InvalidParentState`
  - `set_initial_state_unknown_errors` — returns `StateNotFound`
  - `add_transition_both_states_required` — errors if either state missing
  - `remove_state_with_outgoing_transitions_warns` — removes state, returns warning
  - `undo_add_transition_removes_it` — undo of `add_transition` cleans up

- [ ] **Step 3.5:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 3.6:** Commit:
  ```
  feat(wos-authoring): lifecycle helpers — states, transitions, initial state
  ```

---

### Task 4: Actor + Case File + Correspondence Helpers

**Files:**
- Implement: `src/handlers/actors.rs`
- Implement: `src/handlers/provenance.rs`
- Extend: `src/project.rs`
- Create: `tests/actors_tests.rs`

- [ ] **Step 4.1:** Implement `add_actor`, `set_actor_description`, `add_actor_extension`, `remove_actor` in `handlers/actors.rs`. `add_actor_extension` must validate that the key starts with `x-` and return `ProjectError::InvalidExtensionKey` otherwise. `remove_actor` warns (does not error) if the actor ID appears in any transition's `assignTo` field — scan the full state tree to find them.

- [ ] **Step 4.2:** Implement `add_case_field`, `remove_case_field`, `set_correspondence_metadata` in `handlers/provenance.rs`. `set_correspondence_metadata` routes to the appropriate top-level field (`title`, `description`, `version`, `status`, `url`) and returns `ProjectError::InvalidExtensionKey` for any unrecognized field name. `remove_case_field` warns if the removed field name appears in any guard expression (simple substring match is sufficient — not FEL-aware at this layer).

- [ ] **Step 4.3:** Write `tests/actors_tests.rs` with at minimum:
  - `add_actor_human_appears_in_export`
  - `add_actor_duplicate_errors`
  - `actor_extension_key_must_start_with_x`
  - `remove_actor_warns_if_referenced_in_transition`
  - `add_case_field_appears_in_export`
  - `set_correspondence_metadata_title` — sets document title
  - `set_correspondence_metadata_unknown_field_errors`
  - `undo_add_actor_removes_it`

- [ ] **Step 4.4:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 4.5:** Commit:
  ```
  feat(wos-authoring): actor + case file + correspondence helpers
  ```

---

### Task 5: Governance + AI Integration Helpers

**Files:**
- Implement: `src/handlers/governance.rs`
- Implement: `src/handlers/ai.rs`
- Extend: `src/project.rs`
- Create: `tests/governance_tests.rs`
- Create: `tests/ai_tests.rs`

These helpers write into the `extensions` map of the `KernelDocument` under `x-wos-governance` and `x-wos-ai` keys respectively. The design: each helper reads the existing extension value (or starts fresh), applies its mutation to the deserialized sub-object, and writes it back as a `serde_json::Value`. This keeps `wos-authoring` from importing `GovernanceDocument` or `AIIntegrationDocument` for write operations — it authors the extension fields inline. If a consumer later wants full typed access to the governance or AI document, they use `wos-core`'s deserialization directly on the exported JSON.

- [ ] **Step 5.1:** Implement `set_impact_level`, `add_contract_reference`, `add_due_process_path`, `add_governance_assertion` in `handlers/governance.rs`:
  - `set_impact_level` sets `document.impact_level = Some(level)`.
  - `add_contract_reference` inserts into `document.contracts`.
  - `add_due_process_path` reads `document.extensions["x-wos-governance"]`, deserializes to a `serde_json::Value`, appends to `["dueProcess"]["paths"]` (creating the keys if absent), writes back.
  - `add_governance_assertion` does the same for `["assertionGates"]`.

- [ ] **Step 5.2:** Implement `add_ai_agent`, `add_deontic_constraint`, `set_autonomy_level` in `handlers/ai.rs`:
  - `add_ai_agent` reads `document.extensions["x-wos-ai"]`, appends to `["agents"]` array (error if `agent_id` appears in any existing entry's `id` field).
  - `add_deontic_constraint` appends to `["deonticConstraints"]`.
  - `set_autonomy_level` finds the agent by `agent_id` in `["agents"]` and sets its `autonomy` field.

- [ ] **Step 5.3:** Write `tests/governance_tests.rs` with at minimum:
  - `set_impact_level_operational`
  - `add_contract_reference_appears_in_export`
  - `add_contract_reference_duplicate_errors`
  - `add_due_process_path_creates_extension_key`
  - `add_governance_assertion_appends`
  - `add_governance_assertion_duplicate_errors`
  - `undo_add_contract_reference_removes_it`

- [ ] **Step 5.4:** Write `tests/ai_tests.rs` with at minimum:
  - `add_ai_agent_appears_in_extensions`
  - `add_ai_agent_duplicate_errors`
  - `add_deontic_constraint_appends`
  - `set_autonomy_level_updates_agent`
  - `set_autonomy_level_unknown_agent_errors`
  - `undo_add_ai_agent_removes_it`

- [ ] **Step 5.5:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 5.6:** Commit:
  ```
  feat(wos-authoring): governance + AI integration helpers
  ```

---

### Task 6: Integration Test — Compose Purchase-Order-Approval from Scratch

**Files:**
- Create: `tests/integration.rs`

This test is the acceptance criterion that all helper methods compose correctly end-to-end and that the exported document matches a known-good fixture. It mirrors `formspec-studio-core`'s integration tests that build a full form from scratch using helper methods.

- [ ] **Step 6.1:** Read `fixtures/kernel/purchase-order-approval.json` as the reference document. Identify the complete set of helpers needed to reproduce it: 3 actors, 5 lifecycle states, 7 transitions (some with guards), `impactLevel: "operational"`, initial state, and several `onEntry` actions.

- [ ] **Step 6.2:** Write `tests/integration.rs`:

  ```rust
  #[test]
  fn compose_purchase_order_approval_from_scratch() {
      let mut project = WosProject::new();

      // Metadata
      project.set_correspondence_metadata("title", "Purchase Order Approval").unwrap();
      project.set_correspondence_metadata("version", "1.0.0").unwrap();
      project.set_impact_level(ImpactLevel::Operational).unwrap();

      // Actors
      project.add_actor("requester", ActorKind::Human).unwrap();
      project.add_actor("approver", ActorKind::Human).unwrap();
      project.add_actor("procurementSystem", ActorKind::System).unwrap();

      // States
      project.add_state("submitted", StateKind::Atomic).unwrap();
      project.add_state("pendingDirectorApproval", StateKind::Atomic).unwrap();
      project.add_state("approved", StateKind::Atomic).unwrap();
      project.add_state("rejected", StateKind::Atomic).unwrap();
      project.add_state("cancelled", StateKind::Atomic).unwrap();
      project.set_initial_state("submitted").unwrap();

      // Transitions — guarded fork: two `approve` transitions from `submitted`
      project.add_transition("submitted", "approved", "approve").unwrap();
      project.set_transition_guard("submitted", "approve", "caseFile.amount <= 50000").unwrap();
      project.add_transition("submitted", "pendingDirectorApproval", "approve").unwrap();
      project.set_transition_guard("submitted", "approve", "caseFile.amount > 50000").unwrap();
      // ... remaining transitions (reject, cancel, director approve/reject)

      let exported = project.export_json();
      let fixture: serde_json::Value = serde_json::from_str(
          include_str!("../../../fixtures/kernel/purchase-order-approval.json")
      ).unwrap();

      // Assert structural equality on key fields (key ordering may differ)
      assert_eq!(exported["impactLevel"], fixture["impactLevel"]);
      assert_eq!(exported["actors"].as_array().unwrap().len(), 3);
      assert_eq!(
          exported["lifecycle"]["initialState"],
          fixture["lifecycle"]["initialState"]
      );
      for state_id in ["submitted", "approved", "rejected", "cancelled", "pendingDirectorApproval"] {
          assert!(exported["lifecycle"]["states"][state_id].is_object(),
              "state '{state_id}' missing from export");
      }
  }
  ```

- [ ] **Step 6.3:** Run the test; fix guarded-fork transition behavior (Step 3.3) if needed until passing.

- [ ] **Step 6.4:** Add a second integration test: `undo_redo_sequence_is_stable` — call 5 helpers, undo 3 times, redo 2 times, assert state count and actor count match expectations at each step.

- [ ] **Step 6.5:** Run `cargo test -p wos-authoring`. All tests must pass.

- [ ] **Step 6.6:** Commit:
  ```
  test(wos-authoring): integration — compose purchase-order-approval end-to-end
  ```

---

## Why This Matters

WOS currently has a gap at the `formspec-studio-core` layer. `wos-core` provides types, a parser, and the evaluation algorithm — it is the specification's ground truth — but it has no intent-driven authoring API. Any consumer that wants to *build* a WOS Kernel Document must reach directly into `KernelDocument`, manipulate `IndexMap<String, State>` and `Vec<Transition>` by hand, and handle all the validation logic (duplicate IDs, missing states, extension key namespacing) itself.

This is exactly the situation `formspec-core` was in before `formspec-studio-core` was introduced: powerful types, but no authoring seam. The consequence was that `formspec-mcp`'s tool handlers would have had to each reinvent the same intent-to-primitives translation logic — and do so without any undo capability. `formspec-studio-core` fixed that by providing a single translation layer with consistent error codes, undo/redo, and diagnostics.

`wos-authoring` fills the same gap for WOS. Without it, `wos-mcp`'s 20+ tool handlers would each need to duplicate the guard-expression validation, extension-key namespacing, actor-reference integrity checks, and snapshot management that belong in one place. `wos-synth-core`'s `ToolContext` production implementation, which delegates to `wos-mcp`, would inherit that duplication. The authoring-benchmark loop (`wos-bench`) would have no reliable way to build fixture documents programmatically for scoring. `wos-authoring` closes all three of these gaps with one well-bounded crate whose only job is to translate author intent into correct, atomic, reversible document mutations.

---

## WOS-specific pressures that justify this crate

The "Why This Matters" section above frames `wos-authoring` by analogy to `formspec-studio-core`. This section anchors each capability in a concrete WOS-specific pressure — not a Formspec parallel.

1. **Atomic multi-command transactions.** An LLM driving the authoring loop via `wos-mcp` may generate a sequence like "add state X + add transition X→Y + set Y as terminal" that fails validation halfway through. Without atomic batching, the document ends up in a half-migrated state the LLM must diagnose and clean up. With atomic batching at the helper boundary, the whole sequence rolls back on failure, and the LLM receives a single diagnostic it can address in one repair prompt. The users here are both `wos-synth-core`'s repair loop and interactive Claude Desktop users — both are authoring through `wos-mcp` tool calls that each map to one or more helper invocations.

2. **Undo/redo over batches.** `wos-bench` benchmarks multiple authoring attempts per problem statement. Between attempts, it needs to roll back to a known-good baseline cheaply without re-parsing the source document. Undo at helper granularity is faster than re-deserializing from JSON and re-running all preceding helpers from scratch. The beneficiary is exclusively `wos-bench` — the benchmarking harness is the pressure that makes undo worth implementing now.

3. **Intent-driven helpers (the 28 listed).** Each helper encodes a WOS authoring pattern that an LLM would otherwise have to sequence manually from raw type mutations. `add_rights_impacting_decision`, for example, composes 4–6 primitives (state + transitions + governance assertion + impact level + actor assignment) that have specific co-occurrence constraints in the WOS spec. An LLM calling primitives one at a time in a multi-turn loop has more opportunities to violate these constraints mid-sequence. Consolidating in `wos-authoring` means `wos-mcp` tool handlers stay to ≤5 lines and the LLM makes one tool call instead of six, receiving one structured result instead of six incremental responses. The user is any LLM author via `wos-mcp`.

4. **Command pipeline (dispatch + handlers + diagnostics).** A dispatch-style mutation model gives a single chokepoint for instrumentation that multiple WOS consumers need independently: `wos-bench` needs trace emission and metric counters to score iteration quality; future `wos-studio` (GUI authoring, on the roadmap but out of scope here) needs the same mutation events for live preview. Centralizing in the pipeline means neither consumer has to monkey-patch the authoring loop or add tracing ad hoc inside individual helpers.

> **If any of these pressures disappears, revisit the scope of `wos-authoring`.** In particular: if `wos-bench` never lands (Q1 is reversed), undo/redo can be trimmed. If `wos-mcp` consumers never ask for multi-primitive helpers, the helper set can shrink to 1:1 wrappers over `wos-core` primitives (but keep the crate as the thin wrapper layer — `wos-mcp` tool handlers still benefit from separation).

---

## Estimated Effort

~2 engineer-weeks for the complete crate including all 28 helpers, round-trip tests, and the integration test. The pipeline and history foundation (Task 2) is the riskiest piece — the undo-at-helper-granularity discipline needs to be established correctly before any helpers land. Tasks 3–5 are mechanical given the foundation. Task 6 is the validation gate; if the integration test is hard to make pass, the most likely cause is the guarded-fork transition behavior needing refinement.

**Helper count summary:** 9 lifecycle + 5 transitions + 4 actors + 3 case file/correspondence + 4 governance + 3 AI integration = **28 total intent-driven helper methods**.
