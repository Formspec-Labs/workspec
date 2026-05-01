# Studio Spec: Studio-to-WOS Mapping

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.11 StudioToWosMapping, §3 Mapping states, §5 WOS as canonical substrate, §6 WOS concept cross-reference.
**PRD anchor:** [`../VISION.md`](../VISION.md) §6 (Studio-to-WOS Mapping Contract), §16 Phase-1 Epic 1.3.
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md).

## Scope

The Studio-to-WOS Mapping Contract is the **load-bearing invariant** of the entire product (PRD §5 Principle 8): every approved Studio object MUST declare its mapping state, and the mapping is the bridge from Studio's authoring vocabulary to formal WOS semantics. Without the contract, Studio would risk becoming a parallel semantics system competing with WOS (PRD §15 Risk #1).

This spec defines:

- the StudioToWosMapping record shape;
- the four mapping states (`mapsToWos` / `authoringOnly` / `requiresSpecExtension` / `unmappedButApproved`) and the precedence rule;
- the **target-resolution model**: how a `mapsToWos` mapping points into specific WOS concepts and `wos-workflow.schema.json` JSON paths;
- the ExtensionRecord that accompanies a `requiresSpecExtension` mapping;
- the unmapped-rationale that accompanies an `unmappedButApproved` mapping;
- the lifecycle of mapping records;
- the normative contract for mapping declaration, target validity, extension proposals, and the cross-cutting requirement that workflow-bearing PolicyObjects map to WOS before publication.

This spec is the **reviewable contract** that lets technical reviewers (PRD §3 "Technical implementers", §12 "Studio-to-WOS Mapping" user stories) inspect how Studio output relates to WOS without needing to read the entire workspace.

### Authoring vocabulary vs. substrate (the four mapping states encode this split)

Studio's authoring vocabulary is intentionally broader than the WOS substrate (see CM §5 "WOS as canonical substrate"). The four mapping states are the structural mechanism that lets authors capture rights-impacting authoring discipline — *who said what, when, with what authority, what we assumed, what was superseded* — without forcing the WOS substrate to grow concepts that do not belong in it. The states partition every PolicyObject into one of four dispositions:

- **`mapsToWos`** — authoring concept and substrate concept align; the PolicyObject's content projects into a specific `wos-workflow.schema.json` JSON path. This is the common case for rights-impacting workflows (NoticeRequirement, AppealRight, ActorMapping, DecisionRule, EvidenceRequirement, …).
- **`authoringOnly`** — authoring concept exists for review, citation, and reproducibility, but the substrate does not need to evaluate it at runtime. Examples: PolicySource, AuthorityRank, Assumption, Conflict, ReviewerResolution, Supersession, ApplicabilityScope, EffectivePeriod. These project as **provenance and rationale** (via ApprovalPackage citations and `AuthoringProvenanceRecord`s), not as schema content.
- **`requiresSpecExtension`** — authoring concept identifies a real gap in the substrate; the ExtensionRecord queues it as a candidate WOS-side enhancement at one of the six canonical kernel seams. Stays workspace-local until the substrate ratifies the extension.
- **`unmappedButApproved`** — escape hatch with required rationale. Expected to be **rare and noisy**; reviewers are explicitly required to justify each one.

This split is what makes "WOS is the canonical substrate" non-restrictive. The substrate stays focused on what it must evaluate at runtime; the workspace carries everything authoring needs without bleeding into the artifact. The precedence rule (`mapsToWos > authoringOnly > requiresSpecExtension > unmappedButApproved`) keeps the substrate clean by preferring projection over workspace-only carriage whenever both are possible.

## Out of scope

- The Studio→WOS compiler implementation (Stage 5).
- The kernel/governance/AI/advanced/sidecars schema details (live under [`../../specs/`](../../specs/) and [`../../schemas/`](../../schemas/)).
- The runtime evaluation of mapped artifacts (lives in `wos-runtime` and adapter crates).

## Terminology

