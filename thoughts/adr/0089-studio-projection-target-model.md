# ADR-0089: Studio projection-target model — `ProjectionTarget` / `ExportSink`

**Status:** Proposed 2026-05-04 · Amended 2026-05-04 (validation)
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md) — generalizes the Phase-9 export-bundle emitter into a uniform `ProjectionTarget` port. Also amends [`studio/specs/policy-object-model.md`](../../studio/specs/policy-object-model.md) (DecisionModel unification + DataRequirement family) and [`studio/specs/binding-and-integration.md`](../../studio/specs/binding-and-integration.md) (DecisionBinding within-tier unification).

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0090 (sibling — publish/export boundary; defines signing of emitted artifacts)
- ADR 0073 (parent — IntakeHandoff Formspec ↔ WOS contract; Studio reuses for Form Projection)
- [`crates/wos-formspec-binding`](../../crates/wos-formspec-binding) (existing binding adapter)
- Parent [`specs/core/spec.md`](../../formspec/specs/core/spec.md) AD-01 "Schema is data, not code" (validated 2026-05-04 — Formspec contract anticipates external Definition emission)

---

## Amendment 2026-05-04 — Formspec source-emit, DecisionModel unify, Requirement family

Three §2 decisions tightened after parent-spec validation
(`spec-expert` + `wos-expert` 2026-05-04). The amendments
strengthen the architectural posture while staying inside what
parent specs already permit.

### §2.4 — Formspec ownership boundary: emit source JSON

**Original (now superseded):** "Stage 8 default is a thin
adapter that wraps Formspec's existing compiler. A Studio-side
intermediate representation is admitted only if integration
friction proves substantial."

**Amended:** **Studio emits Formspec source JSON.** The Form
Projection writes a `formspec-form.json` artifact that is
byte-shape-identical to a hand-authored Formspec Definition.
Formspec compiler treats Studio output exactly like author-
emitted input. **No Rust-API coupling between products.**

**Rationale.** Validated against
[`schemas/definition.schema.json:3-5, 56-65, 682`](../../formspec/schemas/definition.schema.json)
and [`specs/core/spec.md:111`](../../formspec/specs/core/spec.md)
AD-01. The Formspec Definition has no "authoring origin" field;
`formspec-studio-core` produces the same shape any external tool
would. Calculated values, computed defaults, and round-trip
preservation of `x-` extensions are all part of Formspec's
existing contract. The "thin wrapper" alternative would have
introduced Rust-API coupling Formspec actively rejects.

