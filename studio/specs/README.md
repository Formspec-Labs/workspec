# WOS Studio (Authoring) — Specs Index

**Seventeen** W3C-style specs that together define how WOS Studio (Authoring) transforms source documents into a `$wosWorkflow` file (and other operational artifacts). Each spec follows the three-section rubric (Normative Contract / Composition / Conformance) per [`../../CONVENTIONS.md`](../../CONVENTIONS.md).

The specs are organized into seven families plus a system-architecture anchor (added 2026-05-04 as Stage 7 landed). New readers should start with [Pipeline core](#pipeline-core); architects should start with [System architecture](#system-architecture).

---

## Pipeline core

The four specs that define the core transformation: source documents → reviewed structured objects → user-facing workflow → compiled `$wosWorkflow`.

- [**`source-vault.md`**](source-vault.md) — sources, versions, sections, citations. Cross-document supersession via `Supersession` PolicyObjects. Lifecycle and supersession events. JSON-LD ingest path for canonical-source-publishing regulations. Multi-language source content. The system of record for primary input artifacts.
- [**`policy-object-model.md`**](policy-object-model.md) — the structured object families (source-and-authority, requirement, due-process, workflow-mapping bridge kinds, review-and-uncertainty). The intermediate representation between sources and the compiled artifact. ~30 kinds. **Bridge kinds** subsection enumerates the 7 mapping-family kinds. Deontic-composition section serializes via OASIS LegalRuleML. DataElement.sensitivity uses W3C DPV vocabulary. ProtectedCategory kind for equity authoring (composes parent advanced equity stream).
- [**`workflow-intent.md`**](workflow-intent.md) — the user-facing draft of the workflow (16 element kinds: phase / step / decision / review / notice / deadline / appeal / exception / hold / data-collection / evidence-request / system-check / AI-assistance / manual-override / completion-outcome / phase-end). The bridge from each kind to kernel constructs. Carries `wosVersionPin` and workflow-level `effectivenessRef`.
- [**`compiler-contract.md`**](compiler-contract.md) — the transformation contract: input shape, output shape, determinism guarantees, the three external gates (`schema-pass` / `lint-pass` / `conformance-pass`), failure modes, projection rules. The center-of-gravity spec. Includes `wosVersionPin` versioning, reproducibility/disaster-recovery contract, and parent `wos-event-types.md` (PLN-0384) composition.

## Foundation seams

The four specs that define cross-cutting foundational seams. Three are seam-only (contracts; deferred implementations); one is composable.

- [**`runtime-observation-seam.md`**](runtime-observation-seam.md) — **seam contract** for ingesting runtime case-trace observations (closes 3 dangling references in current spec set; Phase-4 implementation deferred). Wire format, ingest paths (subscription / poll / batch), trigger contract for cluster detection, replay contract, promotion contract.
- [**`effectiveness-and-applicability.md`**](effectiveness-and-applicability.md) — the **single composable Effectiveness object** (jurisdictions, temporal scope, appellate state, provisional flag, supersession links). SourceVersion / PolicyObject (where applicable) / Mapping reference by `effectivenessRef` — never copied; one canonical home. Composes parent `wos-delivery.schema.json#appliesWhen` FEL.
- [**`identity-and-attestation.md`**](identity-and-attestation.md) — **seam contract** for identity claims that authorize authoring actions. Composes parent **PLN-0381** (identity attestation stack ADR; promoted to P0 WOS-side commitment 2026-04-27). IdentitySubject, AttestationEnvelope, AuthorityGrant resolution, revocation semantics, signing-key boundary.
- [**`terminology-and-canonical-vocabulary.md`**](terminology-and-canonical-vocabulary.md) — CanonicalTerm registry resolving cross-workspace DataElement identity; W3C **DPV** vocabulary for sensitivity classification; plain-English projection layer (Sarah's "operator language" concern as a structural artifact). Composes parent `wos-ontology-alignment.schema.json` PROV-O / JSON-LD vocabularies.

## Bridge

The single spec that ties Studio objects to WOS concepts.

- [**`studio-to-wos-mapping.md`**](studio-to-wos-mapping.md) — the four mapping states (`mapsToWos` / `authoringOnly` / `requiresSpecExtension` / `unmappedButApproved`); target resolution into `wos-workflow.schema.json` JSON paths; ExtensionRecord proposals; six-seam attachment per ADR-0077. Slight WOS-side extension proposals queued (ApplicabilityScope/EffectivePeriod first-class, x-wos-studio formalization, DPV/canonicalTermRef on caseFile.fields, top-level wosVersionPin, governance.deonticConstraints LegalRuleML). **`decisionTable` LANDED parent-side as Kernel §4.5.1 (2026-05-01); now `mapsToWos`.**

## Trust

The two specs that make the produced artifact defensible.

- [**`authoring-provenance.md`**](authoring-provenance.md) — the append-only audit trail: AuthoringProvenanceRecord, origin classes (5: `source` / `approved-interpretation` / `local-practice` / `assumption` / `runtime-observed`), provenance-edge model, projection rules. **Cryptographic anchoring** via parent `custodyHook` (PLN-0385) four-field append wire surface. **AI extraction subtype** with full model lineage (modelId, modelVersion, promptTemplateRef, temperature, seed, humanApprover). **Audit event catalog** in `wos.authoring.*` namespace (composes parent PLN-0384). **Compaction policy** (immutable log + projection-only compaction). **PROV-O export** as first-class auditor-interop format.
- [**`readiness-validation.md`**](readiness-validation.md) — the readiness/lint engine: 6 tiers (S1–S6), rule registry, severity ladder, waiver model, publication-blocker contract. Cross-cutting rules for Effectiveness, AI provenance, version pinning, equity, accessibility, jurisdictional variation, identity, compliance, cryptographic chain integrity, terminology.

## Workspace + Tooling

The two specs covering workspace state and testable artifacts.

- [**`workspace.md`**](workspace.md) — the attachment-point entity for everything. Workspace, ReviewerRole registry, WorkspacePolicy, audit-log boundary. **AuthorityGrant** (RBAC authority-per-action). **Compliance metadata section** (SOC 2 / FedRAMP / StateRAMP / NIST 800-53). **Federation extensibility slot** (`x-federation`; deferred). **Key management section** (Studio holds no private keys). **Compaction policy gate** (workspace admins cannot compact the underlying log).
- [**`scenario-authoring.md`**](scenario-authoring.md) — Scenario data model, 16 scenario types (4 added in v3/v4: `equity-probe`, `accessibility-check`, `jurisdictional-variation`, `runtime-observation-replay`), expected-vs-observed comparison contract, conformance-trace correspondence with `wos-tooling.schema.json`.

## Lifecycle

The two specs that gate publication and manage post-publication evolution.

- [**`review-and-approval.md`**](review-and-approval.md) — ApprovalDecision, ApprovalPackage, `approved-with-conditions` semantics, multi-role gating, override-cannot-bypass-`block`, publication-gate contract. ReviewerRole registry lives in [`workspace.md`](workspace.md). **ApprovalPackage carries** `wosVersionPin`, ComplianceAttestations, IdentitySigningKeyRefs, custodyAnchorReceipt. **Key-rotation handling** for in-flight ApprovalDecisions.
- [**`change-impact.md`**](change-impact.md) — ChangeImpactReport with durable lifecycle (`produced → acknowledged → closed`), **seven trigger kinds** (source-version-change, policy-object-edit, mapping-update, runtime-observation-cluster, jurisdictional-supersession, wos-version-deprecation, compliance-attestation-expiry). Semantic diff between workflow versions, scenario regression contract.

## Integration

The single spec that defines binding-and-integration objects.

- [**`binding-and-integration.md`**](binding-and-integration.md) — five kinds: `ServiceBinding` (workflow step ↔ OpenAPI/Arazzo), `EventBinding` (workflow event ↔ kernel event with CloudEvents extension attrs), `PolicyEngineBinding` (workflow check ↔ OPA/Cedar/XACML), `DecisionTable` (multi-row extension to `DecisionRule` projecting to parent kernel `decisionTables[*]` + `DecisionTableGuard` per Kernel §4.5.1), `DMNImport` (one-way DMN→DecisionTable transpilation; **no DMN export** stands). **Scenario-as-contract-test** relationship. **Runtime-observation seam attachment hook** as specialized EventBinding.

## System architecture

The Stage 7 anchor that composes the 16 specs above into a deployable system.

- [**`reference-architecture.md`**](reference-architecture.md) — the **Studio-tier system-architecture spec** (Stage 7, landed 2026-05-04). Layer model, component model (cited against the 16 specs above), Studio-side port catalog (16 ports — disjoint from `crates/wos-server`'s port set), external-OSS-adapter seam catalog (7 seams: `KnowledgeMemoryAdapter`, `DataConnectorAdapter`, `MetadataCatalogAdapter`, `LineageAdapter`, `DataContractAdapter`, `QualityCheckAdapter`, `ProjectionTarget`), projection-target model (`ProjectionTarget` / `ExportSink` — WOS workflow + Formspec form co-equal first-class), canonical flows (10), trust/governance invariants, replay/rebuild contract. Six sibling ADRs decompose the load-bearing decisions ([`../../thoughts/adr/0086`](../../thoughts/adr/0086-studio-knowledge-platform-reference-architecture.md) … `0091`). Companion: [`stage-8-vertical-slice.md`](stage-8-vertical-slice.md).
- [**`stage-8-vertical-slice.md`**](stage-8-vertical-slice.md) — the **Stage 8 production vertical-slice plan** (planning artifact, not a normative spec). Width-one path through every layer of the reference architecture; reuses [`../examples/snap-redetermination-from-sources/`](../examples/snap-redetermination-from-sources/) as the corpus and reproducibility target.

---

## Cross-spec MUST tracking

Each spec carries `SA-MUST-<topic>-NNN` tracking IDs for normative contracts. IDs are stable; ID prefixes name the spec:

| Prefix | Spec |
|---|---|
| `SA-MUST-source-*` | source-vault |
| `SA-MUST-pom-*` | policy-object-model |
| `SA-MUST-prov-*` | authoring-provenance |
| `SA-MUST-map-*` | studio-to-wos-mapping |
| `SA-MUST-rv-*` | readiness-validation |
| `SA-MUST-scn-*` | scenario-authoring |
| `SA-MUST-ra-*` | review-and-approval |
| `SA-MUST-ci-*` | change-impact |
| `SA-MUST-ws-*` | workspace |
| `SA-MUST-wfi-*` | workflow-intent |
| `SA-MUST-cmp-*` | compiler-contract |
| `SA-MUST-bind-*` | binding-and-integration |
| `SA-MUST-rtos-*` | runtime-observation-seam |
| `SA-MUST-eff-*` | effectiveness-and-applicability |
| `SA-MUST-id-*` | identity-and-attestation |
| `SA-MUST-term-*` | terminology-and-canonical-vocabulary |
| `SA-MUST-arch-*` | reference-architecture |

MUSTs that cannot yet be enforced (because Stage-3 schemas, Stage-4 lint, or Stage-5 compiler don't exist) are flagged with `*(schema-pending)*`, `*(lint-pending)*`, `*(runtime-pending)*`, or `*(fixture-pending)*` per the CONVENTIONS.md gap-tracking convention.

## Composition with parent wos-spec

Studio composes (does not reinvent) the following parent-stack contracts:

| Parent contract | Used by Studio for |
|---|---|
| [`specs/kernel/custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md) (PLN-0385) | Cryptographic anchoring of authoring audit log |
| `wos-event-types.md` (PLN-0384, in flight) | Audit event-type taxonomy (`wos.authoring.*`, `wos.compiler.*`) |
| Identity attestation stack ADR (PLN-0381) | IdentitySubject + AttestationEnvelope shapes |
| [`specs/ai/ai-integration.md`](../../specs/ai/ai-integration.md) §3.4 | AI extraction provenance (model lineage) |
| [`specs/advanced/equity-config.md`](../../specs/advanced/equity-config.md) | ProtectedCategory + equity-probe scenarios |
| [`specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) + [`specs/governance/due-process-config.md`](../../specs/governance/due-process-config.md) | Effectiveness composition with `continuationOfServices` / `escalationPath` for appellate-state semantics |
| [`RELEASE-STREAMS.md`](../../RELEASE-STREAMS.md) + [`COMPATIBILITY-MATRIX.md`](../../COMPATIBILITY-MATRIX.md) | `wosVersionPin` claim string format |
| [`schemas/sidecars/wos-delivery.schema.json#appliesWhen`](../../schemas/sidecars/wos-delivery.schema.json) | Compiler-derived FEL `appliesWhen` from Effectiveness |
| [`schemas/sidecars/wos-ontology-alignment.schema.json`](../../schemas/sidecars/wos-ontology-alignment.schema.json) | PROV-O / JSON-LD export; canonical-vocab alignment |

## External standards (selective reuse)

| Standard | Adopted | Where |
|---|---|---|
| **W3C JSON-LD** | YES | `source-vault.md` ingest path |
| **W3C DPV (Data Privacy Vocabulary)** | YES | `policy-object-model.md` DataElement.sensitivity; `terminology-and-canonical-vocabulary.md` |
| **W3C PROV-O** | YES | `authoring-provenance.md` export format |
| **OASIS LegalRuleML** | YES | `policy-object-model.md` deontic composition serialization |
| **DMN one-way import** | YES | `binding-and-integration.md` `DMNImport` binding kind |
| **OpenAPI / Arazzo / CloudEvents** | YES | `binding-and-integration.md` (composes parent integration profile per `WOS-FEATURE-MATRIX.md` §12) |
| **OPA / Cedar / XACML** | YES | `binding-and-integration.md` PolicyEngineBinding |
| BPMN one-way import | DEFERRED | revisit when concrete agency demand surfaces |
| W3C SHACL | DEFERRED | revisit when JSON-LD ingest matures + an agency publishes shapes |
| RDF / Turtle export | DEFERRED | JSON-LD covers the export need |
| AsyncAPI | REJECTED (parent CLAUDE.md) | superseded by CloudEvents |
| DMN export | REJECTED (parent CLAUDE.md) | one-way import only |
| FEEL as authority language | REJECTED (parent CLAUDE.md) | FEL is the WOS authority; FEEL→FEL transpilation only at DMN-import boundary |
| SHACL as workflow authority | REJECTED (parent CLAUDE.md) | FEL stands; SHACL reserved for source-side validation only |

## What is not (yet) in this folder

- **JSON Schemas** (Stage 3) — the structural form of every Studio object. Landed 2026-05-01 at `studio/schemas/` (**15 schemas** total) per CM §6.1 schema composition strategy — was nominally ~14 in the original Stage-3 commit; the 15th (`wos-studio-mapping.schema.json`) landed in the audit-fix pass. Wave-1 review remediation (2026-05-02) tightened per-kind body enforcement, wired `wos-studio-common` lifecycle enums via `$ref`, and added 20 negative tests under `tests/schemas/test_studio_negative.py`. WOS is the canonical projection substrate per CM §5 (target-neutral authoring + risk-profile differentiation via embedded-block conditionality).
- **Lint engine** (Stage 4) — the implementation of the rule registry that fires the readiness findings these specs name.
- **Studio→WOS compiler** (Stage 5) — the implementation of `compiler-contract.md`.
- **Scenario simulator** (Stage 6) — the runtime that exercises Scenarios.
- **Reference architecture** (Stage 7) — LANDED 2026-05-04 at [`reference-architecture.md`](reference-architecture.md); the system-component anchor that composes the 16 specs above into a deployable architecture.
- **Vertical slice** (Stage 8) — pending; planning artifact at [`stage-8-vertical-slice.md`](stage-8-vertical-slice.md). The SNAP-redetermination example exists at [`../examples/snap-redetermination-from-sources/`](../examples/snap-redetermination-from-sources/) as the Stage 8 reproducibility target.

See [`../VISION.md`](../VISION.md) §17 (Implementation Roadmap) for the staging.

## What used to be here but is no longer

- `runtime-observation.md` (full spec) — replaced in v4 by `runtime-observation-seam.md` (seam contract only; Phase-4 implementation deferred).

## Companion-PRD reconciliation (2026-05-03)

A 2026-05-03 companion-PRD proposal on "Studio integration / binding / interoperability" was reviewed and **mostly reconciled into existing specs**. Of its nine "Required Studio Capability Modules":

- **Decision modeling** — already in [`policy-object-model.md`](policy-object-model.md) §"DecisionRule" + [`binding-and-integration.md`](binding-and-integration.md) §"DecisionTable" + DMNImport one-way import.
- **Service / API binding** — already in [`binding-and-integration.md`](binding-and-integration.md) §"ServiceBinding" (OpenAPI + Arazzo).
- **Event binding** — already in [`binding-and-integration.md`](binding-and-integration.md) §"EventBinding" (CloudEvents).
- **Policy / authority-check binding** — already in [`binding-and-integration.md`](binding-and-integration.md) §"PolicyEngineBinding" (OPA / Cedar / XACML).
- **Traceability graph validation** — already in [`readiness-validation.md`](readiness-validation.md) tier system + [`../STUDIO-LINT-MATRIX.md`](../STUDIO-LINT-MATRIX.md) (70 rules).
- **Provenance / audit export** — already in [`authoring-provenance.md`](authoring-provenance.md) §"PROV-O export" + ApprovalPackage in [`review-and-approval.md`](review-and-approval.md).
- **Runtime trace import** — already in [`runtime-observation-seam.md`](runtime-observation-seam.md) (Studio-native shape; not XES/OCEL).
- **Legacy process import (BPMN/CMMN/SCXML)** — DEFERRED per the table above.
- **Legal source ingestion** — JSON-LD + Akoma Ntoso ingest paths in [`source-vault.md`](source-vault.md) §"JSON-LD source ingest" + §"Akoma Ntoso ingest path" (latter added 2026-05-03 in E6.3); LegalRuleML deontic composition in [`policy-object-model.md`](policy-object-model.md).

Three items were genuinely net-new and landed as targeted edits:

- **Capabilities-first product rule** — [`../VISION.md`](../VISION.md) §"Capabilities-first product rule" (added 2026-05-03).
- **Binding Inspector** read-only aggregation surface — [`binding-and-integration.md`](binding-and-integration.md) §"Binding Inspector" (added 2026-05-03).
- **Akoma Ntoso ingest path** — [`source-vault.md`](source-vault.md) §"Akoma Ntoso ingest path" (added 2026-05-03).

The companion PRD's other content was redundant with the existing 16 specs; not authored as a parallel document to avoid two product narratives drifting against each other.

## Cross-references

- Product vision: [`../VISION.md`](../VISION.md).
- Concept model (entity definitions): [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md). **§5 WOS as canonical substrate** names the load-bearing principle that lets Studio's authoring vocabulary stay target-neutral while WOS scales down for low-risk workflows. **§6.1 Schema composition strategy** explains how Stage-3 reduces from ~33 schemas to ~10 by composition with parent WOS schemas.
- Folder entry-point: [`../README.md`](../README.md).
- Examples / vertical slice: [`../examples/`](../examples/).
- Repo conventions (three-section rubric): [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
- WOS schemas (compilation targets): [`../../schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json), [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json).
