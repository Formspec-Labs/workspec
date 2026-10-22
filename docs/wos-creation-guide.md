# WOS Workflow Creation Guide

This guide documents the end-to-end WOS workflow creation pipeline—from initializing an editable draft to authoring lifecycle states, actors, contracts, governance, and AI oversight, running static and replay verification, and exporting a verified workflow for deployment packaging.

---

## 1. Overview & Architecture

WOS authoring is structured around a single native JSON authority and shared verification owners:

```text
MCP Client / Agent / CLI
       │
       ▼
  wos-mcp (executable catalog: src/catalog.rs → tool-catalog.json)
       │
       ├──► Authoring mutations ──► wos-authoring (WosProject façade → RawWosProject)
       │                                  │
       │                                  ├── Native JSON authority (serde_json::Value)
       │                                  ├── Owning-schema fragment validation (native_schema)
       │                                  └── Fallible typed read view (KernelDocument in wos-core)
       │
       ├──► Static validation   ──► wos-lint + wos-formspec-binding (document / project scope)
       │
       ├──► Behavioral replay   ──► wos-conformance + wos-runtime (deterministic replay proof)
       │
       └──► Export & packaging  ──► tools/deployment-tools (wos-mcp-receipt.json + package-manifest.json)
```

### Component Responsibilities

- **`wos-mcp`**: Exposes the 20 MCP tools declared in [`../crates/wos-mcp/src/catalog.rs`](../crates/wos-mcp/src/catalog.rs) and checked into [`../crates/wos-mcp/tool-catalog.json`](../crates/wos-mcp/tool-catalog.json). Both the stdio JSON-RPC server and in-process dispatch execute the same catalog declarations and validate inputs and outputs against bundled local JSON Schemas.
- **`wos-authoring`**: Provides [`WosProject`](../crates/wos-authoring/src/project.rs), the sole public authoring façade. Internally, `RawWosProject` (`pub(crate)` in [`../crates/wos-authoring/src/raw.rs`](../crates/wos-authoring/src/raw.rs)) implements `IWosProjectCore` and dispatches `pub(crate) enum Command` ([`../crates/wos-authoring/src/command.rs`](../crates/wos-authoring/src/command.rs)) against the complete native `serde_json::Value` document with bounded full-document undo/redo history (`UNDO_DEPTH = 100`).
- **`wos-core`**: Defines the runtime `KernelDocument` model, `StateKind`, `ActorKind`, `ImpactLevel`, `TransitionEvent`, and AI policy rule source navigation. `WosProject::typed_snapshot` projects the native JSON document into a `KernelDocument` when needed by typed query tools, while `WosProject::native_snapshot` returns the lossless native JSON value.
- **`wos-lint` & `wos-formspec-binding`**: Validate structural and semantic rules across single documents (`document` scope) or composed workflow-plus-dependency bundles (`project` scope, checking `definition`, `mapping`, `delivery`, and `ontology-alignment` dependencies).
- **`wos-conformance`**: Executes deterministic workflow replay against inline conformance fixtures, controlled clocks, and explicit observation expectations, and computes the unified authoring readiness decision.
- **`tools/deployment-tools`**: Consumes the exported native workflow JSON and verification outputs to seal `wos-mcp-receipt.json` and `package-manifest.json` for downstream runtime admission.

---

## 2. Stage 1 — Creating or Loading a Project

Every MCP authoring session manages open projects in a bounded `ProjectRegistry` (up to 20 concurrent projects per session).

### Project Lifecycle Tools & `WosProject` Constructors

| MCP Tool | Rust `WosProject` Equivalent | Purpose |
|----------|------------------------------|---------|
| `wos_ping` | *(server health check)* | Verify that the `wos-mcp` server is responsive (`{"pong": true}`). |
| `wos_create_kernel` | `WosProject::new_kernel` / `WosProject::new` | Create a new editable draft (`$wosWorkflow: "1.0"`, `impactLevel: "operational"`, empty `actors`, `lifecycle.states`, `contracts`) and return `{"project_id": "<uuid>", "draft": true}`. |
| `wos_load_document` | `WosProject::from_native_document` / `WosProject::from_document` | Load an existing native WOS Workflow from inline `"json"` or a `"path"` after passing the tier-1 lint gate, preserving all authored native fields verbatim. |
| `wos_list_projects` | *(registry inspection)* | List all open `project_id` UUIDs and their count. |
| `wos_describe_document` | `WosProject::native_snapshot` | Summarize top-level `state_count`, `transition_count`, `actor_count`, `impact_level`, and `ai_agent_count`. |
| `wos_close_project` | *(registry cleanup)* | Close and remove an open `project_id` from the session registry. |

