# WOS TODO

Working backlog for the Workflow Orchestration Standard specification suite. Session narratives and all closed items live in [COMPLETED.md](COMPLETED.md); architectural commitments and scope lines live in the [stack-wide vision model](../.claude/vision-model.md); this file indexes active work, blocked items, and trigger-gated future work.

**Last audited:** 2026-04-20 (post vision-model sweep; scope reshaped per Trellis boundary + v1.0 expansion under minutes-not-days).

## Snapshot

| Health | Value |
|---|---|
| Specs / schemas | 20 specs · 25 schemas · 0 SCHEMA-DOC-001 violations |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) · 6 MVP (`wos-authoring`, `wos-mcp`, `wos-synth-core/-mock/-anthropic/-cli`) · 1 spike (`wos-synth-spike`, keep-with-deletion-horizon) |
| Tests | `cargo test --workspace` 1012 green · `pytest tests/schemas/` 188 / 11 skipped / 1 xfailed · `npm run docs:check` exit 0 |
| Lint matrix | 103 rules (35 T1 · 59 T2 · 9 T3 · 12 Tested · 91 Draft) |
| CI gates | `schema_doc_zero_regression` · `every_promoted_*_rule_has_executable_or_annotated_evidence` · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Navigation:** [**User profile** (read first)](../.claude/user_profile.md) · [**Vision model**](../.claude/vision-model.md) (stack-wide; WOS section inside) · [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [ADRs](../thoughts/adr/) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR.

**Scoring note.** Per [`user_profile.md`](../.claude/user_profile.md) economic model: dev/time is free, architectural drift is expensive. Ordering uses **`Imp × Debt`**; Cx is preserved as a scheduling dimension but does not change priority. Debt values trend **up** between sessions on pre-1.0 work. Score notation: `[Imp / Cx / Debt]`; the number in parentheses is `Imp × Debt`.

1. **#20 Typed event meta-vocabulary** `[8 / 7 / 8]` (**64**) — replace `Transition.event: string` with a 5-kind typed union (`timer | message | signal | condition | error`). 185 fixtures / 844 occurrences; every synth / bench / export / analyzer currently building against the loose string shape. Plan: [2026-04-20](thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md). **Gate softened:** vision-model captures defaults for OQ1 (`$join` engine-synthesized-only) and OQ4 (closed enum + `x-*` payload extension). User may override at implementation time; no longer a hard block.
2. **#F3b Runtime §10.3 conformance + #22a prerequisite** `[7 / 5 / 6]` (**42**) — rewrite `crates/wos-core/src/eval.rs:412-421` as a post-mutation re-scan matching Runtime §10.3. ADR [0059](thoughts/adr/0059-continuous-mode-post-mutation-rescan.md). Task 3 closed out-of-band (`a683c03`); Tasks 1, 2, 4, 5 remain. **Do #22a ProvenanceKind tier-typing first** (module-bottleneck heuristic; both touch `provenance.rs`); splitting the file unblocks parallel work on F3b + custodyHook + Temporal adapter. **Gate: none — ready to execute.**
3. **`custodyHook` Trellis joint ADR** `[6 / 4 / 6]` (**36**) — the concrete provenance-record shape WOS emits and Trellis expects to anchor. Joint-design ADR spanning both submodules; surfaced as load-bearing during vision-model pass. Coordinates with Trellis's reference implementation (ADR 0004 per vision-model status). **Gate: none — Trellis has a reference implementation; design the wire-format ADR.**
4. **Cross-reference shape ADR** `[6 / 2 / 6]` (**36**) — `<entity>Ref` (URI) / `<entity>Key` (local string) / `<entity>Id` (sibling id). Three patterns named, not unified — they denote different things. Renames `templateRef` → `templateKey`, `escalationChainRef` → `escalationStepId`. **Gate: none — draft takes ~2 hours.**
5. **`DurableRuntime` trait extraction + Temporal/Restate spike** `[7 / 5 / 5]` (**35**) — extract the center-vs-adapter seam from `wos-runtime`; ship in-memory adapter + spike Temporal + Restate against it; pick production backend based on direct observation of Rust SDK ergonomics + ops fit. Three-way conformance posture (spec + reference + production adapter) unlocks. Spike must also answer the multi-tenant contract (namespaces vs. partitions + per-tenant provenance-log scoping); the tenant-scope answer #3 waits on comes out of this spike. **Gate: none — vision-model default is "spike both, pick from observation."**
6. **Signature Profile workflow semantics** `[7 / 5 / 5]` (**35**) — DocuSign-parity workflow semantics for WOS (signer roles via `actorExtension`, flow patterns as lifecycle tags, intent capture, signer-authentication policy schema, `SignatureAffirmation` provenance record shape). Cryptographic integrity + cert-of-completion live in Trellis; WOS only emits the evidence record. **Gate: α DocuSign parity bar (vision-model default: ESIGN/UETA/eIDAS + top ~80% common-case features; NOT administrative UX surface — confirm at drafting time).**

*Falling off Do next at Imp × Debt < 30:* §4.5 merges (20, owner decision needed), §5.5 `wos-bench` (18), §4.4 release-trains Tasks 4-5 (15). All live in Backlog.

---

## Backlog

### Structural merges (§4.5) — blocked on owner decision

Three merges ratified by the 2026-04-20 [sidecar audit](thoughts/reviews/2026-04-20-sidecar-contract-audit.md). **Gate: user decision — one PR or three?** Vision-model recommendation: three discrete PRs for review hygiene; audit recommended one. Either is acceptable; owner picks.

- **Assertion Library → Workflow Governance** `[4 / 2 / 5]` (**20**) — `AssertionUse` seam already landed session 8; merge is mechanical file-move.
- **Verification Report → Advanced Governance** `[3 / 2 / 2]` (**6**) — it's a processor **output**, miscategorized as a sidecar.
- **Due Process Config partial merge → Workflow Governance** `[5 / 3 / 4]` (**20**) — residual sections duplicate Governance §3.1/§3.5.

Companion decisions from session-9 agent dispatch: M-1 Drift Monitor + Agent Config merge remains BLOCKED (standalone fixture); M-2 Notification Template + Due Process merge remains REJECTED (categories don't align).

### Release + benchmarking — ready, lower priority

- **§4.4 Release trains Tasks 4-5** `[5 / 4 / 3]` (**15**) — Changesets tooling + GitHub Actions release workflow. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-release-trains.md). Tasks 1-3 landed session 8.
- **§5.5 `wos-bench` synthesis benchmark** `[6 / 5 / 3]` (**18**) — live Anthropic run closes Q-V0-1..4 from the v0 spike retrospective. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md).

