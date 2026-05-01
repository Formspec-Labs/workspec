# WOS Studio (Authoring) — Specs Index

**Sixteen** W3C-style specs that together define how WOS Studio (Authoring) transforms source documents into a `$wosWorkflow` file. Each spec follows the three-section rubric (Normative Contract / Composition / Conformance) per [`../../CONVENTIONS.md`](../../CONVENTIONS.md).

The specs are organized into seven families, in approximate read order. New readers should start with [Pipeline core](#pipeline-core).

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

- [**`studio-to-wos-mapping.md`**](studio-to-wos-mapping.md) — the four mapping states (`mapsToWos` / `authoringOnly` / `requiresSpecExtension` / `unmappedButApproved`); target resolution into `wos-workflow.schema.json` JSON paths; ExtensionRecord proposals; six-seam attachment per ADR-0077. Slight WOS-side extension proposals queued (decisionTable row coverage, ApplicabilityScope/EffectivePeriod first-class, x-wos-studio formalization, DPV/canonicalTermRef on caseFile.fields, top-level wosVersionPin).

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

- [**`binding-and-integration.md`**](binding-and-integration.md) — five kinds: `ServiceBinding` (workflow step ↔ OpenAPI/Arazzo), `EventBinding` (workflow event ↔ kernel event with CloudEvents extension attrs), `PolicyEngineBinding` (workflow check ↔ OPA/Cedar/XACML), `DecisionTable` (multi-row extension to `DecisionRule` compiling to chained-FEL-guard sequence), `DMNImport` (one-way DMN→DecisionTable transpilation; **no DMN export** stands). **Scenario-as-contract-test** relationship. **Runtime-observation seam attachment hook** as specialized EventBinding.

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

- **JSON Schemas** (Stage 3) — the structural form of every Studio object. Per CM §5.1 schema composition strategy: ~10 schemas (not ~33) by composition with parent WOS schemas.
- **Lint engine** (Stage 4) — the implementation of the rule registry that fires the readiness findings these specs name.
- **Studio→WOS compiler** (Stage 5) — the implementation of `compiler-contract.md`.
- **Scenario simulator** (Stage 6) — the runtime that exercises Scenarios.
- **Reference architecture** (Stage 7) — the system-component documents.
- **Vertical slice** (Stage 8) — the FAFSA ISIR worked example. **Started in v4** at [`../examples/`](../examples/).

See [`../VISION.md`](../VISION.md) §17 (Implementation Roadmap) for the staging.

## What used to be here but is no longer

- `runtime-observation.md` (full spec) — replaced in v4 by `runtime-observation-seam.md` (seam contract only; Phase-4 implementation deferred).

## Cross-references

- Product vision: [`../VISION.md`](../VISION.md).
- Concept model (entity definitions): [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md). **§5.1 Schema composition strategy** explains how Stage-3 reduces from ~33 schemas to ~10 by composition with parent WOS schemas.
- Folder entry-point: [`../README.md`](../README.md).
- Examples / vertical slice: [`../examples/`](../examples/).
- Repo conventions (three-section rubric): [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
- WOS schemas (compilation targets): [`../../schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json), [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json).
