# WOS TODO

Working backlog for the Workflow Orchestration Standard specification suite. Session narratives and all closed items live in [COMPLETED.md](COMPLETED.md); this file indexes only active work, open decisions, and parked/deferred items.

**Last audited:** 2026-04-20 (post session 9).

## Snapshot

| Health | Value |
|---|---|
| Specs / schemas | 20 specs · 25 schemas · 0 SCHEMA-DOC-001 violations |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) · 6 MVP (`wos-authoring`, `wos-mcp`, `wos-synth-core/-mock/-anthropic/-cli`) · 1 spike (`wos-synth-spike`, keep-with-deletion-horizon) |
| Tests | `cargo test --workspace` 1012 green · `pytest tests/schemas/` 188 / 11 skipped / 1 xfailed · `npm run docs:check` exit 0 |
| Lint matrix | 103 rules (35 T1 · 59 T2 · 9 T3 · 12 Tested · 91 Draft) |
| CI gates | `schema_doc_zero_regression` · `every_promoted_*_rule_has_executable_or_annotated_evidence` · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Navigation:** [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [ADRs](../thoughts/adr/) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR. Scores `[Imp / Cx / Debt]` on a 0-10 scale.

1. **#F3b Runtime §10.3 conformance** `[7 / 4 / 5]` — rewrite `crates/wos-core/src/eval.rs:412-421` as a post-mutation re-scan matching Runtime §10.3. ADR [0059](thoughts/adr/0059-continuous-mode-post-mutation-rescan.md). Task 3 closed out-of-band (`a683c03`); Tasks 1, 2, 4, 5 remain (~2-3 engineer-days). **Gate: none — ready to execute.**
2. **#20 Typed event meta-vocabulary** `[8 / 7 / 6]` — replace `Transition.event: string` with a 5-kind typed union. Plan: [2026-04-20](thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md). 185 fixtures / 844 occurrences to migrate; ~8-10 engineer-days. **Gate: user decision on OQ1 (`$join` disposition) + OQ4 (vendor-kind extension shape) — see Open questions.**
3. **§4.5 Structural merges** `[Cx 2 + 2 + 3]` — three merges ratified by the 2026-04-20 [sidecar audit](thoughts/reviews/2026-04-20-sidecar-contract-audit.md): assertion-library → workflow-governance, verification-report → advanced-governance (Output Artifacts), due-process-config residue → workflow-governance. See Backlog § Structural merges. **Gate: user decision — one PR or three?**
4. **§4.4 Release trains Tasks 4-5** — Changesets tooling + GitHub Actions release workflow. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-release-trains.md). Tasks 1-3 landed session 8. **Gate: none.**
5. **§5.5 `wos-bench` synthesis benchmark** — live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). **Gate: none.**

---

## Backlog

### Behavior + authoring surface

Normal feature work. Schedule once the critical path clears.

- **#26a `AccessControl.canRead` enforcement semantics** `[6 / 3 / 4]` — normative processor behavior on `canRead → false`: redact / null / raise / skip. Prerequisite to #26b.
- **#26b `caseFieldPolicy` schema** `[6 / 6 / 4]` — per-field read/write scopes by actor role.
- **#36 Equity RemediationTrigger expression language** `[6 / 4 / 4]` — FEL extension vs. restricted DSL vs. FEL + windowing. Prerequisite to #35.
- **#35 Equity Config enforcement semantics** `[7 / 5 / 4]` — processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime.
- **#24b + #25 joint design** `[#24b 7/6/4 · #25 6/7/6]` — Reasoning tier rule-firing trace + Catala-style defeasibility. Must compose with `sourceAuthority` rank (§6.2) + Integration Profile §11.2.
- **#43 Assurance × impact-level composition** `[6 / 5 / 4]` — minimum Assurance floor for AI-assisted rights-impacting determinations?
- **#30 WS-HumanTask lifecycle completion** `[5 / 5 / 2]` — task-level `Suspended`, distinct `Cancelled`, explicit `Return` with rework counter.
- **#27 Cancellation regions** `[4 / 6 / 3]` — YAWL-style named regions, distinct from existing `cancellationPolicy` join policy.
- **#28 Claim-check artifact references** `[4 / 4 / 2]` — typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` with integrity-check at retrieval. `inputDigest`/`outputDigest` already wired through `wos-export`.
- **#29b Milestone reactive transition firing (GSM-style)** `[6 / 5 / 2]` — ships after #29a (landed session 4).
- **#3 Policy-based migration routing** `[5 / 6 / 2]` — `migrationPolicy: grandfather | migrateAll | migrateByState | expression`. `tenant`-scope behavioral contract is an open sub-question.

### Structural merges (§4.5)

All three ratified by the 2026-04-20 sidecar audit. See Do next item 3 for decision.

- **Assertion Library → Workflow Governance** `[4 / 2 / 3]` — absorb as "Named Assertions". `AssertionUse` seam already landed session 8; merge is mechanical.
- **Verification Report → Advanced Governance** `[3 / 2 / 2]` — it's a processor **output**, miscategorized as a sidecar.
- **Due Process Config partial merge → Workflow Governance** `[5 / 3 / 4]` — residual `independenceConstraint` / `appealRouting` / `continuationPolicies` duplicate Governance §3.1/§3.5.
- **M-1 Drift Monitor + Agent Config — BLOCKED.** `fixtures/ai/benefits-drift-monitor.json` ships standalone; ship #37 standalone binding instead.
- **M-2 Notification Template + Due Process Config — REJECTED.** 4 non-due-process categories.

