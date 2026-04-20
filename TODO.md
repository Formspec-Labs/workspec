# WOS TODO

**Last audited:** 2026-04-20 (session 8 close — 8-agent parallel dispatch: §4.1 #2 review fixes committed, §4.3a F2/F3a/F4/F5a/F5b all landed, §4.4 release trains Tasks 1-3 + #40 Task SLA + #38 Assertion Library cross-doc refs landed, §4.6 #45 sidecar audit delivered, #20 typed events plan + F3b ADR drafted; 4 semi-formal code reviews still in flight)

**Snapshot**

| Metric | Value |
|---|---|
| Specs / schemas | 20 specs · 25 schemas (21 production + 4 meta: conformance / lint / mcp / synth) · 0 SCHEMA-DOC-001 violations across all (`all_production_schemas_have_zero_schema_doc_violations` CI gate) |
| Fixtures | 53 document + 150 conformance (147 top-level + 3 export); session 8 added 2 K-049 regression fixtures (indexed + wildcard cycles) + 1 SLA authoring happy-path + 1 assertion-library reference fixture |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) + 6 MVP (`wos-authoring` @ 50 tests, `wos-mcp` @ 22 tools, **`wos-synth-core` @ 13 tests, `wos-synth-mock` @ 3, `wos-synth-anthropic` @ 2, `wos-synth-cli`** — DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal \| grep -E 'reqwest\|tokio\|anthropic'`) + 1 spike (`wos-synth-spike` @ 17 tests, keep-with-deletion-horizon) |
| Lint matrix | 103 rules in `LINT-MATRIX.md` (35 T1 · 59 T2 · 9 T3 · 12 Tested · 91 Draft; AI-058 added; K-049 stays Tested pending F3b-driven LoadBearing promotion) |
| Python tests | `pytest tests/schemas/` — 171 passed / 11 skipped / 1 xfailed (+50 vs session 7: +9 ProvenanceOutcome, +4 CapabilityInvocationRecord, +27 Task SLA, +12 AssertionReference minus dedup) |
| CI gates | `schema_doc_zero_regression` (all 21 production schemas) · `every_promoted_*_rule_has_executable_or_annotated_evidence` (Tested/LoadBearing) · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Links:** [Core extraction plan](thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Completed archive](COMPLETED.md) · [ADR 0065](../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Next actionable work items (ordered by ROI)

> Session 8 landed 2026-04-20 (~23 commits across 8 parallel agents): §4.1 #2 review fixes (`02ca0c1` + `a041433` + `25026dd` + `abe3c76`); §4.3a F3a (`e15bd80`) / F4 (`8855591`) / F2 (`ee05cec`) / F5a (`2d890d3`) / F5b (`ae3589f`) plus LINT-MATRIX regen (`d46d172`); §4.4 release trains Tasks 1-3 (`78283ae` + `2c53f62` + `49de6c0` + `9aee9be`); §4.4 #40 Task SLA (`8b466fa` + `bc5de5f` + `130a51e`); §4.4 #38 Assertion Library cross-doc refs (`77695eb` + `f862d1f` + `21e9195`); §4.6 #45 sidecar audit (`9900e39`); #20 typed events plan (`6cad36e`); F3b ADR 0059 (`fcd2c19`). Five of six §4.3a items CLOSED — only F3b remains (implementation; ADR landed).

1. **§4.1 #20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union. **Plan drafted** (`6cad36e`, `thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md`). **Load-bearing open questions:** OQ1 (`$join` disposition) and OQ4 (vendor-kind extension shape) block Task 1. **Actual fixture count: 185 files / 844 occurrences** (higher than original ~175 estimate). ~8-10 engineer-days after OQ1/OQ4 resolved.
2. **§4.3a #F3b Runtime §10.3 conformance** `[Imp 7 / Cx 6 / Debt 5]` — ADR 0059 landed (`fcd2c19`). All preconditions satisfied: F5a schema + `ProvenanceKind::ConvergenceCapReached` variant already exist (`2d890d3`); F2 structured-path K-049 lands the cycle shapes F3b closes. 5 tasks, ~3-5 engineer-days. READY TO EXECUTE.
3. **§4.4 Release trains Tasks 4-5** — Changesets tooling + GitHub Actions release workflow. Tasks 1-3 foundation landed (`78283ae` + `2c53f62` + `49de6c0`). See [plan](thoughts/plans/2026-04-16-wos-release-trains.md).
4. **§5.5 Synthesis benchmark (`wos-bench`)** — unblocked since §5.4 scaffold complete (Tasks 1-7). See [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective.
5. **§4.3b Session-8 review follow-ups** — see §4.3b below. New batch from the in-flight 4 parallel semi-formal reviews; two already returned (D Task SLA, H Assertion Library) with 2 warnings + 2 nits each.

---

## Recent session log

Full session-by-session narratives (sessions 2–7) live in [COMPLETED.md](COMPLETED.md). Summary of the closed sessions:

- **Session 8 (2026-04-20, ~23 commits, 8-agent parallel dispatch)** — §4.1 #2 review fixes committed; §4.3a F2/F3a/F4/F5a/F5b all landed; §4.4 release trains Tasks 1-3 + #40 Task SLA + #38 Assertion Library cross-doc refs; §4.6 #45 sidecar audit delivered; #20 typed events plan; F3b ADR 0059. Four semi-formal code reviews of the session's work in flight; three returned (B/D/H) with WARNING/NIT follow-ups filed under §4.3b.
- **Session 7 (2026-04-20, 8 commits)** — DRAFTS triage; K-049 continuous-mode cycle detection; capability preconditions + AI-057; v0 spike Tasks 4–5; review Finding 1 fix; §4.3a follow-ups filed + expert-refined.
- **Session 6 (2026-04-20)** — §5.4 Task 7 synth-trace schema; §4.1 #2 deterministic adverse-decision notice; §4.2 #21/#37/#39; §4.3 #13/#57.
- **Session 5 (2026-04-19, 10 commits)** — §4.2 #37 / #46 / §4.1 #24a closure.
- **Session 4 (2026-04-18, 7 commits)** — wos-synth four-crate scaffold + §4.1 chain unblocking.
- **Session 3 (2026-04-18, ~50 commits)** — parallel-agent close: §5.1 schema description audit, §5.2 structured lint diagnostics, §5.3 trace-emitting conformance, §4.2 rule coverage, wos-mcp Tasks 3–6.
- **Session 2 (2026-04-18)** — code-review closeout of the 2026-04-17/18 parallel-agent batch.

---

## Active work items (2026-04-20 session 8 close)

Legend: 🟡 partial · 🔴 not started · 🚨 has blocker from review

Items still open. All landed work is in [COMPLETED.md](COMPLETED.md).

- 🔴 **§4.1 #20 Typed event meta-vocabulary** — plan drafted (`6cad36e`). Load-bearing open questions (OQ1 `$join` disposition, OQ4 vendor-kind shape) must be resolved before Task 1. ~8-10 engineer-days once unblocked. 185 fixtures / 844 occurrences to migrate.
- 🔴 **§4.3a #F3b Runtime §10.3 conformance** — ADR 0059 landed (`fcd2c19`). All preconditions satisfied. 5 tasks, ~3-5 engineer-days. **READY TO EXECUTE** — highest-leverage open item.
- 🟡 **§4.3b Session-8 review follow-ups** — see §4.3b. 3 of 4 reviews returned (B/D/H); A wos-lint cluster still running. Combined surface: ~4 WARNINGs + ~8 NITs across Task SLA, Assertion Library, and schema cluster.
- 🟡 **§4.4 Split release trains** — Tasks 1-3 landed; Tasks 4-5 (Changesets tooling + release workflow) open. See [plan](thoughts/plans/2026-04-16-wos-release-trains.md).
- 🔴 **§5.5 Synthesis benchmark (`wos-bench`)** — [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective.
- 🔴 **§4.2 #22a ProvenanceKind tier-typing** — lock-in pressure relieved by PE.2's exhaustive match; remaining value is data-shape cleanliness. Part of the broader #22 crate split (§4.6). Consider bundling with F3b which also touches provenance.rs.
- 🔴 **§4.5 Structural merges** — G's §4.6 #45 audit ratifies three: `assertion-library → workflow-governance` (Cx 2; H's `AssertionUse` seam makes it cheaper), `verification-report → advanced-governance` "Output Artifacts" (Cx 2), `due-process-config` 3-section residual → workflow-governance (Cx 3). See §4.5 below.

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

### 4.3a — K-049 / AI-057 review follow-ups — **5 of 6 closed** (2026-04-20)

Opened after the 2026-04-20 semi-formal review of `4fd32e3` + `19ad643`. Five of six items landed during session 8's 8-agent dispatch; only #F3b remains (ADR done, implementation not yet started). Details of closed items in [COMPLETED.md](COMPLETED.md).

- [x] **#F2 K-049 structured-path comparison** — `ee05cec`. `Vec<Segment>` + `reaches()` §3.6.4 reachability; 2 regression fixtures (indexed + wildcard); helper unit tests.
- [x] **#F3a K-049 message reword + `$continuous` fixture** — `e15bd80`. Message is spec-faithful; `$continuous`-event fixture added.
- [ ] **#F3b Runtime §10.3 conformance — rewrite `eval.rs` post-mutation re-scan driver** `[Imp 7 / Cx 6 / Debt 5]` — **ADR 0059 landed `fcd2c19`**. All preconditions satisfied (F5a's `ProvenanceOutcome` enum + `ProvenanceKind::ConvergenceCapReached` variant both exist). 5 tasks, ~3-5 engineer-days. Ready to execute. See `thoughts/adr/0059-continuous-mode-post-mutation-rescan.md`.
- [x] **#F4 AI-058 boolean-AST-root lint** — `8855591`. `is_boolean_shaped(&Expr)` `pub(super)` in `fel_analysis.rs` for reuse; 3 unit tests.
- [x] **#F5a Kernel `$defs/ProvenanceOutcome`** — `2d890d3`. Open-enum with reserved literals + `x-` vendor pattern; optional `outcome` on `FactsTierRecord`; Rust `ProvenanceKind::ConvergenceCapReached` variant.
- [x] **#F5b AI schema `if/then`** — `ae3589f`. `CapabilityInvocationRecord` $def enforces `outcome = "preconditionNotSatisfied"` when `data.invocationBlocked: true`.

### 4.3b — Session-8 review follow-ups (opened 2026-04-20)

From the 2026-04-20 semi-formal reviews of session 8's parallel-agent landings. Four reviews dispatched; B, D, H returned; A (wos-lint cluster) still in flight. WARNINGs land here; OBSERVATION-severity items (enum-rejection test gaps, style-only deviations) filed as individual tickets or deferred.

**From Review B (schema cluster F5a + F5b):**

- [ ] **#F5c F5a runtime-emission wiring** `[Imp 5 / Cx 2 / Debt 3]` — Review B Finding 3. `ProvenanceRecord` at `crates/wos-core/src/provenance.rs:283-359` has no `outcome` field, and `crates/wos-core/src/eval_mode.rs:78-100` still emits `ProvenanceKind::CaseStateMutation` + `data.convergenceCapReached: true` rather than the newly-declared `ProvenanceKind::ConvergenceCapReached` variant. F5a is schema-only staging until this lands. **Rolls into F3b Task 3** per ADR 0059.
- [ ] **#F5d F5b composition story** `[Imp 4 / Cx 3 / Debt 3]` — Review B Finding 1 + 7. `CapabilityInvocationRecord` is an orphan `$def` — no `$ref` composes it over real provenance output. The `if/then` MUST never fires against real data. Options: (a) ship a conformance composer that loads AI+kernel schemas over provenance logs, (b) move the `if/then` into `wos-provenance-record.schema.json`, or (c) soften the spec prose at `specs/ai/ai-integration.md:159`. Recommend (b) — kernel provenance schema is the single validation point.
- [ ] **#F5e Vendor-extension regex normalization** `[Imp 2 / Cx 1 / Debt 2]` — Review B Finding 2. `^x-[a-zA-Z][a-zA-Z0-9-]*$` in new `ProvenanceOutcome` `$def` diverges from established `^x-[a-z][a-z0-9-]*$` at `wos-kernel.schema.json:816` and `wos-workflow-governance.schema.json:1527`, and from registry canonical `^x-[a-z0-9]+(-[a-z0-9]+)*$` at `wos-extension-registry.schema.json:216`. Align on lowercase-kebab.

**From Review D (§4.4 #40 Task SLA):**

- [ ] **#40a `expectedDuration` `indefinite` semantics** `[Imp 3 / Cx 2 / Debt 2]` — Review D Finding 1. `SlaDefinition.expectedDuration` accepts `"indefinite"` (regex copy from `HoldPolicy`) but `WarningThreshold.beforeBreach` and `EscalationStep.gracePeriod` don't — suggesting the author recognized `indefinite` is nonsense for pre-breach warnings. Either drop `indefinite|` from SLA `expectedDuration` (align with siblings) OR expand the description + add a conformance test that pins processor semantics for indefinite SLAs.
- [ ] **#40b `startEvent` name pattern** `[Imp 3 / Cx 1 / Debt 2]` — Review D Finding 2. `SlaDefinition.startEvent` only requires `type: string, minLength: 1`. Add a lightweight `pattern` matching kernel event-name grammar (reject `$`-prefixed reserved names + whitespace) so common authoring typos fail schema-time, not lint-time.
- [ ] **#40c `EscalationStep.id` vs `escalationChainRef` drift** `[Imp 2 / Cx 1 / Debt 1]` — Review D Finding 6. `BreachPolicy.escalationChainRef` description says "matched by level or id" but `EscalationStep` has no `id` field. Either add OPTIONAL `id` (future-proof) or rewrite the description to say "by level" only.
- Review D Findings 4-5 (NIT: 4 enum rejection tests + `indefinite` fixture branch) filed as test-suite completeness items; land together with #40a.

**From Review H (§4.4 #38 Assertion Library refs):**

- [ ] **#38a Stale `.llm.md` regeneration** `[Imp 2 / Cx 1 / Debt 1]` — Review H Finding 1. `specs/governance/assertion-library.llm.md` is stale (`npm run docs:check` fails). CLAUDE.md "Spec Authoring Contract" mandates `npm run docs:generate` after any schema/spec change. Commit `f862d1f` didn't regen. `workflow-governance.llm.md` and `ai-integration.llm.md` may also need regen.
- [ ] **#38b Cross-schema `$ref` plumbing** `[Imp 3 / Cx 4 / Debt 3]` — Review H Finding 3. "One-line `$ref AssertionUse`" adoption claim understates reality: no cross-schema URI `$ref` exists anywhere in `schemas/`. Adopting `AssertionUse` from `workflow-governance.schema.json` requires either (a) introducing cross-schema URI-ref plumbing (validator base-URI + resolver + test harness) or (b) duplicating the three `$def`s. Either way the §4.5 `assertion-library → workflow-governance` merge is the natural landing point (absorbs the shape cleanly).
- TODO-text currency fix (Review H Finding 2) — #38 entry below is stale post-landing; fix inline this session. G-064 named explicitly.
- Review H Findings 4-8 (OBSERVATION: spec prose clarifications, test coverage nits) captured inline above; no separate tracking.

**From Review A (wos-lint cluster F3a + F4 + F2):**

- [ ] **#F4a AI-058 builtin-allowlist drift** `[Imp 4 / Cx 1 / Debt 2]` — Review A Finding 1. `is_boolean_shaped`'s boolean-returning builtin allowlist at `fel_analysis.rs:197-217` is out of sync with `fel-core`'s `BUILTIN_FUNCTIONS` catalog. Missing three valid entries (`every`, `some`, `boolean`) — false positives on valid preconditions like `every(caseFile.items, $ > 0)`. Includes one bogus entry (`isBoolean`) that isn't a registered builtin at all. **Fix:** derive the allowlist from `builtin_function_catalog()` filtering on `→ boolean` signatures instead of hard-coding. Add coverage tests per builtin.
- [ ] **#F2a Guard-walker short-circuit regression test** `[Imp 2 / Cx 1 / Debt 1]` — Review A Finding 2. The short-circuit at `continuous_mode.rs:398-402` is load-bearing (without it, `PostfixAccess` chains emit spurious stem paths that over-match under prefix reachability). Currently only covered indirectly via `k049_ignores_acyclic_continuous_kernel`. Add a direct test: two transitions reading `caseFile.input` and writing `caseFile.output`, explicit zero-diagnostic assertion + comment citing the short-circuit contract.
- Review A Findings 3-8 (NIT + OBSERVATION): docstring clarifications, `NullCoalesce` admission in `is_boolean_shaped`, adversarial `normalize_setdata_path` test cases, `reaches()` symmetry comment. Defer or bundle with #F4a.

**Open question surfaced across Reviews B + D:** several schemas now carry cross-reference properties (`calendarRef`, `templateRef`, `escalationChainRef`, `assertionRef`) with inconsistent shape conventions (URI vs. local-id vs. URN). Worth a separate "Cross-reference shape conventions" ADR to pin the distinction and prevent future drift.

### 4.4 — Behavioral backlog (after §4.1–§4.3 stabilize)

Specifies processor behavior, governance semantics, or runtime obligations. Not usability-critical, not foundational lock-in — schedule once the critical path and cheap batch have landed. Dependencies noted where they exist.

- [ ] **#26a `AccessControl.canRead` enforcement semantics** `[Imp 6 / Cx 3 / Debt 4]` — Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact / return `null` / raise error / skip action. Conformance fixtures per branch. Interface exists as pure stub today (defaults `true`, zero call sites). **Prerequisite to #26b.**
- [ ] **#26b `caseFieldPolicy` schema** `[Imp 6 / Cx 6 / Debt 4]` — `caseFieldPolicy` `$def` in workflow-governance schema; per-field read/write scopes by actor role. Governance-layer.
- [ ] **#36 Equity RemediationTrigger expression language** `[Imp 6 / Cx 4 / Debt 4]` — FEL extension vs. restricted DSL vs. FEL + windowing. **Prerequisite to #35.**
- [ ] **#35 Equity Config enforcement semantics** `[Imp 7 / Cx 5 / Debt 4]` — Specify processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime per `ReportingSchedule`; define "suspended workflow" behaviorally. Applies to human AND AI decisions. Runtime seam partially in place (`ProvenanceKind::EquityAlert`, lifecycle emission in `event_handler.rs`); behavioral enforcement still absent.
- [ ] **#24b + #25 joint design** *(rule-firing trace + defeasibility)* `[#24b: 7/6/4 · #25: 6/7/6]` — Reasoning Tier gains ordered rule list, intermediate state, outcome; Catala-style default logic with declared rule priorities. Load-bearing coupling — evaluation order requires defeasibility answer. Must compose with `sourceAuthority` rank (§6.2) and Integration Profile §11.2 ("restrict, never relax").
- [ ] **#43 Assurance × impact-level composition rule** `[Imp 6 / Cx 5 / Debt 4]` — Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact. Respect Invariant 6.
- [x] **#38 Assertion Library cross-document reference protocol** — session 8 (`77695eb` + `f862d1f` + `21e9195`). `AssertionReference` / `AssertionInlineUse` / `AssertionUse` three-$def split in `wos-assertion-gate.schema.json`; hybrid-mix rejection via `oneOf` + `additionalProperties: false`; spec §2.3 resolution semantics; G-064 designed but not implemented. **Follow-ups in §4.3b:** #38a stale `.llm.md` regen, #38b cross-schema `$ref` plumbing claim needs honest framing.
- [x] **#40 Task SLA authoring surface** — session 8 (`8b466fa` + `bc5de5f` + `130a51e`). Four OPTIONAL properties on `TaskPattern` (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`); §10.4 spec subsection; 27-case contract test file; happy-path fixture. **Follow-ups in §4.3b:** #40a `expectedDuration` `indefinite` semantics, #40b `startEvent` pattern, #40c `EscalationStep.id` ↔ `escalationChainRef` drift.
- [ ] **#30 WS-HumanTask lifecycle completion** `[Imp 5 / Cx 5 / Debt 2]` — Extend 8-state model: task-level `Suspended`, distinct `Cancelled` terminal, explicit `Return` with rework counter, group-forwarding distinct from person-delegation.
- [ ] **#27 Cancellation regions** `[Imp 4 / Cx 6 / Debt 3]` — YAWL-style named region spanning arbitrary structural levels, fireable as a unit. Distinct from existing `cancellationPolicy` join policy.
- [ ] **#28 Claim-check artifact references** `[Imp 4 / Cx 4 / Debt 2]` — Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` as case-field value with normative integrity-check at retrieval. `inputDigest`/`outputDigest` fields are already wired through `ProvenanceRecord` and the export crate (`wos-export/src/{ocel,xes,prov_o}.rs`); remaining work is the `ExternalArtifactRef` type and population/retrieval contract.
- [ ] **#29b Milestone reactive transition firing (GSM-style)** `[Imp 6 / Cx 5 / Debt 2]` — `MilestoneFired` enqueues event, or `$milestone.*` FEL boolean for guards. Ships after #29a.
- [ ] **#3 Policy-based migration routing** `[Imp 5 / Cx 6 / Debt 2]` — `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. Composes with Governance §2.9. **Open sub-questions:** `tenant`-scope behavioral contract undefined (0 code matches); version pinning on provenance records.

### 4.5 — Structural merges (schema consolidation)

Ratified by G's §4.6 #45 sidecar audit (`9900e39`, 2026-04-20) — see `thoughts/reviews/2026-04-20-sidecar-contract-audit.md`. All three merges below: KEEP verdict withdrawn, MERGE verdict confirmed by the Step-0 + three-question rubric. Schedule alongside whichever critical-path item naturally touches them.

- [ ] **Assertion Library → Workflow Governance** `[Imp 4 / Cx 2 / Debt 3]` — Absorb as "Named Assertions" section. #38 shape-layer already landed (session 8) via the `AssertionUse` seam, so the merge is purely mechanical file-move: the three `$def`s travel into `workflow-governance.schema.json`, consumers drop the cross-schema `$ref` problem entirely. Audit + H's Review Finding 9: `AssertionUse` seam makes this merge CHEAPER, not harder. **Prerequisite:** cross-schema `$ref` plumbing decision (#38b) — merging avoids the decision.
- [ ] **Verification Report → Advanced Governance** `[Imp 3 / Cx 2 / Debt 2]` — Audit confirms: it's a processor **output**, not input — miscategorised as a sidecar. CONVENTIONS.md is written for input-carrying sidecars. Absorb as "Output Artifacts" section of Advanced Governance.
- [ ] **Due Process Config partial merge → Workflow Governance** `[Imp 5 / Cx 3 / Debt 4]` — Audit confirms: post-NoticeTemplate reconciliation, residual `independenceConstraint` / `appealRouting` / `continuationPolicies` duplicate Governance §3.1/§3.5 structurally. Absorb.
- **M-1 Drift Monitor + Agent Config — BLOCKED.** Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone. Ship #37 standalone binding instead; reconsider merge if fixture is revised.
- **M-2 Notification Template + Due Process Config — REJECTED.** 4 non-due-process categories. Ship #39 standalone linkage instead.

**Open from audit (user verdict needed):**

1. Ship the three §4.5 merges as one PR (audit recommendation: one PR, treat as a single schema-consolidation pass) or three discrete PRs?
2. Extract a shared `targetedLookupRef` `$def` across `calendarRef` / `templateRef` / `escalationChainRef` / `assertionRef` now, or let it emerge organically?

### 4.6 — Engineering hygiene (deprioritized)

Organizational debt, not architectural. First adopter won't notice. Schedule when the relevant code is actively being touched for another reason.

- [ ] **#22 Crate split along tier boundaries** `[Imp 5 / Cx 3 / Debt 3]` *(ProvenanceKind tier-typing extracted to §4.2 as #22a)* — Split `wos-core` → `wos-kernel | wos-governance | wos-ai | wos-advanced`. Split `wos-runtime/src/runtime.rs` (now 4451 lines, up from 3821) along action-kind dispatch. Add CI dependency fence. Remaining scope is purely organizational; first adopter won't notice. **Note:** `wos-formspec-binding → wos-runtime` inversion is already landed (`wos-formspec-binding/Cargo.toml:10-13`); `runtime.rs` lives in `wos-runtime`, not `wos-core`.
- [x] **#45 Sidecar normative-contract audit** — session 8 (`9900e39`). 9 sidecars audited: 3 KEEP / 3 MERGE / 3 RESHAPE / 0 RETIRE. Six open questions filed for user verdict. Report at `thoughts/reviews/2026-04-20-sidecar-contract-audit.md`. Audit ratifies the three §4.5 merges.

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