### Behavioral / governance (1.0 scope under minutes-not-days)

Per vision model: no "defer to 1.1" bucket on greenfield. These all land at 1.0 unless explicit architectural prerequisite unresolved.

- **#35 Equity Config enforcement semantics** `[7 / 5 / 4]` (**28**) — processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime. Prerequisite: #36 resolved (vision model: FEL + restricted-domain profile).
- **#36 Equity RemediationTrigger expression language** `[6 / 4 / 4]` (**24**) — FEL + restricted-domain profile per vision model; no windowing escape hatch. Implementation.
- **#26a `AccessControl.canRead` enforcement semantics** `[6 / 3 / 4]` (**24**) — normative processor behavior on `canRead → false`: redact / null / raise / skip. Prerequisite to #26b.
- **#26b `caseFieldPolicy` schema** `[6 / 6 / 4]` (**24**) — per-field read/write scopes by actor role.
- **#43 Assurance × impact-level composition** `[6 / 5 / 4]` (**24**) — minimum Assurance floor per impact level (rights-impacting ≥ `high`; safety-impacting ≥ `high`; operational ≥ `standard`) per vision model.
- **#24b + #25 joint design** `[#24b 7/6/4 · #25 6/7/6]` — Reasoning tier rule-firing trace + Catala-style defeasibility. Vision model: `workflow-governance` with `(sourceAuthority, priority)` lexicographic. After ADR.
- **#38 G-064 Assertion Library resolution lint** `[5 / 3 / 3]` (**15**) — implementation of the lint designed in session 8.
- **#40 Task SLA runtime implementation** — beyond the session-8 authoring surface; wire §10.3 runtime obligations.
- **Bulk Operations spec** (relocated from Future specs) — admin-portal-driven; parallel case instantiation + bulk state transitions.
- **#28 Claim-check artifact references** `[4 / 4 / 5]` (**20**) — typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }`.
- **#30 WS-HumanTask lifecycle completion** `[5 / 5 / 2]` (**10**) — task-level `Suspended`, distinct `Cancelled`, explicit `Return` with rework counter.
- **#27 Cancellation regions** `[4 / 6 / 3]` (**12**) — YAWL-style named regions distinct from `cancellationPolicy` join policy.
- **#29b Milestone reactive transition firing (GSM-style)** `[6 / 5 / 2]` (**12**) — ships after #29a (landed session 4).
- **#3 Policy-based migration routing** `[5 / 6 / 2]` (**10**) — `migrationPolicy: grandfather | migrateAll | migrateByState | expression`. Tenant-scope sub-question finalizes with `DurableRuntime` tenant contract.

### Hygiene / refactors

Sequenced for module-bottleneck relief, not delayed by it.

- **#22 Crate split along tier boundaries** `[5 / 3 / 3]` (**15**) — `wos-core` → `wos-{kernel,governance,ai,advanced}`; `wos-runtime/runtime.rs` (4451 lines) split along action-kind dispatch; CI fence. (#22a ProvenanceKind tier-typing is the part that sits on F3b's critical path — handled in Do next #2.)

### Audit + evidence products

Build on the stable provenance export surface. #48 Merkle provenance moved to Trellis scope; see "Moved to Trellis" below.

