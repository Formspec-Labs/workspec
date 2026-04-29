---
title: WOS Extension Registry
version: 1.0.0-draft.1
date: 2026-04-18
status: draft
---

> **Absorbed (ADR 0076 D-5).** Extension Registry is a `$view` of the merged tooling schema at [`schemas/wos-tooling.schema.json#/$views/extensionRegistry`](../../schemas/wos-tooling.schema.json) (entries: `extensionRegistry__Root`, `extensionRegistry__RegistryEntry`, `extensionRegistry__ExtensionsMap`, `extensionRegistry__JsonSchemaUri`). Registry documents now use the `$wosTooling` envelope marker; the standalone `wos-extension-registry` schema and `$wosExtensionRegistry` marker are retired. Prose below remains as normative reference for vendor-namespace governance and seam catalog semantics.

# WOS Extension Registry v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-18
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Extension Registry is a JSON document that catalogs the named extension seams a WOS deployment exposes. The kernel defines six seams (Kernel §10) and treats each as an opaque attachment point — higher layers (Governance, AI Integration, Advanced) and vendors bind concrete shapes to those seams. Until now, the catalog of which seams exist, which are stable enough to depend on, and which vendor namespaces are claimed lived only in prose. This spec defines a machine-readable registry document that adopters scan to answer "what extension points does WOS expose, and which can I bet on?"

The registry is descriptive, not prescriptive. It catalogs seams; it does not enforce shape — that is the job of the kernel schema, sidecar schemas, and vendor schemas referenced by registry entries. Lint tooling consumes registries to validate that `x-`-prefixed extension keys belong to a declared vendor namespace; documentation tooling consumes registries to render seam catalogs.

For the WOS-Trellis `custodyHook` binding, the registry also carries WOS-owned identifier metadata under reserved `x-wos-*` keys in the root `extensions` object. These keys publish the `wos.*` event-type namespace and the reserved TypeID family prefixes used by the authored-record append surface. They are machine-readable reference data, not additional seam kinds.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a registry document type companion to the WOS Kernel Specification v1.0. The MVP scope is a catalog with discovery and lifecycle: per-seam validation profiles, runtime registration / deregistration ceremonies, and vendor approval workflows are explicitly deferred to a future iteration.

---

## 1. Document Structure

A WOS Extension Registry is a JSON document identified by the `$wosExtensionRegistry` document type marker. It declares one or more entries, each cataloging a single seam binding.

### 1.1 Root Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosExtensionRegistry` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `version` | string | REQUIRED | Version of this registry document. SemVer RECOMMENDED. |
| `entries` | array of RegistryEntry | REQUIRED | Catalog payload. At least one entry. |
| `$schema` | string (URI) | OPTIONAL | URI of the JSON Schema this document conforms to. |
| `title` | string | OPTIONAL | Human-readable name for the registry. |
| `description` | string | OPTIONAL | Human-readable description of what the registry catalogs. |
| `extensions` | object | OPTIONAL | Vendor extension data. Keys MUST be prefixed with `x-`. The WOS Working Group also uses reserved `x-wos-*` keys here for WOS-owned identifier metadata tied to the `custodyHook` binding. |

### 1.2 RegistryEntry Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `seam` | string | REQUIRED | Stable identifier for the seam being cataloged. |
| `kind` | enum | REQUIRED | One of `actor-extension`, `contract-hook`, `provenance-layer`, `lifecycle-hook`, `custody-hook`, `vendor-extension`. |
| `lifecycle` | enum | REQUIRED | One of `draft`, `stable`, `deprecated`, `retired`. |
| `description` | string | REQUIRED | Human-readable description of the seam binding. |
| `specRef` | string (URI) | OPTIONAL | URI to the spec section that defines this seam's behavior. |
| `schemaRef` | string (URI) | OPTIONAL | URI to a JSON Schema fragment that constrains the seam payload. |
| `composition` | enum | OPTIONAL | One of `merge`, `replace`, `augment`. See §4. |
| `since` | string | OPTIONAL | Version or date when this entry became available. |
| `deprecatedSince` | string | CONDITIONAL | Required when `lifecycle` is `deprecated` or `retired`. |
| `replacedBy` | string | OPTIONAL | `seam` identifier of the entry that supersedes this one. |
| `examples` | array | OPTIONAL | Sample payloads for documentation. Not normative. |
| `vendorPrefix` | string | CONDITIONAL | Required when `kind` is `vendor-extension`. |
| `extensions` | object | OPTIONAL | Per-entry vendor extension data. |

---

## 2. The Six Kernel Seams

The kernel defines six extension seams (Kernel §10). This section gives one subsection per seam describing what it extends, what shape it accepts, and an exemplar registry entry.

### 2.1 actorExtension

**Extends:** Kernel §10.1.

