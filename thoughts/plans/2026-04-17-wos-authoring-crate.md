# wos-authoring Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `wos-authoring` — the intent-driven authoring helper layer over `wos-core`. Provides a `RawWosProject` (command dispatch + undo/redo + normalization) and a higher-level `WosProject` (50+ helper methods translating author intent into batched commands). Analogous to `packages/formspec-studio-core` over `packages/formspec-core` in the parent project.

**Architecture:** Two-tier composition.

- `RawWosProject` (in `raw.rs`): transactional state machine. Holds the in-memory representation of a WOS kernel document (plus optional governance / AI / advanced sidecars). Accepts typed `Command` enum values, dispatches them through a handler pipeline, maintains an undo/redo stack, emits diagnostics on invalid commands.
- `WosProject` (in `project.rs`): helper façade. Takes a `RawWosProject` via `IWosProjectCore` interface. Exposes intent-level methods: `add_state`, `add_transition`, `add_actor`, `set_impact_level`, `add_due_process_path`, `add_agent`, `emit_event`, `enter_state`, etc. Each helper translates to N low-level commands in one atomic batch.

The README of `packages/formspec-studio-core` forbids consumers from importing `formspec-core` directly; `wos-authoring` inherits the same discipline — downstream crates (wos-mcp, wos-synth-core, wos-bench) consume `wos-authoring` and never reach past it into `wos-core`.

**Tech Stack:** Rust (new `wos-authoring` crate), serde, serde_json, thiserror, tracing, wos-core (existing).

**Spec anchor:** Architectural decision from 2026-04-17 — the authoring stack (core → authoring → mcp → synth-core) mirrors Formspec's (types → engine → core → studio-core → mcp → chat) layer for layer. See ADR 0065 in the parent repo (landing in parallel with this plan).

---

## Prerequisites

- `wos-core` exists (it does) with `KernelDocument`, `Actor`, `State`, `Transition`, `ImpactLevel`, `ContractReference`, and related model types.
- Decision to mirror Formspec's studio-core pattern (2026-04-17, ADR 0065 parent repo).
- No cyclic deps; `wos-authoring` depends on `wos-core` only.

## Completion criteria

1. `crates/wos-authoring/` compiles as a workspace member; `cargo check --workspace` is green.
2. `IWosProjectCore` trait defined; `RawWosProject` struct implements it.
3. At least 10 command variants implemented (`AddState`, `AddTransition`, `AddActor`, `SetImpactLevel`, `AddContract`, `RemoveState`, `RenameState`, `AddActorDeontic`, `SetTimer`, `AddExtensionKey`) with unit tests for each.
4. `WosProject` helper layer with at least 5 intent-level methods demonstrating the composition pattern.
5. Undo/redo works for at least 3 command types (verified by tests).
6. Public API surface documented in a crate-level `README.md`.
7. `cargo test -p wos-authoring` is green.

## File structure

- `crates/wos-authoring/Cargo.toml` — deps: `wos-core`, `serde`, `serde_json`, `thiserror`, `tracing`.
- `crates/wos-authoring/src/lib.rs` — public re-exports; module declarations; crate-level doc comment.
- `crates/wos-authoring/src/command.rs` — `Command` enum + `CommandResult` + `AppliedCommand`.
- `crates/wos-authoring/src/raw.rs` — `RawWosProject` struct + `IWosProjectCore` trait.
- `crates/wos-authoring/src/handlers.rs` — one pure handler function per command variant.
- `crates/wos-authoring/src/history.rs` — `HistoryManager` ring buffer with undo/redo stacks.
- `crates/wos-authoring/src/project.rs` — `WosProject` helper façade.
- `crates/wos-authoring/src/diagnostics.rs` — `AuthoringDiagnostic` error type for command rejections.
- `crates/wos-authoring/README.md` — public API docs + boundary statement.
- `crates/wos-authoring/tests/authoring_session.rs` — end-to-end integration test.

---

## Task 1: Scaffold the crate

**Files:**
- Create: `crates/wos-authoring/Cargo.toml`
- Create: `crates/wos-authoring/src/lib.rs` (stub)
- Modify: root `Cargo.toml` — add `crates/wos-authoring` to workspace `members`.

- [ ] **Step 1.1:** Add `"crates/wos-authoring"` to the `members` array in root `Cargo.toml`.

- [ ] **Step 1.2:** Create `crates/wos-authoring/Cargo.toml`:

