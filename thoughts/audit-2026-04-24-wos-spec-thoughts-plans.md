---
date: 2026-04-24
scope: work-spec/thoughts/{plans,adr,specs}/ — plans folder + ADR-0059..0061 + Phase-11 integration master
source_count: 24
archive_candidates: 13
skill: squashing-specs
investigator: spec-investigator
extended: 2026-04-24 — added ADR-0061, ADR-0060, ADR-0059, and 2026-04-11 Phase 11 integration master
---

# Audit — work-spec/thoughts/{plans,adr,specs}/ — 2026-04-24

Walked **20** plan documents newest-to-oldest on 2026-04-24, then extended the same day to cover three ADRs (0061, 0060, 0059) and one integration spec (2026-04-11 Phase 11 master) — total **24** documents. Sort key for the plans walk: `Date:` YAML when present; else `YYYY-MM-DD-` from filename; for `0059-unified-ledger-*.md` (no date prefix) **mtime** (2026-04-24) so it sorts ahead of filename-dated plans. Same-calendar ties break by filename descending. The four extension docs were investigated as a single batch, also newest-to-oldest (`0061` 2026-04-21 → `0060` 2026-04-21 → `0059` 2026-04-20 → `phase11` 2026-04-11). Supersession chains: **none** declared authoritatively across plans (pre-scan grep hits were colloquial “replaces” in prose, not doc supersession); the Phase 11 master declares supersession of three upstream-Formspec-repo files (out of submodule scope — see §Extension). Per-doc verdicts from `spec-investigator`. Archive-move checklist at the bottom — review before executing.

## Declared-supersession pre-scan (hint only)

Grep for `(supersedes|replaces|obsoletes|rolls up into|merged into)` under `work-spec/thoughts/plans/*.md` surfaced only in-line prose (e.g. “replaces K-007 bunching”, “Supersedes nothing”, “replaces the current fixture-count metric”). **No batch superseded** another plan file; the walk did not short-circuit any file.

## Summary table

| # | Document | Frontmatter / banner | Verdict | Key finding |
|---|----------|----------------------|---------|-------------|
| 1 | `0059-unified-ledger-as-canonical-event-store.md` | Locked narrative — 2026-04-22 | MOSTLY RESOLVED (narrative) | Phase 1 WOS duties met; Phase 2/3 explicitly future-gated — not a task checklist plan. |
| 2 | `2026-04-20-wos-typed-event-meta-vocabulary.md` | Agentic-worker banner | FULLY RESOLVED | All 10 tasks closed; `TransitionEvent` typed model in schema/Rust/spec. |
| 3 | `2026-04-18-wos-facts-tier-input-snapshot.md` | Agentic-worker banner | MOSTLY RESOLVED | Schema + Rust + runtime + pytest landed; **K-DET-001** conformance + fixture migration **not** found. |
| 4 | `2026-04-17-wos-synth-v0-spike.md` | Agentic-worker banner | MOSTLY RESOLVED | Spike crate + retrospective done; live Anthropic iteration metrics still follow-up. |
| 5 | `2026-04-17-wos-schema-regression-tests.md` | Agentic-worker banner | MOSTLY RESOLVED | Test layers far exceed plan; **dedicated CI job** for `pytest tests/schemas` **not** confirmed (no workflow under `work-spec/.github`). |
| 6 | `2026-04-17-wos-mcp-crate.md` | Agentic-worker banner | FULLY RESOLVED | `wos-mcp` + 22 tools + schemas + integration tests landed. |
| 7 | `2026-04-17-wos-authoring-crate.md` | Agentic-worker banner | MOSTLY RESOLVED | Core façade + commands landed; some planned helpers / file layout differ from plan. |
| 8 | `2026-04-16-wos-trace-emitting-conformance.md` | Agentic-worker banner | FULLY RESOLVED | `ConformanceTrace`, golden traces, explain/diff CLIs, schema. |
| 9 | `2026-04-16-wos-synthesis-benchmark.md` | Agentic-worker banner | NOT STARTED | No `wos-bench` crate, no `BENCHMARK.md`, backlog in `TODO.md`. |
| 10 | `2026-04-16-wos-synth-crate.md` | Agentic-worker banner | MOSTLY RESOLVED | Four-crate split + Task 7 schema landed; criterion “10 NL problems converge” blocked on benchmark plan. |
| 11 | `2026-04-16-wos-structured-lint-diagnostics.md` | Agentic-worker banner | FULLY RESOLVED | `LintDiagnostic`, SARIF/JSON/text formatters, schema, migration doc. |
| 12 | `2026-04-16-wos-schema-description-audit.md` | Agentic-worker banner | PARTIAL | `SCHEMA-DOC-001` exists; triage doc + tier backfills + matrix promotion **open**. |
| 13 | `2026-04-16-wos-rule-coverage-conformance.md` | Agentic-worker banner | MOSTLY RESOLVED | Registry graduation + coverage CLI + matrix regen landed; seeded LoadBearing promotions + standalone CI steps **partial**. |
| 14 | `2026-04-16-wos-release-trains.md` | Agentic-worker banner | PARTIAL | Tasks 1–3 docs landed; **Changesets + `wos-release.yml` + consumer README** open (`TODO.md`). |
| 15 | `2026-04-16-wos-provenance-record-schema-extension.md` | Agentic-worker banner | FULLY RESOLVED | Eight fields + tier classifier + exporters + SP-EXPORT-004. |
| 16 | `2026-04-15-wos-provenance-export.md` | (none) | FULLY RESOLVED | `wos-export` + fixtures + COMPLETED PE.1/PE.2 narrative. |
| 17 | `2026-04-15-wos-custody-and-assurance.md` | Plan 1 of 3 banner | MOSTLY RESOLVED | Kernel `custodyHook` + Assurance spec/schema landed; **Governance** §2.9 / §4.9 / §7.15 + legal cross-ref **not** in spec/schema despite some matrix rows. |
| 18 | `2026-04-14-wos-spec-section-1-implementation.md` | (none) | FULLY RESOLVED | All 11 slices closed per `COMPLETED.md`. |
| 19 | `2026-04-13-wos-runtime-crate.md` | **Status:** Completed | FULLY RESOLVED | `wos-runtime` + `DurableRuntime` + integration handlers; exceeds original shape. |
| 20 | `2026-04-10-wos-core-extraction.md` | **Status:** Completed | FULLY RESOLVED | `wos-core` extraction phases complete per code + `COMPLETED.md`. |

