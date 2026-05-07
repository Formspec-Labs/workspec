---
date: 2026-05-06
scope: work-spec/thoughts/ — all .md files (84 total) across root, adr/, archive/, examples/, plans/, practices/, research/, reviews/, specs/
investigator: cross-stack-scout (8 parallel agent swarm)
skill: squashing-specs
---

# Audit — all work-spec/thoughts/ — 2026-05-06

## Resolved by housekeeping pass (2026-05-06)

All items below were executed same-day by a single agent pass. Files edited or moved are listed per item.

| # | Action | What was done |
|---|--------|---------------|
| P0 | Bulk-update studio refs | `../../studio/` → `../../policy-studio/` in ADRs 0083-0091 (9 files edited via replaceAll) |
| P1 | Flip ADR-0059 frontmatter | `Status: Proposed` → `Status: Landed` in ADR-0059. ADR then archived to `archive/adr/0059-*` |
| P2 | Execute 13 archive moves | 9 plans → `archive/plans/`, 3 ADRs → `archive/adr/`, 1 spec → `archive/specs/`, 1 review → `archive/reviews/` (14 files moved) |
| P4 | Create missing stub | `archive/drafts/wos-agent-tier-amendments.md` written with "Missing — superseded by canonical specs" note |
| P5 | Reconcile Phase 11 master | Frontmatter `status: proposed` → `landed`. Supersedes list trimmed to 1 local source; other 2 noted as cross-repo. Spec file archived to `archive/specs/`. Table in §1 updated |
| P6 | Archive di-review | `reviews/2026-04-22-di-review-open-questions.md` → `archive/reviews/` |
| P7 | Relocate studio content to policy-studio/ | 9 ADRs (0083-0091 studio) moved to `policy-studio/thoughts/adr/`; 2 plans moved to `policy-studio/thoughts/archive/plans/`; all cross-refs updated for new locations |

Launched 8 parallel agents to read and validate every `.md` file under `work-spec/thoughts/` (84 files across 12 directories). Each agent determined per-file: current status, implementation evidence, supersession state, cross-reference health, and gaps.

---

## 1. Root thoughts/ (3 files)

### `2026-04-24-standards-absorption-gap-analysis.md`
**STATUS: PARTIAL**

Architectural note proposing three refactor targets abstracted from BPMN/SCXML/DMN/CMMN/WS-HumanTask:
1. Governed output commit pipeline (`mutationSource` + `verificationLevel`) — **LANDED** in schema, spec, Rust model, lint rules
2. `activationCriteria` — **NOT IMPLEMENTED** — zero schema definitions or Rust types
3. `DecisionServiceEvidence` — **NOT IMPLEMENTED** — no governance schema `$defs` entry

Not superseded — still a live reference.

### `audit-2026-04-24-wos-spec-thoughts-plans.md`
**STATUS: PENDING**

Meta-audit of 24 docs. Recommendations mostly unexecuted:
- 13 archive-move candidates not moved (`archive/plans/` does not exist)
- 2 frontmatter flips not done (ADR-0059 still says "Proposed", Phase 11 master still says "proposed")
- 9 cross-ref deltas unreconciled
- CI automation, K-DET-001 conformance rule, governance §2.9/§4.9/§7.15 not addressed

### `audit-2026-04-28-provenance-emission-completeness.md`
**STATUS: PARTIAL**

