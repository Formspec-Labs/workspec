# Open Questions: DI Review + External-Owner Boundaries

**Date:** 2026-04-22
**Source:** DI review passes on `thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md` + search of all `*.md` files containing "temporal"
**Status:** Questions awaiting decision; none resolved yet.

**Blocking-relationship summary** (decide in this order):

1. **Q2** (timer spec conflict) — spec-side question with short path; gates Phase 7 start.
2. **Q1** (adapter posture) — gates phase-sequence communication to stakeholders.
3. **Q3** (chain posture) — gates A5 endpoint semantics.
4. **Q5** (B1 path) — gates Phase 2 start.
5. **Q8** (FieldError break) — gates Phase 0.5 trait-shape work.
6. **Q12** (G1–G5 fixtures) — gates real conformance claims.

Remaining questions are independently resolvable.

---

## Strategic / architectural questions

### Q1. Engine-adapter posture — "held" vs "Phase 5 commitment"

**The conflict.** Current plan (`2026-04-18-wos-remainder-di-seam-framing.md`) says §7 engine adapters are *held until commercial demand*. Prior plan (`thoughts/plans/2026-04-13-wos-runtime-crate.md` step 5) treats `wos-temporal` as an active Phase 5 commitment. `WOS-IMPLEMENTATION-STATUS.md:82` lists Temporal Workflow as an unchecked Phase 1 blocker.

| Option | Tradeoff |
|---|---|
| **A.** Retire prior Phase 5; current plan's "hold" wins | Simpler story; matches our actual capacity; but contradicts status doc (must update) and loses the adapter-demonstration value |
| **B.** Reinstate Phase 5 as active; elevate H4 skeleton to real impl | Largest credibility lift (demonstrates portability); unlocks first paying customer who asks "does it run on Temporal"; costs ~4 weeks of adapter work |
| **C.** Split: current plan's "hold" for full adapter, but land H1/H3/H4 as adapter-enablement; make Phase 5 the next commitment after Phase 7 | Middle ground; we preserve the hold posture while clearing every structural blocker so "hold" becomes a 2-week commitment away, not 2-month |

**Tentative recommendation:** C. It reconciles both positions without forcing a capacity decision now. Update `WOS-IMPLEMENTATION-STATUS.md:82` to say "unblocked after Phase 7, ships on demand trigger."

---

### Q2. Spec §9 G4 vs H1 (timer extraction) — normative conflict

**The conflict.** `specs/kernel/spec.md` §9 G4: *"Timer state is part of the CaseInstance (§S3.1) and is persisted at every durability checkpoint."* H1 (current plan) proposes extracting timer state to a `TimerService` seam.

| Option | Tradeoff |
|---|---|
| **A.** Amend spec §9.1 G4 + §S3.1 + §6.1 to "timer state MUST be durably persisted, placement implementation-defined" | Preserves H1; loses one sentence of specificity; opens door to divergent impls if not carefully worded |
| **B.** Keep spec as-is; abandon H1 | No engine-adapter path without spec change later; we pay migration cost at first-commercial-demand time |
| **C.** Land H1 as extension point, not replacement — keep `CaseInstance.timers` snapshot as canonical (spec-conformant), but also populate `TimerService` seam mirroring the same state | Adapter-ready + spec-conformant; but duplicates state (two sources of truth → consistency bugs) |
| **D.** Per-processor profile flag — "embedded-timer profile" (conformant) vs "externalized-timer profile" (requires spec carve-out) | Formalises both modes; most complex normatively |

**Tentative recommendation:** A. The existing wording over-specifies — G4's real content is "durable + fires within tolerance," not "stored in X JSON field." Option C's dual-source hazard outweighs its compatibility. Needs 2–3 spec-PR sentences.

---

### Q3. Chain posture (G6 ADR): WOS `previous_hash` redundant with Ledger Merkle chain?

**The open question.** A5 ships a `/provenance/verify` endpoint. What does "valid" mean when both WOS's hash-chain and the Formspec Respondent Ledger's Merkle chain exist?

| Option | Tradeoff |
|---|---|
| **A.** Defense-in-depth — both chains always, independent verification | Strongest audit posture; two tamper-evidence systems fail independently; but operational complexity (every record signed twice) + storage overhead |
| **B.** Transitional — WOS chain retires when Ledger is wired; `NoopLedgerVerifier` deployments keep WOS chain as substitute | Cleanest long-term; single source of tamper-evidence per deployment; but migration semantics tricky for long-running instances |
| **C.** Per-deployment choice — `ChainPosture::{DefenseInDepth, LedgerOnly, WosOnly}` in `ServerConfig`; ADR documents when each is appropriate | Maximum flexibility; but forces every consumer to think about it; 3× the testing matrix |

