# ADR-0091: Studio port/adapter architecture + external-tool adapter seams

**Status:** Proposed 2026-05-04 · Amended 2026-05-04 (validation)
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/CLAUDE.md`](../../studio/CLAUDE.md) §"Boundary discipline" — the existing one-way-dependency rule extends to the new port catalog.

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0087 (sibling — persistence ports use this architecture)
- ADR 0088 (sibling — `ModelAdapter` / `EmbeddingAdapter` use this architecture)
- ADR 0089 (sibling — `ProjectionTarget` is one of these ports)
- ADR 0090 (sibling — `ExportSink` is the destination side of `ProjectionTarget`)
- ADR-0073 (parent — `IntakeHandoff`; precedent for adapter-shaped seams)
- ADR-0084 (parent — PLN-0381 identity attestation Studio anchor; strict-subset placeholder for shared identity-ports)

---

## Amendment 2026-05-04 — Cognee deferred, server-core split, identity-ports sequencing, KnowledgeQueryService

Four §2 decisions tightened or added after parent-spec validation
(`wos-expert` + `spec-expert` 2026-05-04) and a follow-on owner
note about Formspec's chat as a future external knowledge
consumer.

### §2.2 — KnowledgeMemoryAdapter: Cognee deferred to Stage 10+

**Original (now superseded):** Cognee listed as "S8+ exploratory"
reference adapter for `KnowledgeMemoryAdapter`.

**Amended:** **Cognee is NOT used in v1.** Stage 8 ships *boring*
reference adapters:
- `wos-studio-knowledge-graph-pg` — Postgres recursive CTE for
  `PolicyKnowledgeMap` traversal queries.
- `wos-studio-retrieval-pgvector` — pgvector for `RetrievalIndex`.

`KnowledgeMemoryAdapter` seam stays open at the architecture
level; Cognee re-evaluation is a **Stage 10+ research item with
measurable criteria** (retrieval quality vs Postgres baseline at
agency-scale corpus, query latency, ops cost).

**Rationale.** Cognee is young, opinionated, and would couple
v1's correctness story to its evolution and governance gaps.
Boring database tools (Postgres, pgvector) are well-understood
operationally; one Postgres database serves canonical +
projections + graph + retrieval. Single backup story; single
transaction model; single ops surface. The scale where Postgres
recursive CTEs fail is the scale where the workspace would be
sharded anyway (typical agency regulatory graph: ~10K policy
objects, ~50K edges).

### §2.4 — Identity-ports sequencing (was §2.4 open)

**Original (now superseded):** "Stage 8 default: define
Studio-side traits with the same shape as the parent's;
reconcile in Stage 9+ if a shared crate proves useful."

**Amended:** **Build on ADR-0084 strict-subset placeholder for
Stage 8; promote to a shared `crates/wos-identity-ports/` crate
when parent PLN-0381 ratifies.**

`studio/specs/identity-and-attestation.md` explicitly says
Studio's job is to *bind* PLN-0381 primitives, not invent them.
ADR-0084 already pins the Studio-side `AttestationEnvelope`
placeholder as a strict subset of parent PLN-0381's expected
shape, with `$ref` migration when ratified. A shared crate is
the structural form of that convergence.

**Sequencing.** Don't extract the shared crate prematurely —
parent PLN-0381 is unratified. Stage 8 builds Studio's
`IdentityProvider` / `KeyManager` against the ADR-0084
placeholder; promotion to the shared crate happens **at the same
moment** parent PLN-0381 ratifies and Studio swaps the `$ref`.
Both products take the same migration once.

### §2.5 — Stage 7 contract-code location: split at Stage 8 (was §2.1 open)

**Original (now superseded):** "Stage 8 default is to extend
`wos-studio-types`; promote to a dedicated crate only if the
trait surface grows past what the boundary guard sustains."

**Amended:** **Move `arch.rs` out of `wos-studio-types` when the
first Stage 8 adapter lands.** Two new crates:
- `studio/crates/wos-studio-server-core/` — port traits + adapter
  seam traits + type aliases. Stable, slow-changing.
- `studio/crates/wos-studio-server/` — composition root: REST
  API, worker queue, dependency injection. Iterates fast.

`wos-studio-types` returns to its original ~30-line shim role
(`wos-core::studio_api` re-exports only).

**Rationale.** Contract code (stable) and composition code
(fast-iterating) want different crates from day one. Adapter
crates depend on `wos-studio-server-core`, never on
`wos-studio-types`. Build-graph stays clean. Future repo
extraction (`git filter-repo --subdirectory studio/`) produces
a self-consistent tree without reshape. ~2 hours of file moves +
Cargo.toml updates today; avoids superlinear build-time growth
forever.

### §2.6 — `KnowledgeQueryService` placeholder port (NEW)

A new port name reserves architectural space for Studio's
reviewed knowledge graph being consumed by **external runtime
readers** (distinct from `ProjectionTarget`, which is build-time
emit).

**First anticipated consumer:** Formspec's authoring chat. When
a form designer in Formspec asks "what does the regulation say
about household income?", Formspec's chat queries Studio's
`KnowledgeQueryService` rather than maintaining its own knowledge
base. Same source of truth as Studio's authoring chat — a policy
author and a form designer working on the same SNAP redetermina-
tion get the **same answer to the same question**.

**Operations** (wire-format pending Stage 9+ first consumer):
- Semantic search ("what regulations cover this concept?").
- Traceability ("what authority backs this claim, and through
  which sources?").
- Concept resolution ("what does the registry call household
  income, and what fields satisfy it?") — used by
  `DataRequirement` satisfier resolution per ADR 0089 §2.8.
- Effectivity check ("is this rule still in force on date D in
  jurisdiction J?").

**Posture.** Read-only. Same permission gating, source-use
policy, audit-log primitives that Studio uses internally extend
to external consumers without reshape. Wire format: REST + JSON
(matches "documents over the wire, no Rust-API coupling" rule
from ADR 0089 §2.4 amended).

**Stage.** Placeholder at Stage 7. Lands when first external
consumer (Formspec chat candidate) is ready; not Stage 8.
Reference-architecture spec encodes as `SA-MUST-arch-033` and
joins the seam catalog row.

**Future consumers** beyond Formspec chat: report generators,
briefing-memo composers, public knowledge bases, regulator-
facing portals. Same port; same posture.

---

## 1. Context

[`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md)
§"Port catalog" enumerates 16 Studio-side ports. §"External-OSS-
adapter seams" enumerates 7 adapter seams for replaceable
reference adapters (Cognee, dlt/Airbyte, OpenMetadata,
OpenLineage, ODCS, Great Expectations, plus `ProjectionTarget`).