Verified 99 `ProvenanceKind` variants against spec MUST→emit sites. Gap 1 (CapabilityInvocation) closed same-day. Remaining:
- Gap 2 (TaskSkipped) — zero live emission sites (TODO.md #66e, backlog)
- Gap 3 (ConfigurationWarning) — 4 spec MUSTs with no emission path (no TODO.md item)
- Meta finding (schema↔enum parity gate) — no CI enforcement (no TODO.md item)

---

## 2. ADRs — `adr/` (7 files — 9 studio ADRs 0083-0091 moved to `policy-studio/thoughts/adr/`)

### DONE (3)

| ADR | Title | Evidence |
|-----|-------|---------|
| 0062 | Signature Profile workflow semantics | `ProvenanceKind::SignatureAffirmation`, runtime with add/sign/decline/expire, 5 lint test files |
| 0063 | Embedded-vs-sidecar identity boundary | 7 `allOf` conditionals in schema, 3 lint rules (Tier 1), no legacy markers in parser |
| 0064 | Agent as first-class ActorKind | Closed `ActorKind` enum (Human\|System\|Agent), `AgentInvoker` trait, 6 adapter crates (5 skeletons per scope) |

### ACCEPTED — NOT IMPLEMENTED (1)

| ADR | Title | Gap |
|-----|-------|-----|
| 0092 | TypeID as URN namespace-specific string | Zero of 8 work streams started. Schema still has 5-segment URN pattern. `to_instance_urn()` and `urn_scope_and_date()` still exist. |

### PENDING (1)

| ADR | Title | Status |
|-----|-------|--------|
| 0082 | Kernel semantic projection and import | Partial — OntologyAlignment is recognized but full SHACL/SPARQL/PROV-O/XES mechanics not implemented. No verifier |

### PARTIAL (1)

| ADR | Title | Gap |
|-----|-------|------|
| 0083 | Instance migration (runtime/HTTP) | D1-D3 DONE. D4-D7 open (precondition posture, error model, provenance ordering). Restate adapter returns "unsupported" for `migrate_instance` |

---

## 3. Archives — `archive/` (19 files)

### `adr/0057-wos-core-implementation-boundary.md`
**Superseded** — decisions enacted in `wos-core` crate and Runtime Companion. Historical record.

### `adr/0058-wos-core-gap-analysis.md`
**Superseded** — all accepted constructs implemented in Kernel S5.5, Policy Parameters sidecar, Governance S11/S12. Implementation disposition table confirms full resolution.

### `drafts/` (12 files)
- `README.md` — archive index (metadata, not content)
- `wcos-lifecycle-spec.md` → `wos-core-v7-proposal.md` (11 sequential drafts) — **all superseded** by canonical `specs/kernel/spec.md`, `specs/ai/*`. Version chain: v0.1 → v2 (8-layer) → v3 (JSON-LD/SHACL) → v4 (constraint-enhanced) → v5 (Formspec contracts) → v6 (community review) → v7 (minimal kernel + profiles)
- `wos-agent-tier-amendments.md` — **MISSING** (referenced in dir listing but absent on disk)

### `reviews/2026-04-16-architecture-review-handoff.md`
**Archived** — actions became plans (`thoughts/plans/2026-04-16-*` series)

### `reviews/2026-04-16-architecture-review-open-questions.md`
**Archived** — file's own status states "Ready to archive"

### `specs/2026-04-11-wos-s15-formspec-coprocessor-proposal.md`
**Superseded** — merged into `thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md`. Explicit `superseded_by` frontmatter.

---

## 4. Plans — `plans/` (31 files — 2 studio plans moved to `policy-studio/thoughts/archive/plans/`)

### DONE (16)

| Plan | Notes |
|------|-------|
| 2026-04-10-wos-core-extraction | `wos-core/` crate with typed models, Evaluator, ProvenanceRecord |
| 2026-04-13-wos-runtime-crate | `wos-runtime/` with instance lifecycle, event drain, timer wake-up, 7 host handlers |
| 2026-04-14-wos-spec-section-1-implementation | All 11 slices closed: S15 binding, history states, milestones, CloudEvents, etc. |
| 2026-04-15-wos-provenance-export | `wos-export/` with PROV-O, XES, OCEL exporters |
| 2026-04-16-wos-provenance-record-schema-extension | All 8 fields + 3 beyond-plan in `ProvenanceRecord` |
| 2026-04-16-wos-schema-description-audit | `SCHEMA-DOC-001` lint rule, 0 violations, CI gate |
| 2026-04-16-wos-structured-lint-diagnostics | `LintDiagnostic` struct, `--format` CLI, published schema |
| 2026-04-16-wos-synth-crate | Four-crate DIP split: `wos-synth-core/anthropic/mock/cli` |
| 2026-04-16-wos-trace-emitting-conformance | `ConformanceTrace`, golden traces, `explain`/`diff` CLI |
| 2026-04-17-wos-authoring-crate | `WosProject` with 28 helpers, undo/redo, round-trip |
| 2026-04-17-wos-mcp-crate | JSON-RPC-2.0 transport, 22 tools, `dispatch()` |
| 2026-04-17-wos-schema-regression-tests | 3-layer pytest: meta/fixture/spec-example validity |
| 2026-04-17-wos-synth-v0-spike | DONE — kept with deletion horizon; retrospective at `research/` |
| 2026-04-20-wos-typed-event-meta-vocabulary | 5-kind `TransitionEvent` union, 185 fixtures migrated |
| 2026-05-01-stage-4-decision-table-lint-rules | `DecisionTable` model, K-051/052/053 at Tested |
| 2026-05-01-wos-runtime-parity-and-vocab-closure | All open-string leaves closed, inventory committed |
| 2026-05-01-pln0333-ws094-acceptance-checklist | 164/164 tests, ingress smoke + parity, D.1b open |

### PARTIAL (5)

| Plan | Gap |
|------|-----|
| 2026-04-15-wos-custody-and-assurance | No `schemas/assurance/wos-assurance.schema.json`. Lint rules not authored. Plan 2+3 not started |
| 2026-04-16-wos-release-trains | No `.changeset/`, no release CI workflow, no publish/check-compat scripts, no tags |
| 2026-04-16-wos-rule-coverage-conformance | 104/116 rules still at Draft. `fixture_links.rs` test missing. Ratchet automation not wired |
| 2026-04-18-wos-facts-tier-input-snapshot | Fixture migration (~20 fixtures) not confirmed complete |
| 2026-05-01-wos-restate-ws094-execution | D.1b (retryable/stall) open, tracked as WS-105. CI workflow not in this repo |

### PENDING (3)

| Plan | Gap |
|------|-----|
| 2026-04-16-wos-synthesis-benchmark | No `wos-bench` crate, no `BENCHMARK.md`, no `benchmarks/runs/` |
| 2026-05-01-wos-c8-graph-lint-k033-k034 | K-033/K-034 not implemented. K-032 exists with deferral note |
| 2026-05-06-adr0092-typeid-urn-identity-landing | All 8 work streams unstarted. ~42+ files need changes |

### SUPERSEDED (1)

| Plan | Reason |
|------|--------|
| 0059-unified-ledger-as-canonical-event-store | North-star. Phase 1 (export + custodyHook) and Phase 2 (attestation) shipped instead. Phase 3 deferred |

### OPEN (1)

| Plan | Gap |
|------|-----|
| 2026-04-18-wos-remainder-di-seam-framing | Massive gap. Tracks A-H sequencing document; Track A (server wiring) and Track B (seam tightening) items mostly unexecuted. Track G (DI hardening: `AuthProvider→AuthVerifier`, `ProvenanceSigner→LedgerAttachment`) high-priority debt |

---

## 5. Examples — `examples/` (2 files)

| File | Status |
|------|--------|
| `medicaid-redetermination-user-stories.md` | **CURRENT** — archival/educational. Names Temporal (pre-Restate) but still useful as narrative |
| `temporal-reference-implementation.md` | **SUPERSEDED** — Restate replaced Temporal as default adapter per WOS-T3 spike |

---

## 6. Practices — `practices/` (2 files)

| File | Status |
|------|--------|
| `2026-04-17-parallel-agent-dispatch.md` | **CURRENT** — standing team discipline. 5 rules for safe parallel dispatch |
| `README.md` | **CURRENT** — index. `vision-model.md` path may be stale (parent uses `VISION.md`) |

---

## 7. Research — `research/` (4 files)

| File | Status | Gap |
|------|--------|-----|
| `2026-04-17-k012-k017-load-bearing-audit.md` | **ARCHIVAL** | Negative fixture follow-up never tracked |
| `2026-04-17-wos-schema-doc-audit-triage.md` | **PARTIAL** | Pre-ADR-0076 schema landscape — backlog/reshape split needs re-evaluation against merged schema |
| `2026-04-20-wos-synth-v0-spike-findings.md` | **ARCHIVAL** | Live API runs (Q-V0-1 through Q-V0-4) never completed. Spike deletion horizon elapsed |
| `2026-05-01-schema-spec-crate-parity-inventory.md` | **CURRENT** | TODO classifications unclosed — needs regeneration recipe re-run |

---

## 8. Reviews — `reviews/` (4 files)

| File | Status | Gap |
|------|--------|------|
| `2026-04-09-wos-core-companion-review.md` | **ARCHIVAL** | 5→6 seams, 19→9 schemas. Decisions still authoritative |
| `2026-04-20-sidecar-contract-audit.md` | **PARTIAL** | 3 merges done (via ADR-0076). 3 reshapes need closure check (equity-enhancement MUSTs, drift-monitor action semantics, policy-parameters namespace) |
| `2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md` | **CURRENT** | Production adapter backlog partially complete (WS-094/PLN-0333) |
| `2026-04-22-di-review-open-questions.md` | **PARTIAL/STALE** | All 16 decisions still marked `pending`. Several implicitly resolved by later work (Q1→ADR-0084, Q2→Restate adapter spec) |

---

## 9. Specs — `specs/` (3 files)

| File | Status | Gap |
|------|--------|------|
| `2026-04-11-formspec-wos-phase11-integration-master.md` | **CURRENT/archival** | Claims 3 archived sources — only 1 in archive/ dir. Cross-repo backlog reference dead |
| `2026-05-01-wos-restate-durable-runtime-adapter-spec.md` | **CURRENT** | Still "Draft (thoughts lane)" — promotion gated |
| `2026-05-06-api-typeid-identity.md` | **CURRENT** | Very recent. File:line server code references will drift |

---

## Cross-Cutting Issues

1. **Broken cross-refs: ADR 0083-0091** — 8 ADRs reference `../../studio/...` paths that no longer exist. ✅ RESOLVED: ADRs 0083-0091 moved to `policy-studio/thoughts/adr/` with all cross-refs updated for the new location. (Initially fixed refs in place via `../../studio/` → `../../../policy-studio/`, then relocated.)
2. **Stale frontmatter** — ADR-0059 still says `Status: Proposed` despite full implementation. Phase 11 integration master says `status: proposed` with body claiming "Landed." ✅ RESOLVED: ADR-0059 flipped to `Landed` and archived to `archive/adr/`. Phase 11 spec flipped to `status: landed`, supersedes list reconciled, and archived to `archive/specs/`.
3. **ADR-0092 unimplemented** — **Accepted 2026-05-06 (standalone ratification sweep).** Zero of 8 work streams started. Schema, functions, tests all still use old 5-segment URN. Now tracked as Do-next #6 (Stream A) with implementation plan at `thoughts/plans/2026-05-06-adr0092-typeid-urn-identity-landing.md`. *(Previously: Accepted but unimplemented — now Accepted and Do-next.)*
4. **Audit-2026-04-24 recommendations unexecuted** — 13 archive moves not done, 9 cross-ref deltas unreconciled, CI automation not built. ✅ RESOLVED (partial): 13 archive moves executed (14 files: 9 plans, 3 ADRs, 1 spec, 1 review). 9 cross-ref deltas resolved via ADR 0083-0091 relocation to `policy-studio/`. CI automation not addressed.
5. **Provenance gaps stale** — 3 audit findings from 2026-04-28 have no TODO.md tracking for gaps 2 and 3. *(Not addressed — still open.)*
6. **Missing file** — `archive/drafts/wos-agent-tier-amendments.md` listed but absent on disk. ✅ RESOLVED: Stub file created with "Missing — superseded by canonical specs" note.
7. **Missing archive source** — Phase 11 master claims 3 upstream sources archived; only 1 (`wos-s15-formspec-coprocessor-proposal.md`) is in `archive/specs/`. ✅ RESOLVED: Frontmatter trimmed to 1 local source; other 2 noted as cross-repo. Spec archived to `archive/specs/`.
8. **DI review open** — All 16 decisions still `pending`. Several implicitly resolved by ADR-0084 (Restate), ADR-0082 (server), and WOS-T3 spike. ✅ RESOLVED: Document archived to `archive/reviews/`.

---

## Per-directory Rollup

| Directory | Count | Done | Partial | Pending | Superseded/Archived |
|-----------|-------|------|---------|---------|---------------------|
| `thoughts/` (root) | 3 | — | 2 | 1 | — |
| `adr/` | 7 | 3 | 1 | — | — |
| `archive/` | 19 | — | — | — | 19 |
| `examples/` | 2 | — | — | — | 2 |
| `plans/` | 31 | 16 | 5 | 3 | 2 |
| `practices/` | 2 | 2 | — | — | — |
| `research/` | 4 | — | 1 | — | 3 |
| `reviews/` | 4 | — | 2 | — | 2 |
| `specs/` | 3 | — | — | — | 3 |
| **Total** | **73** | **21** | **9** | **3** | **29** |

*(*) ADR-0092 accepted but not implemented. ADRs 0083-0091 (studio) moved to `policy-studio/thoughts/adr/` — their status is tracked in policy-studio's own audit.*

---

## Recommendation Priority

| Priority | Action | Files affected | Done |
|----------|--------|---------------|------|
| P0 | Move ADRs 0083-0091 + 2 plans from work-spec/ to policy-studio/ + fix cross-refs | 11 files | ✅ 2026-05-07 |
| P1 | Flip ADR-0059 frontmatter to `Status: Landed` | 1 file | ✅ 2026-05-06 |
| P2 | Execute audit-2026-04-24 archive moves (13 → 14 actual candidates after adding ADR/spec/review) | 14 files | ✅ 2026-05-06 |
| P3 | Start ADR-0092 implementation or document deferral | 8 work streams | ✅ 2026-05-07 — ADR accepted in ratification sweep; now Do-next #6 (Stream A) |
| P4 | Create missing `wos-agent-tier-amendments.md` stub or remove reference | 1 entry | ✅ 2026-05-06 |
| P5 | Reconcile Phase 11 master archived-source claim with archive contents | 1 file | ✅ 2026-05-06 |
| P6 | Archive or update `di-review-open-questions.md` with resolution status | 1 file | ✅ 2026-05-06 |
| P7 | File TODO.md items for provenance gaps 2-3 | 2 items | ✅ 2026-05-07 — #73 (ConfigurationWarning emission) + #74 (ProvenanceKind enum↔schema parity CI gate) filed |
| P8 | Re-assess research/ and review/ docs against current schema/spec landscape | 4 files | — |
