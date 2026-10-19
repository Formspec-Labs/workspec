# WOS authoring

`WosProject` edits one native JSON document. Explicit operations change only the
requested native values. Imports, exports and undo retain schema-valid values
that the typed `KernelDocument` read view does not model, including root `x-*`
fields and embedded content.

```rust
use wos_authoring::{ImpactLevel, WosProject};
use serde_json::json;
let mut project = WosProject::new(ImpactLevel::Operational, "Review");
project.set_native_fields(serde_json::Map::from_iter([
    ("url".into(), json!("urn:example:review")),
    ("version".into(), json!("1.0.0")),
])).unwrap();
let native = project.native_snapshot();
```

Creation produces an editable draft. Edit payloads use fragments from the
canonical Workflow schema; they do not establish whole-document readiness.
`native_snapshot()` returns the exact current JSON values.
`typed_snapshot()` returns a fallible typed read view for existing consumers;
a schema-valid value outside that model's range returns a diagnostic rather
than silently losing data. `from_native_document(Value)` imports an object
without rebuilding it from selected fields.

Each successful helper call creates one undo step, bounded to 100 entries.
Failed edits restore the full document and leave history unchanged. Named actor
and agent removals preserve other entries and their metadata. Complete native
replacement intentionally replaces the named value and its descendants.
Generic JSON patches and array reordering are unsupported.

Governance, agent declarations, agent constraints and AI oversight use native
`governance`, `agents` and `aiOversight`. Historical `x-wos-governance` and
`x-wos-ai` values survive import/export; obsolete extension-writing helpers
return explicit unsupported-operation errors. They are not MCP tools.

The public helpers call private commands in `raw.rs`; no external caller can
issue raw commands. `native_schema.rs` uses the maintained `jsonschema` validator
with the bundled owner schema and a rejecting resource retriever, so edit
validation cannot fetch remote schemas.
