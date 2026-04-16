# Idea Scratch

Informal notes on improvements worth adopting. Not a plan — just a place to capture thinking before it becomes one.

Each item uses the same shape: **Idea / Context / Source / Benefits**.

**Merged 2026-04-16** with the orchestration-patterns research doc. Design direction, implementation deltas, and capability-validation findings live here alongside the scored backlog. Extended 2026-04-16 with findings from a four-agent spec-suite audit: reinvention risks, missing-lead gaps, contradictions, merge candidates, pre-schema quality concern, and TODO↔IDEA reconciliation.

---

## Draft Evolution

| Version | Key Addition | Outcome |
|---------|--------------|---------|
| v0.1.0 | 7-layer monolith, SCXML/WS-HumanTask foundations, no agents | Decomposed into 18 published specs |
| v2.0.0 | Agent governance, actor model, autonomy, guardrails, confidence, oversight, due process, impact levels, 4-layer provenance | All adopted |
| v3.0.0 | JSON-LD, SHACL, PROV-AGENT, OCEL 2.0, RO-Crate | Not adopted |
| v4.0.0 | Kernel/profile split, tripartite objects, deontic constraints, capability contracts | Deontic adopted; rest deferred |
| v5.0.0 | Formspec integration, FEL, Assist Governance, Mapping DSL, Arazzo, processing model | Most adopted |
| v6.0.0 | Constraint zones, DAG processing, FEL profiles, typed patches | Analyzed below |
| v7 variants | Minimal governance envelope, profile architecture, verifiability test | Architecture rejected; null behavior and verifiability adopted |
| Agent tier (×3) | Early/late agent tier specs, core amendments | Folded into AI Integration spec |
| WCOS | Rename + FEEL | Abandoned |

Published specs are more coherent than any individual draft.

---

## 2026-04-16 Design Direction

### The reframed question

The prior framing — "should the WOS kernel grow BPMN-equivalent orchestration patterns so AI can generate a single document?" — is wrong. "BPMN parity" is an expressiveness target that invites scope creep and positions WOS as a competitor to a 20-year-old standard nobody loves authoring.

The right question: **what must WOS be such that AI can generate a complete, executable, governed workflow in one JSON document that runs on a WOS-native runtime and can also export to engine-specific formats (BPMN, Temporal, SCXML) as interop targets?**

Under that framing, BPMN becomes an export path, not a co-authoring artifact.

### Five design axes

Properties the format must deliver. Not borrowed from any reference system.

1. **AI-generability** — closed finite vocabularies, structured diagnostics, canonical forms, schema-enforceable authoring contracts.
2. **Multi-target semantic fidelity** — every normative behavior has a specified semantic rule, not "implementation-defined." Bindings translate faithfully and document what they lose.
3. **Governance as first-class primitive** — deontic operators, due process, review protocols, authority ranking, provenance as native constructs on a declared seam.
4. **Replay determinism over an append-only event stream** — case state derived by folding events, not reactive re-evaluation. Deterministic replay including governance is a spec guarantee.
5. **Long-running version migration** — workflows in flight outlive governance documents. Version pinning and migration policy are first-class.

The axes describe **format properties, not capability inventory** — capability gaps are tracked in the Adopt/Defer lists below.

### Ground truth: WOS today

**Closed:**

- **State kinds** — 4 (`atomic | compound | parallel | final`) — `wos-kernel.schema.json` `$defs/State.properties.type`
- **Action kinds** — 7 (`createTask | invokeService | setData | emitEvent | startTimer | cancelTimer | log`) — `wos-kernel.schema.json:406`
- **Case-field types** — 8 — `wos-kernel.schema.json:617`
- **FEL** — normatively required for guards, milestones, action parameters — `specs/kernel/spec.md §7.4`; kernel imports `fel_core` directly (`crates/wos-core/src/eval.rs:13`)
- **Integration Profile binding types** — 7 — `request-response | event-emit | event-consume | callback | arazzo-sequence | tool | policy-engine`
- **Six extension seams** — `actorExtension | contractHook | provenanceLayer | lifecycleHook | custodyHook | extensions` — `specs/kernel/spec.md §10`

**Open:**

- **`Transition.event`** — free-form string with `$`-prefix convention in prose; eleven kernel-generated events (`$join`, `$timeout.task`, `$timeout.service`, `$timeout.state`, `$timeout.signal`, `$timeout.workflow`, `$error`, `$compensation.complete`, `$related.stateChanged`, `$related.resolved`, `$related.holdReleased`) are closed but user events are untyped. The single load-bearing openness.
- **`HoldPolicy.holdType`**, **`CaseRelationship.type`**, **`AppealMechanism.reviewerConstraint`** — enums in prose, not schema. Conventional. Tracked in #46.
- **`custodyHook`** — `additionalProperties: true`. Trellis escape hatch, intentional.

**Layering — four documents:**

| Concern | Document | Schema |
|---|---|---|
| Structure (state machine topology) | Kernel | `wos-kernel.schema.json` |
| Governance (due process, holds, delegation, workflow policy) | Workflow Governance | `wos-workflow-governance.schema.json` |
| Execution algorithms (compensation, parallel sync, history) | Lifecycle Detail Companion | (prose) |
| Runtime contract (event delivery, durability, ordering) | Runtime Companion | (prose) |

Governance attaches to the kernel via `lifecycleHook` keyed on **semantic tags** (`determination`, `review`, `adverse-decision`, `quality-check`, `intake`, `appeal`, `notification`, `hold`), not transition IDs. Cleanly-separable at the schema level.

**Conformance** — 197 normative constraints catalogued in `LINT-MATRIX.md` (37 static / 55 cross-doc / 105 dynamic). `crates/wos-lint/src/rules/` has 3230+ lines of typed rule code.

**AI-readiness:**

