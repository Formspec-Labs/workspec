# Stage 8 Vertical Slice — Production Plan

**Stage 8.** Width-one path through every layer of the
[`reference-architecture.md`](reference-architecture.md). One
parser, one model, one set of projection targets, one ExportSink,
one review action, one signed bundle. The slice proves the
architecture end-to-end before parallel projections, OSS adapters,
and production hardening land in Stage 9+.

This document is a planning artifact, not a normative spec — it
sequences the work that realizes the Stage 7 contract. The
underlying obligations live in
[`reference-architecture.md`](reference-architecture.md) and the
16 [`studio/specs/*.md`](README.md) it composes.

## Scope

The Stage 8 slice broadens the prior "workflow-only" framing
captured in [`../VISION.md`](../VISION.md) §17 Stage 8. It is
**deliberately wider** so that the architecture's
multi-projection claim is exercised on day one:

```
upload one source (PDF)
  → ParserAdapter → SourceVault → RetrievalIndex
  → schema-guided AI extraction (one ModelAdapter; one prompt;
    validator + verifier pass; AI lineage captured) producing:
      • one DataRequirement (or DataElement, pending §"Open question")
      • one EvidenceRequirement
      • one DecisionRule (PolicyObject)
  → human review/approval (one Review Queue action)
  → durable PolicyObjects (Working Store + Authoring Ledger)
  → PolicyKnowledgeMap traceability query (cite-back path
    source → claim → policy)
  → projections via ProjectionTarget:
      • Formspec form fields + validation (via wos-formspec-binding)
      • WOS workflow step + decision input (via wos-studio-compiler)
  → one Scenario over (form data + workflow path)
    using wos-studio-scenario
  → ApprovalPackage (existing spec)
  → signed ExportBundle (Phase 9 + signing)
  → one ExportSink (filesystem)
```

The slice **reuses** the existing
[`examples/snap-redetermination-from-sources/`](../examples/snap-redetermination-from-sources/)
fixture as the corpus and target. The compiled
`wos-workflow.json`, scenarios, mappings, approval-package, and
provenance log already exist; Stage 8 wires the live pipeline so
that re-running the slice from PDF input reproduces the
fixture-pinned outputs deterministically.

## Stage 8 ships

### Adapters

- 1× `ParserAdapter` — PDF (e.g., `pdftotext`-backed; minimal
  parsed-section structure).
- **2× `ModelAdapter`** — extractor + verifier with always-
  different model families (`SA-MUST-arch-085`); e.g., Claude
  Opus extracts, Claude Sonnet or GPT-5 verifies. Studio lint
  `RA-LINT-085` enforces.
- 1× `EmbeddingAdapter` — pgvector.
- 1× `SourceVault` — filesystem-backed; content-addressed.
- 1× `WorkingStore` — Postgres (single-tenant for the slice;
  `studio_projections.workspace` schema per ADR 0087).
- 1× `AuthoringLedger` — Postgres `studio_canonical.ledger`;
  per-row Ed25519 signature + prev-hash chain;
  workspace-derived genesis (NOT Trellis-anchored — that path
  is blocked at Stage 7 per `SA-MUST-arch-066`).
- 1× `PolicyKnowledgeMap` — Postgres recursive CTE
  (`wos-studio-knowledge-graph-pg`). Boring reference adapter
  per ADR 0091 §2.2 amended; Cognee NOT used in v1.
- 1× `IdentityProvider` — OIDC with one configured IdP. Built
  on ADR-0084 strict-subset placeholder; promotion to shared
  `crates/wos-identity-ports/` deferred to parent PLN-0381
  ratification.
- 1× `KeyManager` — dev-mode (file-backed); documented gap to
  HSM/KMS in Stage 9+.

### Projection targets (three)

- **WOS workflow projection** — wired through existing
  `wos-studio-compiler`.
- **Formspec form projection** — Studio emits Formspec source
  JSON (per ADR 0089 §2.4 amended). Formspec compiler reads it
  the same way it reads author-emitted Definitions. Existing
  [`../../crates/wos-formspec-binding/`](../../crates/wos-formspec-binding)
  remains the runtime-side intake-handoff seam (ADR 0073);
  Stage 8 adds the authoring-time **emit** path.
- **Decision projection** — emits unified DecisionModel +
  DecisionBinding (per ADR 0089 §2.7 amended). Single emit
  pipeline; replaces the prior three-kind composition.

### Sinks (one)

