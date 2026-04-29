# Plan: Remainder of WOS spec + wos-server work (DI-seam framing)

## Context

This plan sequences the work still open in both **TODO.md** (spec-side) and **crates/wos-server/PARITY.md** (server-side) after the 11-phase parity push, three validation passes, and the DI-seam reframe landed on branch `claude/wos-spec-backend-y17wJ`. Last commit on branch: `93f30b1`.

**What's already done (not re-litigated here):**

- `wos-server` ships every layer of the original parity plan: kernel/instance/task endpoints, provenance hash chain + PROV-O/XES/OCEL export, governance L1 (reads + delegations CRUD + deontic violation listing), agent registry + AI lifecycle L2, advanced L3 stubs (SMT, equity, zones), assurance layer, integration-profile inbound + invoke stub, business-calendar + notification sidecars, lint + conformance REST.
- Three audit passes completed: spec citations (6 corrections applied), server surface (2 discrepancies fixed), status grades + ROI math (4 regrades + 4 rescores + table resort).
- `TODO.md` and `PARITY.md` synced around the **DI-seam framing**: every envelope-stack concern reduces to wiring a host-interface trait. Attestation = `ProvenanceSigner` seam. Explanation rendering = `ReportRenderer` seam. Separation of duties = `AccessControl` seam composition. Integration dispatch = `ExternalService` seam composition. Ledger gating (§15.7) = `ContractValidator` seam composition.
- **Two DI review passes** (external-owner boundaries + down-the-stack placement) identified gaps in both the *shape* of existing seams and the *count* of traits. Summary: the "nine traits" framing was an undercount (`Clock` is a tenth, in `wos-runtime`) and several Temporal/Formspec/Ledger/IdP responsibilities have no seam at all today. Both reviews' findings are integrated as Tracks G and H below. Target architecture for the engine-adapter side is the reference doc at `thoughts/examples/temporal-reference-implementation.md`.

**The framing consequence:** the remaining work splits cleanly into (a) wire two unwired seams + tighten three stubbed ones; (b) ship the spec items that feed the seams (deterministic explanation algorithm, typed event vocabulary, envelope status extensions); (c) fixtures that lock patterns so integrators don't diverge; (d) behavioral backlog + engineering hygiene that don't depend on any seam.

**Intended outcome:** a reference runtime + spec pair that a third party can compose with (a) Formspec + its Coprocessor bridge, (b) a Formspec Respondent Ledger adapter, (c) an external IdP (OIDC/SAML), (d) a workflow engine of their choosing (Temporal, Camunda, AWS Step Functions, or the bundled in-process runtime), and (e) a PDF/email layer — to ship a DocuSign-competitive e-signature product without forking either spec or server. Every DI seam is either wired or has a no-op default that ships spec-correct response shapes; every envelope-flow pattern has a canonical fixture; every normative MUST in the spec has either enforcement or a failing conformance test marking the gap.

---


## Remaining work at a glance