## Per-source verdicts (newest → oldest)

### 1. `0059-unified-ledger-as-canonical-event-store.md`

*(Full investigator output in parent batch — summary: strategic north-star, Phase 1 responsibilities aligned with shipped code; Phase 2/3 gated. Verdict label used in batch: **MOSTLY RESOLVED** for narrative completeness vs future work.)*

### 2. `2026-04-20-wos-typed-event-meta-vocabulary.md`

**FULLY RESOLVED** — All 10 tasks complete per `COMPLETED.md`; `TransitionEvent` union in `wos-kernel.schema.json`, Rust `kernel.rs`, spec prose, migration script; remaining `"event"` strings scoped to runtime inbox / trace records per plan.

### 3. `2026-04-18-wos-facts-tier-input-snapshot.md`

**MOSTLY RESOLVED** — `CaseFileSnapshot`, schema conditionals, runtime tests, `test_facts_tier_snapshot.py` landed. **Gaps:** `K-DET-001` conformance rule not found; `caseFileSnapshot` fixture migration not evidenced; plan checkboxes still unchecked (tracking stale).

### 4. `2026-04-17-wos-synth-v0-spike.md`

**MOSTLY RESOLVED** — `wos-synth-spike` + benchmark problem + retrospective; empirical Anthropic iteration counts still open; plan checkboxes never flipped.

### 5. `2026-04-17-wos-schema-regression-tests.md`

**MOSTLY RESOLVED** — `conftest.py` + three-layer tests + large grown suite (255+ tests per `TODO.md`). **Gap:** Task 5 GitHub Actions job for schema regression **not** found in-repo under `work-spec/.github` (parent repo workflows not re-grepped here).

### 6. `2026-04-17-wos-mcp-crate.md`

**FULLY RESOLVED** — `crates/wos-mcp/` with dispatch, 22 tools, `wos-mcp-tools.schema.json`, `round_trip` / `stdio_transport` tests; minor file-layout vs plan.

### 7. `2026-04-17-wos-authoring-crate.md`

**MOSTLY RESOLVED** — `WosProject` / `RawWosProject` / `Command` pipeline shipped; ~27 helpers vs planned 28 with different names; nested state / some transition metadata helpers called out open in investigator output.

### 8. `2026-04-16-wos-trace-emitting-conformance.md`

**FULLY RESOLVED** — `ConformanceTrace`, golden fixtures, `wos-conformance-explain` / `-diff`, `conformance-trace.schema.json`.

### 9. `2026-04-16-wos-synthesis-benchmark.md`

**NOT STARTED** — No `crates/wos-bench/`, no leaderboard doc, single problem file; tracked as lower-priority backlog in `work-spec/TODO.md` (§5.5).

### 10. `2026-04-16-wos-synth-crate.md`

**MOSTLY RESOLVED** — `wos-synth-core` + mock + anthropic + CLI + trace schema; completion criterion 6 (10 converging NL workflows) depends on unstarted benchmark plan; `DirectToolContext` still stopgap per investigator.

### 11. `2026-04-16-wos-structured-lint-diagnostics.md`

**FULLY RESOLVED** — `LintDiagnostic`, tier rule migration, `format_text` / `format_json` / `format_sarif`, published JSON Schema, `MIGRATION.md`; SARIF substituted for planned `pretty` / `json-lines`.

### 12. `2026-04-16-wos-schema-description-audit.md`

**PARTIAL** — `schema_doc.rs` + `SCHEMA-DOC-001` in matrix at draft; triage markdown, tier backfills, fixture-linked promotion **not** done; plan checkboxes stale vs partial Task 1 land.