- 1× `ExportSink` — filesystem. Trellis-network sink lands in
  Stage 9+.

### Application surface (minimal)

- REST API covering: source upload, extraction trigger, review
  decision, projection trigger, approval submission, export
  download.
- No real-time collaboration; no WebSocket; no gRPC. (Stage 9+.)

### Tests

- Boundary guard test (mirrors
  [`../crates/wos-studio-types/tests/api_surface.rs`](../crates/wos-studio-types/tests/api_surface.rs))
  fires once the first adapter lands.
- **Replay test** — exercises
  [`reference-architecture.md`](reference-architecture.md)
  `SA-MUST-arch-011`: rebuild Working Store + Policy Knowledge
  Map + Retrieval Index from Authoring Ledger + Source Vault +
  recorded AI outputs + versioned metadata. Assert byte-identity
  for the published ExportBundle.
- Slice integration test — full PDF → ExportBundle path,
  deterministic against the SNAP fixture.

## Deliverable sequence

The slice deliberately interleaves substrate and projection work
to surface integration risks early.

| # | Deliverable | Dependency |
|---|---|---|
| 1 | Move `arch.rs` from `wos-studio-types` → new `wos-studio-server-core` crate; new `wos-studio-server` composition root crate (per ADR 0091 §2.5 amended) | Stage 7 contract code |
| 2 | `SourceVault` filesystem adapter | (1) |
| 3 | `WorkingStore` Postgres adapter (`studio_projections.workspace`) | (1) |
| 4 | `AuthoringLedger` Postgres adapter (`studio_canonical.ledger`; Ed25519 per actor + workspace-derived genesis per ADR 0087 §2.3 amended — NOT Trellis-anchored, blocked per `SA-MUST-arch-066`) | (1) |
| 5 | `ParserAdapter` PDF adapter | (1), (2) |
| 6 | **Two** `ModelAdapter`s — extractor + verifier with always-different model families (`SA-MUST-arch-085`); `EmbeddingAdapter` (pgvector); `PromptRegistry` (filesystem-backed); new `wos-studio-confidence.schema.json` per ADR 0088 §2.1 amended | (1) |
| 7 | AI extraction pipeline (Source → Candidate → ConfidenceRecord → Review Queue); `DataRequirement` + `EvidenceRequirement` + `DecisionModel` extracted per ADR 0089 §2.7–2.8 amended | (3), (4), (5), (6) |
| 8 | `PolicyKnowledgeMap` Postgres-CTE adapter (boring reference adapter per ADR 0091 §2.2 amended; Cognee NOT used) | (1), (3) |
| 9 | Workflow projection wired through existing `wos-studio-compiler`; Decision projection emits unified `DecisionModel` + `DecisionBinding` | (3), (4), (8) |
| 10 | Formspec projection — Studio **emits Formspec source JSON** per ADR 0089 §2.4 amended; Formspec compiler consumes ordinarily | (3), (8) |
| 11 | Scenario runner wired through existing `wos-studio-scenario` | (9), (10) |
| 12 | ApprovalPackage assembly + signing (`KeyManager` dev-mode); `SourceUsePolicy` / `ModelUsePolicy` typed structs at API boundary per `SA-MUST-arch-086` | (4), (9), (10), (11) |
| 13 | ExportBundle signing extension (Phase 9 → signed) | (9), (12) |
| 14 | `ExportSink` filesystem adapter | (13) |
| 15 | REST API surface for the slice | (2)–(14) |
| 16 | Replay + slice-integration tests | (15) |

## Open questions — most resolved 2026-05-04

Resolutions per validation 2026-05-04 (`wos-expert` + `spec-expert`)
encoded into ADRs 0087–0091 amendments. Stage 8 wire-up follows
the resolved decisions; only one substrate-level open remains.

### Resolved entry assumptions for Stage 8

