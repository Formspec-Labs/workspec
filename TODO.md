# WOS TODO

Working backlog for the Workflow Orchestration Standard specification suite. Session narratives and all closed items live in [COMPLETED.md](COMPLETED.md); architectural commitments and scope lines live in the [stack-wide vision model](../.claude/vision-model.md); this file indexes active work, blocked items, and trigger-gated future work.

**Last audited:** 2026-04-22 — WOS-T4 Signature Profile runtime/lint/conformance + SIG-* fixtures green; WOS-T1 ADR-0061 `custodyHook` four-field append + receipt stamping closed in code/schemas; semi-formal review follow-ups: `ProvenanceRecord.id` required on serde (no silent mint), `typeid::tenant_from_env_value` + env-free unit tests, `FixtureFormspecProcessor` dead_code reservation. Cross-repo Formspec signed-response fixture, Trellis verification corpora drift checks, and Studio authoring/validation UX remain open (WOS-T4 next slice).

## Snapshot

| Health | Value |
|---|---|
| Specs / schemas | 41 spec/docs under `specs/` · 27 schemas · 0 SCHEMA-DOC-001 violations |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) · 6 MVP (`wos-authoring`, `wos-mcp`, `wos-synth-core/-mock/-anthropic/-cli`) · 1 spike (`wos-synth-spike`, keep-with-deletion-horizon) |
| Tests | Latest targeted gates: `cargo check --workspace` green; `cargo test -p wos-core --lib` green; `cargo test -p wos-runtime --lib` green; `cargo test -p wos-lint` green; `cargo test -p wos-conformance --test signature_profile` 13 green; `pytest tests/schemas -q` 255 passed / 12 skipped / 1 xfailed |
| Lint matrix | 116 rules (35 T1 · 72 T2 · 9 T3 · 1 LoadBearing · 11 Tested · 104 Draft) |
| CI gates | `schema_doc_zero_regression` · `every_promoted_*_rule_has_executable_or_annotated_evidence` · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Navigation:** [**User profile** (read first)](../.claude/user_profile.md) · [**Vision model**](../.claude/vision-model.md) (stack-wide; WOS section inside) · [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [ADRs](../thoughts/adr/) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR.

**Scoring note.** Per [`user_profile.md`](../.claude/user_profile.md) economic model: dev/time is free, architectural drift is expensive. Ordering uses **`Imp × Debt`**; Cx is preserved as a scheduling dimension but does not change priority. Debt values trend **up** between sessions on pre-1.0 work. Score notation: `[Imp / Cx / Debt]`; the number in parentheses is `Imp × Debt`.

1. **Signature Profile workflow semantics** `[7 / 5 / 5]` (**35**) — **WOS-T4 ACTIVE.** DocuSign common-case workflow semantics for WOS: signer roles via `actorExtension`, sequential/parallel/routed/free-for-all flows, intent capture, identity binding, signer-authentication policy schema, reminders, expiry, decline, void, reassignment, and `SignatureAffirmation` provenance. Cryptographic integrity + certificate-of-completion live in Trellis; WOS only emits the semantic evidence record. **Execution plan:** [T4-TODO.md](T4-TODO.md). **Landed 2026-04-22:** [ADR-0062](thoughts/adr/0062-signature-profile-workflow-semantics.md), [Signature Profile spec](specs/profiles/signature.md), [Signature Profile schema](schemas/profiles/wos-signature-profile.schema.json), schema fixtures/tests, Studio generated type binding, `ProvenanceKind::SignatureAffirmation`, schema-constrained `SignatureAffirmation` payload, Rust constructor/helper, Facts-tier classification, custody append inclusion, SIG-001..SIG-012 lint, runtime profile loading, signing task evidence validation, `SignatureAffirmation` emission, sequential/parallel/routed/free-for-all/witness/notary/decline/void/reassignment/expiry semantics, and 13 SIG-* conformance tests. **Next T4 slice:** cross-repo Formspec canonical signed-response fixture, Trellis custody/export vector, and Studio authoring/validation UX.

   **WOS-T4 -COMPLETE- criteria:** Formspec captures signing/consent evidence; WOS routes signing through lifecycle/governance semantics; WOS emits `SignatureAffirmation`; Trellis accepts and anchors the record through `custodyHook`; conformance proves sequential, parallel, routed, free-for-all, expiry, decline, reassignment, witness/counter-signature, notary/in-person authentication, missing-consent rejection, and custody append inclusion.
2. **Provenance emission completeness audit** `[7 / 4 / 5]` (**35**) — verify every WOS MUST that produces an audit event actually emits the provenance record. Distinct from lint-matrix rule evidence: this checks MUST → emission. **Unblocked:** #22a ProvenanceKind tier-typing landed.
3. **Actor authorization shape (`AuthorizationAttestation`)** `[7 / 4 / 5]` (**35**) — stack contract per [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) D-2. **Gate: ADR 0066 accepted.**
4. **ADR 0066 implementation — amendment / supersession / rescission / correction** `[7 / 6 / 5]` (**35**) — six provenance record kinds, `caseRelationship.type = supersedes`, Workflow Governance policy sections, exporter coverage. **Gate: ADR 0066 accepted.**
5. **ADR 0067 implementation — statutory clocks** `[7 / 5 / 5]` (**35**) — `ClockStarted` / `ClockResolved`, `Clock` $def, AppealClock emission, ProcessingSLA emission. **Gate: ADR 0067 accepted.**

### Agent task extract (from this file)

| Task ID | Tracks | Deliverable | Depends on |
|---------|--------|-------------|------------|
| **WOS-T2** | Do next **#2** | [ADR-0060](thoughts/adr/0060-cross-reference-naming-ref-key-id.md) accepted + governance schema/runtime/lint/test sweep | **-COMPLETE-** — Workflow Governance taxonomy closed; G-063/G-066 enforce remaining key/id resolution |
| **WOS-T3** | Do next **#3** | `DurableRuntime` trait + Temporal/Restate spike + tenant-scope notes | **-COMPLETE-** — Restate selected; Temporal deferred; tenant-scope contract recorded |
| **WOS-T4** | Do next **#1** | Signature Profile spec slice + schemas + runtime/conformance | **ACTIVE** — WOS-side runtime/lint/conformance landed; cross-repo Formspec/Trellis/Studio gates remain |
| **WOS-B1** | Backlog | §4.5 structural merges (1 vs 3 PRs) | Owner packaging decision |
| **WOS-B2** | Backlog | Kernel-Basic profile **LoadBearing** declaration + lint-matrix wire | None |

*Falling off Do next at Imp × Debt < 30:* §4.5 structural merges (owner decision needed), §5.5 `wos-bench` (18), §4.4 release-trains Tasks 4-5 (15). All live in Backlog.

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

**Stack contracts (ADR 0066 + 0067, proposed 2026-04-21):**

- **Actor authorization shape (`AuthorizationAttestation`)** `[7 / 4 / 5]` (**35**) — stack contract per [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) D-2. Declares the Facts-tier record shape for a human act performed under a named policy; parallel to AI deontic constraints. Binds Workflow Governance policies (`amendmentPolicy`, `rescissionPolicy`, future `authorizationPolicy`) to the attestation record. Cheap — IAM is adapter; claim shape is center. **Gate: ADR 0066 accepted.**
- **Identity attestation shape — generalize beyond signatures** `[5 / 3 / 4]` (**20**) — WOS-T4 runtime emission now has `SignatureAffirmation.identityBinding` as the first concrete shape. This item generalizes that shape for reuse across non-signature evidence (reviewer-policy assurance refs, amendment-authority attestations, review-gate credentials). Lifts the Signature Profile per-field shape into a reusable `$def` after runtime emission proves the shape is sufficient. **Gate: T4 runtime emission landed.**
- **ADR 0066 implementation — amendment / supersession / rescission / correction** `[7 / 6 / 5]` (**35**) — six provenance record kinds (`CorrectionAuthorized`, `AmendmentAuthorized`, `DeterminationAmended`, `RescissionAuthorized`, `DeterminationRescinded`, `AuthorizationAttestation`); `caseRelationship.type` extension with `supersedes`; `amendmentPolicy` and `rescissionPolicy` Workflow Governance sections; exporter coverage for the six kinds. **Gate: ADR 0066 accepted.**
- **ADR 0067 implementation — statutory clocks** `[7 / 5 / 5]` (**35**) — `ClockStarted` / `ClockResolved` provenance record kinds (Facts tier); `Clock` $def in kernel schema; AppealClock emission wired to adverse-decision transition path; ProcessingSLA wired to intake-complete. Softens #51 (trigger-gated) — a contract now exists. **Gate: ADR 0067 accepted.**

**Prior behavioral items:**

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

- **#22 Crate split along tier boundaries** `[5 / 3 / 3]` (**15**) — `wos-core` → `wos-{kernel,governance,ai,advanced}`; `wos-runtime/runtime.rs` (still a large single module; ≈3.7k lines) split along action-kind dispatch; CI fence. (**#22a** provenance module split + `ProvenanceAuditTier` landed 2026-04-21 — see [COMPLETED.md](COMPLETED.md).)

### Audit + evidence products

Build on the stable provenance export surface. #48 Merkle provenance moved to Trellis scope; see "Moved to Trellis" below.

- **#52 Simulation trace format** `[4 / 3 / 2]` (**8**) — normative replay contract + conformance fixtures. Event log format already shipped via `wos-export::xes`.

### Verifiability closure (1.0)

Per [vision-model.md v1.0 scope snapshot](../.claude/vision-model.md#v10-scope-snapshot--the-7-ratification-gates): "every normative MUST across Kernel + Governance + AI Integration has a passing Tested fixture." CI lint-matrix gates cover rule → fixture; these close the remaining verifiability claims.

- **Provenance emission completeness audit** `[7 / 4 / 5]` (**35**) — verify every WOS MUST that produces an audit event actually emits the provenance record. Distinct from `every_promoted_*_rule_has_executable_or_annotated_evidence`: that checks rules; this checks MUST → emission. **Unblocked:** #22a ProvenanceKind tier-typing landed (2026-04-21); audit runs against the tier-split structure.
- **Kernel-Basic conformance profile LoadBearing declaration** `[5 / 2 / 3]` (**15**) — promote the profile; fixtures already exist via the shared conformance suite. One-line declaration plus any missing lint-matrix wiring.

### Regulatory — 1.0 separate-spec deliverables

Per vision model, these are 1.0 deliverables (not deferred) because spec writing is cheap under minutes-not-days and the compliance posture is load-bearing for the SBA adopter.

- **#50 EU AI Act alignment** `[7 / 5 / 4]` (**28**) — Art. 13-14 alignment spec.
- **#53 OMB M-24-10 compliance** `[6 / 4 / 3]` (**18**) — process-documentation-shaped; overlaps Assurance + impact-level plumbing.

### Interoperability + speculative (trigger-gated)

- **SCXML interoperability** `[3 / 6 / 2]` (**6**) — bidirectional WOS ↔ SCXML mapping. Trigger: ecosystem demand.
- **#51 Statutory deadline chains** `[4 / 7 / 5]` (**20**) — must compose with #31 business calendars + typed kernel events (`TransitionEvent`, #20). Trigger: first production deployment exposes concrete need.

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

WOS's first production runtime target is now the Restate adapter selected by WOS-T3. Additional adapters are trigger-gated on commercial adopter request or SDK maturity.

- **#49a Camunda 8 Worker** `[5 / 8 / 3]` — BPMN target; broadest external fixture diversity.
- **#49c AWS Step Functions** `[5 / 8 / 3]` — broadest commercial reach; narrowest semantic fit.

(#49b Temporal was evaluated by WOS-T3 and deferred until the Rust workflow API stabilizes.)

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
| #32 | Multi-Instance Iteration | 6/7/5 | #20 landed — unblocked. Highest-priority deferred item. |
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
3. **`custodyHook` evidence path to Trellis** — WOS→Trellis **authored append wire** is fixed by [ADR-0061](thoughts/adr/0061-custody-hook-trellis-wire-format.md) and WOS-T1 closeout (four-field input, `(caseId, recordId)` idempotency, `CustodyAppendReceipt` → `canonical_event_hash` on provenance). **Open** is cross-stack **proof**: Formspec canonical signed-response artifacts, Trellis append/export vectors staying byte-aligned with live WOS emitters, and Studio authoring gates — same bundle as WOS-T4 “Next slice” / Trellis verification maintenance, not an undecided WOS wire-format ADR.

For stack-wide active uncertainties (backend spike γ, wos-runtime role δ, SBA timeline, multi-tenant model, rendering service), see [vision-model.md § Active uncertainties](../.claude/vision-model.md#active-uncertainties-wos-scope).

---

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