- **Mapping record** — a `StudioToWosMapping` instance bound to one PolicyObject.
- **Mapping state** — one of `mapsToWos` | `authoringOnly` | `requiresSpecExtension` | `unmappedButApproved`.
- **Target** — the WOS concept (named in a spec) and the JSON path inside `wos-workflow.schema.json` (or a sidecar schema) that the Studio object compiles to.
- **Extension Record** — a candidate spec-extension proposal authored when a Studio concept lacks a WOS counterpart.
- **Compile-time check** — a check the Studio→WOS compiler performs before emitting a `$wosWorkflow` artifact.
- **Unmapped rationale** — the documented reason a `unmappedButApproved` object is acceptable.

## Data model

### `StudioToWosMapping` (CM §1.11, extended)

```text
StudioToWosMapping {
  id, subjectRef (PolicyObject id), state,
  targets[] (when state = mapsToWos),
  extensionRecordRef (when state = requiresSpecExtension),
  unmappedRationale (when state = unmappedButApproved),
  approvedBy, approvedAt, lifecycleState,
  workspaceId, version
}

Target {
  wosConceptId,         // dotted name, e.g. "governance.dueProcess.notice"
  wosSpecPath,          // path to spec doc, e.g. "../../specs/governance/workflow-governance.md"
  wosJsonPath,          // JSONPath into wos-workflow.schema.json, e.g. "$.governance.notices[*]"
  fieldBindings[],      // per-field map from Studio body fields → WOS schema fields
  mappingNotes          // free-text reviewer notes
}

ExtensionRecord {
  id, candidateConceptName, motivatingPolicyObjectRefs[],
  proposedSeam (one of the six canonical kernel seams; see CLAUDE.md),
  proposedSemantics (prose), proposedSchemaSketch (informal),
  evidenceCitations[], lifecycleState, approvedBy?, approvedAt?
}
```

### Mapping states (recap of CM §3)

- **`mapsToWos`** — `targets[]` is required and non-empty.
- **`authoringOnly`** — workspace-only; no WOS emission. Default for source-and-authority and review-and-uncertainty PolicyObject families ([`policy-object-model.md`](policy-object-model.md)).
- **`requiresSpecExtension`** — `extensionRecordRef` is required.
- **`unmappedButApproved`** — `unmappedRationale` is required; expected to be **rare and noisy** (PRD §6).

### Precedence (recap of CM §3.5)

```
mapsToWos > authoringOnly > requiresSpecExtension > unmappedButApproved
```

If a PolicyObject could plausibly receive multiple states, the higher-precedence state is the correct choice. Reviewers MAY override (the override is recorded in [`authoring-provenance.md`](authoring-provenance.md) `originClass`-aware events) but only with a documented rationale.

## Lifecycle

A mapping record's lifecycle:

```text
draft → reviewed → approved → published → superseded
```

- `draft`: the mapping has been authored but not reviewed.
- `reviewed`: a technical reviewer has examined targets / extension record / rationale.
- `approved`: ApprovalDecision recorded.
- `published`: the host PolicyObject is part of a PublishedWorkflowPackage.
- `superseded`: a later version of the mapping (or a state transition, e.g., `requiresSpecExtension → mapsToWos` once the extension lands) replaces this one.

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `draft` | `reviewed` | technical reviewer examined |
| `reviewed` | `approved` | ApprovalDecision recorded |
| `reviewed` | `draft` | reviewer requested edits |
| `approved` | `published` | host workflow published |
| `approved` | `superseded` | new mapping version (state change or target edit) approved |
| `published` | `superseded` | host workflow superseded by later version |

Mapping state changes (`mapsToWos ↔ requiresSpecExtension ↔ authoringOnly ↔ unmappedButApproved`) are themselves treated as new mapping versions, **not** edits. A mapping that moves from `requiresSpecExtension` to `mapsToWos` (because the upstream WOS extension landed) creates a new mapping record `vN+1`; the prior record transitions to `superseded`. Provenance preserves both.

## Normative Contract

### Mapping declaration

