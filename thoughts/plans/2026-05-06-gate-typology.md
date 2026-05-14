# Gate typology & parallelization plan (archived 2026-05-15)

Originally lived in `work-spec/TODO.md` as the audited 2026-05-06 planning artifact for the 0066/0067/0070/0092 ADR ratification sweep. Archived because most rows are now ✅ done; the navigational distillation (5-stream table + sequencing bullets) lives inline in TODO.md. Restore this file if a similar gate-classification exercise is needed for a future ratification cluster.

---

Audited 2026-05-06. Every item classified by gate type; ADR status + cross-repo dependencies mapped; independent workstreams designed.

### ADR status summary

Four ADRs (all Accepted 2026-05-06) gate the heavyweight Do-next items. All four are **independent** of each other. ADRs 0066/0067/0070/0071 share the same cluster-ratification gate (ratified); 0092 is standalone (amends ADR 0082, also ratified).

| ADR | Status | Gates WOS TODO | Ratification |
|-----|--------|----------------|-------------|
| **0066** — Amendment & Supersession | **Accepted** 2026-05-06 | #3, #4, #71 | Cluster ratification sweep |
| **0067** — Statutory Clocks | **Accepted** 2026-05-06 | #5 | Cluster ratification sweep |
| **0070** — Failure & Compensation | **Accepted** 2026-05-06 | #70, #72 | Cluster ratification sweep |
| **0092** — TypeID-in-URN Identity | **Accepted** 2026-05-06 | #6 | Standalone; amends ADR 0082 |

**Ratified 2026-05-06.** All four ADRs accepted (cluster 0066–0071 + standalone 0092). Do-next #3–#6 + backlog #70–#72 now unblocked (8 items, 296 combined Imp×Debt).

### Items by gate type

**No gate — start immediately (15 items):**

| Item | Score | Work type | Stream |
|------|-------|-----------|--------|
| #7 Multi-step session DAG | 20 | Schema + spec + API endpoint | A | ✅ 2026-05-07 |
| #58 Envelope status extension | 35 | Schema + spec | C | ✅ 2026-05-07 |
| #26a AccessControl.canRead semantics | 24 | Spec + conformance | C |
| #43 Assurance × impact-level | 24 | Spec + conformance | C | ✅ 2026-05-07 |
| #50 EU AI Act alignment | 28 | Spec | C |
| #38 G-064 Assertion Library lint | 15 | Lint impl | C |
| #28 Claim-check artifact refs | 20 | Schema + spec | C |
| #30 WS-HumanTask lifecycle | 10 | Schema + spec | C |
| #27 Cancellation regions | 12 | Schema + spec | C |
| #29b Milestone reactive firing | 12 | Runtime | C |
| #53 OMB M-24-10 compliance | 18 | Spec | C |
| Bulk Operations spec | — | Spec | C |
| §5.6 Repositioning/demo artifacts | 8 | Docs | D |
| #66a–#66g Runtime Companion §15 | various | Runtime + conformance | B |
| #32 Multi-Instance Iteration | 30 | Spec + runtime (trigger #20 met; promote from Deferred) | C |

**ADR-gated — now unblocked (ratified 2026-05-06):**

| Item | Score | Gate | Stream |
|------|-------|------|--------|
| #3 AuthorizationAttestation actor shape | 35 | ADR 0066 Accepted | B |
| #4 Amendment/supersession/rescission/correction | 35 | ADR 0066 Accepted | B |
| #5 Statutory clocks implementation | 35 | ADR 0067 Accepted | B |
| #6 TypeID-in-URN identity | 35 | ADR 0092 Accepted | A | ✅ Landed 2026-05-07 (WS-1–8: schema, wos-core, server, specs, tests, case-portal verified) |
| #70 AppendFailure typed enum | 30 | ADR 0070 Accepted | B |
| #71 ReinstatementPolicy + K-A-010 | 24 | ADR 0066 Accepted | B |
| #72 Cluster variant emission wiring | 24 | ADR 0070 Accepted + #70 | B |
| Identity attestation generalize | 20 | PLN-0381 parent ADR pending | C |

**Code-gated — requires another TODO item first:**

| Item | Gate | Depends on | Stream |
|------|------|------------|--------|
| #2 Capability-precondition emission | ADR 0064 orchestrator missing | AgentRuntime trait / DurableRuntime method design | E |
| #35 Equity Config enforcement | #36 must resolve first | FEL restricted-domain profile | C |
| #36 Equity RemediationTrigger expr | FEL restricted-domain profile | fel-core work | C |
| #26b caseFieldPolicy schema | #26a must land first | #26a | C |
| #40 Task SLA runtime | ADR 0067 D-2.1 migration | ADR 0067 runtime emission | B |
| #59 CloudEvent envelope catalog | #20 + #30 | Typed events + task lifecycle | C |
| #60 Envelope reference fixtures | #20 + #30 | Typed events + task lifecycle | C |
| #61 SoD conformance fixtures | #23 OverrideRecord | OverrideRecord schema landing | C |
| #3b Policy-based migration routing | DurableRuntime tenant contract | Tenant-scope sub-question | C |