### 13. `2026-04-16-wos-rule-coverage-conformance.md`

**MOSTLY RESOLVED** — `Graduation` enum, fixture fields, `wos-rule-coverage` CLI, matrix regen from registries, test guards; mass promotion to LoadBearing seeded set largely **not** done; Tasks 6–7 as separate workflow/binary partially subsumed by `--strict` + tests.

### 14. `2026-04-16-wos-release-trains.md`

**PARTIAL** — `RELEASE-STREAMS.md`, per-stream changelogs, `COMPATIBILITY-MATRIX.md` landed; Changesets + publish glue + `wos-release.yml` + README “Versioning” **open** (matches `TODO.md` §4.4 Tasks 4–5).

### 15. `2026-04-16-wos-provenance-record-schema-extension.md`

**FULLY RESOLVED** — All tasks `[x]`; eight optional fields, tier classifier, runtime population, three exporters, SP-EXPORT-004; `COMPLETED.md` PE.2 narrative.

### 16. `2026-04-15-wos-provenance-export.md`

**FULLY RESOLVED** — Full `wos-export` crate + conformance fixtures + PE closeouts.

### 17. `2026-04-15-wos-custody-and-assurance.md`

**MOSTLY RESOLVED** — Kernel §10.5 `custodyHook` + assurance spec/schema; **Governance** extensions (schema upgrade, quorum delegation, legal hold) and legal-sufficiency cross-ref **missing** while some feature-matrix rows show ✅ — **matrix/spec drift** flagged by investigator.

### 18. `2026-04-14-wos-spec-section-1-implementation.md`

**FULLY RESOLVED** — Eleven slices closed in `COMPLETED.md`; binding handlers, milestones, CloudEvents, Arazzo path, fixture growth beyond plan.

### 19. `2026-04-13-wos-runtime-crate.md`

**FULLY RESOLVED** — Frontmatter Completed; crate shape evolved with intake, signatures, integration_handlers, durable seam.

### 20. `2026-04-10-wos-core-extraction.md`

**FULLY RESOLVED** — Frontmatter Completed; `wos-core` model/eval/provenance/traits as implemented; plan success checkboxes may still be visually unchecked in file (cosmetic vs `COMPLETED.md`).

## Extension — ADRs + Phase 11 spec (added 2026-04-24)

Same-day extension covering three ADRs (`thoughts/adr/`) and one integration master (`thoughts/specs/`). All four investigated in a single `spec-investigator` batch, newest-to-oldest. None of the four were superseded inbound; one (`phase11`) declares supersession of three upstream-Formspec-repo files (out of WOS-submodule scope). All four are archive candidates after a small frontmatter housekeeping pass — see §Archive-move checklist.

### Summary table — extension

| # | Document | Frontmatter status | Verdict | Key finding |
|---|----------|--------------------|---------|-------------|
| 21 | `adr/0061-custody-hook-trellis-wire-format.md` | Accepted 2026-04-21 | FULLY RESOLVED | All eight §5 cascade items landed; Trellis `append/010` regenerated with dCBOR + 2-tuple. |
| 22 | `adr/0060-cross-reference-naming-ref-key-id.md` | Accepted 2026-04-21 | FULLY RESOLVED (for §2 first-landed set) | Four declared renames landed in Workflow Governance; `correspondence-metadata.templateRef` is a residual taxonomy candidate. |
| 23 | `adr/0059-continuous-mode-post-mutation-rescan.md` | **Proposed** 2026-04-20 (stale — code is fully landed) | FULLY RESOLVED (frontmatter STALE) | All five §6 tasks closed across `2d890d3`/`a683c03`/`bdf7063`/`f03ca40`; K-049 is the only LoadBearing rule. |
| 24 | `specs/2026-04-11-formspec-wos-phase11-integration-master.md` | **proposed** (body claims Landed/Resolved) | MOSTLY RESOLVED | All §7 publication items satisfied; doc overtaken by ADR-0073 (intake handoff) + ADR-0061 (custody wire); frontmatter housekeeping needed. |

### Per-source verdicts — extension (newest → oldest)

#### 21. `adr/0061-custody-hook-trellis-wire-format.md`

**FULLY RESOLVED.** All eight §5 cascade items confirmed in HEAD:

