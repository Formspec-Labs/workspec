# WOS TODO

**Last audited:** 2026-04-20 (session 7 close — DRAFTS triage archived, K-049 continuous-mode cycle detection, Capability preconditions + AI-057, v0 spike Tasks 4-5 complete, semi-formal review Finding 1 fixed, §4.3a review follow-ups filed + expert-refined)

**Snapshot**

| Metric | Value |
|---|---|
| Specs / schemas | 20 specs · 25 schemas (21 production + 4 meta: conformance / lint / mcp / synth) · 0 SCHEMA-DOC-001 violations across all (`all_production_schemas_have_zero_schema_doc_violations` CI gate) |
| Fixtures | 53 document + 150 conformance (147 top-level + 3 export); T3 green (`kernel_conformance` 133 passed, `trace_parity` 20 passed) |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) + 6 MVP (`wos-authoring` @ 50 tests, `wos-mcp` @ 22 tools, **`wos-synth-core` @ 13 tests, `wos-synth-mock` @ 3, `wos-synth-anthropic` @ 2, `wos-synth-cli`** — DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal \| grep -E 'reqwest\|tokio\|anthropic'`) + 1 spike (`wos-synth-spike` @ 17 tests, keep-with-deletion-horizon) |
| Lint matrix | 102 rules in `LINT-MATRIX.md` (35 T1 · 58 T2 · 9 T3 · 11 Tested · 91 Draft; regenerated from code registries) |
| CI gates | `schema_doc_zero_regression` (all 21 production schemas) · `every_promoted_*_rule_has_executable_or_annotated_evidence` (Tested/LoadBearing) · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Links:** [Core extraction plan](thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Completed archive](COMPLETED.md) · [ADR 0065](../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Next actionable work items (ordered by ROI)

> Session 7 landed 2026-04-20 (8 commits): DRAFTS triage archived (`0d17f9f`); §4.3 #56 K-049 continuous-mode cycle detection (`4fd32e3`); §4.3 #12 capability preconditions + AI-057 (`19ad643`); v0 spike Task 4 conformance gate (`f6320c2`); v0 spike Task 5 retrospective + plan propagation (`a80e37d`); K-049 review Finding 1 fix onEntry/onExit (`2c6a2e2`); §4.3a review follow-ups filed (`64962ea`) and expert-refined (`4ceddb7`). Session 6 items (#2 deterministic adverse-decision notice, §5.4 Task 7 synth-trace schema, §4.2 #21/#39/#37, §4.3 #13/#57) remain uncommitted in the working tree as of 2026-04-20.

1. **§4.1 #20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union. Closes the kernel's last load-bearing openness. DRAFTS triage complete (2026-04-20) — unblocked. Needs design decision on the 5-kind taxonomy + ~175-fixture migration strategy before implementation.
2. **§4.3a K-049 / AI-057 review follow-ups** `[mixed]` — six items with expert-decided approaches (2026-04-20 spec-expert + wos-expert consultations). See §4.3a below. #F5a (kernel `ProvenanceOutcome` enum) is the highest-leverage single item: closes both §3.3.1 `preconditionNotSatisfied` and §10.3 `convergenceCapReached` MUSTs in one schema change.
3. **§4.4 Split release trains** — unblocked since §4.2 close. See [plan](thoughts/plans/2026-04-16-wos-release-trains.md).
4. **§5.5 Synthesis benchmark (`wos-bench`)** — unblocked since §5.4 scaffold complete (Tasks 1–7). See [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective.

---

## Recent session log

Full session-by-session narratives (sessions 2–7) live in [COMPLETED.md](COMPLETED.md). Summary of the closed sessions:

- **Session 7 (2026-04-20, 8 commits)** — DRAFTS triage; K-049 continuous-mode cycle detection; capability preconditions + AI-057; v0 spike Tasks 4–5; review Finding 1 fix; §4.3a follow-ups filed + expert-refined.
- **Session 6 (2026-04-20)** — §5.4 Task 7 synth-trace schema; §4.1 #2 deterministic adverse-decision notice; §4.2 #21/#37/#39; §4.3 #13/#57.
- **Session 5 (2026-04-19, 10 commits)** — §4.2 #37 / #46 / §4.1 #24a closure.
- **Session 4 (2026-04-18, 7 commits)** — wos-synth four-crate scaffold + §4.1 chain unblocking.
- **Session 3 (2026-04-18, ~50 commits)** — parallel-agent close: §5.1 schema description audit, §5.2 structured lint diagnostics, §5.3 trace-emitting conformance, §4.2 rule coverage, wos-mcp Tasks 3–6.
- **Session 2 (2026-04-18)** — code-review closeout of the 2026-04-17/18 parallel-agent batch.

---

## Active work items (2026-04-20)

Legend: 🟡 partial · 🔴 not started · 🚨 has blocker from review

Items that are still open. All landed work is in [COMPLETED.md](COMPLETED.md).

- 🔴 **§4.1 #20 Typed event meta-vocabulary** — see §4.1 below. Replace `Transition.event: string` with a 5-kind typed union. Closes kernel's last load-bearing openness. ~175 fixtures to migrate. Needs design decision on taxonomy before implementation.
- 🟡 **§4.3a K-049 / AI-057 review follow-ups** — six expert-refined items (see §4.3a below). Highest leverage: #F5a kernel `ProvenanceOutcome` enum closes both §3.3.1 (`preconditionNotSatisfied`) and §10.3 (`convergenceCapReached`) MUSTs in one schema change.
- 🔴 **§4.4 Split release trains** — [plan](thoughts/plans/2026-04-16-wos-release-trains.md). Changesets + per-stream git tags mirroring ADR 0063. Unblocked since §4.2 close.
- 🔴 **§5.5 Synthesis benchmark (`wos-bench`)** — [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Unblocked since §5.4 scaffold complete. Live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective.
- 🔴 **§4.2 #22a ProvenanceKind tier-typing** — see §4.2 below. Lock-in pressure relieved by PE.2's exhaustive `audit_layer_for_kind` match; remaining value is data-shape cleanliness. Part of the broader #22 crate split (§4.6).

---

## 1 — Reference implementation blockers

> §1 closed 2026-04-14 — see [COMPLETED.md](COMPLETED.md).

---

## 2 — Foundational (zero external dependencies)

- [x] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES. Landed 2026-04-15 — see [COMPLETED.md § Provenance export](COMPLETED.md#provenance-export-pe).
- [ ] **Ontology field identity** *(design not started — do not sequence as active work)* — `ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and regulatory specs in §6, but cannot be scheduled until the spec is drafted. Prerequisite design work: JSON-LD `@context` decision (see Deferred #9), semantic-field-identity protocol, cross-document alignment mechanism. Move to active only once a draft exists.

---

## 3 — Engine adapters (open question — sequencing unresolved)

> **Status:** sequencing unresolved. TODO previously placed engine adapters as a near-term priority; IDEA_SCRATCH #49 marked them Defer with trigger "first commercial deployment requesting a specific adapter." No arbitrating document. Items kept in the backlog below but **not** scheduled until this question is resolved.

- [ ] **#49a Camunda 8 Worker** `[Imp 5 / Cx 8 / Debt 3]` — Delegate BPMN task execution under WOS governance. Most common BPMN target; broadest external fixture diversity.
- [ ] **#49b Temporal Workflow** `[Imp 5 / Cx 8 / Debt 3]` — Map WOS evaluation steps to deterministic replay. Natural fit with WOS evaluator determinism.
- [ ] **#49c AWS Step Functions** `[Imp 5 / Cx 8 / Debt 3]` — Bridge ASL states to WOS transitions. Broadest commercial reach; narrowest semantic fit.

---

## 4 — Active backlog (priority-ordered)

Previously split across "schema closures" and "behavioral specs." Collapsed and re-sorted 2026-04-16 by cost-to-defer + first-adopter enablement.

### Priority logic and scoring rubric

**Priority logic (2026-04-16 re-sort).** Two goals drive order: (A) reduce architectural lock-in while it's still cheap, (B) make WOS immediately usable by a first real adopter. Items are ranked by cost-to-defer, not cost-to-do. Cheap-and-cheap-forever items are bundled separately so they don't crowd the critical path. The prior Urgency formula from IDEA_SCRATCH (`(Imp+Debt)/Cx`) is retired — it over-rewarded low-Cx regression-prevention items. Scores `[Imp/Cx/Debt]` are preserved per item as metadata — they inform relative weight within each tier but do not override cross-tier ordering.

**Score definitions (0–10 scale):**

- **Imp** — **Importance.** How much does this item move the project forward (architectural leverage, first-adopter enablement, civil-rights/compliance weight). Higher = do it.
- **Cx** — **Complexity.** How much real work (design + implementation + test) this takes. Higher = bigger lift.
- **Debt** — **Architectural tech debt if deferred.** How much extra rework lands later if we don't do it now. Higher = cheaper now than later. Confined-scope fixes score low; load-bearing foundational items (0/N fixtures, unclosed escape hatches) score high.

**Score validation (2026-04-16).** Scores audited in parallel by four code-scout agents against live schemas, specs, crates, and fixtures. Adjustments applied: DRAFTS Debt 7→5, #24a Cx 3→4, #20 Cx 6→7, #46 Cx 2→3, #39 Cx 2→1, #12 Cx 2→3, #56 Debt 3→2, #35 Debt 5→4, #40 Debt 5→4, #30 Cx 4→5, #28 Debt 3→2, Assertion-Library merge Cx 3→2, #22 Cx 6→4, #48 Debt 4→6, #51 Debt 3→5. Factual corrections applied to #22 (runtime.rs lives in wos-runtime at 4451 lines, not wos-core at 3821; binding-inversion already landed), #28 (inputDigest/outputDigest already wired through export crate, not prose-only), #56 (continuous_reevaluate has 4 in-crate test callers, not "dead code").

### 4.1 — Critical path (lock-in + usable)

Items that get materially more expensive if deferred, or that block a first real adopter. The critical-path items closed in sessions 4–7 (DRAFTS triage, #24a Facts-Tier snapshot, #23 OverrideRecord, NoticeTemplate reconciliation, #2 deterministic adverse-decision notice, #31 jurisdiction-aware calendar) are archived in [COMPLETED.md](COMPLETED.md). One item remains:

- [ ] **#20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. No `named` wrapper; no escape hatch. Co-type `Action.event` for `startTimer`. Closes kernel's last load-bearing openness. Migration surface is ~175 fixtures containing `"event":` strings (docs + conformance; larger than originally framed); plus schema + Rust model + K-007 lint promotion to schema validation. DRAFTS triage complete (2026-04-20) — unblocked. **Needs a design decision on the 5-kind taxonomy and migration strategy before implementation.**

### 4.2 — Next (mostly closed — #22a is the only open item)

Five of six §4.2 items landed across sessions 4–6 (#21, #37, #39, #29a, #46 — see [COMPLETED.md](COMPLETED.md)). Only #22a remains:

- [ ] **#22a ProvenanceKind tier-typing** `[Imp 4 / Cx 4 / Debt 3]` *(extracted from #22; re-scored 2026-04-16 post-PE.2)* — Replace the 93-variant `ProvenanceKind` monolith enum (`crates/wos-core/src/provenance.rs`) with a tier-typed record (kernel / governance / ai / advanced). **Debt lowered 5→3:** PE.2 added the `audit_layer` field and an exhaustive `audit_layer_for_kind` match, so new variants must now explicitly declare their tier at compile time — the "ossification" pressure is partly relieved. Remaining value is data-shape cleanliness: separating record payloads by tier so each tier's struct carries only the fields it can populate. Still load-bearing for the broader #22 crate split but no longer urgent. The rest of #22 (directory split, runtime.rs split, CI fence) remains organizational and stays in §4.6.

### 4.3 — Cheap batch — **complete** (2026-04-20)

Low-cost, low-risk batch. All items landed; details in [COMPLETED.md](COMPLETED.md). Summary:

- **#34** — covered by `SCHEMA-DOC-001` + `schema_doc_zero_regression` CI gate (no separate gate needed).
- **#57** — `a1100fe` assurance schema `x-lm.critical` coverage.
- **#13** — `31a0e21` verifiability test principle (Kernel §1.2 + Governance §6.1 + AI §1.2).
- **#12** — `19ad643` capability preconditions (schema + spec §3.3.1 + wos-core model + AI-057 lint).
- **#56** — `4fd32e3` + `2c6a2e2` K-049 continuous-mode cycle detection.
- **#42** — session 5 via AI-AUTO-001 + AI-AUTO-002 (escalation-expiry revocation + drift-alert demotion).

### 4.3a — K-049 / AI-057 review follow-ups (opened 2026-04-20)

From the 2026-04-20 semi-formal review of `4fd32e3` (K-049) and `19ad643` (AI-057). Finding 1 (onEntry/onExit) already fixed in `2c6a2e2`; Findings 6, 8, 9 were OBSERVATION-severity style notes and require no action. Six remaining items, each annotated with the concrete approach agreed via the 2026-04-20 spec-expert / wos-expert consultations. Ordered by dependency (#F5a unblocks #F5b; #F3a unblocks #F3b).

- [ ] **#F2 K-049 switch to structured-path comparison (§3.6.4 reachability)** `[Imp 5 / Cx 4 / Debt 4]` — Status quo violates the dep-graph spec, not a benign false-negative budget: Core §3.6.1 enumerates wildcard + indexed refs as first-class graph vertices, and §3.6.4 makes wildcard a set-cover reachability over indexed refs. **Approach (chosen):** replace the `Option<String>` stem comparison in `fel_analysis::simple_access_path_string` call sites (at least for K-049) with a structured `Vec<Segment>` where `Segment ∈ {Dot(Ident), Index(n), Wildcard}` and compare under a "reachable-via" relation that mirrors §3.6.4: `a[*].b` read reaches `a[N].b` write and vice-versa; `a[0].b` read does NOT reach `a[1].b` write; `a[0].b` read reaches a write to the whole `a` group. Normalize the raw-string `setData.action.path` into the same structured form for symmetric comparison. ~dozen lines beyond the current comparator. Adds K-049 regression fixtures for indexed and wildcard cycles. **Spec citations:** `specs/core/spec.md` §3.6.1 (lines 1388-1409), §3.6.4 (lines 1454-1463); `specs/fel/fel-grammar.md` §6.
- [ ] **#F3a K-049 message reword + `$continuous` fixture** `[Imp 3 / Cx 2 / Debt 2]` — Immediate patch so the diagnostic is honest under today's runtime. **Approach:** reword K-049's message to spec-faithful, implementation-neutral phrasing ("per Runtime §10.3 re-evaluation could thrash against the 100-cycle cap"), and add one K-049 unit test using `"event": "$continuous"` so the rule covers the event shape `eval.rs:412-421` actually re-fires today. **Does not** fix the underlying runtime under-implementation (that is #F3b).
- [ ] **#F3b Runtime §10.3 conformance: rewrite `eval.rs` post-mutation re-scan driver** `[Imp 7 / Cx 6 / Debt 5]` — Runtime §10.3 (`specs/companions/runtime.md:510-524`) normatively says "any case state mutation -- whether from a `setData` action, a contract validation result, or an external signal -- the processor re-evaluates all guards". The current runtime at `crates/wos-core/src/eval.rs:412-421` under-implements this by re-firing only transitions whose event is literally `"$continuous"` — an ad-hoc sentinel the spec never reserves. **Approach:** file an ADR scoping the fix, then rewrite `try_fire_guardless_transition` as a post-mutation re-scan over all transitions in the active configuration, with convergence-cap provenance per `runtime.md:517`. Also adds `ProvenanceKind::ConvergenceCapReached` (currently missing from `crates/wos-core/src/provenance.rs:44`). Depends on #F5a for schema shape.
- [ ] **#F4 AI-058 boolean-AST-root lint for FEL boolean slots** `[Imp 6 / Cx 3 / Debt 3]` — AI-057 only enforces parse validity. Core §4.3.1 / §5.2.1 explicitly type bind/shape slots `→ boolean`; §3.4.3 forbids truthy coercion; so a parse-clean `caseFile.amount` or `"open"` in a boolean slot is an authoring bug. **Approach:** add **AI-058 `fel-precondition-non-boolean`** as a T2 warning that inspects the FEL AST root and emits unless the root is one of `{LogicalOr, LogicalAnd, Not, Equality, Comparison, Membership, Ternary (both branches boolean), BooleanLiteral, IfThenElse, call to a boolean-returning builtin: empty / present / selected / isNumber / isString / isDate / isNull / contains / startsWith / endsWith / matches / valid / relevant / readonly / required}`. Apply to Capability preconditions *and* K-049's guard-read extraction for free via a shared classifier. **Also file upstream against Formspec:** `specs/core/spec.md` §3.8.1 covers null-in-boolean but is silent on non-null-non-boolean-in-boolean — normativity gap worth closing in Formspec proper.
- [ ] **#F5a Kernel `$defs/ProvenanceOutcome` enum + `outcome` property on `FactsTierRecord`** `[Imp 6 / Cx 3 / Debt 5]` — Provenance is a kernel-tier concept: `FactsTierRecord` lives in `schemas/kernel/wos-provenance-record.schema.json` with `additionalProperties: true`, and AI-tier logic writes into the same append-only log. **Approach:** add `$defs/ProvenanceOutcome` as a `string` enum seeded with `["preconditionNotSatisfied", "convergenceCapReached"]` plus an `x-wos.open-enum: true` annotation (or `anyOf: [enum, string]` pattern already used elsewhere) so tier-specific outcomes can extend it; add `outcome` as an OPTIONAL property on `FactsTierRecord` typed by the new enum. Closes both the §3.3.1 MUST (preconditionNotSatisfied) and the §10.3 MUST (convergenceCapReached) in one schema change. Regenerate kernel schema-doc gate.
- [ ] **#F5b AI schema enforces `outcome: preconditionNotSatisfied` via `if/then`** `[Imp 4 / Cx 2 / Debt 2]` — After #F5a lands, add an `if/then` branch in `schemas/ai/wos-ai-integration.schema.json` keyed off the capability-invocation record kind that requires `outcome === "preconditionNotSatisfied"` when the record represents a precondition-blocked invocation. Closes the §3.3.1 point-4 MUST at schema-validation time (not just lint).

**Cross-cutting drift surfaced during the consultation (not in the original review):**

- **`ProvenanceKind::ConvergenceCapReached`** — `specs/companions/runtime.md:517` promises this provenance `recordKind`; `crates/wos-core/src/provenance.rs:44` does not declare the variant. Lands with #F3b or #F5a, whichever ships first.

### 4.4 — Behavioral backlog (after §4.1–§4.3 stabilize)

Specifies processor behavior, governance semantics, or runtime obligations. Not usability-critical, not foundational lock-in — schedule once the critical path and cheap batch have landed. Dependencies noted where they exist.

- [ ] **#26a `AccessControl.canRead` enforcement semantics** `[Imp 6 / Cx 3 / Debt 4]` — Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact / return `null` / raise error / skip action. Conformance fixtures per branch. Interface exists as pure stub today (defaults `true`, zero call sites). **Prerequisite to #26b.**
- [ ] **#26b `caseFieldPolicy` schema** `[Imp 6 / Cx 6 / Debt 4]` — `caseFieldPolicy` `$def` in workflow-governance schema; per-field read/write scopes by actor role. Governance-layer.
- [ ] **#36 Equity RemediationTrigger expression language** `[Imp 6 / Cx 4 / Debt 4]` — FEL extension vs. restricted DSL vs. FEL + windowing. **Prerequisite to #35.**
- [ ] **#35 Equity Config enforcement semantics** `[Imp 7 / Cx 5 / Debt 4]` — Specify processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime per `ReportingSchedule`; define "suspended workflow" behaviorally. Applies to human AND AI decisions. Runtime seam partially in place (`ProvenanceKind::EquityAlert`, lifecycle emission in `event_handler.rs`); behavioral enforcement still absent.
- [ ] **#24b + #25 joint design** *(rule-firing trace + defeasibility)* `[#24b: 7/6/4 · #25: 6/7/6]` — Reasoning Tier gains ordered rule list, intermediate state, outcome; Catala-style default logic with declared rule priorities. Load-bearing coupling — evaluation order requires defeasibility answer. Must compose with `sourceAuthority` rank (§6.2) and Integration Profile §11.2 ("restrict, never relax").
- [ ] **#43 Assurance × impact-level composition rule** `[Imp 6 / Cx 5 / Debt 4]` — Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact. Respect Invariant 6.
- [ ] **#38 Assertion Library cross-document reference protocol** `[Imp 5 / Cx 3 / Debt 3]` — `assertionId` on `PipelineStage.assertions[]`; resolution semantics. The library concept exists in prose; the reference mechanism doesn't.
- [ ] **#40 Task SLA authoring surface** `[Imp 6 / Cx 5 / Debt 4]` — Add schema properties for §10.3 normative prose (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`). Currently spec'd as normative processor behavior with no schema surface. Adjacent scaffolding exists (`sla-warning` category in notification-template schema; SLA-aware business calendar schema), which reduces retrofit cost if deferred.
- [ ] **#30 WS-HumanTask lifecycle completion** `[Imp 5 / Cx 5 / Debt 2]` — Extend 8-state model: task-level `Suspended`, distinct `Cancelled` terminal, explicit `Return` with rework counter, group-forwarding distinct from person-delegation.
- [ ] **#27 Cancellation regions** `[Imp 4 / Cx 6 / Debt 3]` — YAWL-style named region spanning arbitrary structural levels, fireable as a unit. Distinct from existing `cancellationPolicy` join policy.
- [ ] **#28 Claim-check artifact references** `[Imp 4 / Cx 4 / Debt 2]` — Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` as case-field value with normative integrity-check at retrieval. `inputDigest`/`outputDigest` fields are already wired through `ProvenanceRecord` and the export crate (`wos-export/src/{ocel,xes,prov_o}.rs`); remaining work is the `ExternalArtifactRef` type and population/retrieval contract.
- [ ] **#29b Milestone reactive transition firing (GSM-style)** `[Imp 6 / Cx 5 / Debt 2]` — `MilestoneFired` enqueues event, or `$milestone.*` FEL boolean for guards. Ships after #29a.
- [ ] **#3 Policy-based migration routing** `[Imp 5 / Cx 6 / Debt 2]` — `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. Composes with Governance §2.9. **Open sub-questions:** `tenant`-scope behavioral contract undefined (0 code matches); version pinning on provenance records.

### 4.5 — Structural merges (schema consolidation)

Absorbed from IDEA_SCRATCH. Schedule alongside whichever critical-path item naturally touches them.

- [ ] **Assertion Library → Workflow Governance** `[Imp 4 / Cx 2 / Debt 3]` — Absorb as "Named Assertions" section. Library without #38 reference protocol is incomplete; absorb rather than fix. Source is a thin 55-line spec + 139-line schema; merge is mechanical.
- [ ] **Verification Report → Advanced Governance** `[Imp 3 / Cx 2 / Debt 2]` — Absorb as "Output Artifacts" section. Thin sidecar.
- [ ] **Due Process Config partial merge → Workflow Governance** `[Imp 5 / Cx 3 / Debt 4]` (pending #45 step 0) — If thin NoticeTemplate drops (per #2) and AppealRouting + ContinuationPolicy remain, the merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage gap structurally.
- **M-1 Drift Monitor + Agent Config — BLOCKED.** Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone. Ship #37 standalone binding instead; reconsider merge if fixture is revised.
- **M-2 Notification Template + Due Process Config — REJECTED.** 4 non-due-process categories. Ship #39 standalone linkage instead.

### 4.6 — Engineering hygiene (deprioritized)

Organizational debt, not architectural. First adopter won't notice. Schedule when the relevant code is actively being touched for another reason.

- [ ] **#22 Crate split along tier boundaries** `[Imp 5 / Cx 3 / Debt 3]` *(ProvenanceKind tier-typing extracted to §4.2 as #22a)* — Split `wos-core` → `wos-kernel | wos-governance | wos-ai | wos-advanced`. Split `wos-runtime/src/runtime.rs` (now 4451 lines, up from 3821) along action-kind dispatch. Add CI dependency fence. Remaining scope is purely organizational; first adopter won't notice. **Note:** `wos-formspec-binding → wos-runtime` inversion is already landed (`wos-formspec-binding/Cargo.toml:10-13`); `runtime.rs` lives in `wos-runtime`, not `wos-core`.
- [ ] **#45 Sidecar normative-contract audit** `[Imp 6 / Cx 5 / Debt 5]` — Retrofit all sidecars against CONVENTIONS.md: Step 0 (does this sidecar deserve independent existence?) + three-question rubric (Structure / Semantics / Composition).

---

## 5 — Audit and evidence products

Build on the stable provenance export surface from §2. Schedule after §4.1 lands.

- [ ] **#48 Merkle provenance chains** `[Imp 6 / Cx 6 / Debt 6]` — Cryptographic hash-chaining for tamper-evident logs. Attaches via Assurance `provenanceLayer` seam. Hash-chaining only initially; full SCITT / RFC 9162 transparency-service integration as later ADR. **Debt raised:** PROV-O / XES / OCEL exports shipped 2026-04-15 without hash-chain hooks — every adopter of those formats now consumes unlinkable output; retrofitting means versioning three export surfaces simultaneously.
- [ ] **#52 Simulation trace format** `[Imp 4 / Cx 3 / Debt 2]` — Normative replay semantics for simulation runs. Event log format is XES (already shipped via `wos-export::xes`). Remaining work: normative replay contract + conformance fixtures.

---

## 6 — Regulatory alignment

External-deadline-driven. Benefits from ontology (§2) landing first.

- [ ] **#50 EU AI Act alignment** `[Imp 7 / Cx 5 / Debt 4]` — Art. 13–14 alignment spec: draft → 1.0.0. Watchlist — external compliance deadlines can force escalation.
- [ ] **#53 OMB M-24-10 compliance** `[Imp 6 / Cx 4 / Debt 3]` — Compliance support spec: draft → 1.0.0. Narrower than EU AI Act; overlaps existing assurance / impact-level plumbing. More process-documentation-shaped than structural, so Debt is lower.

---

## 7 — Interoperability and speculative research

Pick up when §§2–6 stabilize.

- [ ] **SCXML interoperability** `[Imp 3 / Cx 6 / Debt 2]` — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **#51 Statutory deadline chains** `[Imp 4 / Cx 7 / Debt 5]` — Interdependent government deadlines and automated legal consequences. Architecturally expensive — wrong abstraction here is expensive. **Debt raised:** once #31 jurisdiction-aware calendars and #20 typed events land, deadline chains must compose with both; deferring past those without at least a sketch risks an incompatible construct.

---

## Deferred (with triggers)

Items captured but not active; re-score when the named trigger fires.

| IDEA # | Item | Imp | Cx | Debt | Trigger |
|---|---|---:|---:|---:|---|
| #1 | Agent Behavioral Attestations | 2 | 7 | 1 | SLSA-style AI-agent attestation ecosystem matures OR specific deployment demands capability attestation. |
| #4 | Tripartite Object Model | 2 | 9 | 3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1 | 8 | 0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2 | 9 | 5 | Multi-object mutation patterns emerge, or flat→OCEL export shows systematic semantic loss. |
| #9 | JSON-LD Export Surface | 5 | 5 | 3 | `ontology-spec.md` drafts begin OR shipped PROV-O export pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6 | 7 | 5 | #20 lands. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3 | 5 | 2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions | Sustained deployments above 100 cases/minute |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS |
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding). Reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN-expression-language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; SHACL would duplicate. Shipped PROV-O is JSON-LD; if output-shape validation is needed, scope a dedicated item — don't resurrect SHACL wholesale. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Topology rejected; event taxonomy adopted normatively via #20. |

---

## Parked

- [ ] Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- [ ] JSON Patch for fine-grained provenance.
- [ ] FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open questions

1. **Engine-adapter sequencing** — TODO §3 ↔ IDEA Deferred. Defer until first commercial request, or schedule now to validate runtime against production-shape workloads?
2. **Ontology-spec authoring ownership** — who drafts, when?
3. **Timer semantics** (#20). Wall-clock or business-days for `noticeGracePeriod` legal compliance? Business calendar reference opt-in or required?
4. **Registry composition** (#21). Two L1 governance docs attaching rules to the same tag — declaration order, explicit priority, or conflict rejection?
5. **Multi-instance design** (#32). Events, arrays, or both? Governance hooks per-instance vs. per-iteration.
6. **Version migration declaration surface** (#3). Kernel carries governance version or each case? `tenant`-scope behavioral contract?
7. **Canonical forms.** Enforce "simple sequential workflow MUST be expressed as compound state with ordered children, not flat atomic sequence"?
8. **Defeasibility layer** (#25). `workflow-governance` or distinct companion? Priority encoding? Compose with `sourceAuthority` AND Integration Profile §11.2.
9. **Case-field policy vs. L2 `Right`** (#26b). Compose or supersede? Hold policy interaction. Assurance Invariant 6 independence.
10. **Cancellation region semantics** (#27). Set or predicate? Event / guard / explicit action? Compensation run / skip / author's choice?
11. **`ExternalArtifactRef` shape** (#28). 9th case-field type or `$def`? Retrieval contract — sync / deferred / action-body?
12. **Task suspension reducibility** (#30). Always reducible to `holdType: task-suspended`, or independent task state needed?
13. **Equity expression language** (#36). FEL extension, restricted DSL, or FEL + windowing?
14. **Assurance-level composition** (#43). Minimum floor per impact level, disclosure-only, or implementation-defined?
15. **JSON-LD authoring surface** (Deferred #9). Should `@context` land in authoring or stay export-only?
16. **#29b firing mechanism.** Event-based (enqueue synthetic event) or guard-based (`$milestone.*` FEL boolean)?

---

## Completed

Closed-out work items are archived in [`COMPLETED.md`](COMPLETED.md). New completions should be appended there, not tracked here.

---

## Notes

**ADR references (resolved 2026-04-18).** `ADR-0057 (wos-core-implementation-boundary)` and `ADR-0058 (wos-core-gap-analysis)` live in `thoughts/archive/adr/` (implemented). A prior audit looked only in active `thoughts/adr/` and incorrectly flagged them as missing. Citations in `enterprise-implementation-roadmap.md:257`, `thoughts/plans/2026-04-13-wos-runtime-crate.md:423`, `thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md:302`, and `specs/companions/runtime.md:51,:906` all resolve against the archive copies. No action pending — retained here so future audits don't re-raise the same flag.
