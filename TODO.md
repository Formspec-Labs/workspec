# WOS TODO

Active backlog. Completed work → [COMPLETED.md](COMPLETED.md). Stack architecture → [`thoughts/adr/`](../thoughts/adr/). Ordering = **`Imp × Debt`** per economic model in [`../CLAUDE.md`](../CLAUDE.md) §Behavioral interrupts (dev/time free, architectural drift expensive); Cx is scheduling-only. Score notation in ticket headers: `[Imp / Cx / Debt]`. Last audited 2026-05-15.

## Snapshot

**Where specs / schemas / crates live:** invoke the `formspec-specs:wos-core` skill — it carries the 4-layer architectural navigation (L0 Kernel / L1 Governance / L2 AI / L3 Advanced), the sidecar + profile + companion inventory, the 19 REST API spec/schema pairs, and the complete `work-spec/` file map. Don't inline that content here; it drifts.

| Operational state | |
|---|---|
| CI ratchets | `schema_doc_zero_regression` · promotion-evidence + LoadBearing-fixture coverage · promotion-candidate ratchet · **ADR 0082 D-13 Gates 1–6** (schema validity, OpenAPI `$ref` discipline, route coverage, `oasdiff` breaking-change, response conformance, mirror byte-parity) under [`.github/workflows/api-contract-guardrails.yml`](.github/workflows/api-contract-guardrails.yml) |
| Case/process identity | ADR-0093 Option B groundwork in progress (process TypeID family, `WosResourceUrn` admits `process`, N:1 process-to-case-ledger bridge routes wired). Remaining: `WorkflowProcess` rename, `/instances` → case/process route migration, post-ledger ReBAC, generic `DecisionEvent`, broader N:1 conformance. Session detail in [`COMPLETED.md`](COMPLETED.md). |
| E2E/API coverage | Server tests mostly Rust integration / in-process HTTP. Playwright API + browser E2E plan against real `wos-server`, manifest-driven from [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) 130 rows: [`../thoughts/plans/2026-05-09-wos-feature-matrix-e2e-test-plan.md`](../thoughts/plans/2026-05-09-wos-feature-matrix-e2e-test-plan.md). |
| Agent adapter state | `wos-agent-stub` ships; 5 skeleton crates (`-anthropic/-mcp/-a2a/-http/-claude-sdk`) have `unimplemented!()` invoke bodies pending orchestrator seam (ADR 0064 residual epic). |