The repo already has a strong precedent for adapter discipline:
- `studio/CLAUDE.md` enforces one-way dependencies via the
  workspace-wide guard at
  [`crates/wos-studio-types/tests/api_surface.rs`](../../studio/crates/wos-studio-types/tests/api_surface.rs).
- `crates/wos-server/VISION.md` lines 122–133 names a 7-port
  adapter table (EventStore, BlobStore, …) with cargo-enforced
  separation.
- ADR 0073 (IntakeHandoff) demonstrates the adapter-shaped
  seam pattern.

This ADR ratifies that the **same pattern** governs Studio's port
catalog, and that **external tools attach as replaceable
reference adapters behind named seams** rather than becoming
normative dependencies.

A common alternative — "vendor Cognee as Studio's canonical
knowledge memory" — is **rejected**. Studio's typed model
(`wos-studio-model`) + Source Vault + Policy Knowledge Map remain
canonical; external tools sit behind the
`KnowledgeMemoryAdapter` as a retrieval/index accelerant. This
keeps Studio free to swap adapters and prevents external-tool
governance from leaking into Studio's spec surface.

## 2. Decision

### 2.1 Port / adapter discipline

The 16 Studio-side ports (per
[`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md)
§"Port catalog") follow this discipline:

1. **Core crate defines the trait.** Stage 7 ships trait stubs
   either in `studio/crates/wos-studio-types` (extension) or in a
   new `studio/crates/wos-studio-server-core` (open question).
   Stage 8 default is to extend `wos-studio-types`; promote to a
   dedicated crate only if the trait surface grows past what the
   boundary guard sustains.
2. **Each adapter crate depends on core + its substrate library
   only.** No cross-adapter dependencies. Each adapter is
   independently testable, swappable, and removable.
3. **Boundary guard test enforces structurally** — mirror the
   existing
   [`crates/wos-studio-types/tests/api_surface.rs`](../../studio/crates/wos-studio-types/tests/api_surface.rs)
   for the port catalog.
4. **Naming convention.** Adapter crate names follow
   `wos-studio-<port>-<substrate>` (e.g.,
   `wos-studio-source-vault-fs`, `wos-studio-ledger-postgres`,
   `wos-studio-knowledge-cognee`).

### 2.2 External-tool adapter seam catalog

| Seam | Purpose | Reference-adapter candidates | Stage |
|---|---|---|---|
| `KnowledgeMemoryAdapter` | Vector + graph memory for retrieval-assisted authoring | Cognee (prototype only — see §2.3) | S8+ exploratory |
| `DataConnectorAdapter` | Source ingestion (corpora, systems-of-record) | dlt, Airbyte | S9+ |
| `MetadataCatalogAdapter` | Catalog / schema registry | OpenMetadata, DataHub | S9+ |
| `LineageAdapter` | Data-lineage interop | OpenLineage | S9+ |
| `DataContractAdapter` | Data-contract emission | ODCS, Data Contract spec | S9+ |
| `QualityCheckAdapter` | Data-quality checks | Great Expectations, Soda | S9+ |
| `ProjectionTarget` (≡ `ExportSink`) | Output sink (per ADR 0089) | WOS, Formspec, decision, integration, scenario, approval, export-bundle, future report | S7 contract; S8 first impl |

Each seam is a Studio-defined trait. The reference adapter
candidates are the OSS tools the Studio team has surveyed; Stage
9+ may ship one, multiple, or none of them.

### 2.3 Knowledge-memory governance constraints

A `KnowledgeMemoryAdapter` reference impl over Cognee (or any
other knowledge-memory tool) MUST NOT become Studio's canonical
knowledge source of truth unless the substrate preserves,
end-to-end:

- source-span citations,
- review state,
- approval lifecycle,
- AI lineage,
- effectivity / temporal validity,
- conflicts + supersession,
- deterministic export,
- projection semantics.

Until validated, the canonical source-of-truth remains:
- `wos-studio-model` (the typed model).
- Source Vault (immutable source store).
- Policy Knowledge Map (the graph projection of reviewed
  knowledge).

Cognee (or equivalent) sits behind `KnowledgeMemoryAdapter` as a
retrieval/index accelerant — a **performance** dependency, not a
**correctness** dependency. The Stage 7 spec
(`SA-MUST-arch-032`) makes this load-bearing.

### 2.4 Composition with parent server ports

Studio's port catalog is **disjoint** from
`crates/wos-server`'s port catalog (per
[`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md)
§"Port catalog"). The two systems share artifact types (the
published `$wosWorkflow` and ExportBundle) at the boundary; they
do not share storage or port traits.

