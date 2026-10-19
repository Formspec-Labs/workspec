# WOS MCP

The stdio server and in-process `dispatch()` function use the same executable
catalog in `src/catalog.rs`. Each declaration owns its name, description,
argument schema, defaults, output schema, behavior hints, native field coverage
and handler. Named native edits have one declaration per action.

```rust,ignore
let result = wos_mcp::dispatch::dispatch(
    &mut registry, "wos_edit_workflow", project_id,
    serde_json::json!({"project_id":project_id,"fields":{"title":"Review"}}),
).await?;
```

`wos_edit_workflow` defaults to `metadata` when `action` is omitted. Other actions
edit an inline case field, a contract/Formspec reference, a complete nested state,
a state action, governance, an agent, an agent constraint or AI oversight.
Every edit preserves native JSON outside the requested value. Draft creation
returns `draft: true`; payload validation does not claim whole-workflow readiness.
The coverage file records every canonical root field as incremental or
import/export. Historical extension values remain intact, while obsolete
extension-writing tools are retired.

`wos_lint` calls `wos_lint::studio_api::validate` on the current native snapshot.
Its `scope` defaults to `document`; request `project` with immutable
`resolved_dependencies` for composed checks. Each selection declares `role`,
`url`, `version` and complete native `document_json`. A Definition-associated
Mapping may select its exact `definition: {url, version}` separately from its
own locator/version and native compatible-version range. Unknown roles are
refused. Validation never fetches selected documents or schemas.

The `validation` result retains scope, required check groups, pass/fail/not-run
status and reasons, native WOS diagnostics and native Formspec reports. Existing
severity counts summarize the unchanged WOS findings. Missing dependencies fail;
unsupported semantic branches remain not-run. The standard empty JSON Pointer
identifies the document root. Static pass does not establish replay, scenario
coverage, readiness or publication. The current import handler can reject native
AI errors before registration; shared lint acceptance tests set up the same
native snapshot directly and exercise the actual lint handler and dispatch.

`wos_run_conformance` replays the selected project's current native snapshot. It
accepts inline fixture JSON, frozen resolved dependencies, controlled clock and
explicit observation expectations. File-based `base_dir` substitution is
unsupported. Its evidence records both execution and observation failures.

```sh
cargo run -p wos-mcp -- --list-tools
cargo run -p wos-mcp -- --tool-help wos_edit_workflow
cargo run -p wos-mcp --example generate_catalog -- --write
cargo run -p wos-mcp --example generate_catalog -- --check
cargo nextest run -p wos-authoring -p wos-mcp
```

`tool-catalog.json` and `WORKFLOW-COVERAGE.json` are checked generated views.
`tools/list` and command-line help read the same declarations directly. Adding a
tool requires its handler and a catalog declaration, then regeneration. There is
no separate dispatch match or tool-name allowlist.

Native payload schemas reference `schemas/wos-workflow.schema.json`; diagnostics
reference `schemas/wos-tooling.schema.json`. In-process validation registers both
bundled resources and explicitly refuses all other retrieval. Offline consumers
must distribute those owner schemas and register their `$id` values. The
standalone MCP catalog schema is a thin view of the tooling owner; missing owner
resources produce a resolution error.

The binary retains newline-delimited JSON-RPC over stdin/stdout. Transport changes
are outside this authoring change. Tool failures return MCP `isError` results;
unknown tool names return the existing JSON-RPC invalid-params error.