### Hygiene / refactors

Organizational debt; first adopter won't notice. Schedule opportunistically when the relevant code is already being touched.

- **#22a ProvenanceKind tier-typing** `[4 / 4 / 3]` — tier-typed record per `audit_layer`. Consider bundling with F3b Tasks 1-2 (both touch `provenance.rs`).
- **#22 Crate split along tier boundaries** `[5 / 3 / 3]` — `wos-core` → `wos-{kernel,governance,ai,advanced}`; `wos-runtime/runtime.rs` (4451 lines) split along action-kind dispatch; CI fence.
- **Cross-reference shape ADR** — `calendarRef` (URI) vs `templateRef` (plain string key) vs `escalationChainRef` (local id) vs `assertionRef` (URI) diverge. Surfaced by session-9 Reviews B + D. Pin the convention in a standalone ADR before the next cross-ref lands.

### Audit + evidence products

Build on the stable provenance export surface.

- **#48 Merkle provenance chains** `[6 / 6 / 6]` — hash-chained tamper-evident logs via Assurance `provenanceLayer` seam. **Debt raised:** PROV-O / XES / OCEL exports shipped without hash hooks; every format consumer now reads unlinkable output.
- **#52 Simulation trace format** `[4 / 3 / 2]` — normative replay contract + conformance fixtures. Event log format already shipped via `wos-export::xes`.

### Regulatory

External-deadline-driven; watch for compliance escalation.

- **#50 EU AI Act alignment** `[7 / 5 / 4]` — Art. 13-14 alignment spec, draft → 1.0.0.
- **#53 OMB M-24-10 compliance** `[6 / 4 / 3]` — process-documentation-shaped; overlaps Assurance + impact-level plumbing.

### Interoperability + speculative

Pick up once §§2-6 stabilize.

- **SCXML interoperability** `[3 / 6 / 2]` — bidirectional WOS ↔ SCXML mapping (currently informative only).
- **#51 Statutory deadline chains** `[4 / 7 / 5]` — interdependent government deadlines + automated legal consequences. Must compose with #31 business calendars + #20 typed events.

---

## Blocked / needs decision

Items that can't move without a verdict or an external trigger.

### Engine adapters — sequencing unresolved

TODO §3 previously scheduled engine adapters as near-term priority; IDEA_SCRATCH #49 marks them Defer with trigger "first commercial deployment requesting a specific adapter." No arbitrating document.

- **#49a Camunda 8 Worker** `[5 / 8 / 3]` — BPMN target; broadest external fixture diversity.
- **#49b Temporal Workflow** `[5 / 8 / 3]` — natural fit with WOS evaluator determinism.
- **#49c AWS Step Functions** `[5 / 8 / 3]` — broadest commercial reach; narrowest semantic fit.

### Ontology field identity

`ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and §6 regulatory specs. Prerequisite design: JSON-LD `@context` decision, semantic-field-identity protocol, cross-document alignment. Move to active only once a draft exists.

### Sidecar-audit open questions (2026-04-20)

From the 2026-04-20 [sidecar audit](thoughts/reviews/2026-04-20-sidecar-contract-audit.md); need user verdict.

1. Ship the three §4.5 merges as one PR (audit's recommendation) or three?
2. Extract a shared `targetedLookupRef` `$def` across the four divergent cross-ref shapes now, or let it emerge organically alongside §4.5?

---

## Deferred (with triggers)

Captured but not active; re-score when the trigger fires.

| IDEA # | Item | Imp/Cx/Debt | Trigger |
|---|---|---|---|
| #1 | Agent Behavioral Attestations | 2/7/1 | SLSA-style AI-agent attestation ecosystem matures. |
| #4 | Tripartite Object Model | 2/9/3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1/8/0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2/9/5 | Multi-object mutation emerges, or flat→OCEL export shows systematic loss. |
| #9 | JSON-LD Export Surface | 5/5/3 | Ontology spec drafts begin OR shipped PROV-O pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6/7/5 | #20 lands. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3/5/2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|---|---|---|
| Batch Operations | Parallel case instantiation, bulk state transitions | Sustained deployments above 100 cases/minute. |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS. |
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy. |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN expression language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; shipped PROV-O is JSON-LD. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Event taxonomy adopted normatively via #20. |

---

## Parked

- Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- JSON Patch for fine-grained provenance.
- FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open architectural questions

Load-bearing design decisions not yet attached to a single backlog item. Ordered by frequency of downstream dependence.

1. **Typed event taxonomy** (#20). **OQ1:** is `$join` engine-synthesized-only or authored as a `signal`? **OQ4:** closed `kind` enum with `x-*` payload extension, or open `kind` admitting `x-` prefix? Both are load-bearing on #20 Task 1.
2. **Registry composition** (#21). Two L1 governance docs attaching rules to the same tag — declaration order, explicit priority, or conflict rejection?
3. **Defeasibility layer** (#25). Workflow-governance or distinct companion? Priority encoding? Compose with `sourceAuthority` AND Integration Profile §11.2.
4. **Equity expression language** (#36). FEL extension, restricted DSL, or FEL + windowing?
5. **Assurance-level composition** (#43). Minimum floor per impact level, disclosure-only, or implementation-defined?
6. **JSON-LD authoring surface** (Deferred #9). Should `@context` land in authoring or stay export-only?

---

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