**Tentative recommendation:** A with explicit opt-out. Default to defense-in-depth (covers cases where Ledger doesn't see — kernel transitions, governance state); allow `LedgerOnly` when the Ledger is authoritative over the full record shape. Option C's matrix blowup isn't worth it.

---

### Q4. Export framing (IDEA_SCRATCH §23) — run-on-Temporal vs export-to-Temporal

**Two orthogonal meanings.** `IDEA_SCRATCH.md:23` frames Temporal as an **export target** ("export to engine-specific formats (BPMN, Temporal, SCXML) as interop targets"). The reference doc (`thoughts/examples/temporal-reference-implementation.md`) frames it as a **runtime adapter** (embed WOS in a Temporal worker).

| Option | Tradeoff |
|---|---|
| **A.** Run-on only (current H4 framing) | Matches reference doc; one path; clean adapter story |
| **B.** Export-to only | Lower coupling; WOS ships a compiler, Temporal runs it; but loses runtime governance (deontic checks, confidence floors, fallback chains) — these would compile into Temporal workflow code lossily |
| **C.** Both — run-on as primary, export-to as separate secondary artifact | Largest scope; serves genuinely different consumer needs (our-runtime vs their-engine-too) |

**Tentative recommendation:** A, with a one-paragraph disposition retiring B as out-of-scope. Export-to is structurally weaker for WOS because our governance semantics don't compile cleanly into host-engine primitives — it would be a second, degraded semantic surface. Retire to avoid scope creep.

---

## Design-level questions

### Q5. B1 implementation path — `SubmitPolicy` trait vs `validate_in_context` extension

**The choice.** B1 (Phase 2) needs to enforce Runtime §15.7 ledger-gating. The check requires `impactLevel` which `ContractValidator::validate(&self, contract_ref, data)` doesn't have access to.

| Option | Tradeoff |
|---|---|
| **Path 1.** New `SubmitPolicy` trait object, parallel to `CompanionPolicy`; injected via `with_submit_policy` builder | Clean conceptual separation from `CompanionPolicy` (which runs on every event, not just submits); explicit submit-boundary semantics; but adds another trait to wire |
| **Path 2.** Extend `ContractValidator` with `validate_in_context(contract_ref, data, impact_level, instance_id)` default method | Additive, non-breaking; reuses existing trait; but conflates pure-contract validation with runtime-context policy (mixed concerns) |

**Tentative recommendation:** Path 1. The conceptual overlap concern (Path 2) is real — `validate_in_context` would mean "validate contract AND check runtime policies" which is two different things. Keeping them separate via `SubmitPolicy` matches the `CompanionPolicy` pattern and makes defaults easier to override individually.

---

### Q6. B5 Hold CRUD — wait for #20 (Option A) vs synthetic-provenance now (Option B)

**The tradeoff.** B5's endpoints emit `holdApplied` / `holdReleased` events that aren't spec-ratified yet. Routing them through `AppRuntime::enqueue_event` records intent but doesn't mutate `governance_state.active_holds`.

| Option | Tradeoff |
|---|---|
| **A.** Wait for spec #20 (typed event vocab) to land | Clean semantics; no bypass of event machinery; cost: consumers waiting for hold endpoints must wait |
| **B.** Direct storage writes + synthetic provenance record now | Consumers unblocked; but bypasses event processing — two code paths for state mutation (one through runtime, one direct) |

**Tentative recommendation:** A. Option B's two-code-path problem tends to calcify — "temporary" direct writes become permanent because retrofitting them through events is work nobody prioritizes. If consumer pressure mounts, ship B **with a deprecation timer on the direct path** (6-month removal target).

---

### Q7. `AuthKind::Oidc` — ship a reference impl or leave to consumers (G1 follow-up)

**The question.** G1 drops `login` from the auth trait and widens `AuthUser`. That's the trait-level fix. But do we also ship a real OIDC impl?

| Option | Tradeoff |
|---|---|
| **A.** Ship a reference OIDC impl (e.g., against Keycloak or a generic OIDC provider) | Consumers plug-and-play; maintenance burden on us; requires picking one library (e.g., `openidconnect` crate) |
| **B.** Leave OIDC to consumers — only ship the trait + JWT + Mock | Smallest surface; consumers write their own (correctly-configured for their IdP); minor friction for "just want Okta working" consumers |
| **C.** Ship a skeleton impl that validates JWKS + extracts claims, but doesn't handle flows | Middle ground; covers 80% case (token verification); consumers layer flows on top |

**Tentative recommendation:** C. Flows vary wildly by IdP (auth-code vs implicit vs device-code vs PKCE); shipping one blessed flow picks a fight we don't need. But JWKS-based token verification is near-universal — shipping a `GenericOidcVerifier { jwks_url, audience, issuer }` covers most enterprise cases.

---

### Q8. `ValidationResult.errors` widening (G4) — break `Vec<String>` or keep both

**The question.** G4 proposes widening `ValidationResult.errors` from `Vec<String>` to `Vec<FieldError>`. Is this a clean break or an additive extension?

| Option | Tradeoff |
|---|---|
| **A.** Break it cleanly — `errors: Vec<FieldError>` only | Forces all consumers to adopt structured errors; cleanest long-term; pain for existing impls |
| **B.** Keep both — `errors: Vec<String>` + `field_errors: Option<Vec<FieldError>>` | Additive; no breakage; but two shapes for the same concept → drift |
| **C.** Tagged-union — `errors: Vec<ErrorVariant>` where `ErrorVariant = String(String) \| Field(FieldError)` | Additive-but-typed; serde-wise messier; consumers still pattern-match |

**Tentative recommendation:** A. G4 is flagged D=5 precisely because every day we delay multiplies consumers wiring against the old shape. Pre-1.0 is the moment for this break. 3 production impls + 1 pending from B1 = 4 impl migrations; manageable.

---

### Q9. Agent invocation — subsume under `ExternalService` or separate `AgentInvocationService`

**The question.** Reference doc §6 specifies a rich agent-invocation pipeline (autonomy, deontic checks, confidence floor, fallback chain). Current `ExternalService::invoke` signature is generic `(service_ref, input, idempotency_key) -> Result<Value>` — no room for that pipeline.

| Option | Tradeoff |
|---|---|
| **A.** Unified `ExternalService` with agent flag | One seam for all external calls; but `invoke` signature gets bloated (confidence reports, deontic context, autonomy level all plumbed through) |
| **B.** Separate `AgentInvocationService` trait | Clean typed surface for agent-specific protocol (confidence, autonomy, fallback); but duplicates retry/idempotency/correlation logic |
| **C.** `AgentInvocationService` extends/wraps `ExternalService` — agent-specific seam delegates transport to ExternalService, owns governance pipeline | Best of both; agent governance stays typed; transport stays unified; composition is one more layer of indirection |

**Tentative recommendation:** C. The reference doc's agent-invocation pipeline is too structured to fit inside `ExternalService::invoke`'s generic signature. But the actual HTTP/gRPC transport IS the same. Composition is the correct answer.

---

## Tactical / scope questions

### Q10. `wos-temporal` skeleton (H4) depth

| Option | Tradeoff |
|---|---|
| **A.** Zero-code stubs (current H4) — just module declarations + doc comments | ~1 hour; pure signpost; may stale-rot without compiler pressure |
| **B.** Compile-valid trait impls — every trait G7/H1 will need, stubbed with `unimplemented!()` | ~half day; forces trait surface to stay API-stable; CI catches drift |
| **C.** Minimal runnable — in-process mock of Temporal workflow with signal/query loop, calling into wos-runtime | 2–3 days; proves the design works end-to-end; much stronger demonstration artifact |

**Tentative recommendation:** B. A stays frozen too easily; C is too much commitment without demand signal.

---

### Q11. Compensation seam (Kernel §9.5)

**Context.** Search finding #4: kernel §9.5 normatively declares `compensatingAction` on actions and `compensable: true` on scopes; full saga execution semantics deferred to a "Lifecycle Detail companion." Current plan touches neither.

| Option | Tradeoff |
|---|---|
| **A.** Add schema support now — `compensatingAction` on actions + `compensable: true` on scopes; no execution semantics | Normative conformance; no runtime cost; Lifecycle Detail companion handles semantics later |
| **B.** Full saga semantics now — reverse ordering, pivot steps, forward/backward recovery | Large scope; depends on Lifecycle Detail companion existing |
| **C.** Defer entirely — note as debt | Contradicts kernel §9.5 being normative |

**Tentative recommendation:** A. Schema MUST land because it's normative; full execution semantics can wait. Add a one-item task to Track C or E for the schema work.

---

### Q12. Durable-execution G1/G2/G3/G5 conformance verification

**Context.** Search finding #2: kernel §9.1 + runtime §6.1 define five normative durable-execution guarantees. None of G1/G2/G3/G5 have conformance tests. Only G4 (timers) is indirectly addressed via H1.

| Option | Tradeoff |
|---|---|
| **A.** Add conformance fixtures for each G1/G2/G3/G5 (4 fixtures) | Low-cost (~1 day); catches regressions; asserts existing impl actually conforms |
| **B.** Add server-side integration tests only | Faster but doesn't constrain other implementations |
| **C.** Formal verification via replay harness — prove G2 across random event sequences | Strongest; expensive; probably overkill for v0.1 |

**Tentative recommendation:** A. Fixtures are the WOS-native mechanism; they compound value across every implementation, not just ours.

---

### Q13. `inputDigest` / `outputDigest` on provenance records

**Context.** Search finding #5: the 2026-04-09 companion review lists these as "research-validated kernel additions" for lightweight tamper detection. Not in current plan.

| Option | Tradeoff |
|---|---|
| **A.** Add to Track E2 cheap batch — schema extension + computation in provenance emit path | Cheap; adds tamper-detection before ledger is wired; ~half day |
| **B.** Defer until Ledger is wired — Ledger's inclusion proof makes digest redundant | Scope-saver; but leaves gap for deployments without Ledger |
| **C.** Add as optional fields only — present when computed, absent otherwise | Additive; consumers choose |

**Tentative recommendation:** C. Optional fields match the rest of the provenance schema's pattern; consumers without ledger get tamper-detection; consumers with ledger can skip.

---

### Q14. H1 timer migration path

| Option | Tradeoff |
|---|---|
| **A.** Big-bang migration — version bump, scripted fixture rewrite, one PR | Fast; clean; consumer pain concentrated |
| **B.** Dual-write window — `CaseInstance.timers` keeps old shape, new `TimerService` also populated, read from either | Graceful; consumers migrate at their pace; but double-state risk (see Q2 option C tradeoff) |
| **C.** Feature-flag — `WOS_TIMER_MODE=embedded \| externalized`; consumers opt in | Flexible; but doubles test surface permanently |

**Tentative recommendation:** A. Pre-1.0 is the time for this; dual-write invites the exact inconsistency bug we're fixing. Scripted migration + one focused sprint.

---

### Q15. `VisibilityService` granularity (G11)

| Option | Tradeoff |
|---|---|
| **A.** 4 typed methods (case_status, case_file, governance_state, list_cases) — current plan | Typed returns; but rigid; adding new queries = trait change |
| **B.** Unified `query(Query) -> QueryResponse` with enum dispatch | Flexible; adding queries = enum variant; but lose type specificity |
| **C.** Typed methods for common ones + escape-hatch `query_custom` | Best of both; complexity cost |

**Tentative recommendation:** A. Visibility queries are a small, stable set (4 in reference doc, unlikely to grow fast); rigidity is a feature. Revisit if a real consumer needs a fifth.

---

## Cross-cutting

### Q16. `wos-lambda` (mentioned in ADR-0057:234 but not in plan)

**Context.** ADR-0057 lists expected adapter crates: `wos-temporal` (covered by H4) AND `wos-lambda → DynamoStore + S3Resolver + FormspecValidator + HttpService (future)`. Current plan doesn't mention wos-lambda.

| Option | Tradeoff |
|---|---|
| **A.** Add parallel `wos-lambda` skeleton (H4b) | Symmetric with wos-temporal; signals "multi-engine target"; ~half day |
| **B.** Skip; add to backlog | Simpler; wos-temporal is the reference, Lambda derives from it |
| **C.** Add only if a second consumer asks | Minimum-commitment |

**Tentative recommendation:** B. One reference adapter is enough to constrain the trait surface; a second skeleton doesn't add design pressure. Note wos-lambda as a backlog item under Track F.

---

## Decision log

Record decisions here as they land. Format: `Qn — <date> — <option> — <one-line rationale>`.

- Q1 — *pending*
- Q2 — *pending*
- Q3 — *pending*
- Q4 — *pending*
- Q5 — *pending*
- Q6 — *pending*
- Q7 — *pending*
- Q8 — *pending*
- Q9 — *pending*
- Q10 — *pending*
- Q11 — *pending*
- Q12 — *pending*
- Q13 — *pending*
- Q14 — *pending*
- Q15 — *pending*
- Q16 — *pending*

---

## Cross-reference

- **Plan under revision:** `thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`
- **Reference architecture:** `thoughts/examples/temporal-reference-implementation.md`
- **Prior plan (conflicts with current):** `thoughts/plans/2026-04-13-wos-runtime-crate.md`
- **Prior ADR naming adapter crates:** `thoughts/archive/adr/0057-wos-core-implementation-boundary.md`
- **Companion review (G1–G5 guarantees, research-validated additions):** `thoughts/reviews/2026-04-09-wos-core-companion-review.md`
- **Spec sections affected:** `specs/kernel/spec.md` §9 · `specs/companions/runtime.md` §6
- **Status doc to reconcile:** `WOS-IMPLEMENTATION-STATUS.md` line 82
- **IDEA_SCRATCH framing divergence:** `IDEA_SCRATCH.md` line 23