This also opens the door for the future maximalist case — when
Formspec's authoring chat starts consuming Studio's reviewed
knowledge graph, the boundary stays JSON-document-shaped on both
the projection (Studio→Formspec) and the query (Formspec chat
→ Studio's `KnowledgeQueryService`, per ADR 0091 amendment) sides.

### §2.7 — DecisionModel within-tier unification (NEW)

Studio originally modeled decisions across THREE kinds:
- `DecisionRule` (PolicyObject; knowledge-tier rule with citations)
- `DecisionTable` (binding; multi-row form projecting to kernel `decisionTables[*]`)
- `DMNImport` (binding; one-way DMN→DecisionTable transpilation)

**Amendment:** collapse to TWO kinds at the right tiers:

- **Knowledge tier:** unified **`DecisionModel`** PolicyObject
  kind. Subsumes the prior `DecisionRule` + the structured form
  of `DMNImport`. Carries citations, effectivity, semantics.
  Rule-form vs table-form is a slot, not a separate kind.
- **Binding tier:** unified **`DecisionBinding`**. Subsumes the
  prior `DecisionTable` (multi-row form) and projects to kernel
  `decisionTables[*]` + `DecisionTableGuard` per Kernel §4.5.1.
- **DMN one-pass importer:** `DMNImport` becomes a one-way
  DMN→DecisionModel transpiler at the import boundary; the
  Studio-internal artifact is a knowledge-tier `DecisionModel`,
  not a parallel kind. Parent "DMN one-way import only"
  commitment (Kernel §4.5.1.5) preserved — the constraint binds
  the *output*, not the *intermediate Studio kind*.

**Rationale.** Validated against Kernel §4.5.1 and
`wos-workflow.schema.json:326–1181` (`wos-expert` 2026-05-04).
Kernel projection target is agnostic to whether one or three
Studio kinds project to it. AI/governance specs reference
`DecisionRule` only via lifecycle-guard projection; no agent or
oversight rule pins a specific Studio kind name.

**Cost.** ~5–10 Studio lint rules consolidate
(`POM-LINT-009/010/011`, `BIND-LINT-072/073`, `DMN-LINT-001`);
~40 conformance fixtures rewrite. Spec changes scope:
`policy-object-model.md` + `binding-and-integration.md` rename
section, kernel projection target unchanged. **Avoided forever:**
the "wait, is this a DecisionRule or a DecisionTable?" PR
question.

### §2.8 — Requirement family: DataRequirement first-class (NEW)

**Amendment:** introduce a **Requirement family** at the
PolicyObject tier:
- `EvidenceRequirement` (existing) — pre-condition for documents,
  attestations, certifications.
- `DataRequirement` (NEW) — pre-condition for structured data
  fields the case must collect before a workflow gate (e.g.,
  `household_income`, `citizenship_status`, `address`).
- `AccessRequirement` (reserved) — future; identity-proofing
  pre-conditions.
- `IdentityRequirement` (reserved) — future; subject-continuity
  pre-conditions.

**Satisfier seam (load-bearing).** When a Studio
`DataRequirement` says "household_income must be collected before
this step," the satisfier — typically a Formspec form field — is
identified via:

- Formspec **`semanticType`** (per
  `schemas/definition.schema.json:682`, registry-mediated concept
  identifier), OR
- Formspec **`x-wos-satisfies`** extension on the Definition
  root, items, or binds (sanctioned `x-` extension surface;
  preserved on round-trip per `specs/core/spec.md:4402`).

Field `key` is a *Definition-local* identifier (per
`schemas/definition.schema.json:429`); raw key-name equality
collapses on multi-form / multi-jurisdiction stacks (state SNAP
"income" ≠ federal SNAP "income"). **Raw key matching is NOT
admitted as a satisfier seam.**

`IntakeHandoff` carries no satisfaction semantics (validated
2026-05-04). Studio computes satisfaction post-handoff by walking
`definitionRef` against its own DataRequirement registry.

**Rationale.** Validated against `definition.schema.json:182`
(Bind `required` is *form-internal dynamic requiredness*, not a
workflow-tier pre-condition gate) and the absence of a Formspec-
side "data collection requirement" manifest. The satisfier seam
already exists (`semanticType` + `x-` extensions); Studio just
USES it. Risk admitted: if only DataRequirement materializes,
the family naming was rhetorical overhead — mitigated by
ratifying the family but shipping only DataRequirement in v1.

---

## 1. Context

Studio v1 emits multiple operational artifacts: WOS workflows,
Formspec forms, decision artifacts, integration bindings, scenario
suites, approval packages, signed export bundles. v1.1+ adds data
contracts. The maximalist v2+ surface adds reports, public
knowledge bases, and runtime-observation feedback.

Pre-Stage-7 framing treated `wos-studio-compiler` as the only
emitter — implicitly making "WOS workflow" the only first-class
projection. That framing is rejected: it foreclosed the
multi-projection identity Studio actually carries.

This ADR ratifies a **uniform projection-target port**
(`ProjectionTarget`, equivalently `ExportSink`) that every emitter
implements. WOS workflow and Formspec form are co-equal first-
class targets in v1.

## 2. Decision

### 2.1 Uniform `ProjectionTarget` port

Each projection-target implementation adheres to:

```text
trait ProjectionTarget {
    type Intent;          // target-specific intent input (e.g., WorkflowIntent, FormProjectionIntent)
    type Artifact;        // target-specific artifact output (e.g., WosWorkflow, FormspecForm)
    type ValidationReport; // target-specific readiness/lint/conformance report

    fn project(
        &self,
        knowledge: &ValidatedKnowledgeModel,
        intent: &Self::Intent,
    ) -> Result<Self::Artifact, ProjectionError>;

    fn validate(&self, artifact: &Self::Artifact) -> Self::ValidationReport;
}
```

Stage 7 ships the trait stub. Stage 8+ ships concrete impls.

### 2.2 v1 projection-target catalog

| Target | v1 status | Implementation |
|---|---|---|
| WOS workflow (`$wosWorkflow`) | first-class | `wos-studio-compiler` (existing 9-phase pipeline) |
| Formspec form | first-class | New Stage 8 adapter wrapping `wos-formspec-binding` + Formspec packages |
| Decision artifact | spec only | Pending DecisionModel resolution |
| Integration binding package | spec only | Per `studio/specs/binding-and-integration.md` |
| Data contract | spec only | Per `DataContractAdapter` (ADR 0091) |
| Scenario suite | exists | `wos-studio-scenario` |
| ApprovalPackage | specified | Per `studio/specs/review-and-approval.md` |
| ExportBundle | exists | `wos-studio-compiler` Phase 9 |

### 2.3 WOS and Formspec are co-equal

Stage 7 explicitly **rejects** the framing "WOS is the only
output." Both projection targets:
- Consume the same reviewed knowledge model.
- Produce signed, validated artifacts.
- Land in the same ApprovalPackage / ExportBundle.
- Are exercised by the Stage 8 vertical slice.

### 2.4 Formspec ownership boundary

**Formspec owns form definition**. Studio does NOT introduce a
`FormIntent` first-class object. The Form Projection adapter:
- Reads Studio's reviewed knowledge model (DataElement,
  EvidenceRequirement, DecisionRule with citations and
  effectivity).