- **`SA-MUST-map-001`** — Every PolicyObject MUST have exactly one StudioToWosMapping record once its `lifecycleState` reaches `approved`. Approval without a mapping declaration MUST be rejected. *(schema-pending: required record; lint-pending: tier S3 readiness rule.)*
- **`SA-MUST-map-002`** — `mappingState` MUST be exactly one of the four values; multiple-state declarations MUST be rejected. *(schema-pending: enum.)*
- **`SA-MUST-map-003`** — When `mappingState = mapsToWos`, the record MUST carry a non-empty `targets[]`. When `mappingState = requiresSpecExtension`, it MUST carry an `extensionRecordRef`. When `mappingState = unmappedButApproved`, it MUST carry an `unmappedRationale` of at least 50 characters (no one-line "TBD"-style rationales). When `mappingState = authoringOnly`, it MUST NOT carry any of the three. *(schema-pending: state-dependent required fields.)*
- **`SA-MUST-map-004`** — A WorkflowIntent MUST NOT advance from `mapped → validationReady` while any approved PolicyObject it references has `mappingState = unmappedButApproved` without a workflow-level reviewer override. *(lint-pending: tier S3 readiness rule.)*
- **`SA-MUST-map-005`** — A WorkflowIntent MUST NOT advance from `validationReady → scenarioTested` while any approved PolicyObject it references has `mappingState = requiresSpecExtension` whose corresponding ExtensionRecord is in `lifecycleState = open` (i.e., not yet shipped in WOS). *(lint-pending: tier S3 readiness rule; cross-cutting with the upstream WOS spec extension cycle.)*

### Target validity

- **`SA-MUST-map-010`** — Every `Target.wosConceptId` MUST resolve to an actual concept named in one of the WOS specs ([`../../specs/kernel/`](../../specs/kernel/), [`../../specs/governance/`](../../specs/governance/), [`../../specs/ai/`](../../specs/ai/), [`../../specs/advanced/`](../../specs/advanced/), [`../../specs/profiles/`](../../specs/profiles/), [`../../specs/sidecars/`](../../specs/sidecars/), [`../../specs/assurance/`](../../specs/assurance/), [`../../specs/registry/`](../../specs/registry/)). Unknown concepts MUST be flagged as tier-S3 ValidationFindings. *(lint-pending: cross-spec name resolution.)*
- **`SA-MUST-map-011`** — Every `Target.wosJsonPath` MUST be a syntactically valid JSONPath that resolves to a node in `wos-workflow.schema.json` (or its referenced sidecars). *(lint-pending; runtime-pending: schema-traversal validator.)*
- **`SA-MUST-map-012`** — `fieldBindings[]` MUST cover every load-bearing Studio body field that affects the compiled WOS output. A field-binding gap (a Studio body field that affects compilation but has no binding) MUST surface as a tier-S3 ValidationFinding. *(lint-pending; cross-cutting with the compiler in Stage 5.)*
- **`SA-SHOULD-map-013`** — When a Studio object maps to multiple targets (e.g., a NoticeRequirement that maps both to a `governance.dueProcess.notice` entry and to a `wos-delivery` notification template), the mapping SHOULD list all targets explicitly so reviewers see the full surface.
- **`SA-MUST-map-014`** — A `Target` whose `wosJsonPath` includes path segments under `x-` extension keys MUST also carry a reference to an ExtensionRecord (or to an `extensions` registry entry per [`../../specs/registry/extension-registry.md`](../../specs/registry/extension-registry.md)). Bare `x-` targets without an extension audit trail MUST be rejected. *(lint-pending; cross-cutting with the registry spec.)*

### Extension records

