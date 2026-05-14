# ADR 0095 — Governance Pipeline Execution Semantics Are DAG-Shaped

**Status:** Proposed
**Date:** 2026-05-14
**Scope:** WOS Layer 1 (Workflow Governance) — `Pipeline` execution semantics; structural admission of stage dependencies; runtime executor placement; backward compatibility for the current ordered-array authoring form; composition with the kernel statechart and the Signature Profile DAG.

**Related:**
[ADR 0082 (WOS kernel semantic projection and import)](./0082-wos-kernel-semantic-projection-and-import.md) D-1 *Kernel Truth* (kernel topology is statechart, not graph);
[ADR 0078 (kernel `foreach` topology)](../../../thoughts/adr/0078-foreach-topology.md) (pattern for adding a topology primitive without breaking the kernel statechart invariant);
[ADR 0084 (WOS durable runtime adapter)](./0084-wos-restate-durable-runtime-adapter.md) D-4 (`RuntimeOps` / `DurableRuntime` placement; where a new executor would live);
[ADR 0093 (a case is its Trellis ledger)](./0093-case-is-its-trellis-ledger.md) §2.4 (workflow event submission and ledger append discipline; provenance event family pipelines emit into);
[ADR 0075 (rejection register)](../../../thoughts/adr/0075-rejection-register.md) I-1 *Statechart is deliberate*, row 5 *governed discretionary work*;
[ADR 0070 (stack failure and compensation)](../../../thoughts/adr/0070-stack-failure-and-compensation.md) D-5 (no runtime saga; this ADR does not reopen that);
[`work-spec/specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) §5 (data validation pipelines), §5.4 (assertion gate types), §8.1 (rejection policy);
[`work-spec/specs/advanced/advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5 (multi-step sessions with `dependsOn` — precedent);
[`work-spec/crates/wos-core/src/model/governance.rs`](../../crates/wos-core/src/model/governance.rs):218-260 (`Pipeline` and `PipelineStage` typed model — current shape, no dependency field);
[`work-spec/crates/wos-core/src/event_handler.rs`](../../crates/wos-core/src/event_handler.rs):300-344 (current "executor": a provenance translator that fires `PipelineStageCompleted` on `validationPassed` events from external surfaces);
[`work-spec/crates/wos-runtime/src/runtime/signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):381-424, 1624-1664 (Signature Profile precedent: `SigningStep.depends_on: Vec<String>` plus DAG-respecting completion guard);
[`work-spec/schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) `$defs/Pipeline` / `$defs/PipelineStage`;
[`work-spec/TODO.md`](../../TODO.md) `Rejected #5 DAG Processing Model` (lifecycle-level rejection — this ADR is consistent; see §6);
sibling concurrent ADRs (numbers pending allocation): *transition-action DAG*, *form-intake validation*, *statechart-and-DAG complementarity meta-ADR*;
[`/Users/mikewolfd/Work/formspec-stack/TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md) (wos-server as governance overlay upstream of Trellis writes — the refactor that motivates an executor existing at all).

---

## 1. Context

### 1.1 Pipelines are declarative authoring artifacts with no runtime executor

`wos-core::model::governance::Pipeline.stages: Vec<PipelineStage>` ([`governance.rs:218-260`](../../crates/wos-core/src/model/governance.rs)) is typed as an ordered `Vec` with no `depends_on`, no parallel-region marker, no join shape. The spec ([`workflow-governance.md`](../../specs/governance/workflow-governance.md) §5.1) calls a pipeline "a staged processing chain where each stage validates data against contracts with assertion gates between stages." §5.2 names `stages` "Ordered processing stages."

The runtime does **not** walk this structure. [`event_handler.rs`](../../crates/wos-core/src/event_handler.rs):300-344 reveals what the runtime actually does today: when an event of type `validationPassed` arrives, it emits a `PipelineStageCompleted` provenance record; when `validationRejected` arrives, it emits a `PipelineRejection` record; when an event carries `stageResults` it derives a `PipelineRiskProfile` by weakest-link. No stage iteration. No assertion evaluation. No graph or sequence walk. External surfaces — currently outside `wos-core` and `wos-runtime` — perform the validation work and the runtime narrates the outcome into the provenance stream. Pipeline-the-data-structure and pipeline-the-execution are decoupled at the seam: a pipeline declared in a workflow is a declarative authoring artifact; what happens when validation runs is determined elsewhere.

This decoupling has been operationally fine because pipelines have had no executor inside `wos-runtime` to constrain. It becomes load-bearing the moment one appears.

### 1.2 The refactor adds an executor

[`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md) commits wos-server to be a "governance overlay upstream of Trellis writes." Operationally that means wos-server admits or rejects writes against governance policy *before* the Trellis ledger sees them. A workflow that declares a data validation pipeline on `contractHook` (Governance §5.6) needs a process that executes the pipeline: invokes assertion gates, applies `rejectionPolicy` to failures, threads the verdict back into the workflow's lifecycle so the kernel statechart fires the right transition. That process is a pipeline executor. The refactor implies one. This ADR fixes its topology.

### 1.3 Pipeline stages are mostly independent

The assertion-gate taxonomy ([`workflow-governance.md`](../../specs/governance/workflow-governance.md) §5.4) is dominated by gates whose dependency structure on neighbors is *zero*: `source-grounded` checks that extracted values appear in the source document; `arithmetic` recomputes a derived value and compares it; `range` does a bounds check; `format` runs a parser; `temporal` orders timestamps. Each operates on the input data independently. Only `consistency` and `cross-document` reference prior-stage output — `consistency` by declared `referenceStage` ([`governance.rs`](../../crates/wos-core/src/model/governance.rs):293-294), and `cross-document` because the validation joins multiple documents.

The ordered-array model encodes total ordering where the data structure carries no causal dependency. An author who writes `stages: [sourceGrounded, arithmetic, range, format, crossDocument]` has not said "arithmetic depends on sourceGrounded" — they have said "I will type these in some order." A runtime that walks this array in document order is choosing one consistent serialization of a topology the author did not declare. The runtime can choose differently and the result is identical for every pair of stages with no real dependency.

### 1.4 DAG precedents already exist in the spec

Two sibling surfaces in WOS already model step dependencies as `dependsOn`-style arrays:

- **AI multi-step sessions** ([`advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5.1, line 186): *"A multi-step session is a bounded sequence of steps forming a DAG, with defined checkpoints where human review may occur."* §5.3 names the property `dependsOn: array of string — Step identifiers that must complete before this step.*
- **Signature Profile signing flow** ([`signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):381-424): `SigningStep.depends_on: Vec<String>`, enforced at runtime by [`ensure_step_can_complete`](../../crates/wos-runtime/src/runtime/signature.rs):1624-1664 — a step blocks when any dependency is incomplete.

The Signature Profile case is the closer analogue: it ships a runtime that walks step dependencies, blocks selected-but-unsatisfied steps, and aggregates completion. It composes with `SigningFlowType::Sequential` for authors who do want strict document order — exactly the backward-compatibility shape pipelines need.

### 1.5 The rejected DAG row is about a different surface

[`work-spec/TODO.md`](../../TODO.md):814 — `Rejected #5: DAG Processing Model. Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected.` — rejects DAG *as a lifecycle model*, i.e., replacing the kernel statechart with a flat DAG. This ADR explicitly does not propose that. The kernel statechart remains canonical per ADR 0082 D-1 and ADR 0075 I-1; the Signature Profile DAG runs *inside* a signature transition; the multi-step session DAG runs *inside* an agent action; this ADR's pipeline DAG runs *inside* a workflow transition's contract evaluation. Lifecycle topology and sub-layer execution topology are distinct categorical surfaces. §6 of this ADR explains the categorical separation in detail.

### 1.6 Decision posture

Pre-release window, greenfield discipline per [`work-spec/CLAUDE.md`](../../CLAUDE.md) and the [`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md) operating assumptions: no compatibility shim is required, but the current ordered-array authoring form has live fixture coverage and prose, and the DAG additions can be backward compatible without cost. Fixing this *now*, before an executor lands and rebakes a serialization convention into wos-server, costs one optional field, one schema additive change, and the executor's own implementation budget. Fixing it after the executor ships costs all of that plus a renormalization migration.

---

## 2. Decision

When governance pipeline execution semantics are realized — that is, when a runtime component (in `wos-runtime` or in the wos-server governance overlay per [`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md)) walks `Pipeline.stages` to drive validation — those semantics are DAG-shaped. Pipeline stages carry an optional `dependsOn: Vec<String>` of stage ids; the executor walks the DAG topologically; stages with disjoint dependency closures MAY execute in parallel; failures roll up per `RejectionPolicy`; the current ordered-array form is preserved as a degenerate DAG.

Pipelines remain pure authoring artifacts where no executor exists. The model is moot in those deployments and the new field is optional. This ADR is the structural admission decision; it does not require the executor to ship in the same change set.

### D-1. `PipelineStage.dependsOn` is the dependency primitive

`$defs/PipelineStage` in [`work-spec/schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) gains:

| Field | Type | Required | Description |
|---|---|---|---|
| `dependsOn` | array of string | OPTIONAL | Stage identifiers within the same pipeline that MUST complete (verdict reached) before this stage may begin. Default `[]`. Each entry MUST resolve to a sibling `PipelineStage.id` in the same `Pipeline.stages`. Self-references and cycles are a lint and runtime rejection. |

The naming follows the existing WOS precedent: camelCase `dependsOn` matches multi-step session step ([`advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5.3, line 204), Rust idiomatic `depends_on` matches [`SigningStep.depends_on`](../../crates/wos-runtime/src/runtime/signature.rs):420. The Rust typed model in [`governance.rs`](../../crates/wos-core/src/model/governance.rs):231-260 adds `#[serde(default)] pub depends_on: Vec<String>`, serializing as `dependsOn`.

The field is intra-pipeline. A stage MUST NOT declare a dependency on a stage in a different pipeline; cross-pipeline composition is workflow-transition-level, governed by kernel lifecycle, not by the pipeline structure. This keeps the DAG bounded to one pipeline's authoring envelope.

### D-2. Topological execution with parallelism

A pipeline executor MUST:

1. Validate the DAG is well-formed at load time: every `dependsOn` entry resolves to a sibling stage; no cycles; no self-reference. Validation failure is a load-time document rejection, mirroring assertion library resolution rules ([`workflow-governance.md`](../../specs/governance/workflow-governance.md) §9.4).
2. Walk the DAG in topological order. A stage becomes *ready* when every stage in its `dependsOn` closure has reached a terminal verdict (pass, fail, or rejection-policy-mediated outcome — see D-4).
3. Independent stages — stages whose `dependsOn` closures are disjoint — MAY execute concurrently. The executor is permitted, not required, to parallelize. A purely sequential executor remains conformant; parallelism is an operational choice, not a normative obligation, exactly as in the kernel's `foreach` `concurrency` posture ([ADR 0078](../../../thoughts/adr/0078-foreach-topology.md) D-5).
4. Stage-internal evaluation order remains deterministic per assertion (Governance §5.4 gate semantics, FEL evaluation per Kernel §15.5). The non-determinism is *across* independent stages; *within* a stage, evaluation matches today's behavior.

A pipeline that reaches a terminal verdict (all required stages passed, or `RejectionPolicy` consumed) emits the existing provenance shape: `PipelineStageCompleted` per stage that reached a verdict, `PipelineRiskProfile` weakest-link summary, and `PipelineRejection` if applicable ([`event_handler.rs`](../../crates/wos-core/src/event_handler.rs):300-344 shape preserved; only the *who* of emission changes — from external surfaces narrating to a runtime executor emitting directly).

### D-3. Backward compatibility: absent `dependsOn` falls back to document order

When every stage in a pipeline has empty (or absent) `dependsOn`, the executor MUST treat the pipeline as a strict linear sequence in document order. This is the current operational behavior and preserves the prose contract "`stages` — Ordered processing stages" ([`workflow-governance.md`](../../specs/governance/workflow-governance.md) §5.2) for existing workflows.

Mechanically: an executor MAY implement this as "if `dependsOn` is empty, treat the prior stage's id as an implicit dependency," producing a chain DAG; or as a separate fallback path that walks `stages` linearly. Both are equivalent; the choice is implementation detail. The normative requirement is that the verdict trace for a no-`dependsOn` pipeline today matches the verdict trace under this ADR.

This rule is one-directional: an author MAY mix stages with explicit `dependsOn` and stages without. Stages without `dependsOn` in a mixed pipeline still default to document-order dependence, which means: a stage with empty `dependsOn` that appears after a stage with explicit `dependsOn` depends implicitly on the *previous stage in document order*, not on the explicit-DAG stage's predecessors. Authors who want explicit independence from prior stages declare `dependsOn: []` *explicitly with the empty array surfaced in the document*. (Schema constraint: the implicit-document-order rule applies only when `dependsOn` is structurally absent in the document. An explicitly-serialized empty array `dependsOn: []` means "no dependencies, may run as soon as the pipeline starts.")

This is the same distinction the schema already encodes for optional fields via `default` versus an explicit document presence; it is enforced as a lint rule, not a schema constraint, because JSON Schema cannot distinguish "absent" from "present with default value." Lint rule `WOS-GOV-PIPELINE-DEP-001` rejects ambiguity at authoring time when a mixed-document is detected.

### D-4. `RejectionPolicy` join semantics

[`RejectionPolicy`](../../crates/wos-core/src/model/governance.rs):314-322 already lists four behaviors: `retryWithCorrections`, `escalateToSupervisor`, `holdPendingData`, `failWithExplanation`. Under DAG execution, a stage with multiple parents may observe multiple rejection outcomes from its `dependsOn` closure before its own gate fires. The join rule:

| Parent verdict combination | Joined effect at the child stage |
|---|---|
| All parents pass | Child stage proceeds with the union of upstream stage outputs as input. |
| Any parent rejects with `failWithExplanation` | Child stage is skipped; the rejection propagates to the pipeline's terminal verdict. The pipeline emits `PipelineRejection` with the failing parent's structured explanation. |
| Any parent rejects with `escalateToSupervisor` | Pipeline pauses at the child stage; supervisor escalation fires per the parent's policy. Child does not execute until supervisor disposition. |
| Any parent rejects with `holdPendingData` | Child stage is held; pipeline suspends pending external data resolution per the parent's policy. |
| Any parent rejects with `retryWithCorrections` | The retry is parent-scoped: the parent stage re-runs upon corrections. Child stage observes the parent's *next* terminal verdict; until then, child remains in `ready: false` state. |

Stage-level `rejectionPolicy` (current `PipelineStage.rejection_policy: Option<RejectionPolicy>` at [`governance.rs`](../../crates/wos-core/src/model/governance.rs):251) governs the stage's *own* rejection. The join rule above applies to *received* rejections from parents. A stage MAY declare a join override via a new optional field `joinPolicy` (deferred — not in this ADR's scope; see §5.4).

The strongest-policy precedence ordering is: `failWithExplanation` > `escalateToSupervisor` > `holdPendingData` > `retryWithCorrections`. When two parents reject with different policies, the strongest applies. This ordering is normative and mirrors the kernel's escalation taxonomy implicit in [Governance §8.1](../../specs/governance/workflow-governance.md).

### D-5. Pipelines run inside transitions; do not replace the kernel statechart

This ADR's DAG operates strictly below the kernel lifecycle topology. The kernel statechart fires a transition; the transition's `contractHook` ([Kernel §10.2](../../specs/kernel/spec.md)) invokes a pipeline; the pipeline executes as a DAG; the pipeline's verdict (`validationPassed`, `validationRejected`, or a `RejectionPolicy`-mediated continuation) feeds back to the statechart as an event that drives the next transition.

Kernel lifecycle topology remains the canonical authoring and processing form per [ADR 0082](./0082-wos-kernel-semantic-projection-and-import.md) D-1 *Kernel Truth* and [ADR 0075](../../../thoughts/adr/0075-rejection-register.md) I-1 *Statechart is deliberate*. The DAG is a sub-layer execution primitive, not a replacement lifecycle. The deterministic-replay invariant (Kernel §4.2) holds at the pipeline boundary: given the same pipeline document, the same input case file, and the same agent of evaluation, the same verdict is produced. Parallelism inside the DAG does not break this — independent gates evaluate independent predicates; their composition is order-insensitive.

A pipeline DAG MUST NOT escape its transition. Pipeline stages MUST NOT enqueue workflow events, mutate case state outside the contract-bound output binding, or invoke kernel-lifecycle transitions directly. The contract-validation pipeline is a pure validator with structured side-channels (`RejectionPolicy`, provenance emission). This separation is what allows the kernel statechart to remain canonical even as governance gains DAG-shaped sub-execution.

### D-6. Executor placement

This ADR does not mandate which crate hosts the executor. Two viable placements:

1. **`wos-runtime`.** The executor lives next to the in-memory durable runtime; pipelines execute as part of `enqueue_event` / `drain_once` when a transition carries a `contractHook`-bound pipeline. Conformance fixtures pass against the in-memory adapter the same way they do today for `foreach` and signature flows.
2. **wos-server governance overlay.** Per [`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md), wos-server is the governance overlay upstream of Trellis writes; the executor runs there, gating Trellis appends on pipeline verdicts.

Both are consistent with [ADR 0084](./0084-wos-restate-durable-runtime-adapter.md) D-4's "the `RestateRuntimeAdapter` may remain the `RuntimeOps` façade that forwards to ingress *until* in-process embedding is proven stable" — the executor can move between placements as the runtime matures. The three-way-agreement rule ([`work-spec/CLAUDE.md`](../../CLAUDE.md) `Architecture`) applies once the executor exists: spec + in-memory reference + production adapter MUST produce the same verdict trace for the same pipeline document and input.

### D-7. Compatibility with the Signature Profile DAG

The Signature Profile already uses `depends_on: Vec<String>` ([`signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):420) with a runtime guard ([`ensure_step_can_complete`](../../crates/wos-runtime/src/runtime/signature.rs):1624-1664) that blocks dependent steps. This ADR adopts the same primitive at a parallel sub-layer (signature flow ⇄ contract-validation pipeline). The two are not unified into one shared DAG primitive in this ADR — they remain distinct schema surfaces because they validate distinct semantic axes (signing order vs validation order) — but the *shape* is shared: optional dependency list, topological execution, document-order fallback, runtime-enforced completion guard.

A future ADR MAY extract a shared `$defs/DependencyAwareStep` mixin if more sub-layer surfaces adopt this primitive (concurrent sibling ADR *transition-action DAG* is a candidate). This ADR does not extract it yet; one primitive at the schema level is preferable to one shared `$def` until the third instance exists, per [`work-spec/CONVENTIONS.md`](../../CONVENTIONS.md) sidecar-independence rubric.

---

## 3. Consequences

### Positive

- **Expresses topology the author already has in mind.** An author writing `[sourceGrounded, arithmetic, range, crossDocument]` with explicit `dependsOn: [sourceGrounded, arithmetic]` on `crossDocument` declares the actual data dependency. The runtime can parallelize the independent gates and join at `crossDocument`. Today's ordered-array form forces a serialization the author may not actually want.
- **Matches the existing sub-layer DAG precedent.** Multi-step sessions ([`advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5.3) and signing flows ([`signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):420) already use `dependsOn`-style step dependency arrays. Pipelines becoming the third surface with this shape is convergent, not divergent.
- **Composes cleanly with the kernel statechart.** D-5 fixes the categorical separation: lifecycle topology is statechart, sub-layer execution topology is DAG. The complementarity meta-ADR (concurrent sibling) can name pipelines, signing flows, and multi-step sessions as a coherent class of sub-layer DAG executors rather than three ad-hoc surfaces.
- **Backward compatible.** D-3 preserves every existing pipeline document's behavior. The fixture suite and prose are not invalidated by this ADR landing.
- **Unblocks the wos-server governance overlay** ([`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md)) by fixing pipeline execution semantics before the overlay rebakes a serialization convention into its runtime. The executor's topology is decided; only the implementation budget remains.

### Negative

- **Adds an optional field to the typed model and schema.** Every consumer of `$defs/PipelineStage` (lint, fixtures, generated docs) MUST handle the new field. The cost is bounded by the additive shape: existing consumers that ignore `dependsOn` produce the same behavior as today via the D-3 fallback.
- **Lint complexity.** `WOS-GOV-PIPELINE-DEP-001` (D-3 ambiguity) and a DAG-validity rule (`WOS-GOV-PIPELINE-DEP-002`: no cycles, no dangling references) are new lint surfaces. Mitigation: the rules are structural and Kernel-Structural conformance class — no FEL evaluation required, no semantic interpretation needed.
- **Join semantics committed (D-4) but not exercised.** Until the executor lands, no fixture validates the join-policy precedence. The risk is that real-world pipelines with multi-parent rejection surface a case where the D-4 ordering is wrong. Mitigation: D-4 is a documented, testable invariant; if a fixture later disproves it, an ADR amendment is one-line; the ADR does not lock the executor implementation, only the contract.

### Neutral

- **Pipelines as declarative authoring artifacts remain valid.** A workflow that declares a pipeline and no executor processes it remains a well-formed `$wosWorkflow` document. The DAG topology activates only when an executor exists.
- **No change to the assertion-gate taxonomy** ([`workflow-governance.md`](../../specs/governance/workflow-governance.md) §5.4). The seven gate types remain. Only their *composition* gains explicit dependency structure.
- **No change to existing provenance shape** ([`event_handler.rs`](../../crates/wos-core/src/event_handler.rs):300-344). `PipelineStageCompleted`, `PipelineRiskProfile`, `PipelineRejection` retain their schemas. The *emitter* changes (from external surface to runtime executor) when an executor lands; the records' shapes do not.

---

## 4. Implementation plan

This ADR commits the structural decision. Implementation is sequenced across the refactor.

1. **Spec §5 prose extension.** [`workflow-governance.md`](../../specs/governance/workflow-governance.md) §5.2 gains a `dependsOn` row in the PipelineStage property table. §5.3 gains a sub-section *5.3.1 Stage dependencies* describing the optional `dependsOn` array, the topological execution rule, the document-order fallback (D-3), and the join semantics (D-4). §5.6 cross-references the executor placement (D-6) and the kernel-statechart composition (D-5).
2. **Schema additions.** [`wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) `$defs/PipelineStage` adds `dependsOn: { type: array, items: { type: string }, default: [] }` with `x-lm.critical: false` (the field is structural, not a critical decision input on its own). At least one example in the `$defs/Pipeline` `examples` array carries a non-trivial DAG.
3. **Typed model extension.** [`work-spec/crates/wos-core/src/model/governance.rs`](../../crates/wos-core/src/model/governance.rs) `PipelineStage` gains `#[serde(default)] pub depends_on: Vec<String>` at line 251 alongside the existing `rejection_policy`. The serde rename to camelCase is automatic via `#[serde(rename_all = "camelCase")]` already on the struct.
4. **Lint rules.**
    - `WOS-GOV-PIPELINE-DEP-001` (Kernel-Structural): rejects mixed pipelines where some stages declare `dependsOn` and some omit it, unless every omitting stage either (a) has an explicit `dependsOn: []` in the document, or (b) appears before any stage with explicit `dependsOn`. Resolves D-3 ambiguity at authoring time.
    - `WOS-GOV-PIPELINE-DEP-002` (Kernel-Structural): rejects cycles, self-references, and references to non-sibling stage ids in `dependsOn`.
5. **Conformance fixtures.** Three fixture additions under [`work-spec/crates/wos-conformance`](../../crates/wos-conformance):
    - *Sequential-equivalent DAG.* A pipeline with `dependsOn` chained explicitly that produces the same verdict trace as the document-order pipeline. Proves D-3 backward compatibility.
    - *Parallel-stages DAG.* A pipeline with three independent assertion gates plus one cross-document join. Proves D-2 parallelism is admissible and the join receives all parent outputs.
    - *Multi-parent rejection join.* A pipeline where two parents reject with different policies. Proves D-4 join precedence (`failWithExplanation` > `escalateToSupervisor`).
6. **Executor stub.** A non-executing or stub executor lands in `wos-runtime` alongside the typed model extension, so cargo-check enforces the new field's presence at every consumer. The executor's full implementation (D-6) is sequenced separately against the wos-server governance overlay work in [`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md).
7. **Three-way agreement** ([`work-spec/CLAUDE.md`](../../CLAUDE.md) `Architecture`). Once the executor exists, in-memory and production adapters MUST agree on verdict traces for the conformance fixtures. The fixture set authored in step 5 is the agreement boundary.

The schema, typed model, lint rules, and fixtures land independently of the executor. The executor lands when the overlay is ready to host it; the decision in this ADR is structurally complete without it.

---

## 5. Alternatives considered

### 5.1 Keep the flat-sequence model

Rejected. The flat sequence encodes total ordering where no causal dependency exists for the majority of assertion-gate combinations (§1.3). When an executor materializes, it would either invent a sequential ordering convention bound into wos-server (re-running this decision implicitly, less visibly), or expose a parallelism escape hatch in extensions (`x-pipeline-parallel-stages: [...]`), fragmenting the topology surface. Fixing it once, declaratively, in the typed model is structurally cleaner and one optional field.

### 5.2 Saga-style execution with compensating actions

Rejected per [ADR 0070](../../../thoughts/adr/0070-stack-failure-and-compensation.md) D-5: WOS does not run a runtime saga. Pipeline stages are validators with structured rejection policies, not transactional steps with compensating actions. The `RejectionPolicy` enum ([`governance.rs`](../../crates/wos-core/src/model/governance.rs):317-322) is the WOS escalation taxonomy; saga semantics would replace it with a different abstraction (forward action + compensating action pair) the rest of the governance layer does not use. This ADR keeps `RejectionPolicy` as the failure surface.

### 5.3 Reactive recomputation on case-state mutation

Rejected per [`work-spec/TODO.md`](../../TODO.md):814 (`Rejected #5: DAG Processing Model. Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected`). A reactive pipeline that re-evaluates when its inputs change would couple pipeline execution to case-state mutation timing — making determinism dependent on event arrival order rather than transition firing. Pipelines in this ADR execute once per transition that invokes them; re-evaluation requires a new transition. The DAG describes the *internal* execution order of one invocation, not a reactive computation graph across the case lifetime.

### 5.4 Per-stage `joinPolicy` override

Deferred, not rejected. A future ADR MAY add `joinPolicy: { strategy: "strongest" | "first" | "all-must-succeed" }` to `PipelineStage` for fine-grained join control. This ADR commits to "strongest policy wins" (D-4) as the implicit default because (a) it matches the semantic intent of `RejectionPolicy` ordering — stronger rejection should not be overridden by weaker, and (b) introducing the override field before there is fixture-driven evidence for its necessity would be speculative addition. The override slot remains open in the `extensions` map per Kernel §10.6.

### 5.5 Unify with Signature Profile `SigningStep.depends_on` into a shared `$defs/DependencyAwareStep` mixin

Deferred per D-7. The shape is identical; the semantic axes (signing vs validation) are distinct. Premature extraction binds two surfaces' evolution. Revisit when a third surface (concurrent sibling ADR *transition-action DAG* is a candidate) adopts the same primitive — at that point, the shared `$def` rubric ([`work-spec/CONVENTIONS.md`](../../CONVENTIONS.md)) applies and a meta-ADR extracts cleanly.

### 5.6 Land foreach-style topology as a kernel state kind for pipelines

Rejected. Pipelines are not lifecycle states. They run *inside* a transition's contract evaluation; they do not appear in the statechart's state set. Promoting them to a kernel topology kind would violate ADR 0082 D-1 *Kernel Truth* (kernel topology is statechart, not a hybrid of statechart + DAG) and would force every kernel-only deployment to ship a pipeline executor it does not otherwise need. The named-seams invariant ([ADR 0075](../../../thoughts/adr/0075-rejection-register.md) I-9 territory; six canonical seams) places pipeline attachment at `contractHook`, not as a new kernel topology axis.

---

## 6. Relationship to the rejected DAG-as-lifecycle model

This section is non-normative; it exists to address the question this ADR predictably triggers: *if WOS rejected the DAG processing model, why is this ADR adding a DAG?*

[`work-spec/TODO.md`](../../TODO.md):814 records `Rejected #5: DAG Processing Model. Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected.` Read carefully, the rejection has two axes:

1. **DAG-as-lifecycle.** Replacing the kernel statechart with a flat DAG — making nodes-and-edges the canonical lifecycle topology. This is what FlowSpec-style proposals and BPMN-as-core-lifecycle proposals (rejection register row 1, [ADR 0075](../../../thoughts/adr/0075-rejection-register.md)) advocate. WOS rejects this because the statechart is deliberate (I-1), guards compose on transitions, and impact-capped governance attaches at structured state. ADR 0082 D-1 names it.
2. **DAG-as-reactive-recomputation.** A pipeline whose stages re-evaluate when upstream data changes, propagating recomputation through the graph. This conflicts with append-only event-stream folding (the runtime's command-event loop) because it would re-execute stages on mutation timing rather than transition firing.

This ADR proposes neither. The pipeline DAG:

- **Is not the lifecycle.** Pipelines run *inside* a transition's contract evaluation (D-5). The kernel statechart fires the transition; the pipeline executes; the verdict feeds back. Lifecycle topology remains statechart.
- **Is not reactive.** A pipeline executes once per invocation, walks its DAG topologically, reaches a terminal verdict, and stops. Re-execution requires a new transition. The DAG describes internal execution order of one invocation, not a continuously-recomputed dependency graph.

The categorical distinction is: lifecycle topology (rejected as DAG, accepted as statechart) versus sub-layer execution topology (accepted as DAG for pipelines, signing flows, multi-step sessions). The two surfaces are not interchangeable; the rejection of one does not preclude the other. The concurrent sibling meta-ADR *statechart-and-DAG complementarity* (number pending) names this separation as a stack-level invariant.

The Signature Profile already lives in this categorical position: signing flow is a sub-layer DAG (`SigningStep.depends_on` at [`signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):420) running inside a kernel-lifecycle signature transition. Multi-step sessions are the same shape for AI agents ([`advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5). Pipelines becoming the third surface with this shape does not reopen the rejected lifecycle-DAG row — it joins a coherent class of sub-layer execution primitives.

---

## 7. Cross-references

| Reference | Why it matters here |
|---|---|
| [ADR 0082 D-1](./0082-wos-kernel-semantic-projection-and-import.md) | Kernel topology is statechart, not graph. This ADR does not reopen that — the DAG operates strictly below the lifecycle. |
| [ADR 0078](../../../thoughts/adr/0078-foreach-topology.md) | Pattern precedent for adding a topology primitive (`foreach`) without breaking the kernel statechart invariant. Differs: `foreach` is kernel-layer (a fifth state kind); this ADR's DAG is sub-layer (inside a transition). Both leave statechart-as-canonical intact. |
| [ADR 0075 I-1, row 5](../../../thoughts/adr/0075-rejection-register.md) | Statechart-as-deliberate and the rejection of CMMN-style unconstrained runtime mutation. This ADR's pipeline DAG is bounded to a transition and does not mutate the statechart. |
| [ADR 0084 D-4](./0084-wos-restate-durable-runtime-adapter.md) | Executor placement guidance — `RuntimeOps` façade pattern. The pipeline executor can live behind the same façade, in-memory adapter or Restate-mediated, with three-way agreement on verdict traces. |
| [ADR 0093 §2.4](./0093-case-is-its-trellis-ledger.md) | Workflow event submission and ledger append discipline. Pipeline verdicts produce provenance events (`PipelineStageCompleted` etc.) that flow into the case ledger via the same `custodyHook` four-field append shape. |
| [ADR 0070 D-5](../../../thoughts/adr/0070-stack-failure-and-compensation.md) | No runtime saga. This ADR's pipeline rejection model uses `RejectionPolicy`, not compensating actions. |
| [`work-spec/specs/advanced/advanced-governance.md`](../../specs/advanced/advanced-governance.md) §5 | Multi-step session DAG precedent. Same shape (`dependsOn`-array), different sub-layer surface (agent actions vs validation pipelines). |
| [`work-spec/crates/wos-runtime/src/runtime/signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):381-424, 1624-1664 | Signature Profile DAG precedent. Same shape, same runtime enforcement pattern, different semantic axis. |
| [`TRELLIS-WOS-REFACTOR-TODO.md`](../../../TRELLIS-WOS-REFACTOR-TODO.md) | The refactor that motivates the executor existing. wos-server as governance overlay is where the executor likely lives in the production stack. |
| Concurrent sibling ADRs (numbers pending) | *Transition-action DAG*, *form-intake validation*, *statechart-and-DAG complementarity meta-ADR* — this ADR composes with them; the meta-ADR names the categorical separation explicit at the stack level. |
| [`work-spec/TODO.md`](../../TODO.md):814 (Rejected #5) | See §6: the rejection is about lifecycle-DAG and reactive recomputation, not sub-layer DAG execution. This ADR is consistent with that rejection. |
