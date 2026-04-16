# Idea Scratch — Design Direction

**As of 2026-04-16**, active backlog items (Adopt / Defer / Reject / Shipped / Structural Merges / Implementation Priority / Open Questions / Next Steps) have moved to [TODO.md](TODO.md). Positioning material (The Genuine Invention) has moved to [POSITIONING.md](POSITIONING.md). This document now retains only architectural framing that informs how new items should be scored and scoped. **New items: add to TODO, not here.**

**Lens:** greenfield. No users, no legacy, no migration concerns. *Code is cheap, time is cheap, architecture is invaluable.* Delete only when truly not needed; defer when low architectural lock-in; reject when contradicting committed axes. No backward-compat scaffolding.

**Audit trail (2026-04-16):**

- Merge with orchestration-patterns research doc.
- Four-agent spec-suite audit (reinvention / missing-lead / contradictions / pre-schema).
- Code-scout validation against live crates/fixtures.
- Opus-high greenfield pass: removed `named`/`x-*` extensibility wrappers, reframed Tech Debt as architectural lock-in, honest prior-art attribution on Genuine Invention claims, ruthless trim of anticipatory ceremony.
- 2026-04-16 merge: all actionable items absorbed into TODO; this doc reduced to framing only.

---

## Design Direction

### The reframed question

"Should the WOS kernel grow BPMN-equivalent orchestration patterns so AI can generate a single document?" is the wrong question. BPMN parity invites scope creep and positions WOS as a competitor to a 20-year-old standard nobody loves authoring.

The right question: **what must WOS be such that AI can generate a complete, executable, governed workflow in one JSON document that runs on a WOS-native runtime and can also export to engine-specific formats (BPMN, Temporal, SCXML) as interop targets?**

### Five design axes

Properties the format must deliver. Not borrowed from any reference system.

1. **AI-generability** — closed finite vocabularies, structured diagnostics, canonical forms, schema-enforceable authoring contracts.
2. **Multi-target semantic fidelity** — every normative behavior has a specified semantic rule, not "implementation-defined." Bindings translate faithfully and document what they lose.
3. **Governance as first-class primitive** — deontic operators, due process, review protocols, authority ranking, provenance as native constructs on a declared seam.
4. **Replay determinism over an append-only event stream** — case state derived by folding events, not reactive re-evaluation. Deterministic replay including governance is a spec guarantee. **Consequence:** DAG processing (reactive re-evaluation) is explicitly rejected.
5. **Long-running version migration** — workflows in flight outlive governance documents. Version pinning and migration policy are first-class.

Axes describe format properties, not capability inventory.

### Ground truth: WOS today

**Closed vocabularies:**

- **State kinds** — 4 (`atomic | compound | parallel | final`) — `wos-kernel.schema.json` `$defs/State.properties.type`
- **Action kinds** — 7 (`createTask | invokeService | setData | emitEvent | startTimer | cancelTimer | log`) — `wos-kernel.schema.json:406`
- **Case-field types** — 8 — `wos-kernel.schema.json:617`
- **FEL** — normatively required for guards, milestones, action parameters — `specs/kernel/spec.md §7.4`; kernel imports `fel_core` (`crates/wos-core/src/eval.rs:13`)
- **Integration Profile binding types** — 7 — `request-response | event-emit | event-consume | callback | arazzo-sequence | tool | policy-engine`
- **Six extension seams** — `actorExtension | contractHook | provenanceLayer | lifecycleHook | custodyHook | extensions` — `specs/kernel/spec.md §10`
- **Eleven kernel-generated events** — `$join`, five `$timeout.*`, `$error`, `$compensation.complete`, three `$related.*` — `specs/kernel/spec.md §4.10`