- TypeID family prefixes (`case`/`prov`/`gov`/`ai`/`assurance`) registered — `crates/wos-core/src/typeid.rs:16-28` with vendor-prefix grammar at `:244-258`.
- Case TypeID minted at authoring — `crates/wos-core/src/instance.rs:105` (`mint_case_id`).
- Provenance TypeID minted at construction — `crates/wos-core/src/provenance/record.rs:159` (`mint_provenance_id`).
- JSON Schema TypeID pattern enforced on `id` — `schemas/kernel/wos-provenance-record.schema.json:93-95` (`format: wos-record-typeid`).
- Four-field append wire shape (`caseId`, `recordId`, `eventType`, `record`) — `crates/wos-runtime/src/custody.rs:107-120` (`CustodyAppendInput`).
- 2-tuple idempotency `(caseId, recordId)` — `custody.rs:149-153` (`idempotency_tuple`).
- Mechanical JSON→dCBOR converter with §2.7 rejection list (NaN/±Inf/integer overflow/oversize) — `custody.rs:250-330, 289-302, 259-264`; tests at `:502-513`.
- Round-trip fixture corpus byte-match (Rust authority) — `custody.rs:539-572` against `fixtures/kernel/custody-hook/provenance-state-transition/{record.json,record.dcbor,record.sha256}`.
- Normative encoding spec section published — `specs/kernel/custody-hook-encoding.md:1-7` (1.0.0-draft.1) + `schemas/kernel/wos-custody-hook-encoding.schema.json`.
- `CustodyAppendReceipt { canonical_event_hash }` typed `#[non_exhaustive]` — `custody.rs:176-181`; receipt stamping with conflict diagnostic at `crates/wos-runtime/src/runtime/provenance.rs:133-152`.
- Trellis fixture regenerated 2026-04-21 16:23 — `trellis/fixtures/vectors/append/010-wos-custody-hook-state-transition/` now has `input-wos-record.dcbor` + `input-wos-idempotency-tuple.cbor`.
- SignatureAffirmation flows the new wire shape — `custody.rs:574-615`.
- WOS-T1 closeout recorded — `TODO.md:5`; cascade detail at `COMPLETED.md:251-261`; reframed open question at `TODO.md:219` (cross-stack proof, not authored append).

**Outstanding (verbatim):** §4 item 9 — *“Open a shared-stack TypeID ADR (optional)… decide after first-implementation landings reveal whether the shared utility is worth the additional coordination.”* Optional, unscheduled, not a blocker.

**Staleness flags:** §3 Negative wording calls `serde_json_canonicalizer` "superseded" but §2.2 explicitly carves out §8.2.1 case-file snapshots as JCS — dependency is correctly retained at `crates/wos-core/src/provenance/snapshot.rs:22`. Worth a clarifying edit if amendment lands.

**Archive recommendation:** archive — Accepted, all cascade items landed, optional follow-on tracked elsewhere.

#### 22. `adr/0060-cross-reference-naming-ref-key-id.md`

**FULLY RESOLVED for the §2 first-landed rename set.** All four renames live in `schemas/governance/wos-workflow-governance.schema.json`:

- `WarningThreshold.templateKey` — `:1199, 1213-1215`.
- `BreachPolicy.templateKey` — `:1252-1254`.
- `BreachPolicy.escalationStepId` (resolves to sibling `EscalationStep.id` at `:1292`) — `:1257-1259`. Lint G-066 enforces resolution.
- `HoldPolicy.notificationTemplateKey` — `:1603`. Lint G-063 enforces resolution.
- `calendarRef` correctly retains `*Ref` for URI semantics — `:1168-1171`.
- WOS-T2 closure — `COMPLETED.md:32`; `TODO.md:39` (G-063/G-066 enforce remaining key/id resolution).

**Outstanding (verbatim):** *“Future schema PRs SHOULD continue applying the taxonomy until no plain-string key remains under a `*Ref` suffix.”* Migration §4 items 1-3 (dual properties, fixture/harness migration in same merge, Tier-2 deprecation lint) were skipped because WOS chose rename-in-place pre-1.0 — consistent with the ADR text.

**Staleness flags:** `schemas/kernel/wos-correspondence-metadata.schema.json:175` declares `templateRef` as the correspondence template's own `id` (`:117`) — under ADR-0060 taxonomy this is intra-document `*Id`, not URI. Candidate for the next sweep if "no plain-string key remains under a `*Ref` suffix" is acted on.

**Archive recommendation:** archive — Accepted, scoped first-landed set complete; future sweeps land per-PR, not against this ADR.

#### 23. `adr/0059-continuous-mode-post-mutation-rescan.md`

**FULLY RESOLVED — frontmatter STALE.** Doc still reads `Status: Proposed` but all five §6 tasks landed across four commits:

- Task 1+2 (rename + flip default): `try_fire_guardless_transition` removed; `Evaluator::rescan_on_mutation` at `crates/wos-core/src/eval.rs:447-449`; `Transition::participates_in_continuous_rescan` at `crates/wos-core/src/model/kernel.rs:575` no longer treats authored `"$continuous"` as opt-in (per `COMPLETED.md:319`).
- Task 3 (convergence-cap via `outcome` field) — landed `a683c03` per `COMPLETED.md:291`; `crates/wos-core/src/eval_mode.rs:88-112` emits `ProvenanceKind::ConvergenceCapReached` with `outcome: Some("convergenceCapReached")`.
- Task 4 (Tests A/B/C) — A: `eval_mode.rs:246-265` (`continuous_mode_fires_when_guard_satisfied`); B: `:323-383` (`convergence_cap_halts_infinite_loop`); supplementary regression `:269-302` and `:389-476`. C is structural (filter in `participates_in_continuous_rescan`), not a dedicated unit test.
- Task 5 (K-049 LoadBearing promotion) — landed `f03ca40`; `crates/wos-lint/src/rules/registry.rs:1031-1046` shows tier T2, `Graduation::LoadBearing`, two fixtures, `spec_ref: "specs/companions/runtime.md#s10-3"`, `suggested_fix` populated; message cites §10.3 + `CONVERGENCE_CAP` at `crates/wos-lint/src/rules/continuous_mode.rs:243-251`. Recorded at `LINT-MATRIX.md:148`.
- Precondition `#F5a` (kernel `ProvenanceOutcome` enum) confirmed in `2d890d3` per `COMPLETED.md:167`.

