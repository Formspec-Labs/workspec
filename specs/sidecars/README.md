# WOS Sidecar Index

Sidecars are deployment-environment configuration documents that join a workflow by `targetWorkflow` URI. Per ADR 0076 D-3, sidecars MUST NOT alter case state — they vary a single deployment-environment axis (calendar, templates, ontology export). This directory indexes the sidecars that survived the 27 → 6 schema family consolidation.

Sidecars are exempt from the [`CONVENTIONS.md`](../../CONVENTIONS.md) three-section rubric when their schema descriptions cover Normative Contract / Composition / Conformance. The schema is the source of truth; this index is the cold-read entry point.

## Active sidecars

### `wos-delivery` (post-consolidation, ADR 0076 D-3)

**Schema:** [`schemas/sidecars/wos-delivery.schema.json`](../../schemas/sidecars/wos-delivery.schema.json) · 676 lines.
**Purpose:** Deployment-environment delivery configuration. One sidecar carries calendar (business days, holidays, operating hours for SLA evaluation), notifications (template library for due-process notices, hold notifications, SLA warnings), and correspondence (document tracking metadata). Replaces the prior `wos-business-calendar`, `wos-notification-template`, and `wos-correspondence-metadata` sidecars per ADR 0076 D-3.
**Joins by:** `targetWorkflow` URI matching the workflow's `url`.
**Conformance:** schema validation. Calendar resolves SLAs (Governance §10.3); templates resolve notice rendering (Governance §3.x); correspondence supplies document-tracking metadata for audit reports. None of these surfaces alter determinations.
**Required block:** at least one of `calendar` / `notifications` / `correspondence` per the schema's `anyOf`. **Cold-read gate for an LLM authoring a delivery sidecar:** an empty `wos-delivery` document (just `$wosDelivery` + `targetWorkflow`) is REJECTED by the schema. The author MUST include at least one block; when adding only `calendar`, the document is well-formed even with empty `notifications`/`correspondence`, but a truly-empty document is a configuration error caught at validation. This gate prevents accidental no-op sidecars that imply deployment-environment configuration without supplying any.

### `wos-ontology-alignment` (renamed from `wos-semantic-profile`, ADR 0076 D-3)

**Schema:** [`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json) · 385 lines.
**Purpose:** Per-deployment ontology alignment. JSON-LD `@context`, SHACL shapes, PROV-O export, XES/OCEL mapping for interop with downstream process-mining and provenance graph tools. Adds interpretation and export capability — never changes how WOS documents are processed at runtime.
**Joins by:** `targetWorkflow` URI matching the workflow's `url`.
**Conformance:** schema validation. SHACL shapes and JSON-LD `@context` are consumed by export pipelines (`wos-export`); kernel processing is unaffected.
**Note on Step 0 (independence):** ontology alignment fits the "distinct semantic model" test in [`CONVENTIONS.md`](../../CONVENTIONS.md) Step 0 — RDF/SHACL/PROV-O is genuinely outside WOS's processing model. Independent existence justified.

## Legacy sidecar prose docs (retained, not re-authored)

The following legacy spec docs predate the ADR 0076 consolidation and target the now-replaced standalone sidecars. They remain in the tree as historical references; each carries an absorption notice at its head pointing at the merged-schema `$def`:

- [`business-calendar.md`](business-calendar.md) — content absorbed into `wos-delivery.schema.json`'s `calendar` block.
- [`notification-template.md`](notification-template.md) — content absorbed into `wos-delivery.schema.json`'s `notifications` block.
- [`../kernel/correspondence-metadata.md`](../kernel/correspondence-metadata.md) — content absorbed into `wos-delivery.schema.json`'s `correspondence` block.

These legacy docs do not need rewrites; the absorbed schema descriptions are the canonical surface.