```toml
[package]
name = "wos-authoring"
version = "0.1.0"
edition = "2021"

[dependencies]
wos-core = { path = "../wos-core" }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = "1"
tracing = "0.1"
indexmap = "2"
```

- [ ] **Step 1.3:** Create `crates/wos-authoring/src/lib.rs` with a module skeleton and crate doc comment:

```rust
//! Intent-driven authoring helpers for WOS documents.
//!
//! Two-tier API:
//! - [`RawWosProject`] — raw command dispatch, undo/redo, and normalization.
//! - [`WosProject`] — high-level helpers that translate author intent into
//!   batched commands dispatched through the raw tier.
//!
//! Downstream crates (wos-mcp, wos-synth-core, wos-bench) import from
//! `wos-authoring` only. Never bypass this crate to reach `wos-core` directly.

pub mod command;
pub mod diagnostics;
pub mod handlers;
pub mod history;
pub mod project;
pub mod raw;

pub use command::{AppliedCommand, Command, CommandResult};
pub use diagnostics::AuthoringDiagnostic;
pub use project::WosProject;
pub use raw::{IWosProjectCore, RawWosProject};
```

- [ ] **Step 1.4:** Verify `cargo check -p wos-authoring` compiles with no errors (even with empty module stubs).

- [ ] **Step 1.5:** Commit. `build(wos-authoring): scaffold crate with wos-core dependency`.

---

## Task 2: `Command` enum + `CommandResult`

**Files:**
- Create: `crates/wos-authoring/src/command.rs`
- Create: `crates/wos-authoring/src/diagnostics.rs`

Define the core command surface and result types first — all subsequent work builds on them.

- [ ] **Step 2.1:** Define `AuthoringDiagnostic` in `diagnostics.rs`:

```rust
use thiserror::Error;

/// Emitted when a command is rejected by the handler pipeline.
#[derive(Debug, Error)]
pub enum AuthoringDiagnostic {
    #[error("state '{id}' already exists")]
    DuplicateState { id: String },
    #[error("state '{id}' not found")]
    StateNotFound { id: String },
    #[error("actor '{id}' already exists")]
    DuplicateActor { id: String },
    #[error("actor '{id}' not found")]
    ActorNotFound { id: String },
    #[error("transition from '{from_state}' on event '{event}' already exists")]
    DuplicateTransition { from_state: String, event: String },
    #[error("contract '{name}' already exists")]
    DuplicateContract { name: String },
    #[error("extension key '{key}' must start with 'x-'")]
    InvalidExtensionKey { key: String },
    #[error("invalid impact level: '{value}'")]
    InvalidImpactLevel { value: String },
}
```

- [ ] **Step 2.2:** Define the `Command` enum in `command.rs` with at least 10 variants:

```rust
use crate::diagnostics::AuthoringDiagnostic;
use wos_core::{Actor, ActorKind, ImpactLevel, StateKind};
use serde::{Deserialize, Serialize};

/// All mutations that can be applied to a WOS project in authoring mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    /// Add a new state to the kernel lifecycle.
    AddState { id: String, kind: StateKind, label: Option<String> },
    /// Remove a state and all transitions referencing it.
    RemoveState { id: String },
    /// Rename a state ID everywhere it is referenced.
    RenameState { old_id: String, new_id: String },
    /// Add a transition from one state to another, triggered by an event.
    AddTransition { from_state: String, event: String, target: String },
    /// Add an actor declaration to the kernel.
    AddActor { id: String, kind: ActorKind, label: Option<String> },
    /// Set the kernel-level impact classification.
    SetImpactLevel { level: ImpactLevel },
    /// Add a named contract reference.
    AddContract { name: String, binding: String, reference: String },
    /// Add or update a deontic obligation on a named actor.
    AddActorDeontic { actor_id: String, deontic_kind: String, constraint: String },
    /// Set a timer configuration on a named state.
    SetTimer { state_id: String, duration_iso: String, event: String },
    /// Add or update an extension key (`x-*`) at the document root.
    AddExtensionKey { key: String, value: serde_json::Value },
}

/// The inverse of a successfully applied command (stored for undo).
#[derive(Debug, Clone)]
pub struct AppliedCommand {
    pub command: Command,
}

/// The result of dispatching a single command.
pub type CommandResult = Result<AppliedCommand, AuthoringDiagnostic>;
```