**Staleness flags:**

- Header `Status: Proposed` contradicts code reality — should flip to Accepted/Implemented before archive.
- §4.2 back-compat shim plan (keep `"$continuous"` as a no-op alias for one release cycle) was skipped: greenfield-cleanup commit `f03ca40` removed authored `"$continuous"` opt-in entirely. Deviation documented only in `COMPLETED.md:319`, not in the ADR.
- `COMPLETED.md:171, 199` describe F3b as "drafted" / "READY TO EXECUTE" — supersession by `:316-319` documents actual landing. Internal narrative drift, not a code defect.
- Test C ("mutation to a path no guard reads does NOT enter the re-scan loop — fast path") from §4.3 has no dedicated unit test; coverage is structural via `participates_in_continuous_rescan` filtering.

**Archive recommendation:** amend frontmatter (`Proposed` → `Accepted` or `Implemented`) **then** archive — value is now historical rationale for `participates_in_continuous_rescan`.

#### 24. `specs/2026-04-11-formspec-wos-phase11-integration-master.md`

**MOSTLY RESOLVED.** All five §7 publication items satisfied:

- Runtime Companion §12.9 `TaskPresenter` — `specs/companions/runtime.md:737-746`.
- Runtime Companion §15.5 `submitTaskResponse` — `specs/companions/runtime.md:859-924`.
- Runtime Companion §15.6 `ValidationOutcome` — `specs/companions/runtime.md:892-904`.
- Kernel schema fields (`responseMappingRef`, `prefillMappingRef`, `completionEvent`, `failureEvent` on `createTask`; `responseMappingRef`/`prefillMappingRef` on `ContractReference`; `contractRef` discriminator on `createTask`) — `schemas/kernel/wos-kernel.schema.json:1100-1127, 1410, 1419, 932, 1091`.
- Case-instance schema required `activeTasks` array of `ActiveTask` plus `$defs/FormspecTaskContext` and `$defs/ValidationOutcome` — `schemas/companions/wos-case-instance.schema.json:15, 139-142, 295-381, 404-451, 517+` (note: under `schemas/companions/`, not `schemas/kernel/` as the doc text loosely suggests).
- Reference binding crate exists with full S15 surface — `crates/wos-formspec-binding/src/lib.rs:23-664` (`IntakeHandoff`, `FormspecBinding`, `prepare_task`, `validate_submission`, `compute_case_mutation`, `revalidate_submission`); confirmed by `WOS-IMPLEMENTATION-STATUS.md:18, 41-42` and `WOS-FEATURE-MATRIX.md:20` (`ConformanceBinding` deleted, `StubValidator` retained for service-invocation contract validation only).

**Outstanding (§6.10 — "Closed decisions" verified live):** items 1-7 (prefill bidirectional Mapping, `persistTaskDraft`, multi-form modeling, agent submitters via `actorExtension`, `amended` Response → new amendment task, observability taxonomy, P11-BL-050 publication discipline) all reflected in `runtime.md:861, 863, 920-924`.

**Staleness flags:**

- Frontmatter `status: proposed` contradicts body claims of Landed/Resolved at `:51-53` and §7's all-`[x]` checklist.
- Doc has been overtaken by ADR-0073 (case initiation + intake handoff, accepted 2026-04-23 per `TODO.md:72`) and ADR-0061 (custodyHook wire format, this batch). Intake-handoff path now has its own typed binding (`wos-formspec-binding::IntakeHandoff` at `crates/wos-formspec-binding/src/lib.rs:72`) plus `IntakeAcceptanceAdapter` in `wos-runtime`. The doc still treats `submitTaskResponse` as the only ingress; doesn't mention public-intake handoff that is now load-bearing for the SBA + SaaS adopter.
- §6.10 item 7's `P11-BL-050` rule id has no entry in the lint registry — publication discipline honored implicitly; the rule id itself is dead text.
- §10 "Subagent / tooling note" predates the parent CLAUDE.md formal skill registration — informative, not normative; could be deleted.
- §6.10 reference to `thoughts/plans/2026-04-11-phase11-coprocessor-open-backlog.md` (line 264) points at the parent Formspec repo, not this submodule. Cold readers will grep here and find nothing.
- `supersedes_as_single_index` frontmatter cites three upstream-Formspec-repo files; archive verification is out of submodule scope.

**Archive recommendation:** flip `status: proposed` → `accepted` (or `landed`) **then** archive — completed integration index whose rationale stays useful as an archived precursor. Active Phase-11-territory work now lives in WOS-T1/T3/T4 cascades and ADRs 0061/0062/0073.

