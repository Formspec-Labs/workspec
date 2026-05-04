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
| **Form Intent Authoring** | NOT IN STUDIO — Formspec owns form definition. Studio projects via the Formspec ProjectionTarget adapter. |
| **DecisionRule + DecisionTable Authoring** | [`policy-object-model.md`](policy-object-model.md) §"DecisionRule"; [`binding-and-integration.md`](binding-and-integration.md) §"DecisionTable" + §"DMNImport" |
| **Data Requirements Authoring** | [`../VISION.md`](../VISION.md) §9.5 (DataElement). **OPEN — see §"Open questions":** whether to introduce a unified `DataRequirement` first-class object. |
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
| **Form Projection** | [`../../crates/wos-formspec-binding/`](../../crates/wos-formspec-binding) (intake-handoff seam, ADR 0073). Stage 8 wires Studio→Formspec authoring projection. |
| **Decision Projection** | Pending DecisionModel resolution (see §"Open questions") |
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

| Seam | Purpose | Reference-adapter candidates |
|---|---|---|
| `KnowledgeMemoryAdapter` | Vector + graph memory for retrieval-assisted authoring | Cognee (prototype only — see governance caveat) |
| `DataConnectorAdapter` | Source ingestion (corpora, systems-of-record) | dlt, Airbyte |
| `MetadataCatalogAdapter` | Catalog / schema registry for systems-of-record | OpenMetadata, DataHub |
| `LineageAdapter` | Data-lineage interop | OpenLineage |
| `DataContractAdapter` | Data-contract emission | ODCS, Data Contract spec |
| `QualityCheckAdapter` | Data-quality checks over ingested sources / projected outputs | Great Expectations, Soda |
| `ProjectionTarget` (≡ `ExportSink`) | Output sink (see §"Projection target model") | WOS, Formspec, decision, integration, scenario, approval, export-bundle, future report |

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

These block sections of the implementation; each must resolve
before the corresponding adapter can ship.

### Repo-grounded

- **DecisionModel unification.** Today: `DecisionRule` (policy
  object) + `DecisionTable` + `DMNImport` (binding). Should Stage
  7 introduce a unified `DecisionModel` object? Affects Decision
  Projection and the projection-target catalog.
- **DataRequirement first-class status.** `DataElement` exists
  ([`../VISION.md`](../VISION.md) §9.5); should Stage 7 promote
  `DataRequirement` to a first-class object distinct from
  `EvidenceRequirement`? Affects Stage 8 slice.
- **Authoring Ledger persistence port shape.** Record exists; port
  signatures, signing algorithm (Ed25519 per actor vs HMAC), and
  hash-chain framing pending — owned by ADR 0087.
- **Form-projection adapter shape.** Wrap Formspec's existing
  compiler with an adapter, or stand up a Studio-side intermediate
  representation? `wos-formspec-binding` covers intake handoff,
  not authoring projection.
- **Studio dependency on parent `wos-server-ports`.** Does Studio
  share `IdentityProvider` / `KeyManager` definitions with
  `crates/wos-server`, or define its own?
- **Stage 7 contract-code location.** New crate
  `studio/crates/wos-studio-server-core/` (or analogous), or
  extension of `wos-studio-types`? Stage 7 starts with the
  conservative choice (extend `wos-studio-types`); Stage 8 may
  promote to a dedicated crate.

### External-tool grounded

- **Cognee under governance.** Pre-flight checklist
  (`SA-MUST-arch-032`) must be validated before any production
  wiring.
- **Verifier model identity.** Same provider as primary extractor,
  or always different?
- **`ConfidenceRecord` schema.** Exact field set — ADR 0088 owns.
- **Source / model-use policy enforcement.** Declarative policy
  language vs imperative checks at the API boundary?

### Maximalist directions

Continuous corpus subscriptions (`CorpusFeed`); multi-modal
ingestion (parser extensions); cross-org federation
(`PolicyKnowledgeMap` + ledger interop); per-tenant model
fine-tuning; runtime-observation feedback (wos-server → Studio).

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