The kernel defines two actor types: `human` and `system`. Higher layers register additional actor types through this seam. Layer 2 (AI Integration) registers `agent` with additional provenance requirements.

A registry entry for an `actorExtension` binding declares the actor type identifier, the schema fragment that describes additional fields (e.g., model endpoint, autonomy level for `agent`), and the lifecycle stage of that actor type.

```json
{
  "seam": "kernel.actorExtension.agent",
  "kind": "actor-extension",
  "lifecycle": "stable",
  "description": "AI Integration registers the `agent` actor type with model-endpoint, autonomy-level, and provenance-narrative requirements (AI Integration §3).",
  "specRef": "https://wos-spec.org/specs/ai-integration#3-agent-actor",
  "schemaRef": "https://wos-spec.org/schemas/ai-integration/1.0#/$defs/AgentActorDeclaration",
  "since": "1.0.0"
}
```

### 2.2 contractHook

**Extends:** Kernel §10.2.

The kernel defines an abstract contract validation interface. Formspec Definitions are the recommended binding; JSON Schema is the baseline. Layer 1 uses this seam to attach data validation pipelines with assertion gates between stages. Layer 2 uses this seam for the Formspec-as-validator pattern, treating agent output as untrusted input validated against the same Formspec contract a human would submit against.

A registry entry for a `contractHook` binding declares which contract dialect (Formspec, JSON Schema, vendor) the entry attaches and points to the schema or spec section that defines the contract reference shape.

```json
{
  "seam": "kernel.contractHook.formspec",
  "kind": "contract-hook",
  "lifecycle": "stable",
  "description": "Formspec is the recommended contract binding for human task contracts and the agent-output validator pattern (Kernel §10.2, AI Integration §5).",
  "specRef": "https://wos-spec.org/specs/kernel#11-2-conformant-bindings",
  "since": "1.0.0"
}
```

### 2.3 provenanceLayer

**Extends:** Kernel §10.3.

The kernel provides the Facts tier of provenance. Higher layers add interpretive tiers: Layer 1 adds Reasoning and Counterfactual tiers; Layer 2 adds Narrative tier (model-generated explanation, non-authoritative).

A registry entry for a `provenanceLayer` binding declares the tier identifier, the schema for entries written into that tier, and any constraints on tier authority (e.g., narrative tier MUST be marked non-authoritative).

```json
{
  "seam": "ai.provenanceLayer.narrative",
  "kind": "provenance-layer",
  "lifecycle": "stable",
  "description": "Layer 2 narrative tier carries model-generated explanations. MUST be marked non-authoritative; downstream tooling MUST NOT use narrative entries as evidence (AI Integration §7.4).",
  "specRef": "https://wos-spec.org/specs/ai-integration#7-4-narrative-tier",
  "since": "1.0.0"
}
```

### 2.4 lifecycleHook

**Extends:** Kernel §10.4.

This is the primary governance seam. Governance documents from higher layers declare rules that match on semantic transition tags (Kernel §4.12). The kernel publishes tags; layers declare rules against those tags.

A registry entry for a `lifecycleHook` binding declares the hook identifier, the matching mode (tag-based or transition-id-based), and the document type that carries the hook declaration.

```json
{
  "seam": "governance.lifecycleHook.tagBased",
  "kind": "lifecycle-hook",
  "lifecycle": "stable",
  "description": "Governance attaches review and SLA rules to transitions by tag (e.g., all transitions tagged `determination` require dual-blind review). The tag-based mode is the default and applies across the workflow without naming specific transitions (Governance §4).",
  "specRef": "https://wos-spec.org/specs/workflow-governance#4-rules-by-tag",
  "composition": "augment",
  "since": "1.0.0"
}
```

### 2.5 custodyHook

**Extends:** Kernel §10.5.

Every WOS deployment handles protected content under a declared custody posture. The kernel makes no assumption about custody — a trust-the-host monolith and a multi-party distributed binding both conform to the kernel unchanged. Higher layers and bindings attach custody semantics here.

The kernel does NOT define the concrete Trust Profile object. Trellis (the distributed-trust binding) defines that object and binds it to this seam. A monolithic binding may populate this seam with a single declared posture and satisfy conformance.

A registry entry for a `custodyHook` binding catalogs a specific custody profile shape — for example, the Trellis distributed trust profile — and points to the schema that describes its fields.

```json
{
  "seam": "x-trellis.custodyHook.distributed",
  "kind": "custody-hook",
  "lifecycle": "stable",
  "description": "Trellis distributed-trust profile: declares custodian set, recovery-without-user policy, and delegated-compute exposure policy (Trellis §2). Binding registrations populate the kernel `custodyHook` seam with this shape.",
  "specRef": "https://trellis-spec.org/specs/trust-profile#2-distributed",
  "schemaRef": "https://trellis-spec.org/schemas/trust-profile/1.0#/$defs/DistributedTrustProfile",
  "vendorPrefix": "x-trellis-",
  "since": "1.0.0"
}
```

