# WOS MCP Crate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `wos-mcp` — a thin MCP adapter over `wos-authoring` that exposes WOS authoring operations as 22 tools to external agents (Claude Desktop, Cline, etc.) via stdio, and simultaneously exposes the same handlers via an in-process dispatch entry point for direct consumption by `wos-synth-core`'s `ToolContext` implementation.

**Architecture:** Two entry points, one set of handlers. Tool handlers live in `crates/wos-mcp/src/tools/` and are pure functions: they accept a `&ProjectRegistry`, a project ID, and a `serde_json::Value` argument bag, then return a `Result<Value, ToolError>`. The MCP stdio entry point (`server.rs`) wraps these handlers in a JSON-RPC-2.0 loop. The in-process entry point (`dispatch.rs`) exposes `wos_mcp::dispatch(tool_name, args)` as a plain library function — no subprocess, no socket, no serialization overhead. This is the same dual-entry pattern used by the parent Formspec project: `packages/formspec-mcp/src/create-server.ts` is the protocol entry; `packages/formspec-mcp/src/dispatch.ts` (labeled "in-process tool dispatch") calls the same handlers without transport. `wos-mcp` contains ZERO business logic — every tool handler delegates immediately to a `wos-authoring::WosProject` method or to `wos-lint` / `wos-conformance` helpers.

**Tech Stack:** Rust (new `wos-mcp` crate), `wos-authoring`, `wos-lint`, `wos-conformance`, `serde`, `serde_json`, `tokio` (async I/O for stdio transport), `thiserror` (typed errors), optionally `tracing`. Rust MCP SDK ecosystem is an open question — see Open Questions.

**Spec anchor:** Formspec parallel at `packages/formspec-mcp/` and open-questions Q6 at `../archive/reviews/2026-04-16-architecture-review-open-questions.md`. Q6 resolved that `wos-synth` (the authoring loop) is a separate crate from `wos-bench` (the benchmark harness); `wos-mcp` is the missing layer that gives both the authoring loop and external agents a single, stable interface to `wos-authoring` helpers.

**Related:**
- `../../../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md` — forward-reference; ADR is being landed separately.
- `./2026-04-17-wos-authoring-crate.md` — `wos-mcp`'s tool handlers are thin wrappers over `wos-authoring::WosProject` methods. `wos-authoring` must exist first; `wos-mcp` tools land incrementally as authoring helpers land.
- `./2026-04-16-wos-synth-crate.md` — `wos-synth-core` consumes `wos-mcp` via its `ToolContext::Production` implementation. The in-process dispatch entry unblocks `wos-synth` without requiring a subprocess or JSON-RPC plumbing.

---

## Prerequisites

- **`wos-authoring` crate** (see `./2026-04-17-wos-authoring-crate.md`). `wos-mcp` tool handlers are one-line wrappers over `wos-authoring::WosProject` methods. The authoring crate does not need to be feature-complete before work begins — `wos-mcp` tools land incrementally as the underlying helpers land. Scaffold (Task 1–2) and the `ProjectRegistry` (Task 3) can proceed without any authoring helpers; tool files grow as authoring helpers are committed.
- **`wos-lint`** — exposed as the `wos_lint` tool (Task 6). `wos-lint::lint_document` must be callable from library code.
- **`wos-conformance`** — exposed as the `wos_run_conformance` tool (Task 6). Must be callable as a library function, not only as a CLI binary.
- **Decision on Rust MCP SDK** — audit `crates.io` for `mcp-*` or `modelcontextprotocol-*` crates. If none are production-ready, Task 1 includes a minimal hand-rolled JSON-RPC-2.0-over-stdio module inside `server.rs`. This decision must be made and documented before Task 1 is committed. See Open Questions.

## Completion criteria

1. `crates/wos-mcp/` scaffolded with tool handlers covering the core authoring surface.
2. **Dual entry points both work:**
   - MCP protocol over stdio: `wos-mcp` binary launched by Claude Desktop / Cline serves tool calls over JSON-RPC-2.0. Client sends `{"jsonrpc":"2.0","method":"tools/call","params":{"name":"wos_create_kernel","arguments":{...}},"id":1}` and receives a structured response.
   - In-process: `wos_mcp::dispatch(tool_name, args) -> Result<Value, DispatchError>` callable as a library function. `wos-synth-core` uses this via its `ToolContext::Production` implementation. Zero subprocess spawn, zero JSON-RPC overhead.
