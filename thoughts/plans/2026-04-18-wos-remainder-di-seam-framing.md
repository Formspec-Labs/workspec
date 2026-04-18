# Plan: Remainder of WOS spec + wos-server work (DI-seam framing)

## Context

This plan sequences the work still open in both **TODO.md** (spec-side) and **crates/wos-server/PARITY.md** (server-side) after the 11-phase parity push, three validation passes, and the DI-seam reframe landed on branch `claude/wos-spec-backend-y17wJ`. Last commit on branch: `93f30b1`.

**What's already done (not re-litigated here):**

- `wos-server` ships every layer of the original parity plan: kernel/instance/task endpoints, provenance hash chain + PROV-O/XES/OCEL export, governance L1 (reads + delegations CRUD + deontic violation listing), agent registry + AI lifecycle L2, advanced L3 stubs (SMT, equity, zones), assurance layer, integration-profile inbound + invoke stub, business-calendar + notification sidecars, lint + conformance REST.
- Three audit passes completed: spec citations (6 corrections applied), server surface (2 discrepancies fixed), status grades + ROI math (4 regrades + 4 rescores + table resort).
- `TODO.md` and `PARITY.md` synced around the **DI-seam framing**: every envelope-stack concern reduces to wiring one of Runtime §12's nine host-interface traits. Attestation = `ProvenanceSigner` seam. Explanation rendering = `ReportRenderer` seam. Separation of duties = `AccessControl` seam composition. Integration dispatch = `ExternalService` seam composition. Ledger gating (§15.7) = `ContractValidator` seam composition.

**The framing consequence:** the remaining work splits cleanly into (a) wire two unwired seams + tighten three stubbed ones; (b) ship the spec items that feed the seams (deterministic explanation algorithm, typed event vocabulary, envelope status extensions); (c) fixtures that lock patterns so integrators don't diverge; (d) behavioral backlog + engineering hygiene that don't depend on any seam.

**Intended outcome:** a reference runtime + spec pair that a third party can compose with Formspec + an identity adapter + a PDF/email layer to ship a DocuSign-competitive e-signature product, without forking either spec or server. Every DI seam is either wired or has a no-op default that ships spec-correct response shapes; every envelope-flow pattern has a canonical fixture; every normative MUST in the spec has either enforcement or a failing conformance test marking the gap.

---


## Remaining work at a glance

