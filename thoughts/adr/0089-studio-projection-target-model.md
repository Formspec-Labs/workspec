# ADR-0089: Studio projection-target model — `ProjectionTarget` / `ExportSink`

**Status:** Proposed 2026-05-04
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md) — generalizes the Phase-9 export-bundle emitter into a uniform `ProjectionTarget` port.

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0090 (sibling — publish/export boundary; defines signing of emitted artifacts)
- ADR 0073 (parent — IntakeHandoff Formspec ↔ WOS contract; Studio reuses for Form Projection)
- [`crates/wos-formspec-binding`](../../crates/wos-formspec-binding) (existing binding adapter)

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