## Open items rollup

Deduplicated themes; tag sources.

### CI / release automation

- [ ] Schema regression: dedicated **`wos-schema-regression`** (or equivalent) GitHub Actions job with path filters — sources: `2026-04-17-wos-schema-regression-tests.md`
- [ ] Rule coverage: optional **`.github/workflows/wos-coverage.yml`** + `ratchet-check` binary — sources: `2026-04-16-wos-rule-coverage-conformance.md`
- [ ] Release trains Task 4–5: Changesets `fixed` groups, `scripts/wos-publish.mjs`, `.github/workflows/wos-release.yml`, README Versioning — sources: `2026-04-16-wos-release-trains.md`, `work-spec/TODO.md` (§4.4 backlog)
- [ ] Synthesis benchmark: entire `wos-bench` plan — sources: `2026-04-16-wos-synthesis-benchmark.md`, dependent criterion in `2026-04-16-wos-synth-crate.md`

### Conformance / fixtures / facts tier

- [ ] **`K-DET-001`** (determination transitions require `caseFileSnapshot`) + evidence map + fixtures — sources: `2026-04-18-wos-facts-tier-input-snapshot.md`
- [ ] Fixture migration for determination-bearing fixtures (`caseFileSnapshot` population) — sources: `2026-04-18-wos-facts-tier-input-snapshot.md`

### Schema documentation quality

- [ ] **`SCHEMA-DOC-001`** triage pass, offender list doc, per-tier description backfills, promotion past `draft` with fixtures — sources: `2026-04-16-wos-schema-description-audit.md`

### Governance / assurance alignment

- [ ] Governance **§2.9 schema upgrade**, **§4.9 quorum delegation**, **§7.15 legal hold** prose + schema fields; legal-sufficiency cross-ref to Assurance §6; Invariant 6 dedup grep — sources: `2026-04-15-wos-custody-and-assurance.md`, `WOS-FEATURE-MATRIX.md` (per investigator)

### Synth / MCP / authoring follow-ups

- [ ] Live Anthropic runs for **empirical iteration counts** (Q-V0-1..4) — sources: `2026-04-17-wos-synth-v0-spike.md`, `TODO.md` / synthesis benchmark backlog
- [ ] **Authoring** helpers: nested state, transition metadata setters, case field / correspondence APIs per plan — sources: `2026-04-17-wos-authoring-crate.md`
- [ ] Production **`ToolContext`** wiring vs `DirectToolContext` deferral — sources: `2026-04-16-wos-synth-crate.md`

### Program narrative (not “open tasks” but tracked obligations)

- [ ] Phase 3 mapping: every emission mappable to stable unified taxonomy event types — source: `0059-unified-ledger-as-canonical-event-store.md`

### Frontmatter housekeeping (extension)

- [ ] Flip `adr/0059-continuous-mode-post-mutation-rescan.md` `Status: Proposed` → `Accepted` / `Implemented` (all five §6 tasks landed; see verdict 23) — sources: `adr/0059-continuous-mode-post-mutation-rescan.md`
- [ ] Flip `specs/2026-04-11-formspec-wos-phase11-integration-master.md` `status: proposed` → `accepted` / `landed` (§7 publication checklist all satisfied; body already declares Landed/Resolved) — sources: `specs/2026-04-11-formspec-wos-phase11-integration-master.md`
- [ ] Document ADR-0059 §4.2 back-compat shim divergence (the one-release `"$continuous"` no-op alias was skipped in greenfield-cleanup commit `f03ca40`; deviation lives only in `COMPLETED.md:319`) — sources: `adr/0059-continuous-mode-post-mutation-rescan.md`
- [ ] Reconcile ADR-0061 §3 wording that `serde_json_canonicalizer` is "superseded" with the §2.2 carve-out keeping it for §8.2.1 case-file snapshots (`crates/wos-core/src/provenance/snapshot.rs:22`) — sources: `adr/0061-custody-hook-trellis-wire-format.md`

### Cross-reference taxonomy continuation (extension)

- [ ] Future schema PRs SHOULD continue applying the `*Ref` / `*Key` / `*Id` taxonomy until no plain-string key remains under a `*Ref` suffix; first remaining candidate is `schemas/kernel/wos-correspondence-metadata.schema.json:117, 175` (`templateRef` is intra-document `*Id`, not URI) — sources: `adr/0060-cross-reference-naming-ref-key-id.md`

### Stack-wide identifier seam (extension, optional)

- [ ] Open a shared-stack TypeID utility ADR (WOS, Formspec Response IDs, Trellis bundle artifacts could share one TypeID utility crate); explicitly optional and unscheduled; decide after first-implementation landings reveal whether the shared utility is worth coordination cost — sources: `adr/0061-custody-hook-trellis-wire-format.md` §4 item 9

### Phase 11 follow-on alignment (extension)

