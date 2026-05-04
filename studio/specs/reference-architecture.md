# Studio Reference Architecture

**Stage 7.** Defines the system-level reference architecture for WOS
Studio (Authoring): the layer model, the component model (composed
from the 16 existing studio/specs/*.md), the Studio-side port and
adapter-seam catalog, the projection-target model, the canonical
flows, and the trust/governance invariants. Stage 8+ implements
this contract.

This spec is a meta-spec: it composes existing specs by reference
and adds the abstractions required to compose them into a
deployable system. It does NOT restate behavioral semantics that
the 16 existing specs already own — it points at them.

`SA-MUST-arch-*` tracking IDs name the normative obligations.

## Identity

Studio is a **governed knowledge-to-operational-artifact platform**:
ingest authoritative corpora and systems context, structure them
into reviewed knowledge objects, project them into operational
artifacts under deterministic governance.

Two claims:

- **Knowledge platform.** The corpus + reviewed-knowledge graph
  (Source Vault + PolicyObjects + Policy Knowledge Map +
  Effectiveness + Provenance) is reusable across projection
  targets.
- **Multi-projection.** WOS workflow is the first-class workflow
  projection. Formspec form is the first-class form projection.
  Decision artifacts, integration bindings, scenario suites,
  approval packages, and signed export bundles round out v1.
  Future: data contracts, reports, public knowledge bases.

WOS Studio is **not "a WOS backend"** — `crates/wos-server` is the
runtime that consumes Studio's published artifacts.

## Normative Contract

### Layer model

`SA-MUST-arch-001` — Studio MUST be organized as five layers; each
layer reads from the layers below and authors into the layer above.

```
┌──────────────────────────────────────────────────┐
│  Publication Layer                               │
│  workflow / form / decision / binding /          │
│  scenario / approval / export-bundle             │
├──────────────────────────────────────────────────┤
│  Validation Layer                                │
│  readiness (S1–S6) / scenario / coverage /       │
│  traceability / mapping completeness             │
├──────────────────────────────────────────────────┤
│  Design Layer                                    │
│  WorkflowIntent / DecisionRule + DecisionTable / │
│  ServiceBinding + EventBinding + DMNImport /     │
│  Effectiveness / Mappings                        │
├──────────────────────────────────────────────────┤
│  Authoring Layer                                 │
│  AI-native extraction / review / mapping /       │
│  conflict resolution / approval                  │
├──────────────────────────────────────────────────┤
│  Knowledge Layer                                 │
│  Sources + Citations / PolicyObjects /           │
│  CanonicalTerms / Authority / Provenance /       │
│  Assumptions / Conflicts / Effectiveness         │
└──────────────────────────────────────────────────┘
```

### Component model

`SA-MUST-arch-002` — Every Studio component MUST map to one of the
named entries below. Components carry the existing spec citation
where one exists; new abstractions are justified inline.

#### Knowledge layer

| Component | Owner spec / artifact |
|---|---|
| **Source Vault** | [`source-vault.md`](source-vault.md); [`../schemas/wos-studio-source.schema.json`](../schemas/wos-studio-source.schema.json) |
| **Policy Knowledge Map** (a.k.a. Knowledge Graph) | [`../VISION.md`](../VISION.md) §9.3 |
| **Retrieval Index** | NEW (Stage 7 names port; semantic-retrieval projection over sources + reviewed knowledge objects; specified as port only in §"Port catalog") |

#### Authoring layer

| Component | Owner spec / artifact |
|---|---|
| **AI Orchestrator** | [`../VISION.md`](../VISION.md) §10 (AI Copilot); Stage 7 names runtime composition |
| **Review Queue** | [`review-and-approval.md`](review-and-approval.md) + [`../VISION.md`](../VISION.md) §9.2 (Policy Extraction Review) |
| **Authoring Ledger** (persistence of `AuthoringProvenanceRecord`) | [`authoring-provenance.md`](authoring-provenance.md) |

#### Design layer

| Component | Owner spec / artifact |
|---|---|
| **WorkflowIntent Authoring** | [`workflow-intent.md`](workflow-intent.md) |
| **Form Intent Authoring** | NOT IN STUDIO — Formspec owns form definition (per `specs/core/spec.md` AD-01 "Schema is data, not code"). Studio's Form Projection emits Formspec source JSON over the wire (per ADR 0089 §2.4 amended 2026-05-04); no Rust-API coupling between products. |
| **DecisionModel Authoring** (knowledge tier) | Unified per ADR 0089 §2.7 (amended 2026-05-04): one knowledge-tier `DecisionModel` PolicyObject subsumes the prior `DecisionRule` + the structured form of `DMNImport`. Rule-form vs table-form is a slot, not a separate kind. Cited in [`policy-object-model.md`](policy-object-model.md) at the next spec revision. |
| **DecisionBinding Authoring** (binding tier) | Unified per ADR 0089 §2.7: one binding-tier `DecisionBinding` subsumes the prior `DecisionTable` (multi-row form) and projects to kernel `decisionTables[*]` + `DecisionTableGuard` per Kernel §4.5.1. Cited in [`binding-and-integration.md`](binding-and-integration.md) at the next spec revision. |
| **DMN one-pass importer** | `DMNImport` becomes a one-way DMN→DecisionModel transpiler at the boundary; the Studio-internal artifact is a DecisionModel (knowledge-tier), not a parallel kind. Parent "DMN one-way import only" commitment (Kernel §4.5.1.5) preserved. |
| **Requirement family Authoring** | First-class `DataRequirement` PolicyObject kind (per ADR 0089 §2.8 amended 2026-05-04) sibling to existing `EvidenceRequirement`. Future siblings reserved: `AccessRequirement`, `IdentityRequirement`. **Satisfier seam:** a Requirement points at the artifact that satisfies it via Formspec `semanticType` (registry-mediated, per `schemas/definition.schema.json:682`) or an `x-wos-satisfies` extension on the Definition — NEVER raw field-key equality (which collapses on multi-form / multi-jurisdiction stacks). |
| **Integration Binding Authoring** | [`binding-and-integration.md`](binding-and-integration.md) |

#### Validation layer

| Component | Owner spec / artifact |
|---|---|
| **Readiness / Validation Runner** | [`readiness-validation.md`](readiness-validation.md); [`../STUDIO-LINT-MATRIX.md`](../STUDIO-LINT-MATRIX.md); impl `wos-studio-lint` |
| **Scenario Runner** | [`scenario-authoring.md`](scenario-authoring.md); impl `wos-studio-scenario` |
| **Change Impact / Coverage** | [`change-impact.md`](change-impact.md) |

#### Publication layer

| Component | Owner spec / artifact |
|---|---|
| **Workflow Projection** | [`compiler-contract.md`](compiler-contract.md); impl `wos-studio-compiler` (9 phases) |
| **Form Projection** | Studio emits Formspec source JSON (per ADR 0089 §2.4 amended); Formspec compiler consumes it like any author-emitted Definition. [`../../crates/wos-formspec-binding/`](../../crates/wos-formspec-binding) covers the existing intake-handoff seam (ADR 0073); Stage 8 adds the authoring-time emit path. |
| **Decision Projection** | Emits unified DecisionModel + DecisionBinding (per ADR 0089 §2.7); single emit pipeline replaces the three-kind composition. |
| **Integration Binding Projection** | [`binding-and-integration.md`](binding-and-integration.md); emit pipeline pending |
| **ApprovalPackage Builder** | [`review-and-approval.md`](review-and-approval.md) |
| **ExportBundle Builder** | [`compiler-contract.md`](compiler-contract.md) Phase 9 |
| **ProjectionTarget / ExportSink** (port) | NEW — see §"Projection target model" |

#### Application boundary

| Component | Owner spec / artifact |
|---|---|
| **Working Store** | [`workspace.md`](workspace.md) (workspace state); Stage 7 names the storage port |
| **API / Application Boundary** | NEW — Stage 7 names the surface; Stage 8+ implements |

`SA-MUST-arch-003` — Where the repo already names a concept
(`AuthoringProvenanceRecord`, `Policy Knowledge Map`,
`ApprovalPackage`), Studio MUST retain the existing name.
Cross-domain glosses (Authoring Ledger, Knowledge Graph) appear in
this spec for orientation only — they do NOT rename the underlying
artifacts.

### Data ownership model

`SA-MUST-arch-010` — Each data store has exactly one owning component:

- **Authoring Ledger** owns immutable authoring provenance —
  persistence of `AuthoringProvenanceRecord`
  ([`authoring-provenance.md`](authoring-provenance.md)).
  Hash-chained, signed, exportable as PROV-O.
- **Working Store** owns current workspace state — live, mutable;
  the "now" view onto sources, PolicyObjects, mappings, scenarios,
  intents ([`workspace.md`](workspace.md)).
- **Source Vault** owns immutable source binaries + parsed sections
  — content-addressed; new versions = new blobs
  ([`source-vault.md`](source-vault.md)).
- **Policy Knowledge Map** owns the graph projection of reviewed
  knowledge ([`../VISION.md`](../VISION.md) §9.3).
- **Retrieval Index** owns the semantic-retrieval projection
  (embeddings).
- **ApprovalPackage / ExportBundle** own the published reproducible
  boundary ([`review-and-approval.md`](review-and-approval.md);
  [`compiler-contract.md`](compiler-contract.md) Phase 9).

`SA-MUST-arch-011` — **Replay / rebuild contract.** Workspace
projections (Working Store, Policy Knowledge Map, Retrieval Index)
MUST be reconstructible from: the **Authoring Ledger** + the
immutable **Source Vault** blobs + **recorded AI outputs** +
versioned **parser** metadata + versioned **prompt** metadata +
versioned **model identity** metadata + versioned **projection**
metadata.

The audit ledger alone is NOT sufficient. The immutable inputs and
recorded AI outputs are part of the replay contract.

### Port catalog (Studio-side)

`SA-MUST-arch-020` — Studio MUST expose its substrate dependencies
as named ports. Concrete adapters live in separate crates and
depend only on the core port definition + their substrate library.

These ports are **distinct from `crates/wos-server`'s ports**
(EventStore, BlobStore, RuntimeOps, AuthzService, AuthProvider,
KmsAdapter, ProcessingService, Observability, Trellis export per
[`../../crates/wos-server/VISION.md`](../../crates/wos-server/VISION.md)).
Studio's ports live at the **authoring** layer; server ports live
at the **runtime** layer. A subset (`IdentityProvider`,
`KeyManager`) MAY share parent definitions; that decision is open
(see §"Open questions").

| Port | Purpose |
|---|---|
| `SourceVault` | Store + retrieve immutable source documents + parsed sections; content-addressed |
| `AuthoringLedger` | Append + read `AuthoringProvenanceRecord`s; hash-chain integrity; signed entries |
| `WorkingStore` | Current workspace state (relational read-write) |
| `PolicyKnowledgeMap` | Entity / edge persistence; graph queries |
| `RetrievalIndex` | Embed + query semantic vectors |
| `ParserAdapter` | Source bytes + content-type → parsed sections |
| `ModelAdapter` | LLM invocation with structured-output binding; returns AI-lineage record |
| `EmbeddingAdapter` | Text → embedding vector |
| `PromptRegistry` | Versioned prompt template store + promotion governance |
| `IdentityProvider` | OIDC; session resolution |
| `KeyManager` | Publish-time signing keys |
| `ProjectionTarget` (≡ `ExportSink`) | Pluggable projection emitters; see §"Projection target model" |
| `ScenarioRunner` | Composes existing `wos-studio-scenario` |
| `ValidationRunner` | Composes existing `wos-studio-lint` + readiness gates |
| `WorkerQueue` | Async work scheduling for ingestion + AI extraction |
| `CorpusFeed` (placeholder) | Streaming corpus subscription (maximalist) |

`SA-MUST-arch-021` — The Studio core crate MUST define the ports.
Each adapter crate MUST depend on core + its substrate library
only; cross-adapter dependencies are forbidden. Enforced by a
boundary guard test mirroring
[`../crates/wos-studio-types/tests/api_surface.rs`](../crates/wos-studio-types/tests/api_surface.rs).

### External-OSS-adapter seams

`SA-MUST-arch-030` — External tools MUST attach behind named
adapter seams; they MUST NOT become normative dependencies. Stage 7
specifies the seams; Stage 8+ ships reference adapters.

| Seam | Purpose | Reference-adapter candidates | Earliest stage |
|---|---|---|---|
| `KnowledgeMemoryAdapter` | Vector + graph memory for retrieval-assisted authoring | Cognee (research-only; see caveat) | **Stage 10+ research item** with measurable criteria. Not Stage 8. |
| `DataConnectorAdapter` | Source ingestion (corpora, systems-of-record) | dlt, Airbyte | Stage 9+ |
| `MetadataCatalogAdapter` | Catalog / schema registry for systems-of-record | OpenMetadata, DataHub | Stage 9+ |
| `LineageAdapter` | Data-lineage interop | OpenLineage | Stage 9+ |
| `DataContractAdapter` | Data-contract emission | ODCS, Data Contract spec | Stage 9+ |
| `QualityCheckAdapter` | Data-quality checks over ingested sources / projected outputs | Great Expectations, Soda | Stage 9+ |
| `ProjectionTarget` (≡ `ExportSink`) | Output sink (see §"Projection target model") | WOS, Formspec, decision, integration, scenario, approval, export-bundle, future report | Stage 7 contract; Stage 8 first impl |
| `KnowledgeQueryService` (read-only) | Runtime knowledge queries by external consumers (Formspec chat, future reports / briefings / public knowledge bases). Distinct from `ProjectionTarget` (build-time emit) — this is runtime read. | Studio-internal REST + JSON over the wire | **Placeholder** — landed when first external consumer (Formspec chat candidate) is ready; not Stage 8. |

`SA-MUST-arch-031` — Studio's structured-output stance is JSON
Schema + OpenAPI first. This matches the existing schema-first WOS
/ Formspec pattern (15 Studio schemas, WOS workflow schema,
Formspec form schemas) and minimizes early complexity. **LinkML is
deferred** until drift across JSON Schema, JSON-LD, SHACL, docs,
and generated Rust types becomes operationally painful. Current
alignment (JSON Schema + a SHACL sidecar via
`wos-ontology-alignment`) is sufficient for v1.

`SA-MUST-arch-032` — A `KnowledgeMemoryAdapter` reference impl
over Cognee (or any other knowledge-memory tool) MUST NOT become
Studio's canonical knowledge source of truth unless it preserves,
end-to-end:

- source-span citations,
- review state,
- approval lifecycle,
- AI lineage,
- effectivity / temporal validity,
- conflicts + supersession,
- deterministic export,
- projection semantics.

Until validated, the canonical source-of-truth remains Studio's
typed model (`wos-studio-model`) + Source Vault + Policy Knowledge
Map; external knowledge-memory sits behind the adapter as a
retrieval/index accelerant.

**v1 stance (amended 2026-05-04 per ADR 0091):** Stage 8 ships
*boring* reference adapters for graph + retrieval — Postgres
recursive CTE for `PolicyKnowledgeMap`, pgvector for
`RetrievalIndex`. Cognee is NOT used in v1. Re-evaluation is a
Stage 10+ research item with measurable criteria (retrieval
quality vs Postgres baseline, query latency at agency-scale
corpus, ops cost). The seam stays open; the substrate choice
defers.

`SA-MUST-arch-033` — `KnowledgeQueryService` is a read-only
runtime port (NOT a build-time `ProjectionTarget`) for external
consumers querying Studio's reviewed knowledge graph. Stage 7
names it as a placeholder; it lands when the first external
consumer (e.g., Formspec's authoring chat) is ready. Operations:
semantic search, traceability ("what authority backs this claim"),
concept resolution ("what does the registry call household
income, and what fields satisfy it"), effectivity check ("is this
rule still in force on date D in jurisdiction J"). Wire format:
REST + JSON; same posture as the rest of Studio's external
surface; no Rust-API coupling. Same permission gating, source-use
policy, and audit-log primitives that Studio uses internally
extend to external consumers without reshape.

### Projection target model

`SA-MUST-arch-040` — The platform MUST emit artifacts via a
uniform `ProjectionTarget` (≡ `ExportSink`) port. Every projection
follows the same shape:

```
ValidatedKnowledgeModel + Intent
  → projection logic (target-specific compiler/builder)
  → emitted artifact (target-specific schema)
  → validation (target-specific readiness/lint/conformance)
  → packaged into ApprovalPackage / ExportBundle
```

`SA-MUST-arch-041` — Stage 7 ratifies the following projection
targets. WOS workflow and Formspec form are co-equal first-class
projections.

| Target | v1 status | Notes |
|---|---|---|
| WOS workflow (`$wosWorkflow`) | first-class (existing) | `wos-studio-compiler` |
| Formspec form | first-class projection target | Formspec owns form definition; Studio projects via Formspec adapter; reuses `formspec-studio-core`, `formspec-engine`, `formspec-webcomponent` |
| Decision artifact (DecisionRule / DecisionTable / DMNImport) | spec only | Pending DecisionModel resolution |
| Integration binding package | spec only | [`binding-and-integration.md`](binding-and-integration.md) |
| Data contract | spec only | Adapter seam (`DataContractAdapter`) |
| Scenario suite | exists | `wos-studio-scenario` |
| ApprovalPackage | specified | Provenance + signatures |
| ExportBundle | exists | `wos-studio-compiler` Phase 9 |
| Reports / briefing memos | maximalist | Future |
| Public knowledge base | maximalist | Future federation |
| Runtime observation feedback | maximalist | wos-server emits → Studio observes |

### Canonical flows

`SA-MUST-arch-050` — Studio MUST support the following canonical
flows. Each names which components participate, which ports are
exercised, which `AuthoringProvenanceRecord`s emit, where
validation gates apply, where review interleaves.

1. **Source ingest** — upload → `SourceVault` → `ParserAdapter` →
   parsed sections → `RetrievalIndex` → `AuthoringLedger` entry
   (`eventKind: extracted`, `eventSubtype: source-ingest`).
2. **Knowledge extraction** — sources + retrieval + prompt +
   `ModelAdapter` → structured candidate → schema validation +
   cite-back + verifier → ConfidenceRecord → review queue →
   reviewer action → durable PolicyObject → `AuthoringLedger`
   entry (`eventKind: extracted` then `approved`).
3. **Connection / mapping / insight** — second AI pass over
   reviewed knowledge → `PolicyKnowledgeMap` edges + Mappings +
   Conflicts + Gaps + Insights → review → durable.
4. **Workflow projection** — reviewed knowledge + WorkflowIntent +
   Mappings → `wos-studio-compiler` 9-phase pipeline →
   `$wosWorkflow` → readiness + lint + conformance → packaged.
5. **Form projection** — reviewed knowledge + Formspec inputs (held
   in Formspec, not Studio) → Formspec compiler → form artifact →
   form validation → packaged. (`wos-formspec-binding` provides
   the seam.)
6. **Decision projection** — reviewed knowledge + DecisionRule /
   DecisionTable / DMNImport → decision compiler (TBD) →
   artifact → packaged.
7. **Integration binding projection** — reviewed knowledge +
   binding intent → binding compiler → package → packaged.
8. **Validation / scenarios** — composed Validation Runner (S1–S6
   tiers) + Scenario Runner + change-impact + traceability →
   readiness verdict.
9. **Approval + publish** — assemble ApprovalPackage +
   ExportBundle → publish-time signing key (`KeyManager`) →
   ProjectionTarget sink.
10. **Change impact** — `ChangeImpactReport` over the
    `PolicyKnowledgeMap` → review queue refreshes
    ([`change-impact.md`](change-impact.md)).

### Trust / governance invariants

#### Threat-model framing

`crates/wos-server`'s **content-blind respondent-case-data threat
model does not apply to Studio** (per
[`../../crates/wos-server/VISION.md`](../../crates/wos-server/VISION.md)
lines 15–28). Studio still requires:

- `SA-MUST-arch-060` — Tenant isolation (one workspace ↔ one
  corpus ↔ one tenant in v1; structurally separable later).
- `SA-MUST-arch-061` — Access control via existing `AuthorityGrant`
  ([`workspace.md`](workspace.md)).
- `SA-MUST-arch-062` — Auditability via `AuthoringProvenanceRecord`
  ([`authoring-provenance.md`](authoring-provenance.md)).
- `SA-MUST-arch-063` — Source integrity (content-addressed Source
  Vault; source-version pinning).
- `SA-MUST-arch-064` — Publication signing (signed ApprovalPackages
  + ExportBundles).
- `SA-MUST-arch-065` — Source / model-use policy controls (which
  sources may feed which extractions; which models are permitted;
  per-tenant policy bindings).
- `SA-MUST-arch-066` — **Workspace-genesis external attestation —
  blocked at Stage 7.** External anchoring of workspace existence
  via the WOS `custodyHook` seam was evaluated and rejected for
  v1 (per ADR 0087 amendment 2026-05-04, validated against
  `specs/kernel/custody-hook-encoding.md` 2026-05-04). The
  `custodyHook` four-field append is per-case-keyed (TypeID
  prefix `case`, eventType `wos.<layer>.<recordKind>` with layer
  ∈ {kernel, governance, ai, assurance}); workspace-genesis is a
  non-case event with no registered prefix or layer namespace.
  Stage 8 anchors the ledger genesis to a workspace-derived hash
  only (per ADR 0087 §2.3); external attestation requires a
  parent kernel amendment (registered `workspace` family prefix +
  `authoring` layer in PLN-0384's event-types taxonomy) tracked
  as a separate ADR escalation.

#### Authoring invariants

- `SA-MUST-arch-070` — Every AI-emitted artifact MUST carry
  cite-back to a source span.
- `SA-MUST-arch-071` — AI proposals MUST NOT be approved facts.
  Reviewed objects become durable.
- `SA-MUST-arch-072` — Only approved + mapped + valid workspace
  subgraphs MAY publish.
- `SA-MUST-arch-073` — Publication MUST emit immutable signed
  packages.
- `SA-MUST-arch-074` — Studio authors; runtime executes.
  Observations MAY flow back into Studio only through a new
  authoring cycle.

#### AI invariants

- `SA-MUST-arch-080` — Schema-guided structured extraction (LLM
  constrained to a schema; JSON Schema / OpenAPI structured outputs
  first per `SA-MUST-arch-031`).
- `SA-MUST-arch-081` — Validator loop MUST run on every output.
- `SA-MUST-arch-082` — A **ConfidenceRecord** combining:
  schema-validation result, citation-support score, retrieval
  score, verifier result, risk tier, and human-review state. **No
  single signal — least of all the model's self-reported
  confidence — gates approval alone.**
- `SA-MUST-arch-083` — AI lineage captured per call, extending the
  existing `AuthoringProvenanceRecord` AI subtype: model identity
  + version, prompt template + version, input hash, output hash,
  retrieval set hash, validator verdicts, verifier result, timing.
- `SA-MUST-arch-084` — Human approval before durable high-impact
  behavior.
- `SA-MUST-arch-085` — **Verifier model independence.** The verifier
  invocation (per `SA-MUST-arch-082` confidence record) MUST run a
  **different model family** from the primary extractor (e.g.,
  Claude Opus extracts, Claude Sonnet or GPT-5 verifies). Same-
  model-family verification rubber-stamps the extractor's
  systematic biases; family independence is what makes the second
  read genuinely independent. Provider may be the same or
  different; the load-bearing axis is *model family*, not
  *provider*. WOS spec is currently silent on this axis (per ADR
  0088 amendment 2026-05-04 + parent validation 2026-05-04); this
  invariant is Studio-side and enforced by Studio lint rule
  `RA-LINT-085` against workspace AI configuration.
- `SA-MUST-arch-086` — **Source / model-use policy** is enforced
  at the Studio API boundary via typed declarative policy structs
  in Rust (`SourceUsePolicy`, `ModelUsePolicy`), per ADR 0088 §2.6
  amended 2026-05-04. Promotion to `PolicyEngineBinding`
  (OPA/Cedar) is admitted at scale (>50 rules or attribute-based
  access) but not required at v1. The FEL-as-only-expression-
  language commitment binds workflow-runtime guards, not
  Studio-tier admission control (per parent validation
  2026-05-04).

#### Replay invariant

See `SA-MUST-arch-011` above.

## Composition

### Where this attaches

`reference-architecture.md` is the **Studio-tier system-architecture
anchor**. It composes (does not restate) the 16 existing
[`studio/specs/*.md`](README.md) by reference. Six ADRs decompose
the load-bearing decisions:

| ADR | Title |
|---|---|
| 0086 | Studio Knowledge Platform — reference architecture |
| 0087 | Persistence + projection model (Source Vault, Authoring Ledger, Working Store, Policy Knowledge Map, Retrieval Index, replay contract) |
| 0088 | AI extraction + cite-back + ConfidenceRecord contract; structured-output stance |
| 0089 | Projection target model (`ProjectionTarget` / `ExportSink`) |
| 0090 | Publish / export boundary (signing, ApprovalPackage, ExportBundle, ExportSink) |
| 0091 | External-tool adapter seams + port/adapter architecture |

### Precedence

When this spec and a more specific spec (e.g.,
`compiler-contract.md`, `authoring-provenance.md`) appear to
conflict, the more specific spec wins for the artifact it owns.
This spec wins for cross-component composition, port surfaces, and
the projection-target model.

### Conflict handling

If a new component proposed here collides with an existing
artifact in `wos-studio-model` / `wos-studio-types` / a Stage 1–6
spec, the **existing artifact wins** and this spec is amended. New
abstractions are admitted only when none of the 16 existing specs
already covers the concern (see `SA-MUST-arch-002`).

### Versioning

This spec evolves with Studio's stage roadmap. Substantive
revisions to the port catalog, adapter-seam catalog, or
projection-target model require an ADR amendment to the
corresponding 0086–0091 entry.

### Composition with parent wos-spec

Studio composes (does not reinvent) parent contracts already
indexed in [`README.md`](README.md) §"Composition with parent
wos-spec". Studio's port catalog is **disjoint** from
`crates/wos-server`'s port catalog. The two systems share artifact
types (the published `$wosWorkflow` and ExportBundle) at the
boundary — Studio publishes; server consumes.

### Reuse map

| Existing artifact | Reused for |
|---|---|
| `wos-studio-types` | Boundary discipline; extend with `ConfidenceRecord`, `AILineage`, `ProjectionRef`, `ApprovalPackageRef`, `ExportBundleRef` aliases |
| `wos-studio-model` | Knowledge + Design typed model; extend if `Open questions` resolve toward new objects |
| `wos-studio-lint` | Validation Runner |
| `wos-studio-compiler` | Workflow Projection + ExportBundle Builder |
| `wos-studio-scenario` | Scenario Runner |
| 16 [`studio/specs/*.md`](README.md) | Cited inline in §"Component model" |
| 15 [`studio/schemas/`](../schemas/) | Object-shape sources of truth |
| `crates/wos-formspec-binding` | Form-projection + intake-handoff seam (ADR 0073) |
| `crates/wos-server` | Runtime consumer; **separate** port set |
| `crates/wos-export` | Published-artifact transport |
| `crates/wos-core::studio_api` | Boundary surface for Studio→WOS |

### New abstractions (justified)

- `ProjectionTarget` / `ExportSink` — no existing port covers
  heterogeneous projection emission; Phase-9 export bundle is one
  emitter, not a generalized port.
- `AuthoringLedger` (port) — record shape exists in
  [`authoring-provenance.md`](authoring-provenance.md); the
  persistence port is new.
- `PolicyKnowledgeMap` (port) — [`../VISION.md`](../VISION.md) §9.3
  specifies the model; a graph-query port surface is new.
- `AI Orchestrator` (component) — coordinates ports already
  implicit in extraction; not yet a named composition.
- Seven external-OSS-adapter seams (`KnowledgeMemoryAdapter`,
  `DataConnectorAdapter`, `MetadataCatalogAdapter`,
  `LineageAdapter`, `DataContractAdapter`, `QualityCheckAdapter`,
  `ProjectionTarget`).
- `WorkingStore`, `RetrievalIndex` (ports) — implicit today; named.

### Boundary with Formspec

Form Projection wires to Formspec's existing packages
(`packages/formspec-studio-core`, `packages/formspec-engine`,
`packages/formspec-webcomponent`) via the
[`../../crates/wos-formspec-binding/`](../../crates/wos-formspec-binding)
adapter. **Studio does not introduce a "FormIntent" object** —
that overlaps Formspec's authoring surface. Studio supplies the
reviewed knowledge model (DataElement, EvidenceRequirement,
DecisionRule with citations and effectivity) that the Formspec
projection consumes.

### Boundary with `crates/wos-server`

Studio publishes; server consumes. Studio's API surface
(`SA-MUST-arch-002` "API / Application Boundary") is the authoring
HTTP/WS surface. The server's API surface is the runtime case
surface. The two systems share the published artifacts at the
boundary; they do not share the storage substrate or the port set.

## Conformance

### Schema validation

The following are checked by JSON Schema validation today:
- All Studio object shapes in [`../schemas/`](../schemas/) (15
  schemas). New objects introduced by Stage 7 (`ConfidenceRecord`,
  AI-lineage extensions) extend
  `wos-studio-provenance.schema.json` per ADR 0088.

### Lint

The following are checked by `wos-studio-lint` today (70 rules
S1–S6 per [`../STUDIO-LINT-MATRIX.md`](../STUDIO-LINT-MATRIX.md)):
- All readiness gates referenced in §"Validation layer".

### Runtime conformance

The following are checked by `wos-studio-compiler` +
`wos-studio-scenario` today:
- Phase 9 export bundle determinism + reproducibility.
- Scenario expected-vs-actual trace comparison.

### Stage 7 boundary tests

`SA-MUST-arch-090` — A boundary guard test MUST mirror
[`../crates/wos-studio-types/tests/api_surface.rs`](../crates/wos-studio-types/tests/api_surface.rs)
for the Studio-side port catalog: enforce that adapter crates
depend on core + their substrate library only.

### Stage 7 conformance skeletons

Trait-shape tests against the port catalog (no impls) live with
the contract-code crate landed in Stage 7. Stage 8 fills them with
real adapters.

### Stage 7 gaps (`*(impl-pending)*`)

The following normative obligations are spec-only at Stage 7;
Stage 8+ adapters realize them:

- `SA-MUST-arch-020` *(impl-pending)* — port crate not yet wired.
- `SA-MUST-arch-021` *(impl-pending)* — boundary guard test stub
  ships in Stage 7; cross-adapter assertion fires at Stage 8 when
  the first adapter lands.
- `SA-MUST-arch-030..032` *(impl-pending)* — adapter seams have no
  reference adapters yet.
- `SA-MUST-arch-040..041` *(impl-pending)* — `ProjectionTarget`
  port is named; only WOS workflow projection (existing
  `wos-studio-compiler`) and Phase-9 ExportBundle are wired today.
- `SA-MUST-arch-080..083` *(impl-pending)* — AI extraction
  pipeline lands in Stage 8.

## Open questions

Most prior open questions resolved 2026-05-04 after parent-spec
validation by `wos-expert` + `spec-expert` (Formspec). Resolutions
are encoded in ADRs 0087–0091 amendments and in the new
`SA-MUST-arch-085`, `SA-MUST-arch-086`, `SA-MUST-arch-066`,
`SA-MUST-arch-033`. The remainder is genuinely open.

### Resolved 2026-05-04

| Question | Resolution | Owner |
|---|---|---|
| DecisionModel unification | Unify within tier: knowledge-tier `DecisionModel` + binding-tier `DecisionBinding`; `DMNImport` becomes a one-pass importer producing `DecisionModel` | ADR 0089 §2.7 |
| DataRequirement first-class status | Promoted to first-class PolicyObject kind; sibling to `EvidenceRequirement`; satisfier seam via `semanticType` / `x-wos-satisfies`, NOT raw field-key equality | ADR 0089 §2.8 |
| Form-projection adapter shape | Studio emits Formspec source JSON; Formspec compiler treats it as ordinary author-emitted Definition; zero Rust-API coupling | ADR 0089 §2.4 (amended) |
| Verifier model identity | Always-different model family (`SA-MUST-arch-085`); enforced by Studio lint `RA-LINT-085` | ADR 0088 §2.4 (amended) |
| Source / model-use policy enforcement | Typed declarative Rust structs at API boundary (`SA-MUST-arch-086`); promote to `PolicyEngineBinding` only at scale | ADR 0088 §2.6 (amended) |
| ConfidenceRecord schema location | Own file: `wos-studio-confidence.schema.json`; not a `$def` in another schema | ADR 0088 §2.1 (amended) |
| KnowledgeMemoryAdapter / Cognee | Skip for v1; Postgres recursive CTE + pgvector as boring reference adapters; Cognee Stage 10+ research item with measurable criteria | ADR 0091 §2.2 (amended); `SA-MUST-arch-032` v1 stance |
| Stage 7 contract-code location | Split to `wos-studio-server-core` when first adapter lands at Stage 8; `arch.rs` moves out of `wos-studio-types` | ADR 0091 §2.5 (amended) |
| Studio dependency on parent identity ports | Build on ADR-0084 strict-subset placeholder; promote to shared `wos-identity-ports` crate when parent PLN-0381 ratifies | ADR 0091 §2.4 (amended) |
| Workspace-genesis external attestation via `custodyHook` | **BLOCKED** at Stage 7 — `custodyHook` is per-case-keyed; workspace-genesis is non-case event with no registered prefix or layer namespace; deferred pending parent kernel amendment (PLN-0384 layer expansion) | `SA-MUST-arch-066`; ADR 0087 §2.3 (amended) |

### Still open

- **Authoring Ledger persistence port shape — verifier of chain
  on cold reads.** Stage 8 default: per-row Ed25519 signature +
  prev-hash chain (per ADR 0087 §2.3–2.4). Cold-read verifier
  performance + pagination shape pending Stage 8 wire-up.
- **`KnowledgeQueryService` operations contract.** `SA-MUST-arch-033`
  names the operations; the wire-format JSON Schema (semantic
  search response, traceability response, concept resolution
  response, effectivity response) lands when the first external
  consumer (Formspec chat candidate) is ready, not Stage 8.
- **Maximalist:** continuous corpus subscriptions (`CorpusFeed`);
  multi-modal ingestion (parser extensions); cross-org federation
  (`PolicyKnowledgeMap` + ledger interop); per-tenant model
  fine-tuning; runtime-observation feedback (wos-server → Studio);
  external workspace-genesis attestation (gated on PLN-0384 kernel
  amendment per `SA-MUST-arch-066`).

## Cross-references

- Stage 8 vertical-slice plan:
  [`stage-8-vertical-slice.md`](stage-8-vertical-slice.md).
- ADR set: [`../../thoughts/adr/0086-studio-knowledge-platform-reference-architecture.md`](../../thoughts/adr/0086-studio-knowledge-platform-reference-architecture.md)
  through `0091`.
- VISION roadmap: [`../VISION.md`](../VISION.md) §17 (Stage 7 entry
  to be updated to point here).
- Existing 16 specs: [`README.md`](README.md).
- Existing 15 schemas: [`../schemas/`](../schemas/).
- Boundary guard pattern:
  [`../crates/wos-studio-types/tests/api_surface.rs`](../crates/wos-studio-types/tests/api_surface.rs).
- Parent server VISION:
  [`../../crates/wos-server/VISION.md`](../../crates/wos-server/VISION.md).
- Conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
