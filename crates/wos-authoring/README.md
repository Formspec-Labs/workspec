# wos-authoring

Intent-driven authoring API for WOS Kernel Documents.

## Purpose

`wos-authoring` is the single seam every mutation to a `KernelDocument`
flows through. It sits between `wos-core` (types, parser, evaluator —
no mutation API) and its consumers (`wos-mcp` today; future
`wos-synth-core`, `wos-bench`, `wos-studio`). Callers name intents
(`add_state`, `set_impact_level`, `add_actor_deontic`, …) and receive
structured results plus undo/redo — they never touch the underlying
`IndexMap<String, State>` or the `Command` dispatch enum.

This crate is to `wos-core` what `formspec-studio-core` is to
`formspec-core`: an atomic, reversible, intent-driven authoring layer
over a typed but inert core.

## Layering

```
┌─────────────────────────────────────────────────────────────┐
│ wos-mcp / wos-synth-core / wos-bench  (consumers)           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ wos-authoring::WosProject            (this crate — layer 1) │
│   add_state, add_transition, add_actor, set_impact_level,   │
│   add_contract, add_actor_deontic, set_timer,               │
│   add_milestone, add_extension_key, undo, redo              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ wos-core::KernelDocument              (typed model, layer 0)│
└─────────────────────────────────────────────────────────────┘
```

`wos-authoring` depends only on `wos-core`. Consumers that need
authoring must depend on `wos-authoring`, not on `wos-core`
mutation APIs (there are none to depend on) or the
internal `Command` enum (it is `pub(crate)`).

## Public surface

Exactly one façade — `WosProject` — plus the types needed to describe
results and call site parameters:

| Symbol                  | Purpose                                                   |
| ----------------------- | --------------------------------------------------------- |
| `WosProject`            | The only entry point for mutation.                        |
| `AuthoringResult`       | `Result<AppliedCommand, AuthoringDiagnostic>`.            |
| `AuthoringDiagnostic`   | Structured error or warning with JSON-pointer-style path. |
| `Severity`              | `Error` or `Warning`.                                     |
| `ActorKind`             | Re-exported from `wos-core` (`Human` or `System`).        |
| `ImpactLevel`           | Re-exported from `wos-core` (4 kernel §S6 variants).      |
| `StateKind`             | Re-exported from `wos-core`.                              |

**Not exposed:** `Command`, `RawWosProject`, `IWosProjectCore`,
`AppliedCommand`, `CommandResult`, `dispatch`. These are `pub(crate)`
so the façade is the only path to mutation.

## Usage

```rust
use wos_authoring::{ActorKind, ImpactLevel, StateKind, WosProject};

let mut project = WosProject::new(ImpactLevel::Operational, "Purchase Order");

project.add_state("submitted", StateKind::Atomic)?;
project.add_state("approved", StateKind::Atomic)?;
project.add_actor("approver", ActorKind::Human)?;
project.add_transition("submitted", "approved", Some("approve".into()), None)?;

// Undo/redo at helper-call granularity.
project.undo()?;
assert!(!project.snapshot().lifecycle.states.contains_key("approved"));
project.redo()?;

let document = project.snapshot();
```

## Undo / redo model

Every successful helper call captures a pre-command snapshot of the
full `KernelDocument` onto a bounded history stack (default depth
100). Undo swaps the snapshot back in and pushes the current state
onto the redo stack; redo mirrors the move. A forward helper call
after an undo clears the redo branch — there is no split-timeline
replay. Failed helper calls do not touch either stack since the
document itself was never mutated.

Snapshot-based restoration was chosen over per-command inversion
because WOS commands touch nested lifecycle states, the extension
map, and the contracts map — inverting every shape symmetrically
would require a second handler for each variant. Inverses recorded
on `AppliedCommand` remain available for future optimizations.

## Extension points

The crate is deliberately narrow. If you need to add a new
authoring operation:

1. **Add a `Command` variant** in `src/command.rs`. Keep it flat —
   no nested command types.
2. **Implement the handler** as a private method on `RawWosProject`
   in `src/raw.rs`, following the pattern:
   ```rust
   fn apply_add_thing(&mut self, id: String) -> CommandResult {
       // 1. Validate (return Err(AuthoringDiagnostic::error(...)) on rejection).
       // 2. Mutate self.doc.
       // 3. Return Ok(AppliedCommand::with_inverse(label, inverse))
       //    or AppliedCommand::without_inverse(label).
   }
   ```
3. **Wire it into `dispatch`** (the `match cmd { ... }` in `raw.rs`).
4. **Expose a façade helper** on `WosProject` in `src/project.rs`:
   ```rust
   pub fn add_thing(&mut self, id: impl Into<String>) -> AuthoringResult {
       self.core.dispatch(Command::AddThing { id: id.into() })
   }
   ```
5. **Write tests at both layers**: a raw-level unit test in
   `raw.rs::tests` and a façade-level test in `project.rs::tests`.
   Red-green-refactor — write the failing test first.

Snapshot-based undo works automatically; no extra code is required
to make a new command undoable.

## Running tests

```bash
cargo check -p wos-authoring --tests    # zero warnings required
cargo test -p wos-authoring              # unit + integration tests
```

## Design choices

- **No public `dispatch`.** The `Command` enum is internal.
  Consumers call named helpers so the public API stays stable as
  commands evolve.
- **Extensions as `serde_json::Value`.** Handlers that write into
  `x-wos-ai` / `x-wos-timers` / `x-wos-governance` author the
  extension fields inline rather than importing the typed companion
  documents (`GovernanceDocument`, `AIIntegrationDocument`). This
  keeps the dependency graph simple; consumers who need typed access
  deserialize the exported JSON through `wos-core` directly.
- **AI agents are not actors.** `ActorKind` is `Human | System` per
  kernel §S3. AI agents live in `x-wos-ai.agents` and are created
  by the forthcoming `add_ai_agent` helper; custom actor categories
  go through the §10.6 `actorExtension` seam.