- [ ] Phase 11 master should be updated (or noted-and-archived) to reference the public-intake handoff path that ADR-0073 introduced and that `crates/wos-formspec-binding/src/lib.rs:72` (`IntakeHandoff`) + `IntakeAcceptanceAdapter` implement — currently the master treats `submitTaskResponse` as the only ingress — sources: `specs/2026-04-11-formspec-wos-phase11-integration-master.md`
- [ ] Either retire the dead `P11-BL-050` rule id (no entry in lint registry) or land the rule — sources: `specs/2026-04-11-formspec-wos-phase11-integration-master.md` §6.10 item 7
- [ ] Consider deleting §10 "Subagent / tooling note" — predates the parent CLAUDE.md formal skill registration — sources: `specs/2026-04-11-formspec-wos-phase11-integration-master.md`

## Cross-ref deltas

*(No `requirements-matrix.md` / `ratification-checklist.md` in `work-spec/` — deltas are vs `TODO.md` and investigator matrix/spec claims.)*

- **`TODO.md`** lists §4.4 release Tasks 4–5 and §5.5 `wos-bench` as backlog — **aligned** with investigator **PARTIAL** / **NOT STARTED** verdicts on those plans (no drift).
- **`TODO.md`** claims `#24a` / Facts-tier snapshot closed in narrative while **`2026-04-18-wos-facts-tier-input-snapshot.md`** still lists **K-DET-001** + fixture work — **drift** between completed narrative and conformance depth.
- **`WOS-FEATURE-MATRIX.md`** rows marked ✅ for governance items **without** matching spec/schema prose — **drift** between matrix and normative artifacts (`2026-04-15-wos-custody-and-assurance.md` finding).
- **Schema regression CI:** `TODO.md` cites `pytest tests/schemas` as a health snapshot, but **no** `work-spec/.github/workflows/**` references that suite — optional **drift** if parent-repo CI is expected to live only at monorepo root (not verified in this audit run).

### Extension cross-ref deltas

- **ADR-0059 frontmatter `Status: Proposed`** vs code state (all five §6 tasks landed across `2d890d3`/`a683c03`/`bdf7063`/`f03ca40`; K-049 is the only `LoadBearing` rule per `LINT-MATRIX.md:148` and `crates/wos-lint/src/rules/registry.rs:1031-1046`) — **drift** between ADR header and shipped reality.
- **Phase 11 spec frontmatter `status: proposed`** vs body claims (`:51-53` "Landed/Resolved"; §7 all `[x]`) and verified live state (Runtime Companion §15.5/§15.6/§12.9 + kernel/case-instance schema fields + `wos-formspec-binding` crate) — **drift** between frontmatter and body, and between body and the newer ADR-0073 / ADR-0061 framing that has overtaken Phase 11's framing of "the WOS-Formspec coprocessor."
- **`COMPLETED.md:171, 199`** still describe ADR-0059 F3b as "drafted" / "READY TO EXECUTE" — superseded internally by `:316-319` documenting actual landing — **internal narrative drift** in `COMPLETED.md`.
- **Phase 11 spec `P11-BL-050`** referenced as a publication rule (§6.10 item 7) but no rule with that id exists in the lint registry — **dead text**, not a normative drift.
- **ADR-0060 follow-on coverage:** WOS-T2 sweep closed Workflow Governance taxonomy; `schemas/kernel/wos-correspondence-metadata.schema.json:117, 175` still has `templateRef` for an intra-document `id` — **drift** between ADR-0060's "no plain-string key remains under a `*Ref` suffix" goal and current Correspondence Metadata schema.

## Archive-move checklist

Proposed `git mv` for each **FULLY RESOLVED** plan (per investigator). **Review each line. Do not blind-execute.** Narrative, partial, and not-started plans are intentionally omitted.

```bash
# Archive candidates — review each before running (repo root: formspec)
git mv work-spec/thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md work-spec/thoughts/archive/plans/2026-04-20-wos-typed-event-meta-vocabulary.md
git mv work-spec/thoughts/plans/2026-04-17-wos-mcp-crate.md work-spec/thoughts/archive/plans/2026-04-17-wos-mcp-crate.md
git mv work-spec/thoughts/plans/2026-04-16-wos-trace-emitting-conformance.md work-spec/thoughts/archive/plans/2026-04-16-wos-trace-emitting-conformance.md
git mv work-spec/thoughts/plans/2026-04-16-wos-structured-lint-diagnostics.md work-spec/thoughts/archive/plans/2026-04-16-wos-structured-lint-diagnostics.md
git mv work-spec/thoughts/plans/2026-04-16-wos-provenance-record-schema-extension.md work-spec/thoughts/archive/plans/2026-04-16-wos-provenance-record-schema-extension.md
git mv work-spec/thoughts/plans/2026-04-15-wos-provenance-export.md work-spec/thoughts/archive/plans/2026-04-15-wos-provenance-export.md
git mv work-spec/thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md work-spec/thoughts/archive/plans/2026-04-14-wos-spec-section-1-implementation.md
git mv work-spec/thoughts/plans/2026-04-13-wos-runtime-crate.md work-spec/thoughts/archive/plans/2026-04-13-wos-runtime-crate.md
git mv work-spec/thoughts/plans/2026-04-10-wos-core-extraction.md work-spec/thoughts/archive/plans/2026-04-10-wos-core-extraction.md
```