**Navigation:** [**Interrupts + model** (read first)](../CLAUDE.md) · [**Stack ADRs**](../thoughts/adr/) · [`work-spec/CLAUDE.md`](CLAUDE.md) · [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Gate typology & parallelization plan

Sequenced across 5 independent workstreams. Full gate audit + ADR ratification history archived to [`thoughts/plans/2026-05-06-gate-typology.md`](thoughts/plans/2026-05-06-gate-typology.md). Stream A (Identity) fully landed 2026-05-07 (#6 ADR 0092 TypeID + #7 multi-step session DAG); row retired.

| Stream | Items | Work type | Scope |
|--------|-------|-----------|-------|
| **B — Provisioning** | #3, #4, #5, #70, #71, #72, #66 (and #66a–#66g sub-items), #40 | Governance policies, runtime emission wiring, export, conformance, Runtime Companion parity | WOS center + wos-export + wos-conformance + wos-runtime |
| **C — Spec & Schema** | #26a, #26b, #50, #28, #30, #27, #29b, #53, Bulk Ops, #32, #35, #36, #38, #59, #60, #61, identity attestation, #24b/#25, #3 (migration) | Schema additions, spec prose, conformance fixtures, lint impl | WOS schemas + specs + lint |
| **D — Authoring & Tooling** | ADR 0065 cluster (`fs-uqpw`/`fs-d79a`/`fs-xkui`/`fs-s5wm`/`fs-k63f` — production seam, MCP validation, spike closure, provider hygiene, plan reconcile), §5.5 `wos-bench`, §4.4 release trains, §5.6 | MCP/synth/authoring cleanup, benchmarking, CI, docs | `wos-{mcp,synth-core,authoring,bench}` crates |
| **E — Cross-repo** | #1 T4 closeout, #2 AI orchestrator + 5 agent adapters, identity attestation cross-repo | Trellis COC, Studio UI, AgentRuntime trait, cross-repo ADR coordination | Trellis + formspec-studio + WOS crates |
| **F — Envelope productization** (added 2026-05-14) | Content-Attestation Envelope epic `fs-m8ic` + 15 child tickets — port-catalog spec (P-1..P-14), verifier facade (P-13), conformance bundles, two design decisions (`fs-n357` `documentRef` shape, `fs-wi8a` intent baseline, `fs-fqt1` port-catalog home) | DocuSign-class envelope-routed signed-content evidence | Formspec Definition + Response schema, WOS Signature Profile §2.11/§2.13.1, Trellis clarifying ADR, port catalog (likely WOS sidecar), verifier facade |

**Stream-internal sequencing:**
- **B:** Governance policies (#3/#4/#71) independent once ADRs accepted; #70 → #72 (typed-enum prerequisite for emission wiring, encoded as `fs-zjpl` deps `fs-vmsx`); export work (#4/#5) independent of runtime emission; #66a–#66g mostly independent
- **C:** #26a → #26b (encoded); #30 pairs with #58 (landed 2026-05-07); #35 → #36 (encoded); #59/#60 dependency on typed events + task lifecycle lives in ticket prose, not encoded as deps
- **D:** ADR 0065 production-seam ticket parents the synth/MCP follow-ups; plan-reconcile (`fs-k63f`) → production-seam (`fs-uqpw`); spike-closure tickets independent
- **E:** AI orchestrator (#2) blocks the 5 agent adapter invoke bodies (`fs-ixl3`/`fs-mhhv`/`fs-njef`/`fs-pizw`/`fs-wzps` all dep `fs-w9dv`); T4 COC and Studio UI independent
- **F:** Port-catalog home decision (`fs-fqt1`) → port-catalog spec (`fs-acyi`) → verifier facade (`fs-jjwc`) → conformance bundles (`fs-6pb7` → `fs-o1lw`); two foundational design choices (`fs-n357`, `fs-wi8a`) gate the schema cascade

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR.

<!-- tk:start epic=fs-ttv6 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-ttv6 — edit tickets via tk, not this section -->

Cross-repo Do-next #1 (WOS-T4) — workflow-tier signature slice; cross-stack rollup at `fs-w0li`. Other items below are WOS-spec-internal slices, parented here.

**P2 — standard:**

- **#4 — ADR 0066 implementation (WOS center: 7 record kinds + policies + runtime + export)** `[7/6/5]=35` · `fs-j94f` · P2
   blocked-by Actor authorization shape (AuthorizationAttestation) —… `fs-u2on` · links ADR 0066 — amendment and supersession (cross-stack rollup) `fs-lwsj`

   WOS center slice of ADR 0066 (amendment/supersession/rescission/correction). ADR Accepted 2026-05-06; cluster ratification sweep complete. LANDED: 7 ProvenanceKind variants + schema $defs + Facts-tier classification + audit_layer_for_kind tests + 2/3 export paths (PROV-O + XES). REMAINING (per checklist :256-281): governance policy sections (amendmentPolicy, rescissionPolicy, reinstatementPolicy, correctionPolicy, supersessionPolicy) each binding AppealMechanism-shaped gate + impact-level assurance floor; lint K-A-010; companion examples + fixtures emitting superseding workflow process + runtime targetCase URI validation; runtime emission on governed transitions (gated by #72); OCEL export; wos-conformance fixtures; WOS-IMPLEMENTATION-STATUS.md updates.

   **Acceptance:**
     - All 6 checklist sections green (kernel/provenance done; workflow governance, caseRelationship, runtime+binding, OCEL export, conformance+docs remaining).
     - Composes onto LB_T4 (fs-w0li — workflow-tier slice) and parent ADR 0066 rollup (fs-lwsj).

- **#5 — ADR 0067 implementation (WOS center: clocks runtime + export + conformance)** `[7/5/5]=35` · `fs-lq08` · P2
   blocked-by Task SLA runtime implementation `fs-96nd` · links ADR 0067 — statutory clocks (cross-stack rollup) `fs-olho`, Align notice evidence and deadline requirements across WOS… `fs-hltb`, ADR 0067 — Runtime clock emission, ResponseClock and… `fs-7yfo`, WS-073 — ADR 0067 reference prove-out (HTTP/storage) `fs-f8vm`

   WOS center slice of ADR 0067 (statutory clocks). ADR Accepted 2026-05-06. LANDED: clockStarted / clockResolved ProvenanceKind + schema $defs (Clock with clockId/clockKind/originEventHash/duration/calendarRef/statuteReference/computedDeadline + ClockResolved with resolution/resolvingEventHash/resolvedAt) + Facts-tier classification + Task SLA authoring surface + business calendar §7.1 infrastructure. REMAINING (per checklist :283-308): runtime emission for 4 clock kinds (AppealClock / ProcessingSLA / GrantExpiry / StatuteClock) + pause/resume per D-4; Task SLA runtime + ADR 0067 D-2.1 SlaDefinition→ProcessingSLA migration; #51 statutory deadline chain composition with §7.1; wos-export PROV-O/OCEL/XES paths for clock kinds; wos-conformance fixtures (start, satisfied, elapsed, paused segment); kernel/companion prose for MUST behavior; ADR §Open questions resolution.

   **Acceptance:** All 6 checklist sections green. Cross-stack rollup ADR_67 (fs-olho); workspec-server prove-out at wos-server WS-073.

- **#3 — Actor authorization shape (AuthorizationAttestation) — governance policy + runtime emission** `[7/4/5]=35` · `fs-u2on` · P2
   links Admit and run selected workflow governance through… `fs-ps24`

   Stack contract per ADR 0066 D-2. WOS-center provenance ALREADY LANDED: ProvenanceKind::AuthorizationAttestation at kind.rs:350, schema $defs/AuthorizationAttestationRecord, Facts-tier classification, export adapters (PROV-O/XES; OCEL pending). REMAINING: governance policy sections + runtime emission wiring. ADR 0066 Accepted 2026-05-06.

   **Acceptance:**
     - Governance policy sections (workflow-governance.md) bind AuthorizationAttestation to authorizing actor; runtime emits on governed transitions; OCEL export path.
     - Tracked in ADR 0066 execution checklist items 1-2.

- **#10 — WOS-SIGNATURE-RECEIPT-CONSUMPTION-001 — VerificationReceipt consumption in wos-formspec-binding** `[6/4/5]=30` · `fs-f5ls` · P2
   links WOS-T4 cross-stack proof + signature-attestation /… `fs-w0li`

   LANDED: VerificationReceipt types (Rust + TS); signature_evidence() signature ready for receipt injection; shared Formspec COSE helpers + Ed25519 webcrypto/ring/Trellis adapters verify COSE_Sign1; runtime carries optional Formspec authoredSignatures[*].verificationReceipt bytes through SignatureEvidence into SignatureAffirmation.verificationReceipt; Posture Declaration receiptSigningRequired:true emits SignatureAdmissionFailed(posture_floor_unmet) when signed receipt bytes absent. REMAINING: dispatch to real Verifier port; produce signed VerificationReceipt bytes from adapter; map failed/unsupported receipt results into SignatureAdmissionFailed.receipt; populate byte bundles with real receipt COSE.

   **Acceptance:**
     - Real Verifier port dispatch; signed receipt bytes produced; receipt failure mapping; byte bundles carry real receipt COSE.
     - Debt 5: COSE decode shared but adapter dispatch + receipt injection remain deep surfaces.

- **#11 — WOS-POSTURE-DECLARATION-CONSUMPTION-001 — Posture Declaration consumption residual** `[6/4/5]=30` · `fs-sf0g` · P2
   links WOS-T4 cross-stack proof + signature-attestation /… `fs-w0li`

   LANDED: posturePolicy field in wos-workflow.schema.json (url + optional version); deploymentLocalSigningIntents sunset notice; PostureDeclaration schema in formspec/schemas/ and work-spec/schemas/; load_posture_declaration URL fetch/parse/cache path in wos-runtime; posture-floor admission-path tests; posture allowedMethods gates on binding-supplied signatureMethod; posture allowedSigningIntents rejects non-allowlisted intents with SignatureAdmissionFailed(method_unsupported). REMAINING: binding-driven registry-corrupt / adapter-unavailable conformance coverage once receipt/verifier dispatch exists.

   **Acceptance:** Conformance coverage of registry-corrupt and adapter-unavailable paths via binding-driven fixtures. Debt 5: posture loading adds HTTP fetch + cache + version-pinning surface.

- **#8 — WOS-SIGNATURE-ADMISSION-FAILED-RECORD-001 — SignatureAdmissionFailed wiring residual** `[7/5/4]=28` · `fs-wa59` · P2
   links WOS-T4 cross-stack proof + signature-attestation /… `fs-w0li`

   LANDED: SignatureAdmissionFailedRecord $def in wos-provenance-log.schema.json + SignatureAdmissionFailedData with 7-reason enum + recordKind enum entry + ProvenanceKind::SignatureAdmissionFailed variant + Facts-tier classification + Rust constructor + 7-reason runtime test coverage + Formspec binding parses/carries signatureMethod / reports MethodUnregistered + EvidenceDivergence / runtime posture allowedMethods/allowedSigningIntents enforcement + unregistered-but-step-matching signing intents reach admission-failure branch + runtime step intent/signer-authority/consent signedAt divergence gates emit SignatureAdmissionFailed with structured failureContext. REMAINING: actual Formspec verifier/posture/registry-corrupt/adapter paths populating remaining nonprimitive binding-reported failure reasons beyond current runtime/test adapters.

   **Acceptance:**
     - All 7 admission-failure reasons reachable from real Formspec verifier/posture/registry/adapter paths (not only test adapters).
     - Debt 4: schema + runtime structure landed; wiring to all reason paths remains.

**P3 — sustaining:**

- **#2 — AI-runtime capability-precondition emission wiring** `[6/5/4]=24` · `fs-w9dv` · P3
   blocked-by Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m` · links Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m`

   Typed Rust path landed 2026-04-28 (5 unit + 6 Python schema tests pass). REMAINING: (a) runtime emission site — AI §3.3.1 step 1-3 specifies precondition evaluation but no runtime path actually evaluates Capability.preconditions (crates/wos-core/src/model/ai.rs:197); declarable but not fired. (b) JSON conformance fixture pair (blocked + permitted) under fixtures/conformance/. (c) Ergonomic constructor variant once call-site count justifies. BLOCKER (verified 2026-05-14): ADR 0064 AgentInvoker port landed (trait + 6 adapter crates) and WosRuntime accepts AgentInvokerRegistry, but ORCHESTRATOR that calls AgentInvoker::invoke() from transition execution still does not exist. DurableRuntime has no invoke_agent method; new AgentRuntime trait or method addition may be required.

   **Acceptance:**
     - The maintained runtime action path invokes AgentInvoker only after native capability selection, preconditions, required input rights and input validation pass.
     - Emit the provenance defined by the reviewed AI Integration §3.3.1: blocked capabilityInvocation records retain data.invocationBlocked=true and outcome=preconditionNotSatisfied; do not invent an alternative record kind.
     - Independently authored permitted/blocked JSON fixtures observe actual invocation count, received input and retained provenance.
     - Use the shared native loader and ConfidenceReport type from fs-k07m.
     - GATE: reviewed invocation action path, with output validation/review/governed commit composed through fs-t9h8 when that outcome is claimed.

- **#14 — WOS-SIGNATURE-ADMISSION-REVOCATION-001 — revocation_policy enforcement** `[7/3/2]=14` · `fs-yciq` · P3
   links WOS-T4 cross-stack proof + signature-attestation /… `fs-w0li`, K-DET-001 — Determination-snapshot conformance + fixture… `fs-91cr`, SNAP-SPINE-001 — UWU-5 extracted: revocation-operations… `fs-q1ut`

   wos-runtime/src/runtime/signature.rs:1182 carries TODO(Phase 3.3): enforce revocation_policy when present. SignaturePolicy.revocation_policy field declared (line 686) but never inspected after admission gate at line 1182 — admission succeeds even when revocation would block. No prior TODO.md entry tracked this gap.

   **Acceptance:**
     - Admission path inspects revocation_policy and rejects when policy blocks; negative conformance fixture proves revocable signer rejected after revocation; positive fixture proves non-revoked signer still admits.
     - WHY: signature profile production-readiness gates on complete enforcement; deployment setting revocation_policy gets silent non-compliance today.

**Recently closed (kept for traceability; archive when stale):**

- ~~**#9 — WOS-FORMSPEC-CANONICALIZATION-001 — Consume Formspec canonical signed-payload helper**~~ `fs-q6kt` · CLOSED — wos-formspec-binding now depends on formspec-canonical for signedPayload.digest computation and no longer owns a local serde_json_canonicalizer + NUL-separated domain clone;…
- ~~**#6 — ADR 0092 — TypeID-in-URN identity landing**~~ `fs-cccd` · CLOSED — Narrow WosResourceUrn from 5-segment to 3-segment urn:wos:<typeid>.
- ~~**#12 — WOS-CONFORMANCE-SIG-FIXTURES-001 — SIG-027..030 + re-cast SIG-014..026**~~ `fs-j0vj` · CLOSED — SIG-014/SIG-017 prove Formspec signedPayload pin/digest divergence as SignatureAdmissionFailed(evidence_divergence); SIG-015 proves step signing-intent mismatch;…
- ~~**#7 — Multi-step session DAG topology (P2, §21 from API coverage audit)**~~ `fs-n72b` · CLOSED — Schema-only close of the single deferred gap from API coverage audit.
- ~~**#13 — WORKSPEC-SERVER-SIGNATURE-FIXUP-001 — workspec-server integration test fixup**~~ `fs-tdpx` · CLOSED — workspec-server now compiles against current WorkflowProcess field set; signature read service exposes separate SignatureAffirmation and SignatureAdmissionFailed provenance…

<!-- tk:end -->

---

## ADR execution checklists

### ADR 0066 — execution checklist (WOS center) {#adr-0066-exec-checklist}

**Gate:** [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) **Accepted 2026-05-06 (cluster ratification sweep).** Formspec Respondent Ledger work and Trellis vectors/verifier/export stay owned in parent [`TODO-STACK.md`](../TODO-STACK.md) and closed Trellis Waves 40/47 in [`../trellis/COMPLETED.md`](../trellis/COMPLETED.md); this block is the **WOS spec + schema + runtime + export** slice.

<!-- tk:start epic=fs-w5bz -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-w5bz — edit tickets via tk, not this section -->

Gate: ADR 0066 Accepted 2026-05-06 (cluster ratification sweep). Formspec Respondent Ledger work + Trellis vectors/verifier/export stay owned in parent TODO-STACK.md and closed Trellis Waves 40/47; this is the WOS spec + schema + runtime + export slice.

**P2 — standard:**

- **ADR 0066 §2 — Workflow Governance (5 policy sections + K-A-010 lint)** · `fs-b4sa` · P2
   blocked-by ReinstatementPolicy schema $def + lint K-A-010 `fs-g5nu` · links Align canonical workflow governance schema, typed model and… `fs-duw8`

   Per ADR 0066 D-2. Zero landed. Cross-link: W_71 (fs-g5nu) covers the ReinstatementPolicy $def + K-A-010 lint slice.

   **Acceptance:** Normative policy sections — amendmentPolicy, rescissionPolicy, reinstatementPolicy, correctionPolicy, supersessionPolicy — each binding an AppealMechanism-shaped gate; impact-level assurance floor (rights-impacting → authorizing actor Assurance ≥ high) per D-2; lint rule K-A-010 enforces closed five-mode amendment taxonomy; conformance fixtures.

- **ADR 0066 §6 — Conformance + docs (wos-conformance fixtures per kind + matrix update)** · `fs-hsgi` · P2

   Zero landed.

   **Acceptance:** wos-conformance fixtures per kind; WOS-IMPLEMENTATION-STATUS.md / matrix rows updated as applicable.

- **ADR 0066 §4 — Runtime + binding (governed-transition emission, intake protection)** · `fs-k54o` · P2
   blocked-by Reference-runtime emission wiring for cluster variants `fs-zjpl`

   Zero landed. Cross-link: W_72 (fs-zjpl) covers the broader reference-runtime emission wiring for all 14 cluster variants; this slice is the ADR 0066 subset.

   **Acceptance:**
     - wos-runtime (and wos-formspec-binding where intake/custody intersects) emit the 7 new records on governed transitions; intake paths never silently mutate prior responses when a correction lineage exists (ADR Context).
     - GATE: W_72 ($W_72).

**P3 — sustaining:**

- **ADR 0066 §3 — caseRelationship.type = supersedes (companion examples + runtime URI validation)** · `fs-mb3o` · P3

   Schema enum includes supersedes at schemas/api/instance.schema.json:945,957; Kernel prose at specs/kernel/spec.md:772; K-048 lint enforces x- prefix for non-standard values. Remaining: examples + runtime validation.

   **Acceptance:** Companion examples + fixtures emitting superseding workflow process; runtime validation of targetCase URI shape.

**Recently closed (kept for traceability; archive when stale):**

- ~~**ADR 0066 §1 — Kernel / provenance (7 ProvenanceKind variants + tier map + tests)**~~ `fs-xe7l` · CLOSED — Seven ProvenanceKind variants (correctionAuthorized, amendmentAuthorized, determinationAmended, rescissionAuthorized, determinationRescinded, reinstated, authorizationAttestation)…
- ~~**ADR 0066 §5 — Export (OCEL event types for amendment/supersession kinds)**~~ `fs-z3ll` · CLOSED — wos-export: PROV-O + XES event types for all 7 kinds landed (prov_o.rs:685-691, xes.rs:683-689).

<!-- tk:end -->

### ADR 0067 — execution checklist (WOS center) {#adr-0067-exec-checklist}

**Gate:** [ADR 0067](../thoughts/adr/0067-stack-statutory-clocks.md) **Accepted 2026-05-06 (cluster ratification sweep).** Trellis `open-clocks.json`, verifier advisories, append vectors **043–046**, `verify/018-export-043-open-clocks`, and `tamper/051-clock-calendar-mismatch` stay in parent [`TODO-STACK.md`](../TODO-STACK.md) and closed Trellis Wave 39 in [`../trellis/COMPLETED.md`](../trellis/COMPLETED.md); Formspec **StatuteClock** origination on respondent acts stays in parent [`TODO-STACK.md`](../TODO-STACK.md); reference-server prove-out is [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-073**.

<!-- tk:start epic=fs-m25i -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-m25i — edit tickets via tk, not this section -->

Gate: ADR 0067 Accepted 2026-05-06. Trellis Wave 39 closed; Formspec StatuteClock stays in parent TODO-STACK.md; reference-server prove-out at WS-073.

**P2 — standard:**

- **ADR 0067 — Runtime clock emission, ResponseClock and pause/resume** · `fs-7yfo` · P2
   links Task SLA runtime implementation `fs-96nd`, Align notice evidence and deadline requirements across WOS… `fs-hltb`, WS-073 — ADR 0067 reference prove-out (HTTP/storage) `fs-f8vm`, ADR 0067 implementation (WOS center: clocks runtime +… `fs-lq08`…

   Implement the declared clock kinds, including ResponseClock under D-8, plus pause/resume semantics. ADR 0067 D-2.1 deprecates SlaDefinition in favor of ProcessingSLA with kind discriminator — migration not yet done.

   **Acceptance:** AppealClock — adverse-decision / deterministic notice path (composes with Gov §4.1 #2); ProcessingSLA — intake accepted / intake-complete workflow event (with SlaDefinition→ProcessingSLA migration per D-2.1); GrantExpiry — benefit award issued transition; StatuteClock — WOS-owned triggers only (Formspec-originated use respondent-ledger path); Pause/resume per D-4 (clockResolved with resolution:paused + new clockStarted carrying residual duration, no separate ClockPaused record kind).

- **ADR 0067 §6 — Conformance + open-question encoding/deferral** · `fs-t9rl` · P2

   Zero conformance landed. Plus 3 open questions to encode or defer — filed as decision sub-tickets.

   **Acceptance:** wos-conformance fixtures (start, satisfied, elapsed, paused segment); kernel/companion prose for MUST-level behavior; resolve or formally defer the 3 ADR §Open questions (each a sibling decision ticket).

**P3 — sustaining:**

- **ADR 0067 §4 — #51 Statutory deadline chains (compose with business calendars + typed kernel events)** · `fs-lyt8` · P3

   Business calendar §7.1 infrastructure exists at specs/sidecars/business-calendar.md:185-196 (6-step normative algorithm). Composition with typed kernel events trigger-gated. Tracked broader at W_51 (fs-wsf0).

   **Acceptance:** Compose with §7.1 business calendars + typed kernel events; revisit trigger-gate once center contract ships (ADR 0067 accepted + D-1/D-2 shipped).

- **ADR 0067 §3 — Task SLA (#40 runtime implementation)** · `fs-pdwr` · P3
   blocked-by Task SLA runtime implementation `fs-96nd`

   Authoring surface landed at schemas/wos-workflow.schema.json:7762-7789 (slaDefinitions, SlaDefinition, warningThresholds, breachPolicy, escalationChain). Runtime implementation tracked at W_40 (fs-96nd); this row is the ADR 0067 cross-reference for clock contract overlap.

   **Acceptance:** Runtime SLA implementation lands (per W_40); cross-reference clock contract where Task SLA durations overlap rights-impacting deadlines (gated on ADR 0067 D-2.1 migration via ADR67_L2).

**Recently closed (kept for traceability; archive when stale):**

- ~~**ADR 0067 §1 — Kernel / provenance (clockStarted + clockResolved + Clock $def)**~~ `fs-nx1i` · CLOSED — clockStarted + clockResolved ProvenanceKind variants landed at kind.rs:361-377; payload $defs (Clock with…
- ~~**ADR 0067 §5 — wos-export (PROV-O / OCEL / XES for clockStarted + clockResolved)**~~ `fs-hfdc` · CLOSED — Zero landed for clock kinds across all three export paths.

<!-- tk:end -->

---

## Backlog

### Envelope-stack enablement (§4.7) **[Stream: C]**

<!-- tk:start epic=fs-cb7x -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-cb7x — edit tickets via tk, not this section -->

Stream: C. Cross-system envelope coordination + fixtures + separation-of-duties. Spec home: integration.md.

**P2 — standard:**

- **#59 — CloudEvent envelope-flow type catalog (integration.md)** `[6/3/4]=24` · `fs-nw43` · P2
   links Content-Attestation Envelope — spec authoring `fs-m8ic`

   Normative event-type catalog in integration.md for cross-system envelope coordination: envelopeCreated, signerInvited, signerAuthenticated, signerSigned, signerDeclined, envelopeCompleted, envelopeVoided, envelopeExpired, reminderDue. DISTINCT from #20 (kernel-internal event vocabulary per transition); #59 is cross-system wire contract that identity providers, email adapters, webhook consumers speak. Without it, every WOS-based signature stack defines own event names and integration ecosystem fragments.

   **Acceptance:** 9 event types normatively defined in integration.md; CloudEvents-compliant envelope shape; conformance vector per type.

**P3 — sustaining:**

- **#60 — Envelope reference fixtures (3-5 canonical kernel docs)** `[5/3/3]=15` · `fs-eerp` · P3
   links Content-Attestation Envelope — spec authoring `fs-m8ic`

   Three to five canonical kernel documents under fixtures/kernel/envelope-*.json demonstrating composition patterns: envelope-2signer-sequential.json, envelope-parallel-witness.json, envelope-decline-reroute.json, envelope-with-approver.json, envelope-reminder-expire.json. Plus matching conformance fixtures exercising full lifecycle. FIXTURE-ONLY work — no new schema surface. Critical for lock-in: locked patterns prevent divergent re-inventions across vendors building on WOS.

   **Acceptance:** 5 envelope fixtures + matching conformance fixtures land in fixtures/. Depends on #20 typed events and #30 task-lifecycle for the decline fixture.

- **#61 — Separation-of-duties conformance fixture batch** `[5/2/3]=15` · `fs-ekr5` · P3
   links Content-Attestation Envelope — spec authoring `fs-m8ic`

   Two to three fixtures under fixtures/conformance/ exercising AccessControl seam separation-of-duties rejection: (1) agent attempts to review own output → rejected; (2) delegated human attempts to re-review as original author → rejected; (3) separation-of-duties bypass with authority override → recorded as provenance with OverrideRecord. Pairs with #23 OverrideRecord schema landing. AccessControl seam shape already in wos-core traits; missing: conformance contract that reference processors MUST reject these attempts.

   **Acceptance:** 3 SoD conformance fixtures land; reference processor rejects (1) and (2), records (3) as OverrideRecord. GATE: #23 OverrideRecord schema landing.

**Recently closed (kept for traceability; archive when stale):**

- ~~**#58 — Envelope (instance-level) status extension**~~ `fs-rw2g` · CLOSED — Extend WorkflowProcess.status with first-class declined / voided / expired discriminators.

<!-- tk:end -->
### Release + benchmarking — ready, lower priority **[Stream: D]**

<!-- tk:start epic=fs-id17 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-id17 — edit tickets via tk, not this section -->

Stream: D. Release-train tooling + wos-bench synthesis benchmark. Lower priority.

- **§5.5 — wos-bench synthesis benchmark (Claim A falsification harness)** `[6/5/3]=18` · `fs-6qj8` · P3
   links ADR 0065: complete current-spec synthesis and independently… `fs-xkui`

   Pairs with ADR 0065 Q6 / synth split. Plan: thoughts/plans/2026-04-16-wos-synthesis-benchmark.md. Spike open questions: thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md#open-questions (Q-V0-1..4 need LIVE Anthropic runs). Sub-deliverables: scaffold crates/wos-bench (wos-synth-core + wos-synth-mock, optional Anthropic flag); problem statements + benchmarks/runs/<date>-<model>/results.json; rubric library + CLI; BENCHMARK.md leaderboard + methodology; scheduled/manual CI with secrets; inline ConformanceFixture wrapper vs wos_conformance::smoke_test_document-style API (spike Option B).

   **Acceptance:** crates/wos-bench scaffolded; CI runs benchmarks; BENCHMARK.md leaderboard; Q-V0-1..4 closed with live numbers.

- **§4.4 — Release trains Tasks 4-5 (Changesets + GitHub Actions)** `[5/4/3]=15` · `fs-qzir` · P3

   Plan: thoughts/plans/2026-04-16-wos-release-trains.md. Tasks 1-3 landed session 8. Remaining: Changesets tooling + GitHub Actions release workflow for the wos-* crate cluster.

   **Acceptance:** Changesets configured across wos-* crates; GitHub Actions release workflow tags + publishes coordinated versions.

<!-- tk:end -->
### ADR 0065 — authoring stack closure (MCP / synth / spike) **[Stream: D]**

**Anchors:** [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · MCP plan [2026-04-17](thoughts/plans/2026-04-17-wos-mcp-crate.md) · Synth plan [2026-04-16](thoughts/plans/2026-04-16-wos-synth-crate.md) · Spike retrospective [2026-04-20](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md). Plan markdown checkboxes in those files are **stale vs `main`** in places; this subsection is the working backlog until checkboxes are rebased.

<!-- tk:start epic=fs-d59u -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-d59u — edit tickets via tk, not this section -->

Close the gaps between authoring a WOS workflow and running the exact exported result. The supported process is: requirements and reviewed scenarios → native authoring → canonical schema/project checks → real WOS replay → explicit review/repair → verified export/reopen → actual host execution. Reuse ADR 0065's existing MCP/synthesis work and the current Policy Studio authoring model.

**P1 — high-leverage:**

- **WOS creation: qualify author-to-runtime journeys with real behavioral tests** · `fs-6r3f` · P1
   blocked-by WOS authoring: bind verification and export to the exact… `fs-rbn7`, ADR 0065: verify the full authoring flow through a real MCP… `fs-d79a`, Align counterfactual authoring checks with the selected… `fs-k6k9`, Align initial-case Mapping validation with protected intake… `fs-iif8`, Detect unreachable workflow states, nonfinal dead ends and… `fs-8371`, Complete the D1 appeal branch after a granted… `fs-xyv7`, Align notice evidence and deadline requirements across WOS… `fs-hltb` · links Automated D1 export end-to-end test on a real-size seeded… `fs-ipgs`, ADR 0080 — Governed Output-Commit Pipeline residue (~20%… `fs-t9h8`, Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m`, WOS appeals route and applicant appeal UI `fs-79hi`…

   Prove the complete creation process: requirements and reviewed scenarios, native authoring, validation, runtime replay, export/reopen, actual host admission and execution. Use the artifact produced by the supported tools with real Formspec evaluation and mapping; canned conformance responses cannot qualify this path.

   **Acceptance:**
     - 1.
     - A documented repeatable command takes a reviewed creation request/scenario set through supported MCP/CLI authoring operations and Policy Studio compile/replay to exact verified exports.
     - Reuse the shared native-check and deterministic trace-informed repair proof from fs-uqpw/fs-563r and native Studio replay proof from fs-8qbf; fs-d79a must establish the real-client path, and fs-rbn7 must bind exact tested/exported inputs.
     - Preserve tool transcripts, artifact identities and failure-stage results.
     - Live-provider generation guidance, quality and measurements remain unqualified under deferred fs-xkui and are not prerequisites for this supported author-to-host journey. 2.
     - Admit the exported candidate unchanged through maintained server configuration/publication APIs in an isolated test environment.
     - Exercise actual Formspec engine validation, actual Mapping, task creation/completion and selected runtime storage/authorization; no test-side reconstruction or manual policy patch. 3.
     - Independently reviewed scenarios cover approval, denial with required notice, missing information and correction, unauthorized action, deadline handling, and retry/restart.
     - Assert actual case state, tasks, notices and retained provenance/evidence for the supported path.
     - Timer tests may control the owning clock; durable restart/retry requires the real selected durable service. 4.
     - Include a real rendered respondent/staff interaction for the representative journey and a deliberately broken guard/mapping/expectation control proving the suite detects wrong behavior.
     - Test doubles are declared individually and cannot replace the Formspec/WOS/durable behavior being qualified. 5.
     - CI runs deterministic validation/replay and an explicitly selected service-backed integration lane; unavailable required services produce not-run/failure rather than green.
     - Retain exact revisions, native exits, transcripts/traces and artifact hashes with bounded diagnostics. 6.
     - The creation guide names supported operations, review points, commands, emitted artifacts and observed limitations.
     - Each claimed scenario has current evidence; historical D1 success, lint-only passes and local memory-backed adapters are not promoted to fresh end-to-end proof.

**P3 — sustaining:**

- **ADR-0065 — Synth provider + schema hygiene (#65k/l/m + Anthropic cache marker)** `[5/3/3]=15` · `fs-s5wm` · P3
   links WOS-SYNTH-ANTHROPIC-CACHE-001 — Wire CacheAnchor to… `fs-6n85`

   #65k: AnthropicPrompter folds CacheAnchor data into system prompt verbatim until Anthropic SDK exposes cache control; wire real cache blocks when available (crates/wos-synth-anthropic/src/lib.rs). PAIRS with monorepo audit row WOS-SYNTH-ANTHROPIC-CACHE-001 — this entry is the delivery item. #65l: schemas/synth/wos-synth-trace.schema.json exists; add/verify schemars round-trip validation test per synth plan Task 7. #65m: Do NOT extend ToolContext with speculative methods until second impl (W_65_SEAM) proves the shape.

   **Acceptance:** CacheAnchor travels through Anthropic cache-control headers (not inline); SynthTrace schema drift test passes round-trip; ToolContext discipline preserved (no speculative methods).

- **ADR 0065: complete current-spec synthesis and independently tested repair** · `fs-xkui` · P3
   blocked-by ADR 0065: share MCP tool execution with synthesis and the… `fs-uqpw`, ADR 0064: carry actual conformance traces into synthesis… `fs-563r` · links §5.5 — wos-bench synthesis benchmark (Claim A falsification… `fs-6qj8`

   Finish the synthesis quality work using current spec/schema guidance, required reviewed scenarios and actual behavioral feedback. Structured lint fields already reach repair prompts; do not rebuild that completed sub-slice. fs-563r owns adding the existing runtime trace to repair; this ticket owns generation policy, success criteria and measured quality.

   **Acceptance:**
     - 1.
     - Every advertised synthesis layer receives the applicable current canonical schema/spec guidance and declared capabilities.
     - Unsupported layers refuse explicitly rather than running with empty guidance.
     - Generation and repair share one guidance selector and these inputs; adding a supported layer must not require synchronized selection edits in both prompt builders. 2.
     - Required scenarios execute for each candidate through fs-uqpw.
     - A lint-clean but behaviorally wrong candidate cannot succeed; unavailable/empty required conformance cannot become a pass. 3.
     - A controlled failing case reaches repair with structured lint and observed runtime differences, then re-executes a changed candidate successfully.
     - Preserve candidates, prompt inputs, traces, iteration counts and failed/exhausted outcomes.
     - Do not change reviewed expectations to make a candidate pass. 4.
     - Malformed generated JSON reaches a repair attempt through a maintained typed discriminant or stable error code.
     - Current production DirectToolContext stringifies the error and exits; SynthError::UnparseableOutput is not constructed.
     - The retired spike already classifies LintError variants, so reuse its useful assertions without extending that implementation or describing production substring matching as an existing defect.
     - Use the supported upstream replay/document API (the historical smoke_test_document name was a proposal, not an assumed existing API). 5.
     - Run the existing purchase-order spike/benchmark with the live configured provider and retain Q-V0-1..4 measurements: iterations, dominant first-pass diagnostics, schema/FEL/governance fixes and behavioral repair.
     - Coordinate with fs-6qj8; one measurement corpus satisfies both, without duplicating the broader benchmark framework.
     - No quality or cost result is asserted without measurement. 6.
     - Update the spike findings with bounded conclusions and the actual supported route.
     - Studio consumers expose semantic repairs for explicit reviewer choice as required by scenario-authoring.md.

**P4 — backlog / trigger-gated:**

- **ADR 0065: document the supported creation process and reconcile old plans** · `fs-k63f` · P4

   Reconcile historical authoring/MCP/synth plans with shipped source and document the supported end-to-end creation process. This remains independently actionable: current gaps can be documented before implementation completes.

   **Acceptance:**
     - 1.
     - Reconcile thoughts/plans/2026-04-17-wos-authoring-crate.md with actual raw.rs/command.rs/project.rs or retain a precise remaining-gap map.
     - Mark verified landed plan steps or freeze obsolete checkboxes with an explicit pointer to current tickets. 2.
     - The guide explains required inputs, review/scenario preparation, native edits, schema/project checks, real replay, repair/review, export/reopen and host admission.
     - Show supported commands/API entry points and resulting artifacts. 3.
     - Clearly distinguish draft, statically valid, behaviorally verified and host-qualified results.
     - Describe unsupported operations and not-run checks without implying completion. 4.
     - Link generated tool documentation and the maintained runnable example; no handwritten duplicate list of tool schemas/actions.
     - Keep prose aligned when the epic's final acceptance is executed. 5.
     - Existing pending implementation tickets remain open with their own evidence requirements.
     - This documentation ticket has no hard dependency on fs-uqpw; updating docs does not implement or qualify that seam.

**Recently closed (kept for traceability; archive when stale):**

- ~~**ADR 0065: verify the full authoring flow through a real MCP client**~~ `fs-d79a` · CLOSED — Exercise the shipped wos-mcp binary through an actual MCP host/client.
- ~~**Align initial-case Mapping validation with protected intake handoff context**~~ `fs-iif8` · CLOSED — Actual new shared project validation rejects the maintained D1 initial-case Mapping with five E1110 findings because it validates handoff.responseRef, response.data,…
- ~~**WOS authoring: bind verification and export to the exact tested artifacts**~~ `fs-rbn7` · CLOSED — A successful check is useful only for the workflow and dependencies that were actually tested.
- ~~**Policy Studio: replace placeholder scenario gates with native WOS replay**~~ `fs-8qbf` · CLOSED — Policy Studio currently walks declared transitions for ActualTrace and its compiler conformance gate can pass with a stub marker.
- ~~**WOS authoring: replay exact candidates through the maintained runtime**~~ `fs-cilt` · CLOSED — Expose a supported reusable replay API over wos-conformance/WosRuntime.
- ~~**WOS authoring: share canonical schema and project validation across callers**~~ `fs-h9gq` · CLOSED — MCP currently runs single-document lint, while Policy Studio has its own schema gate that removes unsupported regex patterns.
- ~~**Reconcile the AI behavioral spec, native Workflow schema and runtime**~~ `fs-k07m` · CLOSED — Actual fs-d59u authoring/replay integration exposes a disagreement among normative prose, schema and runtime.
- ~~**Replace event-hint provenance with evidence from completed native operations**~~ `fs-pzju` · CLOSED — During fs-k07m review, the shared reference companion was found to call wos-core/src/event_handler.rs branches that manufacture non-AI success records from ordinary event hints:…
- ~~**WOS authoring: current-spec edits through one declarative MCP tool catalog**~~ `fs-rxgc` · CLOSED — WOS authoring still writes governance and AI content into x-wos-governance/x-wos-ai while the current Workflow schema defines native fields.
- ~~**ADR 0065: share MCP tool execution with synthesis and the CLI**~~ `fs-uqpw` · CLOSED — Complete the existing production ToolContext seam using the same catalog, handlers, canonical validation and replay used by MCP.

<!-- tk:end -->
### Behavioral / governance (1.0 scope under minutes-not-days)

Per repo-root [`VISION.md`](../VISION.md) operating frame: no "defer to 1.1" bucket on greenfield. These all land at 1.0 unless explicit architectural prerequisite unresolved. **[Stream assignments: B/C — see subsections below.]**

**Stack contracts (ADRs 0066, 0067):** **[Stream: B/C]**

<!-- tk:start epic=fs-wq1f -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-wq1f — edit tickets via tk, not this section -->

Stream: B/C. 1.0 scope under minutes-not-days. Per repo-root VISION.md operating frame: no 'defer to 1.1' bucket on greenfield.

- **Identity attestation shape — generalize beyond signatures** `[5/3/4]=20` · `fs-bmvg` · P3
   links Stack ADR (TBD) — identity attestation (PLN-0381) `fs-cfi1`, Trellis #3 — Identity attestation bundle shape (use… `fs-cz3j`

   WOS-T4 runtime emission now has SignatureAffirmation.identityBinding as first concrete shape. This item GENERALIZES that shape for reuse across non-signature evidence (reviewer-policy assurance refs, amendment-authority attestations, review-gate credentials). Coordinates with parent PLN-0381, PLN-0380, PLN-0384.

   **Acceptance:**
     - Generalized IdentityAttestation shape lands in wos-core; reused by reviewer-policy assurance refs, amendment-authority attestations, review-gate credentials.
     - GATE: T4 runtime emission landed (✓); parent stack ADR ratification (SAI_IDATT = fs-cfi1).

<!-- tk:end -->

**Maximalist cluster follow-ups (post-Session 14–16):** **[Stream: B]**

The 2026-04-28 cluster ratification landed 14 new `ProvenanceKind` variants + closed enums + DNS-tenant cap + five-mode amendment taxonomy + `InstanceStatus::Stalled` declaratively at HEAD. The items below close the **declarable-but-not-fired** gap at the runtime/adapter boundary — same shape as #2 (capability-precondition emission) and #67 ConfigurationWarning. Without them, schemas + lint ratchet ahead of the runtime and the next conformance-suite expansion will surface a wave of "declared-but-never-emitted" gaps.

<!-- tk:start epic=fs-7a5t -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-7a5t — edit tickets via tk, not this section -->

- **#70 — DurableRuntime::AppendFailure typed enum** `[6/4/5]=30` · `fs-vmsx` · P2
   links ADR 0070 — cross-layer failure and compensation `fs-qzg6`, Trellis #6 — ADR 0070 execution: CommitAttemptFailure… `fs-uj32`, Accepted signed TaskSubmit stalls in native preparation `fs-naer`

   Replace Result<_, RuntimeError> failure surface in DurableRuntime adapter contract with closed AppendFailure { Retryable, BudgetExhausted, Terminal } enum carrying typed reason codes. Today every adapter (in-memory + Restate + future) uses RuntimeError (not String) but classification of commit-attempt outcomes still string-matches into branching logic. WHY: ADR 0070 D-4.3 pins commit-failure taxonomy as substrate-classified, retry-budget-aware, with Stalled as terminal lifecycle state. NOT STARTED: no AppendFailure enum exists; CommitFailureKind enum exists in provenance layer only (record.rs:96). Composes with #72. ADR 0070 Accepted 2026-05-06.

   **Acceptance:** AppendFailure enum lands in wos-runtime DurableRuntime trait; adapters classify retryable vs budget-exhausted vs terminal; CommitAttemptFailure conformance fixture proves the three classes.

- **#71 — ReinstatementPolicy schema $def + lint K-A-010** `[6/3/4]=24` · `fs-g5nu` · P2
   links ADR 0066 — amendment and supersession (cross-stack rollup) `fs-lwsj`

   Add ReinstatementPolicy $def to wos-workflow.schema.json Workflow Governance embedded block (parallel to amendmentPolicy / rescissionPolicy); register lint K-A-010 enforcing closed five-mode amendment taxonomy. NOT STARTED — Reinstated provenance kind exists (kind.rs:350) but no governance policy shape or lint. ADR 0066 Accepted 2026-05-06.

   **Acceptance:** ReinstatementPolicy $def in wos-workflow.schema.json; K-A-010 lint enforces five-mode amendment taxonomy; conformance fixture proves reinstatement policy gates Reinstated emission.

- **#72 — Reference-runtime emission wiring for cluster variants** `[6/6/4]=24` · `fs-zjpl` · P2
   blocked-by DurableRuntime::AppendFailure typed enum `fs-vmsx` · links ADR 0070 — cross-layer failure and compensation `fs-qzg6`

   Wire the 14 cluster ProvenanceKind variants into runtime emission sites. Constructors exist; schema guards exist; audit-tier dispatch exhaustive. ZERO runtime emission sites — blocked by #70. ADR 0070 Accepted 2026-05-06; #70 not started.

   **Acceptance:** All 14 cluster ProvenanceKind variants fire from runtime emission sites; conformance fixture per variant. GATE: W_70 (fs-vmsx once filed) must land first.

**Recently closed (kept for traceability; archive when stale):**

- ~~**#73 — ConfigurationWarning runtime emission**~~ `fs-eqb7` · CLOSED — Wired 3 of 4 spec MUST sites in companion.rs: (a) drift-monitor.policyRef — ConfigurationWarning emitted when policyRef unresolvable; (b) notification-template.key — emitted on…
- ~~**#74 — ProvenanceKind enum ↔ schema recordKind parity**~~ `fs-wvt7` · CLOSED — Created canonical registry at schemas/record-kind-registry.json (131 entries, 1:1 with Rust enum).

<!-- tk:end -->

**Prior behavioral items:** **[Stream: C]**

<!-- tk:start epic=fs-79nz -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-79nz — edit tickets via tk, not this section -->

Stream: C. Per-feature 1.0 work surface: equity, access control, joint design, lint, SLA, claim-check, lifecycle, regions, milestones, migration, DCR, SMT, canary, counterfactual, multi-instance.

**P2 — standard:**

- **#78 — Counterfactual tier emission** `[6/4/5]=30` · `fs-jg0f` · P2
   links Admit and run selected workflow governance through… `fs-ps24`

   Add ProvenanceAuditTier::Counterfactual variant + dedicated ProvenanceKind variant(s). Wire Layer 1 injection path so runtime can stamp audit_layer='counterfactual'. CURRENT: AuditLayer::Counterfactual exists for parsing but no runtime emission. Maps to Feature Matrix 8.3. RELATED to #24b but distinct (reasoning trace vs counterfactual tier).

   **Acceptance:** ProvenanceAuditTier::Counterfactual variant lands; runtime emits with audit_layer='counterfactual'; conformance fixture; Feature Matrix 8.3 row green.

- **#32 — Multi-Instance Iteration (promoted from Deferred 2026-05-07)** `[6/7/5]=30` · `fs-zawy` · P2

   Score [6/7/5]=30 (was Deferred 6/7/5; trigger met — promoted to active Backlog No-gate table 2026-05-07). Spec + runtime work for multi-instance iteration semantics.

   **Acceptance:** Multi-instance iteration spec lands; runtime implements iteration semantics; conformance fixtures per iteration pattern.

- **#35 — Equity Config enforcement semantics (processor obligations)** `[7/5/4]=28` · `fs-3bs4` · P2
   blocked-by Equity RemediationTrigger expression language (FEL… `fs-ww51`

   Processor obligations for RemediationTrigger.action; wire DisparityMethod to runtime. PREREQUISITE: #36 resolved (stack vision: FEL + restricted-domain profile).

   **Acceptance:** Processor enforces RemediationTrigger.action per spec; DisparityMethod wired to runtime; conformance fixture per trigger action.

- **#24b + #25 — Reasoning tier rule-firing trace + Catala-style defeasibility** `[7/6/4]` · `fs-paz1` · P2
   links Admit and run selected workflow governance through… `fs-ps24`

   Score: #24b [7/6/4]=28; #25 [6/7/6]=36 (cluster total ~64). Vision model: workflow-governance with (sourceAuthority, priority) lexicographic. After ADR.

   **Acceptance:** Reasoning trace shape + Catala-style defeasibility lands per ADR; conformance fixture proves rule-firing ordering. GATE: ADR (to be authored).

- **#75 — DCR runtime evaluator** `[6/5/4]=24` · `fs-1b2p` · P2

   Runtime evaluation of DCR constraint zones (Advanced §4). Schema + provenance record kinds + instance-schema DCR state exist; actual condition/response/include/exclude/milestone relation evaluation algorithm needs Rust runtime path. Maps to Feature Matrix 1.6.

   **Acceptance:** DCR runtime evaluator lands in wos-runtime; conformance fixture per relation kind; Feature Matrix 1.6 row green.

- **#26a — AccessControl.canRead enforcement semantics** `[6/3/4]=24` · `fs-l2w3` · P2

   Normative processor behavior on canRead → false: redact / null / raise / skip. Prerequisite to #26b.

   **Acceptance:** Spec defines canRead→false processor behavior (closed enum); conformance fixture per mode.

- **#26b — caseFieldPolicy schema (per-field read/write scopes)** `[6/6/4]=24` · `fs-mci3` · P2
   blocked-by AccessControl.canRead enforcement semantics `fs-l2w3`

   Per-field read/write scopes by actor role.

   **Acceptance:** caseFieldPolicy $def lands in schema; per-field scoping enforced; conformance fixture proves cross-role read/write rejection. GATE: #26a (fs-l2w3) must land first.

- **#36 — Equity RemediationTrigger expression language (FEL restricted-domain profile)** `[6/4/4]=24` · `fs-ww51` · P2

   FEL + restricted-domain profile per VISION §X (no windowing escape hatch). Implementation. Gates #35.

   **Acceptance:** FEL restricted-domain profile spec'd; fel-core implementation; #35 unblocked. CROSS-REPO: fel-core work.

**P3 — sustaining:**

- **#28 — Claim-check artifact references** `[4/4/5]=20` · `fs-fpjz` · P3

   Typed ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }.

   **Acceptance:** ExternalArtifactRef $def lands; lint enforces content-hash binding; conformance fixture proves hash-verified reference.

- **#38 — G-064 Assertion Library resolution lint** `[5/3/3]=15` · `fs-4cse` · P3
   links WS-033 — Pipeline validation endpoint (Gov §5.4) `fs-zhuj`

   Implementation of the lint designed in session 8. Gates wos-server WS-033 (Pipeline validation endpoint).

   **Acceptance:** G-064 lint rule lands; references resolve at same site pipelines read them; conformance fixture proves unresolved-reference rejection.

- **#76 — SMT verification integration (Advanced §8)** `[5/6/3]=15` · `fs-mpx5` · P3

   Advanced §8 specifies verifiable constraint subset (decidable fragment, verification interface). At minimum: conformance fixture proving interface claim; external SMT solver integration. Maps to Feature Matrix 5.4.

   **Acceptance:** Conformance fixture proves Advanced §8 interface claim; external SMT solver integration; Feature Matrix 5.4 row green.

- **#30 — WS-HumanTask lifecycle completion (Suspended / Cancelled / Return)** `[5/5/2]=10` · `fs-2iy1` · P3

   Task-level Suspended, distinct Cancelled, explicit Return with rework counter. §4.7: task-level decline / return is half of signer-decline semantics; pairs with #58 envelope-status for instance-level decline / void / expire (#58 landed 2026-05-07).

   **Acceptance:** Task lifecycle extends with Suspended / Cancelled / Return states; rework counter increments on Return; conformance per state.

- **#40 — Task SLA runtime implementation** · `fs-96nd` · P3
   links ADR 0067 — Runtime clock emission, ResponseClock and… `fs-7yfo`

   Score per §10.3. Beyond session-8 authoring surface; wire §10.3 runtime obligations. §4.7: spec home for envelope reminders + expirations once runtime fires slaDefinitions / warningThresholds / breachPolicy. Gated on ADR 0067 D-2.1 migration (SlaDefinition→ProcessingSLA).

   **Acceptance:**
     - Runtime fires slaDefinitions; warningThresholds emit warnings; breachPolicy enforces escalationChain; conformance fixture covers SLA lifecycle.
     - GATE: ADR 0067 D-2.1 migration (W_ADR0067).

- **Bulk Operations spec (admin-portal-driven)** · `fs-vt0y` · P3
   links WS-098 — DocuSign 100% admin surface (HTTP scaffold)… `fs-88yq`

   Admin-portal-driven; parallel case instantiation + bulk state transitions. Relocated from Future specs.

   **Acceptance:** Bulk Operations spec authored; schema + conformance fixtures land. Composes with wos-server WS-098 (DocuSign admin surface trigger).

**P4 — backlog / trigger-gated:**

- **#27 — Cancellation regions (YAWL-style)** `[4/6/3]=12` · `fs-2los` · P4

   YAWL-style named regions distinct from cancellationPolicy join policy.

   **Acceptance:** Named cancellation regions $def lands; runtime cancels region as unit; conformance fixture proves region-scoped cancellation.

- **#77 — Canary deployment schema formalization** `[4/3/3]=12` · `fs-8rpq` · P4
   links Specify and implement reserved native AI oversight settings `fs-yhw8`, WS-044 — Write-side drift detection (Drift §1.3)… `fs-2dmc`, Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m`

   deploymentSequence property in drift-monitor spec prose needs schema $def in Advanced block. Shadow mode already has schema; canary phase (canaryPercentage, canaryDuration) is prose-only. Maps to Feature Matrix 5.22.

   **Acceptance:** deploymentSequence + canary $defs land in Advanced block; conformance fixture; Feature Matrix 5.22 row green.

- **#29b — Milestone reactive transition firing (GSM-style)** `[6/5/2]=12` · `fs-s6oa` · P4

   Ships after #29a (landed session 4). GSM-style reactive milestone firing.

   **Acceptance:** Milestones fire reactively on condition match; conformance fixture proves multi-milestone choreography.

- **#3 (migration) — Policy-based migration routing** `[5/6/2]=10` · `fs-imxz` · P4
   links ADR 0071 — cross-layer migration and versioning `fs-i0lz`, Trellis #7 — ADR 0071 execution: CaseOpenPin and migration… `fs-n9qv`

   migrationPolicy: grandfather | migrateAll | migrateByState | expression. Tenant-scope sub-question finalizes with DurableRuntime tenant contract. §4.7: tenant-scope sub-question blocks multi-tenant envelope deployments (Open Q7 refers).

   **Acceptance:** migrationPolicy closed enum lands; tenant-scope contract resolves; conformance per policy mode. Cross-stack: SAI_71 (fs-i0lz) — ADR 0071 cross-layer migration.

**Recently closed (kept for traceability; archive when stale):**

- ~~**#43 — Assurance × impact-level composition**~~ `fs-dx9v` · CLOSED — Minimum Assurance floor per impact level (rights-impacting ≥ high; safety-impacting ≥ high; operational ≥ standard) per stack vision.

<!-- tk:end -->
### Untracked debt (monorepo audit 2026-05-08)

<!-- tk:start epic=fs-dyhj -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-dyhj — edit tickets via tk, not this section -->


**Recently closed (kept for traceability; archive when stale):**

- ~~**WOS-SYNTH-ANTHROPIC-CACHE-001 — Wire CacheAnchor to Anthropic cache control API**~~ `fs-6n85` · CLOSED — crates/wos-synth-anthropic/src/lib.rs:9-10 folds CacheAnchor data into system prompt verbatim because SDK doesn't expose cache control.
- ~~**WOS-MCP-TYPED-ACCESSOR-001 — Read current native agents through typed authoring accessors**~~ `fs-07x9` · CLOSED — Replace wos-mcp describe's raw extensions['x-wos-ai']['agents'] lookup with the typed current Workflow agents field.

<!-- tk:end -->
### Hygiene / refactors **[Stream: C]**

Sequenced for module-bottleneck relief, not delayed by it.

<!-- tk:start epic=fs-ig53 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-ig53 — edit tickets via tk, not this section -->

- **#22 — Crate split along tier boundaries (wos-core, wos-runtime)** `[5/3/3]=15` · `fs-0voe` · P3

   wos-core → wos-{kernel,governance,ai,advanced}; wos-runtime/runtime.rs (still large single module ≈3.7k lines) split along action-kind dispatch; CI fence. #22a (provenance module split + ProvenanceAuditTier) landed 2026-04-21.

   **Acceptance:** wos-core split into 4 tier-aligned crates; wos-runtime/runtime.rs split along action-kind dispatch; CI fence enforces dep direction.

<!-- tk:end -->
### Runtime Companion parity **[Stream: B]**

<!-- tk:start epic=fs-yzcd -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-yzcd — edit tickets via tk, not this section -->

Stream: B. wos-runtime parity vs published §15 MUSTs; PARITY drift; HTTP §15 fixtures. Decomposed into #66a-#66g sub-items. Full context: workspec-server/crates/wos-server/TODO.md (WS-011, WS-074, WS-075).

- **#66 — Runtime §15 processor parity (umbrella)** `[7/5/5]=35` · `fs-9pjk` · P1
   links WS-075 — Cross-stack §15 conformance fixtures (HTTP) `fs-fd0q`, Admit and run selected workflow governance through… `fs-ps24`

   Umbrella for Runtime Companion §15 / Phase 11 parity work. Decomposed into #66a-#66g children. Full context at workspec-server/crates/wos-server/TODO.md (WS-011, WS-074, WS-075). Cross-stack: wos-server WS-072 (ADR 0066 prove-out), WS-073 (ADR 0067 prove-out), WS-075 (§15 conformance fixtures HTTP).

   **Acceptance:** All 7 #66a-#66g children closed; wos-runtime parity vs published MUSTs verified; conformance fixtures green.

<!-- tk:end -->
### Verifiability **[Stream: C]**

<!-- tk:start epic=fs-r06l -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-r06l — edit tickets via tk, not this section -->

Stream: C. Determination snapshots, LoadBearing promotion, simulation trace.

**P2 — standard:**

- **K-DET-001 — Determination-snapshot conformance + fixture migration** `[6/3/5]=30` · `fs-91cr` · P2
   links SNAP-SPINE-001 — UWU-5 extracted: revocation-operations… `fs-q1ut`, WOS-SIGNATURE-ADMISSION-REVOCATION-001 — revocation_policy… `fs-yciq`

   Conformance gate for Facts-tier snapshots on determination transitions.

   **Acceptance:** K-DET-001 conformance gate active; existing fixtures migrated to new shape; CI ratchet.

- **Seeded LoadBearing-promotion batch + rule-coverage CI** `[6/4/4]=24` · `fs-6w7l` · P2

   1 LoadBearing rule today; land promotion set + CI gate together.

   **Acceptance:** LoadBearing-promotion batch lands; rule-coverage CI gate active; LINT-MATRIX.md updated.

**P3 — sustaining:**

- **WOS-B2 — Kernel-Basic profile LoadBearing declaration + lint-matrix wire** · `fs-n1kj` · P3

   Backlog. Kernel-Basic profile LoadBearing declaration + lint-matrix wire. No prerequisites.

   **Acceptance:** Kernel-Basic profile declared LoadBearing; LINT-MATRIX.md updated; rule-coverage CI gate active.

**P4 — backlog / trigger-gated:**

- **#52 — Simulation trace format** `[4/3/2]=8` · `fs-nn0n` · P4

   Simulation trace format spec.

   **Acceptance:** Simulation trace format spec'd; reference implementation in wos-runtime; one fixture.

<!-- tk:end -->
### ADR 0064 residual **[Stream: D]** {#adr-0064-residual}

<!-- tk:start epic=fs-y32q -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-y32q — edit tickets via tk, not this section -->

Stream: D. ADR 0064 agent-adapter implementations + structured lint diagnostics + trace-emitting conformance + companion drift lint. Six adapter crates declared, one shipped (wos-agent-stub); five are skeletons with unimplemented!() invoke bodies trigger-gated on orchestrator seam (Do-next #2).

**P1 — high-leverage:**

- **#A1 — wos-agent-anthropic invoke body** `[7/4/5]=35` · `fs-ixl3` · P1
   blocked-by AI-runtime capability-precondition emission wiring `fs-w9dv`

   Wire Anthropic SDK (anthropic-sdk 0.1.x currently used by wos-synth-anthropic) into AnthropicInvoker::invoke() body. Reuse streaming-collection pattern from wos-synth-anthropic. Needs tokio + reqwest deps. GATE: Do-next #2 (W_AI_PRECOND = fs-w9dv) AgentRuntime trait / DurableRuntime method landed.

   **Acceptance:** AnthropicInvoker::invoke() body wired; tests prove Anthropic streaming integration; conformance fixture per agent type.

**P2 — standard:**

- **#A4 — wos-agent-a2a invoke body** `[6/7/5]=30` · `fs-njef` · P2
   blocked-by AI-runtime capability-precondition emission wiring `fs-w9dv`

   A2A multi-agent orchestrator client; delegate to sub-agent per AgentSpec.a2a config. Needs A2A protocol client library. GATE: Do-next #2 + A2A SDK maturity.

   **Acceptance:** A2A agent adapter invoke body wired; multi-agent orchestrator delegation works; conformance fixture.

- **#A3 — wos-agent-mcp invoke body** `[6/6/5]=30` · `fs-pizw` · P2
   blocked-by AI-runtime capability-precondition emission wiring `fs-w9dv`, ADR 0065: verify the full authoring flow through a real MCP… `fs-d79a`

   MCP client; connect to MCP server per AgentSpec.mcp config; marshal AgentContext into tool call; collect tool result. Needs rust-mcp-sdk (same transport decision as #65e at W_65_MCP = fs-d79a). GATE: Do-next #2.

   **Acceptance:** MCP agent adapter invoke body wired; tests prove tool-call marshalling; conformance fixture.

- **#A2 — wos-agent-http invoke body** `[6/5/5]=30` · `fs-wzps` · P2
   blocked-by AI-runtime capability-precondition emission wiring `fs-w9dv`

   Generic HTTP/OpenAPI caller; parse AgentSpec.http config for endpoint + method + headers; POST AgentContext JSON; parse AgentResponse from response body. Needs reqwest + tokio deps. GATE: Do-next #2 (W_AI_PRECOND).

   **Acceptance:** HTTP agent adapter invoke body wired; tests prove POST + parse; conformance fixture.

- **#A5 — wos-agent-claude-sdk invoke body** `[5/5/5]=25` · `fs-mhhv` · P2
   blocked-by AI-runtime capability-precondition emission wiring `fs-w9dv`

   Claude Agent SDK client; delegate to Claude agent per AgentSpec.claudeAgentSdk config. GATE: Do-next #2 + SDK maturity.

   **Acceptance:** Claude Agent SDK adapter invoke body wired; conformance fixture.

- **Specify and implement reserved native AI oversight settings** · `fs-yhw8` · P2
   blocked-by Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m` · links WS-044 — Write-side drift detection (Drift §1.3)… `fs-2dmc`, Reconcile the AI behavioral spec, native Workflow schema… `fs-k07m`, Canary deployment schema formalization `fs-8rpq`

   The fs-k07m expert review found behavioral obligations without complete native authored shapes for aiOversight.oversightExtensions, reviewSampling, narrativeTier, driftDetection and assistGovernanceProxy, plus training-data and optimization disclosures. The reviewed spec reserves these configured fields and requires native/schema refusal; older runtime structs and open schema objects are not normative authority. Complete only explicitly chosen behaviors through their existing owners before advertising supported configuration.

   **Acceptance:**
     - 1.
     - Each enabled setting has explicit normative inputs, processing, outputs and checks, with expert approval before code/schema. 2.
     - One canonical schema/model and maintained owner route interpret it consistently; remaining undefined settings still refuse. 3.
     - Tests observe actual independent-first visibility/review, sampling selection, non-authoritative narrative handling, drift/disclosure or proxy behavior for every feature claimed, including causal negative controls.
     - Emitted provenance alone does not prove an operation ran. 4.
     - Runtime/API/MCP/Studio capability reporting names supported, unsupported and not-run stages; no AI Complete claim until all required operations are proved. 5.
     - Existing related owners are linked rather than duplicated, and exact evidence/limitations are retained.

**P4 — backlog / trigger-gated:**

- **ADR 0064 residual — COMP-001 companion drift lint** `[4/2/4]=8` · `fs-pqwm` · P4

   Companion drift lint — trigger-gated.

   **Acceptance:** COMP-001 lint detects companion drift; trigger: companion spec changes break consumers in field.

**Recently closed (kept for traceability; archive when stale):**

- ~~**ADR 0064: carry actual conformance traces into synthesis repair**~~ `fs-563r` · CLOSED — Structured passing/failing ConformanceTrace and deltas already exist.
- ~~**ADR 0064: align the existing lint diagnostic schema with emitted results**~~ `fs-p2pd` · CLOSED — The structured LintDiagnostic/output and synthesis mapping already exist.

<!-- tk:end -->
### Regulatory (1.0) **[Stream: C]**

<!-- tk:start epic=fs-cmzh -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-cmzh — edit tickets via tk, not this section -->

Stream: C. EU AI Act + OMB M-24-10 + repositioning docs.

**P2 — standard:**

- **#50 — EU AI Act alignment (Art. 13-14)** `[7/5/4]=28` · `fs-tjeb` · P2

   Art. 13-14 alignment spec.

   **Acceptance:** EU AI Act Art. 13-14 alignment spec'd; aiOversight embedded block carries disclosure metadata; conformance fixture proves Art. 13 disclosure emission.

**P3 — sustaining:**

- **#53 — Federal AI process-documentation policy mapping for the selected deployment** · `fs-jfdh` · P3

   Current authority correction (2026-10-18): OMB M-25-21, dated April 3, 2025, explicitly rescinds and replaces M-24-10 on page 1. [Official OMB memorandum](https://www.whitehouse.gov/wp-content/uploads/2025/02/M-25-21-Accelerating-Federal-Use-of-AI-through-Innovation-Governance-and-Public-Trust.pdf). This establishes the replacement fact; it does not establish that M-25-21 is the sole or exhaustive current authority for every deployment.

   **Acceptance:**
     - 1.
     - Identify the applicable current federal AI policy and deployment obligations before selecting the requirements mapping; retain the original M-24-10 references as historical context. 2.
     - Map applicable process-documentation requirements to the maintained assurance and impact-level metadata, recording both supported requirements and any gaps. 3.
     - Add conformance evidence for those concrete metadata and emission requirements.
     - This is technical evidence for the selected deployment, not blanket software compliance certification.

**P4 — backlog / trigger-gated:**

- **§5.6 — Repositioning/demo artifacts (README + demo narrative)** `[4/2/2]=8` · `fs-k13g` · P4

   Verify README.md leads with two-claim framing (not 'AI-native' tagline) per handoff §5.6; author demo narrative (requirement → workflow trace) once wos-synth is stable. Partially satisfied by POSITIONING.md; gap closure never explicitly verified.

   **Acceptance:** README two-claim framing verified; demo narrative authored once wos-synth stable.

<!-- tk:end -->
---

## Trigger-gated

Work that does not start until an explicit external trigger fires (commercial-request, ecosystem demand, production-deployment signal, sibling-repo ratification, vendor SDK maturity). Items here are real and tracked, just not actionable today.

### FEL restricted-domain profile (#35/#36)

Cross-repo dependency on fel-core. Not blocking WOS Stream B/C/D/E/F. Sequence when fel-core restricted-domain profile ships. See [`VISION.md`](../VISION.md) §X for FEL as the only expression language; [PLANNING.md](../PLANNING.md) for fel-core coordination.

### Engine adapters (commercial-request-gated)

WOS's first production runtime target is Restate (selected by WOS-T3). Additional adapters are gated on commercial adopter request or SDK maturity.

<!-- tk:start epic=fs-x2m9 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-x2m9 — edit tickets via tk, not this section -->

- **#49a — Camunda 8 Worker engine adapter [TRIGGER-GATED]** `[5/8/3]=15` · `fs-5gg7` · P4

   BPMN target; broadest external fixture diversity. WOS's first production runtime target is Restate adapter (selected by WOS-T3). Additional adapters trigger-gated on commercial adopter request or SDK maturity.

   **Acceptance:** Camunda 8 Worker adapter implemented; conformance fixtures pass. TRIGGER: commercial adopter request.

- **#49c — AWS Step Functions engine adapter [TRIGGER-GATED]** `[5/8/3]=15` · `fs-86vh` · P4

   Broadest commercial reach; narrowest semantic fit. (#49b Temporal evaluated by WOS-T3 and deferred until Rust workflow API stabilizes.)

   **Acceptance:** Step Functions adapter implemented; conformance fixtures pass (within semantic fit limits). TRIGGER: commercial adopter request.

<!-- tk:end -->

### Interoperability + speculative

<!-- tk:start epic=fs-t0y5 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-t0y5 — edit tickets via tk, not this section -->

Trigger-gated. SCXML interop + statutory deadline chains. Land on ecosystem demand or production deployment trigger.

- **#51 — Statutory deadline chains [TRIGGER-GATED]** `[4/7/5]=20` · `fs-wsf0` · P4

   Must compose with #31 business calendars + typed kernel events (TransitionEvent, #20). Trigger: first production deployment exposes concrete need.

   **Acceptance:** Statutory deadline chain spec composes with §7.1 business calendars + typed kernel events; conformance fixture. TRIGGER: first production deployment exposes concrete need.

- **SCXML interoperability — bidirectional WOS ↔ SCXML mapping [TRIGGER-GATED]** `[3/6/2]=6` · `fs-srpg` · P4

   Bidirectional WOS ↔ SCXML mapping. Trigger: ecosystem demand.

   **Acceptance:** Bidirectional mapping spec'd; reference implementation; conformance fixture. TRIGGER: ecosystem demand surfaces.

<!-- tk:end -->

### Deferred (captured, awaiting trigger)

Captured but not active; re-score when the trigger fires.

<!-- tk:start epic=fs-8sv0 -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-8sv0 — edit tickets via tk, not this section -->

- **#9 — JSON-LD Projection/Import Surface [DEFERRED]** `[5/5/3]=15` · `fs-66du` · P4

   Trigger: ontology spec drafts begin OR shipped PROV-O pulls @context into authoring.

   **Acceptance:** JSON-LD projection/import surface lands when triggered.

- **#33 — Inclusive-OR / Event-Choice / Boundary Events [DEFERRED]** `[3/5/2]=6` · `fs-20sy` · P4

   Trigger: authoring frustration with workarounds (externally observable signal).

   **Acceptance:** Inclusive-OR / event-choice / boundary events land when trigger observed.

- **Ontology field identity — semantic-field-identity protocol [DEFERRED]** · `fs-2jgz` · P4

   ontology-spec.md does not exist. ADR 0076 (product-tier consolidation) settles the lane: semantic projection/import belongs to wos-ontology-alignment sidecar, not Kernel substrate. Move to active only once a draft exists.

   **Acceptance:** Prerequisite design lands: (1) semantic-field-identity protocol; (2) cross-document alignment; (3) executable projection/import conformance. Then promote to active.

- **FEEL-to-FEL migration guide [PARKED — trigger: first DMN shop asks]** · `fs-4ef3` · P4

   On-demand documentation. Write when the first DMN shop asks; not speculative work.

   **Acceptance:** Concrete external request from a DMN-shop user lands. Then author guide.

- **JSON Patch for fine-grained provenance [PARKED]** · `fs-v532` · P4

   Speculative — finer-grained provenance shape using JSON Patch operations. No active demand; recorded for transparency. Append-only provenance with full-record events remains current model.

   **Acceptance:** Concrete use case where current provenance granularity fails to capture a needed audit signal.

<!-- tk:end -->

### Future specs (not yet authored)

Federation Profile and Bulk Operations relocated — see "Moved to Trellis" and Backlog / behavioral items respectively.

<!-- tk:start epic=fs-4uys -->
<!-- GENERATED by scripts/generate-todo.mjs from tk epic fs-4uys — edit tickets via tk, not this section -->

- **Learning Profile — retraining governance [TRIGGER-GATED]** · `fs-q0e6` · P4

   Future spec. Retraining governance. Trigger: long-lived AI agents need retraining policy.

   **Acceptance:** Learning Profile spec authored when long-lived AI agents need retraining policy.

<!-- tk:end -->
## Moved to Trellis (scope-out)

Per [`VISION.md`](../VISION.md) §XI, Trellis is the integrity layer and owns these concerns. WOS emits records via `custodyHook`; Trellis anchors them. Tracked here only to close the loop on items that used to be listed as WOS work.

- **#48 Merkle provenance chains** — Trellis. Hash-chaining + SCITT alignment are Trellis primitives.
- **Federation Profile** (cooperative trust-anchor network) — Trellis. Previously tracked as WOS Future spec.
- **SCITT strictness** (full vs. adjacent) — Trellis decides.
- **Checkpoint seal protocol** — Trellis.
- **Proof-of-inclusion + transparency-log submission tooling** — Trellis.
- **Certificate-of-completion export bundle format** — Trellis export-bundle primitive.

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

## Open architectural questions

Active decisions are tracked as P0–P2 tickets in `tk` (`tag:decision`). Three WOS-scope stack-wide uncertainties pinned in [`VISION.md`](../VISION.md) §X *Active uncertainties (WOS-scope)*: **DocuSign parity scope**, **multi-tenant on Restate vs Temporal**, **rendering service for signature artifacts**. Per-ADR open questions live alongside their ADR's execution checklist epic (e.g. ADR 0067 §6 has 3 sub-tickets covering timestamp granularity, post-hoc `elapsed` semantics, and multi-jurisdictional emit shape).

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