### Example: Creating a Draft Project

**MCP (`wos_create_kernel`):**

```json
{
  "tool": "wos_create_kernel",
  "arguments": {}
}
```

**Rust (`wos-authoring`):**

```rust
use wos_authoring::{ImpactLevel, WosProject};

// Default operational draft:
let mut project = WosProject::new_kernel();

// Or with explicit impact level and title:
let mut custom = WosProject::new(ImpactLevel::RightsImpacting, "Benefits Eligibility Review");
```

---

## 3. Stage 2 — Authoring the State Machine & Actors

Once a project is registered, build the lifecycle state graph, transitions, actors, and impact level using dedicated kernel tools or `WosProject` helper methods.

### Lifecycle, Actor, Inspection, and History Surface

| MCP Tool | Rust `WosProject` Methods | Behavior |
|----------|---------------------------|----------|
| `wos_add_state` | `WosProject::add_state`, `WosProject::add_state_described` | Add a top-level state (`atomic`, `compound`, `parallel`, `foreach`, or `final`) with optional human-readable `label` (`description`) and `metadata` (`extensions["x-meta"]`). |
| `wos_set_initial_state` | `WosProject::set_initial_state` | Set `lifecycle.initialState` to an existing top-level state ID. |
| `wos_add_transition` | `WosProject::add_transition`, `WosProject::add_transition_typed` | Add a transition from `from` to `to` with either a legacy string `trigger` or a canonical typed `event` (`message`, `signal`, `timer`, `condition`, `error`) and optional FEL `guard`. |
| `wos_remove_state` | `WosProject::remove_state` | Remove a top-level state and automatically prune inbound transitions targeting it, returning `transitions_removed`. |
| *(Rust façade only)* | `WosProject::rename_state` | Rename a top-level state and repoint incoming transitions and `lifecycle.initialState`. |
| *(Rust façade only)* | `WosProject::add_milestone`, `WosProject::remove_milestone` | Add or remove named FEL milestone conditions under `lifecycle.milestones`. |
| `wos_add_actor` | `WosProject::add_actor`, `WosProject::remove_actor` | Declare an actor (`human`, `system`, or `agent`) in `actors[]`. Removing an actor via `WosProject::remove_actor` emits a warning in `WosProject::diagnostics` if referenced by `assignTo`. |
| `wos_add_actor_extension` | `WosProject::add_actor_extension` | Attach an `x-`-prefixed extension key/value to an existing actor (`actors[].extensions`). |
| `wos_set_impact_level` | `WosProject::set_impact_level` | Set the document-level `impactLevel` (`informational`, `operational`, `safety-impacting`, `rights-impacting`). |
| *(Rust façade only)* | `WosProject::add_contract`, `WosProject::set_contract_definition_version`, `WosProject::set_timer`, `WosProject::add_extension_key` | Convenience helpers for adding basic contract entries, pinning `definitionVersion`, setting `extensions["x-wos-timers"]`, and writing `x-`-prefixed root extensions. |
| `wos_preview_state_graph` | `WosProject::typed_snapshot` | Render the state machine as a `mermaid` (`stateDiagram-v2`) or Graphviz `dot` diagram. |
| `wos_search` | `WosProject::typed_snapshot`, `WosProject::native_snapshot` | Search `state`, `transition`, `actor` (via typed read view) or `constraint` (via `native_rule_sources` over raw native JSON). |
| *(Rust façade only)* | `WosProject::undo`, `WosProject::redo`, `WosProject::can_undo`, `WosProject::can_redo`, `WosProject::diagnostics` | Step backward or forward through bounded full-document snapshots and inspect accumulated warning diagnostics. |

---

## 4. Stage 3 — Authoring Native Workflow Sections (`wos_edit_workflow`)

Top-level native Workflow sections—such as `caseFile`, `contracts`, compound/parallel nested states, lifecycle phase actions (`onEntry`/`onExit`), `governance`, `agents`, `deonticConstraints`, and `aiOversight`—are authored through `wos_edit_workflow`.

