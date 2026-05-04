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

### Adapters (one each)

- 1× `ParserAdapter` — PDF (e.g., `pdftotext`-backed; minimal
  parsed-section structure).
- 1× `ModelAdapter` — one LLM provider; prompt-template directory;
  structured-output binding.
- 1× `EmbeddingAdapter` — pgvector (or in-memory for the slice).
- 1× `SourceVault` — filesystem-backed; content-addressed.
- 1× `WorkingStore` — Postgres (single-tenant for the slice).
- 1× `AuthoringLedger` — Postgres; hash-chained; signed entries
  per ADR 0087.
- 1× `PolicyKnowledgeMap` — Kuzu (candidate); fall back to a
  Postgres recursive-CTE store if Kuzu integration slips.
- 1× `IdentityProvider` — OIDC with one configured IdP.
- 1× `KeyManager` — dev-mode (file-backed); documented gap to
  HSM/KMS in Stage 9+.

### Projection targets (two)

- WOS workflow projection — wired through existing
  `wos-studio-compiler`.
- Formspec form projection — wired through
  [`../../crates/wos-formspec-binding/`](../../crates/wos-formspec-binding)
  + Formspec's existing `formspec-studio-core` /
  `formspec-engine`. Pending the §"Open question" on adapter
  shape.

Decision projection deferred unless the §"Open question" on
DecisionModel resolves to "use existing DecisionRule alone" —
in which case the Stage 8 slice projects DecisionRule directly
(no new compiler needed).

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
| 1 | `wos-studio-server-core` (or `wos-studio-types` extension) — port traits + ConfidenceRecord/AILineage type aliases | Stage 7 contract code |
| 2 | `SourceVault` filesystem adapter | (1) |
| 3 | `WorkingStore` Postgres adapter | (1) |
| 4 | `AuthoringLedger` Postgres adapter (Ed25519 per actor — ADR 0087) | (1) |
| 5 | `ParserAdapter` PDF adapter | (1), (2) |
| 6 | `ModelAdapter` + `EmbeddingAdapter` + `PromptRegistry` | (1) |
| 7 | AI extraction pipeline (Source → Candidate → ConfidenceRecord → Review Queue) | (3), (4), (5), (6) |
| 8 | `PolicyKnowledgeMap` adapter (Kuzu) | (1), (3) |
| 9 | Workflow projection wired through existing `wos-studio-compiler` | (3), (4), (8) |
| 10 | Formspec projection wired through `wos-formspec-binding` (resolution of open question first) | (3), (8) |
| 11 | Scenario runner wired through existing `wos-studio-scenario` | (9), (10) |
| 12 | ApprovalPackage assembly + signing (`KeyManager` dev-mode) | (4), (9), (10), (11) |
| 13 | ExportBundle signing extension (Phase 9 → signed) | (9), (12) |
| 14 | `ExportSink` filesystem adapter | (13) |
| 15 | REST API surface for the slice | (2)–(14) |
| 16 | Replay + slice-integration tests | (15) |

## Open questions blocking Stage 8

Mirrored from
[`reference-architecture.md`](reference-architecture.md)
§"Open questions"; sequenced here by the slice deliverable they
unblock.

1. **DataRequirement first-class status.** Blocks deliverable 7.
   Default: extend `DataElement` for the slice; promote to
   `DataRequirement` only if the slice exercises a use that
   `DataElement` cannot model.
2. **DecisionModel unification.** Blocks deliverable 9 (decision
   input embedded in workflow). Default: project `DecisionRule`
   directly; defer unification.
3. **Form-projection adapter shape.** Blocks deliverable 10.
   Default: thin adapter that wraps Formspec's existing compiler
   inputs; do not introduce a Studio-side intermediate
   representation unless the integration friction proves
   substantial.
4. **Authoring Ledger persistence port shape.** Blocks deliverable
   4. ADR 0087 owns; Stage 8 entry assumption: Ed25519 per actor.
5. **Studio dependency on parent `wos-server-ports`.** Blocks
   deliverable 1 (KeyManager + IdentityProvider). Default for
   Stage 8: define Studio-side traits with the same shape as the
   parent's; reconcile in Stage 9+ if a shared crate proves
   useful.
6. **Stage 7 contract-code location.** Affects deliverable 1.
   Default: extend `wos-studio-types` for Stage 7 / 8; promote to
   `wos-studio-server-core` only if the trait surface grows past
   what the boundary guard can sustain.

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