| Original open | Resolution | Affects deliverable |
|---|---|---|
| DataRequirement first-class | **First-class PolicyObject kind** sibling to `EvidenceRequirement`. Satisfier seam via Formspec `semanticType` / `x-wos-satisfies` (per `definition.schema.json:682`), NOT raw field-key equality. (ADR 0089 §2.8) | 7 |
| DecisionModel unification | **Unify within tier** — knowledge-tier `DecisionModel` + binding-tier `DecisionBinding`; `DMNImport` becomes one-pass importer producing `DecisionModel`. (ADR 0089 §2.7) | 9 |
| Form-projection adapter shape | **Studio emits Formspec source JSON.** Formspec compiler treats Studio output exactly like author-emitted Definition; zero Rust-API coupling. (ADR 0089 §2.4 amended) | 10 |
| Authoring Ledger persistence | Ed25519 per actor + per-row hash chain + workspace-derived genesis hash. **Trellis-anchored genesis blocked** — `custodyHook` is per-case-keyed; `wos.authoring.*` event-type namespace not registered. Requires parent kernel amendment (PLN-0384 layer expansion). (ADR 0087 §2.3, amended) | 4 |
| Studio identity-port reuse | **Build on ADR-0084 strict-subset placeholder for Stage 8; promote to shared `crates/wos-identity-ports/` crate when parent PLN-0381 ratifies.** Both products take the same migration once. (ADR 0091 §2.4 amended) | 1 |
| Contract-code location | **Move `arch.rs` out of `wos-studio-types`** when first adapter lands. New crates: `wos-studio-server-core` (port traits + type aliases) + `wos-studio-server` (composition root). (ADR 0091 §2.5 amended) | 1 |
| Verifier model identity | **Always-different model family.** Stage 8 ships two `ModelAdapter` instances (e.g., Claude Opus extracts, Claude Sonnet or GPT-5 verifies). Studio lint `RA-LINT-085` enforces. (`SA-MUST-arch-085`; ADR 0088 §2.4 amended) | 6, 7 |
| Source/model-use policy | **Typed declarative Rust structs at Studio API boundary** (`SourceUsePolicy`, `ModelUsePolicy`). No OPA/Cedar at v1. (`SA-MUST-arch-086`; ADR 0088 §2.6 amended) | 15 |
| ConfidenceRecord schema location | **Own file:** `studio/schemas/wos-studio-confidence.schema.json`. Other schemas `$ref` into it. (ADR 0088 §2.1 amended) | 7 |
| KnowledgeMemoryAdapter / Cognee | **Skip for v1.** Boring reference adapters: Postgres recursive CTE (`PolicyKnowledgeMap`), pgvector (`RetrievalIndex`). Cognee re-evaluation Stage 10+ research item. (ADR 0091 §2.2 amended) | 8 |

### Still open (Stage 8 wire-up)

- **Authoring Ledger cold-read verifier.** Per-row signature +
  prev-hash chain shape pinned (ADR 0087 §2.3); cold-read
  verifier performance + pagination shape pending Stage 8
  wire-up. Affects deliverable 4.
- **Stage 8 fixture target — SNAP example reproducibility.**
  The slice MUST reproduce the existing
  [`../examples/snap-redetermination-from-sources/`](../examples/snap-redetermination-from-sources/)
  ExportBundle byte-identically (replay test per `SA-MUST-arch-011`
  + ADR 0087 §2.5). Open: any drift between the existing fixture
  and the unified DecisionModel + DataRequirement family will be
  resolved by **migrating the SNAP fixture forward** as part of
  deliverable 1, not by holding the architecture back.

### Out of scope for Stage 8 (per amended ADRs)

- Trellis-anchored workspace-genesis attestation (gated on parent
  kernel amendment; tracked separately).
- `KnowledgeQueryService` runtime read API (lands when first
  external consumer — Formspec chat candidate — is ready).
- Cognee adapter (Stage 10+ research).
- `wos-identity-ports` shared crate extraction (gated on parent
  PLN-0381 ratification).

## Out of scope (Stage 9+)

- Production OSS adapters per
  [`reference-architecture.md`](reference-architecture.md)
  §"External-OSS-adapter seams" (Cognee, dlt, OpenMetadata,
  OpenLineage, ODCS, Great Expectations).
- Real-time collaboration (WebSocket).
- Second model provider.
- gRPC service-to-service.
- Prompt-registry promotion governance (versioned promotion ladder).
- Multi-tenant deployment.
- Public-knowledge-base projection.
- Cross-org federation.
- Trellis-network ExportSink.
- HSM/KMS production wiring.

## Cross-references

- Reference architecture:
  [`reference-architecture.md`](reference-architecture.md).
- Existing fixture target:
  [`../examples/snap-redetermination-from-sources/`](../examples/snap-redetermination-from-sources/).
- ADR set: 0086–0091 in
  [`../../thoughts/adr/`](../../thoughts/adr/).
- VISION roadmap: [`../VISION.md`](../VISION.md) §17 Stage 8 entry
  (to be updated to point here).