- **`SA-MUST-map-020`** — An ExtensionRecord MUST identify exactly one of the **six canonical kernel seams** (per [`../../CLAUDE.md`](../../CLAUDE.md): `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions` / `x-` keys) as the proposed attachment point. *(schema-pending: enum constraint; cross-cutting with [ADR 0077](../../thoughts/adr/0077-named-seams-invariant.md).)*
- **`SA-MUST-map-021`** — An ExtensionRecord MUST carry at least one `motivatingPolicyObjectRef` — the Studio concept(s) that lack a current WOS counterpart. Speculative extensions ("we might need this someday") MUST be rejected at extension creation. *(lint-pending; PRD §5 Principle 9: AI proposes; humans approve.)*
- **`SA-MUST-map-022`** — `proposedSemantics` MUST be substantive prose (at least 200 characters); `proposedSchemaSketch` MUST be a non-empty informal sketch. Empty or one-line entries MUST be rejected. *(schema-pending: minimum-length constraint.)*
- **`SA-MUST-map-023`** — When an ExtensionRecord is `approved` and the corresponding upstream WOS spec extension lands, the implementation MUST: (a) mark the ExtensionRecord `lifecycleState = shipped`; (b) prompt reviewers to migrate the host PolicyObject's mapping from `requiresSpecExtension` to `mapsToWos` with the now-existing target. The migration is reviewer-driven, not automatic. *(runtime-pending; cross-cutting with the upstream extension PR cycle.)*
- **`SA-SHOULD-map-024`** — Repeated ExtensionRecords on the same `proposedSeam + candidateConceptName` across multiple workspaces SHOULD surface as a tier-S3 portfolio-level finding to product owners (PRD §16 Phase-1 Epic 1.3 user story: "identify candidate WOS extensions from repeated mapping gaps").

### Slight WOS-side extension proposals (queued ExtensionRecord candidates)

The following are **candidate slight extensions to parent WOS schemas** identified during v3/v4 design. Each is logged as a portfolio-level ExtensionRecord (per `SA-SHOULD-map-024`) for parent-stack consideration. Until ratified parent-side, the corresponding Studio concepts use either `requiresSpecExtension` mapping state with `x-` extension envelopes (per ADR-0077) OR the compiler emits derived FEL into existing parent fields.

| Studio concept | Proposed parent-side change | Why slight | Until ratified |
|---|---|---|---|
| `ApplicabilityScope` PolicyObject | Add `wos-workflow.schema.json#applicabilityScope` (workflow-level jurisdictional scope) | Lets workflow-level Effectiveness map to first-class parent field rather than `x-wos-studio.applicability` envelope | Compiler emits derived FEL `appliesWhen` on existing `governance.notices[*]` / `lifecycle.transitions[*]` per Effectiveness; workflow-level scope lives in `x-wos-studio.applicability` extension key (ADR-0077 `x-` keys) |
| `EffectivePeriod` PolicyObject | Add `wos-workflow.schema.json#effectivePeriod` (workflow-level temporal scope) | Mirrors above for temporal | Same — compiler-derived FEL + `x-wos-studio` envelope |
| `DecisionTable` (rows) | Add `wos-tooling.scenarios[*].decisionTable` row-coverage shape | Lets a DecisionTable's row #N be cited by Scenario S as covered; today scenarios cite guard expressions, losing row-granularity traceability | Compiler emits chained FEL guards; row-coverage tracked Studio-side only; reviewer sees the row→scenario link via Studio UI but not in the published artifact |
| `wos-workflow.x-wos-studio` envelope | Formalize a reserved `x-wos-studio` extension key on `$wosWorkflow` (per ADR-0077 `x-` patternProperties; the proposal FORMALIZES the slot in the schema doc) | Studio's compact provenance + citation manifest project here today via `x-wos-studio` extension envelope; formalizing the slot lets downstream consumers fetch without ApprovalPackage retrieval | `x-wos-studio` envelope is already emitted via ADR-0077; downstream consumers parse from there |
| `governance.deonticConstraints[*]` LegalRuleML | (Existing field; verify it accepts OASIS LegalRuleML JSON-LD vocabulary terms `lrml:Obligation` / `lrml:Permission` / `lrml:Prohibition` / `lrml:Right` / `lrml:defeasible`) | Compose with parent existing field; if parent field shape doesn't accept these terms, propose extension | Studio's compiler emits LegalRuleML-shaped content per `policy-object-model.md` `SA-MUST-pom-050`; if parent rejects, fall back to `x-wos-studio.deontic` extension envelope |
| `caseFile.fields[*].canonicalTermRef` and `sensitivity` (DPV) | Promote `x-canonical-term` and `x-dpv-sensitivity` extension keys to first-class fields on `wos-workflow.schema.json#caseFile.fields[*]` | DataElement annotation per `terminology-and-canonical-vocabulary.md`; today via `x-` keys per ADR-0077 | `x-canonical-term` and `x-dpv-sensitivity` extension keys per ADR-0077 |
| Workflow-level `wosVersionPin` | Promote pin to top-level `wosVersionPin` field on `$wosWorkflow` (today implicit via envelope-version + parent claims-map per RELEASE-STREAMS.md) | Reduces ambiguity: structural field lets external verifiers parse the claim without prose | Per `compiler-contract.md` `SA-MUST-cmp-050`, the pin is in the compile manifest; downstream consumers parse manifest |