- **#52 Simulation trace format** `[4 / 3 / 2]` (**8**) — normative replay contract + conformance fixtures. Event log format already shipped via `wos-export::xes`.

### Verifiability closure (1.0)

Per [vision-model.md v1.0 spec-freeze line](../.claude/vision-model.md#v10-spec-freeze-line): "every normative MUST across Kernel + Governance + AI Integration has a passing Tested fixture." CI lint-matrix gates cover rule → fixture; these close the remaining verifiability claims.

- **Provenance emission completeness audit** `[7 / 4 / 5]` (**35**) — verify every WOS MUST that produces an audit event actually emits the provenance record. Distinct from `every_promoted_*_rule_has_executable_or_annotated_evidence`: that checks rules; this checks MUST → emission. Sequences after #22a ProvenanceKind tier-typing so the audit runs against the tier-split structure, not pre-split `provenance.rs`.
- **Kernel-Basic conformance profile LoadBearing declaration** `[5 / 2 / 3]` (**15**) — promote the profile; fixtures already exist via the shared conformance suite. One-line declaration plus any missing lint-matrix wiring.

### Regulatory — 1.0 separate-spec deliverables

Per vision model, these are 1.0 deliverables (not deferred) because spec writing is cheap under minutes-not-days and the compliance posture is load-bearing for the SBA adopter.

- **#50 EU AI Act alignment** `[7 / 5 / 4]` (**28**) — Art. 13-14 alignment spec.
- **#53 OMB M-24-10 compliance** `[6 / 4 / 3]` (**18**) — process-documentation-shaped; overlaps Assurance + impact-level plumbing.

### Interoperability + speculative (trigger-gated)

- **SCXML interoperability** `[3 / 6 / 2]` (**6**) — bidirectional WOS ↔ SCXML mapping. Trigger: ecosystem demand.
- **#51 Statutory deadline chains** `[4 / 7 / 5]` (**20**) — must compose with #31 business calendars + #20 typed events. Trigger: first production deployment exposes concrete need.

---

## Moved to Trellis (scope-out)

Per vision model, Trellis is the integrity layer and owns these concerns. WOS emits records via `custodyHook`; Trellis anchors them. Tracked here only to close the loop on items that used to be listed as WOS work.

- **#48 Merkle provenance chains** — Trellis. Hash-chaining + SCITT alignment are Trellis primitives.
- **Federation Profile** (cooperative trust-anchor network) — Trellis. Previously tracked as WOS Future spec.
- **SCITT strictness** (full vs. adjacent) — Trellis decides.
- **Checkpoint seal protocol** — Trellis.
- **Proof-of-inclusion + transparency-log submission tooling** — Trellis.
- **Certificate-of-completion export bundle format** — Trellis export-bundle primitive.

---

## Blocked / needs decision

Items that can't move without a verdict or an external trigger.

### §4.5 PR packaging

Vision model recommends three discrete PRs; sidecar audit recommended one. Owner picks. See Do-next-adjacent "Structural merges" section above.

### Engine adapters — trigger-gated (commercial request)

WOS's production runtime is now the Temporal/Restate adapter (Do next #5). Additional adapters are trigger-gated on commercial adopter request.

- **#49a Camunda 8 Worker** `[5 / 8 / 3]` — BPMN target; broadest external fixture diversity.
- **#49c AWS Step Functions** `[5 / 8 / 3]` — broadest commercial reach; narrowest semantic fit.

(#49b Temporal moved into Do next #5 as the production runtime choice.)

### Ontology field identity — design not started

`ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and §6 regulatory specs. Prerequisite design: JSON-LD `@context` decision, semantic-field-identity protocol, cross-document alignment. Move to active only once a draft exists.

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

Federation Profile and Bulk Operations relocated — see "Moved to Trellis" and Backlog / behavioral items respectively.

| Spec | Description | Trigger |
|---|---|---|
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

Most prior open questions (OQ1, OQ4, #21 Registry composition, #25 Defeasibility, #36 Equity expression, #43 Assurance × impact-level, #9 JSON-LD authoring) are now **resolved** per the vision model's [Settled architectural commitments](../.claude/vision-model.md#settled-architectural-commitments). Remaining genuinely-open decisions:

1. **α — DocuSign parity bar** (gates Signature Profile drafting). Default per vision model: ESIGN/UETA/eIDAS compliance + top ~80% of DocuSign common-case workflow features; NOT administrative UX surface. Confirm when drafting begins.
2. **§4.5 PR packaging** (sidecar-audit Q1). One PR (audit recommendation) or three (vision-model recommendation)?
3. **`custodyHook` Trellis contract shape** — joint-design ADR between WOS and Trellis. Load-bearing for WOS 1.0 closure. Tracked as Do next #3.

For stack-wide active uncertainties (backend spike γ, wos-runtime role δ, SBA timeline, multi-tenant model, rendering service), see [vision-model.md § Active uncertainties](../.claude/vision-model.md#active-uncertainties-wos-scope).

---

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