**Cross-repo-gated — dependent on sibling repo work:**

| WOS item | Cross-repo dependency | Owner repo | Status |
|----------|----------------------|------------|--------|
| #1 T4 COC rendering (T4-10) | Trellis HTML-to-PDF reference renderer | Trellis | Open — byte composition done (Wave 25); rendering not yet landed |
| #1 T4 Studio UI (T4-11) | formspec-studio 11 items | formspec-studio | Open |
| #1 T4 vendor x-* floor | PLN-0384 event taxonomy ratification | WOS/Stack | P0 Open |
| #1 T4 shared fixture bundle (T4-12) | PLN-0067/0068/0069 + Trellis items #1/#2 | Stack/Trellis | P1 Open |
| Identity attestation shape | PLN-0381 + Trellis item #3 | Stack/Trellis | P0 Open (gated on PLN-0384) |
| #66f amendment task linkage | WS-072 wos-server ADR 0066 prove-out | workspec-server | Gated on ADR 0066 |
| #66 clock parity | WS-073/074/075 wos-server | workspec-server | Gated on ADR 0067 |
| ADR 0066 runtime + conformance | WS-072 + Trellis COMPLETED Waves 40/47 + PLN-0050/51/55/56 | workspec-server/Trellis/Formspec | Gated on ADR 0066 acceptance |
| ADR 0067 runtime + export | WS-073 + Trellis COMPLETED Wave 39 + PLN-0157/59/60/61 | workspec-server/Trellis/Formspec | Gated on ADR 0067 acceptance |

**Owner-decision-gated — unblocks with verdict:**

| Item | Decision needed | Stream |
|------|----------------|--------|
| #7 scope (additionalProperties) | false + reconcile 4 missing fields per owner decision | A |

### Parallelization plan — 5 workstreams

When ADR ratification sweep completes, work distributes across 5 independent tracks:

| Stream | Items | Work type | Scope |
|--------|-------|-----------|-------|
| **A — Identity** | #6 (TypeID), #7 (session DAG) | Schema refactor + URN parsers + ~55 fixtures + 10 spec docs + API endpoint | WOS only, self-contained |
| **B — Provisioning** | #3, #4, #5, #70, #71, #72, #66a–#66g, #40 | Governance policies, runtime emission wiring, export, conformance, Runtime Companion parity | WOS center + wos-export + wos-conformance + wos-runtime |
| **C — Spec & Schema** | #58, #26a, #26b, #43, #50, #28, #30, #27, #29b, #53, Bulk Ops, #32, #35, #36, #38, #59, #60, #61, identity attestation, #24b/#25, #3 | Schema additions, spec prose, conformance fixtures, lint impl | WOS schemas + specs + lint |
| **D — Authoring & Tooling** | #65a–#65o (ADR 0065), §5.5 wos-bench, §4.4 release trains, §5.6, structural merges | MCP/synth/authoring cleanup, benchmarking, CI, docs | wos-{mcp,synth-core,authoring,bench} crates |
| **E — Cross-repo** | #1 T4 closeout, #2 AI orchestrator, identity attestation cross-repo | Trellis COC, Studio UI, AgentRuntime trait, cross-repo ADR coordination | Trellis + formspec-studio + WOS crates |

Streams A–D are WOS-internal and fully independent of each other. Stream E coordinates across repos but does not block A–D.

**Stream-internal sequencing:**
- **A:** #6 (TypeID URN rewrite) is prerequisite for all URN-using work → run first; #7 runs in parallel (touches different schema surface)
- **B:** Governance policies (#3/#4/#71) are independent of each other once ADRs accepted; #70 must land before #72; export work (#4/#5 export paths) independent of runtime emission; #66a–#66g are mostly independent sub-items
- **C:** #26a → #26b (sequential); #30 pairs with #58; #35 blocked on #36; #59/#60 blocked on #20/#30; rest are fully independent
- **D:** #65a → #65b → #65c (sequential ToolContext chain); #65g–#65j are independent of each other; #65n → #65o (sequential plan reconciliation)
- **E:** AI orchestrator (#2) semi-blocked until AgentRuntime seam design complete; T4 COC and Studio UI are independent of each other

**Highest-leverage first moves:**
1. ~~Ratify ADRs 0066/0067/0070/0092~~ **Done 2026-05-06** — 8 items unblocked
2. ~~Structural merges (§4.5)~~ **Done 2026-05-07** — assertion-library + due-process-config absorbed
3. Ratify PLN-0384 (unblocks vendor x-* floor + identity attestation + Trellis item #3)
4. Start Stream A #6 + Stream C ungated spec items in parallel
5. Start Stream B governance policies + export alongside Stream A
6. Spin up Stream D authoring cleanup + Stream E cross-repo coordination
