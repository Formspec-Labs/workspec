# ADR 0094 — Optional DAG Semantics for Transition Action Bundles

**Status:** Proposed
**Date:** 2026-05-14
**Scope:** WOS Kernel — `State.on_entry` / `State.on_exit` / `Transition.actions` bundles; runtime action executor; conformance fixtures; `wos-workflow.schema.json` Action definition.

**Related:**
[ADR 0082 (kernel statechart and semantic projection)](./0082-wos-kernel-semantic-projection-and-import.md) D-1 (Kernel Truth — statechart canonical);
[ADR 0084 (Restate durable runtime adapter)](./0084-wos-restate-durable-runtime-adapter.md) D-1 (Virtual Object single-writer per `process_id`);
[ADR 0078 (`foreach` topology)](../../../thoughts/adr/0078-foreach-topology.md) (precedent — sub-layer extension to a kernel primitive);
[ADR 0093 (case is its Trellis ledger)](./0093-case-is-its-trellis-ledger.md);
[ADR 0075 (rejection register) row 8](../../../thoughts/adr/0075-rejection-register.md) (FlowSpec flat node/edge topology rejection) and WOS [`TODO.md`](../../TODO.md) Rejected row #5 (DAG Processing Model);
[`work-spec/specs/kernel/spec.md`](../../specs/kernel/spec.md) §4.3 (States), §4.7 (Transition Execution Sequence), §9.2 (Actions), §9.2.13.1 (Sequential Execution Within a State), §9.2.15.3 (Parallel Region Actions);
[`work-spec/crates/wos-core/src/model/kernel.rs`](../../crates/wos-core/src/model/kernel.rs):595-599 (State `on_entry`/`on_exit`), :1084-1182 (Action shape), :1184-1195 (ActionKind enum);
[`work-spec/crates/wos-runtime/src/runtime/drain.rs`](../../crates/wos-runtime/src/runtime/drain.rs):166-178 (`apply_observed_actions` site);
[`work-spec/crates/wos-runtime/src/runtime/signature.rs`](../../crates/wos-runtime/src/runtime/signature.rs):378-424 (`SigningFlow` / `SigningStep.depends_on` precedent);
sibling: ADR (proposed) — *Governance Pipeline Execution Topology* ([`work-spec/thoughts/adr/`](.));
sibling: ADR (proposed) — *Form-Intake Validation Topology* ([`formspec-server/thoughts/adr/`](../../../formspec-server/thoughts/adr/));
sibling: ADR (proposed) — *Phase-2 Causal-Deps Activation* ([`trellis/thoughts/adr/`](../../../trellis/thoughts/adr/));
sibling: ADR (proposed) — *Statechart and DAG Complementarity (Stack Meta-ADR)* ([`thoughts/adr/`](../../../thoughts/adr/)).

---

## 1. Context

### 1.1 What action bundles are

Three Kernel surfaces carry ordered action lists: `State.on_entry`, `State.on_exit` (typed at [`work-spec/crates/wos-core/src/model/kernel.rs`](../../crates/wos-core/src/model/kernel.rs):595-599 as `Vec<Action>`), and `Transition.actions`. Spec §4.7.3 / §4.7.4 define the per-transition execution sequence — `onExit` of source states (innermost-first), transition actions, `onEntry` of target states (outermost-first), then provenance emission. Within any one bundle, §9.2 names sequential document order: *"Actions within a single state's `onEntry` or `onExit` execute sequentially in document order. The processor MUST NOT reorder actions within a state or transition."*

Today's `Action` enumerates seven kinds ([`work-spec/crates/wos-core/src/model/kernel.rs`](../../crates/wos-core/src/model/kernel.rs):1184-1195): `createTask`, `invokeService`, `setData`, `emitEvent`, `startTimer`, `cancelTimer`, `log`. The struct has no `id` and no dependency edge; ordering is implicit in `Vec` position.

### 1.2 The reality

