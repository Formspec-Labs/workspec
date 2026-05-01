# WOS Studio (Authoring) — Specs Index

Twelve W3C-style specs that together define how WOS Studio (Authoring) transforms source documents into a `$wosWorkflow` file. Each spec follows the three-section rubric (Normative Contract / Composition / Conformance) per [`../../CONVENTIONS.md`](../../CONVENTIONS.md).

The specs are organized into six families, in approximate read order. New readers should start with [Pipeline core](#pipeline-core).

---

## Pipeline core

The four specs that define the core transformation: source documents → reviewed structured objects → user-facing workflow → compiled `$wosWorkflow`.

- [**`source-vault.md`**](source-vault.md) — sources, versions, sections, citations. Cross-document supersession via `Supersession` PolicyObjects. Lifecycle and supersession events. The system of record for primary input artifacts.
- [**`policy-object-model.md`**](policy-object-model.md) — the structured object families (source-and-authority, requirement, due-process, workflow-mapping bridge kinds, review-and-uncertainty). The intermediate representation between sources and the compiled artifact. ~30 kinds; the **Bridge kinds** subsection enumerates the 7 mapping-family kinds that produce kernel constructs.
- [**`workflow-intent.md`**](workflow-intent.md) — the user-facing draft of the workflow (16 element kinds: phase / step / decision / review / notice / deadline / appeal / exception / hold / data-collection / evidence-request / system-check / AI-assistance / manual-override / completion-outcome / phase-end). The bridge from each kind to kernel constructs.
- [**`compiler-contract.md`**](compiler-contract.md) — the transformation contract: input shape, output shape, determinism guarantees, the three external gates (`schema-pass` / `lint-pass` / `conformance-pass`), failure modes, projection rules. The center-of-gravity spec.

## Bridge

The single spec that ties Studio objects to WOS concepts.

- [**`studio-to-wos-mapping.md`**](studio-to-wos-mapping.md) — the four mapping states (`mapsToWos` / `authoringOnly` / `requiresSpecExtension` / `unmappedButApproved`); target resolution into `wos-workflow.schema.json` JSON paths; ExtensionRecord proposals; six-seam attachment per ADR-0077.

## Trust

The three specs that make the produced artifact defensible.

- [**`authoring-provenance.md`**](authoring-provenance.md) — the append-only audit trail: AuthoringProvenanceRecord, origin classes (5: `source` / `approved-interpretation` / `local-practice` / `assumption` / `runtime-observed`), provenance-edge model, projection rules into the published artifact.
- [**`readiness-validation.md`**](readiness-validation.md) — the readiness/lint engine: 6 tiers (S1–S6), rule registry, severity ladder, waiver model, publication-blocker contract.
- [**`workspace.md`**](workspace.md) — the attachment-point entity for everything. Workspace, ReviewerRole registry, WorkspacePolicy, audit-log boundary, identity-and-permission surface.

## Tooling

The single spec that defines testable scenario artifacts.

- [**`scenario-authoring.md`**](scenario-authoring.md) — Scenario data model, 12 scenario types (happy-path / adverse-determination / appeal-filed / agent-fallback / ...), expected-vs-observed comparison contract, conformance-trace correspondence with `wos-tooling.schema.json`.

## Lifecycle

The two specs that gate publication and manage post-publication evolution.

- [**`review-and-approval.md`**](review-and-approval.md) — ApprovalDecision, ApprovalPackage, `approved-with-conditions` semantics, multi-role gating, override-cannot-bypass-`block`, publication-gate contract. ReviewerRole registry lives in [`workspace.md`](workspace.md).
- [**`change-impact.md`**](change-impact.md) — ChangeImpactReport with durable lifecycle (`produced → acknowledged → closed`), four trigger kinds (source-version-change, policy-object-edit, mapping-update, runtime-observation-cluster), semantic diff between workflow versions, scenario regression contract.

## Integration

The single spec that defines binding-and-integration objects.

- [**`binding-and-integration.md`**](binding-and-integration.md) — four kinds: `ServiceBinding` (workflow step ↔ OpenAPI/Arazzo), `EventBinding` (workflow event ↔ kernel event with CloudEvents extension attrs), `PolicyEngineBinding` (workflow check ↔ OPA/Cedar/XACML), `DecisionTable` (multi-row extension to `DecisionRule` compiling to chained-FEL-guard sequence). No DMN export.

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

MUSTs that cannot yet be enforced (because Stage-3 schemas, Stage-4 lint, or Stage-5 compiler don't exist) are flagged with `*(schema-pending)*`, `*(lint-pending)*`, `*(runtime-pending)*`, or `*(fixture-pending)*` per the CONVENTIONS.md gap-tracking convention.

## What is not (yet) in this folder

- **JSON Schemas** (Stage 3) — the structural form of every Studio object.
- **Lint engine** (Stage 4) — the implementation of the rule registry that fires the readiness findings these specs name.
- **Studio→WOS compiler** (Stage 5) — the implementation of `compiler-contract.md`.
- **Scenario simulator** (Stage 6) — the runtime that exercises Scenarios.
- **Reference architecture** (Stage 7) — the system-component documents.
- **Vertical slice** (Stage 8) — the FAFSA ISIR worked example.

See [`../VISION.md`](../VISION.md) §17 (Implementation Roadmap) for the staging.

## What used to be here but is no longer

- `runtime-observation.md` — Phase-4 future-track spec, removed in favor of writing it when Phase 4 begins. No spec in the current set materially depended on it.

## Cross-references

- Product vision: [`../VISION.md`](../VISION.md).
- Concept model (entity definitions): [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md).
- Folder entry-point: [`../README.md`](../README.md).
- Repo conventions (three-section rubric): [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
- WOS schemas (compilation targets): [`../../schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json), [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json).