### 2.5.1 WOS-owned custody append identifiers

The `custodyHook` seam itself stays abstract at the kernel layer. The WOS-owned authored-record binding described in [WOS Custody Hook Encoding](../kernel/custody-hook-encoding.md) uses two identifier sets that must also be published:

- the `wos.*` event-type namespace
- the reserved TypeID family prefixes

The registry publishes those through reserved root `extensions` keys:

- `x-wos-custody-event-types`
- `x-wos-typeid-family-prefixes`
- `x-wos-owning-spec-version`

These keys are WOS-owned metadata, not vendor extensions. They are published in `extensions` because the registry's first-class structure still catalogs seams; the identifier metadata is attached reference data for the `custodyHook` seam rather than a new seam kind.

The `x-wos-custody-event-types` array SHOULD publish, at minimum, one entry for each WOS-owned layer:

- `wos.kernel.*`
- `wos.governance.*`
- `wos.ai.*`
- `wos.assurance.*`

Each event-type entry SHOULD disclose:

- `eventType`
- `recordKind`
- `layer`
- `typeIdFamilyPrefix`
- `specRef`

The `x-wos-typeid-family-prefixes` array SHOULD publish, at minimum, the reserved WOS prefixes:

- `case`
- `prov`
- `gov`
- `ai`
- `assurance`

The root `x-wos-owning-spec-version` value pins which WOS spec version owns the published identifier set. Trellis-bound registries reference this version when binding the WOS-authored append surface.

### 2.6 vendor-extension

**Extends:** Kernel §10.6.

WOS supports two parallel mechanisms for vendor extensions: an `extensions` container property and `x-`-prefixed sibling keys on any object. Both follow the naming rules of Kernel §10.6: lowercase ASCII, shape `x-<namespace>-<name>`, and the reserved prefix `x-wos-` is forbidden for non-WOS use.

A registry entry for a `vendor-extension` binding REQUIRES `vendorPrefix` and SHOULD declare a `schemaRef` so lint tooling can validate that any `x-` key matching the prefix conforms to the declared schema.

```json
{
  "seam": "x-acme.tenantId",
  "kind": "vendor-extension",
  "lifecycle": "stable",
  "description": "Acme tenant identifier attached to actor declarations and provenance entries to scope multi-tenant deployments. Single-string opaque identifier; lint tooling validates the prefix only.",
  "vendorPrefix": "x-acme-tenant-",
  "since": "1.0.0"
}
```

---

## 3. Lifecycle Semantics

This section is normative.

### 3.1 The Four Stages

| Stage | Promise | Adopter Action |
|-------|---------|----------------|
| `draft` | Shape MAY change without notice. Behavior MAY change. | Experimental use only. MUST NOT bind in production. |
| `stable` | Shape preserved across minor registry versions. Behavior preserved. | Safe to bind. Catalog SHOULD remain stable for the major-version window. |
| `deprecated` | Continues to resolve. Shape unchanged from stable. Behavior unchanged. | Migrate to the entry named in `replacedBy`. |
| `retired` | Processors MUST reject documents binding to a retired entry. | Migration MUST be complete before adopting a registry version that lists the entry as `retired`. |

### 3.2 Transition Rules

Lifecycle transitions are monotonic in a single registry version line:

1. `draft` → `stable`: any version. Marks the entry as production-ready.
2. `stable` → `deprecated`: MUST set `deprecatedSince` and SHOULD set `replacedBy`.
3. `deprecated` → `retired`: MAY occur in a major-version bump. MUST NOT skip directly from `stable` to `retired`.
4. `retired` → any: forbidden. A retired entry is gone; reusing the `seam` identifier under a different shape is forbidden because it would silently change behavior for adopters that pinned an older registry version.

A registry MAY remove a `retired` entry entirely in the next major version.

### 3.3 What Lifecycle Does NOT Promise

- Lifecycle does NOT promise behavioral compatibility across registries from different publishers. A `stable` Trellis entry and a `stable` Acme entry both promise their own shape; they do not promise to compose with each other.
- Lifecycle does NOT promise schema-level backwards compatibility for `vendor-extension` entries — vendors govern their own schemas. The lifecycle promise is about the seam binding (its existence and prefix claim), not the shape behind it.

---

## 4. Composition Semantics

This section is normative.

When multiple registrations bind to the same `seam`, the `composition` field on the registry entry tells processors how to combine them.

### 4.1 The Three Modes