Most action bundles in practice are independent side-effects fanned out at a single lifecycle event: emit a kernel event, log an audit line, schedule a deadline timer, notify an external service, mutate a derived projection field. The list ordering is a topological linearization of a DAG with **no real dependencies**. The author wrote them in some order because `Vec<Action>` is the only shape the schema admits; the runtime executes them sequentially because `Vec` implies order. Nothing in the semantics requires the second action to wait for the first when the second does not read what the first wrote.

This is an implicit DAG hiding as a list. The shape lies about the dependency structure: a reader cannot tell from the document whether the order is load-bearing (rare — e.g. `setData` followed by `emitEvent` carrying the data) or accidental (common — e.g. parallel notifications). Conformance traces (Kernel §9.2.15.3) already permit parallel-region actions to execute concurrently, but within one bundle the linearization is mandatory.

### 1.3 The existing precedent

The Signature Profile already names this primitive. [`SigningStep`](../../crates/wos-runtime/src/runtime/signature.rs):378-424 carries `id: String` and `depends_on: Vec<String>`, with a `SigningFlowType` enum (`Sequential` / `Parallel` / `Routed` / `FreeForAll`) describing flow shape. Multi-signer workflows declare their step graph explicitly: the runtime walks the DAG, parallelizing where `depends_on` is empty and serializing where dependencies bind. The pattern is local to the Signature Profile but it is the existing WOS-side DAG primitive, and the shape is small (`id` plus `depends_on`).

### 1.4 The Harel commitment is not at risk