Two ports — `IdentityProvider` and `KeyManager` — MAY share
parent definitions because identity and key custody are
cross-cutting infrastructure. **Open**: Stage 8 default is to
define Studio-side traits with the same shape as the parent's;
reconcile in Stage 9+ if a shared crate proves useful.

### 2.5 Stage 7 contract code

Stage 7 ships:

- Trait stubs for every port + every adapter seam.
- Type aliases (`ConfidenceRecord`, `AILineage` extension,
  `ProjectionRef`, `ApprovalPackageRef`, `ExportBundleRef`).
- Boundary guard test (mirrors existing pattern; cross-adapter
  rule fires once the first adapter lands at Stage 8).
- Conformance skeleton tests against the trait surface (shape only).

Stage 7 does NOT ship any concrete adapter.

## 3. Rejected Alternatives

- **Vendor a specific tool (e.g., Cognee) as canonical SoT.**
  Rejected; couples Studio's correctness to external governance.
- **One mega-port crate.** Rejected; trait surface diverges by
  responsibility (storage vs AI vs publication); separation aids
  comprehension and testing.
- **No boundary guard.** Rejected; the parent server's adapter
  table only stays clean because the dep graph is structurally
  enforced. Studio adopts the same pattern.
- **Adapter naming free-for-all.** Rejected; consistent
  `wos-studio-<port>-<substrate>` naming makes the adapter set
  trivially auditable.
- **Share `crates/wos-server`'s port traits directly.** Rejected;
  the abstraction layers are different (authoring vs runtime).
  Sharing trait shapes for `IdentityProvider` / `KeyManager` only,
  and only by convention until Stage 9+ revisits.

## 4. Consequences

### Positive

- Adapter discipline is structurally enforced.
- External-tool churn doesn't churn Studio's spec surface.
- Cognee remains an option without becoming a constraint.
- Future projection targets, source connectors, lineage exporters,
  and quality checks all attach the same way.

### Negative

- 16 ports + 7 seams produce a proliferation of small adapter
  crates over Stage 8–10. Mitigated by the naming convention and
  the boundary guard.
- Open: Stage 7 contract-code location (extend `wos-studio-types`
  vs new `wos-studio-server-core`). Default chosen; revisited if
  surface grows.
- Open: parent-port reuse for `IdentityProvider` / `KeyManager`.
  Default chosen; revisited if a shared crate proves useful.

### Neutral

- The port catalog joins the existing studio crate set without
  reshape.

## 5. Conformance

- `SA-MUST-arch-020..032` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Boundary guard test (Stage 7 stub; Stage 8 cross-adapter
  assertion).
- Adapter naming convention enforced by Stage 8+ code review.
- Existing
  [`crates/wos-studio-types/tests/api_surface.rs`](../../studio/crates/wos-studio-types/tests/api_surface.rs)
  pattern is the structural reference.