**Open (tracked for closure in TODO §4):** `Transition.event` is free-form string (TODO #20 closes to 5 typed kinds, no `named` escape hatch). `HoldPolicy.holdType`, `CaseRelationship.type`, `AppealMechanism.reviewerConstraint` are prose enums without schema enforcement (TODO #46 closes them). `custodyHook.additionalProperties: true` escape hatch (TODO #22 closes it; Trellis shape moves to TODO #21 extension registry).

**Layering — four documents:**

| Concern | Document | Schema |
|---|---|---|
| Structure (state machine topology) | Kernel | `wos-kernel.schema.json` |
| Governance (due process, holds, delegation, policy) | Workflow Governance | `wos-workflow-governance.schema.json` |
| Execution algorithms (compensation, sync, history) | Lifecycle Detail Companion | (prose) |
| Runtime contract (event delivery, durability, ordering) | Runtime Companion | (prose) |

Governance attaches via `lifecycleHook` keyed on **semantic tags** (`determination`, `review`, `adverse-decision`, `quality-check`, `intake`, `appeal`, `notification`, `hold`), not transition IDs.

**Conformance** — 197 normative constraints in `LINT-MATRIX.md` (37 static / 55 cross-doc / 105 dynamic); 3230+ lines of typed rule code.

**AI-readiness** — `x-lm.critical` annotations on 131 schema nodes (all currently pass description+examples per validation); `.llm.md` files per spec; rule-ID-keyed conformance fixtures; `wos-lint` structured diagnostics.

**Code-level smells (tracked in TODO #22):**

- `wos-core` exports L2/L3 modules in kernel crate (`crates/wos-core/src/lib.rs:22-37`)
- `ProvenanceKind` — 93-variant monolith enum (`crates/wos-core/src/provenance.rs`)
- `wos-runtime/src/runtime.rs` — 3821 lines, mixed dispatch
- `wos-formspec-binding` depends on `wos-runtime` — inversion
- `impactLevel` lives in kernel but is consumed only by governance (decided: stays)
- Kernel fixtures named after Layer 1/2/3 concerns (relocate under TODO #22)
- `wos-correspondence-metadata.schema.json` under `schemas/kernel/` self-describes as sidecar (relocate under TODO #22)
- `DRAFTS/` contains 12 kernel version proposals — triage before any schema/spec PR lands

**BPMN relationship** — Harel statechart semantics, not BPMN topology (`kernel/spec.md:636`). Appendix A acknowledges BPMN event-taxonomy adoption; TODO #20 makes that adoption normative (typed union) rather than informative. Any durable execution runtime is valid (Kernel §A). Export path is via a `wos-bpmn-export` crate; WOS is the authoring surface.

### Non-goals

- Not BPMN parity.
- Not Formspec's 4-phase reactive processing model. Forms are frozen inputs; workflows are append-only event streams.
- Not demoting governance to "advisory presentation." Audit logs, notifications, deadlines are behavioral obligations.
- Not putting deontic operators in the extension registry — MUST/MAY/SHALL-NOT are core primitives.
- Not single-version response pinning. Cases outlive governance versions.
- Not eliminating BPMN. Export target, not authoring surface.

---

## Architectural Decisions Confirmed

Decisions that hold under re-audit from first principles (2026-04-16 greenfield lens).

- **Constraint zones as overlay**, not kernel state type. Implementations shouldn't require DCR understanding; five state types violates KISS.
- **Monolithic document over tripartite** — reinforced 2026-04-16 by research corpus (compass Direction 3 + statechart lifecycle). Not preserved by fixture inertia.
- **Event-driven evaluation over DAG** — committed at axis 4. Reactive re-evaluation explicitly rejected.
- **FEL over FEEL** — purpose-built.
- **Kernel includes lifecycle** — coherent single-document understanding.
- **Granular decomposition over kernel/profile binary** — target sidecar count ~12, determined per sidecar under the keep-separate test below.
- **Hybrid layered architecture with statechart lifecycle** — compass Direction 3 + Direction 1 validated.
- **BPMN as export target, not authoring surface** — topology rejected; event taxonomy adopted normatively via TODO #20.
- **`impactLevel` stays in kernel** — Runtime §2.4 requires governance-independent kernel eval; `impactLevel` gates governance strength. Caveat: decision and §2.4 share authorship circularity; revisit if downstream specs expose the problem.
- **Sidecar keep-separate test** — a sidecar earns independent existence when it has a **distinct semantic model** or **distinct artifact lifecycle**, not because "regulators might update it" (anticipatory / no deployments exist). Survivors: Policy Parameters, Business Calendar, Equity Config, Assurance, Integration Profile, Lifecycle Detail, Runtime Companion, Advanced Governance, Notification Template, Due Process Config (partial merge pending per TODO #45 step 0), Workflow Governance (absorbs Assertion Library), Advanced Governance (absorbs Verification Report).

---

## Capability Status (vs. Research Corpus)

2026-04-16 audit cross-checked 12 research-corpus "missing from all systems" capabilities.

- **Implemented:** Temporal parameter versioning (OpenFisca-style, `policy-parameters.md`); separation of duties (specified `§7.2`, runtime enforcement via TODO #22 cleanup); business-calendar-aware SLAs (calendar shipped; jurisdiction selection in TODO #31); AI confidence annotations (`§6.3`, `§7.1`).
- **Partial / in flight:** decision provenance (TODO #24a + #24b); override accountability (TODO #23); WS-HumanTask lifecycle (TODO #30); GSM artifact-centric progression (TODO #29a + #29b); equity monitoring (TODO #35 + #36); role-scoped visibility (TODO #26a + #26b); AI drift governance (TODO #37).
- **Not implemented (tracked in TODO):** defeasibility (#25); cancellation regions (#27); claim-check (#28); Assurance × impact composition (#43).

---

## Cross-Project Dependencies

Stability of certain WOS constructs is gated by external work:

- **Formspec Assist** — `ai-integration.md §14` proxy stabilizes when Assist upstream stabilizes.
- **Formspec Core** — FEL grammar (Kernel §7.4 imports `fel_core`). Affects TODO #36 (equity expression language).
- **Trellis** — `custodyHook.additionalProperties` escape hatch will close via TODO #22; Trellis shape relocates to TODO #21 registry entry.

---

## Convention

Every new or revised spec MUST include **Normative Contract** (processor MUST/SHOULD/MAY obligations), **Composition** (seam attachment, precedence, conflict resolution), and **Conformance** (fixture rule-ID patterns) sections. Retrofit existing specs via TODO #45. Template in [CONVENTIONS.md](CONVENTIONS.md).

---

## Research Corpus

| File | Role |
|------|------|
| `compass_artifact...markdown.md` | 50+ standard survey; 7-layer recommendation (compass Direction 3 validates current architecture) |
| `Toward an open...docx` | Feature taxonomy; DCR / Catala / XES / GSM / OpenFisca discoveries |
| `Agentic AI Integration...docx` | Agent protocols (MCP, A2A); OWASP / NIST / EU AI Act |
| `AI-Native Workflow Standards...docx` | BPMN / DMN / CMMN survey (mostly redundant with compass) |
| `prompts/research-prompt.md` | Prompt for the compass artifact |

**Research-identified improvements tracked:** R1 SCITT transparency → TODO #48. R2 SLSA agent attestations → informs Deferred #1.