- [ ] **Step 2.3:** Write a unit test for each variant confirming it roundtrips through `serde_json` without error.

- [ ] **Step 2.4:** `cargo test -p wos-authoring` green.

- [ ] **Step 2.5:** Commit. `feat(wos-authoring): Command enum with 10 variants + CommandResult`.

---

## Task 3: `IWosProjectCore` trait + `RawWosProject`

**Files:**
- Create: `crates/wos-authoring/src/raw.rs`

- [ ] **Step 3.1:** Define the `IWosProjectCore` trait — the seam that `WosProject` and downstream crates depend on:

```rust
use wos_core::KernelDocument;
use crate::command::{Command, CommandResult};
use crate::diagnostics::AuthoringDiagnostic;

/// The seam between the raw dispatch layer and the helper façade.
///
/// `RawWosProject` is the only concrete implementation.
/// `WosProject` (wos-authoring) and downstream crates (wos-mcp) depend
/// on this trait, never on the concrete struct.
pub trait IWosProjectCore {
    /// Dispatch one command. Returns the applied command or a diagnostic.
    fn dispatch(&mut self, command: Command) -> CommandResult;
    /// Dispatch multiple commands as one atomic batch.
    /// If any command fails, all preceding commands in the batch are rolled back.
    fn batch(&mut self, commands: Vec<Command>) -> Result<Vec<AppliedCommand>, AuthoringDiagnostic>;
    /// Undo the last applied command. Returns false if history is empty.
    fn undo(&mut self) -> bool;
    /// Redo the last undone command. Returns false if redo stack is empty.
    fn redo(&mut self) -> bool;
    /// Whether undo is available.
    fn can_undo(&self) -> bool;
    /// Whether redo is available.
    fn can_redo(&self) -> bool;
    /// Read-only access to the current kernel document.
    fn kernel(&self) -> &KernelDocument;
    /// Replace the in-memory kernel document wholesale (e.g. for load from disk).
    fn load(&mut self, kernel: KernelDocument);
    /// Export the current kernel document (cloned, ready for serialization).
    fn export(&self) -> KernelDocument;
}
```

- [ ] **Step 3.2:** Implement `RawWosProject`:

```rust
pub struct RawWosProject {
    kernel: KernelDocument,
    history: crate::history::HistoryManager,
}

impl RawWosProject {
    pub fn new() -> Self { ... }
    pub fn from_kernel(kernel: KernelDocument) -> Self { ... }
}

impl IWosProjectCore for RawWosProject { ... }
```

`dispatch` delegates to `crate::handlers::apply_command`. On success, it pushes a snapshot to `HistoryManager`. On failure, it rolls back the in-progress mutation and returns the diagnostic.

- [ ] **Step 3.3:** Write tests: dispatch a known-good command and verify `kernel()` reflects the change; dispatch a known-bad command and verify the kernel is unchanged.

- [ ] **Step 3.4:** `cargo test -p wos-authoring` green.

- [ ] **Step 3.5:** Commit. `feat(wos-authoring): RawWosProject implements IWosProjectCore`.

---

## Task 4: Handler pipeline — one function per command

**Files:**
- Create: `crates/wos-authoring/src/handlers.rs`

Handlers are pure functions: they take the current `KernelDocument` by mutable reference and the command parameters, mutate the document, and return `Result<AppliedCommand, AuthoringDiagnostic>`.

- [ ] **Step 4.1:** Implement a top-level dispatch function:

```rust
pub fn apply_command(
    kernel: &mut KernelDocument,
    command: Command,
) -> CommandResult {
    match command {
        Command::AddState { id, kind, label } => apply_add_state(kernel, id, kind, label),
        Command::RemoveState { id } => apply_remove_state(kernel, id),
        Command::RenameState { old_id, new_id } => apply_rename_state(kernel, old_id, new_id),
        Command::AddTransition { from_state, event, target } => {
            apply_add_transition(kernel, from_state, event, target)
        }
        Command::AddActor { id, kind, label } => apply_add_actor(kernel, id, kind, label),
        Command::SetImpactLevel { level } => apply_set_impact_level(kernel, level),
        Command::AddContract { name, binding, reference } => {
            apply_add_contract(kernel, name, binding, reference)
        }
        Command::AddActorDeontic { actor_id, deontic_kind, constraint } => {
            apply_add_actor_deontic(kernel, actor_id, deontic_kind, constraint)
        }
        Command::SetTimer { state_id, duration_iso, event } => {
            apply_set_timer(kernel, state_id, duration_iso, event)
        }
        Command::AddExtensionKey { key, value } => apply_add_extension_key(kernel, key, value),
    }
}
```