**Ensure** `work-spec/thoughts/archive/plans/` exists before running (create with `mkdir -p` if needed). **Do not** archive `0059-unified-ledger-as-canonical-event-store.md` under a “resolved implementation plan” rubric — it is a **living program narrative**, not a closed task list.

### Extension archive candidates — ADRs + Phase 11 spec

```bash
# Extension archive candidates — review each before running (repo root: formspec)

# Both ADR-0061 and ADR-0060 are Accepted in frontmatter; archive directly.
git mv work-spec/thoughts/adr/0061-custody-hook-trellis-wire-format.md work-spec/thoughts/archive/adr/0061-custody-hook-trellis-wire-format.md
git mv work-spec/thoughts/adr/0060-cross-reference-naming-ref-key-id.md work-spec/thoughts/archive/adr/0060-cross-reference-naming-ref-key-id.md

# ADR-0059: BEFORE archiving, edit frontmatter `**Status:** Proposed` → `**Status:** Accepted` (or `Implemented`).
# Optional: add a one-line note documenting that §4.2 back-compat shim was skipped (`f03ca40`).
git mv work-spec/thoughts/adr/0059-continuous-mode-post-mutation-rescan.md work-spec/thoughts/archive/adr/0059-continuous-mode-post-mutation-rescan.md

# Phase 11 master: BEFORE archiving, flip frontmatter `status: proposed` → `accepted` (or `landed`).
# Optional: add a one-line "overtaken by ADR-0073 / ADR-0061" note in the body or frontmatter.
git mv work-spec/thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md work-spec/thoughts/archive/specs/2026-04-11-formspec-wos-phase11-integration-master.md
```

**Ensure** `work-spec/thoughts/archive/adr/` and `work-spec/thoughts/archive/specs/` exist before running. **Two of these four require a frontmatter edit before `git mv`** (ADR-0059 and the Phase 11 master) — sequence the edit + move into the same commit so reviewers see the rationale together.

## Recommendations

1. **Checkbox hygiene:** Many landed plans still show `- [ ]` in the source `.md`. Either archive them after a final “mark complete” edit pass, or add a one-line banner “Execution record: see `COMPLETED.md` §…” to stop false “open plan” signals.
2. **Fix matrix/spec drift** for custody follow-on governance rows before treating those capabilities as shippable claims.
3. **Close the facts-tier loop:** either reopen a scoped TODO for K-DET-001 + fixtures, or add the conformance artifacts and update the plan so `#24a` narrative matches enforcement depth.
4. **Bench + synth:** keep `2026-04-16-wos-synthesis-benchmark.md` and synth completion criterion 6 explicitly linked in `TODO.md` until `wos-bench` exists — avoids silent dependency chains.
5. **Frontmatter housekeeping pass (extension finding):** ADR-0059 says `Proposed`, Phase 11 master says `proposed`, both are fully landed. Combine the two header flips with the four extension archive moves into a single housekeeping commit. Same pattern as recommendation #1, but at the ADR/spec layer rather than the plan layer — the work is done, only the metadata hasn’t caught up.
6. **Cross-doc themes worth tracking (extension):**
   - **TypeID + custody-hook wire format** spans verdict #21 (`adr/0061`) and #24 (`specs/phase11`). ADR-0061 pinned the four-field append surface, the TypeID identity primitive, and the dCBOR encoding discipline that the Phase 11 case-instance schema (`ActiveTask.lastValidationOutcome`) and the `wos-formspec-binding` runtime API both compose with via the post-0061 receipt-stamping path at `crates/wos-runtime/src/runtime/provenance.rs:142`.
   - **K-049 + §10.3** connect verdict #23 (`adr/0059`) to the lint-matrix promotion flow surfaced in plans verdict #13 (`2026-04-16-wos-rule-coverage-conformance.md`). ADR-0059 Task 5 is the trigger for K-049's LoadBearing graduation, and K-049 is still the *only* `LoadBearing` rule in the registry (`TODO.md:14` snapshot, `crates/wos-lint/src/rules/registry.rs:1043`). Useful framing for the seeded-LoadBearing-promotions backlog item.
   - **Stale completion metadata** unifies ADR-0059, the Phase 11 master, and the broader "checkbox hygiene" finding from the plans walk into one organizational signal: completed work whose status-line metadata hasn't caught up. A single sweep can close all three.

---

**Hand-off:** Review `work-spec/thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md`, then run the archive-move blocks in batches after creating `archive/plans/`, `archive/adr/`, and `archive/specs/` if desired. **Two extension archive moves require a one-line frontmatter edit first** (ADR-0059 and the Phase 11 master) — combine edit + move in the same commit. **Archive-candidate count:** 9 **FULLY RESOLVED** implementation plans + 4 extension docs (2 directly archivable + 2 archive-after-frontmatter-flip) = **13 total**. **Cross-ref delta count:** 4 plan-walk bullets + 5 extension bullets = **9 total**.