| Mode | Combine rule | When to use |
|------|--------------|-------------|
| `merge` | Object key-wise merge. Later registrations override earlier keys field-by-field. Nested objects merge recursively. | Object-shaped seams where independent layers contribute disjoint fields (e.g., a custody hook where one layer adds custodian list and another adds recovery policy). |
| `replace` | Latest registration wholly supersedes prior bindings. Earlier values are discarded. | Singleton seams where only one binding can be authoritative (e.g., a primary contract dialect declaration). |
| `augment` | Each registration appends to a list. Order is preserved. Entries do not collide. | List-shaped seams where every contribution is additive (e.g., lifecycle hooks attaching governance rules to tags). |

### 4.2 Defaults

When `composition` is absent, the default is:

- `merge` for entries whose `schemaRef` resolves to an object schema.
- `augment` for entries whose `schemaRef` resolves to an array schema.
- `replace` when `schemaRef` is absent — the registry cannot infer shape, so the conservative default is "last writer wins".

### 4.3 Conflicts

A processor that loads two registries declaring the same `seam` with conflicting `composition` values MUST reject the load with a structured error identifying both registry sources and the conflicting values. Composition is part of the seam contract; silently picking one rule would corrupt downstream document processing.

---

## 5. Discovery and Resolution

This section is normative.

### 5.1 Discovery

A WOS deployment discovers registries through one of:

1. **Explicit reference.** A kernel document, sidecar, or processor configuration names a registry by URI. The processor fetches the registry at load time.
2. **Bundled registry.** The WOS Working Group publishes a built-in `WOS Core Extension Registry` cataloging the six kernel seams. Processors SHOULD include this as an implicit baseline.
3. **Vendor-supplied registry.** A vendor whose extensions appear in a deployment publishes its own registry; the deployment lists the vendor registry alongside the core registry.

A processor MAY load multiple registries. Order of discovery is implementation-defined but MUST be deterministic for a given deployment configuration so that resolution outcomes are reproducible across replays.

### 5.2 Resolution

To resolve a seam binding:

1. Collect all entries across loaded registries that match the requested `seam` identifier.
2. Filter to entries whose `lifecycle` is not `retired`. A `retired` entry MUST cause the processor to reject the document if a non-retired alternative does not exist.
3. If exactly one entry remains, use it.
4. If multiple entries remain, apply the entry's `composition` mode to combine them. Composition conflicts are resolved per §4.3.

### 5.3 Following `replacedBy`

When the resolved entry is `deprecated` and declares `replacedBy`, the processor SHOULD emit a deprecation warning naming the replacement. Processors MAY auto-follow the `replacedBy` chain to locate the live entry, but SHOULD record both the deprecated entry the document referenced and the live entry actually used in provenance, so audit traces capture the migration silently performed.

`replacedBy` chains MUST be acyclic and MUST terminate at a non-deprecated entry. A processor that detects a cycle MUST reject the registry with a structured error naming the entries in the cycle.

---

## 6. Conformance

This section is normative.

### 6.1 Required Behaviors

A WOS-conformant processor that loads an Extension Registry MUST:

1. Reject documents that do not validate against the registry schema (`$wosExtensionRegistry` marker, required fields present, enum values respected).
2. Reject documents binding to entries whose `lifecycle` is `retired`.
3. Reject registries containing a `replacedBy` cycle.
4. Reject loads when two registries declare the same `seam` with conflicting `composition` values.
5. Treat `seam` identifiers as opaque, case-sensitive strings.
6. Preserve `x-`-prefixed keys on registry entries through round-trip read/write cycles unless the processor documents the stripping behavior.

### 6.2 Absence Behavior

When no registry is loaded, the processor MUST behave as if only the implicit `WOS Core Extension Registry` were present (the six kernel seams at `lifecycle: stable`). Vendor extensions in this case are accepted but unvalidated — lint tooling cannot match `x-` keys to a known prefix.

### 6.3 Warnings vs Errors

| Condition | Severity |
|-----------|----------|
| Document binds to a `deprecated` entry. | warning |
| Document binds to a `draft` entry. | warning |
| Document binds to an unrecognized seam (no entry in any loaded registry). | warning (forward-compat) |
| Document binds to a `retired` entry. | error |
| Registry contains a `replacedBy` cycle. | error |
| Two registries declare the same `seam` with conflicting `composition`. | error |
| Registry document fails schema validation. | error |

The forward-compatibility rule (unrecognized seam → warning, not error) mirrors Kernel §10.6 processor behavior on unrecognized `x-` keys: WOS prefers preserving forward compatibility over rejecting documents that name extensions a processor does not yet know about.

---

## References

### Normative

- WOS Kernel Specification v1.0 §10 (Named Extension Seams).
- WOS Kernel Specification v1.0 §10.6 (`extensions` and `x-` keys).

### Informative

- WOS Workflow Governance Specification v1.0 §4 (Tag-based governance attachments via `lifecycleHook`).
- WOS AI Integration Specification v1.0 §3 (Agent actor type registered via `actorExtension`).
- Trellis Trust Profile Specification (custody hook binding example).