- `x-lm.critical` and `x-lm.intent` annotations on 16+ kernel fields; present in all other schemas
- `.llm.md` files for every spec
- Rule-ID-keyed conformance fixtures (`K-011-*`, `G-030-*`, `AI-041-*`)
- `wos-lint` emits structured diagnostics
- Gaps: no enforcement gate for `x-lm.critical` + `description` + `examples` (→ #34); no normative structured-diagnostics schema; no canonical-forms requirements

**Code-level smells:**

- `wos-core` crate exports `autonomy`, `confidence`, `deontic`, `explain`, `proxy` — L2/3 concerns in kernel crate (`crates/wos-core/src/lib.rs:22-37`)
- `ProvenanceKind` — 93-variant enum spanning kernel through AdvGov (`crates/wos-core/src/provenance.rs`) — shotgun surgery
- `wos-runtime/src/runtime.rs` — 3821 lines, mixed action-kind dispatch
- `wos-formspec-binding` depends on `wos-runtime` (`crates/wos-formspec-binding/Cargo.toml:12`) — dependency inversion
- `impactLevel` (`wos-kernel.schema.json:64-74`) declared in kernel but consumed only by governance (`specs/kernel/spec.md:333`) — decided to stay in kernel per Runtime §2.4 isolation (see #22)
- Kernel fixtures named after Layer 1/2/3 concerns (`autonomy-caps.json`, `deontic-enforcement.json`, `dcr-zone.json`, `due-process.json`) — relocate under #22
- `schemas/kernel/wos-correspondence-metadata.schema.json` self-describes as sidecar but lives under `schemas/kernel/` — relocate under #22
- `DRAFTS/` contains 10+ kernel version proposals (`wos-core-v2` through `v7-kernel`)

**BPMN relationship** — compound/parallel/history are Harel statechart semantics, not BPMN. `specs/kernel/spec.md:636` disavows BPMN topology. Appendix A acknowledges event-taxonomy adoption. #20 will make the event-taxonomy adoption normative (schema-level typed union), not merely informative. Kernel §A: "any durable execution runtime is valid."

### BPMN relationship

After the deltas below, BPMN relates to WOS in three ways:

1. **Export target.** A `wos-bpmn-export` crate translates WOS documents into BPMN 2.0 XML for procurement, compliance, and legacy-tooling consumers. One-way. WOS is the source of truth.
2. **Import adapter.** A `wos-bpmn-import` adapter extracts the lifecycle skeleton (states, transitions, parallel regions) into a WOS kernel document, then requires manual addition of governance. Lossy — WOS governance has no BPMN equivalent to import from.
3. **Interop.** For cross-organizational interchange, BPMN remains the lingua franca.

BPMN stops being a co-authored artifact produced alongside the WOS document. Authoring collapses to WOS.

### Non-goals (rejected paths)

- **Not BPMN parity.** BPMN has ~100 element types, redundant constructs, and is hostile to AI generation. Note: BPMN's event-taxonomy vocabulary IS adopted (informatively today, normatively via #20); topology is not.
- **Not Formspec's 4-phase reactive processing model.** Forms are frozen inputs; workflows are append-only event streams.
- **Not demoting governance to "advisory presentation."** Audit logs, notifications, deadlines are behavioral obligations.
- **Not putting deontic operators in an extension registry.** MUST/MAY/SHALL-NOT are core primitives.
- **Not single-version response pinning.** Cases outlive governance versions.
- **Not eliminating BPMN.** Export target, not authoring surface.

---

## Adopt

### #1 Agent Behavioral Attestations — **Importance 2/10 · Complexity 7/10 · Tech Debt 1/10**

- **Idea:** Optional `attestations` property on agent definitions, each with `issuer`, `subject`, `claims`, `issued`, `expires`, `verificationMethod`. Processor validates claimed autonomy doesn't exceed what attestations support. Lint warns on agents above `manual` in `rights-impacting` workflows without attestations. Provenance records `attestationVerified`.
- **Context:** Currently missing from AI Integration spec. **Zero overlap with Assurance §5** — Assurance covers *identity* attestation (four-field: subject, predicate, basis, validityScope); #1 covers *capability* attestation (SLSA-style: issuer, subject, claims, verificationMethod). The two concepts are orthogonal — neither subsumes the other. Depends on a third-party ecosystem that doesn't exist yet.
- **Source:** v6 §9.6; SLSA pattern (Research R2).
- **Benefits:** Anchors autonomy claims to externally verifiable evidence. Enables separation between "we built this agent" and "this agent is independently audited." Natural fit once SLSA-style ecosystems mature.
- **Importance rationale:** Building trust infrastructure for providers that don't exist. Premature until there's a real issuer ecosystem.
- **Complexity rationale:** New schema object, autonomy-cap validation logic, lint rule, provenance marker, and verification-method semantics. Cross-spec touch (AI Integration schema + prose + lint + provenance) plus unresolved design questions (who issues, how verification works).
- **Tech debt rationale:** Purely additive optional property. Nobody is inventing workarounds in its absence, and waiting actually *reduces* risk — the attestation format should be shaped by whoever builds the issuer ecosystem, not by us guessing.

### #2 Deterministic Adverse-Decision Notice (dual-form) — **Importance 9/10 · Complexity 7/10 · Tech Debt 8/10**

- **Idea:** Adverse-decision notices (Governance §3.2) MUST be produced by a specified deterministic algorithm that derives two co-synchronized outputs from Facts + Reasoning tiers: a **machine-readable artifact** (structured, citable, diffable under audit) and a **human-prose artifact** (plain language, suitable for legal service to the subject). Neither output is model-generated. Identical inputs MUST produce identical outputs in both forms.
- **Scaffolding present — partial** (from 2026-04-16 code-scout validation): `AdverseDecisionPolicy` typed at `wos-workflow-governance.schema.json:231-286` but **no fields are required** (permissive scaffolding). `NoticeTemplate` has **two conflicting definitions** — the Due Process sidecar version is `sections: array of string` only; the Notification Template sidecar version is richly typed (`requiredVariables`, FEL `condition`, typed `contentType`). `noticeTemplateRef` resolution between the two is not validated by any code path. The "processor rejects missing sections" claim is **inaccurate** — what exists is a static lint (G-062 in `crates/wos-lint/src/rules/tier1.rs:780-853`) with heuristic id-matching, plus a hardcoded stub `NoticeSent` provenance emission in `event_handler.rs:72-81`. **Zero runtime rendering code exists.**
- **Remaining work:** the deterministic assembly algorithm (Facts + Reasoning tiers → both forms), the determinism contract (fixtures proving identical inputs → identical outputs), AND a real rendering pipeline (not just a stub emission). The two NoticeTemplate definitions should be reconciled or one removed.
- **Rescoring rationale (2026-04-16 post-validation):** Complexity 6 → 7. Initial rescope assumed rendering scaffolding was in place; it isn't.
- **Context:** Reconciles two previously separate framings — "Dual-Readability Narrative" (v6 §12.7, TODO §4) and "Deterministic Notice" (earlier draft). Sits at Governance §3.2, not in provenance. Distinct from the Narrative tier (AI Integration §13, non-authoritative).
- **Source:** v6 §12.7 + TODO §4 + reconciliation 2026-04-15 + audit 2026-04-16.
- **Benefits:** Serves all four notice users — subject (plain language), attorney (citable artifact matching machine view), auditor (machine-diffable against provenance), implementer (fixtures prove "same facts → same notice in both forms"). Closes the remaining gap in due-process compliance.
- **Importance rationale:** WOS stakes identity on rights-impacting due-process governance. Without this, implementers serve AI-generated text as legal notice or produce bilingual artifacts whose two views silently diverge.
- **Complexity rationale (revised):** The algorithm, determinism invariant, and fixtures. The output surfaces are already specified.
- **Tech debt rationale:** Implementers claiming rights-impacting conformance are using Narrative tier as de-facto legal notice — both non-deterministic AND single-form. Each deployment bakes in a notice pipeline that will have to be replaced.
- **Cross-references:** Builds on #24a (tightened Facts Tier input snapshot). **Delivery mechanism**: Notification Template §4.4 is the downstream consumer — the deterministic algorithm populates the template's sections, respecting FEL-conditional architecture and `requiredVariables` enforcement. Part of the unified #23 → #24a → #2 ADR sequence.

### #3 Policy-Based Migration Routing — **Importance 5/10 · Complexity 6/10 · Tech Debt 4/10**

- **Idea:** `migrationPolicy` enum with four options: `grandfather`, `migrateAll`, `migrateByState`, `expression`. In-flight tasks complete under version-at-creation; workflow advances under new topology after.
- **Context:** Runtime Companion §11 supports manual `migrate` with effectively `grandfather` semantics. **Governance §2.9 already has `schemaUpgrade` with `migrationMechanism` enum (`formspec-changelog | custom-map | declared-equivalence`) and `scope` enum (`instance | workflow | tenant`).** #3 **composes with** §2.9 rather than filling empty space — `migrationMechanism` answers "how was the migration done," `migrationPolicy` answers "when does migration fire for which in-flight instances." Directly realizes axis 5.
- **Source:** v6 §18.2.
- **Open sub-questions:** `tenant`-scope migration behavioral contract — §2.9 declares the enum variant with no normative semantics for who triggers, what isolation, how in-flight cross-tenant instances interact. #3's ADR should own resolving this. Version pinning on provenance records — do `migrateByState` and `expression` update `definitionVersion` at the migration boundary or only for records created after?
- **Benefits:** Lets operators roll out new versions without freezing in-flight work or forcing mass migration. Clear semantics for the four realistic rollout strategies.
- **Importance rationale:** Operational polish rather than new capability.
- **Complexity rationale:** Four policies × semantics × conformance fixtures. `expression` requires FEL evaluation context. `migrateByState` needs state-reference validation and a lint rule.
- **Tech debt rationale:** Operators invent migration scheduling privately via manual `migrate` timing today.

### #12 Capability Preconditions — **Importance 6/10 · Complexity 2/10 · Tech Debt 4/10**

- **Idea:** `preconditions` array on agent capabilities: FEL expressions evaluated before invocation. If unsatisfied, skip the capability and fall through to fallback chain.
- **Context:** Missing from AI Integration spec. Agents are invoked in states where they'll reliably fail — wasting tokens, time, and confidence budget. `Capability` `$def` has four properties (`id`, `description`, `inputContractRef`, `outputContractRef`) — no `preconditions`. `ActionOverride` similarly lacks it.
- **Source:** v4 §9.3.
- **Benefits:** Smallest schema delta of any adoption candidate. Prevents real failure modes. Composes naturally with existing fallback chains.
- **Importance rationale:** Practical ergonomic improvement. Lifting preconditions to declarative FEL makes them inspectable, testable, and enforceable by the processor.
- **Complexity rationale:** Add `preconditions: string[]` to capability schema, specify FEL evaluation context (same as deontic constraints), reuse existing fallback chain for skip behavior, one or two fixtures.
- **Tech debt rationale:** Capability authors embed precondition checks inside capability bodies. Those checks are invisible to `wos-lint`, untestable in isolation, cause agent invocation and contract-validation failure before fallback fires (wasting token spend and confidence budget), and are expensive to migrate to declarative FEL later.

### #13 Verifiability Test Principle (kernel-level) — **Importance 4/10 · Complexity 1/10 · Tech Debt 1/10**

- **Idea:** Add a design-goal bullet in Kernel §1.2 with cross-references in Governance §6.1 and AI Integration §1.2: "Can a second system, given only the spec and definition, cheaply verify behavior was correct?" Frames why 4-layer provenance, processor-enforced guardrails, and calibrated confidence exist.
- **Context:** Task-level verifiability exists (Governance §9.1 Verifiability Matrix). Kernel-level design principle is missing. **Frames axis 4 (replay determinism).** Doc-only change. Verification Report sidecar (`specs/advanced/verification-report.md`) is an artifact of this principle — produces authoring-time immutable records binding to workflow definition, not a replacement for the principle.
- **Source:** v7-proposal §3.
- **Benefits:** Gives future contributors a litmus for proposed features. Explains the "why" behind existing architectural choices.

### #20 Typed Event Meta-Vocabulary (Delta 1) — **Importance 8/10 · Complexity 5/10 · Tech Debt 6/10**

- **Idea:** Replace `Transition.event: string` with `Transition.event: string | TypedEvent` where `TypedEvent` is an enum-tagged union: `{ kind: "timer" | "message" | "signal" | "condition" | "error" | "named", ... }`. The `named` variant wraps today's free-form strings, preserving backward compatibility.
- **Context:** `Transition.event` is the kernel's single load-bearing openness (`wos-kernel.schema.json:348`). Eleven kernel-generated events are already closed; user-authored events are untyped. Subsumes timeout categories from Kernel §9.7; aligns with Integration Profile `event-emit`/`event-consume` bindings.
- **Co-typing requirement:** `Action.event` (used by `startTimer` actions, Kernel §9.2) is a second free-form string that MUST be co-typed with `Transition.event`, or typed events become an untyped back-door for firing arbitrary event names including `$`-reserved ones.
- **Source:** 2026-04-16 orchestration-patterns research; BPMN event taxonomy filtered through CloudEvents.
- **Benefits:** Closes the authoring-surface openness at the transition trigger. Covers ~80% of real governed-workflow patterns without engine-specific authoring.
- **Scope:** Schema change in `wos-kernel.schema.json`; 16 of 17 kernel-fixture migrations; Rust update in `crates/wos-core/src/model/kernel.rs`; lint rule K-007 promoted from runtime check to schema validation; `specs/kernel/spec.md §4.10` update; Appendix A BPMN-event-taxonomy row updated to reflect normative (not informative) adoption; new conformance fixtures per kind.
- **Open sub-questions:** Timer format (ISO 8601 / cron / calendar-aware). **Load-bearing:** the `timer` kind's calendar semantics (wall clock vs. business days) directly determine whether `noticeGracePeriod` (Governance §3.2, ISO 8601 duration) means calendar days or business days — a legal-compliance question. Getting this wrong requires retrofitting every timer-triggered adverse-decision fixture. Message correlation (explicit `correlationKey` vs. case ID). Signal scope (case-local / parent-subcase / global). Interaction with governance hooks (deontic on timer expiry, due-process on message receipt).
- **Tech debt rationale:** Every fixture and spec section authored under the free-form event assumption carries implicit semantics that will need re-verification post-typing.

### #21 First-Class Extension Registry Document (Delta 2) — **Importance 5/10 · Complexity 5/10 · Tech Debt 3/10**

- **Idea:** `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md`. Registry entries for the six named seams, typed event meta-kinds (from #20), Integration Profile binding types, Semantic Profile ontology mappings. Define discovery (well-known URL + config override), lifecycle (draft → stable → deprecated → retired), composition semantics (conflict resolution on same-tag attachment).
- **Context:** Six extension seams exist (Kernel §10) but have no discovery catalog. Processors can't distinguish unknown-but-registered extensions from undeclared ones. Additive — no existing documents change.
- **Source:** 2026-04-16 research; Formspec registry pattern.
- **Open sub-questions:** Two Layer-1 governance docs attaching to the same tag — order resolution? Static catalog vs. runtime service (static recommended). Promotion path from registry entry to core primitive. Deontic operators explicitly NOT in the registry (core-only).

### #22 Crate Split Along Tier Boundaries (Delta 3) — **Importance 6/10 · Complexity 6/10 · Tech Debt 5/10**

- **Idea:** Split `crates/wos-core/` into `wos-kernel | wos-governance | wos-ai | wos-advanced`. Replace monolithic `ProvenanceKind` enum (93 variants) with tier-typed record: `ProvenanceRecord { tier: KernelTier|GovernanceTier|AITier|AdvancedTier, payload: ... }`. Invert `wos-formspec-binding → wos-runtime` to `wos-runtime → wos-formspec-binding`. Split `wos-runtime/src/runtime.rs` along action-kind dispatch. Add CI dependency-fence check analogous to Formspec's `check:deps`.
- **Context:** `wos-core` exports L2/L3 modules. `ProvenanceKind` is shotgun surgery. `wos-runtime` is 3821 lines. Binding→runtime dependency is inverted. Rust-only; no spec or schema changes.
- **In-scope cleanups:** Relocate kernel-fixtures named after Layer 1/2/3 concerns (`autonomy-caps.json`, `deontic-enforcement.json`, `dcr-zone.json`, `due-process.json`) to correct tier directories. Relocate `schemas/kernel/wos-correspondence-metadata.schema.json` to `schemas/sidecars/` — it self-describes as sidecar but lives under kernel.
- **Source:** 2026-04-16 research; Formspec layer-fence precedent.
- **Decided:** `impactLevel` stays in the kernel. Runtime §2.4 requires kernel evaluation to not depend on governance outcome, and `impactLevel` is precisely the kernel signal that gates governance strength — moving it would invert the dependency. The crate split must not duplicate `impactLevel` across `wos-kernel` and `wos-governance`.
- **Open sub-questions:** `custodyHook.additionalProperties: true` — keep as Trellis escape hatch or tighten?

### #23 OverrideRecord Schema — **Importance 7/10 · Complexity 2/10 · Tech Debt 5/10**

- **Idea:** Promote Governance §7.3's three-field requirement (rationale + authority verification + supporting evidence) into an `OverrideRecord` `$def` in `wos-workflow-governance.schema.json`. Enforce at schema-validation time, not just prose.
- **Context:** §7.3 is normative prose; override records resolve to Facts Tier `extensions` (untyped object). No schema-level enforcement. Small, additive.
- **Source:** 2026-04-16 capability validation pass.
- **Benefits:** Due-process compliance becomes schema-checkable rather than prose-hope. Top of Urgency table.
- **Part of unified ADR sequence:** #23 → #24a → #2.

### #24a Mandatory Facts-Tier Input Snapshot — **Importance 8/10 · Complexity 3/10 · Tech Debt 7/10**

- **Idea:** Tighten Facts Tier §8.2: make a typed, structured case-file input snapshot MANDATORY at `determination`-tagged transitions (not the current untyped OPTIONAL `inputs`).
- **Context:** `kernel/spec.md §8.2` currently has `inputs` as OPTIONAL and untyped. Without a guaranteed snapshot, replay determinism (axis 4) cannot be established and the due-process individualized-explanation requirement (Governance §3.3) has no load-bearing data to reference. **Silent dependency of #2** — the deterministic notice algorithm has no complete data to assemble from without this.
- **Source:** 2026-04-16 capability validation pass; audit split from combined #24.
- **Benefits:** Closes the replay-determinism gap in provenance. Small, high-value delta that unblocks #2 and composes with #23.

### #24b Structured Rule-Firing Trace — **Importance 7/10 · Complexity 6/10 · Tech Debt 4/10**

- **Idea:** Add a structured trace of rule evaluation to the Reasoning Tier: ordered list of rules evaluated, intermediate state, and outcome. For AI-assisted decisions, the trace includes which calibration path the confidence report took.
- **Context:** `rulesApplied` (`workflow-governance.md §6.2`) captures *which* rules fired but not evaluation order or intermediate state. **Load-bearing coupling with #25 (defeasibility)** — you cannot specify evaluation order in a trace without knowing whether defeasibility is part of the evaluation. Joint design required.
- **Source:** 2026-04-16 capability validation pass; audit split from combined #24.

### #25 Defeasibility Primitive in Governance — **Importance 7/10 · Complexity 7/10 · Tech Debt 6/10**

- **Idea:** Catala-style default logic as a governance construct. Declared rule priorities with specificity encoding — general rule vs. specific exception, with lexicographic or explicit priority. Not an FEL conditional hand-coding pattern.
- **Context:** Not implemented anywhere in governance or advanced governance. Rule conflict resolution is today a FEL-inside-FEL pattern — exactly the anti-pattern the research corpus flags. Assertion Gate Library (§25-adjacent) has no priority/override/specificity construct either.
- **Source:** 2026-04-16 capability validation pass; Catala (Inria) / LegalRuleML.
- **Open sub-questions:** New `DefeasibleRule` construct or extension of existing rule bundle? Priority encoding — specificity, numeric, or topological over an `overrides` relation? Distinct companion doc (`policy-defeasibility`) or folded into `workflow-governance`? **Load-bearing (1):** composition with `sourceAuthority` rank (`workflow-governance.md §6.2`, rank 1-4). Authority rank and specificity are orthogonal ordering dimensions. Same-rank tie-breaking is unspecified today — must be resolved jointly. **Load-bearing (2):** composition with Integration Profile §11.2 ("a policy engine can restrict, never relax"). When external policy engine denies and a defeasible WOS rule would permit, which wins? Must be specified.

### #26a canRead Enforcement Semantics — **Importance 6/10 · Complexity 3/10 · Tech Debt 4/10**

- **Idea:** Specify normative processor behavior on `AccessControl.canRead(actorId, fieldPath) → false`. Candidates: redact from evaluation context, return `null`, raise error, skip the action. Add conformance fixtures for each branch.
- **Context:** Runtime Companion §12.5 already defines the `canRead` host interface, but the spec provides no MUST/SHOULD statement about processor behavior when it returns `false`. The interface is a no-op stub without semantics. Prerequisite to #26b.
- **Source:** 2026-04-16 audit.

### #26b caseFieldPolicy Schema — **Importance 6/10 · Complexity 6/10 · Tech Debt 4/10**

- **Idea:** `caseFieldPolicy` `$def` in `wos-workflow-governance.schema.json` — per-field read/write scopes by actor role, referencing kernel `caseFile` field identifiers. Composes with `impactLevel` (Genuine Invention #6).
- **Context:** `caseFile` is flat and unpartitioned (`kernel/spec.md §5`). No field-level authorization. L2 `Right` governs agent input entitlements, not human visibility.
- **Decided:** Schema location is governance. The kernel defines the case file; field-level visibility depends on actor roles, impact level, and hold policies (all L1 concepts). Placing it in kernel would violate the Runtime §2.4 isolation invariant.
- **Source:** 2026-04-16 capability validation pass.
- **Open sub-questions:** Compose with L2 `Right` (actor policy first, then agent filter) or supersede? Interaction with hold policies and redacted provenance? **Load-bearing:** `rights-impacting` workflows likely need to forbid AI agents from seeing protected characteristics — composes with AI Integration §6.3 (`agentProvenance`) and §4 (deontic constraints). **Assurance Invariant 6** — disclosure posture and assurance level MUST remain independent predicates. A `caseFieldPolicy` that conflates the two violates it. Design input for the ADR.

### #27 Cancellation Regions — **Importance 4/10 · Complexity 6/10 · Tech Debt 3/10**

- **Idea:** YAWL-style named region: an explicit set of tasks/states spanning arbitrary structural levels, fireable as a unit. Interacts with compensation — cancelled tasks may still need compensation handlers.
- **Context:** `cancellationPolicy ∈ {wait-all, cancel-siblings, fail-fast}` on parallel states (`kernel/spec.md §4.4`) is a join policy, not a named cross-structural scope.
- **Source:** 2026-04-16 capability validation pass; YAWL.
- **Open sub-questions:** Region as state-ID set or predicate? Fired by event, guard, or explicit action? Compensation — run / skip / author's choice?

### #28 Claim-Check Artifact References — **Importance 4/10 · Complexity 4/10 · Tech Debt 3/10**

- **Idea:** Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` usable as a case-field value, with normative integrity-check requirement at retrieval. Either a 9th case-field type or a governance-layer wrapper.
- **Context:** Only one weak precedent exists: `wos-correspondence-metadata.schema.json` `CorrespondenceEntry.contentRef` informally uses the claim-check pattern (URI pointer), but the field is an untyped `string` with no hash. **2026-04-16 code-scout validation REFUTED the earlier "Kernel §8.2 `inputDigest`/`outputDigest` precedent" claim** — those fields exist in spec prose only; zero code, no schema field on `ProvenanceRecord`, no Rust struct member, no population or verification site anywhere in `crates/`. Treat #28 as a **net-new integrity mechanism, not formalization of an existing precedent.**
- **Decided:** Separate the *type* from the *retrieval contract*. The `ExternalArtifactRef` type lives in the kernel — case-field values are kernel-owned. The retrieval contract (who fetches, when, integrity-check enforcement) lives in governance via a `contractHook`.
- **Source:** 2026-04-16 capability validation pass; code-scout validation; Enterprise Integration Patterns.
- **Open sub-questions:** 9th case-field enum variant at kernel, or a kernel `$def` reusable from case-field values? Retrieval contract shape in governance — synchronous processor fetch, deferred-on-access, or action-body responsibility? Should #28 also wire `inputDigest`/`outputDigest` into the `ProvenanceRecord` type (filling the spec-prose-only gap discovered in validation) as part of scope? Arguably yes — same integrity machinery.

### #29a Milestone Spec-Lag Closure (KS.2 observable behavior) — **Importance 5/10 · Complexity 2/10 · Tech Debt 6/10**

- **Idea:** Update Kernel §4.13 prose and Milestone schema to normatively describe KS.2's shipped observable-firing behavior. The spec-runtime drift on the *provenance emission* half is closable right now — the code already has a clear policy.
- **Context (resolved by 2026-04-16 code-scout validation):** Runtime evaluates milestones **once per "write-then-react" envelope** — post-transition-settled (`runtime.rs:602-609`), post-integration-binding (`integration_handlers/*.rs`), post-task-response-merge (`runtime.rs:903-912`). Evaluation is **not per-setData, not a fixed tick, and not a dedicated sync-point primitive.** Deduplication via `fired_milestones` set (`milestones.rs:49-56`). K-M-001 through K-M-005 fixtures prove autonomous provenance emission, dedup, ordering, and integration-binding coverage. **The "open sub-question" about sync-point semantics is answered in code.** This sub-task is pure spec prose + a trivial `triggerMode: "writeSettled"` schema property reflecting the shipped policy.
- **`continuous` mode interaction:** None. `continuous_reevaluate` (`eval_mode.rs:55-115`) is dead code — never called from `wos-runtime`. The "100-cycle convergence cap vs. milestone evaluation" question is moot because continuous mode is unwired.
- **Source:** 2026-04-16 audit; 2026-04-16 code-scout validation.
- **Benefits:** Closes live spec/runtime contradiction. Ships a documented policy that matches what the runtime already does.
- **Part of:** split from former #29 after code-scout validation showed sync-point question was resolved.

### #29b Milestone Reactive Transition Firing (GSM-style) — **Importance 6/10 · Complexity 5/10 · Tech Debt 2/10**

- **Idea:** Add a mechanism for `MilestoneFired` to enqueue an event that transitions can react to, or expose a `$milestone.*` FEL variable usable in guards. This is the "GSM-style artifact-centric progression" half that KS.2 did NOT ship.
- **Context (from 2026-04-16 code-scout validation):** `evaluate_milestones` (`milestones.rs:27-70`) only appends `ProvenanceKind::MilestoneFired` records and updates `fired_milestones`. It does **not** enqueue an event, does **not** mutate case state, does **not** trigger transitions. **No active fixture references a transition conditioned on a milestone.** The token `$milestone.*` appears only in `DRAFTS/wos-core-spec.md:656` — not in active specs or code. Authors today must inject transition-firing events manually after a milestone condition would have been satisfied.
- **Scope:** New schema property on `Milestone` (`firesEvent: string`?) + runtime event-emission logic + FEL variable exposure + conformance fixtures proving "milestone condition satisfied → transition fires without external event injection."
- **Source:** 2026-04-16 audit; 2026-04-16 code-scout validation.
- **Benefits:** Completes GSM-style artifact-centric progression that the research corpus recommended.
- **Open sub-questions:** Event-based (enqueue synthetic event) or guard-based (expose `$milestone.foo` as FEL boolean)? Interaction with existing `fired_milestones` dedup (does re-fire require reset?)? Backward compatibility — existing fixtures assume no autonomous firing.
- **Complexity rationale:** Genuinely new capability touching milestone module, event queue, FEL evaluation context, and fixture corpus.

### #30 WS-HumanTask Lifecycle Completion — **Importance 5/10 · Complexity 4/10 · Tech Debt 4/10**

- **Idea:** Extend the 8-state model (`created | assigned | claimed | completed | failed | delegated | escalated | skipped`) to close the WS-HumanTask gap: add task-level `Suspended` (WOS holds are case-level today), a distinct `Cancelled` terminal (separate from `skipped`, which is "not applicable for this case"), explicit `Return` transition with rework-iteration counter, named forwarding-to-group operation distinct from delegation-to-person.
- **Context:** `specs/governance/workflow-governance.md §10.1` covers the core path with 8 states; extensions are consistent gaps vs. the WS-HumanTask 10-state reference.
- **Source:** 2026-04-16 capability validation pass; WS-HumanTask 1.1.
- **Design note:** Task-level `Suspended` may be reducible to a case-level hold with `holdType: task-suspended`, reusing the existing `lifecycleHook` seam rather than introducing a new attachment point. Open sub-question: is task-level suspension always reducible to case-level hold, or do operational cases need genuinely independent task state (e.g., one task awaits a resource while another task in the same state continues)?

### #31 Jurisdiction-Aware Business Calendar Selection — **Importance 6/10 · Complexity 3/10 · Tech Debt 4/10**

- **Idea:** Runtime resolution of which business calendar applies from a case-file field (e.g., `applicant.jurisdiction`), replacing the current "implementation-defined" selection in `sidecars/business-calendar.md §7`.
- **Context:** Multiple-calendar composition is defined; selection is implementation-defined. Real cross-jurisdiction workflows need a normative selection mechanism.
- **Rescoring rationale (2026-04-16):** Importance 4 → 6 after audit. Implementation-defined calendar selection in a rights-impacting multi-jurisdiction workflow produces a compliance risk: two conformant processors can calculate the same legal deadline differently. Not operational polish.
- **Source:** 2026-04-16 capability validation pass; rescore after opinion pass.
- **Benefits:** Closes the final normative gap in business calendar SLAs.

### #34 `x-lm.critical` Enforcement Gate — **Importance 6/10 · Complexity 1/10 · Tech Debt 2/10**

- **Idea:** CI gate (`docs:check` rule) that rejects schema PRs where an `x-lm.critical: true` node lacks both `description` and at least one `examples` entry. Treat as hard validation, not convention.
- **Context:** `x-lm.critical` is present on 131 nodes across 18 schemas and enforced only by convention. **2026-04-16 code-scout validation: current violation count is ZERO — all 131 nodes carry both description and examples today.** Debt is purely prospective (regression prevention, not remediation). Separate coverage gap surfaced: `schemas/assurance/wos-assurance.schema.json` has zero `x-lm.critical` annotations — see #57.
- **Source:** 2026-04-16 opinion pass; code-scout validation.
- **Benefits:** Locks in a clean invariant. Prevents future schema PRs from introducing AI-generability regressions. Zero dependencies, actionable immediately.
- **Rescoring rationale (2026-04-16 post-validation):** Tech Debt 4 → 2. All currently-critical nodes pass. The gate guards future additions, not current violations.

### #35 Equity Config Enforcement Semantics — **Importance 7/10 · Complexity 5/10 · Tech Debt 5/10**

- **Idea:** Specify processor obligations for `RemediationTrigger.action` values (`review | audit | suspend | notify`); wire `DisparityMethod` evaluation to a runtime schedule per `ReportingSchedule`; define what "suspended workflow" means behaviorally. Add conformance fixtures.
- **Context:** Equity Config sidecar (`equity-config.md` + `wos-equity.schema.json`) is fully specified at structure level: `ProtectedCategory` (demographic grouping → case-file path + expected values), `DisparityMethod` (4 statistical methods: `rateDifference`, `rateRatio`, `standardizedDifference`, `chi2`), `ReportingSchedule` (automated equity reporting), `RemediationTrigger` (escalation on disparity finding). Zero references in IDEA before 2026-04-16 audit; no implementation tracking in TODO. Biggest missing-lead gap in the suite. Applies to human AND AI decisions per spec (civil rights concern, not AI-specific). **Depends on #36.**
- **Source:** 2026-04-16 audit.
- **Benefits:** Makes WOS's civil-rights-compliance claim actionable. Ships first-class statistical disparity monitoring as a processor obligation.

### #36 Equity RemediationTrigger Expression Language — **Importance 6/10 · Complexity 4/10 · Tech Debt 4/10**

- **Idea:** Specify the expression language for `RemediationTrigger.condition`. Schema currently declares untyped `string`; prose examples use constructs FEL can't express (*"disparity > 0.20 for 2 consecutive periods"*). Decide: (a) extend FEL with temporal/windowing operators, (b) introduce restricted DSL for equity conditions, (c) mandate FEL + windowing functions.
- **Context:** Load-bearing prerequisite to #35. Without an expression language, remediation triggers have no conformance target.
- **Source:** 2026-04-16 audit.
- **Risks:** Extending FEL with temporal operators is a grammar change (Kernel §7.4 rejects grammar extensions). A restricted DSL means two expression languages. Windowing functions in FEL is lightest path but requires careful semantics.

### #38 Assertion Library Cross-Document Reference Protocol — **Importance 5/10 · Complexity 3/10 · Tech Debt 3/10**

- **Idea:** Add `assertionId` (or `assertionLibraryRef`) property to `PipelineStage.assertions[]` entries, enabling stages to reference library entries by ID instead of inlining full assertion objects. Specify resolution semantics (named library lookup order, version pinning).
- **Context:** Library defines `AssertionDefinition`; `PipelineStage.assertions` takes inline array; no reference property. The library concept exists in prose; the mechanism that makes it a *library* doesn't.
- **Source:** 2026-04-16 audit.

### #40 Task SLA Authoring Surface — **Importance 6/10 · Complexity 5/10 · Tech Debt 5/10**

- **Idea:** Task SLA configuration — `slaDefinitions` (deadlines), `warningThresholds`, `breachPolicy` (∈ `escalate | reassign | notify | extend`), `escalationChain` — must be authorable in a governance document. Today §10.3 specifies these as normative processor behavior with no schema properties. Explicitly: "not declared in the Workflow Governance Document."
- **Context:** Normative obligations without an authoring surface are half-specs. Implementers must encode task SLAs outside WOS documents. 2026-04-16 audit flagged as structural gap.
- **Source:** 2026-04-16 audit.
- **Open sub-questions:** Schema placement (governance — likely; or kernel, since tasks are case-instance concepts). Composition with #30 (lifecycle extensions) and #31 (jurisdiction-aware calendar).

### #42 Autonomy-Lifecycle Conformance Fixture Batch — **Importance 5/10 · Complexity 2/10 · Tech Debt 3/10**

- **Idea:** Two autonomy-lifecycle-timing fixtures (narrowed 2026-04-16 after code-scout validation): (1) escalation expiry — `EscalationRule.escalationExpiry` (AI Integration §5.4) revokes escalated autonomy on expiry; (2) drift-alert-triggered demotion — Drift Monitor `alertThresholds[]` firing triggers `DemotionRule` from Agent Config (if #37/M-1 not taken, becomes part of that work).
- **Removed from original batch (covered today):** Calibration-expiry is already covered by `ac-001-expired-calibration-cap.json` (rule AC-001); generic demotion is covered by `ai-028-demotion-next-invocation.json` and `ai-029-pending-recalibration.json` for humanOverride-triggered demotion. What's missing is specifically escalation revocation and drift-sourced demotion.
- **Context:** Normative processor obligations without conformance fixtures are invisible conformance holes. Shared pattern: expiry → runtime enforcement → fixture verification.
- **Source:** 2026-04-16 audit; revised after code-scout validation.

### #43 Assurance × Impact-Level Composition Rule — **Importance 6/10 · Complexity 5/10 · Tech Debt 4/10**

- **Idea:** Specify whether a minimum Assurance level (L1–L4) is required for AI-assisted determinations at `rights-impacting` impact level. Neither AI Integration nor Assurance references the other. Must respect Invariant 6 (independence of disclosure posture and assurance level).
- **Context:** Two orthogonal axes both bear on high-stakes decisions with no composition rule. Adversaries can deploy AI-assisted rights-impacting workflows with minimal assurance binding.
- **Source:** 2026-04-16 audit.
- **Options:** (a) Mandate minimum assurance floor per impact level. (b) Require disclosure of chosen assurance level without mandating a floor. (c) Leave implementation-defined (status quo). Recommend (a).

### #45 Sidecar Normative-Contract Audit — **Importance 6/10 · Complexity 5/10 · Tech Debt 5/10**

- **Idea:** Audit all sidecars (Agent Config, Drift Monitor, Assertion Library, Equity Config, Policy Parameters, Due Process Config, Business Calendar, Notification Template, Verification Report) against a three-question rubric: (1) Structure (brief; schema is canonical); (2) Semantics — MUST/SHOULD/MAY per field; (3) Composition — seam attachment, precedence, conflict resolution. Produce a delta per sidecar. Populate Processor Obligations + Composition + Fixture-Coverage sections using the F-2 template.
- **Context:** 2026-04-16 audit found sidecars consistently skip (2) and (3) — pre-schema content masquerading as normative spec. The pattern drives most "no normative binding between X and Y" findings. Addressing sidecars one at a time treats symptoms; this treats the pattern.
- **Source:** 2026-04-16 audit.
- **Interacts with:** Every individual sidecar fix (#35, #36, #38, #39/M-2, #44) — this audit provides the template those fixes populate.

### #46 Schema-Prose Enum Alignment Batch — **Importance 4/10 · Complexity 2/10 · Tech Debt 3/10**

- **Idea:** Batched alignment of schema types with normative prose enums. Items (revised 2026-04-16 after code-scout validation):
  1. `CaseRelationship.type` — prose enum `(parent | child | sibling | related | supersedes)` not in schema (`wos-kernel.schema.json:580`). Add `anyOf` (enum ∪ `x-*` pattern) to preserve extensibility.
  2. `HoldPolicy.holdType` — enum in prose, not schema (`wos-workflow-governance.schema.json:989`). Add with extensibility. **Plus:** §12.2 table, §7.15, and the schema disagree three ways on whether `legal-hold` belongs here — §12.2 omits it, §7.15 treats it as distinct, schema includes it. Reconcile as part of this item.
  3. `AppealMechanism.reviewerConstraint` — §3.5 prose MUSTs independent adjudicator; schema has optional free-form string (`wos-workflow-governance.schema.json:304`). Tighten to required with enum including `independentFromOriginal`. **Plus:** `AppealMechanism.continuationScope` at `:329` has the same shape smell (untyped free-string where prose implies closed vocabulary) — treat together.
  4. `DelegationScope.conditions` — FEL expression with unspecified evaluation context (`wos-workflow-governance.schema.json:963`). Add citation to Runtime §8.2 (matches the pattern used by §3.7 scope fields and AI Integration agent-autonomy FEL fields).
  5. **ISO 8601 duration fields need `pattern`** — `AppealMechanism.appealWindow`, `HoldPolicy.expectedDuration`, and similar ISO-8601 fields across governance schemas declare `"type": "string"` with no format validation. Batch-add a duration pattern.
  6. **Drift Monitor `AlertThreshold` prose table missing** (not schema): `wos-drift-monitor.schema.json:157` already has the complete `$def`; `specs/ai/drift-monitor.md` lacks the matching prose table. Add the prose table so spec/schema are aligned.
- **Removed from original batch:** "HoldPolicy.timeoutAction add `extend`" — mis-scoped; `extend` is a Task SLA `breachPolicy` value for a different lifecycle, not a HoldPolicy concept. "AlertThreshold `$def` completion" — the `$def` exists; gap is prose-side (now covered as item 6).
- **Context:** Each is small and defensible alone; batching prevents pattern from recurring on next schema PR.
- **Source:** 2026-04-16 audit; revised after code-scout validation.

### #47 Provenance Export (PROV-O / OCEL 2.0 / XES) — **Importance 7/10 · Complexity 8/10 · Tech Debt 6/10**

- **Idea:** Serialize internal provenance to W3C PROV-O, OCEL 2.0, and IEEE 1849 XES export formats. 93 `ProvenanceKind` variants already stable; export path missing. Field-level vocabulary mapping tables for each target.
- **Context:** TODO §2 "Foundational" — top zero-dependency work item. Kernel §8.4: "detailed vocabulary mapping is deferred to the Semantic Profile (Phase 3)." This is the deferred work. Unlocks TODO §4 audit products (#48 Merkle chains, #52 simulation trace). Interacts with #24a (input snapshot format must be exportable) and #22 (`ProvenanceKind` restructuring must preserve export mapping).
- **Variant classification (2026-04-16 code-scout validation):** Of the 93 variants: ~48% obvious mapping (lifecycle, timers, task lifecycle, events), ~32% needs design (deontic subsystem has no native PROV vocabulary; autonomy "computed" derivation has no obvious PROV anchor; confidence decay is time-based erosion, not an event; "blocked-action" records handle awkwardly in PROV-O; DCR has no standard export vocabulary), ~20% payload-convention-dependent (`data: Option<Value>` is untyped JSON, export needs per-variant convention work). Empty-timestamp handling is already flagged as export-unsafe.
- **Source:** TODO §2; promoted to IDEA 2026-04-16 audit.
- **Benefits:** Ships industry-standard audit-tool interop without waiting on engine bindings. Unlocks process-mining compatibility.
- **Rescoring rationale (2026-04-16 post-validation):** Complexity 7 → 8. Mapping is ~48% mechanical, rest requires semantic design decisions (especially deontic/autonomy/confidence subsystems). Not a 1-2 week implementation.
- **Options:** (a) All three formats in one spec. (b) Three separate specs. (c) PROV-O only, defer OCEL/XES. Recommend (c) — PROV-O first, the ambiguous subsystems force design work that should land once, not three times.

### #48 Merkle Provenance Chains — **Importance 6/10 · Complexity 6/10 · Tech Debt 4/10**

- **Idea:** Cryptographic hash-chaining for tamper-evident provenance logs. Append-only, signed tree heads, inclusion proofs, consistency proofs. Attaches to Assurance spec via `provenanceLayer` seam.
- **Context:** TODO §4. Matches research corpus R1 (SCITT / RFC 9162 / Certificate Transparency). Assurance §5.2 explicitly excludes cryptographic signing; this is the net-new content. Depends on #47 (stable export format to hash against).
- **Source:** Research R1; TODO §4; promoted 2026-04-16.
- **Options:** (a) Full Certificate Transparency-style transparency service. (b) Hash-chaining only (lighter). (c) Full SCITT receipt integration. Recommend (b) initially.

### #52 Simulation Trace Format — **Importance 4/10 · Complexity 4/10 · Tech Debt 2/10**

- **Idea:** Standardized replay format for simulation runs — validation, tooling, regression testing. Likely reuses XES event log format from #47.
- **Context:** TODO §4. Low complexity if #47 lands first. Enables deterministic replay tooling across implementations.
- **Source:** TODO §4.
- **Options:** (a) Piggyback on XES export. (b) Standalone format. Recommend (a).

### #56 Runtime §2 Isolation-Invariant Lint Rule — **Importance 5/10 · Complexity 2/10 · Tech Debt 3/10**

- **Idea:** Static AST lint rule detecting `setData` → guard dependency cycles in `continuous`-mode documents. Runtime §2.4 isolation invariant is normative but unvalidated; the 100-cycle convergence cap (§10.3) is currently the only backstop.
- **Context:** 2026-04-16 audit found `continuous` mode + `setData` in `onEntry` with guard dependency produces the convergence-cap scenario. No rule in the 197-rule catalogue targets `setData` → guard dependency cycles statically. **Note:** 2026-04-16 code-scout validation found `continuous_reevaluate` is dead code — never called from `wos-runtime`. The lint still matters (prevents defective documents from shipping), but the convergence scenario can't actually manifest today because continuous mode is unwired.
- **Source:** 2026-04-16 audit; refined after code-scout validation.

### #37 Drift Monitor Demotion Policy Binding — **Importance 6/10 · Complexity 3/10 · Tech Debt 5/10**

- **Idea:** Add normative binding from `alertThresholds[].action` (Drift Monitor) to `DemotionRule` (Agent Config). Candidate: `alertThresholds[].policyRef` linking to a named demotion rule. Specify how the binding fires.
- **Context:** Promoted from fallback status to active Adopt after M-1 merge was blocked (2026-04-16 code-scout validation found a live standalone Drift Monitor fixture). The two specs today describe the same operational mechanism (drift detection → autonomy demotion) from opposite sides with no normative link.
- **Source:** 2026-04-16 audit + code-scout validation.
- **Urgency:** (6+5)/3 = 3.67.

### #39 ContinuationPolicy Normative Linkage — **Importance 4/10 · Complexity 2/10 · Tech Debt 3/10**

- **Idea:** Specify how `AppealMechanism.continuationOfServices: true` resolves to a specific `ContinuationPolicy` sidecar entry. Candidate: `AppealMechanism.continuationPolicyRef` or a governance-wide default.
- **Context:** Promoted from fallback status to active Adopt after M-2 merge was rejected (2026-04-16 code-scout validation confirmed non-due-process uses of Notification Template). The sidecar fully specifies `ContinuationPolicy`; nothing connects a setting of `continuationOfServices: true` to a specific policy.
- **Source:** 2026-04-16 audit + code-scout validation.
- **Options:** (a) Explicit `continuationPolicyRef` property (preferred). (b) Implicit governance-wide default. Recommend (a).
- **Urgency:** (4+3)/2 = 3.5.

### #57 Assurance Schema `x-lm.critical` Coverage — **Importance 3/10 · Complexity 1/10 · Tech Debt 2/10**

- **Idea:** Add `x-lm.critical: true` annotations to key nodes in `schemas/assurance/wos-assurance.schema.json`. Today the schema has **zero** such annotations while every other 17 schemas mark critical nodes.
- **Context:** Surfaced by 2026-04-16 code-scout validation during #34 scan. Either Assurance genuinely has nothing spec-critical to flag (unlikely for an assurance layer handling attestations and subject continuity) or the annotations were never added. Either way, #34 gate would have nothing to check in Assurance.
- **Source:** 2026-04-16 code-scout validation.
- **Benefits:** Completes AI-generability coverage across all 18 schemas.
- **Urgency:** (3+2)/1 = 5.0.

---

## Defer

### #4 Tripartite Object Model — **Importance 2/10 · Complexity 9/10 · Tech Debt 3/10**

- **Idea:** Split into ActivityDefinition / WorkflowDefinition / Task as separate documents.
- **Context:** Monolithic works for 146 fixtures + 41 samples. Can be added later as a "packed" layer.
- **Source:** v4-v6.
- **Trigger to re-score:** Activity-definition reuse across workflows becomes a real pattern.

### #5 DAG Processing Model — **Importance 2/10 · Complexity 8/10 · Tech Debt 5/10**

- **Idea:** Rebuild → Recalculate → Re-evaluate → Notify phases.
- **Context:** Event-driven evaluation handles every current test case.
- **Source:** v5-v6.
- **Trigger to re-score:** Event-driven evaluation hits a scale wall.

### #6 Typed Patch Operations — **Importance 1/10 · Complexity 8/10 · Tech Debt 0/10**

- **Idea:** AST-level edits with 4-stage validation.
- **Context:** No authoring tool exists yet.
- **Source:** v4-v6.
- **Trigger to re-score:** Authoring tool starts shipping structural edits.

### #7 OCEL 2.0 Object-Centric Case Model — **Importance 2/10 · Complexity 9/10 · Tech Debt 5/10**

- **Idea:** Typed objects with E2O relationships as *internal case state model* (not export).
- **Context:** OCEL 2.0 already specified as provenance **export** target in Semantic Profile §6.4. The export surface acts as a watching post — systematic lossy mappings surface early.
- **Source:** v4-v5.
- **Trigger to re-score:** Multi-object mutation patterns emerge, or flat→OCEL export shows systematic loss.

### #17 SHACL Cross-Cutting Validation — **Importance 0/10 · Complexity 7/10 · Tech Debt 0/10**

- **Idea:** SHACL shapes for cross-document validation constraints.
- **Context:** Tied to JSON-LD decision (rejected).
- **Source:** v3-v4.

### #32 Multi-Instance Iteration (orchestration primitive) — **Importance 6/10 · Complexity 7/10 · Tech Debt 5/10**

- **Idea:** "For each fired `$X` event, spawn an instance" / "for each element of `caseFile.attachments`." Iteration over events or case-data arrays.
- **Context:** The one orchestration pattern in the 2026-04-16 research with NO viable workaround. Forces engine-specific authoring today. Governance-hooks-per-instance vs. per-iteration is a load-bearing design question — per-iteration generates N governance events for an N-item batch with compliance/audit-trail volume implications.
- **Source:** 2026-04-16 research; BPMN multi-instance markers; workflow patterns.
- **Trigger to re-score:** Waits on #20 (typed events). Then highest-priority deferred item.

### #33 Inclusive-OR Joins / Event-Based Choice / Boundary Events — **Importance 3/10 · Complexity 5/10 · Tech Debt 2/10**

- **Idea:** Remaining orchestration patterns with ugly-but-viable workarounds.
- **Context:** Event-based choice via parallel regions with `fail-fast`; boundary events via parallel regions with event-driven exits; inclusive-OR awkward but encodable.
- **Source:** 2026-04-16 research.
- **Trigger to re-score:** Authoring frustration becomes consistent signal.

### #44 Prospective/Simulation Values in Policy Parameters — **Importance 4/10 · Complexity 3/10 · Tech Debt 2/10**

- **Idea:** Extend `ParameterDefinition.values` with a `prospective` flag or introduce `prospectiveValues`. OpenFisca supports reform simulation natively.
- **Context:** Current spec supports past-and-present effective dates only. Missing for modeling pending-legislation impact.
- **Source:** 2026-04-16 audit.
- **Trigger to re-score:** First reform-simulation consumer appears.

### #49 Engine Adapters (Camunda / Temporal / Step Functions) — **Importance 5/10 · Complexity 8/10 · Tech Debt 3/10**

- **Idea:** External commercial engines act as the WOS runtime — Camunda 8 Worker, Temporal Workflow, AWS Step Functions bridges.
- **Context:** TODO §3. **Orthogonal to Integration Profile** — Integration Profile §3.5–§3.6 cover WOS invoking external systems; engine adapters are the inverse (external engines run WOS). Requires #20 (typed events) first. Three targets with different semantic fits.
- **Source:** TODO §3.
- **Trigger to re-score:** First commercial deployment requesting a specific adapter.

### #50 EU AI Act / OMB M-24-10 Alignment Specs — **Importance 7/10 · Complexity 5/10 · Tech Debt 4/10**

- **Idea:** Normative alignment spec mapping WOS constructs to EU AI Act Art. 13–14 and OMB M-24-10 requirements. Distinct from informative citations already in Governance §1.1.
- **Context:** TODO §5. External-deadline-driven. Citations exist in Governance today; normative alignment with compliance checklists does not.
- **Source:** TODO §5.
- **Trigger to re-score:** Procurement deadline or regulatory inquiry forces earlier action. Watch — may escalate out of Defer without notice.

### #51 Statutory Deadline Chains — **Importance 4/10 · Complexity 7/10 · Tech Debt 3/10**

- **Idea:** Interdependent government deadlines and automated legal consequences. E.g., 30-day notice triggers 10-day response window triggers 5-day finalization period.
- **Context:** TODO §6. Related to #31 (jurisdiction-aware calendar) and `policy-parameters.md`. No chained-deadline construct exists today; achievable via manual FEL authoring, not normative spec.
- **Source:** TODO §6.
- **Trigger to re-score:** First deployment needing chained legal deadlines.

### #53 Full Lifecycle Soundness Verification — **Importance 5/10 · Complexity 9/10 · Tech Debt 3/10**

- **Idea:** Formal methods for kernel lifecycle soundness — linear-time logic verification, Petri-net-style deadlock / livelock / proper-termination proofs. Research corpus recommendation.
- **Context:** TODO parked. Interacts with Verification Report sidecar (artifact) and #13 (design principle).
- **Source:** TODO parked; research corpus (Petri nets, DECLARE).
- **Trigger to re-score:** First deployment with formal-verification procurement requirement.

### #54 JSON Patch for Fine-Grained Provenance — **Importance 2/10 · Complexity 5/10 · Tech Debt 1/10**

- **Idea:** Use JSON Patch semantics for fine-grained case-file mutation in provenance.
- **Context:** TODO parked. Low priority.
- **Source:** TODO parked.

### #55 FEEL-to-FEL Migration Guide — **Importance 2/10 · Complexity 2/10 · Tech Debt 0/10**

- **Idea:** Doc-only migration guide for teams coming from DMN/FEEL to FEL.
- **Context:** TODO parked. Triggers on first DMN-based deployment asking.
- **Source:** TODO parked.
- **Trigger to re-score:** First DMN migration inquiry.

---

## Reject

### #8 FEL Conformance Profiles

- **Idea:** Three FEL dialects (Basic/Standard/Extended).
- **Context:** Kernel §7.4 explicitly rejects grammar extensions.
- **Source:** v5-v6.

### #9 JSON-LD Serialization

- **Idea:** Native JSON-LD wire format.
- **Context:** Plain JSON/YAML is simpler and sufficient.
- **Source:** v3-v5.

### #10 WCOS + FEEL

- **Idea:** Rename to WCOS and switch expression language to FEEL.
- **Context:** Both abandoned.
- **Source:** wcos-lifecycle draft.

### #18 Minimal Governance Envelope

- **Idea:** Strip lifecycle from kernel.
- **Context:** Produces a document that cannot be understood in isolation.
- **Source:** v7.

### #19 FEEL Expression Language

- **Idea:** Use FEEL instead of FEL.
- **Context:** FEEL carries DMN assumptions.
- **Source:** v2-v4.

### BPMN Parity as an Authoring Goal

- **Idea:** Match BPMN's ~100 element types for co-authoring or interchange symmetry.
- **Context:** 2026-04-16 research reframed BPMN as export target. BPMN has redundant constructs and is hostile to AI generation. **Nuance:** topology rejected; event taxonomy adopted (informatively today, normatively via #20).
- **Source:** 2026-04-16 research.

---

## Architectural Decisions Confirmed

- **Constraint zones as overlay**, not kernel state type — implementations don't need DCR understanding.
- **Monolithic document** over tripartite — can decompose later.
- **Event-driven evaluation** over DAG — upgrade when needed.
- **Plain JSON/YAML** over JSON-LD — semantic web as companion layer.
- **FEL** over FEEL — purpose-built.
- **Kernel includes lifecycle** — coherent single-document understanding.
- **18-spec decomposition** over kernel/profile binary — better granularity.
- **Null behavior on deontic constraints** (formerly #11) — `nullBehavior` on Permission/Prohibition/Obligation with impact-level defaults. `specs/ai/ai-integration.md §4.2-4.5 + §5`; `NullBehavior` `$def`.
- **Arazzo integration sequences** (formerly #14) — Integration Profile §3.5:220-251. Shipped via NB.4.
- **Non-HTTP tool invocation** (formerly #15) — `tool` binding kind supporting `command-line`, `batch-file`, `database-procedure`, `graph-query`. Integration Profile §3.6:253-307. Shipped via NB.4.
- **Assist Governance Proxy** (formerly #16) — `ai-integration.md §14`; schema `AssistGovernanceProxy`. Stabilizes with Assist layer. Shipped via NB.4.
- **Hybrid layered architecture with statechart lifecycle** (2026-04-16) — validated against research corpus. Compass recommended Direction 3 (hybrid process + policy + task) with Direction 1's statechart lifecycle — exactly what the four-document WOS architecture implements.
- **BPMN as export target, not authoring surface** (2026-04-16) — authoring collapses to WOS.
- **RFC 9535 `outputBinding` profile** (2026-04-16, shipped NB.2) — explicit inclusion (member access, index, wildcard, slice); exclusion (recursive descent, filter expressions); rejection MUST at load time (lint rule I-001). Integration Profile §3.3.1.
- **CloudEvents correlation key format** (2026-04-16, shipped NB.3) — `{instanceId}:{bindingId}:{invocationId}` as the CloudEvents subject attribute. Stable external interop contract. Integration Profile §6.
- **`finiteDomainDeclarations`** (2026-04-16, shipped AG010) — schema-level domain enumeration suppressing SMT variable-to-variable equality warnings. Advanced Governance `VerifiableConstraint`.
- **`impactLevel` stays in kernel** (2026-04-16) — Runtime §2.4 isolation requires kernel evaluation independent of governance outcome; `impactLevel` is the kernel signal gating governance strength. Moving it would invert dependency.
- **Sidecar keep-separate rationale** (2026-04-16, documented via M-3) — Policy Parameters, Business Calendar, Equity Config, Assurance, Integration Profile, Verification Report, Advanced Governance, Lifecycle Detail/Runtime Companions remain separate sidecars. Rationale: **independent updatability without parent version bump** — regulators updating protected categories, holidays, or benefit thresholds shouldn't force a governance document version bump. Stated explicitly in Equity Config §1.

---

## Structural Merges (pending)

### M-1 · Merge Drift Monitor into Agent Config

**Status:** proposed 2026-04-16. **Blocker confirmed by 2026-04-16 code-scout validation.**

Merge `drift-monitor.md` into `agent-config.md` (or rename to "Agent Lifecycle & Monitoring"). Collapse schemas. Add normative binding from alert thresholds to `DemotionRule`. The two specs describe the same operational mechanism (drift detection → autonomy demotion) from opposite sides with no normative link. Independent-updatability rationale doesn't apply — drift thresholds and demotion targets co-evolve.

**Blocker:** `fixtures/ai/benefits-drift-monitor.json` ships a standalone Drift Monitor **without** an accompanying Agent Config for the same workflow. M-1 can only proceed if that fixture is rewritten into a merged document OR this coupling is accepted retroactively. The "never used independently" precondition is false today.

**Recommendation:** reject M-1 until the benefits-adjudication fixture is revised. Ship **#37 Drift Monitor demotion policy binding** as a standalone Adopt entry now (Imp 6 / Cx 3 / Debt 5 → Urgency 3.67) — promoted to active Adopt below.

### M-2 · Merge Notification Template into Due Process Config — REJECTED

**Status:** proposed 2026-04-16. **Rejected 2026-04-16 after code-scout validation.**

The fixture scope audit prerequisite surfaced standalone non-due-process uses: the single shipped Notification Template fixture (`fixtures/sidecars/benefits-notification-templates.json`) defines templates across **four** distinct categories — `adverse-decision`, `hold-notification`, `appeal-acknowledgment`, `sla-warning`. The spec (`notification-template.md:103-113`) enumerates **six** categories, of which only `adverse-decision` ties to due-process. `HoldPolicy.notificationTemplateRef` (`workflow-governance.md:540`) uses the template sidecar from a non-due-process context.

**Decision:** Do not merge. Ship **#39 ContinuationPolicy normative linkage** (promoted to active Adopt entry below, Imp 4 / Cx 2 / Debt 3 → Urgency 3.5).

### M-3 · Document keep-separate rationale

**Status:** proposed 2026-04-16.

Add rationale preamble to each sidecar: *"This sidecar is separate from its parent because its contents must be updatable without a parent version bump — e.g., regulators update [X] independently of governance structure."* Applies to specs listed in the Architectural Decisions Confirmed entry above.

---

## The Genuine Invention

Capabilities no existing standard covers:

1. **Deontic constraints** — Permissions / Prohibitions / Obligations / Rights (LegalRuleML).
2. **Structured oversight as spec** — not "human reviews" but *how* (independentFirst, considerOpposite).
3. **Due process as structural requirement** — normative notice / explanation / appeal / continuation.
4. **Epistemic status separation** — 4-layer provenance: facts ≠ narrative.
5. **Authority-ranked reasoning traces** — who decided what, under what authority, with what confidence.
6. **Impact-level-dependent behavior** — proportionality: stricter defaults for rights-impacting.
7. **Civil-rights monitoring with statistical primitives** (added 2026-04-16) — `equity-config.md` specifies normative disparity monitoring with 4 statistical methods (`rateDifference`, `rateRatio`, `standardizedDifference`, `chi2`), protected-category schemas, automated reporting schedules, and structured remediation triggers. First-class civil-rights instrument applying to HUMAN AND AI decisions. No existing workflow standard covers this. See #35/#36.
8. **AI model drift as first-class governance primitive** (candidate, added 2026-04-16) — `drift-monitor.md` specifies typed drift metrics, alert thresholds, and automatic autonomy demotion as normative workflow-governance constructs. ML-ops literature has drift detection; normative binding to autonomy demotion in a workflow spec is novel. Pending prior-art confirmation before final inclusion.

Everything else (SCXML lifecycle, DMN decisions, WS-HumanTask tasks, Temporal durability) has known ancestors.

---

## Capability Status vs. Research Corpus

Validated 2026-04-16 against the workflow-standards research (`wos-spec/research/compass_artifact_*.md`) via the `wos-expert` navigation skill. Items the research flags as "missing from all existing systems":

**Implemented — WOS strengths:**

- Temporal parameter versioning (OpenFisca-style) — `specs/governance/policy-parameters.md`
- Separation of duties / four-eyes — *specified* at `workflow-governance.md §7.2`; runtime enforcement currently resides in the `wos-core` monolith and will be separated cleanly via #22.
- Business calendar-aware SLAs — `sidecars/business-calendar.md` (jurisdiction selection pending — #31)
- AI confidence annotations — `ai-integration.md §6.3, §7.1`

**Partial — tracked in Adopt:**

- Decision provenance records — #24a (mandatory input snapshot) + #24b (rule-firing trace)
- Structured override accountability — #23
- WS-HumanTask lifecycle — #30
- GSM artifact-centric progression — #29a (spec-lag closure of KS.2 observable firing) + #29b (reactive transition firing, not yet shipped)
- Civil-rights equity monitoring — `equity-config.md` shipped at structure; enforcement semantics (#35) and expression language (#36) pending
- AI drift governance — `drift-monitor.md` + `agent-config.md` shipped in parts; normative binding pending (M-1)
- Role-scoped visibility — Runtime §12.5 `AccessControl.canRead` interface shipped; enforcement semantics (#26a) and `caseFieldPolicy` (#26b) pending

**Not implemented — tracked in Adopt:**

- Defeasibility / Catala default logic — #25
- Cancellation regions (YAWL) — #27
- Claim-check pattern — #28 (precedents in correspondence metadata + `inputDigest`/`outputDigest`)

**Not implemented — not tracked specifically:**

- Assurance × impact-level composition — #43

---

## Cross-Project Dependencies

WOS depends on specs owned by adjacent projects. Stability of certain WOS constructs is gated by external work:

- **Formspec Assist** — `ai-integration.md §14` Assist Governance Proxy stabilizes only when Assist stabilizes. Schema is fully typed; draft-within-draft status is driven by upstream.
- **Formspec Core** — FEL grammar and evaluation. Kernel §7.4 imports `fel_core` directly (`crates/wos-core/src/eval.rs:13`). Grammar changes in Formspec core propagate to WOS evaluation semantics. See also #36 (equity expression language) — depends on whether FEL grammar extensions are permitted.
- **Trellis** — custodyHook `additionalProperties: true` (`wos-kernel.schema.json:149`) is an intentional escape hatch for Trellis-driven custody declarations. Future Trellis decisions may tighten or extend this.

These aren't blockers but must be tracked: any WOS ADR touching these surfaces should check upstream status.

---

## IDEA ↔ TODO Reconciliation

IDEA_SCRATCH is the design backlog (why/what to build); TODO.md is the execution tracker (what's actively in flight and completed). Significant overlap exists. Cross-references:

| IDEA entry | TODO item | Notes |
|---|---|---|
| #2 Deterministic adverse-decision notice | TODO §4 Audit products | Already cross-referenced from TODO |
| #28 Claim-check artifact references | TODO §6 Interop | Add cross-ref |
| #26a/#26b Role-scoped visibility | TODO §6 Interop "Role-based field visibility" | Add cross-ref |
| #47 Provenance export | TODO §2 Foundational | IDEA tracks design; TODO tracks execution |
| #48 Merkle provenance chains | TODO §4 Audit products | Same |
| #49 Engine adapters | TODO §3 | Same |
| #50 Regulatory alignment specs | TODO §5 | Same |
| #51 Statutory deadlines | TODO §6 | Same |
| #52 Simulation trace format | TODO §4 | Same |
| #53 Full lifecycle soundness | TODO parked | Lifted into IDEA Defer |
| #54 JSON Patch provenance | TODO parked | Same |
| #55 FEEL-to-FEL migration | TODO parked | Same |
| — | TODO §2 Ontology field identity | Not yet tracked in IDEA (unwritten spec `specs/ontology-spec.md`) |
| — | TODO future: Batch / Federation / Learning | Trigger-gated; not yet in IDEA |

TODO has counts dated 2026-04-15; refresh needed to reflect 2026-04-16 work. IDEA's scoring framework and TODO's execution-status model serve different purposes — they should cross-reference, not merge.

---

## Implementation Priority

Each item scored on three 0-10 axes:

- **Importance** — structural impact on the spec's core claims.
- **Complexity** — effort to build now (schema + prose + fixtures + cross-spec touch).
- **Tech Debt** — cost of *not* having it.

Two derived metrics:

- **ROI = Importance / Complexity** — cheapest wins.
- **Urgency = (Importance + Tech Debt) / Complexity** — captures "this matters AND waiting makes it worse."

### Adopt — ranked by Urgency

Scores proposed 2026-04-16, revised after opinion pass and code-scout validation. Not yet peer-reviewed.

| Item | Imp | Cx | Debt | ROI | Urgency | One-line case |
|------|----:|---:|-----:|----:|--------:|---------------|
| **#34 `x-lm.critical` enforcement gate** | 6 | 1 | 2 | 6.0 | 8.0 | CI gate; 0 current violations, pure regression prevention. |
| **#23 OverrideRecord schema** | 7 | 2 | 5 | 3.5 | 6.0 | Due-process compliance, schema-checkable. |
| **#29a Milestone spec-lag closure** | 5 | 2 | 6 | 2.5 | 5.5 | Close live spec/runtime drift; sync-point question resolved in code. |
| **#57 Assurance `x-lm.critical` coverage** | 3 | 1 | 2 | 3.0 | 5.0 | Zero annotations today — fills 18th schema. |
| **#13 Verifiability test principle** | 4 | 1 | 1 | 4.0 | 5.0 | Doc-only, widened placement. |
| **#24a Mandatory Facts-Tier input snapshot** | 8 | 3 | 7 | 2.67 | 5.0 | Unblocks #2 + #23; closes replay-determinism gap. |
| **#12 Capability preconditions** | 6 | 2 | 4 | 3.0 | 5.0 | Small schema delta; saves token spend pre-fallback. |
| **#42 Autonomy-lifecycle fixture batch** | 5 | 2 | 3 | 2.5 | 4.0 | Narrowed to escalation-expiry + drift-triggered demotion. |
| **#56 Runtime §2 isolation lint rule** | 5 | 2 | 3 | 2.5 | 4.0 | Static cycle detector (continuous mode currently unwired). |
| **#37 Drift Monitor demotion binding** | 6 | 3 | 5 | 2.0 | 3.67 | Promoted from M-1 fallback after merge blocker confirmed. |
| **#46 Schema-prose enum alignment batch** | 4 | 2 | 3 | 2.0 | 3.5 | Revised batch after validation. |
| **#39 ContinuationPolicy normative linkage** | 4 | 2 | 3 | 2.0 | 3.5 | Promoted from M-2 fallback after merge rejected. |
| **#26a canRead enforcement semantics** | 6 | 3 | 4 | 2.0 | 3.33 | Interface exists as stub; spec the behavior. |
| **#31 Jurisdiction-aware calendar selection** | 6 | 3 | 4 | 2.0 | 3.33 | Multi-jurisdiction compliance risk. |
| **#20 Typed event meta-vocabulary (Delta 1)** | 8 | 5 | 6 | 1.6 | 2.8 | Closes load-bearing openness. |
| **#38 Assertion library cross-doc reference** | 5 | 3 | 3 | 1.67 | 2.67 | Adds `assertionId` to pipeline stages. |
| **#36 Equity RemediationTrigger expression language** | 6 | 4 | 4 | 1.5 | 2.5 | Prerequisite to #35. |
| **#2 Deterministic adverse-decision notice** | 9 | 7 | 8 | 1.29 | 2.43 | Assembly algorithm + rendering; scaffolding thinner than first framed. |
| **#35 Equity Config enforcement semantics** | 7 | 5 | 5 | 1.4 | 2.4 | Makes civil-rights claim actionable. |
| **#30 WS-HumanTask lifecycle completion** | 5 | 4 | 4 | 1.25 | 2.25 | Adds `Suspended`, `Cancelled`, `Return`. |
| **#45 Sidecar normative-contract audit** | 6 | 5 | 5 | 1.2 | 2.2 | Meta-concern — prevents pattern recurrence. |
| **#40 Task SLA authoring surface** | 6 | 5 | 5 | 1.2 | 2.2 | Add schema for §10.3 SLA definitions. |
| **#43 Assurance × impact-level composition** | 6 | 5 | 4 | 1.2 | 2.0 | Min assurance floor for rights-impacting AI. |
| **#25 Defeasibility primitive** | 7 | 7 | 6 | 1.0 | 1.86 | Closes "general rule + exception" gap. |
| **#22 Crate split (Delta 3)** | 6 | 6 | 5 | 1.0 | 1.83 | Engineering hygiene. |
| **#24b Structured rule-firing trace** | 7 | 6 | 4 | 1.17 | 1.83 | Joint design with #25. |
| **#28 Claim-check artifact references** | 4 | 4 | 3 | 1.0 | 1.75 | Net-new integrity (precedent claim refuted). |
| **#26b caseFieldPolicy schema** | 6 | 6 | 4 | 1.0 | 1.67 | Multi-role confidential cases. |
| **#48 Merkle provenance chains** | 6 | 6 | 4 | 1.0 | 1.67 | Tamper-evident audit logs. |
| **#47 Provenance export (PROV-O / OCEL / XES)** | 7 | 8 | 6 | 0.88 | 1.625 | Deontic/autonomy subsystems require design; not mechanical. |
| **#21 Extension registry (Delta 2)** | 5 | 5 | 3 | 1.0 | 1.6 | Ecosystem discovery. |
| **#29b Milestone reactive firing (GSM-style)** | 6 | 5 | 2 | 1.2 | 1.6 | Genuine new capability split from #29 after validation. |
| **#3 Policy-based migration routing** | 5 | 6 | 4 | 0.83 | 1.5 | Realizes axis 5. |
| **#52 Simulation trace format** | 4 | 4 | 2 | 1.0 | 1.5 | Depends on #47. |
| **#27 Cancellation regions** | 4 | 6 | 3 | 0.67 | 1.17 | YAWL-style regions. |
| **#1 Agent behavioral attestations** | 2 | 7 | 1 | 0.29 | 0.43 | Premature. |

**Suggested order (by Urgency):** #34 → #23 → #29a → #57 → #13 → #24a → #12 → #42 → #56 → #37 → #46 → #39 → #26a → #31 → #20 → #38 → #36 → #2 → #35 → #30 → #45 → #40 → #43 → #25 → #22 → #24b → #28 → #26b → #48 → #47 → #21 → #29b → #3 → #52 → #27 → defer #1.

**Observations after the 2026-04-16 audit + code-scout validation passes:**

- **#34 falls from 10.0 to 8.0** — still top, but Debt reclassified as prospective (zero current violations found across 131 `x-lm.critical` nodes).
- **#29a rises to #3** — the code-scout finding that KS.2's sync-point policy is determinable in code ("once per write-then-react envelope") collapsed the "load-bearing open sub-question" to a small spec prose + schema property update.
- **#29b sinks to low-mid-table** — the reactive-firing half that KS.2 did NOT ship is a genuine new capability, not a drift-closure.
- **#37 and #39 promoted to active Adopt** — M-1 blocked, M-2 rejected. Both ship as standalone binding/linkage entries.
- **#57 new** — Assurance schema has zero `x-lm.critical` annotations; #34 gate has nothing to check there without this.
- **#2 drops to 2.43** — Cx up from 6 to 7 because rendering scaffolding is thinner than first assumed (static lint + stub provenance, no runtime rendering code).
- **#47 drops to 1.625** — Cx up from 7 to 8; deontic/autonomy/confidence subsystems have no mechanical PROV mapping.
- **Equity Config work (#35/#36) sits mid-table** but represents one of the two biggest *missing-lead* items the audit found. Worth explicit prioritization in a Phase-1 push even if Urgency alone doesn't place it at the top.
- **Unified provenance-record-shape ADR sequence (#23 → #24a → #2)** now clusters less tightly because #2 slid down. Still execute together for design coherence.

### Defer — ranked by Tech Debt

| Item | Imp | Cx | Debt | Trigger to re-score |
|------|----:|---:|-----:|---------------------|
| **#32 Multi-instance iteration** | 6 | 7 | 5 | Waits on #20. Highest-priority deferred. |
| **#7 OCEL 2.0 object-centric case** | 2 | 9 | 5 | Multi-object mutation or flat→OCEL export shows loss. |
| **#5 DAG processing model** | 2 | 8 | 5 | Event-driven hits scale wall. |
| **#50 EU AI Act / OMB M-24-10 alignment** | 7 | 5 | 4 | Procurement deadline or regulatory inquiry. May escalate. |
| **#49 Engine adapters (Camunda / Temporal / Step Functions)** | 5 | 8 | 3 | First commercial deployment requesting adapter. Waits on #20. |
| **#4 Tripartite object model** | 2 | 9 | 3 | Activity-definition reuse becomes a real pattern. |
| **#51 Statutory deadline chains** | 4 | 7 | 3 | First deployment needing chained legal deadlines. |
| **#53 Full lifecycle soundness verification** | 5 | 9 | 3 | Formal-verification procurement requirement. |
| **#33 Inclusive-OR / event-choice / boundary events** | 3 | 5 | 2 | Authoring frustration becomes consistent signal. |
| **#44 Prospective/simulation values** | 4 | 3 | 2 | First reform-simulation consumer. |
| **#54 JSON Patch provenance** | 2 | 5 | 1 | Provenance-diff tooling demand. |
| **#6 Typed patch operations** | 1 | 8 | 0 | Authoring tool ships structural edits. |
| **#55 FEEL-to-FEL migration guide** | 2 | 2 | 0 | First DMN migration inquiry. |
| **#17 SHACL cross-cutting validation** | 0 | 7 | 0 | Only if JSON-LD adopted (rejected). |

*Moved to Architectural Decisions Confirmed after 2026-04-15 cross-reference: #14 Arazzo, #15 Non-HTTP tool invocations, #16 Assist Governance Proxy. Moved to Architectural Decisions Confirmed after 2026-04-16 audit: RFC 9535 outputBinding profile, CloudEvents correlation key format, finiteDomainDeclarations, `impactLevel` kernel placement, sidecar keep-separate rationale.*

**Insight:** **#50 regulatory alignment** is the deferred item worth watchful monitoring — external compliance deadlines can force escalation without notice. **#32 multi-instance** unlocks the "single-document" claim once #20 lands.

### Rubrics

**Importance (value):**

- **10** — Blocks a core spec claim or compliance posture.
- **8-9** — Closes a structural gap visible from the spec's stated scope.
- **5-7** — Genuine capability or ergonomic win; not blocking.
- **3-4** — Documentation / pedagogy / clarity only.
- **0-2** — Premature, speculative, or dependent on a nonexistent ecosystem.

**Complexity (cost to build now):**

- **10** — New layer or cross-cutting change touching kernel + multiple companions.
- **7-9** — New schema object, new processor semantics, conformance fixtures, lint rules, cross-spec prose.
- **4-6** — Schema additions with new semantics or enum branches; moderate prose; fixtures.
- **2-3** — Small schema addition reusing existing evaluation machinery; a few fixtures.
- **0-1** — Documentation-only; no schema, no tests.

**Tech Debt (cost of deferring):**

- **9-10** — Delaying bakes wrong abstraction into every downstream spec; retrofit is a breaking change.
- **7-8** — Foundational concept. Implementers actively inventing workarounds; divergent conventions multiplying.
- **5-6** — Debt accrues with the fixture/spec corpus. Mechanical migration but grows expensive over time.
- **3-4** — Bounded debt per unit (e.g., per capability, per operator). Grows slowly.
- **1-2** — Additive feature. Waiting costs almost nothing.
- **0** — Dependency-gated or consumer-free. Waiting is strictly better.

Dropped: #11 — already adopted; see Architectural Decisions Confirmed.

---

## Dependencies and Sequencing

```text
#34 (x-lm.critical gate) ─> no dependencies; ship anytime

#20 (typed events) ──┬─> #32 Multi-instance primitive
                     │   (+ co-type startTimer.event with transition event)
#21 (registry) ──────┼─> Typed event extensions
                     │
#22 (crate split) ───┴─> CI dependency fence ─> WOS→BPMN exporter
                     └─> kernel fixture relocation + correspondence-metadata relocation

          ┌─> #23 (OverrideRecord schema)       ┐
#24a ─────┼─> composes with ────────────────────┼─> #2 (adverse-decision notice)
          │                                      │   (+ Notification Template §4.4 is delivery mechanism)
          └──────────────────────────────────────┘

#25 (defeasibility) ──joint design──> #24b (rule-firing trace)
                      │
                      ├─> composition with sourceAuthority rank (§6.2)
                      └─> composition with Integration Profile §11.2 ("restrict, never relax")

#29a (milestone lag closure) ──> spec prose + schema `triggerMode` property
                              └─> sync-point answer from code: "once per write-then-react envelope"
#29b (milestone reactive firing) ──> genuine new capability; can ship after #29a

#36 (equity expression language) ──prerequisite──> #35 (equity enforcement)

#45 (sidecar audit) ──> template for #35 #36 #38 #39 #44

#47 (provenance export) ──┬──> #48 (Merkle chains)
                          └──> #52 (simulation trace)

#26a (canRead enforcement) ──prerequisite──> #26b (caseFieldPolicy schema)

M-1 (Drift + Agent merge) ──BLOCKED──> ship #37 standalone instead
  Blocker: benefits-adjudication fixture ships standalone Drift Monitor without Agent Config.
M-2 (Due Process + Notification merge) ──REJECTED──> ship #39 standalone
  Reason: Notification Template has confirmed non-due-process uses (hold, appeal-ack, sla-warning).
```

- **#34** ships immediately. Zero dependencies.
- **#20** ships independently. Backward-compatible via `named`. Timer calendar semantics irreversible after fixtures land.
- **#21** depends on #20. Can be scoped smaller (seams only) and shipped first.
- **#22** independent of spec deltas. Blocks any new tier-crossing feature. `impactLevel` stays in kernel — decided.
- **Unified ADR sequence #23 → #24a → #2** — provenance record shape. Don't develop independently.
- **Joint design pass #25 + #24b.** Evaluation-order requires defeasibility answer + authority-rank composition + §11.2 composition.
- **#29** requires axis-4 spec AND sync-point sub-question resolved first.
- **#36 must land before #35** — enforcement without expression language has no conformance target.
- **#47 unlocks the audit-products tier** (#48, #52) and interacts with #24a.
- **#26a must land before #26b** — no `caseFieldPolicy` makes sense without enforcement semantics on the underlying interface.

---

## Open Questions

Design questions not ready for scoring:

1. **Timer semantics precision** (#20). Does `after: P30D` mean 30 calendar days or 30 business days by default? Is the business calendar reference opt-in or required?
2. **Registry composition** (#21). Two Layer-1 governance docs attach rules to the same tag. Declaration order, explicit priority, or conflict rejection?
3. **Multi-instance design** (#32). Iterate over events, over case-data arrays, or both? Governance hooks per-instance or per-iteration (audit-volume implications)? Cardinality constraints?
4. **Version migration declaration surface** (#3). Does the kernel carry the governance version, or does each case instance? How does the author declare migration policy? `tenant`-scope behavior?
5. **Canonical forms.** Should the spec enforce "simple sequential workflow MUST be expressed as a compound state with ordered children, not as a flat sequence"? Elsewhere?
6. **DRAFTS/ resolution.** 10+ kernel version proposals need triage — kept, archived, or deleted — before deltas land on a moving floor.
7. **Defeasibility layer** (#25). `workflow-governance` or distinct `policy-defeasibility` companion? Priority encoding — specificity, numeric, or topological over `overrides`? Composition with `sourceAuthority` rank AND Integration Profile §11.2.
8. **Case-field policy vs. L2 `Right`** (#26b). Compose (actor policy first, then agent filter) or supersede? Interaction with hold policies, redacted provenance, `rights-impacting` AI agent restrictions on protected characteristics, Assurance Invariant 6.
9. **Cancellation region semantics** (#27). State-ID set or predicate? Fired by event, guard, or explicit action? Compensation — run / skip / author's choice?
10. **Milestone reactive firing mechanism** (#29b). Event-based (enqueue synthetic event) or guard-based (expose `$milestone.foo` FEL boolean)? Interaction with existing `fired_milestones` dedup? (The sync-point question from the prior framing is resolved: runtime evaluates "once per write-then-react envelope" per code validation.)
11. **`ExternalArtifactRef` kernel shape** (#28). 9th case-field enum variant or kernel `$def` reusable from case-field values? Retrieval contract shape in governance. Bundled scope: should #28 also wire `inputDigest`/`outputDigest` into `ProvenanceRecord` (filling the spec-prose-only gap)?
12. **Task suspension vs. case-level hold** (#30). `Suspended` always reducible to `holdType: task-suspended`, or operational cases need independent task state?
13. **Equity expression language** (#36). Extend FEL with temporal/windowing operators? Restricted DSL? FEL + windowing functions?
14. **Assurance-level composition** (#43). Mandate minimum assurance floor per impact level, or disclosure-only, or implementation-defined?
15. **Spec preamble template rollout** (F-2). Enforce retroactively via #45 or prospectively on new specs only?
16. **`NoticeTemplate` reconciliation** (#2). Two conflicting definitions exist — due-process sidecar (thin, `sections: array of string`) vs. notification-template sidecar (rich, typed). Reconcile as part of #2, or keep both with `noticeTemplateRef` dispatch?

---

## Convention: Spec Preamble Template (F-2)

Proposed 2026-04-16 as an authoring discipline to prevent pre-schema spec content. Every spec produced or revised going forward SHOULD include:

```markdown
## Normative Contract

This specification defines [X]. A WOS processor conforming to this specification MUST:
- [obligation 1]
- [obligation 2]
- ...

## Composition

This specification attaches to the Kernel via the `[seam]` seam. It composes with
[list of other specs] according to the following precedence: [...].
Conflicts resolve by [...].

## Conformance

See fixtures `[rule-ID pattern]` for normative behavior validation.
```

**Rationale.** 2026-04-16 audit found sidecars consistently skip composition semantics and processor obligations, shipping as pre-schema content. The test: *if you delete the schema, can the spec stand as a normative contract?* If not, it's pre-schema.

**Rollout:** retrofit via #45 (sidecar audit). New specs required to include at authoring time.

---

## Research Corpus

| File | Size | Role |
|------|------|------|
| `compass_artifact...markdown.md` | 26K | 50+ standard survey, 7-layer recommendation |
| `Toward an open...docx` | 48K | Feature taxonomy, DCR/Catala/XES/GSM/OpenFisca discoveries |
| `Agentic AI Integration...docx` | 49K | Agent protocols (MCP, A2A), OWASP/NIST/EU AI Act |
| `AI-Native Workflow Standards...docx` | 28K | BPMN/DMN/CMMN survey (mostly redundant with compass) |
| `prompts/research-prompt.md` | 26K | Research prompt that generated compass |

Research confirms the published architecture is sound: compass recommended Direction 3 (Hybrid + statechart lifecycle) — exactly what was built. Research prompt required agent governance as core landscape — exactly what v2 added.

**Research-identified improvements now tracked:**

- **R1 SCITT transparency service** — cryptographic audit proofs. → #48 Merkle chains.
- **R2 SLSA agent attestations** — "statement + predicate + subjects" pattern. → informs #1.

**Deliberate coverage gaps:** OCEL 2.0 object-centric audit (#7), DCR Graphs constraint zones (extension overlay).

**2026-04-16 validation + audit passes:**

- Validation pass: 12 research-corpus "missing from all systems" capabilities cross-checked via `wos-expert`. 4 implemented, 4 partial, 4 not implemented.
- Opinion pass: surfaced #34 (`x-lm.critical` gate) from Open Questions to Adopt; split #24 into #24a/#24b.
- Four-agent audit pass: surfaced 19+ new/extended proposals, merge candidates, pre-schema quality concern, contradictions including live spec/runtime drift (milestones).

---

## Evidence Summary

Spec-suite citations backing the Ground Truth and capability claims:

| Claim | Source |
|---|---|
| State kinds closed to 4 | `wos-kernel.schema.json` `$defs/State.properties.type`; `specs/kernel/spec.md §4.3` |
| `Transition.event` is free-form string | `wos-kernel.schema.json:348` |
| FEL normatively required for guards | `specs/kernel/spec.md §7.4`; `crates/wos-core/src/eval.rs:13` |
| Kernel-generated events closed set (11) | `specs/kernel/spec.md §4.10` |
| Governance attaches via tag-based `lifecycleHook` | `specs/kernel/spec.md §10.4, §4.12`; `specs/companions/runtime.md §8` |
| Four-document separation | Kernel + `specs/governance/` + `specs/companions/lifecycle-detail.md` + `specs/companions/runtime.md` |
| Three tier systems, 197 constraints | `LINT-MATRIX.md`; `crates/wos-lint/src/rules/` |
| Six formal extension seams | `specs/kernel/spec.md §10` |
| `ProvenanceKind` monolith | `crates/wos-core/src/provenance.rs` (93 variants) |
| `wos-runtime/src/runtime.rs` is 3821 lines | `crates/wos-runtime/src/runtime.rs` |
| `wos-core` exports Layer 2/3 modules | `crates/wos-core/src/lib.rs:22-37` |
| Dependency inversion (binding → runtime) | `crates/wos-formspec-binding/Cargo.toml:12` |
| `impactLevel` in kernel is governance-consumed | `wos-kernel.schema.json:64-74`; `specs/kernel/spec.md:333` |
| 10+ unresolved drafts | `DRAFTS/` |
| BPMN topology rejected, event taxonomy adopted | `specs/kernel/spec.md:636`; Appendix A |
| `x-lm.critical` present, unenforced | `wos-kernel.schema.json` (16 instances); no `docs:check` gate |
| Temporal parameters implemented | `specs/governance/policy-parameters.md` |
| Separation of duties §7.2 | `specs/governance/workflow-governance.md §7.2` |
| Business calendar sidecar | `specs/sidecars/business-calendar.md` |
| AI confidence — ConfidenceReport | `specs/ai/ai-integration.md §6.3, §7.1` |
| Override §7.3 three-field normative | `specs/governance/workflow-governance.md §7.3` |
| Milestones spec/runtime drift (provenance half shipped, reactive half not) | `specs/kernel/spec.md §4.13` (observable-only) vs. KS.2 fixtures K-M-001..005 + `crates/wos-runtime/src/milestones.rs:27-70` + `runtime.rs:602-609, 903-912` |
| Milestone sync-point policy (code-determined) | `crates/wos-runtime/src/runtime.rs:602-609, 903-912`; `integration_handlers/*.rs` |
| `continuous_reevaluate` is dead code | `crates/wos-core/src/eval_mode.rs:55-115` — never called from runtime |
| `AccessControl.canRead` is a stub | `crates/wos-core/src/traits/mod.rs:106-115, 242-258` (default `true`); zero call sites in `wos-runtime` |
| `inputDigest`/`outputDigest` are spec-prose only | `specs/kernel/spec.md:403-411` vs. zero matches in `crates/` for `inputDigest`/`outputDigest` |
| #2 scaffolding thinner than framed | `AdverseDecisionPolicy` has no required fields; NoticeTemplate has two definitions (due-process sidecar = thin; notification-template sidecar = rich); no runtime rendering code; G-062 is a static lint with heuristic id-matching |
| `NoticeSent` is a stub emission | `crates/wos-core/src/event_handler.rs:72-81` |
| `tenant`-scope migration unimplemented | Zero matches for `tenant` or `schemaUpgrade` in `crates/` |
| Facts Tier `inputs` never populated in conformance | 0 of 146 conformance fixtures populate `inputs`; one narrative-only fixture uses it |
| 131 `x-lm.critical` nodes all pass gate today | Scan across 18 schemas |
| Assurance schema has zero `x-lm.critical` | `schemas/assurance/wos-assurance.schema.json` |
| M-1 blocker: standalone Drift Monitor fixture | `fixtures/ai/benefits-drift-monitor.json` ships without matching Agent Config |
| M-2 rejection: Notification Template non-due-process uses | `fixtures/sidecars/benefits-notification-templates.json` — 4 categories; spec §103-113 enumerates 6 |
| §12.2 / §7.15 / schema disagree on `legal-hold` | `workflow-governance.md §12.2, §7.15`; `wos-workflow-governance.schema.json:991` |
| Case-file unpartitioned | `specs/kernel/spec.md §5` |
| `AccessControl.canRead` interface exists | `specs/companions/runtime.md §12.5` |
| Cancellation regions absent | `cancellationPolicy: cancel-siblings` is join-policy only — `specs/kernel/spec.md §4.4` |
| Claim-check precedents | `wos-correspondence-metadata.schema.json` `CorrespondenceEntry.contentRef`; `specs/kernel/spec.md §8.2` `inputDigest`/`outputDigest` |
| Defeasibility absent | Searched `workflow-governance.md`, `advanced-governance.md`, `kernel/spec.md`, `assertion-library.md` — no match |
| Assertion Library has no reference protocol | `assertion-library.md`; `PipelineStage.assertions[]` inline-only |
| `ContinuationPolicy` no parent trigger | `wos-due-process.schema.json` vs. `AppealMechanism.continuationOfServices` |
| Task §10 has no schema properties | `workflow-governance.md §10` ("not declared in the Workflow Governance Document") |
| Equity Config fully specified | `equity-config.md`; `wos-equity.schema.json` |
| Drift Monitor ↔ Agent Config binding missing | `drift-monitor.md §1.5` `action: demoteToAssistive` vs. `agent-config.md §1.4` `DemotionRule` |
| Assurance Invariant 6 | `assurance.md §4.3` |
| `AlertThreshold` `$def` missing | `wos-drift-monitor.schema.json` |
| Integration Profile §11.2 "restrict, never relax" | `specs/profiles/integration.md §11.2` |
| Governance §2.9 `schemaUpgrade` enums | `workflow-governance.md §2.9`; `wos-workflow-governance.schema.json properties/schemaUpgrade` |

---

## Relevant Documents

### In `wos-spec/`

| Document | Relevance |
|---|---|
| `specs/kernel/spec.md` | Lifecycle (§4), FEL (§7.4), seams (§10), reserved events (§4.10), tags (§4.12), milestones (§4.13), cancellation policy (§4.4), Facts Tier (§8.2) |
| `schemas/kernel/wos-kernel.schema.json` | Transition event openness (L348), impactLevel (L64), custodyHook (L149) |
| `specs/governance/workflow-governance.md` | §2.9 schemaUpgrade, §3.2 adverse-decision, §6 reasoning records, §7.2 separation of duties, §7.3 override, §10 tasks, §12 holds |
| `specs/governance/policy-parameters.md` | Temporal parameter versioning |
| `specs/governance/assertion-library.md` | Gate library (reference protocol gap) |
| `specs/governance/due-process-config.md` | NoticeTemplate, AppealRouting, ContinuationPolicy |
| `specs/ai/ai-integration.md` | §6.3 agentProvenance, §7.1 ConfidenceReport, §14 Assist Governance Proxy |
| `specs/ai/agent-config.md` | Capability / ActionOverride / DemotionRule |
| `specs/ai/drift-monitor.md` | Alert thresholds (binding gap) |
| `specs/assurance/assurance.md` | §5 identity attestation, Invariant 6 |
| `specs/advanced/advanced-governance.md` | DCR zones, SMT verification, equity guardrails |
| `specs/advanced/verification-report.md` | SMT verification artifact sidecar |
| `specs/advanced/equity-config.md` | Protected categories, statistical methods, remediation triggers |
| `specs/sidecars/business-calendar.md` | Holidays, operating hours, multi-calendar composition |
| `specs/sidecars/notification-template.md` | FEL-conditional sections, required-section enforcement |
| `specs/companions/lifecycle-detail.md` | Compensation, parallel sync, history, SCXML mapping §7 |
| `specs/companions/runtime.md` | §2 isolation invariants, §8 governance ordering, §9.3 explanation ordering, §10.3 continuous mode, §12.5 AccessControl |
| `specs/profiles/integration.md` | §3.3.1 RFC 9535 outputBinding profile, §6 CloudEvents correlation, §11.2 policy-engine restriction |
| `LINT-MATRIX.md` | 197 normative constraints |
| `fixtures/kernel/` | Minimal and maximal exemplars |
| `crates/wos-core/src/provenance.rs` | Monolithic `ProvenanceKind` enum |
| `DRAFTS/` | Unresolved architectural forks |
| `research/` | Workflow-standards research corpus |

### External reference points (not targets)

| Standard | Relationship to WOS |
|---|---|
| BPMN 2.0.2 | Export target (one-way); topology rejected, event taxonomy adopted |
| SCXML | Interop via bidirectional mapping in Lifecycle Detail §7 |
| Temporal | Reference for durable execution requirements; not a binding target |
| CMMN | Not engaged; case-management concepts absorbed natively |
| WS-HumanTask 1.1 | Reference for task lifecycle; #30 closes 8→10-state gap |
| OpenFisca | Reference for temporal parameters (shipped); reform simulation deferred (#44) |
| Catala / LegalRuleML | Reference for defeasibility (#25) |
| YAWL | Reference for cancellation regions (#27) |
| SLSA | Reference for agent attestations (#1) |
| SCITT / RFC 9162 | Reference for Merkle provenance chains (#48) |
| PROV-O / OCEL 2.0 / IEEE XES | Target formats for provenance export (#47) |
| Formspec core spec | Existence proof; patterns transfer selectively |

---

## Next Steps

1. Ship **#34 `x-lm.critical` enforcement gate** — top Urgency; zero current violations, pure regression prevention.
2. Ship **#29a Milestone spec-lag closure** — live spec/runtime drift; sync-point answer already determined in code ("once per write-then-react envelope"). Spec prose + small schema property update.
3. Triage `DRAFTS/`. The floor must stop moving before deltas land.
4. Ship **#57 Assurance `x-lm.critical` coverage** — trivial addition; closes 18-schema coverage gap.
5. **Unified provenance-record-shape ADR sequence:** #23 (OverrideRecord schema) → #24a (mandatory Facts-Tier input snapshot) → #2 (deterministic adverse-decision notice). One design, three landings. #2 scope includes real rendering pipeline and NoticeTemplate reconciliation (two definitions exist).
6. ADR for **#20 Typed event meta-vocabulary**. Resolve timer calendar-semantics sub-question explicitly — affects `noticeGracePeriod` legal compliance; irreversible once fixtures land. Include `startTimer.event` co-typing.
7. ADR for **#22 Crate split**. `impactLevel` stays in kernel. Fixture + correspondence-metadata relocation included. Parallel with provenance sequence.
8. ADR for **#47 Provenance export** — unlocks #48 and #52. Start with PROV-O only; deontic/autonomy subsystems need design decisions that should land once.
9. Ship fast-and-cheap batch: **#42 Autonomy-lifecycle fixtures** (narrowed to escalation-expiry + drift-triggered demotion), **#56 Runtime §2 isolation lint** (even though continuous mode currently unwired), **#46 schema-prose enum alignment** (revised batch).
10. Ship **#37 Drift Monitor demotion binding** (M-1 blocked) and **#39 ContinuationPolicy linkage** (M-2 rejected). Both promoted to active Adopt.
11. **Joint design pass:** #25 (defeasibility) + #24b (rule-firing trace). Includes `sourceAuthority` rank composition AND Integration Profile §11.2 composition.
12. Ship **#26a canRead enforcement** (prerequisite for #26b). Interface exists as stub; only spec the behavior.
13. Ship **#36 equity expression language** (prerequisite for #35). #35 follows.
14. **#45 Sidecar normative-contract audit** — interleave with individual sidecar fixes, template is F-2.
15. Design pass on **#29b Milestone reactive firing** — genuine new capability, not lag closure. Can ship after #29a lands.
16. Design pass on **#32 Multi-instance iteration** — blocks "AI generates complete workflow in one document." Waits on #20.
17. Defer **#21 Extension registry** until #20 and #22 land.
18. Schedule remaining Adopt entries by Urgency: #31 → #38 → #30 → #43 → #40 → #25 → #28 → #48.
19. Watch **#50 Regulatory alignment** — external deadlines can force escalation.
20. Update **TODO.md** audit date and counts to reflect 2026-04-16 work; add cross-references per IDEA ↔ TODO reconciliation table.
21. Refine Genuine Invention #8 (drift monitoring) — confirm novelty against ML-ops prior art before final inclusion.
22. Revise benefits-adjudication fixture so Drift Monitor + Agent Config are paired (prerequisite to reconsidering M-1).