Each edit validates its payload fragment offline against [`../schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json) (via [`../crates/wos-authoring/src/native_schema.rs`](../crates/wos-authoring/src/native_schema.rs)) and mutates only the targeted JSON locus in `RawWosProject`. All other native fields in the document remain untouched.

> [!IMPORTANT]
> Legacy extension-writing helpers (`WosProject::add_due_process_path`, `WosProject::add_assertion_gate`, `WosProject::add_deontic_constraint`, and `WosProject::add_actor_deontic`) are retired and return an error directing callers to use native `governance` and `agents` edits instead.

### All 10 `wos_edit_workflow` Actions

When `action` is omitted in a `wos_edit_workflow` call, it defaults to `"metadata"`. Full field-level coverage is tracked in [`../crates/wos-mcp/WORKFLOW-COVERAGE.json`](../crates/wos-mcp/WORKFLOW-COVERAGE.json).

| `wos_edit_workflow` Action | Target Root Field(s) | Rust `WosProject` Method | Payload & Validation Fragment |
|----------------------------|----------------------|--------------------------|-------------------------------|
| `metadata` *(default)* | `$wosWorkflow`, `wosVersionPin`, `url`, `version`, `title`, `description`, `status`, `impactLevel` | `WosProject::set_native_fields` | `{"fields": {...}}` — validates each supplied property against `/properties/<field>`. |
| `case_field` | `caseFile` | `WosProject::put_case_field` | `{"name": "<field>", "definition": {...}}` — validates against `/$defs/FieldDeclaration` and writes `caseFile.fields.<name>` (errors if `caseFile.contractRef` is used). |
| `contract` | `contracts` | `WosProject::put_contract` | `{"name": "<contract>", "contract": {...}}` — validates against `/$defs/Contracts/additionalProperties` (including FormSpec `binding`, `ref`, and `definitionVersion`). |
| `state` | `lifecycle` | `WosProject::put_state` | `{"pointer": "/lifecycle/states/<id>", "state": {...}}` — validates locator path (supporting nested `states` and `regions/<r>/states`) and `/$defs/State`. |
| `state_action` | `lifecycle` | `WosProject::append_state_action` | `{"pointer": "/lifecycle/states/<id>", "phase": "onEntry" \| "onExit", "value": {...}}` — validates phase via `state_action_phases()` and action against `/$defs/LifecycleAction`. |
| `governance` | `governance` | `WosProject::set_native_fields` | `{"value": {...}}` — validates and replaces top-level `/properties/governance` (including `taskCatalog`, `reviewProtocols`, `dueProcess`, `appealProcess`). |
| `agent` | `agents` | `WosProject::put_agent` | `{"value": {...}}` — validates against `/$defs/AgentDeclaration` and upserts into `agents[]` by `id`. |
| `remove_agent` | `agents` | `WosProject::remove_agent` | `{"agent_id": "<id>"}` — removes the matching agent entry from `agents[]`. |
| `agent_constraint` | `agents` | `WosProject::add_agent_constraint` | `{"agent_id": "<id>", "constraint": {...}}` — validates against `/$defs/DeonticConstraint` and appends to the agent's `deonticConstraints[]`. |
| `ai_oversight` | `aiOversight` | `WosProject::set_native_fields` | `{"value": {...}}` — validates and replaces top-level `/properties/aiOversight`. |

### Example: Authoring Metadata, Case Fields, Contracts, and Governance

```json
{
  "tool": "wos_edit_workflow",
  "arguments": {
    "project_id": "<project_id>",
    "action": "metadata",
    "fields": {
      "url": "urn:wos:workflow:procurement-approval",
      "version": "1.0.0",
      "title": "Procurement Approval",
      "status": "draft",
      "impactLevel": "operational"
    }
  }
}
```

```json
{
  "tool": "wos_edit_workflow",
  "arguments": {
    "project_id": "<project_id>",
    "action": "case_field",
    "name": "amount",
    "definition": {
      "type": "number",
      "description": "Requested purchase order total"
    }
  }
}
```

---

## 5. Stage 4 — Shared Checks (`wos_lint`, `wos_run_conformance`, `wos_check_authoring`)

Draft creation and incremental payload validation do not by themselves establish whole-workflow readiness. WOS separates static validation, behavioral replay conformance, and the combined authoring gate into three tools backed by [`../schemas/wos-tooling.schema.json`](../schemas/wos-tooling.schema.json).

### 1. Static Validation (`wos_lint`)

Runs `wos-lint` static analysis against the project's current `WosProject::native_snapshot`:
- **`scope: "document"`** *(default)*: Validates the standalone workflow JSON against schema, structural, FEL, governance, and AI policy rules.
- **`scope: "project"`**: Composes validation across the workflow and immutable `resolved_dependencies` (`role` in `["definition", "mapping", "delivery", "ontology-alignment"]`, `url`, `version`, `document_json`, and optional `definition: {url, version}` for mappings).
- Returns `diagnostics`, severity counts (`block_count`, `error_count`, `warning_count`, `info_count`), and the structured `validation` report.

### 2. Behavioral Conformance Replay (`wos_run_conformance`)

Replays the project's current native snapshot deterministically through `wos-conformance`:
- Requires inline `fixture_json` (file-based `base_dir` substitution is refused).
- Accepts `resolved_dependencies`, `clock_start_ms`, `required_observations`, and `expected_observations` (`pointer` + `expected` value).
- Returns `passed`, `status` (`"pass"`, `"fail"`, or `"refused"`), cryptographic input/runtime `identity`, step `attempts`, `finalSnapshot`, `provenance`, `trace`, and any `failures`, `refusals`, `differences`, or `observationErrors`.

### 3. Unified Authoring Readiness Gate (`wos_check_authoring`)

Freezes a single project snapshot and runs both `validation` (`wos_lint` arguments, defaulting to `scope: "project"`) and all required `replays` scenarios (`[{ "id": "<scenario_id>", "arguments": { ... } }]`) against that identical snapshot:
- Produces `validation`, `static_identity`, `static_owner_identity`, `replays`, and the canonical `decision` (`authoringDecision__Root`).
- Setting `draft_only: true` allows static-only inspection while marking the decision as an unverified draft that cannot be used as a sealed deployment handoff.

---

## 6. Stage 5 — Exporting & Sealing into a Deployment Package

Once `wos_check_authoring` passes with all required static checks and conformance replay scenarios verified:

1. **Export the Native Workflow (`wos_export_document`)**:
   - Call `wos_export_document` (or `WosProject::native_snapshot` in Rust) to serialize the exact native JSON document, preserving all modeled and import-export root fields (`intake`, `bindings`, `provenance`, `signature`, `custody`, `advanced`, `assurance`, `decisionTables`, `extensions`, `execution`, `evaluationMode`, `maxRelationshipEventDepth`).
2. **Verify FormSpec Contract & Binding Alignment (`wos-formspec-binding`)**:
   - Ensure every FormSpec contract declared in `contracts` (via `wos_edit_workflow` with `action: "contract"` or `WosProject::put_contract`) pins its `definitionVersion` and matches the `definition` and `mapping` documents supplied in `resolved_dependencies`.
3. **Seal the Deployment Package (`tools/deployment-tools`)**:
   - Pass the exported workflow JSON, resolved dependency artifacts, and the `wos_check_authoring` verification output to `tools/deployment-tools` to emit the cryptographic `wos-mcp-receipt.json` and `package-manifest.json`.
   - Runtime hosts verify the receipt and manifest identities against the workflow and dependency digests before admitting the workflow for execution.

---

## 7. Reference Links

To avoid schema drift, this guide links directly to the executable declarations and checked artifacts:

- **MCP Server & CLI Usage**: [`../crates/wos-mcp/README.md`](../crates/wos-mcp/README.md)
- **Generated MCP Tool Catalog (all 20 tools & JSON Schemas)**: [`../crates/wos-mcp/tool-catalog.json`](../crates/wos-mcp/tool-catalog.json)
- **Generated Workflow Root Field Coverage Matrix**: [`../crates/wos-mcp/WORKFLOW-COVERAGE.json`](../crates/wos-mcp/WORKFLOW-COVERAGE.json)
- **Executable Tool Declarations**: [`../crates/wos-mcp/src/catalog.rs`](../crates/wos-mcp/src/catalog.rs)
- **Authoring Façade (`WosProject`)**: [`../crates/wos-authoring/src/project.rs`](../crates/wos-authoring/src/project.rs)
- **Authoring Crate Root**: [`../crates/wos-authoring/src/lib.rs`](../crates/wos-authoring/src/lib.rs)
- **Canonical Workflow JSON Schema**: [`../schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json)
- **Canonical Tooling JSON Schema**: [`../schemas/wos-tooling.schema.json`](../schemas/wos-tooling.schema.json)
- **Reconciled Historical Authoring Plan**: [`../thoughts/plans/2026-04-17-wos-authoring-crate.md`](../thoughts/plans/2026-04-17-wos-authoring-crate.md)