- Emits a Formspec form artifact via Formspec's compiler inputs.
- Validates against Formspec's form-validation pipeline.

The Stage 8 default (per
[`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md))
is a thin adapter that wraps Formspec's existing compiler. A
Studio-side intermediate representation is admitted only if
integration friction proves substantial.

### 2.5 Future projection targets attach the same way

Reports, briefing memos, public knowledge bases, and runtime-
observation feedback all attach as new `ProjectionTarget`
implementations behind the same port. No architectural reshape
needed.

### 2.6 Phase-9 export bundle generalization

Today's [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md)
Phase 9 emits an export bundle for the workflow projection. Stage
7 generalizes:
- The bundle composition logic stays in `wos-studio-compiler` as
  the WOS workflow projection's `ProjectionTarget` impl.
- A separate Stage 8 component (Export Bundle Builder) composes
  multiple `ProjectionTarget` outputs into a multi-target signed
  ExportBundle (per ADR 0090).

## 3. Rejected Alternatives

- **WOS workflow as the only first-class projection.** Rejected;
  forecloses Formspec / decision / data-contract / future targets.
- **Formspec as a sub-projection of WOS workflow.** Rejected;
  forms are co-equal authored artifacts, not derivatives.
- **Studio-side `FormIntent` object.** Rejected; overlaps Formspec
  authoring surface.
- **Phase-9 as the only emitter (no port).** Rejected; the
  multi-target ExportBundle requires uniform composition.
- **Decision projection blocks Stage 7.** Rejected; Stage 7
  ratifies the port; DecisionModel unification (open question)
  blocks only the decision adapter, not the architecture.

## 4. Consequences

### Positive

- Multi-projection identity is encoded in the trait surface.
- Adding a new projection target is an adapter implementation,
  not an architectural change.
- WOS / Formspec parity is structural.

### Negative

- The Form Projection adapter requires a Formspec-side
  authoring-tools integration (Stage 8 deliverable).
- Open: Form-projection adapter shape (thin wrapper vs Studio-side
  intermediate). Stage 8 default chosen; revisited if friction.
- Open: DecisionModel unification (DecisionRule + DecisionTable +
  DMNImport). Decision projection deferred until resolved.

### Neutral

- The existing `wos-studio-compiler` becomes one
  `ProjectionTarget` impl among many; no internal reshape.

## 5. Conformance

- `SA-MUST-arch-040..041` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Stage 8 deliverables 9, 10, 11 in
  [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md).
- Trait stub: Stage 7 contract code.