**Server side (PARITY.md top-ROI rows still open):** 6 P0/high-priority items — wire `ProvenanceSigner` seam (ROI 25), wire `ReportRenderer` seam (25), legal-sufficiency disclosure on exports (20), `PolicyLayeredValidator` with §15.7 ledger-gating (12.5), `RoleBasedAccessControl` separation-of-duties (12.5), `/explain` handler (12.5, rides on ReportRenderer + spec #2). Plus ~10 medium-ROI items (event idempotency, policy as-of resolution, chain-integrity verify endpoint, subject continuity-hash, hold CRUD, `IntegrationDispatchService` with correlation tokens, pipeline validation endpoint, calibration expiry, migration endpoint, drift write-side endpoint).

**Spec side (TODO.md open items):** 7 items in §4.1 critical path (DRAFTS triage, #24a facts-tier snapshot, #23 OverrideRecord, NoticeTemplate reconciliation, #2 adverse-decision notice, #20 typed event vocabulary, #31 jurisdiction-aware calendar) + **1 proposed addition C8** (Runtime §9.1 explanation algorithm — see Track C); 6 in §4.2 next-batch; 6 in §4.3 cheap batch (parallelizable); 13 in §4.4 behavioral backlog; 4 in §4.7 envelope-stack enablement (new items #58–#61); 3 structural merges in §4.5; 2 hygiene in §4.6; §5/§6/§7 downstream.

**Completed and excluded from this plan:** the 11-phase parity implementation, three validation audits, DI-seam sync, all existing PARITY rows marked `full`.

**Explicit out-of-scope for this plan:** Formspec Respondent Ledger cryptographic primitives (upstream, plug via `ProvenanceSigner`); real Z3 solver / real SHACL engine / real drift detector computation / in-server SPARQL triplestore (consumer-injected when demanded); multi-step sessions, agent circuit breakers, counterfactual explanation (deferred for consumer-demand signal); studio UI changes (separate effort); Correspondence-vs-Notification merger (spec editorial, not implementation).

---

## Guiding principles

1. **DI seams are the contract.** Runtime §12's host-interface traits are the composition surface — ten in scope today (nine in `wos-core::traits` plus `Clock` in `wos-runtime::runtime`; hoisting `Clock` to `wos-core::traits` for consistency is Track H2's first item), with ~8 more surfaced by the external-owner review (`TaskStore`, `NotificationService`, `TimerService`, `Coprocessor`, `IdentityResolver`, `DirectoryService`, `PolicyEngine`, `VisibilityService` — all landed as Track G). Server's job is to accept consumer-injected implementations, provide sensible no-op defaults, and enforce that seams are wired when the spec (e.g. §15.7) demands it. Every "build X primitive" temptation that's not a seam gets refused — it goes out to consumers via a trait.

2. **Stubs ship spec-correct shapes.** Every stub response already returns the envelope the spec calls for; swapping a stub for a real impl is transparent to consumers. Never replace a stub with a 501 — keep the shape and let the stub document its noop status in the payload.

3. **Spec work feeds server work.** #2 (deterministic explanation algorithm) unblocks the `/explain` handler. #20 (typed event vocabulary) unblocks envelope-flow fixtures. #23 (OverrideRecord) unblocks SoD conformance fixtures. #30 + #58 (task + instance envelope lifecycle) unblocks decline flows. Sequence the server work to ride behind the spec work, not to block on it — ship the seam first with a `Noop` default, then swap to real algorithm/data when spec lands.

4. **Fixtures lock the patterns.** Every reference composition (2-signer sequential, parallel-witness, etc.) ships as a canonical fixture. Without that, integrators diverge and the ecosystem fragments. Fixture work is load-bearing, not decorative.

5. **Compounding debt first.** Within the ranked list, every row flagged D=5 goes before every row flagged D≤3, regardless of priority. Breaking-change exposure compounds per consumer per week.

6. **Respect external-owner boundaries.** Four upstream/integration systems legitimately own responsibilities that wos-server currently holds or would hold by default: (a) **Formspec** owns form rendering, response validation, and response-to-ledger packaging (seams: `ContractValidator`, new `Coprocessor`); (b) the **Formspec Respondent Ledger** owns cryptographic proof primitives — Merkle accumulation, inclusion proofs, anchoring (seam: `ProvenanceSigner` widened to typed `LedgerAttachment`); (c) a **workflow engine** — Temporal reference, others pluggable — owns durable timers, event history, crash recovery, signal queuing, activity retry/idempotency, visibility (seams: `TimerService`, `VisibilityService`, additive event-sourced `InstanceStore` variant; reference shape in `thoughts/examples/temporal-reference-implementation.md`); (d) an **external IdP** (OIDC/SAML/managed) owns credential storage + authentication (seams: `AuthVerifier` narrowed from `AuthProvider`, `IdentityResolver`, `DirectoryService`). Seams must be shaped to *let these owners own their responsibilities*, not to re-implement them behind a thin facade.

7. **Trait-shape mistakes compound faster than feature gaps.** Widening a trait later is a breaking change for every consumer that wired the current shape; shipping a missing default is additive. When the review finds a shape-level mistake (return type too narrow, signature forecloses a real impl), it ranks above feature work in the phase sequence — see Phase 0.5 below.

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

**Placement correction:** the `JsonReportRenderer` default impl belongs in **wos-core** next to `DefaultRuntime` (wos-core already bundles the other seven trait defaults — placing this one in wos-server splits the default surface). Server owns only (a) config + injection, (b) the feature-gated `HtmlReportRenderer` skeleton (HTML templating isn't wos-core's job).

- **wos-core extension** (`crates/wos-core/src/traits/mod.rs`) — `impl ReportRenderer for DefaultRuntime` with passthrough JSON behaviour; export `JsonReportRenderer` as a standalone zero-state struct for consumers who don't use `DefaultRuntime`.
- **wos-server new file** `crates/wos-server/src/runtime/renderer.rs` — thin re-export of `wos_core::JsonReportRenderer`, plus feature-gated `HtmlReportRenderer` skeleton for week 2.
- **Config** `ServerConfig::renderer_kind` + env `WOS_RENDERER=json|html`.
- **`AppRuntime::build`** — inject renderer.
- **Unblocks A4** (`/explain` handler).

### A3. Legal-sufficiency disclosure on exports (~30 min)

**Placement correction:** the `wosDisclosure` block is a property of each export *format*, not of the server. Canonical home is **wos-export** (which already owns `prov_o.rs` / `xes.rs` / `ocel.rs`). Server becomes a passthrough caller.

- **wos-export extension** (`crates/wos-export/src/{prov_o,xes,ocel}.rs`) — each export function takes an optional `&Disclosure` argument and emits the `wosDisclosure` block inline. Define a single `Disclosure { conditions: Vec<String>, spec_section: String, implementation_profile: String }` struct in `wos-export::disclosure`.
- **wos-server edit** `crates/wos-server/src/services/semantic_service.rs` — construct the `Disclosure` value from `ServerConfig` + attestation state, pass it into the wos-export call. No payload-wrapping in server.
- New unit tests in **wos-export** (not wos-server) asserting the disclosure block is present and valid in every format.

### A4. `/instances/:id/explain` handler (~4 hr — handler + scaffold service; blocked on A2 + explanation-algorithm decision)

**Scope correction vs prior draft:** Spec TODO #2 is `Governance §3.2 — adverse-decision notice`, which produces machine-readable + human-prose **notices**. Runtime §9.1 (`specs/companions/runtime.md`) is a **separate** deterministic algorithm for **explanation assembly** — no TODO item currently owns its implementation. The `/explain` endpoint depends on §9.1, not #2 directly; the two algorithms share skeleton but have different output shapes. See new Track C8 below.

**Placement correction:** the explanation algorithm itself (§9.1) is runtime semantics — canonical home is **wos-runtime**, exposed as `wos_runtime::explain(&CaseInstance, &ProvenanceLog) -> ExplanationDoc`. The scaffold service inside server shrinks to an HTTP wrapper.

- New handler in `crates/wos-server/src/http/instances.rs` (~50 lines, delegation pattern mirrors `http/applicant.rs`).
- **wos-runtime** scaffold function (~150–250 lines) — implements a minimal §9.1-shape payload (`explanationLevel`, `reasoning`, `rulesApplied`, `authorityRanking`, `counterfactuals`) populated from currently-available provenance. Response payload carries `algorithmId: "wos-runtime-scaffold-0.1"` so consumers know the output is pre-§9.1.
- Server's `crates/wos-server/src/services/explanation_service.rs` becomes a thin wrapper that calls `wos_runtime::explain` and passes the result through `ReportRenderer`.
- When the real §9.1 algorithm lands (Track C8), the scaffold's internals swap inside wos-runtime without changing the wire shape. The `applicant_service::determination` view stays as-is — it's an applicant-facing projection, not §9.1's spec shape, so it's NOT the right scaffold source.

### A5. Chain-integrity verify endpoint (~1 hr)

**Placement correction:** the verification algorithm (hash-chain walk + signature check over a `ProvenanceLog`) is a ledger-domain concern that every consumer of wos-core should have access to, not just the server. Canonical home is **wos-core::provenance** (new `verify_chain(&ProvenanceLog, &dyn LedgerVerifier) -> ChainVerifyReport`). Server keeps only the HTTP wrapper + the service-level call site.

- **wos-core new function** `crates/wos-core/src/provenance.rs::verify_chain` — parameterised on the `LedgerVerifier` trait added in Track G6 so a real Ledger impl can supply Merkle-path checking, and the in-process default does hash-walk only.
- **wos-server edit** `crates/wos-server/src/services/provenance_service.rs` — replace the existing ~11-line helper with a one-line delegation to `wos_core::provenance::verify_chain`.
- **wos-server new handler** `GET /api/instances/:id/provenance/verify` in `crates/wos-server/src/http/instances.rs`.
- Response: `{ valid: bool, firstBrokenSeq: Option<i64>, reason: Option<String>, algorithmId: String }` — note the new `algorithmId` field per the Ledger-redundancy ADR (Track G6); makes the verify result explicit about which chain was checked (WOS hash-chain only vs WOS + Ledger inclusion proofs).

### A6. JSON-LD context endpoint (~30 min)

**Placement correction:** the JSON-LD context content is defined by the vocabulary of each export format; wos-export already embeds its contexts. Server serves bytes it gets from wos-export — no static-file duplication.

- **wos-export extension** — export a `context(format: ExportFormat) -> &'static str` function returning the canonical JSON-LD context bytes per format (prov-o, xes, ocel, general).
- **wos-server new handler** `GET /api/semantic/jsonld-context?format=prov-o` in `crates/wos-server/src/http/semantic.rs` (new file, or fold into existing integration.rs) — passes the format query param through to `wos_export::context(...)`.

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

**Placement correction:** §15.7 ledger-gating is a **normative MUST** from the WOS spec, applicable to every runtime consumer — not a wos-server policy. The default-on enforcement belongs in **wos-runtime** (either a `LedgerGatingSubmitPolicy` bundled as the default submit-path policy, or the `validate_in_context` default-method override on `DefaultRuntime`'s `ContractValidator` impl). Server's role is to opt in by default and expose config to opt out for non-spec-conforming deployments.

- **wos-runtime default** (Path 1): `crates/wos-runtime/src/submit_policy.rs` — `LedgerGatingSubmitPolicy` default-on; server's `AppRuntime::build` composes it into the runtime unless `WOS_SUBMIT_POLICY=permissive` is set.
- **wos-runtime default** (Path 2): override `ContractValidator::validate_in_context` on a new `DefaultSubmitValidator` wrapper in wos-runtime; server wraps its injected validator.

Either path — enforcement lives in wos-runtime, server composes it in. Unblocks §15.7 conformance for every runtime consumer, not just wos-server.

### B2. `RoleBasedAccessControl` (~half day)

**Placement correction:** the separation-of-duties + delegation-chain + autonomy-cap rules are all normative spec requirements (Gov §7.2, Gov §6, AI §5.3) that every runtime consumer must enforce by default. Canonical home is **wos-runtime** — a `DefaultAccessControl` decorator that wraps any injected `AccessControl` and adds the three normative checks. Server composes it in by default.

Replace `crates/wos-server/src/runtime/access.rs::PermissiveAccessControl`. Enforce (in the wos-runtime decorator, not in server):

- **Separation of duties (Gov §7.2, AI §1.5):** on transitions tagged `review` (or any tag where the kernel declares `reviewRole`), reject when the caller's identity equals the author of the artifact being reviewed. Authorship is read from the latest provenance record touching that artifact.
- **Delegation chain validation (Gov §6):** when the caller is acting under a delegation, verify the delegator → delegate chain is live (not revoked, within `validFrom/validUntil`) and that the scope covers the attempted action.
- **Autonomy cap (AI §5.3, pre-calibration expiry):** for AI-actor-typed callers, reject when declared autonomy level exceeds the workflow's `impactLevel` ceiling per #43 when it lands.

Policy source is pluggable — internal `PolicyEngine` seam (see Track G5) so integrators can swap OPA / Cedar / custom without forking the decorator.

### B3. `IntegrationDispatchService` + correlation tokens (~1 day, **non-breaking**)

Replace `crates/wos-server/src/runtime/service.rs::EchoExternalService`. Read integration bindings from the resolver, dispatch on `IntegrationBindingKind`:

- `RequestResponse` → reqwest POST with the binding's request contract.
- `EventEmit` → publish via Socket.IO + optional webhook per binding config.
- `ArazzoSequence` → sequential multi-step dispatch (parallel is stretch).
- `Tool` → CWL-informed invocation (stub returns declared output shape until a real tool-runner lands).
- `PolicyEngine` → external adapter via `PolicyEngine` trait (XACML / OPA / Cedar); default is `EchoPolicy` with `{ decision: "permit" }`. **The trait itself is defined in Track G5; B3 just wires the binding-kind switch to call it.**

**Placement correction:** the binding-kind switch ("given a binding kind, pick the right `ExternalService` impl") is runtime-level orchestration. Canonical home for the dispatcher is **wos-runtime** — server ships only the concrete transport adapters (reqwest HTTP client, Socket.IO emitter). Today's `crates/wos-server/src/runtime/service.rs::EchoExternalService` becomes a `wos-runtime::BindingKindDispatcher` wrapping a map of `BindingKind → Box<dyn ExternalService>` provided by the server's transport layer.

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

**Placement correction:** the `DriftReport` *shape* belongs in wos-core::model (alongside `ai.rs`, `governance.rs`, `notification_template.rs`). The SQL schema and HTTP endpoint stay in server.

- **wos-core new module** `crates/wos-core/src/model/drift_report.rs` — typed `DriftReport` struct with fields per Drift Monitor §x.
- **wos-server** Add `POST /api/agents/:id/drift` in `crates/wos-server/src/http/agents.rs` for external detectors to write reports.
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
- **§7 Interop + speculative research** — engine adapters (Camunda 8, Temporal, AWS Step Functions) held until a first-commercial-deployment-demands trigger fires. **Track H below upgrades the seam shape in advance of adapter work** so the first real adapter doesn't demand a breaking-change cycle.

---

## Track G — external-owner DI hardening

DI-review findings that widen seams or add missing seams so the four external-owner boundaries (Formspec, Formspec Respondent Ledger, workflow engine, external IdP) can own their responsibilities without wos-server-shaped leakage. Every item here is either a **trait-shape fix** (compounding-debt risk grows per consumer) or a **missing seam** (feature gap with an additive fix). Ordered by compounding-debt urgency.

### G1. Narrow `AuthProvider` to `AuthVerifier`; widen `AuthUser` (~half day, **compounding — D=5**)

`AuthProvider::login(email, password)` at `crates/wos-server/src/auth/mod.rs:77` forecloses every real OIDC/SAML IdP — those IdPs handle the credential check *before* wos-server sees anything; the server only receives tokens. The trait must not require `login`.

- **New trait** `AuthVerifier` in `crates/wos-server/src/auth/mod.rs` with a single required method `verify(&self, access_token: &str) -> AuthResult<AuthContext>`. Optional methods `refresh` and `logout` live on a separate `SessionManager` sub-trait (implemented by `JwtAuth` and `MockAuth`, not required of OIDC impls).
- **Keep `login` as a server-owned extension**, not a trait contract — only the JWT/Mock dev-path impls provide it; handlers that call `login` downcast to `SessionManager` or return a clear "not supported by configured provider" error for pure-verify IdPs.
- **Widen `AuthUser`** from `role: String` to `roles: Vec<String>, groups: Vec<String>, claims: serde_json::Map<String, Value>`. `role` field stays as a computed first-of-roles for backward-compat, deprecated in a comment.
- **Config** `AuthKind::{Jwt, Mock, Oidc}` — OIDC variant carries issuer URL + JWKS URL + audience + claim mapping. Real OIDC impl lands in Track G10 (or is left as a consumer-shipped crate).

**Why now:** every new handler wired against `AuthProvider::login` multiplies the eventual breaking-change cost. The trait is used by `auth/middleware.rs` + ~6 handler files today; delaying this fix past Phase 1 compounds against every new handler added in Phases 2–6.

### G2. Widen `ProvenanceSigner::sign` return to typed `LedgerAttachment` (~half day, **compounding — D=5**)

`ProvenanceSigner::sign(&record) -> Vec<u8>` at `crates/wos-core/src/traits/mod.rs:118-127` is too narrow to carry the output of a real Formspec Respondent Ledger or any Merkle-anchored ledger. A real ledger returns `{ leaf_hash, merkle_path, root, anchor_ref, signed_at, proof_kind }` — a `Vec<u8>` bytes-blob forces either (a) opaque serialisation that no verifier can decode without ledger-specific knowledge baked into every consumer, or (b) storage of structured data out-of-band with the signature as a loose pointer. Both break the seam.

- **New type** `LedgerAttachment` in `crates/wos-core/src/provenance.rs`:
  ```rust
  pub struct LedgerAttachment {
      pub proof_kind: ProofKind,        // enum { Ed25519, MerkleInclusion, AnchoredMerkle, Composite }
      pub leaf_hash: Vec<u8>,            // canonical hash of the record
      pub signature: Option<Vec<u8>>,    // signer-bound byte signature (Ed25519 case)
      pub merkle_path: Option<Vec<[u8; 32]>>,  // inclusion proof, if ledger is Merkle-based
      pub anchor_ref: Option<String>,    // transaction hash / checkpoint URL for anchored ledgers
      pub signed_at: chrono::DateTime<chrono::Utc>,
      pub key_id: Option<String>,
  }
  ```
- **Trait change** `ProvenanceSigner::sign(&record) -> Result<LedgerAttachment, Self::Error>`. Existing `NoopSigner` (Track A1) returns an attachment with `proof_kind: Ed25519`, empty signature, zeroed leaf hash. Real Ledger adapter (upstream, consumer-injected) returns the full attachment.
- **`ProvenanceSigner::verify`** signature updated to accept `&LedgerAttachment` instead of `&[u8]`.
- **Provenance record** gets a new optional field `attestation: Option<LedgerAttachment>`. Exports (PROV-O/XES/OCEL via wos-export) preserve the structure rather than embedding opaque bytes.

**Why now:** Track A1 wires the seam. Landing the typed shape **after** Track A1 ships means every exporter + every downstream consumer of provenance records has to be migrated. Landing it **before** Track A1 means `NoopSigner` is the only existing caller. Do it before A1.

### G3. Add `IdentityResolver` trait in wos-core (~3 hr)

Bridges `AuthContext` (the output of `AuthVerifier`) to `actor_id` (the input to `AccessControl`). Today the mapping is implicit — handlers inline `ctx.user.id` as `actor_id`, which forecloses (a) delegation (`actor_id` may differ from the authenticated user under Gov §6), (b) service-account impersonation, (c) OIDC `sub` that isn't the same as the WOS actor ID.

- **New trait** `IdentityResolver` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait IdentityResolver {
      type Error: std::error::Error;
      fn resolve(
          &self,
          auth_context: &AuthClaims,              // minimal subset: sub, groups, claims
          instance_ctx: Option<&CaseInstance>,    // for delegation-chain resolution
      ) -> Result<ResolvedActor, Self::Error>;
  }
  pub struct ResolvedActor {
      pub actor_id: String,
      pub delegation_chain: Vec<DelegationLink>,  // empty when no delegation active
      pub impersonated: Option<String>,            // Some(original_actor) when impersonating
  }
  ```
- **Default impl** on `DefaultRuntime`: returns `actor_id = auth_context.sub`, empty delegation chain.
- **Server wiring**: handlers call `identity_resolver.resolve(ctx, instance)` before calling into `AccessControl`. Middleware `RequireAuth` extracts `AuthClaims` and attaches it; the resolution happens at handler-dispatch time because it needs the instance.

**Why now:** same logic as G1. Every handler that inlines `ctx.user.id` is debt; centralising via the seam before Phase 2's `RoleBasedAccessControl` work lands is additive, whereas doing it after means retrofitting every call site.

### G4. Widen `ValidationResult.errors` to typed `FieldError[]` (~3 hr, **compounding — D=4**)

`ValidationResult { valid, errors: Vec<String> }` at `crates/wos-core/src/traits/mod.rs:83-89` loses every piece of structure a real Formspec validator produces. Field paths, error codes, i18n keys, parameter substitutions — all erased into prose strings. Consumers (studio, API clients, reviewer dashboard) can't localise, can't highlight fields, can't group.

- **New type** `FieldError` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub struct FieldError {
      pub path: String,                             // JSON Pointer to the field
      pub code: String,                              // stable machine code, e.g. "required", "out-of-range"
      pub message: String,                           // default English prose for debugging
      pub params: serde_json::Map<String, Value>,   // substitution vars for i18n rendering
  }
  ```
- **Trait change** `ValidationResult { valid: bool, errors: Vec<FieldError> }`. The string-based form goes away — not additive because this is still pre-1.0 and the trait has 3 production-grade impls (two in wos-server, `PolicyLayeredValidator` pending from B1). Fix once, before B1 lands.
- **Conversion helper** `FieldError::legacy_string(&self) -> String` for consumers that want flat-prose rendering, lets the studio migrate incrementally.

**Why now:** B1 (Phase 2) is about to land `PolicyLayeredValidator` as a new impl. If it wires against the current `Vec<String>` shape, every B1 check-site has to be rewritten when G4 lands later. Same compounding-debt logic as G2.

### G5. Promote `PolicyEngine` trait to wos-core (~2 hr)

B3 mentions "`PolicyEngine` trait (XACML / OPA / Cedar)" but no such trait exists in the codebase today — it's prose-only. Adding it to wos-core alongside the other host-interface traits gives B2 and B3 a concrete seam to wire.

- **New trait** `PolicyEngine` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait PolicyEngine {
      type Error: std::error::Error;
      fn evaluate(
          &self,
          policy_ref: &str,                        // URL or ID into a policy document
          subject: &PolicyRequest,                  // actor, action, resource, context
      ) -> Result<PolicyDecision, Self::Error>;
  }
  pub struct PolicyRequest { /* actor, action, resource, env */ }
  pub enum PolicyDecision { Permit, Deny { reason: String }, Indeterminate, NotApplicable }
  ```
- **Default impl** on `DefaultRuntime`: returns `Permit` always (matches current permissive behaviour).
- **Named stub** `EchoPolicy` for config `POLICY_ENGINE=echo`, returns `Permit { reason: "echo-stub" }` with provenance so consumers see the decision path in audit trails.
- **Wiring**: B2's `RoleBasedAccessControl` decorator and B3's `PolicyEngine` binding-kind dispatcher both consume `&dyn PolicyEngine` rather than defining their own.

### G6. WOS chain ↔ Ledger chain ADR + `LedgerVerifier` trait (~half day)

The plan assumes WOS's `previous_hash` provenance chain (A5) coexists with Formspec Respondent Ledger's Merkle chain. Today the relationship is undefined. Two questions need answers before A5 ships a verify endpoint:

1. **Redundancy posture.** Is WOS's hash-chain (a) defense-in-depth alongside the Ledger's Merkle chain, or (b) a transitional mechanism retired when a real Ledger is wired? Either answer is defensible; unanswered, A5 risks shipping a verify endpoint that returns `valid: true` even when the Ledger's own chain is broken.
2. **Verifier interface.** Third parties must be able to verify an exported PROV-O bundle + Ledger anchor **without** running wos-server. The verification code can't be wos-server-resident; it must live in wos-core and parameterise over the Ledger impl.

Deliverables:

- **New ADR** `thoughts/decisions/YYYY-MM-DD-wos-chain-vs-ledger-chain.md`. Recommend posture (a) — defense-in-depth — on the grounds that (i) WOS's chain verifies across records the Ledger doesn't see (kernel transitions, action execution, governance state), (ii) the two chains fail independently, so any inconsistency is itself evidence. Document the alternative (retire WOS chain) as a future path if operational friction proves the defense-in-depth cost isn't worth it.
- **New trait** `LedgerVerifier` in `crates/wos-core/src/provenance.rs`:
  ```rust
  pub trait LedgerVerifier {
      fn verify_attachment(
          &self,
          record: &ProvenanceRecord,
          attachment: &LedgerAttachment,
      ) -> Result<bool, String>;
  }
  ```
- **Default impl**: `NoopLedgerVerifier` returns `Ok(true)` and sets `algorithmId: "noop-ledger-verifier"` on the chain-verify response. Real Ledger adapter (upstream) supplies a `FormspecRespondentLedgerVerifier` with inclusion-proof checking.
- **A5 consumer**: `verify_chain(log, verifier) -> ChainVerifyReport` takes `&dyn LedgerVerifier`. Report includes `wos_chain_valid`, `ledger_attachments_valid`, `records_without_attachment` counts. Consumers choose what "valid" means for their deployment.

### G7. `Coprocessor` trait in `wos-formspec-binding` (~1 day)

The reference architecture (`thoughts/examples/temporal-reference-implementation.md` §8) names the **Coprocessor** as the bridge between Formspec submissions and WOS case instances. It has three responsibilities: (a) map submission → case file (optionally via Mapping DSL), (b) validate-and-fire (validate response, fire WOS event), (c) link respondent ledger (emit a provenance record linking the Ledger checkpoint). No trait exists today; `wos-formspec-binding` is a one-file crate with no bridge types.

- **Populate `crates/wos-formspec-binding/src/`** with:
  - `lib.rs` — re-exports of the submission + response types shared with Formspec.
  - `coprocessor.rs` — `Coprocessor` trait:
    ```rust
    pub trait Coprocessor {
        type Error: std::error::Error;
        fn map_submission_to_case_file(
            &self,
            submission: &SubmissionPayload,
            kernel: &KernelDocument,
        ) -> Result<CaseFile, Self::Error>;
        fn validate_and_fire(
            &self,
            submission: &SubmissionPayload,
            kernel: &KernelDocument,
            validator: &dyn ContractValidator,
        ) -> Result<WosEvent, Self::Error>;
        fn link_respondent_ledger(
            &self,
            submission: &SubmissionPayload,
            instance: &CaseInstance,
        ) -> ProvenanceRecord;
    }
    ```
  - `submission.rs` — shared types (`SubmissionPayload`, `WosEvent` submission variant).
- **Default impl** `DirectNameMapping` for the "Formspec field names match case file field names" fallback per reference doc §8. Real Mapping-DSL impl is upstream (Formspec-owned).
- **Server wiring**: `POST /api/submissions` handler calls `coprocessor.validate_and_fire(...)` then `coprocessor.link_respondent_ledger(...)` before dispatching the resulting event into the runtime.
- **Unblocks** the Formspec coprocessor integration called out in TODO.md (currently a prose gap, no implementation anchor).

### G8. `TaskStore` trait distinct from `InstanceStore` (~1 day)

The reference architecture (`temporal-reference-implementation.md` §9) models human tasks as a first-class store distinct from the instance store — with their own lifecycle (Created/Assigned/Claimed/Completed per Gov §10), their own ACL rules (`excludedOwners`, `potentialOwners`, separation-of-duties on claim), and their own query surface (reviewer-dashboard "my queue"). Today wos-core has `TaskPresenter` (present-to-actor semantics) but no store-level seam; the server inlines task persistence into its SQLite storage layer.

- **New trait** `TaskStore` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait TaskStore {
      type Error: std::error::Error;
      fn insert(&mut self, task: &Task) -> Result<(), Self::Error>;
      fn get(&self, task_id: &str) -> Result<Task, Self::Error>;
      fn update_status(&mut self, task_id: &str, status: TaskStatus, actor: &str) -> Result<(), Self::Error>;
      fn list_for_actor(&self, actor_id: &str, filter: TaskFilter) -> Result<Vec<Task>, Self::Error>;
      fn actor_determined_case(&self, actor_id: &str, case_id: &str) -> Result<bool, Self::Error>;
  }
  ```
- **New type** `Task` in `crates/wos-core/src/instance.rs` (or a new `crates/wos-core/src/task.rs`) carrying id, case_id, task_ref, status, owner, potential_owners, excluded_owners, sla, context.
- **SoD check moves here**: `actor_determined_case` is the hook B2's separation-of-duties enforcement calls. Default impl queries provenance.
- **Default impl** on `DefaultRuntime`: in-memory `HashMap<TaskId, Task>`, matching the existing in-memory pattern.
- **Server migration**: `crates/wos-server/src/storage/` gets a `SqliteTaskStore` impl against a new `tasks` table (migration 0004). `wos-server` handlers for task claim / complete delegate to `TaskStore` instead of inline SQL.
- **Why separate from `InstanceStore`:** reference doc is explicit — Temporal stores instance state in its event history, but tasks live in a Postgres table external to Temporal. Making `TaskStore` a distinct seam means the Temporal adapter (Track H3) can use Temporal for instances + Postgres for tasks without an awkward inner join.

### G9. `NotificationService` trait (~3 hr)

Reference doc §2 lists `send_notification` as a first-class activity. Today notifications are inlined into `crates/wos-server/src/services/notifications_service.rs` with no trait. A real deployment swaps SMTP / SendGrid / SMS / push — that's per-deployment, so it needs a seam.

- **New trait** `NotificationService` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait NotificationService {
      type Error: std::error::Error;
      fn send(&self, envelope: &NotificationEnvelope) -> Result<DeliveryReceipt, Self::Error>;
  }
  pub struct NotificationEnvelope {
      pub template_ref: String,              // reference to a Notification Template sidecar
      pub to: Vec<Recipient>,
      pub variables: serde_json::Map<String, Value>,
      pub channel: ChannelKind,              // Email | Sms | Push | Webhook
      pub idempotency_key: Option<String>,
  }
  ```
- **Default impl** on `DefaultRuntime`: a `LogNotificationService` that records the notification into provenance and returns a synthetic receipt. Real impls ship as consumer-provided adapters.
- **Server wiring**: `notifications_service.rs` shrinks to a thin caller + the reminder-scheduling logic (which moves to Track H1's `TimerService`).

### G10. `DirectoryService` trait (~3 hr)

Once `AuthVerifier` (G1) stops owning the user directory, someone has to — handlers that list reviewers-for-assignment need a source of truth. Real IdPs expose user/group APIs (Okta, Azure AD, Google Workspace); default impl reads the local users table.

- **New trait** `DirectoryService` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait DirectoryService {
      type Error: std::error::Error;
      fn user_by_id(&self, id: &str) -> Result<Option<UserRecord>, Self::Error>;
      fn list_users(&self, filter: UserFilter) -> Result<Vec<UserRecord>, Self::Error>;
      fn list_groups(&self) -> Result<Vec<GroupRecord>, Self::Error>;
      fn users_in_group(&self, group_id: &str) -> Result<Vec<UserRecord>, Self::Error>;
  }
  ```
- **Default impl** on `DefaultRuntime`: empty directory — integrators opt in.
- **Server impl** `SqliteDirectoryService` reads the existing `users` table. Future OIDC impl (consumer-provided) calls the IdP's SCIM or equivalent endpoint.
- **Wiring**: B2's delegation-chain validation uses `DirectoryService::user_by_id` to resolve delegator/delegate metadata; task-assignment handlers use `users_in_group` to resolve `potentialOwners: "role:reviewer"` style references.

### G11. `VisibilityService` trait (~3 hr)

Reference doc §10 names four query surfaces for external observers: case status, provenance log, case file, governance state. Today each is an ad-hoc HTTP handler inside wos-server; an engine-adapter (Track H) needs to map WOS queries to Temporal `queryWorkflow` calls. A seam fixes both.

- **New trait** `VisibilityService` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait VisibilityService {
      type Error: std::error::Error;
      fn case_status(&self, instance_id: &str) -> Result<CaseStatusView, Self::Error>;
      fn case_file(&self, instance_id: &str) -> Result<serde_json::Value, Self::Error>;
      fn governance_state(&self, instance_id: &str) -> Result<GovernanceStateView, Self::Error>;
      fn list_cases(&self, filter: CaseFilter) -> Result<Vec<CaseListEntry>, Self::Error>;
  }
  ```
- **Default impl** on `DefaultRuntime`: projects views from the in-memory `InstanceStore`. Server's SQLite impl reads from the existing storage tables.
- **Temporal-adapter impl** (Track H3): each method translates to `temporal_client.query_workflow(case_id, WosQuery::...)` per reference doc §10. That's the payoff — without this seam, every query handler in wos-server has to be ported individually to the Temporal adapter.
- **Provenance query** is intentionally NOT on this trait — reference doc §10 explicitly says "Provenance is queried from the provenance store directly (not Temporal), because the append-only log may be large and is stored separately." Keep `ProvenanceService` separate, as today.

### Track G roll-up

11 items split into three groups by compounding-debt urgency:

- **Trait-shape compounding (must land before consumers multiply):** G1 (AuthVerifier), G2 (LedgerAttachment), G4 (FieldError). All under a day each.
- **Missing seams (additive):** G3 (IdentityResolver), G5 (PolicyEngine), G7 (Coprocessor), G8 (TaskStore), G9 (NotificationService), G10 (DirectoryService), G11 (VisibilityService). Mostly additive; G8 and G7 are the larger ones (~1 day each) because they move real code.
- **Architectural decision:** G6 (Chain ADR + LedgerVerifier). Half-day for the ADR + trait, but the ADR itself needs consensus.

Total estimated effort: **~6 engineer-days** across Track G, with G1+G2+G4 fitting into Phase 0.5 (~1.5 days) and the rest spread across Phases 2–6.5.

---

## Track H — structural corrections (engine-adapter readiness)

The external-owner review surfaced one **structural** issue that forecloses the workflow-engine adapter boundary (§7 interop), plus two additive changes needed to keep the door open. Distinct from Track G because these touch `CaseInstance`'s shape, not just trait signatures. Do **not** land Track H1 casually — it's a coordinated refactor across wos-core + wos-runtime + every existing timer test. But leave the current shape and the first real Temporal adapter pays the full migration cost.

### H1. Extract timer state from `CaseInstance` to `TimerService` seam (~3 days, **structural — D=5**)

**The problem.** Today `wos-core::timer::Timer` (at `crates/wos-core/src/timer.rs`) holds full timer state — `id`, `created_at_ms`, `deadline_ms`, `fires_event`, `created_in_state`, `duration_iso`, `duration_ms`. Timers are stored as fields on `CaseInstance` (grep `timers:` in `crates/wos-core/src/instance.rs`). That makes timer existence part of the data Temporal (or any durable-timer service) would store. A Temporal adapter can't own timers whose state lives inside the workflow payload — Temporal owns timers *natively* via `ctx.timer_with_signal(duration, signal)` per reference doc §4 step 4. **Structural mismatch.**

**The fix — follow the reference architecture.** Reference doc §4 has `Evaluator::process_event()` return `eval_result.timer_operations: Vec<TimerOp>` as a *plan* — the evaluator says "start timer X for duration D, firing event E" or "cancel timer X"; the host executes the plan. Timers become refs + intent, not state.

Deliverables:

- **New trait** `TimerService` in `crates/wos-core/src/traits/mod.rs`:
  ```rust
  pub trait TimerService {
      type Error: std::error::Error;
      fn schedule(
          &mut self,
          instance_id: &str,
          timer_id: &str,
          duration: chrono::Duration,
          fires_event: &str,
      ) -> Result<(), Self::Error>;
      fn cancel(&mut self, instance_id: &str, timer_id: &str) -> Result<(), Self::Error>;
      fn list_pending(&self, instance_id: &str) -> Result<Vec<TimerRef>, Self::Error>;
  }
  pub struct TimerRef { pub timer_id: String, pub fires_event: String, pub deadline_ms: u64 }
  ```
- **New type** `TimerOp` in `crates/wos-core/src/timer.rs`:
  ```rust
  pub enum TimerOp {
      Start { timer_id: String, duration: chrono::Duration, fires_event: String, created_in_state: String, duration_iso: String },
      Cancel { timer_id: String, reason: String },
  }
  ```
- **Narrow `CaseInstance.timers`** from the current full-state `Vec<Timer>` to `Vec<TimerRef>` (id + fires_event + deadline snapshot, no duration origin). The origin-timestamp and duration-ISO move into the `TimerService` impl's storage.
- **Evaluator change**: `WosRuntime`'s action-processing (at `crates/wos-runtime/src/runtime.rs`, grep `startTimer` / `cancelTimer`) returns a `Vec<TimerOp>` alongside its existing result. The host (in-process default, server, or Temporal adapter) executes the ops against its `TimerService`.
- **Default in-process impl** `InMemoryTimerService` in wos-runtime — replaces the current embedded-state behaviour; passes all existing tests unchanged because the aggregate semantics are identical.
- **Server wiring**: `crates/wos-server/src/services/timer_task.rs` becomes the default `TimerService` impl (polling SQLite); existing tick loop becomes the scheduling-and-firing runtime for the default impl.
- **Temporal-adapter implication**: a `TemporalTimerService` impl is a 20-line shim — `schedule` → `ctx.timer_with_signal(...)`, `cancel` → `ctx.cancel_timer(...)`. Engine adapter becomes trivially buildable when Track H3 lands.

**Migration cost.** Every fixture / test that asserts on `CaseInstance.timers[i].created_at_ms` or `duration_ms` needs updating — the origin info moves out. Grep `created_at_ms` gives the blast radius across runtime tests + fixture files. Scripted migration + manual review, one sprint.

**Why not skip.** The plan currently says "§7 Interop + speculative research — engine adapters held until a first-commercial-deployment-demands trigger fires." That's a reasonable posture for *building* the adapter, but the timer-state structural choice is a *shape-level* foreclosure that compounds with every new consumer of the current `CaseInstance.timers` shape. Land the shape fix before the blast radius grows; defer the adapter itself until demand fires.

### H2. Hoist `Clock` trait to `wos-core::traits` (~1 hr)

Correctness/consistency fix. `Clock` lives at `crates/wos-runtime/src/runtime.rs:104` with `SystemClock` impl at line 162 — sitting apart from the nine traits in `wos-core::traits`. That's the framing inconsistency flagged in principle 1. Hoisting:

- **Move** `pub trait Clock` + `pub struct SystemClock` + `impl Clock for SystemClock` from `crates/wos-runtime/src/runtime.rs` to `crates/wos-core/src/traits/mod.rs`. Add `impl Clock for DefaultRuntime` delegating to `SystemClock`.
- **Re-export** from `crates/wos-runtime/src/lib.rs` so existing consumers that import `wos_runtime::{Clock, SystemClock}` keep working. No breaking change.
- **Update PARITY.md** to remove the "nine traits" phrasing; say "ten host-interface traits in wos-core + ~8 new seams from Track G".

### H3. Additive event-sourced variant on `InstanceStore` (~half day)

Adapter-readiness fix. `InstanceStore::load(instance_id)` returns a full snapshot; `InstanceStore::save(instance)` persists a full snapshot. Works fine for SQLite. Doesn't work for Temporal's event-sourced model — Temporal stores the event history, rehydrates state by replay; there's no "snapshot to save" because every activity result is already in the history.

Make this additive, not breaking:

- **New trait method** `InstanceStore::replay_events(instance_id, from_seq) -> Result<Vec<StoredEvent>, Self::Error>` with a default impl returning `Ok(vec![])` (snapshot-only stores return nothing; callers that want event-sourcing check non-empty and replay).
- **New trait method** `InstanceStore::append_event(instance_id, event) -> Result<EventSeq, Self::Error>` with default impl returning `Err(NotSupported)` — snapshot stores keep saving whole instances; event-sourced stores implement this instead of `save`.
- **Companion flag** on `InstanceStore`: `fn prefers_event_sourcing(&self) -> bool { false }` — the runtime uses this to decide whether to call `save(instance)` or `append_event(instance_id, event)` after each transition.
- **No change to existing SQLite impl** — it defaults to snapshot mode, same behaviour as today.
- **Temporal-adapter impl** (Track H4 skeleton only): `TemporalInstanceStore` overrides `prefers_event_sourcing` + `append_event` + `replay_events`, leaves `save` unimplemented.

**Why additive works here (vs G2 which is breaking):** the return type of `sign` has to change to carry structured ledger data — every consumer needs the new shape. `InstanceStore`'s two modes (snapshot vs event-sourced) can coexist because no single consumer needs both at once — they pick one based on store impl. Additive default methods suffice.

### H4. `wos-temporal` crate skeleton (future-marker, **0 effort now; document the target**)

The reference architecture at `thoughts/examples/temporal-reference-implementation.md` is detailed enough to serve as the target design for the first engine adapter. Rather than building it now, **add a placeholder Cargo workspace member** that captures the target crate shape without implementation. This serves two purposes: (a) locks in the trait surface Track G/H1 must deliver, (b) gives the "first-commercial-demand trigger" (§7) a concrete starting point rather than a blank page.

Deliverables (all zero-implementation):

- **New workspace member** `crates/wos-temporal/` with:
  - `Cargo.toml` — declared `publish = false`, dependency stubs for `temporal-sdk` (feature-gated, optional).
  - `src/lib.rs` — module declarations pointing to file-per-concept stubs: `workflow.rs`, `coprocessor.rs`, `activities/*.rs`, `store/*.rs`, `queries.rs`, `signals.rs` — mirroring reference doc §2's layout.
  - Each file contains a single `#![allow(dead_code)]` module-level attribute + a doc-comment pointing at the corresponding reference-doc section. No code.
- **README.md** in `crates/wos-temporal/` pointing at the reference doc and listing the Track G/H traits this crate will implement.
- **CI guard**: `cargo build -p wos-temporal` must succeed (forces the skeleton to stay compile-valid as the workspace evolves) even though no logic exists.
- **Triggers for real implementation**: the commercial-demand flag from §7. When it fires, H4 becomes a real track.

**Why stub now, not later.** A real consumer asking "does WOS run on Temporal?" can point at this crate + the reference doc and see "these are the traits that would be implemented" without the plan having to re-derive the architecture under time pressure. Documentation work, not engineering work — under half a day.

### Track H roll-up

Four items, mixed effort:

- **H1 Timer extraction** — ~3 days, structural, D=5. The one item that must land even if the Temporal adapter never ships, because the `CaseInstance.timers` shape is compounding debt per fixture.
- **H2 Clock hoist** — ~1 hr, correctness/consistency fix. Can ride with any other wos-core change.
- **H3 Event-sourced InstanceStore variant** — ~half day, additive, can slip.
- **H4 wos-temporal skeleton** — ~half day, documentation-only, anchors the target architecture.

Total estimated effort: **~4 engineer-days** across Track H. H1 is the one that justifies the cost; the others are cheap riders.

---

## Phased sequence

Interleaves the tracks by dependency. Each phase is a work-chunk that can be executed by a small team in parallel when the items don't depend on each other.

### Phase 0 — unblock (1 day)

Clear the prereqs that gate everything else.

- **C1 DRAFTS triage** — unblocks C6 (#20).
- **A3 Legal-sufficiency disclosure** — independent quick win (now lands in **wos-export**, not server, per placement correction).
- **A6 JSON-LD context endpoint** — independent quick win (**wos-export** extension + server passthrough).
- **H2 Clock hoist to `wos-core::traits`** — ~1 hr correctness fix; rides with any other change.

### Phase 0.5 — compounding trait-shape fixes (~1.5 days, **must precede Phase 1**)

Three trait-shape fixes that compound per consumer. Landing them before Phase 1 means `NoopSigner` (A1) and `PolicyLayeredValidator` (B1) are built against the corrected shapes from the start; landing them after means every Phase-1/2 consumer gets a breaking-change migration later. Per Guiding Principle 7.

- **G1 `AuthVerifier` narrowing** — drop `login` from trait, widen `AuthUser` to roles + groups + claims. Unblocks OIDC.
- **G2 `ProvenanceSigner` → typed `LedgerAttachment`** — before A1 wires `NoopSigner` against the old `Vec<u8>` shape.
- **G4 `ValidationResult.errors` → `FieldError[]`** — before B1 wires `PolicyLayeredValidator` against the string shape.

Parallel-safe — three independent trait edits.

### Phase 1 — wire the unwired seams (1 day)

- **A1 `ProvenanceSigner` seam** — `NoopSigner` + config + injection (now emits a typed `LedgerAttachment` thanks to G2).
- **A2 `ReportRenderer` seam** — `JsonReportRenderer` default moves to **wos-core** per placement correction; server owns only config + injection + HTML skeleton.
- **A5 Chain-integrity verify endpoint** — algorithm lands in **wos-core::provenance** (parameterised on `LedgerVerifier` from G6); server keeps only HTTP wrapper.
- **G3 `IdentityResolver` trait** — additive trait + default impl; server middleware wires claim extraction now so Phase 2's `RoleBasedAccessControl` can consume resolved actors from day one.

Unblocks Phase 3's `/explain` endpoint and Phase 4 attestation work.

### Phase 2 — tighten the stubbed seams (~3 days)

- **B1 `PolicyLayeredValidator` + §15.7 default** — enforcement lives in **wos-runtime** (per placement correction); server opts in.
- **B2 `RoleBasedAccessControl` + SoD default** — decorator lives in **wos-runtime**; server composes.
- **B3 `IntegrationDispatchService` + correlation tokens** — binding-kind dispatcher lives in **wos-runtime**; server ships only concrete HTTP/Socket.IO transport adapters. Moved up from Phase 6 per the plan's own D=5 compounding-debt rule.
- **G5 `PolicyEngine` trait** — concrete trait in `wos-core::traits` so B2's decorator and B3's dispatcher have a seam to wire rather than prose references.
- **G6 Chain ADR + `LedgerVerifier` trait** — half-day ADR + trait; A5 (already landed) switches to parameterised verify on this trait once it exists.

Parallel-safe. Stops the three compounding "permissive behaviour shipped" / "stale trait signature shipped" debts in one phase; lands the policy-engine + ledger-verifier seams the subsequent tracks depend on.

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
- **B8 Drift write-side endpoint** — `DriftReport` type lands in **wos-core::model** per placement correction.
- **C7 #31 Jurisdiction-aware calendar**.

### Phase 6.5 — external-owner seam fill-in (~3 days, parallel-safe with Phase 6)

Remaining Track G items. Each is additive (no breaking change). These lock the external-owner boundaries for the first commercial deployment without waiting for the engine-adapter trigger.

- **G7 `Coprocessor` trait** — populate `wos-formspec-binding` with bridge types + default `DirectNameMapping` impl.
- **G8 `TaskStore` trait** — extract task persistence from `wos-server/src/storage/` into a `wos-core` trait + SQLite default impl; SoD hook (`actor_determined_case`) lands here.
- **G9 `NotificationService` trait** — extract notification delivery from `wos-server/src/services/notifications_service.rs` into a `wos-core` trait; server keeps the reminder-scheduling logic (that moves in Phase 7's H1).
- **G10 `DirectoryService` trait** — user/group listing seam; SQLite default reads local `users` table.
- **G11 `VisibilityService` trait** — the four query surfaces; seam that enables engine-adapter query mapping in H4.

### Phase 7 — structural corrections (~3–4 days, **coordinated, serial**)

Track H. Timer extraction is the one high-blast-radius item — it touches `CaseInstance`, the evaluator's return type, every timer fixture, every timer test. Do it in one focused sprint rather than amortising.

- **H1 Timer extraction** — ~3 days; scripted fixture migration + manual review; runtime evaluator returns `Vec<TimerOp>` plan; server's `timer_task.rs` becomes the default `TimerService` impl.
- **H3 Event-sourced `InstanceStore` variant** — ~half day; additive default methods; no existing consumer breaks.
- **H4 `wos-temporal` crate skeleton** — ~half day; documentation-only workspace member mirroring reference doc §2 layout.

**Gate before proceeding to Phase 8**: full `cargo nextest run --workspace` green after H1's fixture migration. This is the single place in the plan where a D=5 structural change is worth pausing the backlog for.

### Phase 8 — behavioral backlog cheap batch (1 sprint, parallelizable)

Track E2 — all six items ship together.

### Phase 9 — behavioral backlog next-batch (ongoing)

Track E1 — six items, ~4–6 weeks total.

### Phase 10 — behavioral backlog depth (ongoing, prioritised by envelope-stack demand signal)

Track E3 — thirteen items. Promote #43 + #40 + #30 + #38 ahead of the others for envelope-stack composition.

### Phase 11 — structural merges + hygiene (spare-capacity)

Track E4 + E5. Schedule when code in the merged sidecars is being actively touched for another reason.

### Phase 12 — long tail

Track F — Merkle chains (§5 #48) is the next compounding-debt item beyond the Phase 0–7 scope; schedule once Phase 7 wraps.

### Total estimate

Phases 0–7 strictly serial, single-engineer: ~**10–13 weeks** (Phase 0: 1d · Phase 0.5: 1.5d · Phase 1: ~1.5d · Phase 2: ~3d · Phase 3: ~5 weeks · Phase 4: ~3d · Phase 5: ~1 week · Phase 6: ~3d · Phase 6.5: ~3d · Phase 7: ~3–4d). Phase 3 remains the bottleneck. Phase 7's H1 is the second-largest single-phase cost.

With **2–3 engineers** running Phase 3 spec work in parallel with Phase 4–6.5 server work (most server items don't block on spec except A4/explanation scaffold), overall elapsed time drops to **~4–5 weeks**. Phase 3 itself drops to 2–3 weeks with 3 engineers; Phase 6.5 parallelises across two engineers.

Phase 7 (timer extraction) is best done by one engineer serially — parallel work on `CaseInstance.timers` creates merge-conflict hell. Schedule it at a natural lull in the spec work.

Phases 8–12 run indefinitely as backlog flows; the envelope-stack is usable end-to-end after Phase 6; the Temporal-adapter door is genuinely open after Phase 7.

---

## Verification

### Per-phase gates

Each phase must pass before the next starts:

| Phase | Gate |
|---|---|
| 0 | `cargo build --workspace` clean · `DRAFTS/` resolved · disclosure block present in all three export formats (tests land in **wos-export**) · `Clock` importable from `wos_core::traits` |
| 0.5 | `AuthProvider::login` removed from trait (handlers that need it downcast to `SessionManager`) · `ProvenanceSigner::sign` returns `LedgerAttachment` (unit test constructs one) · `ValidationResult.errors: Vec<FieldError>` (existing tests updated) |
| 1 | New seams visible in `AppRuntime::build` · `WOS_SIGNER=noop` + `WOS_RENDERER=json` defaults boot cleanly · `GET /provenance/verify` returns `{valid: true, algorithmId: "…"}` on seeded fixture · `IdentityResolver::resolve` called at every handler entry point (grep audit) |
| 2 | `cargo nextest run -p wos-server` green · new integration test `tests/policy_validator.rs` rejects rights-impacting submit without `respondentLedgerRef` (enforcement runs inside wos-runtime's default policy) · new test `tests/separation_of_duties.rs` rejects self-review agent transition · `PolicyEngine` trait present in `wos-core::traits` with `EchoPolicy` default · Chain ADR committed to `thoughts/decisions/` · `LedgerVerifier` trait exported from `wos-core::provenance` |
| 3 | `cargo nextest run -p wos-lint` + `cargo nextest run -p wos-conformance` green · fixtures retrofit verified (every determination transition has `inputs`) · three determinism fixtures for #2 produce identical outputs bit-for-bit |
| 4 | `/explain` returns a payload whose shape matches Runtime §9.1 (algorithm lives in **wos-runtime**; server is thin wrapper) · dupe `idempotencyToken` on events → single drain (asserted) · as-of policy resolution unit test green |
| 5 | All five envelope-reference fixtures parse + lint clean · envelope-decline-reroute conformance fixture runs end-to-end · `#61` SoD fixtures run and (correctly) fail against B2's enforcement |
| 6 | `ExternalService::invoke` signature updated in wos-core with correlation token · existing integration-profile fixtures still parse · migration endpoint round-trips `WosRuntime::migrate` · `jurisdiction` field on case-state drives correct calendar selection · `DriftReport` type in `wos-core::model` |
| 6.5 | `Coprocessor`, `TaskStore`, `NotificationService`, `DirectoryService`, `VisibilityService` traits present in `wos-core::traits` (or `wos-formspec-binding` for `Coprocessor`) with `DefaultRuntime` impls · server's task / notification / directory / visibility handlers delegate to traits (no inline persistence) · `wos-formspec-binding` crate populated with bridge types |
| 7 | `CaseInstance.timers` is `Vec<TimerRef>` (not `Vec<Timer>`) · `TimerService` trait present in `wos-core::traits` with `InMemoryTimerService` default in wos-runtime · evaluator returns `Vec<TimerOp>` alongside existing result · `crates/wos-temporal/` builds clean · `cargo nextest run --workspace` green · fixture migration complete (grep `created_at_ms` in `fixtures/` returns zero hits at call-site depth) |
| 8–12 | Backlog items verified item-by-item per their own acceptance criteria |

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

- `cargo nextest run -p wos-conformance` runs on every commit; 146 existing fixtures stay green.
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

Placement corrections from the DI review pull several items out of server into their canonical crates (see "wos-runtime / wos-core / wos-export / wos-formspec-binding" subsection above). Server-local new files are limited to HTTP handlers, config, migrations, and concrete transport adapters:

- `crates/wos-server/src/runtime/signer.rs` — `NoopSigner` thin wrapper (A1; returns the typed `LedgerAttachment` per G2).
- `crates/wos-server/src/runtime/renderer.rs` — thin re-export of `wos_core::JsonReportRenderer` + feature-gated `HtmlReportRenderer` skeleton (A2; default impl moved to wos-core per placement).
- `crates/wos-server/src/services/explanation_service.rs` — thin wrapper around `wos_runtime::explain` (A4; algorithm moved to wos-runtime per placement).
- `crates/wos-server/src/http/policy.rs` — as-of resolution (A8).
- `crates/wos-server/src/http/holds.rs` — hold CRUD (B5).
- `crates/wos-server/src/http/semantic.rs` — `/jsonld-context` passthrough (A6; bytes served from wos-export).
- `crates/wos-server/src/storage/task_store.rs` — `SqliteTaskStore` impl of wos-core's `TaskStore` trait (G8).
- `crates/wos-server/src/storage/directory.rs` — `SqliteDirectoryService` impl (G10).
- `crates/wos-server/src/storage/visibility.rs` — SQLite-backed `VisibilityService` impl (G11).
- `crates/wos-server/migrations/0003_drift_reports.sql` — drift-report storage (B8).
- `crates/wos-server/migrations/0004_tasks.sql` — task table (G8).

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

### wos-runtime / wos-core — changes (expanded)

**Trait-shape edits (Phase 0.5 — compounding fixes):**
- `wos-server/src/auth/mod.rs` — `AuthProvider` narrowed to `AuthVerifier` trait (`verify` only); `SessionManager` sub-trait holds `refresh`/`logout`/`login`. `AuthUser` widens to `roles + groups + claims` (G1).
- `wos-core::traits::ProvenanceSigner` — `sign` returns `LedgerAttachment` (new type in `wos-core::provenance`); `verify` accepts `&LedgerAttachment` (G2).
- `wos-core::traits::ValidationResult.errors` — `Vec<FieldError>` instead of `Vec<String>`; `FieldError` new type in `wos-core::traits` (G4).

**Additive trait additions (Phase 1, 2, 6.5):**
- `wos-core::traits::IdentityResolver` (G3, Phase 1).
- `wos-core::traits::PolicyEngine` + `PolicyRequest` + `PolicyDecision` (G5, Phase 2).
- `wos-core::provenance::LedgerVerifier` + `verify_chain` function (G6, Phase 2).
- `wos-formspec-binding::Coprocessor` (G7, Phase 6.5).
- `wos-core::traits::TaskStore` + `wos-core::task::Task` (G8, Phase 6.5).
- `wos-core::traits::NotificationService` (G9, Phase 6.5).
- `wos-core::traits::DirectoryService` (G10, Phase 6.5).
- `wos-core::traits::VisibilityService` (G11, Phase 6.5).

**Trait moves (Phase 0):**
- `wos-runtime::runtime::Clock` moves to `wos-core::traits::Clock` with wos-runtime re-export (H2).

**Structural edits (Phase 7):**
- `wos-core::instance::CaseInstance.timers` narrows from `Vec<Timer>` to `Vec<TimerRef>` (H1).
- `wos-core::timer::TimerOp` new type (H1).
- `wos-core::traits::TimerService` new trait (H1).
- `wos-runtime::WosRuntime` evaluator return type extended with `Vec<TimerOp>` alongside existing result; action-processing at `runtime.rs` lines around `startTimer`/`cancelTimer` refactored (H1).
- `wos-core::traits::InstanceStore` — new default methods `prefers_event_sourcing`, `append_event`, `replay_events` (H3).

**Non-shape-changing edits (carried forward):**
- `wos-core::traits::ExternalService` gains a default method `invoke_with_correlation(service_ref, input, idempotency_key, correlation_token)` delegating to `invoke` (B3). **Non-breaking** — existing adapters keep compiling against the current `invoke` signature; new integration handlers call `invoke_with_correlation`.
- `wos-core::traits::ContractValidator` — optionally extended with `validate_in_context(contract_ref, data, impact_level, instance_id)` default method (B1 alternative path) if the submit-path `SubmitPolicy` approach isn't adopted. Non-breaking either way.
- `wos-runtime::WosRuntime` (`crates/wos-runtime/src/runtime.rs`) — new `Box<dyn ProvenanceSigner>` + `Box<dyn ReportRenderer>` + `Box<dyn TimerService>` + `Box<dyn PolicyEngine>` + `Box<dyn NotificationService>` + `Box<dyn TaskStore>` + `Box<dyn DirectoryService>` + `Box<dyn VisibilityService>` + `Box<dyn IdentityResolver>` + `Box<dyn Coprocessor>` fields, new builder methods per each. Thread the signer into every provenance-emit site.

### New files — wos-core / wos-runtime / wos-export / wos-formspec-binding / wos-temporal

- `crates/wos-core/src/provenance.rs` — `verify_chain` function + `LedgerAttachment` type + `LedgerVerifier` trait + `NoopLedgerVerifier` default (G2/G6).
- `crates/wos-core/src/model/drift_report.rs` — `DriftReport` type (B8 placement fix).
- `crates/wos-core/src/task.rs` — `Task`, `TaskStatus`, `TaskFilter` types (G8).
- `crates/wos-runtime/src/submit_policy.rs` — default-on `LedgerGatingSubmitPolicy` (B1 placement fix).
- `crates/wos-runtime/src/access_control.rs` — `DefaultAccessControl` decorator with SoD + delegation-chain + autonomy-cap enforcement (B2 placement fix).
- `crates/wos-runtime/src/dispatch.rs` — `BindingKindDispatcher` (B3 placement fix).
- `crates/wos-runtime/src/timer_service.rs` — `InMemoryTimerService` (H1).
- `crates/wos-runtime/src/explain.rs` — §9.1 explanation algorithm scaffold; internals swap to C8 when it lands (A4 placement fix).
- `crates/wos-export/src/disclosure.rs` — `Disclosure` struct + inline wrapping in prov_o/xes/ocel (A3 placement fix).
- `crates/wos-export/src/context.rs` — `context(format)` function serving canonical JSON-LD context bytes (A6 placement fix).
- `crates/wos-formspec-binding/src/{coprocessor,submission}.rs` — `Coprocessor` trait + bridge types + `DirectNameMapping` default (G7).
- `crates/wos-temporal/` — new workspace crate, skeleton only, mirrors reference doc §2 layout (H4).
- `thoughts/decisions/YYYY-MM-DD-wos-chain-vs-ledger-chain.md` — ADR (G6).

### Coordination — PARITY.md + TODO.md

Both documents get updated in-place as phases land. Existing sections stay; status columns and ranking table rows are mutated. No new top-level sections needed in either document.

---

## Success criteria

**End of Phase 0.5:** trait shapes are right. `AuthVerifier`, typed `LedgerAttachment` return on `ProvenanceSigner::sign`, `FieldError[]` on `ValidationResult` — these three compound-debt fixes land before any consumer wires against the old shapes. `cargo build --workspace` green; existing tests updated (not rewritten) to the new shapes.

**End of Phase 2:** no stubbed seam remains. Every `cargo nextest run -p wos-server` still green. PARITY.md DI seam status table shows all ten host-interface traits either wired-real or wired-real-with-policy-source-pluggable, plus `PolicyEngine` + `LedgerVerifier` promoted from prose to concrete traits. Chain ADR committed.

**End of Phase 4:** `/instances/:id/explain` serves Runtime §9.1-shaped output (real algorithm once #2 lands in wos-runtime; scaffold before). Chain-integrity verify returns `valid: true` on seeded fixtures; `algorithmId` field discloses which chain(s) were checked. Legal-sufficiency disclosure present on every export format (enforced by wos-export tests, not wos-server).

**End of Phase 5:** envelope-reference fixtures all parse, lint clean, and run end-to-end through conformance. Ecosystem integrators have canonical patterns to copy.

**End of Phase 6:** envelope stack is composable. A third party can plug Formspec + an identity adapter + a PDF/email layer and sign a 2-signer document end-to-end with auditable provenance, §15.7-gated rights-impacting submits (enforcement in wos-runtime, not server), and an attestation path wired through the injected `ProvenanceSigner` seam returning typed `LedgerAttachment`. The default `NoopSigner` makes the path end-to-end testable but signatures are empty; **externally-verifiable signing** requires either the feature-flagged `Ed25519FileKeySigner` reference impl or a consumer-injected HSM / cloud KMS / Respondent Ledger adapter — all of which slot into the same seam without further plumbing.

**End of Phase 6.5:** external-owner boundaries are all real traits. Formspec plugs in via `Coprocessor`. A real Ledger plugs in via `ProvenanceSigner` returning typed attachments + `LedgerVerifier` for inclusion-proof checks. Any OIDC IdP plugs in via `AuthVerifier` + `IdentityResolver` + `DirectoryService` without touching server code. Task / notification / visibility all have trait seams. Server is thinner by ~300–500 LOC (logic moved down; HTTP and config remain).

**End of Phase 7:** structural timer fix landed. `CaseInstance.timers` is `Vec<TimerRef>`; evaluator returns `Vec<TimerOp>`; `TimerService` trait is the only path to durable timers. `crates/wos-temporal/` skeleton compiles; reference-doc-aligned adapter can be implemented by a consumer in ~2 weeks (H1's timer seam + H3's event-sourced store variant + H4's skeleton remove every shape-level blocker). §7 engine-adapter work is genuinely unblocked, not just "held pending trigger."

**Downstream (Phases 8–12):** TODO.md §4.2, §4.3, §4.4 drain; engineering hygiene absorbed as code is touched; audit/evidence products (§5) deliver Merkle chains when demand signal appears; first engine adapter (Temporal reference) ships when commercial-demand trigger fires, reading the skeleton + reference doc as its starting specification.

---





