3. **22 tools exposed** across six domain families (see tool list below).
4. Every tool has a JSON Schema for its parameters, consumable by the MCP protocol layer and by LLMs building tool-call payloads.
5. Round-trip integration test: load a fixture kernel document, apply 10+ tool calls via `wos_mcp::dispatch`, export the result, verify structural correctness via `wos_lint`.
6. `schemas/mcp/wos-mcp-tools.schema.json` published — a tool-catalog schema listing all 22 tools with their arg schemas and return-value shapes. Vendor tooling (IDE plugins, CI validators) can consume this to verify what WOS MCP offers.

### Tools (22)

**Document management (4)**

| Tool | Purpose |
|------|---------|
| `wos_create_kernel` | Create a new in-memory WOS kernel document and register it. Returns a project ID. |
| `wos_load_document` | Load a WOS document from a JSON string or file path. Validates against the kernel schema. Returns a project ID. |
| `wos_export_document` | Serialize the current project state to a WOS kernel JSON string. |
| `wos_describe_document` | Return a human-readable summary of the document (states, actors, transitions, impact level, AI agent count). |

**Lifecycle (4)**

| Tool | Purpose |
|------|---------|
| `wos_add_state` | Add a workflow state with optional metadata. |
| `wos_add_transition` | Add a transition between two states. |
| `wos_set_initial_state` | Set the initial state of the workflow. |
| `wos_remove_state` | Remove a state (and its outbound transitions). |

**Actors (2)**

| Tool | Purpose |
|------|---------|
| `wos_add_actor` | Register a named actor (human or system role). |
| `wos_add_actor_extension` | Attach extension metadata to an existing actor. |

**Governance (3)**

| Tool | Purpose |
|------|---------|
| `wos_add_due_process_path` | Attach a due-process path (appeal/review route) to the workflow. |
| `wos_add_assertion_gate` | Add an assertion gate that must pass before a transition fires. |
| `wos_set_impact_level` | Set the workflow's equity-impact classification (low/medium/high/critical). |

**AI integration (2)**

| Tool | Purpose |
|------|---------|
| `wos_add_ai_agent` | Register an AI agent with role, model, and capability declarations. |
| `wos_add_deontic_constraint` | Attach a deontic constraint (must/must-not/may) to an AI agent or state. |

**Validation and introspection (7)**

| Tool | Purpose |
|------|---------|
| `wos_lint` | Run `wos-lint` over the project and return structured `LintDiagnostic` objects. |
| `wos_run_conformance` | Run `wos-conformance` and return a `ConformanceTrace`. |
| `wos_preview_state_graph` | Render a Mermaid or DOT diagram of the state machine for human inspection. |
| `wos_search` | Search states, transitions, actors, or constraints by name/type/field value. |
| `wos_list_projects` | List all open project IDs and their document summaries. |
| `wos_close_project` | Close and deregister a project from the in-memory registry. |
| `wos_ping` | Health-check tool. Returns `"pong"`. Used in scaffold tests and transport smoke tests. |

## File structure

- **Create:** `crates/wos-mcp/Cargo.toml` — deps: `wos-authoring`, `wos-lint`, `wos-conformance`, `serde`, `serde_json`, `tokio` (full), `thiserror`, `uuid`, optionally `tracing`.
- **Create:** `crates/wos-mcp/src/lib.rs` — public exports: `dispatch`, `ToolRegistry`, `ProjectRegistry`, handler types.
- **Create:** `crates/wos-mcp/src/server.rs` — MCP-stdio server binary target. JSON-RPC-2.0 loop over stdin/stdout.
- **Create:** `crates/wos-mcp/src/dispatch.rs` — in-process entry point. `pub fn dispatch(tool: &str, args: Value) -> Result<Value, DispatchError>`. Shares the same handler table as `server.rs`.
- **Create:** `crates/wos-mcp/src/registry.rs` — `ProjectRegistry` holding in-memory `WosProject` instances keyed by UUID. Pattern from `packages/formspec-mcp/src/registry.ts`.
- **Create:** `crates/wos-mcp/src/errors.rs` — `ToolError`, `DispatchError`. `ToolError` is the per-handler error type; `DispatchError` wraps it with the tool name for the dispatch layer.
- **Create:** `crates/wos-mcp/src/tools/lifecycle.rs` — `wos_add_state`, `wos_add_transition`, `wos_set_initial_state`, `wos_remove_state`.
- **Create:** `crates/wos-mcp/src/tools/actors.rs` — `wos_add_actor`, `wos_add_actor_extension`.
- **Create:** `crates/wos-mcp/src/tools/governance.rs` — `wos_add_due_process_path`, `wos_add_assertion_gate`, `wos_set_impact_level`.
- **Create:** `crates/wos-mcp/src/tools/ai.rs` — `wos_add_ai_agent`, `wos_add_deontic_constraint`.
- **Create:** `crates/wos-mcp/src/tools/validation.rs` — `wos_lint`, `wos_run_conformance`, `wos_preview_state_graph`.
- **Create:** `crates/wos-mcp/src/tools/query.rs` — `wos_search`, `wos_list_projects`, `wos_close_project`, `wos_describe_document`, `wos_ping`.
- **Create:** `crates/wos-mcp/src/tools/document.rs` — `wos_create_kernel`, `wos_load_document`, `wos_export_document`.
- **Create:** `crates/wos-mcp/tests/round_trip.rs` — round-trip integration test (dispatch path).
- **Create:** `crates/wos-mcp/tests/stdio_transport.rs` — child-process smoke test for the MCP stdio binary.
- **Create:** `schemas/mcp/wos-mcp-tools.schema.json` — tool-catalog schema.
- **Modify:** root `Cargo.toml` workspace members.