- [ ] **Step 4.2:** Implement each handler function. Validation rules:
  - `apply_add_state`: reject if `id` already exists in `lifecycle.states`.
  - `apply_remove_state`: reject if `id` not found; cascade-remove transitions that target this state.
  - `apply_rename_state`: reject if `old_id` not found or `new_id` already exists; rewrite transition targets.
  - `apply_add_transition`: reject if `from_state` or `target` state not found; reject duplicate `(from_state, event)` pairs.
  - `apply_add_actor`: reject if `id` already exists in `actors`.
  - `apply_set_impact_level`: always succeeds (replaces existing value).
  - `apply_add_contract`: reject if `name` already exists in `contracts`.
  - `apply_add_actor_deontic`: reject if `actor_id` not found; append deontic to actor's obligation list.
  - `apply_set_timer`: reject if `state_id` not found; write timer config into state extension.
  - `apply_add_extension_key`: reject if `key` does not start with `x-`; upsert into `extensions`.

- [ ] **Step 4.3:** Write at least one unit test per handler covering both the happy path and the primary rejection case.

- [ ] **Step 4.4:** `cargo test -p wos-authoring` green.

- [ ] **Step 4.5:** Commit. `feat(wos-authoring): handler pipeline — 10 command handlers with validation`.

---

## Task 5: Undo/redo history

**Files:**
- Create: `crates/wos-authoring/src/history.rs`

- [ ] **Step 5.1:** Implement `HistoryManager` as a ring buffer of `KernelDocument` snapshots:

```rust
pub struct HistoryManager {
    /// Snapshots in chronological order. The last entry is the current state before
    /// the most recent command — pop it on undo.
    undo_stack: Vec<KernelDocument>,
    /// Snapshots popped by undo — push back on redo, clear on any new dispatch.
    redo_stack: Vec<KernelDocument>,
    max_depth: usize,
}

impl HistoryManager {
    pub fn new(max_depth: usize) -> Self { ... }
    /// Push a pre-command snapshot so it can be restored on undo.
    pub fn push_undo(&mut self, snapshot: KernelDocument) { ... }
    /// Pop the most recent undo snapshot (returns None if empty).
    pub fn pop_undo(&mut self) -> Option<KernelDocument> { ... }
    /// Pop the most recent redo snapshot.
    pub fn pop_redo(&mut self) -> Option<KernelDocument> { ... }
    /// Push a snapshot onto the redo stack (called during undo).
    pub fn push_redo(&mut self, snapshot: KernelDocument) { ... }
    /// Clear the redo stack (called when a new command is dispatched after undos).
    pub fn clear_redo(&mut self) { ... }
    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
}
```

- [ ] **Step 5.2:** Wire `HistoryManager` into `RawWosProject.dispatch`:
  1. Clone the current kernel → `snapshot`.
  2. Apply the command.
  3. On success: `history.clear_redo()`, `history.push_undo(snapshot)`.
  4. On failure: leave history untouched (kernel unchanged, no snapshot pushed).

- [ ] **Step 5.3:** Implement `RawWosProject::undo`:
  1. Call `history.pop_undo()` → `prev`.
  2. `history.push_redo(self.kernel.clone())`.
  3. Replace `self.kernel` with `prev`.
  Returns `false` if undo stack was empty.

- [ ] **Step 5.4:** Write tests covering at least 3 command types:
  - `AddState` + undo → state gone; redo → state back.
  - `SetImpactLevel` + undo → original level restored; redo → new level back.
  - `AddTransition` + undo → transition gone; redo → transition back.
  - Ring buffer cap: after `max_depth + 5` commands, `undo_stack.len() == max_depth`.

- [ ] **Step 5.5:** `cargo test -p wos-authoring` green.

- [ ] **Step 5.6:** Commit. `feat(wos-authoring): undo/redo history with ring-buffer semantics`.

---

## Task 6: `WosProject` helper façade

**Files:**
- Create: `crates/wos-authoring/src/project.rs`

`WosProject` wraps any `IWosProjectCore` implementation. It exposes intent-level helpers — each helper translates to one or more `Command` dispatches batched atomically. If any command in a batch fails, the entire batch is rolled back.