**Server side (PARITY.md top-ROI rows still open):** 6 P0/high-priority items — wire `ProvenanceSigner` seam (ROI 25), wire `ReportRenderer` seam (25), legal-sufficiency disclosure on exports (20), `PolicyLayeredValidator` with §15.7 ledger-gating (12.5), `RoleBasedAccessControl` separation-of-duties (12.5), `/explain` handler (12.5, rides on ReportRenderer + spec #2). Plus ~10 medium-ROI items (event idempotency, policy as-of resolution, chain-integrity verify endpoint, subject continuity-hash, hold CRUD, `IntegrationDispatchService` with correlation tokens, pipeline validation endpoint, calibration expiry, migration endpoint, drift write-side endpoint).

**Spec side (TODO.md open items):** 7 items in §4.1 critical path (DRAFTS triage, #24a facts-tier snapshot, #23 OverrideRecord, NoticeTemplate reconciliation, #2 adverse-decision notice, #20 typed event vocabulary, #31 jurisdiction-aware calendar) + **1 proposed addition C8** (Runtime §9.1 explanation algorithm — see Track C); 6 in §4.2 next-batch; 6 in §4.3 cheap batch (parallelizable); 13 in §4.4 behavioral backlog; 4 in §4.7 envelope-stack enablement (new items #58–#61); 3 structural merges in §4.5; 2 hygiene in §4.6; §5/§6/§7 downstream.

**Completed and excluded from this plan:** the 11-phase parity implementation, three validation audits, DI-seam sync, all existing PARITY rows marked `full`.

**Explicit out-of-scope for this plan:** Formspec Respondent Ledger cryptographic primitives (upstream, plug via `ProvenanceSigner`); real Z3 solver / real SHACL engine / real drift detector computation / in-server SPARQL triplestore (consumer-injected when demanded); multi-step sessions, agent circuit breakers, counterfactual explanation (deferred for consumer-demand signal); studio UI changes (separate effort); Correspondence-vs-Notification merger (spec editorial, not implementation).

---

## Guiding principles

1. **DI seams are the contract.** Runtime §12's nine host-interface traits are the composition surface. Server's job is to accept consumer-injected implementations, provide sensible no-op defaults, and enforce that seams are wired when the spec (e.g. §15.7) demands it. Every "build X primitive" temptation that's not a seam gets refused — it goes out to consumers via a trait.

2. **Stubs ship spec-correct shapes.** Every stub response already returns the envelope the spec calls for; swapping a stub for a real impl is transparent to consumers. Never replace a stub with a 501 — keep the shape and let the stub document its noop status in the payload.

3. **Spec work feeds server work.** #2 (deterministic explanation algorithm) unblocks the `/explain` handler. #20 (typed event vocabulary) unblocks envelope-flow fixtures. #23 (OverrideRecord) unblocks SoD conformance fixtures. #30 + #58 (task + instance envelope lifecycle) unblocks decline flows. Sequence the server work to ride behind the spec work, not to block on it — ship the seam first with a `Noop` default, then swap to real algorithm/data when spec lands.

4. **Fixtures lock the patterns.** Every reference composition (2-signer sequential, parallel-witness, etc.) ships as a canonical fixture. Without that, integrators diverge and the ecosystem fragments. Fixture work is load-bearing, not decorative.

5. **Compounding debt first.** Within the ranked list, every row flagged D=5 goes before every row flagged D≤3, regardless of priority. Breaking-change exposure compounds per consumer per week.

---

## Track A — server DI seam wiring + easy quick wins

These land in the first week. None require spec work or external dependencies. Most are under a day of work each.

### A1. Wire `ProvenanceSigner` seam (~4–6 hr)

Runtime §12.6 defines the trait. `wos-core::traits::ProvenanceSigner` already exists, but `WosRuntime::new` (at `crates/wos-runtime/src/runtime.rs:347-383`) takes only the seven generic params `store/resolver/presenter/access/service/validator/clock` — **no signer slot**. Wiring requires:

- **wos-runtime extension** (`crates/wos-runtime/src/runtime.rs`): add `Box<dyn ProvenanceSigner>` field on `WosRuntime` struct, add `with_provenance_signer` builder method (mirrors existing `with_companion_policy` at line 386), thread signer into every provenance-emit site (grep `append_provenance` + internal record builders). No call site currently computes signatures; this adds the first.
- **wos-server new file** `crates/wos-server/src/runtime/signer.rs` — `NoopSigner` default (returns empty `Vec<u8>` signature, stamps `attestation.signer = "noop"` into the record's metadata), placeholder `Ed25519FileKeySigner` feature-gated behind `--features signer-ed25519`.
- **Config**: `ServerConfig::signer_kind` enum (`Noop` | `Ed25519File` | `External`) + env `WOS_SIGNER=noop|ed25519-file|external`. Same pattern as existing `StorageKind` / `AuthKind` (precedent in `config.rs:135,140`).
- **`AppRuntime::build`** (`runtime/mod.rs:44-87`) — extend the `ConcreteWosRuntime` type alias to pin the new generic or `Box<dyn>` slot; call `.with_provenance_signer(signer)` at build time.
- **Default response envelope** carries `{ "attestation": { "signer": "noop", "signature": "", "keyId": null } }` so consumers see the field even when no signing happens.
- **Tests**: existing wos-runtime tests construct `WosRuntime::new` directly — audit and update if the signer slot is required at construction vs via builder.

### A2. Wire `ReportRenderer` seam (~4–6 hr)

Runtime §12.7 trait. Same pattern as A1 — requires the same wos-runtime builder extension. `ReportRenderer` has no slot on `WosRuntime::new` today; add `with_report_renderer` builder method alongside the signer one (can land in a single wos-runtime PR).

- **wos-server new file** `crates/wos-server/src/runtime/renderer.rs` — `JsonReportRenderer` default (passes payload through as JSON), feature-gated `HtmlReportRenderer` skeleton for week 2.
- **Config** `ServerConfig::renderer_kind` + env `WOS_RENDERER=json|html`.
- **`AppRuntime::build`** — inject renderer.
- **Unblocks A4** (`/explain` handler).

### A3. Legal-sufficiency disclosure on exports (~30 min)

- Edit `crates/wos-server/src/services/semantic_service.rs` — wrap the PROV-O / XES / OCEL payload generators with a `wosDisclosure` block carrying `{ conditions: [...], specSection: "assurance/assurance.md §6", implementationProfile: "wos-server/0.1" }`. Block is empty-list when no claims are made; populated when attestation is wired.
- New unit tests asserting the disclosure block is present and valid in every format.

### A4. `/instances/:id/explain` handler (~4 hr — handler + scaffold service; blocked on A2 + explanation-algorithm decision)

**Scope correction vs prior draft:** Spec TODO #2 is `Governance §3.2 — adverse-decision notice`, which produces machine-readable + human-prose **notices**. Runtime §9.1 (`specs/companions/runtime.md`) is a **separate** deterministic algorithm for **explanation assembly** — no TODO item currently owns its implementation. The `/explain` endpoint depends on §9.1, not #2 directly; the two algorithms share skeleton but have different output shapes. See new Track C8 below.

- New handler in `crates/wos-server/src/http/instances.rs` (~50 lines, delegation pattern mirrors `http/applicant.rs`).
- New scaffold service `crates/wos-server/src/services/explanation_service.rs` (~150–250 lines) — implements a minimal §9.1-shape payload (`explanationLevel`, `reasoning`, `rulesApplied`, `authorityRanking`, `counterfactuals`) populated from currently-available provenance. Response payload carries `algorithmId: "wos-server-scaffold-0.1"` so consumers know the output is pre-§9.1.
- When the real §9.1 algorithm lands (Track C8), the scaffold's internals swap without changing the wire shape. The `applicant_service::determination` view stays as-is — it's an applicant-facing projection, not §9.1's spec shape, so it's NOT the right scaffold source.

### A5. Chain-integrity verify endpoint (~1 hr)

- New handler `GET /api/instances/:id/provenance/verify` in `crates/wos-server/src/http/instances.rs`.
- Wraps existing `ProvenanceService::verify_chain` helper (already defined at `crates/wos-server/src/services/provenance_service.rs:111`, zero callers today).
- Response: `{ valid: bool, firstBrokenSeq: Option<i64>, reason: Option<String> }`.

### A6. JSON-LD context endpoint (~30 min)

- New handler `GET /api/semantic/jsonld-context` in `crates/wos-server/src/http/semantic.rs` (new file, or fold into existing integration.rs).
- Static file served from `fixtures/semantic/context.jsonld` (copy from existing wos-export crate's embedded context).

### A7. Event idempotency on `POST /events` (~2 hr)

- Add `idempotencyToken: Option<String>` to event submission body.
- In `SqliteRuntimeStore::enqueue_event`, check the `event_queue` table for an existing row with the same instance + token within a 24h window before insert.
- Return the prior result's shape on dedupe hit.
- New integration test in `tests/http_smoke.rs` asserting dupe token → single drain.

### A8. Policy-parameters as-of resolution (~2 hr)

- New handler `POST /api/policy/:url/resolve?asOf=<RFC3339>` in a new `crates/wos-server/src/http/policy.rs`.
- Iterate date-indexed values in the policy-parameters sidecar; return the entry whose `[validFrom, validUntil]` contains the asOf instant.
- Body: `{ parameterId: String }`; response: `{ value: serde_json::Value, validFrom, validUntil }`.

### A9. Subject continuity-hash validation (~2 hr, ship with explicit algorithm label)

- Extend `GET /api/subjects/:ref/assurance-chain` response in `crates/wos-server/src/http/assurance.rs` with `chainValid: bool`, `firstInvalidId: Option<String>`, and **`algorithmId: "wos-server-0.1-sha256-canonical"`** so a future spec-ratified algorithm can coexist without silently invalidating stored responses.
- Continuity-hash construction: SHA-256 over canonical-JSON of each fact + prior fact's hash. Seed from `subject_ref`. Canonical-JSON form is itself under-specified in WOS — document the specific canonicalizer used (sorted keys, UTF-8, no whitespace) in the server README and flag as a spec gap to escalate.
- If spec #62 or similar ratifies a different algorithm later, responses already *served* remain correct-for-their-algorithm. **Stored hashes** in the database are another matter: migrating to a new algorithm requires either a dual-column storage schema (both algorithms retained for read) or a re-hash migration on the existing chain. Flag as a known forward-compat cost and track alongside C8's landing.

---

## Track B — server seam tightening (replace stubs with policy-composing impls)

These replace the three stubbed seams with real policy-aware implementations. Under a day each. Track B follows Track A in sequence — the quick wins are completed first so the stubbed seams are the only ones left.

### B1. `PolicyLayeredValidator` + submit-path policy hook (~1 day)

**Architectural correction vs prior draft:** `ContractValidator::validate(&self, contract_ref, data)` at `wos-core/src/traits/mod.rs:70-80` **does not have access to the workflow's `impactLevel`** — it only sees the contract reference and the payload. The §15.7 ledger-gating check needs `impactLevel`, which lives at the submit-path in `WosRuntime` (the runtime holds the kernel + governance + instance when a task response arrives).

Revised design — two landing sites:

- **`PolicyLayeredValidator`** replaces `PermissiveValidator` for the pure-contract layer:
  - Formspec contract validation (delegate to injected `FormspecProcessor`, permit when absent).
  - Signature-class **shape** checks once spec #43 lands (e.g. payload has a `signatureClass` field whose value is in the allowed enum).

- **Submit-path policy hook** (`wos-runtime::submit_task_response` or a new trait object wired alongside) handles checks that require runtime context:
  - Runtime §15.7 ledger-gating: if `instance.impact_level ∈ {rights-impacting, safety-impacting}`, reject submits lacking `respondentLedgerRef`. Emit the normative `ledgerEvidenceMissing` failure per §S15.5.
  - Signature-class ↔ assurance-level binding (#43): compare submit's `signatureClass` to instance's recorded `AssuranceLevel`; reject under-levelled.

**Two implementation paths — the plan leaves the choice open; resolve at implementation start:**

- **Path 1 — new `SubmitPolicy` trait object.** Parallel to `CompanionPolicy` (at `crates/wos-runtime/src/companion.rs`), injected via `with_submit_policy` builder. **Distinction from `CompanionPolicy`:** `CompanionPolicy::evaluate_event` is invoked *before lifecycle processing* on every event (deontic / autonomy / confidence gating). `SubmitPolicy` would be invoked specifically at task-response-submit boundary, where the instance's `impactLevel` is known and §15.7's ledger-evidence requirement applies. One could argue this belongs inside `CompanionPolicy` with a new event-type discriminator — that's defensible but widens `CompanionPolicy`'s contract.
- **Path 2 — extend `ContractValidator` with `validate_in_context` default method.** Non-breaking addition: new trait method `validate_in_context(contract_ref, data, impact_level, instance_id)` with a default impl delegating to `validate`. `PolicyLayeredValidator` overrides the default and performs §15.7 gating. Existing `ContractValidator` impls keep compiling. Simpler; preferred unless a stronger case surfaces for a separate trait.

`wos-server` ships `LedgerGatingSubmitPolicy` (Path 1) or the layered-validator override (Path 2) as the default-on policy.

Unblocks §15.7 conformance.

### B2. `RoleBasedAccessControl` (~half day)

Replace `crates/wos-server/src/runtime/access.rs::PermissiveAccessControl`. Enforce:

- **Separation of duties (Gov §7.2, AI §1.5):** on transitions tagged `review` (or any tag where the kernel declares `reviewRole`), reject when the caller's identity equals the author of the artifact being reviewed. Authorship is read from the latest provenance record touching that artifact.
- **Delegation chain validation (Gov §6):** when the caller is acting under a delegation, verify the delegator → delegate chain is live (not revoked, within `validFrom/validUntil`) and that the scope covers the attempted action.
- **Autonomy cap (AI §5.3, pre-calibration expiry):** for AI-actor-typed callers, reject when declared autonomy level exceeds the workflow's `impactLevel` ceiling per #43 when it lands.

Policy source is pluggable — internal `PolicyEvaluator` seam so integrators can swap OPA / Cedar / custom without forking `RoleBasedAccessControl`.

### B3. `IntegrationDispatchService` + correlation tokens (~1 day, **non-breaking**)

Replace `crates/wos-server/src/runtime/service.rs::EchoExternalService`. Read integration bindings from the resolver, dispatch on `IntegrationBindingKind`:

- `RequestResponse` → reqwest POST with the binding's request contract.
- `EventEmit` → publish via Socket.IO + optional webhook per binding config.
- `ArazzoSequence` → sequential multi-step dispatch (parallel is stretch).
- `Tool` → CWL-informed invocation (stub returns declared output shape until a real tool-runner lands).
- `PolicyEngine` → external adapter via `PolicyEngine` trait (XACML / OPA / Cedar); default is `EchoPolicy` with `{ decision: "permit" }`.

**Correlation tokens — non-breaking pattern.** `ExternalService::invoke(&self, service_ref, input, idempotency_key)` lives at `wos-core/src/traits/mod.rs:92-103`, with **9 call sites** across `wos-runtime/integration_handlers/*` + `wos-conformance/stubs.rs` + `wos-server/runtime/service.rs`. Rather than add a fourth positional argument (breaking every caller + downstream adapter), add a default trait method:

```rust
fn invoke_with_correlation(
    &self,
    service_ref: &str,
    input: &serde_json::Value,
    idempotency_key: Option<&str>,
    correlation_token: Option<&str>,
) -> Result<serde_json::Value, Self::Error> {
    let _ = correlation_token;
    self.invoke(service_ref, input, idempotency_key)
}
```

Old adapters keep working; new integration handlers call `invoke_with_correlation`. Wire a callback registry in wos-runtime that resolves callbacks to pending invocations via the token. Socket.IO broadcast + `POST /api/events/inbound` already exist; correlation makes the round-trip complete.

This is phase-2 work precisely because the breaking-change risk is gone: the pattern is additive, so existing consumers can't break. The urgency drops from "must land before adapters ship" to "should land before correlation-dependent adapters ship" — still important, but no longer a compound-debt trap.

### B4. Pipeline validation endpoint (~1 day, blocked on spec #38)

- New handler `POST /api/governance/:url/validate-pipeline` in `crates/wos-server/src/http/governance.rs`.
- Body: `{ pipelineId: String, inputs: Value }`.
- Resolves assertion references via spec #38's `assertionId` protocol (once landed); evaluates each assertion via FEL expressions; returns `{ passed, failures: [{ assertionId, message }] }`.
- Scaffold the endpoint now with the current unresolved-reference behavior (fail-open with "pipeline references not yet resolvable" note) so consumers can integrate against the shape; swap to real evaluation when #38 lands.

### B5. Hold CRUD (~half day — needs spec-backed event vocabulary)

**Caveat from feasibility review:** the event names `holdApplied` / `holdReleased` are not spec-ratified today. Routing them through `AppRuntime::enqueue_event` records the intent in provenance as `UnmatchedEvent` (same pattern as `applicant_service::submit_appeal`) but **does not mutate `governance_state.active_holds`** because no kernel action handles these events. Shipping the endpoints this way records intent without effect — a half-measure.

Two implementation options:

- **Option A (recommended):** defer B5 until spec #20 (typed event vocabulary, Track C6) lands with `holdApplied` / `holdReleased` as standard events, and tie the runtime's hold-lifecycle handling to them. Cost: one additional sprint of wait.
- **Option B (pragmatic):** ship the endpoints now with direct storage writes to `governance_state.active_holds` + a synthetic provenance record (not routed through `enqueue_event`). Cleaner behaviour but bypasses the runtime's event processing — consumers see the hold state change but not through the standard transition machinery. Document the deviation.

New handlers: `POST /api/instances/:id/holds`, `DELETE /api/instances/:id/holds/:holdId` in new `crates/wos-server/src/http/holds.rs`. Reads project `governance_state.active_holds` unchanged in either option.

### B6. Calibration expiry enforcement (~3 hr)

- Extend existing `timer_task.rs` to check agent calibration `validUntil` every tick and cap autonomy to `assistive` when expired. Emit `calibrationExpired` provenance record + `CalibrationRequired` CloudEvent so external recalibrators can schedule.
- No new endpoint needed; enforcement is internal.

### B7. Migration endpoint (~1 day)

- New handler `POST /api/instances/:id/migrate` in `crates/wos-server/src/http/instances.rs`.
- Wraps `WosRuntime::migrate` (already exists in wos-runtime).
- Body: `{ targetVersion: String, mapping: Option<Value> }`.
- Records a `SchemaUpgrade` provenance record per Gov §2.9.

### B8. Drift detection write-side endpoint (~3 hr)

- Add `POST /api/agents/:id/drift` in `crates/wos-server/src/http/agents.rs` for external detectors to write reports.
- Store latest per-agent report in a new `drift_reports` table (migration 0003).
- Existing `GET /api/agents/:id/drift` serves the stored report (currently returns the noop stub).
- Resolves the spec framing issue: processor stores+serves, detector computes externally.

### B9. Suspend / resume endpoints (deferred — only if a consumer asks)

- `POST /api/instances/:id/suspend`, `POST /api/instances/:id/resume`. Both wrap existing `WosRuntime` methods.
- Kept in scope but unscheduled until a real case materialises — no current consumer signal.

---

## Track C — spec critical path (TODO.md §4.1)

Items already prioritised as critical-path in TODO.md. All must land to unblock the envelope-stack work in Tracks D + E.

### C1. DRAFTS triage `[Imp 5 / Cx 3 / Debt 5]`

`DRAFTS/` contains 12 kernel-version proposals. Classify each as archive / delete / extract. **Blocks #20** (typed event vocabulary). Files are inert markdown but the review-time tax compounds as long as they sit.

Deliverable: every file in `DRAFTS/` either moved to `thoughts/archive/` with a one-line disposition note, or deleted with rationale in the commit message, or extracted into an active spec PR.

### C2. #24a Mandatory Facts-Tier input snapshot `[Imp 8 / Cx 4 / Debt 7]`

Tighten `Kernel §8.2` — case-file input snapshot MUST be populated and typed at `determination`-tagged transitions. Zero fixtures populate `inputs` today; ~51 fixtures touch determination transitions and need retrofitting.

Work:
- Spec change: Kernel §8.2 prose + JSON schema tightening of `ProvenanceRecord.inputs`.
- Fixture retrofit: walk `fixtures/conformance/**/*.json` and populate `inputs` on every `determination`-tagged transition record.
- New lint rule K-INPUT-001 (T2) asserting `inputs` is populated at determination transitions.
- **Silent dependency of #2.** **Unblocks #23.**

### C3. #23 OverrideRecord schema `[Imp 6 / Cx 2 / Debt 4]`

Promote Governance §7.3's three-field requirement (rationale + authority verification + supporting evidence) into a typed `OverrideRecord` `$def` in the workflow-governance schema. Part of the ADR sequence #23 → #24a → #2.

Work:
- Add `$def/OverrideRecord` to `schemas/governance/wos-workflow-governance.schema.json`.
- Reference from `Transition` and `ProvenanceRecord`.
- Add a conformance fixture exercising override-with-authority and override-without-authority rejection.

### C4. NoticeTemplate reconciliation `[Imp 7 / Cx 2 / Debt 5]`

Two conflicting NoticeTemplate schemas exist: thin `sections: string[]` in Due Process schema vs. rich `TemplateSection[]` with FEL conditions in Notification Template schema. Drop the thin version; Notification Template is canonical. **Blocks #2.**

Work:
- Remove the `NoticeTemplate` `$def` from `schemas/governance/wos-due-process.schema.json`.
- Update any governance-schema references to point to the Notification Template version.
- Migration: check if any fixtures use the thin version; if so, upgrade them.

### C5. #2 Deterministic adverse-decision notice (dual-form) `[Imp 9 / Cx 7 / Debt 6]`

The main deliverable. Specifies the deterministic algorithm that derives two co-synchronised outputs (machine-readable + human-prose) from the same Facts + Reasoning provenance. Identical inputs MUST produce identical outputs in both forms.

**Depends on:** #24a + #23 + NoticeTemplate reconciliation (C2 + C3 + C4 must land first).

Work:
- Spec: Governance §3.2 deterministic-algorithm pseudocode (similar structure to Runtime §9.1's explanation algorithm; share skeleton).
- Runtime seam: implement the algorithm in `crates/wos-runtime` or a new `crates/wos-notice` crate (decide based on whether the algorithm is reusable across explanation + notice).
- Delivery: Notification Template §4.4 FEL-conditional sections + `requiredVariables` enforcement.
- Fixtures: three determinism fixtures (same inputs → same outputs bit-for-bit) + one per explanation-level (individualised / categorical / aggregate).
- Server wiring: **this is what the `ReportRenderer` seam from A2 feeds.** Once #2 lands, the `/explain` handler (A4) switches from the scaffold `applicant_service::determination` payload to the real §9.1 algorithm output.

### C6. #20 Typed event meta-vocabulary `[Imp 8 / Cx 7 / Debt 6]`

Replace `Transition.event: string` with a typed union: `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. Also co-types `Action.event` for `startTimer`. Closes the kernel's last load-bearing openness.

**Depends on:** DRAFTS triage (C1).

Work:
- Spec change: Kernel §4.x — event taxonomy.
- Schema tightening: `schemas/kernel/wos-kernel.schema.json` — new `$def/TypedEvent`.
- Rust model change: `wos-core::Transition.event` goes from `String` to enum.
- Migration: ~168 fixture files need their `"event"` strings promoted to typed envelopes. **Do this as a scripted migration plus manual review**.
- Lint rule K-007 promotion to schema validation.
- **Unblocks #59 CloudEvent envelope-flow type catalog** and **unblocks #60 envelope fixtures** (both in Track D) — the typed vocabulary is the internal substrate that #59's cross-system catalog mirrors outward.

### C7. #31 Jurisdiction-aware business calendar selection `[Imp 6 / Cx 3 / Debt 4]`

Runtime resolution of which business calendar applies, read from a case-file field (e.g. `applicant.jurisdiction`). Replaces current "implementation-defined" selection.

Work:
- Spec change: Business Calendar sidecar §x — `calendarSelection.fromCaseField` property.
- Schema tightening.
- Runtime change in `crates/wos-runtime` — resolve the calendar at deadline-computation time using the case-state field.
- Server wiring: `crates/wos-server/src/services/calendar_service.rs` — pass resolved calendar ID through to `next_business_moment`.
- Conformance fixtures: one calendar-per-jurisdiction multi-deadline fixture.

**Compliance risk without this** for multi-jurisdiction rights-impacting workflows (eIDAS cross-border envelopes, for example).

### C8. Runtime §9.1 deterministic explanation algorithm (new TODO item, un-numbered in TODO.md) `[estimated Imp 7 / Cx 5 / Debt 5]`

`specs/companions/runtime.md` §9.1 specifies a deterministic explanation-assembly algorithm that no TODO item currently owns. Distinct from spec #2 (Gov §3.2 adverse-decision notice — same determinism principle, different output shape and consumer). A4's `/explain` handler depends on this.

Work:
- Propose a new TODO.md item (call it #62 or next-available) for "Runtime §9.1 explanation-assembly algorithm implementation" — add to TODO.md §4.1 critical path.
- Implementation can share the notice-assembly skeleton from #2 (both are deterministic provenance-driven derivations), but with §9.1's output shape (`explanationLevel | reasoning | rulesApplied | authorityRanking | counterfactuals`).
- Determinism fixtures: three fixtures asserting identical inputs → identical outputs.
- Once landed, `explanation_service.rs` swaps its scaffold for the real algorithm; `algorithmId` on the response bumps from `wos-server-scaffold-0.1` to `wos-runtime-§9.1`.

**Depends on:** #24a + #23 (same facts-tier + override-record prerequisites as #2).

---

## Track D — envelope-stack enablement (TODO.md §4.7)

Four new spec items that make the DocuSign-class signature workflow composable. Runs in parallel with late Track C (depends on #20 and #30).

### D1. #58 Envelope (instance-level) status extension `[Imp 7 / Cx 3 / Debt 5]`

Extend `CaseInstance.status` with first-class `declined | voided | expired` discriminators, each carrying required metadata:
- `declined`: `declineReason: String`, `declinedBy: ActorRef`, `declinedAt: RFC3339`.
- `voided`: `voidedBy: ActorRef`, `voidedAt: RFC3339`, optional `voidReason`.
- `expired`: `expiredAt: RFC3339`, `expirationBasis: { deadlineId, originalDeadline }`.

Companion to #30: #30 handles task-level cancellation, #58 handles instance-level envelope status.

Work:
- Spec: Runtime §3.4 Status Transitions extension.
- Schema: `schemas/companions/wos-case-instance.schema.json` discriminated-union expansion.
- Rust model: `wos-core::instance::InstanceStatus` gets the three new variants with inline metadata.
- Runtime: `WosRuntime` handles `decline` / `void` / `expire` events by transitioning to the corresponding terminal status.
- Server wiring: handlers `POST /api/instances/:id/decline`, `/void`, `/expire`.
- Conformance fixture: envelope-decline-reroute exercising the decline path.

### D2. #59 CloudEvent envelope-flow type catalog `[Imp 6 / Cx 3 / Debt 4]`

Normative event-type catalog in `specs/profiles/integration.md` for cross-system envelope coordination. Canonical types:

- `envelopeCreated`, `signerInvited`, `signerAuthenticated`, `signerSigned`, `signerDeclined`
- `envelopeCompleted`, `envelopeVoided`, `envelopeExpired`
- `reminderDue`, `ledgerCheckpointAttached` (latter is the Respondent Ledger integration point)

Work:
- Spec: new Integration Profile §12 "Envelope Event Type Catalog" appendix.
- Each event type gets a canonical `data` shape (reference to the `CaseInstance`, `respondentLedgerRef` where applicable, ISO 8601 timestamp).
- **Depends on #20** for the internal event vocabulary this mirrors outward.

### D3. #60 Envelope reference fixtures `[Imp 5 / Cx 3 / Debt 3]`

Three to five canonical kernel documents under `fixtures/kernel/` that every WOS signature-stack integrator can start from:

- `envelope-2signer-sequential.json` — intake → signer-1 task → signer-2 task → completed.
- `envelope-parallel-witness.json` — primary signer + witness task in parallel region; both must complete.
- `envelope-decline-reroute.json` — signer-1 declines → reroute to alternate signer → retry.
- `envelope-with-approver.json` — pre-sign approver task gates the sign phase.
- `envelope-reminder-expire.json` — reminders at T-7 and T-3, expiry at T=30 days.

Plus matching conformance fixtures under `fixtures/conformance/` exercising full lifecycles (create → invite → sign → complete; create → invite → decline → void; create → reminder fires → expire).

**Depends on #20 (typed events) and #30 (task-level decline).** Fixture-only work but critical for lock-in.

### D4. #61 Separation-of-duties conformance fixture batch `[Imp 5 / Cx 2 / Debt 3]`

Two to three fixtures exercising the `AccessControl` seam's SoD rejection paths:

1. Agent attempts to review its own output → rejected.
2. Delegated human attempts to re-review an artifact they originally authored → rejected.
3. Separation-of-duties bypass via `OverrideRecord` with authority → recorded as provenance.

**Depends on #23 (OverrideRecord)** for the authority-bypass fixture.

Pairs with Track B2 (`RoleBasedAccessControl`) — fixtures assert the enforcement; without enforcement wired, fixtures fail (which is the spec-conformance signal).

---

## Track E — spec §4.2 (next-batch) + §4.3 (cheap batch) + §4.4 (behavioral backlog)

Track E items are independent of the envelope-stack critical path. Ship when there's slack, or fold into the sprint that naturally touches the relevant code.

### E1. §4.2 items (next-batch — after §4.1 lands)

- **#22a ProvenanceKind tier-typing** `[4/4/3]` — split the 93-variant enum into tier-typed records.
- **#46 Schema-prose enum alignment batch** `[4/3/3]` — close `CaseRelationship.type`, `HoldPolicy.holdType`, `AppealMechanism.reviewerConstraint`/`continuationScope` to enums; add FEL citation to `DelegationScope.conditions`; ISO 8601 duration patterns; Drift Monitor AlertThreshold prose table.
- **#21 Extension registry (seams-only MVP)** `[5/4/3]` — catalog the six kernel seams + Trellis custody shape with lifecycle + composition semantics.
- **#29a Milestone spec-lag closure** `[5/2/5]` — add `triggerMode: "writeSettled"` to Milestone schema + Kernel §4.13 prose.
- **#37 Drift Monitor demotion policy binding** `[6/3/5]` — `alertThresholds[].policyRef` binding to `DemotionRule`.
- **#39 ContinuationPolicy normative linkage** `[4/1/3]` — `continuationPolicyRef` on `AppealMechanism`; schemas and $defs already exist, work is one field plus resolution prose.

### E2. §4.3 cheap batch (ship together in one sprint — parallelizable)

- **#34 `x-lm.critical` enforcement gate** `[6/1/2]` — CI rule.
- **#57 Assurance schema `x-lm.critical` coverage** `[3/1/2]` — annotate key nodes.
- **#13 Verifiability test principle** `[4/1/1]` — Kernel §1.2 design-goal bullet + cross-refs.
- **#12 Capability preconditions** `[6/3/4]` — `preconditions` array on agent capabilities; FEL evaluated before invocation.
- **#56 Runtime §2 isolation-invariant lint rule** `[5/2/2]` — static AST lint for `setData` → guard dependency cycles in `continuous`-mode.
- **#42 Autonomy-lifecycle conformance fixture batch** `[5/2/2]` — two fixtures.

### E3. §4.4 behavioral backlog (after §4.1/§4.3 stabilise)

- **#26a `AccessControl.canRead` enforcement semantics** `[6/3/4]` — specify behavior on `canRead → false` (redact / null / error / skip); **prerequisite to #26b**.
- **#26b `caseFieldPolicy` schema** `[6/6/4]` — per-field read/write scopes by actor role.
- **#36 Equity RemediationTrigger expression language** `[6/4/4]` — **prerequisite to #35**.
- **#35 Equity Config enforcement semantics** `[7/5/4]` — processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` per `ReportingSchedule`; define "suspended workflow" behaviorally.
- **#24b + #25 rule-firing trace + defeasibility** `[7/6/4 + 6/7/6]` — Reasoning Tier ordered rule list + Catala-style default logic.
- **#43 Assurance × impact-level composition rule** `[6/5/4]` — envelope-stack signature-class binding home; resolves Open Q15.
- **#38 Assertion Library cross-document reference protocol** `[5/3/3]` — `assertionId` on `PipelineStage.assertions[]`; enables Track B4.
- **#40 Task SLA authoring surface** `[6/5/4]` — `slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`; envelope reminders + expirations.
- **#30 WS-HumanTask lifecycle completion** `[5/5/2]` — task-level `Suspended` / `Cancelled` / `Return`; companion to #58.
- **#27 Cancellation regions** `[4/6/3]` — YAWL-style named regions.
- **#28 Claim-check artifact references** `[4/4/2]` — typed `ExternalArtifactRef` with integrity-check at retrieval.
- **#29b Milestone reactive transition firing** `[6/5/2]` — `MilestoneFired` event or `$milestone.*` FEL boolean.
- **#3 Policy-based migration routing** `[5/6/2]` — `migrationPolicy` enum; tenant-scope sub-question blocks multi-tenant.

### E4. §4.5 structural merges

- Assertion Library → Workflow Governance (absorb as "Named Assertions").
- Verification Report → Advanced Governance (absorb as "Output Artifacts").
- Due Process Config partial merge → Workflow Governance (pending #45 step 0).

### E5. §4.6 engineering hygiene (deprioritised)

- **#22 Crate split along tier boundaries** — split wos-core; split runtime.rs along action-kind dispatch; add CI fence.
- **#45 Sidecar normative-contract audit** — retrofit all sidecars against CONVENTIONS.md.

---

## Track F — long tail (§5 audit/evidence, §6 regulatory, §7 interop)

- **§5 #48 Merkle provenance chains** `[6/6/6]` — cryptographic hash-chaining for tamper-evident logs; attaches via Assurance `provenanceLayer` seam. PROV-O / XES / OCEL exports shipped without hash-chain hooks — retrofitting means versioning three export surfaces simultaneously. Debt compounds per consumer.
- **§5 #52 Simulation trace format** `[4/3/2]` — normative replay semantics; event log format is XES (already shipped); remaining work is the normative replay contract + conformance fixtures.
- **§6 Regulatory alignment** — external-deadline-driven; benefits from ontology landing first (which blocks on §2 ontology-spec draft, currently not started).
- **§7 Interop + speculative research** — engine adapters (Camunda 8, Temporal, AWS Step Functions) held until a first-commercial-deployment-demands trigger fires.

---

## Phased sequence

Interleaves the tracks by dependency. Each phase is a work-chunk that can be executed by a small team in parallel when the items don't depend on each other.

### Phase 0 — unblock (1 day)

Clear the prereqs that gate everything else.

- **C1 DRAFTS triage** — unblocks C6 (#20).
- **A3 Legal-sufficiency disclosure** — independent quick win.
- **A6 JSON-LD context endpoint** — independent quick win.

### Phase 1 — wire the unwired seams (1 day)

- **A1 `ProvenanceSigner` seam** — `NoopSigner` + config + injection.
- **A2 `ReportRenderer` seam** — `JsonReportRenderer` + config + injection.
- **A5 Chain-integrity verify endpoint** — wrap existing helper.

Unblocks Phase 3's `/explain` endpoint and Phase 4 attestation work.

### Phase 2 — tighten the stubbed seams (~2.5 days)

- **B1 `PolicyLayeredValidator`** with §15.7 ledger-gating policy layer.
- **B2 `RoleBasedAccessControl`** with separation-of-duties enforcement.
- **B3 `IntegrationDispatchService` + correlation tokens** — moved up from Phase 6 per the plan's own D=5 compounding-debt rule. `ExternalService::invoke` signature change is breaking for every external adapter; land it before consumers ship more adapters against the current shape. Scaffolded binding-kind dispatchers can remain stubs; the priority is locking the trait signature.

Parallel-safe. Stops the three compounding "permissive behaviour shipped" / "stale trait signature shipped" debts in one phase.

### Phase 3 — facts / override / notice substrates (2–3 weeks, spec-led, parallelism-dependent)

_Estimate assumes 2–3 engineers running C2/C3/C4/C6 concurrently and C5 picking up the moment its prerequisites land. With a single engineer this phase is ~5 weeks (Cx sum: #24a=4, #23=2, NoticeTemplate=2, #2=7, #20=7, #31=3 ≈ 25 engineer-days). #2 has a hard serial dependency on C2+C3+C4; plan accordingly._

- **C2 #24a** Mandatory Facts-Tier input snapshot → fixture retrofit.
- **C3 #23** OverrideRecord schema → unblocks D4 and the override fixture.
- **C4 NoticeTemplate reconciliation** → unblocks #2.
- **C5 #2** Deterministic adverse-decision notice (dual-form) → blocks Phase 4's explanation endpoint on real content.
- **C6 #20** Typed event meta-vocabulary (parallel with C2–C5; depends only on C1) → blocks D2 + D3.
- **C8 Runtime §9.1 explanation algorithm** (shares #24a + #23 prerequisites with C5; can run parallel with C5) → unblocks A4's real algorithm. Phase 4's A4 scaffold ships regardless; C8 swaps the internals post-Phase-4 when it lands.

### Phase 4 — endpoints that ride on the wired seams (2–3 days)

- **A4 `/instances/:id/explain` handler** — ships with a purpose-built §9.1-shape scaffold service; swaps internals to C8's real algorithm when landed. Uses A2's renderer.
- **A7 Event idempotency on `POST /events`**.
- **A8 Policy-parameters as-of resolution**.
- **A9 Subject continuity-hash validation**.

### Phase 5 — envelope-stack enablement (1 week, spec + fixture-heavy)

- **D1 #58** Envelope instance-level status extension.
- **D2 #59** CloudEvent envelope-flow type catalog (depends on C6).
- **D3 #60** Envelope reference fixtures (depends on C6 + Track E3 #30).
- **D4 #61** Separation-of-duties conformance fixtures (depends on C3).

Parallel-safe with Phase 6.

### Phase 6 — integration correctness + auxiliary endpoints (~3 days)

B3 moved up to Phase 2 to avoid breaking-change exposure. Remaining Phase-6 items:

- **B4 Pipeline validation endpoint** — scaffolded now, swapped to real eval when Track E3 #38 lands.
- **B5 Hold CRUD** (Option B — direct storage writes path; if Option A is chosen, defer this item until Phase 8 after C6 #20 lands).
- **B6 Calibration expiry enforcement**.
- **B7 Migration endpoint**.
- **B8 Drift write-side endpoint**.
- **C7 #31 Jurisdiction-aware calendar**.

### Phase 7 — behavioral backlog cheap batch (1 sprint, parallelizable)

Track E2 — all six items ship together.

### Phase 8 — behavioral backlog next-batch (ongoing)

Track E1 — six items, ~4–6 weeks total.

### Phase 9 — behavioral backlog depth (ongoing, prioritised by envelope-stack demand signal)

Track E3 — thirteen items. Promote #43 + #40 + #30 + #38 ahead of the others for envelope-stack composition.

### Phase 10 — structural merges + hygiene (spare-capacity)

Track E4 + E5. Schedule when code in the merged sidecars is being actively touched for another reason.

### Phase 11 — long tail

Track F — Merkle chains (§5 #48) is the next compounding-debt item beyond the Phase 0–6 scope; schedule once Phase 6 wraps.

### Total estimate

Phases 0–6 strictly serial, single-engineer: ~**8–10 weeks** (Phase 0: 1d · Phase 1: ~1.5d · Phase 2: ~2.5d · Phase 3: ~5 weeks · Phase 4: ~3d · Phase 5: ~1 week · Phase 6: ~3d). Phase 3 is the bottleneck — it's specifically 5 weeks if a single engineer handles #24a + #23 + NoticeTemplate + #2 + #20 + #31 + C8 serially.

With **2–3 engineers** running Phase 3 spec work in parallel with Phase 4–6 server work (most server items don't block on spec except A4/explanation scaffold), overall elapsed time drops to **~3–4 weeks**. Phase 3 itself drops to 2–3 weeks with 3 engineers.

Phases 7–11 run indefinitely as backlog flows; the envelope-stack is usable end-to-end after Phase 6.

---

## Verification

### Per-phase gates

Each phase must pass before the next starts:

| Phase | Gate |
|---|---|
| 0 | `cargo build -p wos-server` clean · `DRAFTS/` resolved · disclosure block present in all three export formats (unit tests) |
| 1 | New seams visible in `AppRuntime::build` · `WOS_SIGNER=noop` + `WOS_RENDERER=json` defaults boot cleanly · `GET /provenance/verify` returns `{valid: true}` on seeded fixture |
| 2 | `cargo test -p wos-server` green · new integration test `tests/policy_validator.rs` rejects rights-impacting submit without `respondentLedgerRef` · new test `tests/separation_of_duties.rs` rejects self-review agent transition |
| 3 | `cargo test -p wos-lint` + `cargo test -p wos-conformance` green · fixtures retrofit verified (every determination transition has `inputs`) · three determinism fixtures for #2 produce identical outputs bit-for-bit |
| 4 | `/explain` returns a payload whose shape matches Runtime §9.1 · dupe `idempotencyToken` on events → single drain (asserted) · as-of policy resolution unit test green |
| 5 | All five envelope-reference fixtures parse + lint clean · envelope-decline-reroute conformance fixture runs end-to-end · `#61` SoD fixtures run and (correctly) fail against B2's enforcement |
| 6 | `ExternalService::invoke` signature updated in wos-core with correlation token · existing integration-profile fixtures still parse · migration endpoint round-trips `WosRuntime::migrate` · `jurisdiction` field on case-state drives correct calendar selection |
| 7–9 | Backlog items verified item-by-item per their own acceptance criteria |

### End-to-end envelope flow

After Phase 6, this sequence must work against a running `wos-server` with default seam config:

```bash
# Create envelope from reference fixture
curl -XPOST $API/instances -d @fixtures/kernel/envelope-2signer-sequential.json
# → instance id

# Invite signer-1
curl -XPOST $API/instances/$ID/events -d '{"event":"signerInvited","data":{"signerId":"s1"}}'

# Signer-1 submits (with Formspec response carrying respondentLedgerRef)
curl -XPOST $API/tasks/$TID/response -d '{"response":{"status":"completed",...,"respondentLedgerRef":"..."}}'
# → Completed

# Audit trail: chain valid + disclosure block present
curl $API/instances/$ID/provenance/verify
curl $API/instances/$ID/provenance/export?format=prov-o | jq .wosDisclosure

# /explain serves Runtime §9.1-shaped output
curl $API/instances/$ID/explain | jq '.explanationLevel, .reasoning, .rulesApplied'
```

### Conformance regression guard

- `cargo test -p wos-conformance` runs on every commit; 146 existing fixtures stay green.
- New fixtures from D3 + D4 + C5 determinism + #61 SoD land in the same harness; count goes to ~160.
- CI rule: no spec PR may merge if conformance fixtures drop below the previous count.

### PARITY.md sync

After each server-facing phase, update `crates/wos-server/PARITY.md`:
- Rows transitioning `stub` → `full` or `partial` → `full` get regraded.
- New phases that retire a gap row (e.g. Phase 1 retires the two unwired-seam rows at ROI 25) trim the ranking.
- Validation pass runs again after Phase 6 — three agents (citations, server surface, status grades) re-verify.

---

## Critical files

### Spec side — to be modified

- `specs/kernel/spec.md` — #24a (§8.2 Facts-Tier prose), #20 (§4.x event taxonomy).
- `specs/governance/workflow-governance.md` — #2 (§3.2 deterministic notice), #23 (§7.3 override), #43 (signature-class × assurance binding).
- `specs/governance/due-process-config.md` — NoticeTemplate reconciliation.
- `specs/companions/runtime.md` — #9 algorithm alignment with #2, #58 status extension at §3.4.
- `specs/profiles/integration.md` — #59 new §12 event-type catalog.
- `specs/ai/ai-integration.md` — #43 assurance-level × impact composition.
- `specs/sidecars/business-calendar.md` — #31 jurisdiction-aware selection.

### Spec side — schemas to be modified

- `schemas/kernel/wos-kernel.schema.json` — #20 typed event, #24a `ProvenanceRecord.inputs` tightening.
- `schemas/governance/wos-workflow-governance.schema.json` — #23 `OverrideRecord` `$def`, #46 enum alignments.
- `schemas/governance/wos-due-process.schema.json` — NoticeTemplate removal.
- `schemas/companions/wos-case-instance.schema.json` — #58 envelope-status discriminator.
- `schemas/sidecars/wos-business-calendar.schema.json` — #31 `calendarSelection.fromCaseField`.

### Spec side — new fixtures

- `fixtures/kernel/envelope-2signer-sequential.json` · `envelope-parallel-witness.json` · `envelope-decline-reroute.json` · `envelope-with-approver.json` · `envelope-reminder-expire.json` (D3).
- `fixtures/conformance/envelope-*.json` — matching end-to-end fixtures (D3).
- `fixtures/conformance/sod-*.json` — three SoD fixtures (D4).
- `fixtures/conformance/notice-determinism-*.json` — three fixtures for #2 determinism.
- `fixtures/conformance/jurisdiction-calendar.json` — multi-jurisdiction deadline (C7).

### Server side — new files

- `crates/wos-server/src/runtime/signer.rs` — `NoopSigner` + trait wiring (A1).
- `crates/wos-server/src/runtime/renderer.rs` — `JsonReportRenderer` + trait wiring (A2).
- `crates/wos-server/src/services/explanation_service.rs` — §9.1-shape scaffold service backing the `/explain` handler (A4); internals swap to the real C8 algorithm when it lands.
- `crates/wos-server/src/runtime/submit_policy.rs` (if Path 1 is chosen for B1) — `LedgerGatingSubmitPolicy` default-on policy.
- `crates/wos-server/src/http/policy.rs` — as-of resolution (A8).
- `crates/wos-server/src/http/holds.rs` — hold CRUD (B5).
- `crates/wos-server/migrations/0003_drift_reports.sql` — drift-report storage (B8).

### Server side — to be modified

- `crates/wos-server/src/runtime/mod.rs::AppRuntime::build` — seam injection for A1/A2/B1/B2/B3.
- `crates/wos-server/src/runtime/validator.rs` — `PolicyLayeredValidator` replacement (B1).
- `crates/wos-server/src/runtime/access.rs` — `RoleBasedAccessControl` replacement (B2).
- `crates/wos-server/src/runtime/service.rs` — `IntegrationDispatchService` replacement (B3).
- `crates/wos-server/src/services/semantic_service.rs` — legal-sufficiency disclosure (A3).
- `crates/wos-server/src/services/provenance_service.rs` — `verify_chain` exposure (A5).
- `crates/wos-server/src/services/timer_task.rs` — calibration expiry (B6).
- `crates/wos-server/src/http/instances.rs` — `/explain` handler (A4), event idempotency (A7), `/migrate` (B7), `/decline`+`/void`+`/expire` (D1).
- `crates/wos-server/src/http/agents.rs` — drift write-side (B8).
- `crates/wos-server/src/http/governance.rs` — `/validate-pipeline` scaffold (B4).
- `crates/wos-server/src/http/assurance.rs` — continuity-hash extension (A9).
- `crates/wos-server/src/config.rs` — `WOS_SIGNER`, `WOS_RENDERER` env + enum variants.

### wos-runtime / wos-core — minimal changes

- `wos-core::traits::ExternalService` gains a default method `invoke_with_correlation(service_ref, input, idempotency_key, correlation_token)` delegating to `invoke` (B3). **Non-breaking** — existing adapters keep compiling against the current `invoke` signature; new integration handlers call `invoke_with_correlation`.
- `wos-core::traits::ContractValidator` — optionally extended with `validate_in_context(contract_ref, data, impact_level, instance_id)` default method (B1 alternative path) if the submit-path `SubmitPolicy` approach isn't adopted. Non-breaking either way.
- `wos-runtime::WosRuntime` (`crates/wos-runtime/src/runtime.rs`) — new `Box<dyn ProvenanceSigner>` + `Box<dyn ReportRenderer>` fields on the struct (~line 347–383), new builder methods `with_provenance_signer` / `with_report_renderer` (mirroring `with_companion_policy` at line 386). If adopting `SubmitPolicy` for B1, also add `with_submit_policy` builder. Thread the signer into every provenance-emit site.

### Coordination — PARITY.md + TODO.md

Both documents get updated in-place as phases land. Existing sections stay; status columns and ranking table rows are mutated. No new top-level sections needed in either document.

---

## Success criteria

**End of Phase 2:** no stubbed seam remains. Every `cargo test -p wos-server` still green. PARITY.md DI seam status table shows all nine seams either wired-real or wired-real-with-policy-source-pluggable.

**End of Phase 4:** `/instances/:id/explain` serves Runtime §9.1-shaped output (real algorithm once #2 lands; scaffold before). Chain-integrity verify returns `valid: true` on seeded fixtures. Legal-sufficiency disclosure present on every export format.

**End of Phase 5:** envelope-reference fixtures all parse, lint clean, and run end-to-end through conformance. Ecosystem integrators have canonical patterns to copy.

**End of Phase 6:** envelope stack is composable. A third party can plug Formspec + an identity adapter + a PDF/email layer and sign a 2-signer document end-to-end with auditable provenance, §15.7-gated rights-impacting submits, and an attestation path wired through the injected `ProvenanceSigner` seam. The default `NoopSigner` makes the path end-to-end testable but signatures are empty; **externally-verifiable signing** requires either the feature-flagged `Ed25519FileKeySigner` reference impl or a consumer-injected HSM / cloud KMS / Respondent Ledger adapter — all of which slot into the same seam without further plumbing.

**Downstream (Phases 7–11):** TODO.md §4.2, §4.3, §4.4 drain; engineering hygiene absorbed as code is touched; audit/evidence products (§5) deliver Merkle chains when demand signal appears.

---





