Kernel §4 commits `Lifecycle` to a Harel/SCXML-derived statechart per [ADR 0082](./0082-wos-kernel-semantic-projection-and-import.md) D-1 (*"the hierarchical `$wosWorkflow` lifecycle remains the canonical authoring and processing form"*) and [ADR 0075 row 8](../../../thoughts/adr/0075-rejection-register.md) (FlowSpec flat-node/edge topology rejected at I-1). Action bundles are **not part of statechart topology** — they are side-effects fired *during* a transition (steps 1 and 3 of §4.7.4's transition-execution sequence). Adding DAG semantics inside a bundle does not change which states exist, which transitions fire, when guards evaluate, how the deterministic-evaluation algorithm (§4.2) operates, or how case-state mutations land. The lifecycle remains a pure function of (states × event × guards → next states); only the *order of execution of co-equal side-effects* gains expressivity.

The relevant WOS [`TODO.md`](../../TODO.md) *Rejected* row #5 ("DAG Processing Model — Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected") rejected a **replacement** for the lifecycle/processing model — reactive re-evaluation of a node graph at the place statecharts run. This ADR proposes nothing of that shape: it does not introduce a node graph, does not introduce reactive re-evaluation, does not change folding, and does not displace any statechart primitive. It adds opt-in expressivity inside one cell of the existing statechart's execution sequence. The rejection holds; this ADR is on the other side of the line.

### 1.5 Sibling ADRs

This ADR is one of four sub-layer DAG additions being authored concurrently, joined by a stack-level meta-ADR:

- **Governance Pipeline Execution Topology** (WOS, sibling) — DAG semantics for the governance pipeline (Workflow Governance §6 review/validation/audit) where today's pipeline shape is implicit.
- **Form-Intake Validation Topology** (`formspec-server`) — DAG semantics for intake validation steps.
- **Phase-2 Causal-Deps Activation** (`trellis`) — activating `causal_deps` on Trellis events under a Phase-2 substrate.
- **Statechart-and-DAG Complementarity** (stack meta-ADR, root `thoughts/adr/`) — the framing principle: statechart governs *behavior over time* (states, transitions, lifecycle events); DAG governs *computation order* within a behavioral cell (action bundle, validation pipeline, governance pipeline, event-causality graph). The two are orthogonal, and the layering is named.

Cross-stack consistency: each sub-layer DAG addition reuses the same `id` + `depends_on` shape (the Signature Profile precedent), keeps the DAG **opt-in** so existing documents are unchanged, and treats the DAG as a *declaration of intent* that the runtime adapter MAY linearize under host constraints. The meta-ADR enumerates the four sub-layers and asserts the principle.

---

## 2. Decision

Action bundles (`State.on_entry`, `State.on_exit`, `Transition.actions`) gain **optional** DAG semantics. The default shape remains `Vec<Action>` with sequential document-order semantics — every existing Kernel document continues to validate, lint, and execute identically.

Authors who want explicit parallelism declare actions with optional `id: String` and `depends_on: Vec<String>` fields. When any action in a bundle declares `depends_on`, the runtime treats the bundle as a DAG: independent actions execute concurrently (subject to runtime-adapter constraints — e.g. ADR 0084 D-1's Restate Virtual Object single-writer-per-`process_id` may force serialization regardless), and dependent actions wait for their predecessors. Provenance ordering follows topological order, not declaration order.

### D-1. `Action` gains optional `id` and `depends_on`

Extend `Action` ([`work-spec/crates/wos-core/src/model/kernel.rs`](../../crates/wos-core/src/model/kernel.rs):1087-1182) with two optional fields:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | OPTIONAL | Stable identifier within the bundle. Required when other actions in the same bundle declare `dependsOn` referencing this action. MUST match `^[_a-zA-Z][a-zA-Z0-9_-]*$`. MUST be unique within its enclosing bundle. |
| `dependsOn` | array of string | OPTIONAL | IDs of other actions in the same bundle that MUST complete before this action begins. Each entry MUST resolve to a sibling action's `id` in the *same* bundle (cross-bundle dependencies are not expressible — bundles are separate per Kernel §4.7.4 step order). Cycles are forbidden. |

Field naming follows the [`SigningStep.depends_on`](../../crates/wos-runtime/src/runtime/signature.rs):420 precedent (kebab-case in JSON via serde rename, snake_case in Rust).

Bundle scope is the enclosing `on_entry` / `on_exit` / `Transition.actions` list. Dependencies do not cross bundle boundaries — `onEntry` actions of state X cannot depend on `onExit` actions of state Y, and transition actions cannot depend on the source state's `onExit`. The §4.7.4 step ordering (innermost-first `onExit` → transition `actions` → outermost-first `onEntry`) remains the only inter-bundle ordering surface. Authors who need cross-bundle dependencies are using the wrong primitive — they want a transition or a substate, not an action.

### D-2. Opt-in: absence of `dependsOn` preserves current semantics

If **no** action in a bundle declares `dependsOn`, the bundle is processed as today: sequential execution in document order, with §9.2 / §9.2.13.1 unchanged. Actions without `id` are anonymous and remain sequential under this rule.

Mixed bundles are admitted: a bundle MAY contain a mix of actions with and without `id`. Actions without `id` cannot be referenced by `dependsOn`. Their execution order relative to identified actions is governed by D-3.

### D-3. DAG semantics when any action declares `dependsOn`

If any action in a bundle declares a non-empty `dependsOn`, the bundle is processed as a DAG:

1. **Topological order is derived** from the bundle. Anonymous actions (no `id`) are treated as having an implicit dependency on the action immediately preceding them in document order — this preserves "do thing, then this anonymous side-effect" intuition for partially-annotated bundles. Anonymous actions at the head of the bundle have no implicit dependency.
2. **Independent actions** (no `dependsOn`, or `dependsOn` entries all satisfied) MAY execute concurrently.
3. **Dependent actions** MUST wait for every action named in their `dependsOn` to complete (with non-error outcome under Kernel §9.5 compensation rules) before they begin.
4. **Provenance order** follows topological order. When two actions are concurrent, provenance MAY record either order, but the recorded order MUST be a valid topological linearization of the DAG.

The DAG is **per-bundle**, not per-workflow. Each `on_entry`, each `on_exit`, and each `transition.actions` list resolves to its own DAG (or to a sequential list when no `dependsOn` is declared).

### D-4. Runtime adapters MAY serialize; DAG declares intent, not execution

The DAG semantics describe **declared intent**. A runtime adapter MAY linearize execution under host constraints. In particular:

- **Restate Virtual Object** ([ADR 0084 D-1](./0084-wos-restate-durable-runtime-adapter.md)) keys a workflow process by `process_id` and serializes handler invocations per key. Action execution inside one handler runs in one journaled thread of control; ADR 0084 D-4 already establishes that non-deterministic I/O routes through SDK-journaled APIs. A Restate-backed processor MAY linearize a declared-parallel action bundle within the single Virtual Object handler call — this remains conformant, because the DAG declares intent and the adapter chooses execution strategy.
- **In-memory adapter** (`wos-runtime`, the conformance oracle) MAY execute independent actions sequentially in topological order — also conformant.
- **A future Tokio-based adapter** with no single-writer constraint MAY parallelize independent actions onto a worker pool — also conformant.

Three-way agreement (in-memory + Restate + future production adapter) holds because the spec defines the **observable outcome** — the topological-order provenance stream, the case-state mutations, the emitted events — not the wall-clock execution strategy. Two adapters producing the same provenance trace from the same DAG and case state are equivalent at the spec layer.

This separation is load-bearing: it permits the DAG to be a *static analyzable artifact* (lint can verify topology, schema can verify the field shape, AI authors can declare independence without committing the runtime to actually parallelize) without forcing every adapter to ship a work-stealing scheduler.

### D-5. Provenance ordering is topological, not declaration

Today's provenance stream records actions in the order they execute. Under §9.2.15.3, parallel-region actions MAY execute concurrently, and provenance MUST record the actual execution order. Under this ADR:

- Sequential bundles (no `dependsOn`) — provenance records in declaration order (unchanged).
- DAG bundles — provenance records in topological order. When the DAG admits multiple valid linearizations (concurrent actions), the adapter records the one it chose to execute. Two conformant adapters processing the same DAG document MAY emit different linearizations and remain conformant *provided* both linearizations respect the topology.

This is the same posture parallel regions (§9.2.15.3) and `foreach` with `concurrency: N` (ADR 0078 D-5) already take. The kernel's deterministic-evaluation invariant (§4.2) applies to **state transitions**, not to the within-transition action linearization across concurrent surfaces. Two conformant processors MUST agree on which actions executed and on the case-state mutation outcome; they MAY disagree on the recorded interleaving within a DAG bundle.

### D-6. Static validation and lint

- **L-action-dag-001 — `dependsOn` references resolve.** Every entry in an action's `dependsOn` MUST match an `id` of another action in the same bundle. Unresolved references fail. Conformance class: **Kernel Structural** (schema/lint resolvable).
- **L-action-dag-002 — no cycles.** The DAG induced by `dependsOn` MUST be acyclic. Cycles fail. Conformance class: **Kernel Structural**.
- **L-action-dag-003 — `id` uniqueness per bundle.** Within one bundle, every `id` MUST be unique. Conformance class: **Kernel Structural**.
- **L-action-dag-004 — `id` referenced only when needed.** A WARN-class diagnostic flags actions that declare `id` but no other action references them — the field is unused weight. Conformance class: **Kernel Structural**, warning-tier.

### D-7. Schema additions

Extend the `Action` `$defs` block in [`work-spec/schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) with `id` and `dependsOn` properties. Both default to absent. Add the pattern constraint on `id` and the array-of-string shape on `dependsOn`. No conditional `allOf` is required — the fields are independent of `kind`. The lint rules in D-6 are not schema-encodable (they require bundle-scope analysis); they land in `wos-lint` as Kernel Structural lints.

### D-8. Compensating actions follow the same DAG

`Action.compensating_action` ([`work-spec/crates/wos-core/src/model/kernel.rs`](../../crates/wos-core/src/model/kernel.rs):1175-1177) is a single boxed action attached to its parent. Under Kernel §9.5 compensation, the compensating action runs if the parent fails. This shape is unchanged: a compensating action is a *child* of one action, not a *peer* in the bundle. Compensating actions do not appear in the bundle's `id` space and cannot be referenced via `dependsOn`. If complex compensation flows need their own DAG, authors model them as a compensating *bundle* via a structured rollback transition, not by nesting `dependsOn` inside compensation.

### D-9. Migration and conformance

- Existing Kernel documents (every fixture under `wos-conformance`, every customer document if any existed) continue to validate, lint, and execute identically. No `Vec<Action>` shape changes.
- The Rust `Action` struct gains two `Option`-defaulted fields; existing constructors and tests compile unchanged.
- The action-executor sites in `wos-runtime` (notably [`drain.rs:176-178`](../../crates/wos-runtime/src/runtime/drain.rs) `apply_observed_actions`) gain a DAG-resolution path that activates only when `dependsOn` is non-empty on at least one action in the bundle. The sequential path remains the default.
- New conformance fixtures land under `wos-conformance`: (a) sequential bundle (regression — confirms no behavior change); (b) two-node DAG with one dependency (`b dependsOn [a]`); (c) diamond DAG (`d dependsOn [b, c]`, `b dependsOn [a]`, `c dependsOn [a]`); (d) mixed anonymous + identified actions (verifies D-3 implicit-edge rule); (e) cycle rejection (lint failure, processor reject); (f) cross-adapter agreement (in-memory + Restate produce equivalent topological-order provenance streams).

---

## 3. Consequences

**Positive.**

- **Authoring honesty.** The shape of the document matches the dependency reality. A bundle of five independent notifications declares itself as such; a `setData` → `emitEvent` chain declares the dependency. AI authors emitting Kernel documents can declare independence, which is information lint can verify and tooling can act on.
- **Adapter latitude.** Production adapters with concurrency budgets can parallelize independent actions without violating spec; adapters with single-writer constraints (Restate Virtual Object per ADR 0084) keep linearizing without violating spec either. The DAG decouples *declared intent* from *runtime execution strategy*.
- **Static analyzability.** Lint can verify that a bundle's declared DAG is acyclic and well-referenced. Tooling can render bundles as graphs in authoring UIs. Conformance traces can describe execution as a topological walk, making the trace more meaningful when the bundle is non-trivial.
- **Sibling consistency.** Reusing the `id` + `depends_on` shape from the Signature Profile, and from the three sibling sub-layer ADRs, keeps the stack's DAG vocabulary uniform. One pattern, applied at four sub-layers under one meta-ADR.

**Negative.**

- **Two execution paths in the runtime.** The action executor in `wos-runtime` gains a DAG-resolution path alongside the existing sequential path. The DAG path runs only when `dependsOn` is non-empty, so the sequential path remains the hot path for the common case, but the code surface grows. Mitigation: the DAG resolver is a small topological-sort routine; the executor's hot path is unchanged.
- **Schema surface grows.** Two new optional properties on `Action`. Authoring complexity rises slightly: authors who learn `dependsOn` must also learn that anonymous actions get implicit document-order dependencies (D-3 step 1). Mitigation: documents that never use `id` see no behavior change.
- **Provenance non-determinism inside a bundle.** Two adapters MAY record different topological linearizations of the same DAG. This is consistent with §9.2.15.3 parallel-region semantics and ADR 0078 D-5 `foreach`-`concurrency` semantics, but it expands the surface where two conformant traces disagree on within-event ordering. Mitigation: the disagreement is bounded to within-bundle interleavings; case-state outcome, emitted events, and state transitions remain identical.

**Neutral.**

- **Statechart commitment unchanged.** ADR 0082 D-1 ("Kernel Truth") and ADR 0075 row 8 (flat-node-graph rejection) are not affected. The lifecycle remains a Harel statechart; action bundles are not a topology surface. The DAG lives strictly inside one execution-sequence cell.
- **WOS TODO Rejected row #5 ("DAG Processing Model") unchanged.** The rejection was of a reactive-re-evaluation processing model that *replaces* the lifecycle. This ADR adds sub-layer DAG inside an existing lifecycle cell. The rejection still applies to any future proposal to make the *lifecycle itself* a DAG.
- **`foreach` topology (ADR 0078) unchanged.** ForEach iterations remain bounded-collection loops; per-iteration `onEntry` / `onExit` bundles MAY use DAG semantics in the same way any other bundle MAY. `concurrency` on `foreach` controls iteration-level parallelism; `dependsOn` controls action-level parallelism within one iteration's bundle. Orthogonal axes.
- **Existing kernel processors unchanged.** A processor that ignores `id` / `dependsOn` and treats every bundle as sequential is incorrect under this ADR but produces a conformant subset (every document without `dependsOn` works). Strict-conformance processors implement D-3 and the lint rules.

---

## 4. Alternatives considered

- **Keep `Vec<Action>` as canonical, never declare independence.** Rejected. The implicit DAG is real today — authors *want* to express "these are independent" and have no way to. Adapters parallelize on §9.2.15.3 across regions but not within a single state's bundle. Leaving parallelism hidden means production adapters either over-serialize (slow) or invent their own analysis (drift). One declared shape is better than scattered inferences.
- **Mandatory DAG: require `id` and `dependsOn` on every action.** Rejected. Most existing bundles are short (1–3 actions) and don't need the shape. Forcing every author to invent stable IDs and reason about dependencies for trivial bundles is hostile to AI authoring (more required fields = more dimensions of failure for LLM emission) and breaks every existing fixture. Opt-in is the right posture.
- **Hierarchical action groups (`group: { actions: [...], parallel: true }`).** Rejected. A second nesting layer inside bundles. Group boundaries become a topology surface (does a group fail-fast? does it wait-all? does it compose with `dependsOn` across groups?), and the answer would replicate parallel-state semantics inside the action layer. If hierarchical composition of independent execution units is needed, the existing kernel primitives (compound states, parallel states with regions, `foreach`) are the right place — they already have well-defined cancellation, completion, and provenance semantics. Action bundles stay flat.
- **Use `SigningFlowType`-style enum on the bundle (`type: "sequential" | "parallel"`).** Rejected as the *only* shape. An enum at the bundle level can express "all parallel" or "all sequential" but cannot express partial DAGs (most action sets fall here: two independent notifications followed by a logging step that waits for both). The Signature Profile uses the enum *and* `depends_on` together because flow shape and per-step dependency are different axes. Action bundles don't need a flow-shape enum — the DAG (with empty `dependsOn` meaning fully parallel and document-order anonymous-edge inference meaning fully sequential) covers both extremes plus the middle.
- **Land action DAG as a sidecar / governance-layer concern.** Rejected. Action execution is Kernel-layer — it happens in §4.7.4's transition-execution sequence, inside the Kernel runtime, before any governance attachment. A sidecar that declared "this action set is parallel" would have no place to attach: the Kernel runtime is what dispatches the actions, and the Kernel runtime is what would honor (or ignore) the parallel declaration. Either Kernel knows about DAG (this ADR) or DAG isn't real to the runtime. There is no middle.

---

## 5. Implementation notes

The work is small. The Rust changes are `Action` field additions plus a DAG resolver in `wos-runtime`; the schema changes are two optional properties on the `Action` `$defs` block; the spec changes add a `§9.2.13.2 Optional DAG Semantics` subsection cross-referencing this ADR. The lint rules (D-6) land in `wos-lint` as Kernel Structural diagnostics. Conformance fixtures (D-9) land in `wos-conformance` and exercise both the in-memory adapter and the Restate adapter (three-way agreement per `work-spec/CLAUDE.md`).

No migration is required — every existing document works unchanged. The opt-in nature of the feature means it can land incrementally: schema + Rust struct fields first (no runtime change), then DAG resolver + lint rules (the executor path activates when `dependsOn` is observed), then conformance fixtures (which exercise the new path).