- [ ] **Step 6.1:** Implement the struct and constructor:

```rust
/// High-level authoring API for WOS documents.
///
/// Wraps a [`RawWosProject`] through the [`IWosProjectCore`] interface.
/// All authoring goes through helper methods — never call `dispatch` directly.
pub struct WosProject {
    core: Box<dyn IWosProjectCore>,
}

impl WosProject {
    pub fn new() -> Self {
        Self { core: Box::new(RawWosProject::new()) }
    }
    pub fn from_kernel(kernel: KernelDocument) -> Self {
        Self { core: Box::new(RawWosProject::from_kernel(kernel)) }
    }
}
```

- [ ] **Step 6.2:** Implement at least 5 intent-level helper methods:

**`create_kernel_document(title, impact_level) -> &mut Self`**
Sets `kernel.title` and dispatches `SetImpactLevel`. Convenience for first-touch initialization.

**`add_state_with_transitions(id, kind, transitions_out) -> Result<(), AuthoringDiagnostic>`**
Batches `AddState` + one `AddTransition` per entry in `transitions_out`. If any transition references an unknown target state, the batch rolls back and returns the diagnostic.

**`add_actor_with_deontics(id, kind, label, deontics) -> Result<(), AuthoringDiagnostic>`**
Batches `AddActor` + one `AddActorDeontic` per entry in `deontics`.

**`add_rights_impacting_decision(state_id, actor_id, notice_event, decision_event) -> Result<(), AuthoringDiagnostic>`**
Batches: `AddState` (decision state), `AddTransition` (notice → decision), `AddTransition` (decision → terminal), `AddActorDeontic` (actor obligation to decide). This is the pattern the WOS spec mandates for rights-impacting decisions.

**`add_agent_participant(id, label, capabilities) -> Result<(), AuthoringDiagnostic>`**
Batches `AddActor` (kind: Agent) + `AddExtensionKey` for each capability entry.

- [ ] **Step 6.3:** Delegate undo/redo and read accessors directly to `core`:

```rust
pub fn undo(&mut self) -> bool { self.core.undo() }
pub fn redo(&mut self) -> bool { self.core.redo() }
pub fn can_undo(&self) -> bool { self.core.can_undo() }
pub fn can_redo(&self) -> bool { self.core.can_redo() }
pub fn kernel(&self) -> &KernelDocument { self.core.kernel() }
pub fn export(&self) -> KernelDocument { self.core.export() }
```

- [ ] **Step 6.4:** Write one test per helper method. Each test creates a fresh `WosProject`, calls the helper, and asserts the exported kernel matches expectations.

- [ ] **Step 6.5:** `cargo test -p wos-authoring` green.

- [ ] **Step 6.6:** Commit. `feat(wos-authoring): WosProject façade with 5 intent-level helpers`.

---

## Task 7: Public README + API boundary enforcement

**Files:**
- Create: `crates/wos-authoring/README.md`

The README should match the structure and discipline of `packages/formspec-studio-core/README.md`.

- [ ] **Step 7.1:** Write `README.md` covering:

  1. **What this crate is.** One paragraph: `wos-authoring` is the intent-driven authoring layer for WOS documents. It wraps `wos-core`'s raw model types with a transactional command pipeline, undo/redo, and high-level helpers that translate author intent into batched mutations. This is the layer that `wos-mcp` exposes as MCP tools.

  2. **Quick Start.** Show creating a `WosProject`, calling `add_state_with_transitions`, `add_actor_with_deontics`, calling `export()`, and serializing to JSON.

  3. **Architecture.** Reproduce the two-tier diagram:
     ```
     WosProject (project.rs)            — helper façade
       └─ core: Box<dyn IWosProjectCore>
            └─ RawWosProject (raw.rs)   — command dispatch + undo/redo
                 └─ wos-core            — typed model + evaluator (read-only to authoring)
     ```

  4. **Layering rule (mandatory callout box).** "Consumers of `wos-authoring` — including `wos-mcp`, `wos-synth-core`, and `wos-bench` — MUST import all WOS model types and helpers from `wos-authoring`. Never reach past this crate into `wos-core` directly. `wos-authoring` re-exports everything downstream needs."

  5. **Command catalog.** Table listing all 10 commands with a one-line description each.

  6. **Undo/redo.** Short note on the ring-buffer depth default (50 snapshots) and how to override it.

  7. **Development.** `cargo build -p wos-authoring` and `cargo test -p wos-authoring`.