---

## Two audiences, same handler functions

`wos-mcp` serves two distinct caller populations over a single set of handler functions:

1. **External MCP clients** — Claude Desktop, Cline, or any MCP-protocol-speaking agent. Reached via the stdio binary (`wos-mcp` executable), which wraps the handler functions in JSON-RPC-2.0 over stdin/stdout. `server.rs` owns this: it reads lines from stdin, parses JSON-RPC requests, routes to the handler table, writes JSON-RPC responses to stdout, and handles SIGTERM for graceful shutdown.

2. **In-workspace Rust callers** — `wos-synth-core`'s `ToolContext` implementation, `wos-bench`'s scoring code, integration tests. These call the handler functions directly as ordinary library functions. No protocol, no serialization beyond what the handler itself needs, no subprocess.

In JavaScript/TypeScript (parent Formspec's context), the equivalent pattern requires a `dispatch()` shim because client and server compile as separate packages. Rust workspaces have no such boundary — `wos-synth-core` can `use wos_mcp::tools::lint_document` as a normal import. The "in-workspace" path is therefore not a separate entry point but simply the default function-call path; no dispatch indirection is needed in Rust.

What IS shared across both audiences: the tool handler signatures (input → output), the JSON Schemas published for each tool (consumed by MCP clients for tool-call generation and by in-workspace callers alike for argument validation), and the tool-catalog schema at `schemas/mcp/wos-mcp-tools.schema.json`.

Both audiences read from the same handler table: a `HashMap<&'static str, HandlerFn>` where each value is a function pointer with signature `fn(&ProjectRegistry, &str, Value) -> Result<Value, ToolError>`. The handler implementations live in `src/tools/` and are never aware of which caller population invoked them.

The `ProjectRegistry` is shared across both audiences within a single process. In the stdio binary, one `Arc<Mutex<ProjectRegistry>>` is created at startup and passed to the server loop. In library use, the caller creates a `ProjectRegistry` and passes it through; `wos-synth-core` manages the registry's lifetime alongside its own state.

---

## Open questions

**Q-A: Which Rust MCP SDK (if any) is stable enough to adopt?**

Candidates as of April 2026: `mcp-rust-sdk`, `rust-mcp`, `modelcontextprotocol` (if a Rust SDK is published). If none are production-ready (pinned releases, tested against Claude Desktop, maintained), Task 1 hand-rolls a minimal JSON-RPC-2.0-over-stdio implementation in `server.rs`. This is ~150 lines: read a line, deserialize a `JsonRpcRequest`, route by `method`, serialize a `JsonRpcResponse`, write the line. The spec is stable and simple; hand-rolling avoids taking a dependency on an ecosystem that may not yet be stable. Decision deadline: Task 1. Document the decision and rationale in a `NOTES.md` at the crate root.

**Q-B: `schemars` derive vs. hand-authored parameter schemas?**

`schemars` can derive JSON Schema from Rust structs annotated with `JsonSchema`. This gives tight coupling between the Rust type and the schema — a type change updates the schema automatically. The downside: `schemars` embeds Rust-specific semantics (e.g., `Option<T>` becomes `anyOf: [T, null]`), and serde format changes silently change the schema. Hand-authored schemas in `schemas/mcp/` are decoupled, stable across serde refactors, and human-readable to LLMs consuming them via tool-list endpoints. **Lean hand-authored** for the tool-catalog schema (`schemas/mcp/wos-mcp-tools.schema.json`) published for external consumers. For internal validation of tool arguments at the dispatch boundary, a lightweight hand-rolled check (required-field presence) is sufficient. Revisit if the tool count grows past 40.

---

## Tasks

### Task 1: Scaffold + JSON-RPC-2.0 transport

**Files:**
- Create: `crates/wos-mcp/Cargo.toml`, `src/lib.rs`, `src/server.rs`, `src/errors.rs`
- Create: `src/tools/query.rs` (stub — `wos_ping` only)
- Modify: root `Cargo.toml`

- [ ] **Step 1.1:** Add `crates/wos-mcp` to workspace `members` in root `Cargo.toml`. Dependencies: `serde`, `serde_json`, `tokio` (full), `thiserror`, `uuid` (v4 feature).
- [ ] **Step 1.2:** Decide on Rust MCP SDK (see Open Questions Q-A). Document the decision in `crates/wos-mcp/NOTES.md`. If hand-rolling: implement the JSON-RPC-2.0 read/dispatch/write loop in `server.rs` (~150 lines). If using an SDK: add it to `Cargo.toml` and adapt.
- [ ] **Step 1.3:** Implement `wos_ping` in `src/tools/query.rs`. The handler takes `(registry: &ProjectRegistry, project_id: &str, args: Value) -> Result<Value, ToolError>` and returns `json!({"pong": true})`.
- [ ] **Step 1.4:** Wire `wos_ping` into `server.rs` so a JSON-RPC `tools/call` request with `name="wos_ping"` returns the pong response.
- [ ] **Step 1.5:** Write an integration test in `tests/stdio_transport.rs` that spawns the binary as a child process, sends a JSON-RPC request, and asserts the response contains `"pong"`.
- [ ] **Step 1.6:** `cargo nextest run -p wos-mcp` passes. Commit: `feat(wos-mcp): scaffold crate + JSON-RPC-2.0 stdio transport + wos_ping`.

### Task 2: Expose tool handlers as public Rust functions + MCP stdio server over them

**Files:**
- Create: `src/dispatch.rs`
- Create: `src/registry.rs` (stub — no `WosProject` yet, just the UUID-keyed map skeleton)

- [ ] **Step 2.1:** Define `HandlerFn` type alias and the `TOOL_HANDLERS` static map in `dispatch.rs`. Initially contains only `wos_ping`. All handler functions are `pub` — in-workspace Rust callers (`wos-synth-core`, `wos-bench`, tests) import them directly by name without needing a dispatch shim.
- [ ] **Step 2.2:** Implement `pub fn dispatch(registry: &ProjectRegistry, tool: &str, args: Value) -> Result<Value, DispatchError>` as a convenience wrapper for callers that receive the tool name as a runtime string (e.g., the stdio server). Direct callers that know the tool name statically call the handler function directly.
- [ ] **Step 2.3:** Refactor `server.rs` to route through `TOOL_HANDLERS` rather than invoking handlers directly in an inline match. Both the stdio server and in-workspace callers now share the same handler table.
- [ ] **Step 2.4:** Unit test: call `wos_mcp::tools::query::wos_ping(registry, "", json!({}))` directly as a library function — asserts the same output as what the stdio binary returns for the equivalent JSON-RPC call.
- [ ] **Step 2.5:** `cargo nextest run -p wos-mcp` passes. Commit: `feat(wos-mcp): expose tool handlers as pub Rust functions + MCP stdio server over shared handler table`.

### Task 3: `ProjectRegistry` + document management tools

**Files:**
- Expand: `src/registry.rs` — full `ProjectRegistry` with `HashMap<Uuid, WosProject>`
- Create: `src/tools/document.rs` — `wos_create_kernel`, `wos_load_document`, `wos_export_document`, `wos_describe_document`

- [ ] **Step 3.1:** Implement `ProjectRegistry` in `src/registry.rs`. Fields: `entries: HashMap<Uuid, WosProject>`. Methods: `new_project() -> Uuid`, `get(&Uuid) -> Result<&WosProject, ToolError>`, `get_mut(&Uuid) -> Result<&mut WosProject, ToolError>`, `close(&Uuid)`, `list() -> Vec<(Uuid, &WosProject)>`. No phase model needed — WOS documents are always in authoring state. Max 20 projects enforced.
- [ ] **Step 3.2:** Implement `wos_create_kernel`: calls `wos_authoring::WosProject::new_kernel()`, registers in the registry, returns `{"project_id": "<uuid>"}`.
- [ ] **Step 3.3:** Implement `wos_load_document`: accepts `{"json": "<wos-kernel-json-string>"}` or `{"path": "<file-path>"}`, parses, validates against the kernel schema, registers, returns `{"project_id": "<uuid>"}`. On schema error, returns a `ToolError::ValidationFailed` with the first validation message.
- [ ] **Step 3.4:** Implement `wos_export_document`: serializes the project to a JSON string. Returns `{"document": "<json-string>"}`.
- [ ] **Step 3.5:** Implement `wos_describe_document`: returns `{"state_count": N, "transition_count": N, "actor_count": N, "impact_level": "...", "ai_agent_count": N}` by reading the current document.
- [ ] **Step 3.6:** Add all four handlers to `TOOL_HANDLERS`. Write unit tests for create + export round-trip and describe. `cargo nextest run -p wos-mcp` passes. Commit: `feat(wos-mcp): project registry + document-management tools`.

### Task 4: Lifecycle + actor tools

**Files:**
- Create: `src/tools/lifecycle.rs` — `wos_add_state`, `wos_add_transition`, `wos_set_initial_state`, `wos_remove_state`
- Create: `src/tools/actors.rs` — `wos_add_actor`, `wos_add_actor_extension`

- [ ] **Step 4.1:** Implement `wos_add_state`. Args: `{"project_id": "...", "state_id": "...", "label": "...", "metadata": {...}}`. Delegates to `wos_authoring::WosProject::add_state(state_id, label, metadata)`.
- [ ] **Step 4.2:** Implement `wos_add_transition`. Args: `{"project_id": "...", "from": "...", "to": "...", "label": "...", "trigger": "..."}`. Delegates to `WosProject::add_transition`.
- [ ] **Step 4.3:** Implement `wos_set_initial_state`. Args: `{"project_id": "...", "state_id": "..."}`. Delegates to `WosProject::set_initial_state`.
- [ ] **Step 4.4:** Implement `wos_remove_state`. Args: `{"project_id": "...", "state_id": "..."}`. Delegates to `WosProject::remove_state`. Returns count of transitions also removed.
- [ ] **Step 4.5:** Implement `wos_add_actor` and `wos_add_actor_extension`. Wire all six into `TOOL_HANDLERS`.
- [ ] **Step 4.6:** Round-trip test: `wos_create_kernel` → `wos_add_state` × 3 → `wos_add_transition` × 2 → `wos_set_initial_state` → `wos_export_document` → parse → assert three states and two transitions present. `cargo nextest run -p wos-mcp` passes. Commit: `feat(wos-mcp): lifecycle + actor tools`.

### Task 5: Governance + AI tools

**Files:**
- Create: `src/tools/governance.rs` — `wos_add_due_process_path`, `wos_add_assertion_gate`, `wos_set_impact_level`
- Create: `src/tools/ai.rs` — `wos_add_ai_agent`, `wos_add_deontic_constraint`

- [ ] **Step 5.1:** Implement `wos_add_due_process_path`. Args: `{"project_id": "...", "path_id": "...", "description": "...", "steps": [...]}`. Delegates to `WosProject::add_due_process_path`.
- [ ] **Step 5.2:** Implement `wos_add_assertion_gate`. Args: `{"project_id": "...", "gate_id": "...", "assertion": "...", "transition": "..."}`. Delegates to `WosProject::add_assertion_gate`.
- [ ] **Step 5.3:** Implement `wos_set_impact_level`. Args: `{"project_id": "...", "level": "low|medium|high|critical"}`. Validates against the enum before delegating.
- [ ] **Step 5.4:** Implement `wos_add_ai_agent`. Args: `{"project_id": "...", "agent_id": "...", "role": "...", "model": "...", "capabilities": [...]}`. Delegates to `WosProject::add_ai_agent`.
- [ ] **Step 5.5:** Implement `wos_add_deontic_constraint`. Args: `{"project_id": "...", "target": "agent_id|state_id", "modality": "must|must_not|may", "action": "..."}`. Delegates to `WosProject::add_deontic_constraint`.
- [ ] **Step 5.6:** Wire all five into `TOOL_HANDLERS`. Unit test: governance tool calls on a fresh kernel produce correct JSON structure when exported. Commit: `feat(wos-mcp): governance + AI integration tools`.

### Task 6: Validation/query tools + tool-catalog schema

**Files:**
- Create: `src/tools/validation.rs` — `wos_lint`, `wos_run_conformance`, `wos_preview_state_graph`
- Expand: `src/tools/query.rs` — `wos_search`, `wos_list_projects`, `wos_close_project`
- Create: `schemas/mcp/wos-mcp-tools.schema.json`
- Create: `tests/round_trip.rs` — full round-trip integration test

- [ ] **Step 6.1:** Implement `wos_lint`. Exports the project document, calls `wos_lint::lint_document(&json_value)`, returns the `Vec<LintDiagnostic>` as a JSON array. Returns `{"diagnostics": [...], "error_count": N, "warning_count": N}`.
- [ ] **Step 6.2:** Implement `wos_run_conformance`. Exports the project document, calls `wos_conformance::run(&json_value)`, returns the `ConformanceTrace` as JSON.
- [ ] **Step 6.3:** Implement `wos_preview_state_graph`. Accepts `{"project_id": "...", "format": "mermaid|dot"}`. Renders the state machine as a Mermaid `stateDiagram-v2` or Graphviz DOT string. No external dependency required — construct the graph string directly from the document's `states` and `transitions` arrays.
- [ ] **Step 6.4:** Implement `wos_search`. Args: `{"project_id": "...", "kind": "state|transition|actor|constraint", "query": "..."}`. Linear scan over the document; returns matching items as a JSON array. Full-text substring match on ID and label fields.
- [ ] **Step 6.5:** Implement `wos_list_projects` (no project_id needed — list all open projects) and `wos_close_project`. Wire all remaining handlers into `TOOL_HANDLERS`.
- [ ] **Step 6.6:** Hand-author `schemas/mcp/wos-mcp-tools.schema.json`. Top-level shape: `{"tools": [{"name": "...", "description": "...", "inputSchema": {...}, "outputSchema": {...}}]}`. One entry per tool. Keep parameter schemas minimal but accurate — required fields and their types.
- [ ] **Step 6.7:** Write the full round-trip integration test in `tests/round_trip.rs`: load the kernel fixture from `wos-spec/fixtures/`, dispatch 10+ tool calls (create, add 3 states, add 2 transitions, set initial state, set impact level, add actor, add AI agent, lint), export, lint again via dispatch, assert zero errors. Use the in-process dispatch path — no subprocess.
- [ ] **Step 6.8:** `cargo nextest run -p wos-mcp` passes. Commit: `feat(wos-mcp): validation/query tools + publish tool-catalog schema`.

---

## Self-review checklist

- Both entry points (stdio binary and `dispatch()`) share the same handler table — no duplicate logic.
- Every tool handler is a pure function: no global state, no internal mutable statics.
- `ProjectRegistry` enforces the 20-project cap and returns typed errors.
- `schemas/mcp/wos-mcp-tools.schema.json` lists all 22 tools with parameter schemas.
- Round-trip integration test covers all six tool families.
- `wos-mcp` contains zero business logic — every handler body is ≤5 lines delegating to `wos-authoring`, `wos-lint`, or `wos-conformance`.

## Why this matters

WOS currently has lower-layer crates (`wos-core`, `wos-lint`, `wos-conformance`) but no external-agent-facing interface. A user wanting to author WOS documents interactively via Claude Desktop or Cline has no entry point. `wos-mcp` is that entry point — it makes WOS a first-class MCP server alongside parent Formspec. It also unblocks `wos-synth-core`'s `ToolContext::Production` implementation: the authoring loop calls `wos_mcp::dispatch` rather than reaching into lint and conformance crates directly through ad-hoc coupling. One adapter layer, two consumers, zero business logic in the adapter.

## Estimated effort

~2 engineer-weeks for scaffold + 22 tools + tool-catalog schema. Each new `wos-authoring` helper added in the future typically produces one corresponding MCP tool (~30 minutes marginal cost per tool once the infrastructure is in place).

---

## Addendum — v0-spike findings (2026-04-20)

The v0 spike
([`thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md`](../research/2026-04-20-wos-synth-v0-spike-findings.md))
does not exercise the MCP transport — it calls `wos_lint` and
`wos_conformance` directly. The dual-entry pattern question (retrospective
Q4, "was dual-entry needed?") is therefore *out of scope for the spike*,
not *unanswered by the spike*. This plan's Task 2 assumptions stand; the
spike provides no new evidence for or against them.

**No plan change.** Record this explicitly so future readers do not
mistake the spike's silence on MCP for a validated "yes, dual-entry
pattern is load-bearing".