These extensions follow the `requiresSpecExtension` workflow if/when reviewer-approved; the `motivatingPolicyObjectRef` is the cross-spec coupling cited above. Workspace-specific gaps (project-level extensions to a particular regulation's terms) remain workspace-scoped per `SA-MUST-map-021` "no speculative extensions."

### Authoring-only mappings

- **`SA-MUST-map-030`** — When `mappingState = authoringOnly`, the implementation MUST NOT emit the host PolicyObject into the compiled `$wosWorkflow` artifact's content. *(runtime-pending: compiler enforcement.)*
- **`SA-MUST-map-031`** — Authoring-only PolicyObjects MAY still contribute to the published artifact's authoring-provenance projection (per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-030`); this is a *projection*, not the object itself. *(runtime-pending.)*
- **`SA-SHOULD-map-032`** — A workspace SHOULD render `authoringOnly` PolicyObjects in a visually distinct way (PRD §12 user story: "authoring-only objects to be explicitly marked so they are not mistaken for runtime behavior"). This is a UX SHOULD; the spec requires the *state*, not the rendering.

### Unmapped-but-approved mappings

- **`SA-MUST-map-040`** — `mappingState = unmappedButApproved` MUST be approved by a reviewer in a role authorized to take this action (workspace policy decides; default: workflow owner role). *(runtime-pending: role-policy.)*
- **`SA-MUST-map-041`** — Each `unmappedButApproved` mapping MUST surface as a tier-S3 `noisy-unmapped` ValidationFinding for the duration of the host PolicyObject's lifetime — the finding is informational (severity `warn`), not blocking, but it is **always visible** to ensure publication-time review. *(lint-pending.)*
- **`SA-MUST-map-042`** — A workspace's published artifacts MUST list every `unmappedButApproved` mapping in the published artifact's release notes (PRD §9.9 approval package). *(runtime-pending: compiler emission of release notes.)*
- **`SA-SHOULD-map-043`** — Repeated `unmappedButApproved` patterns SHOULD trigger a workspace-level review of whether the unmapped mappings should instead be `requiresSpecExtension` (a recurring unmapped-but-approved pattern often indicates a missing WOS concept).

### Cross-spec coupling

- **`SA-MUST-map-050`** — Mapping state changes MUST emit AuthoringProvenanceRecords with `eventKind = mapped` or `mappingStateAssigned` (per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-001`). *(runtime-pending.)*
- **`SA-MUST-map-051`** — Source supersession that demotes a host PolicyObject (per [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-022`) MUST also demote its mapping back to `draft` so the mapping is re-reviewed against the new citation context. *(runtime-pending.)*

## Composition

### Attachment point

Mapping records attach 1:1 to PolicyObjects within a workspace. They are **not** workflow-level — every PolicyObject carries its own mapping, regardless of how many workflows reference it. (Cross-workflow reuse, deferred per [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6, would not change this rule: each workflow's references would still resolve through the per-PolicyObject mapping.)

ExtensionRecords attach at the **workspace** layer (a workspace may accumulate multiple ExtensionRecords pointing at the same proposed seam from different motivating PolicyObjects).

The Studio→WOS compiler (Stage 5) is the **sole consumer** of mapping records that affects WOS output. Other consumers (Validation Center, Workflow Health Dashboard, audit log) read mappings without writing to them.

### Precedence

Mapping-state precedence is normatively fixed (CM §3.5, recapped above). Within `mapsToWos`, target precedence is:

1. **Closest WOS concept** wins. A NoticeRequirement maps to `governance.dueProcess.notice`, not to a generic `extensions.notification`.
2. **Embedded blocks beat sidecars** when both could apply. A signature-flow Studio object maps to the `signature` embedded block, not to `wos-delivery` notifications.
3. **Sidecars beat extension seams** when both could apply. A delivery-environment configuration (calendar, template) maps to `wos-delivery`, not to `x-` keys.
4. **Named seams beat anonymous `x-` keys.** When a kernel seam (`actorExtension`, `contractHook`, etc.) covers the case, prefer the seam over an `x-` extension.

### Conflict handling

Two PolicyObjects mapping to the **same** WOS JSON path (with different field bindings or content) MUST surface as a tier-S3 ValidationFinding `mapping-target-collision`. The implementation MUST NOT silently merge or pick one. Reviewers resolve by re-mapping one of them (often by recognizing that the two PolicyObjects should be merged at the policy-object layer instead).

A PolicyObject whose mapping changes state (e.g., from `requiresSpecExtension` to `mapsToWos` because the upstream extension landed) is **not** a conflict — it is a normal state transition (see Lifecycle).

### Versioning / migration

- Adding a new WOS concept (and a corresponding mapping target) is **non-breaking** at the Studio side.
- Removing or renaming a WOS concept that Studio mappings target is **breaking**: every workspace's mappings must be re-reviewed; the implementation MUST surface tier-S3 findings on every dangling target.
- Upgrading a WOS schema version that Studio compiles against is **breaking** in the same way; a migration playbook is part of the upgrade process.
- Adding a new mapping state to the four-state enum is **schema-breaking** and would require renegotiating PRD §6's contract.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- One mapping record per PolicyObject (`SA-MUST-map-001`).
- State enum (`SA-MUST-map-002`).
- State-dependent required fields (`SA-MUST-map-003`).
- Target shape: `wosConceptId`, `wosSpecPath`, `wosJsonPath`, `fieldBindings[]` schema.
- ExtensionRecord shape: `proposedSeam` enum, minimum-length `proposedSemantics`.

### Lint rules (Stage 4)

Tier-S3 ("Mapping readiness") rules planned:

- `MAP-LINT-001` — every approved PolicyObject has a mapping (SA-MUST-map-001).
- `MAP-LINT-002` — `mapsToWos` carries valid targets (SA-MUST-map-010 + 011).
- `MAP-LINT-003` — `requiresSpecExtension` carries a valid ExtensionRecord (SA-MUST-map-003 + 020 + 022).
- `MAP-LINT-004` — `unmappedButApproved` carries substantive rationale and surfaces as a noisy finding (SA-MUST-map-041).
- `MAP-LINT-005` — no two PolicyObjects collide on the same target (`mapping-target-collision`).
- `MAP-LINT-006` — workflow-bearing PolicyObjects are not `unmappedButApproved` without override (SA-MUST-map-004).
- `MAP-LINT-007` — workflow-bearing PolicyObjects do not have an `open` ExtensionRecord blocking advance (SA-MUST-map-005).
- `MAP-LINT-008` — `x-` targets carry an extension-registry entry (SA-MUST-map-014).

### Runtime conformance fixtures (Stage 4–5)

- Mapping state transitions emit provenance.
- Source supersession demotes mappings.
- Compiler refuses to emit when mapping declarations are missing or invalid.
- Compact projection of provenance includes mapping state and targets.

### Current limitations

- The full target-resolution check (`SA-MUST-map-011`) requires the compiler to traverse `wos-workflow.schema.json` and resolve JSONPath expressions. This is a Stage-5 capability; until then, the lint rule is best-effort string match.
- The closure of the kernel seams enum (`SA-MUST-map-020`) is set in [ADR 0077](../../thoughts/adr/0077-named-seams-invariant.md); any future named seam additions cascade here.

## WOS mappings (the table)

The full per-PolicyObject-kind mapping table is in [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §5. This section enumerates the **WOS targets** by spec path and JSON path so reviewers can audit the surface in one place.

| Studio family | Default state | Primary WOS spec | Primary JSON path |
|---|---|---|---|
| Source-and-authority (PolicySource, AuthorityRank, ApplicabilityScope, EffectivePeriod, Supersession) | `authoringOnly` | — (workspace metadata; citation excerpts project compactly via [`authoring-provenance.md`](authoring-provenance.md)) | — |
| Requirement (Requirement, Obligation, Permission, Prohibition, Condition, ExceptionRule) | `mapsToWos` | [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md), [`../../specs/governance/assertion-library.md`](../../specs/governance/assertion-library.md) | `$.governance.policyParameters`, `$.lifecycle.transitions[*].guard`, deontic constraint declarations |
| DecisionRule | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §guards, [`../../specs/governance/assertion-library.md`](../../specs/governance/assertion-library.md) | `$.lifecycle.transitions[*].guard` (FEL expression or `RuleReference`) |
| Deadline / TimerMapping | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §timers, [`../../specs/governance/policy-parameters.md`](../../specs/governance/policy-parameters.md) | `$.lifecycle.timers`, `$.governance.policyParameters` |
| EvidenceRequirement | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §caseFile, [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) | `$.caseFile.<path>`, `$.governance.validationPipelines` |
| DataElement / CaseFileMapping | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §caseFile | `$.caseFile.<path>` |
| Outcome | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §lifecycle | `$.lifecycle.states[?(@.terminal)]` |
| NoticeRequirement | `mapsToWos` | [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md), [`../../specs/governance/due-process-config.md`](../../specs/governance/due-process-config.md) | `$.governance.notices[*]`; rendering template via [`../../schemas/sidecars/wos-delivery.schema.json`](../../schemas/sidecars/wos-delivery.schema.json) |
| AppealRight | `mapsToWos` | [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) | `$.governance.appeals[*]` |
| ExplanationRequirement / CounterfactualRequirement | `mapsToWos` | [`../../specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md), [`../../specs/ai/ai-integration.md`](../../specs/ai/ai-integration.md) | `$.governance.explanations`, `$.aiOversight.explanations` |
| ContinuationOfServicesRequirement | `mapsToWos` | [`../../specs/governance/due-process-config.md`](../../specs/governance/due-process-config.md) | `$.governance.dueProcess.continuation` |
| WorkflowStepMapping / LifecycleTagMapping / TransitionMapping | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) | `$.lifecycle.states[*]`, `$.lifecycle.transitions[*]` |
| ActorMapping (human) | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §actors | `$.actors[*]` |
| ActorMapping (agent) + AI-Use | `mapsToWos` | [`../../specs/ai/ai-integration.md`](../../specs/ai/ai-integration.md), [`../../specs/ai/agent-config.md`](../../specs/ai/agent-config.md) | `$.actors[*]` (with `type=agent`), `$.agents[*]`, `$.aiOversight` |
| Flexible Case Phase / DCR-equivalent | `mapsToWos` | [`../../specs/advanced/advanced-governance.md`](../../specs/advanced/advanced-governance.md) | `$.advanced.constraintZones[*]` |
| TaskMapping | `mapsToWos` | [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md) §tasks | `$.lifecycle.tasks[*]` |
| ScenarioMapping | `mapsToWos` (via tooling schema) | conformance fixtures + [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json) | `$.scenarios[*]` (in tooling schema) |
| Review-and-uncertainty (Assumption, OpenQuestion, Conflict, ReviewerResolution, ApprovalDecision) | `authoringOnly` (mostly) | — (compact projection via [`authoring-provenance.md`](authoring-provenance.md)) | provenance config |

`requiresSpecExtension` and `unmappedButApproved` are **per-object** declarations — they do not have a default; they are the result of reviewer judgment when the cells above don't fit.

## Examples

### Example 1: A Notice with no current WOS counterpart for "translation parity"

A workspace's NoticeRequirement carries a body field `translationParityRequired: true` — meaning every notice MUST be sent in every language the applicant has indicated comprehension of, with each translation receiving its own delivery confirmation. WOS's `governance.notices[*]` covers `language[]`, but does not cover per-language delivery-confirmation parity.

Reviewer authors:

1. A NoticeRequirement PolicyObject with the parity field. State: `approved`.
2. A StudioToWosMapping with `state = mapsToWos`, target `$.governance.notices[*]`. **Plus** an additional Target with `wosJsonPath = $.x-translation-parity` and a reference to an ExtensionRecord (per `SA-MUST-map-014`).
3. The ExtensionRecord proposes attachment at the `extensions` seam, with proposed semantics: "When a notice carries `x-translation-parity = true`, the runtime MUST emit a separate delivery-confirmation event per recorded language."
4. Until the upstream extension lands, the workflow can advance to `validationReady` (because the `mapsToWos` target covers the structural notice) but **cannot** advance to `scenarioTested` (per `SA-MUST-map-005`) because the ExtensionRecord is still `open`.
5. After the upstream extension PR merges and ships in a WOS version, the ExtensionRecord transitions to `shipped`, the mapping is migrated to a single-target `$.governance.notices[*].x-translation-parity` mapsToWos, and the workflow advances.

### Example 2: An authoring-only PolicySource

A workspace records the `Federal Register Notice 2026-12345 — SNAP Final Rule` as a PolicySource PolicyObject. This object is reference material; it never appears in the compiled `$wosWorkflow`.

1. PolicyObject `kind = PolicySource`, body carries SourceDocument reference, legal nature, issuing authority.
2. StudioToWosMapping `state = authoringOnly`. No `targets[]`, no `extensionRecordRef`, no `unmappedRationale`.
3. Citations from this PolicySource attached to other PolicyObjects (Requirement, NoticeRequirement, etc.) project compactly into the published artifact's authoring-provenance per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-032`.
4. The PolicySource itself never reaches the WOS artifact.

### Example 3: An unmapped-but-approved local-policy override

A workspace's program manager attests that, in this state agency's operational context, denial notices must include the case worker's direct phone number — a practice not required by federal SNAP rules but required by a state administrative directive that the workspace has not yet uploaded as a SourceDocument.

Reviewer paths:

- **Preferred path:** upload the state directive as a SourceDocument, extract the requirement, promote to a Requirement PolicyObject, map to `governance.notices[*].content`. The mapping is then `mapsToWos`.
- **Fallback path** (when the directive is unavailable for upload, e.g., access restriction): create the Requirement PolicyObject backed by an Assumption (origin `local-practice`); declare a `unmappedButApproved` mapping with rationale referencing the directive name and a TODO to re-map when source becomes available. Tier-S3 noisy finding fires for the lifetime of the object; release notes list the unmapped item.

The fallback is **discouraged** — `SA-SHOULD-map-043` flags repeated `unmappedButApproved` patterns as a workspace-level review trigger.

## Open issues

- **Target field-binding granularity.** `fieldBindings[]` could be a free-form key map or a structured DSL. Stage 3 schema work decides; today the spec only says "covers every load-bearing field" and leaves shape unspecified.
- **Target collision detection.** Two `mapsToWos` records targeting the same JSON path is the easy case. Two records targeting *overlapping* paths (e.g., `$.governance.notices[0]` and `$.governance.notices[*]`) is harder; the lint rule's exact predicate is unsettled.
- **ExtensionRecord lifecycle.** The `open → reviewed → approved → shipped → adopted` chain is sketched here but not pinned. Whether `adopted` (every host workspace migrated) is a state or just an implication of `shipped` is unsettled.
- **Cross-workspace mapping reuse.** A PolicyObject that's reused across workspaces would need its mapping to project consistently. Deferred per [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.11, §3, §5.
- PRD: [`../VISION.md`](../VISION.md) §6, §16 Phase-1 Epic 1.3, §12 user stories.
- Upstream: [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md).
- Downstream: [`readiness-validation.md`](readiness-validation.md), [`change-impact.md`](change-impact.md), [`review-and-approval.md`](review-and-approval.md).
- WOS specs: [`../../specs/kernel/spec.md`](../../specs/kernel/spec.md), [`../../specs/governance/`](../../specs/governance/), [`../../specs/ai/`](../../specs/ai/), [`../../specs/advanced/`](../../specs/advanced/), [`../../specs/sidecars/README.md`](../../specs/sidecars/README.md), [`../../specs/registry/extension-registry.md`](../../specs/registry/extension-registry.md).
- ADRs: [ADR 0076 (product-tier consolidation)](../../thoughts/adr/0076-product-tier-consolidation.md), [ADR 0077 (named seams invariant)](../../thoughts/adr/0077-named-seams-invariant.md).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