- [ ] **Step 7.2:** Verify README markdown renders without broken links.

- [ ] **Step 7.3:** Commit. `docs(wos-authoring): README with boundary enforcement + WosProject quickstart`.

---

## Task 8: Integration test — multi-command authoring session

**Files:**
- Create: `crates/wos-authoring/tests/authoring_session.rs`

End-to-end test: create a project from scratch, author a non-trivial workflow, export it, and verify the exported document roundtrips through `wos-core`'s `KernelDocument` deserializer.

- [ ] **Step 8.1:** Write the test:

```rust
use wos_authoring::WosProject;
use wos_core::{ActorKind, ImpactLevel, KernelDocument, StateKind};

#[test]
fn multi_command_session_roundtrips_through_wos_core() {
    let mut project = WosProject::new();

    project.create_kernel_document("Benefits Appeal", ImpactLevel::High);

    project.add_actor_with_deontics(
        "caseworker",
        ActorKind::Human,
        Some("Case Worker".to_string()),
        vec![("must-review", "obligation")],
    ).expect("add_actor_with_deontics");

    project
        .add_state_with_transitions(
            "submitted",
            StateKind::Normal,
            vec![("appeal.received", "under_review")],
        )
        .expect("add submitted state");

    project
        .add_state_with_transitions(
            "under_review",
            StateKind::Normal,
            vec![("decision.issued", "closed")],
        )
        .expect("add under_review state");

    project
        .add_state_with_transitions("closed", StateKind::Final, vec![])
        .expect("add closed state");

    project
        .add_rights_impacting_decision(
            "under_review",
            "caseworker",
            "notice.sent",
            "decision.issued",
        )
        .expect("add rights-impacting decision");

    // Export and roundtrip through wos-core deserialization.
    let exported = project.export();
    let serialized = serde_json::to_string(&exported).expect("serialize");
    let roundtripped: KernelDocument =
        serde_json::from_str(&serialized).expect("deserialize");

    assert!(roundtripped.actors.iter().any(|a| a.id == "caseworker"));
    assert!(roundtripped.lifecycle.states.contains_key("submitted"));
    assert!(roundtripped.lifecycle.states.contains_key("under_review"));
    assert!(roundtripped.lifecycle.states.contains_key("closed"));
    assert_eq!(roundtripped.impact_level, Some(ImpactLevel::High));
}
```

- [ ] **Step 8.2:** Run `cargo test -p wos-authoring` — confirm the integration test passes.

- [ ] **Step 8.3:** Add a second test that exercises undo across the session: undo until empty, verify the kernel is back to its initial (empty) state; redo all the way forward, verify full state is restored.

- [ ] **Step 8.4:** `cargo test -p wos-authoring` green.

- [ ] **Step 8.5:** Commit. `test(wos-authoring): end-to-end multi-command session round-trip test`.

---

## Self-review checklist

- All 10 Task 2 commands land with at least 1 unit test each (happy path + primary rejection).
- `IWosProjectCore` trait is the ONLY public interface into `RawWosProject`; the struct's fields are private.
- Undo/redo is tested across at least 3 command types (Task 5).
- `WosProject` façade has at least 5 intent-level helpers; each helper is one batched transaction (Task 6).
- No `pub use wos_core::*` re-exports that expose `wos-core`'s internals; only the types downstream actually need.
- README explicitly states the "never import from wos-core" rule (Task 7).
- `cargo test -p wos-authoring` is green with no warnings (all tasks).
- `cargo check --workspace` is green (Task 1).

## Why this matters

Without `wos-authoring`, `wos-mcp` tool handlers would either duplicate intent-to-command translation across 25+ MCP tool endpoints or operate at the raw `wos-core` model level (harder to use, harder to test, harder to keep consistent). Formspec proved the value of the dual-tier split: `formspec-core` (130 raw commands) + `formspec-studio-core` (51+ intent-level helpers) back 27 MCP tools in `formspec-mcp`, each of which wraps a helper rather than assembling raw commands. WOS inherits that pattern. This is the foundation that makes `wos-mcp` feasible as a thin adapter and keeps `wos-synth-core`'s loop focused on prompting rather than document construction.

**Estimated effort:** approximately 1.5 engineer-weeks (scaffold + 10 commands + handlers + history + 5 helpers + tests + README).
