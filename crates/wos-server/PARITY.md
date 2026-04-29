# wos-server Parity Matrix

_Cross-references `/specs` + `/schemas` against the server's HTTP + Socket.IO surface on branch `claude/wos-spec-backend-y17wJ` as of commit `645fbd8`._

> **Validation pass applied** (commit `d61b2af` → `9a063ad`). Three independent audits flagged 6 citation errors, 1 path fabrication (`/api/kernels` → actual `/api/bundles`), 4 mis-graded rows, an unsorted "ranked" table, and 4 unfair critiques. All applied.
>
> **DI seam rework applied** (2026-04-18). Re-framed the gap list around Runtime §12 host-interface seams. Two seams are unwired (`ProvenanceSigner`, `ReportRenderer`) — both top-ROI. Three seams are wired-but-stubbed (`AccessControl` permissive, `ContractValidator` permissive, `ExternalService` echo) — tightening them to policy-composing impls is the bulk of envelope-stack readiness. The "provenance attestation" row was dropped from the ranking entirely: it's a consumer-injected plug via `ProvenanceSigner`, not a server gap. Runtime §15.7 ledger-gating enforcement (missed in the prior validation pass) was added as the `PolicyLayeredValidator` item. The `/explain` handler line-count drops from 1-day to ~2-hours once `ReportRenderer` is wired.
>
> Paired spec-side planning lives in [`../../TODO.md §4.7`](../../TODO.md) — three new spec items (#58 envelope status, #59 CloudEvent envelope-flow catalog, #60 envelope reference fixtures) plus cross-ref annotations on existing items (#2, #20, #30, #38, #40, #43) that serve envelope-stack composition once they land.
>
> ▎ **Drift refresh applied** (2026-04-24). Pre-flight: `cargo check -p wos-server` and `cargo nextest run -p wos-conformance` run green after removing stale `crates/wos-synth` workspace member (root `Cargo.toml:9`), upgrading `socketioxide` 0.17→0.18, and fixing 11 type-drift errors between wos-server and current wos-core / wos-runtime types. Net status movement on the 22-row gap ranking: zero rows moved — all server-side seam work still pending per Day 1 / Day 2 / Day 3 sequence. Refresh covers: (a) new subsection ### custody-hook-encoding under Kernel; (b) new top-level ## Signature Profile alongside Integration/Semantic profiles; (c) new ## Extension Registry section; (d) new Kernel §8.2.1 Facts-tier snapshot row (status full, oracle `determination_transition_emits_case_file_snapshot`); (e) verified TODO cross-references — #20 #21 closed; #30 #38 #40 #43 #58 #59 #60 open with correct scoping (note #38 / #40 have closed authoring surfaces but open runtime/lint tails); (f) SignatureAffirmation emission recognised as runtime-wired (`signature.rs:447` / `tasks.rs:364`), status partial pending dedicated read surface; (g) schema-slug asymmetry subsection; (h) two footnotes on Ranked table. Methodology, rubric, and DI-seam framing preserved.
>
> ▎ **Runtime validation applied** (2026-04-24). Walked `wos-runtime/src/runtime.rs` (4680 lines, 40+ tests) against Runtime Companion spec (`specs/companions/runtime.md`). No status rows moved. Four note refinements applied: (a) Runtime §4.3 note corrected — idempotency is implemented for task submission AND `invokeService` via integration-profile step-result replay (oracle `drain_once_consumes_integration_profile_binding_and_replays_persisted_result`); the gap is specifically for _general event submission_ on `POST /events`. (b) Runtime §5.4 note refined — the runtime's integration-profile dispatch is complete (input mapping, output binding, idempotency key expression, step-result replay, contract-validation provenance); the limitation is the server's `EchoExternalService` impl. (c) Custody §1.10 note refined — runtime's `apply_custody_receipt` (`runtime.rs:2657-2798`, `provenance.rs:140-156`) stamps `canonical_event_hash` with idempotent reapply and conflict detection; no server HTTP endpoint exposes it. (d) Runtime §6 note refined — provenance position increment (`drain.rs:205`) lacks optimistic concurrency in `RuntimeStore` trait; safe under `Arc<Mutex<WosRuntime>>` single-node topology, would need OCC guard for distributed adapters.
>
> ▎ **DI seam sprint landed** (2026-04-25). Six ranked gaps closed by TODO.md WS-024–027, WS-031–032. Four DI seams moved from unwired/stubbed to wired-real: `ProvenanceSigner` → `NoopSigner` default (`runtime/signer.rs`, `runtime/mod.rs:81`), `ReportRenderer` → `JsonRenderer` default (`runtime/renderer.rs`, `runtime/mod.rs:82`), `AccessControl` → `RoleBasedAccessControl` with Gov §7.2 / AI §1.5 self-review rejection + delegation chains (`runtime/access.rs:91-114`, `runtime/mod.rs:73`), `ContractValidator` → `PolicyLayeredValidator` with §15.7 ledger-gating guard (`runtime/validator.rs:59-87`, `runtime/mod.rs:75`). Two endpoint gaps closed: `GET /api/instances/:id/provenance/verify` wraps `verify_chain` (`http/instances.rs:132-161`), and `POST /api/instances/:id/events` HTTP-layer idempotency dedup via in-memory `(instance_id, token) → EvaluationResultView` cache (`http/instances.rs:210-217,275-279`, `lib.rs:43`). Summary counts shift: full 30→35, partial 13→11, stub 9→8, none 16→12. Ranked table drops 6 rows (22→16). All validated against source before writing.
>
> ▎ **Server aggregation + surface refresh** (2026-04-24). Implementation notes (no spec obligation changes): (a) `storage::list_instances_all_pages` walks every page under SQLite's `page_size` clamp (max 200); used by `GET /api/tasks` (list + single-task lookup filters), dashboard `metrics` / `stage_metrics`, and `POST /api/equity/evaluate`, so aggregates are not silently truncated to the first page. Tests: `tests/storage_sqlite.rs` (`list_instances_all_pages_collects_beyond_single_sqlite_page`), `tests/equity_outcome_predicate.rs` (250-row cohort), `tests/timer_list_pagination.rs`. (b) `PUT /api/bundles/:url/kernel` requires `Authorization: Bearer` (mock or JWT). (c) CORS: invalid `WOS_CORS_ORIGIN` logs a warning and falls back to `allow_origin(Any)` (credentials not combined with that branch). (d) `POST /api/ai/chat` (Gemini): shared `reqwest::Client`, API key on `x-goog-api-key`. Table corrections: Runtime §7 timers — background poll paginates all instances each tick (correctness for large fleets; cost is still O(instances) per tick). Advanced §3 equity — `outcomePredicate: Some(_)` returns **400 Bad Request** until implemented (not a silent false outcome).

> | # | Item | State | Source |
> > |---|---|---|---|
> > | #2 | Deterministic adverse-decision notice (dual-form) | closed | COMPLETED.md |
> > | #20 | Typed event meta-vocabulary (TransitionEvent) | closed | COMPLETED.md |
> > | #21 | Extension registry (seams-only MVP) | closed (3550fad) | COMPLETED.md |
> > | #30 | WS-HumanTask lifecycle completion (Suspended, Cancelled, Return with rework counter) | open | TODO.md |
> > | #38 | G-064 Assertion Library resolution lint (spec/protocol closed separately) | open | TODO.md |
> > | #40 | Task SLA runtime implementation, incl. signature-class ↔ assurance binding | open, independent of #21 | TODO.md |
> > | #43 | Assurance × impact-level composition | open, not started | TODO.md |
> > | #58 | Envelope status extension | open, not started | TODO.md §Envelope-stack enablement (§4.7) |
> > | #59 | CloudEvent envelope-flow catalog | open, not started | TODO.md §Envelope-stack enablement (§4.7) |
> > | #60 | Envelope reference fixtures | open, not started | TODO.md §Envelope-stack enablement (§4.7) |
> >
> > Do NOT claim #30 / #38 / #40 as closed. Do NOT couple #43 to #21. Line numbers dropped 2026-04-24: TODO.md was reorganised with section-anchored task IDs; resolve by `#NN` ID, not line offset.

**Methodology.** Walked each spec extracting every imperative observable (MUST statements on processor behaviour, enumerated operations, processor-obligation tables). Cross-referenced against `crates/wos-server/src/http/*.rs` routes, `realtime/mod.rs` events, and `runtime/mod.rs` methods. Schema files that define document shapes (not observables) are marked "spec-side" — they're consumed as validation inputs to `POST /api/lint/document`, not served as first-class resources.

**Status legend.**

- **full** — endpoint exists, response matches spec shape, real logic behind it
- **partial** — endpoint exists, covers main use case, missing edge cases
- **stub** — endpoint exists with spec-correct response shape, values are synthetic
- **none** — no server surface; spec obligates one
- **spec-side** — document-shape spec, no server surface expected

**User-value lens.** Every row is also evaluated for "does this solve a real user problem or is it cargo-cult compliance?" Rows flagged are collected in the _User-value critique_ section.

**Maintenance convention.** After any HTTP / auth / storage surface change: append a dated `▎` block (or extend the latest one) at the top of this file and fix any affected matrix rows so later audits do not re-litigate fixed behaviour. Keep [`TODO.md`](TODO.md) as the single source for "what to do next"; this file owns "where are we vs. the spec."

---

## Auth contract (mirror of `README.md` Auth + Storage)

Implementers who skip the README still need these four invariants. They survive every adapter (mock / jwt / future OIDC) and any planned trait-narrowing (`AuthVerifier`, `roles + groups + claims`).

1. **Global logout.** `POST /api/auth/logout` (Bearer access token) increments `users.auth_epoch` AND revokes every `sessions` row for that user. New tokens carry the bumped `auth_epoch`; refresh + verify compare the JWT claim to the stored row, so an in-flight refresh after logout cannot mint a valid pair.
2. **Password rotation.** `Storage::set_user_password_hash` is the only entry point that changes a hash. It performs hash update + `auth_epoch` bump + session revoke in one transaction — old passwords AND old tokens both stop working atomically. Direct UPDATE on `users.password_hash` is a contract violation. Reachable from HTTP via `POST /api/auth/change-password` (WS-002).
3. **`upsert_user` does not overwrite secrets.** On `id` conflict, `upsert_user` updates only `email`, `name`, `role`, `avatar`. It never touches `password_hash` or `auth_epoch`, so a profile-edit flow cannot bypass the token-invalidation path.
4. **Realtime parity.** Under `WOS_AUTH=jwt`, every `kernel:update` re-runs `AuthProvider::verify` against the connect-time token AND re-reads the user row, so role changes and revocations apply to existing sockets without waiting for token expiry. Studios MUST reconnect after logout / role change / password reset to attach a fresh access token.

Companion: `wos-spec/CLAUDE.md` "Key rules" links here for top-level agent context. Trait-narrowing tracked in PARITY ▎ DI seam status row for `AuthProvider`.

---

## Summary

| Status | Count |
|---|---|
| full | 32 |
| partial | 14 |
| stub | 9 |
| none | 13 |
| spec-side | 22 |
| **total** | **90** |

Kernel + runtime companion are mostly implemented (Runtime §12 has eight of the spec's nine host interfaces wired — seven real, one stubbed). Governance L1 read-side and sidecar operations are solid. The gaps cluster in two places: (1) integration-profile real dispatch (currently echo) plus correlation tokens, (2) semantic profile's SHACL / SPARQL (triplestore adapter needed). Stubs are concentrated in advanced L3 (SMT verification, drift detection) — both require external adapters; their response shapes are spec-correct so consumers can integrate today.

---

## DI seam status (Runtime §12 host interfaces)

`wos-runtime` composes nine host-interface traits defined in `wos-core::traits`. The envelope-stack framing (`TODO.md §4.7`) shows that every "signing ceremony" or "attestation" concern reduces to wiring a seam — consumers inject their signer / renderer / identity adapter / policy engine, and WOS stays out of the primitive business. The table below is the authoritative map of what's wired.

| Trait (`wos-core::traits`) | Server impl | Status | Envelope-stack use |
|---|---|---|---|
| `InstanceStore` | `storage::SqliteRuntimeStore` | wired (real) | ✓ |
| `DocumentResolver` | `runtime::BundleServiceResolver` | wired (real) | ✓ |
| `TaskPresenter` | `runtime::SocketIoTaskPresenter` | wired (real) | ✓ |
| `EventQueue` | folded into `WosRuntime` internal queue | wired (real) | ✓ |
| `AccessControl` | `runtime::RoleBasedAccessControl` | wired (real) | ✓ separation-of-duties (Gov §7.2 / AI §1.5) — self-review rejection + delegation chains |
| `ExternalService` | `runtime::EchoExternalService` | wired (echo stub) | **seam for integration dispatch** — replace with `IntegrationDispatchService` reading bindings from resolver |
| `ContractValidator` | `runtime::PolicyLayeredValidator<PermissiveValidator>` | wired (real) | ✓ §15.7 ledger-gating for rights-impacting + safety-impacting submits; inner `PermissiveValidator` delegates contract-shape check |
| `ProvenanceSigner` | `runtime::signer::NoopSigner` | wired (real) | ✓ attestation seam — spec-correct envelope shape; consumers inject `Ed25519FileKeySigner` / HSM / cloud KMS via config |
| `ReportRenderer` | `runtime::renderer::JsonRenderer` | wired (real) | ✓ explanation / COC / notice rendering seam — unblocks `/instances/:id/explain` (WS-029) |

Two seams were unwired and are now wired (2026-04-25 sprint, WS-024 + WS-025):

- `ProvenanceSigner` wired with `NoopSigner` default (spec-correct envelope shape). Consumers plug whatever signer they have.
- `ReportRenderer` wired with `JsonRenderer` default. Turns the `/instances/:id/explain` endpoint into a ~50-line handler once the Runtime §9.1 deterministic algorithm (TODO.md #2, §4.1 critical path) lands.

Two seams were stubbed and are now wired-real (2026-04-25 sprint, WS-026 + WS-027):

- `AccessControl` permissive → `RoleBasedAccessControl` with separation-of-duties enforcement + delegation chains
- `ContractValidator` permissive → `PolicyLayeredValidator` with §15.7 ledger-gating; `PermissiveValidator` remains as inner test double

One seam remains stubbed:

- `ExternalService` echo → `IntegrationDispatchService` with real binding dispatch (TODO WS-028)

**Notes:**

- As of 2026-04-25, four seams wired in the DI seam sprint (WS-024–027): `ProvenanceSigner`, `ReportRenderer`, `AccessControl`, `ContractValidator`. One seam remains stubbed: `ExternalService` (echo).
- SignatureAffirmation emission is wired in runtime (`signature.rs:447` / `tasks.rs:364`) via the `InstanceStore` + provenance path; below the seam layer, does NOT change the nine-seam wiring status.
- Pre-flight compile gate confirmed (`cargo check -p wos-server` + `cargo nextest run -p wos-conformance` green on 2026-04-24) that wos-server remains type-compatible with current wos-core / wos-runtime types and passes all existing conformance fixtures, including K-DET-001 Facts-tier snapshot.

**Framing consequence:** Every "build attestation" / "build explanation rendering" / "build identity proofing" concern the enterprise gap docs flag as a DocuSign-competitive requirement is a **seam composition** problem, not a net-new server module. Four of five seams are now composed (2026-04-25); the remaining one (`ExternalService`) is the integration-dispatch gap (TODO WS-028).

---

## Kernel layer

Spec: `specs/kernel/spec.md` — the authoritative WOS Kernel Specification. Schema: `schemas/wos-workflow.schema.json`.

> Per ADR 0076, kernel is the merged author-time envelope; governance, agents, signature, advanced, custody, and assurance are embedded blocks of the same `wos-workflow.schema.json`. Spec docs remain separate.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Kernel §2.2 Structural | Parse + validate kernel document | wos-workflow | `POST /api/kernel/validate` | full | Routed through `wos-lint::lint_document` |
| Kernel §2.2 Structural | Round-trip kernel without loss | wos-workflow | `PUT /api/bundles/:url/kernel` | full | Serde preserves all fields |
| Kernel §2.2 Structural | List registered kernels | — | `GET /api/bundles` | full | Bundle = kernel + sidecars |
| Kernel §2.2 Structural | Load kernel document | wos-workflow | `GET /api/bundles/:url/kernel` | full |  |
| Kernel §2.2 Structural | Load kernel + sidecars bundle | — | `GET /api/bundles/:url` | full | Joins all attached sidecars |
| Kernel §3 Actor Model | Actor type resolution | wos-workflow | — | spec-side | Embedded in kernel doc; evaluator uses it internally |
| Kernel §4 Lifecycle | Deterministic event evaluation | wos-workflow | `POST /api/instances/:id/events` | full | Routes through `AppRuntime` → `WosRuntime` |
| Kernel §8 Provenance | Append-only provenance with hash chain | wos-workflow | `GET /api/instances/:id/provenance` | full | `ProvenanceService::prepare_batch` enforces chain on write |
| Kernel §8.2.1 | caseFileSnapshot on determination transitions | wos-workflow + wos-provenance-log | via `POST /api/instances/:id/events` → drain | full | Wired in wos-runtime; oracle test `determination_transition_emits_case_file_snapshot` at `runtime.rs:715`; conformance gate K-DET-001. Confirmed green in pre-flight gate 4 |
| Kernel §8 Provenance | Chain integrity verification | wos-workflow | `GET /api/instances/:id/provenance/verify` | full | `ProvenanceService::verify_chain` wrapped as endpoint (WS-031); returns `{valid, brokenAt}` |
| Kernel §11 Contracts | Contract reference resolution | wos-workflow | — | spec-side | Internal to evaluator |

### correspondence-metadata

Spec: `specs/kernel/correspondence-metadata.md`. Schema: `schemas/sidecars/wos-delivery.schema.json` (`$wosDelivery` embedded block).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Corr §1 Document | Validate correspondence metadata | wos-delivery | `POST /api/lint/document` | full | Generic lint endpoint handles it |
| Corr §1.4 Event model | Correspondence entries as provenance records | wos-delivery | `GET /api/instances/:id/provenance` | full | Captured via normal provenance flow |
| Corr §1.2 Templates | Correspondence template application | — | — | **none** | No endpoint for rendering correspondence from a template. **User value: low** — overlaps with notification-template render; consider deduplicating the two spec-side. |

### custody-hook-encoding

Spec: `specs/kernel/custody-hook-encoding.md`. Schema: `schemas/wos-workflow.schema.json` (`custody` embedded block declares anchoring posture; the four-field Trellis append wire per §1.3 is normative prose + `wos-runtime` `custody` module — standalone `wos-custody-hook-encoding.schema.json` removed per ADR 0076).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Custody §1.2 | One-authored-record → one-append invariant | wos-workflow (`custody`) | runtime custodyHook seam | spec-side | Runtime-internal seam obligation; no HTTP endpoint obligated |
| Custody §1.4 | TypeID format on caseId / recordId | wos-workflow (`custody`) | `POST /api/lint/document` | partial | No dedicated lint rule for TypeID format; schema regex enforces shape only. Grep `crates/wos-lint/src` for `wos-case-typeid` / `wos-record-typeid` returns zero hits |
| Custody §1.6 | wos.* eventType namespace ownership | wos-workflow (`custody`) | `POST /api/lint/document` | spec-side | Schema-enforceable |
| Custody §1.6 | Deterministic dCBOR conversion | wos-workflow (`custody`) | runtime custody emitter | spec-side | Runtime-internal at custodyHook seam |
| Custody §1.7 | Rejection list (NaN / Infinity / ill-formed UTF-8) | wos-workflow (`custody`) | runtime custody emitter | spec-side | Runtime-internal |
| Custody §1.10 | WOS MUST stamp returned canonical_event_hash | — | — | none | Runtime has `apply_custody_receipt` (`runtime.rs:2657-2798`, `provenance.rs:140-156`) which stamps `canonical_event_hash` with idempotent reapply and conflict detection (`CustodyReceiptConflict` error when hashes differ). Tested: stamp-when-absent, idempotent-when-matching, conflict-when-differing. No server HTTP endpoint exposes this capability |
| Custody §1.11 | Surface reconciliation when record admits but posture-transition does not | — | — | none | Observable runtime obligation |
| Custody §3.4 | Byte-authority fixture corpus (record.json / record.dcbor / record.sha256) | — | — | spec-side | Fixture obligation |

---

## Companions — Runtime

Spec: `specs/kernel/spec.md` §11 (Runtime Serialization), §12 (Evaluation Modes), §13 (Formspec Coprocessor), §16 (Host Interfaces) — runtime-companion content absorbed per ADR 0076 D-8 (2026-04-28). Within-section §-references below (e.g. "Runtime §3", "Runtime §4") cite the legacy `specs/companions/runtime.md` anchors retained as redirect stubs during the citation sweep.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Runtime §3.1 CaseInstance | Instance serialization | wos-case-instance | `GET /api/instances/:id` | full |  |
| Runtime §3.3 Instance ops | Create instance | wos-case-instance | `POST /api/instances` | full | `WosRuntime::create_instance` |
| Runtime §3.3 Instance ops | List instances with filters | — | `GET /api/instances` | full | Pagination + status/impact filters |
| Runtime §3.3 Instance ops | Suspend / resume / migrate | wos-case-instance | — | **none** | `AppRuntime` does not expose these; runtime supports them internally. **User value: medium** — rarely used in practice; add when demand surfaces |
| Runtime §3.4 Status transitions | Completed / terminated via kernel events | wos-case-instance | `POST /api/instances/:id/events` | full | Handled by evaluator |
| Runtime §4 Event delivery | Enqueue event | — | `POST /api/instances/:id/events` | full | Queue → drain |
| Runtime §4 Event delivery | Drain event queue | — | `POST /api/instances/:id/drain` | full | `drain_until_idle` |
| Runtime §4.3 Exactly-once | Idempotency on event IDs | — | `POST /api/instances/{id}/events` | partial | HTTP-layer dedup added (WS-032): in-memory `(instance_id, idempotencyToken) → EvaluationResultView` cache (`http/instances.rs:210-217,275-279`). Duplicate requests return cached result. Idempotency IS also implemented for task submission (replay via `ReplayKey`/`ReplayValue`) and `invokeService` via integration-profile step-result replay (oracle `drain_once_consumes_integration_profile_binding_and_replays_persisted_result`). Remaining gap: `drain_once` itself does not dedup on `PendingEvent.idempotency_token` — the HTTP-layer cache is defense-in-depth, not runtime-level dedup. **User value: high** for at-least-once producers |
| Runtime §5 Action execution | onEntry/onExit/transition actions | — | `POST /api/instances/:id/events` | full | Evaluator executes |
| Runtime §5.4 invokeService | Service invocation seam | — | via `runtime/service.rs::EchoExternalService` | **stub** | Server's `EchoExternalService` echoes input. The _runtime's_ integration-profile dispatch is complete — input mapping (FEL expressions), output binding (JSONPath), idempotency key expression, step-result replay with `IdempotencyDedup` provenance, and `ContractValidation` provenance for both request and response contracts. Real dispatch lives in integration profile §3; the server's echo impl is the limiting factor, not the runtime path |
| Runtime §5.5 Contract validation | Formspec validation on task submit | wos-case-instance | `POST /api/tasks/:id/response` | partial | `PolicyLayeredValidator` (WS-026) enforces §15.7 ledger-gating for rights/safety-impacting submits; `PermissiveValidator` is the inner layer. Real `FormspecProcessor` not wired — awaits #43 spec-side closure |
| Runtime §6 Durability | Atomic checkpoint | — | n/a | full | `update_instance_atomic` transactional in SQLite. Drain path follows load-evaluate-save with provenance position counter (`drain.rs:205`); save failure leaves store unchanged (oracle `drain_once_save_failure_leaves_store_unchanged`). Note: `provenance_position` increment has no optimistic concurrency in `RuntimeStore` trait — safe under `Arc<Mutex<WosRuntime>` single-node topology; distributed adapters would need OCC guard |
| Runtime §7 Timers | Timer create / cancel / fire | — | `services/timer_task.rs` polls | partial | Poll walks **all** instances each tick via paginated `list_instances` (same 200-row page clamp); correctness holds for large fleets. **Cost** remains O(instances × ticks) — index or event-driven scheduling is future work |
| Runtime §9 Explanation | Explanation assembly | — | `GET /api/applicant/:id/determination` | partial | `applicant_service` already assembles rules-applied + milestones + AI disclosure for the applicant view. The dedicated `/instances/:id/explain` per Runtime §9.1's deterministic-algorithm contract is missing; due-process delivery (Gov §3.3) flows through the partial surface today. **User value: high** for adverse-decision workflows |
| Runtime §10 Eval modes | Dry-run transitions | — | `GET /api/instances/:id/transitions` | full | Pure kernel walk |
| Runtime §11 Multi-version coexistence | Instances pinned to definition version | — | `GET /api/instances/:id` | full | `definition_version` preserved on row |
| Runtime §12 Host interfaces | Nine DI seams (see DI seam status section above) | — | via `runtime/` + `wos-runtime::store` impls | partial | Eight of nine wired — one stubbed (`ExternalService` echo), four wired-real in 2026-04-25 sprint (`ProvenanceSigner` NoopSigner, `ReportRenderer` JsonRenderer, `AccessControl` RoleBasedAccessControl, `ContractValidator` PolicyLayeredValidator), three wired-real from day one (`InstanceStore`, `DocumentResolver`, `TaskPresenter`), one folded into runtime (`EventQueue`). Remaining stub: `ExternalService` echo → `IntegrationDispatchService` (TODO WS-028) |
| Runtime §Formspec Tasks | Present task | wos-case-instance | `task:assigned` event | full | Socket.IO broadcast |
| Runtime §Formspec Tasks | Persist task draft | — | `POST /api/tasks/:id/draft` | full |  |
| Runtime §Formspec Tasks | Submit task response | — | `POST /api/tasks/:id/response` | full | Returns `Completed`/`Failed`/`Rejected` |
| Runtime §Formspec Tasks | Dismiss task | — | `POST /api/tasks/:id/dismiss` | full | Socket.IO `task:dismissed` |

### Lifecycle Detail Companion

Spec: `specs/kernel/spec.md` §4.6/§4.7 (transition evaluation), §4.8 (parallel execution), §4.14 (history states), §9.5 (compensation), §9.7 (timer lifecycle) — lifecycle-detail content absorbed per ADR 0076 D-8 (2026-04-28). Within-section §-references below (e.g. "Lifecycle §3 history") cite the legacy `specs/companions/lifecycle-detail.md` anchors retained as redirect stubs during the citation sweep.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Lifecycle §2 Transition evaluation | Deterministic algorithm | — | — | spec-side | Internal to evaluator. Conformance tests cover it |
| Lifecycle §3 Parallel regions | Fork / join / synchronization | — | — | spec-side | Evaluator implementation detail |
| Lifecycle §4 History states | Shallow / deep history | — | — | spec-side | Evaluator |
| Lifecycle §5 Compensation | Reverse-order compensation | — | — | spec-side | Evaluator; conformance fixture `K-H-*` series |
| Lifecycle §6 Timers | Timer algorithms | — | `timer_task.rs` | full | Polling-based |

---

## Governance L1 (Workflow Governance Basic)

Spec: `specs/governance/workflow-governance.md`. Schema: `schemas/wos-workflow.schema.json` (`governance` embedded block, `$defs/Governance`).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Gov §3 Due process | Notice template declaration | wos-workflow (`governance.dueProcess`) | `GET /api/governance/:url/policy-versions` | partial | Read-side only; no notice history |
| Gov §3.2 Notice | Render adverse-decision notice | wos-workflow (`governance.dueProcess`) | — | **none** | Closest: `POST /api/notifications/:url/render` but it doesn't carry the due-process semantics (grace period, appeal window). **User value: high** — explicit due-process notice rendering is a legal-sufficiency requirement |
| Gov §3.3 Explanation | Assemble explanation | — | — | **none** | See Runtime §9; duplicated obligation |
| Gov §3.4 Counterfactual | Counterfactual explanation | — | — | **none** | Typically derived from FEL evaluation traces; expensive feature with narrow audience. **User value: medium** — only XAI-serious deployments need this |
| Gov §3.5 Appeal | Record appeal | — | `POST /api/applicant/:id/appeal` | full | Routes through `AppRuntime::enqueue_event` |
| Gov §3.6 Continuation of service | Hold management | — | — | **none** | Holds are stored on `CaseInstance.governance_state.active_holds` but no CRUD endpoint. **User value: medium** — benefits adjudication needs this |
| Gov §4 Review protocols | Two-reviewer / supervisor override | — | — | spec-side | Enforced by kernel actor model + lifecycle actions; no separate endpoint needed |
| Gov §11 Delegation of Authority | List delegations | wos-workflow (`governance`) | `GET /api/governance/:url/delegations` | full |  |
| Gov §11 Delegation of Authority | Create delegation | — | `POST /api/governance/:url/delegations` | full | Supervisor-gated |
| Gov §11 Delegation of Authority | Revoke delegation | — | `DELETE /api/governance/:url/delegations/:id` | full |  |
| Gov §5.4 Assertion gates | Pipeline enumeration | wos-workflow (`governance.assertionLibrary`) | `GET /api/governance/:url/pipelines` | full | Pipelines live under §5 Data Validation Pipelines, not §7 |
| Gov §5.4 Assertion gates | Run pipeline against inputs | — | — | **none** | No `POST /validate-pipeline`. **User value: high** — pipelines are the primary data-validation mechanism for untrusted inputs |
| Gov §7 Quality controls | List quality controls | wos-workflow (`governance`) | `GET /api/governance/:url/quality-controls` | full |  |
| Gov §2.9 Schema upgrade | Named lifecycle operation | — | — | **none** | Migration endpoint missing (`POST /api/instances/:id/migrate`). **User value: medium** — rare outside multi-year workflows |

### Due Process Config (sidecar)

Spec: `specs/governance/due-process-config.md`. Schema: `schemas/wos-workflow.schema.json` (`governance.dueProcess`, `$defs/Governance.properties.dueProcess`).

All rows here are **spec-side** — this document defines the _data shape_ for due-process parameters. Consumed through `GET /api/governance/:url/bundle` or validated via `POST /api/lint/document`. No dedicated endpoints required.

### Policy Parameters (sidecar)

Spec: `specs/governance/policy-parameters.md`. Schema: `schemas/wos-workflow.schema.json` (`governance.policyParameters`).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| PolicyParam §1.3 Date-indexed values | Resolve parameter as-of date | wos-workflow (`governance.policyParameters`) | — | **none** | No `POST /policy/:url/resolve?asOf=…`. Date resolution is the whole point of this sidecar. **User value: high** — every regulation-tracking workflow needs as-of resolution |
| PolicyParam §1.5 Regulatory bindings | List bound regulations | wos-workflow (`governance.policyParameters`) | `GET /api/governance/:url/policy-versions` | partial | Returns the projection but no as-of query |

### Assertion Library

Spec: `specs/governance/assertion-library.md`. Schema: `schemas/wos-workflow.schema.json` (`governance.assertionLibrary`).

All rows are **spec-side** — reusable assertion definitions referenced by governance pipelines. No direct endpoint; served through the bundle read path.

---

## AI Integration (L2)

Spec: `specs/ai/ai-integration.md`. Schema: `schemas/wos-workflow.schema.json` (`agents[]` array + `aiOversight` block).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| AI §3 Agent registration | Register agent | wos-workflow (`agents[]`) | `POST /api/agents` | full | Backed by new `agents` table |
| AI §3 Agent registration | List registered agents | wos-workflow (`agents[]`) | `GET /api/agents?workflowUrl=…` | full |  |
| AI §3 Agent registration | Get agent by id | wos-workflow (`agents[*]`) | `GET /api/agents/:id` | full |  |
| AI §3.5 Trust boundary | Trust boundary declaration | wos-workflow (`agents[]`) | `GET /api/governance/:url/agents` | partial | Read-only projection, doesn't expose boundary details |
| AI §1.5 / Gov §7.2 | Separation of duties (agent must not review own output) | — | `runtime::RoleBasedAccessControl` | full | `RoleBasedAccessControl` (WS-027) rejects self-review on `review:{author_id}`-tagged transitions and honours delegation chains (Gov §6). Wired as default in `AppRuntime::build` (`runtime/mod.rs:73`) |
| AI §4 Deontic Constraints | Enumerate constraints on workflow (permissions / prohibitions / obligations for agents) | wos-workflow (`governance` + `agents[]`) | `GET /api/governance/:url/deontic-constraints` | full | Projected from bundle; shared with governance URL space |
| AI §4 Deontic Constraints | List violations per instance | — | `GET /api/instances/:id/deontic-violations` | full | Filtered provenance view |
| AI §5 Autonomy | Autonomy level cap | wos-workflow (`agents[*]`) | — | partial | Stored on agent row, not enforced on actions |
| AI §5.4 Autonomy Escalation (calibration expiry) | Enforce calibration expiry | wos-workflow (`agents[*]`) | — | **none** | No scheduled check; calibration metadata stored but never consulted. **User value: medium** — safety feature for production agents |
| AI §7 Confidence Framework | Per-session confidence timeline | — | — | **none** | No `GET /api/instances/:id/confidence`. **User value: low-medium** — most deployments log confidence outside the case instance |
| AI §8 Fallback Chains | Active fallback chain | wos-workflow (`agents[]`) | — | **none** | Plan called this out; not implemented. **User value: low** — rarely consumed at runtime |
| AI §agent lifecycle | Lifecycle transitions | — | `POST /api/agents/:id/lifecycle-transition` | full | Typed enum at boundary |
| AI §agent deployment | Canary / shadow | — | `POST /api/agents/:id/canary\|shadow` | partial | Writes deployment state; no traffic-splitting enforcement (belongs at gateway, not server) |
| AI §tool use | Tool invocation authorization | — | `POST /api/agents/:id/tool-invocation-check` | **stub** | Returns `{allowed: status==active && deploymentState==production}` — a reasonable default but not the full spec |

### Agent Config (sidecar)

Spec: `specs/ai/agent-config.md`. Schema: `schemas/wos-workflow.schema.json` (`agents[*]` per-agent declarations).

Largely **spec-side** — endpoint config, credentials refs, model version lists. Consumed through agent registration or bundle loading. Calibration expiry is the only behavioural obligation and is flagged above as "none".

### Drift Monitor (sidecar)

Spec: `specs/ai/drift-monitor.md`. Schema: `schemas/wos-workflow.schema.json` (`agents[*].driftMonitoring`).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Drift §1.3 Monitor metrics | Serve drift report shaped by configured metrics | wos-workflow (`agents[*].driftMonitoring`) | `GET /api/agents/:id/drift` | **stub** | Spec defines metric config shape (PSI / KS / threshold); doesn't obligate the processor to compute. Endpoint returns spec-correct envelope with `psi: null, ks: null`. **User value: medium** — real impls have an external detector write reports; suggested follow-up: add a write-side `POST /api/agents/:id/drift` so the GET serves the most-recent externally-produced report |
| Drift §1.5 Deployment sequence | Canary / shadow gating on drift | — | — | spec-side | Enforced at gateway, not server |

---

## Advanced Governance (L3)

Spec: `specs/advanced/advanced-governance.md`. Schema: `schemas/wos-workflow.schema.json` (`advanced` embedded block).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Advanced §3 Equity guardrails | Evaluate equity over window | wos-workflow (`advanced.equity`) | `POST /api/equity/evaluate` | partial | Real group-by runs over **all** instances for `workflow_url` via `list_instances_all_pages`; `outcomePredicate: Some(_)` returns **400** until implemented. Result shape is spec-correct. **User value: high** — main equity observable |
| Advanced §3.3 Async evaluation | Scheduled equity runs | wos-workflow (`advanced.equity`) | — | spec-side | Belongs to a scheduler, not the server |
| Advanced §4 Constraint zones | List zones on workflow | wos-workflow (`advanced`) | `GET /api/governance/:url/constraint-zones` | full | Projected from sidecar |
| Advanced §4.4 Relation evaluation | Compute DCR marking → valid next actions | — | `GET /api/instances/:id/constraint-zones/:zone/valid-actions` | **stub** | Returns declared activities; real marking evaluation against provenance not implemented. **User value: medium** — DCR-style case management is niche today |
| Advanced §5 Multi-step sessions | Session start / continue / complete with cumulative-confidence gating | — | — | **none** | §5.4 specifies cumulative-confidence product across DAG steps with intervention-point checkpoints — distinct from kernel compound states (which have no confidence semantics). **User value: medium** — narrow consumer set (multi-step LLM reasoning chains) |
| Advanced §8 Verifiable constraints | SMT verification | wos-provenance-log | `POST /api/verification/verify` | **stub** | Returns `inconclusive` for every constraint. Real proofs require `WOS_SMT=z3`. Shape is spec-correct — consumers can integrate today |
| Advanced §6 Tool use governance | Tool invocation gating | — | `POST /api/agents/:id/tool-invocation-check` | **stub** | Shared with AI §tool use |
| Advanced §7 Agent lifecycle | State machine transitions | — | `POST /api/agents/:id/lifecycle-transition` | full | Shared with AI §agent lifecycle |
| Advanced §9 Calibration | Recalibration triggers | wos-workflow (`agents[*]`) | — | **none** | See AI §5.4 |
| Advanced §11 (Shadow Mode) | Agent shadow deployment | — | `POST /api/agents/:id/shadow` | partial | Shared with AI; §11 covers Shadow Mode + Circuit Breaker as one combined section |
| Advanced §11 (Circuit Breaker) | Agent-level breaker (errorRateThreshold / cooldownDuration / closed-open-half-open) | — | — | **none** | Agent-semantic — error rate of agent invocations feeds agent lifecycle state via `lifecycleHook`. Distinct from network-layer breakers a service mesh provides. **User value: medium** — standalone-agent deployments need it |

### Verification Report (sidecar)

Spec: `specs/advanced/verification-report.md`. Schema: `schemas/wos-provenance-log.schema.json` (runtime verification certificates).

**Spec-side** document — the output envelope of a verification run. Consumed via `POST /api/verification/verify` response and `GET /api/governance/:url/verification-report` projection.

### Equity Config (sidecar)

Spec: `specs/advanced/equity-config.md`. Schema: `schemas/wos-workflow.schema.json` (`advanced.equity`).

**Spec-side** document defining protected categories, disparity methods, schedule. Consumed via `GET /api/governance/:url/equity-config` (already implemented).

---

## Assurance

Spec: `specs/assurance/assurance.md`. Schema: `schemas/wos-workflow.schema.json` (`assurance` embedded block; former `wos-assurance.schema.json` merged per ADR 0076).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Assurance §2.1 Taxonomy | L1–L4 assurance levels | wos-workflow (`assurance`) | — | full | Enforced at type level via `AssuranceLevel` enum |
| Assurance §2.3 Upgrade facts | Record assurance upgrade | wos-workflow (`assurance`) | `POST /api/instances/:id/identity-facts/:id/upgrade` | full | Forward-only; `upgradedFrom` preserved |
| Assurance §3 Subject continuity | Cross-instance timeline for a subject | wos-workflow (`assurance`) | `GET /api/subjects/:ref/assurance-chain` | partial | Returns ordered facts; continuity-hash validation not implemented. **User value: high** — continuity is the main assurance observable |
| Assurance §4 Invariant 6 | Assurance level ≠ disclosure posture | wos-workflow (`assurance`) | — | full | Enforced at type level (two independent enums on request) |
| Assurance §5 Attestation | Provider-neutral attestation | — | — | **none** | No `/api/instances/:id/identity-facts/:id/attest`. **User value: medium** — legal-sufficiency deployments need attestation; low-assurance deployments don't |
| Assurance §6 Legal sufficiency disclosure | Disclosure metadata on exports when claims are made | — | — | **none** | §6.1 obligates a disclosure of which conditions an implementation relies on (process, signature semantics, records-management, applicable law) **when** the implementation makes claims about evidentiary weight. Server-side exports today make no such claims and therefore are technically compliant; if/when we add attestation (§5), exports must carry the disclosure. **User value: medium** — gating the attestation work, not currently blocking |
| Assurance §custody | Custody posture declaration | — | — | **none** | Plan had `GET /api/instances/:id/custody-posture` as a stretch. **User value: medium** — specialised to chain-of-custody workflows |

---

## Integration Profile

Spec: `specs/kernel/spec.md` §9.2 (the merged invokeService binding surface absorbing prior Integration Profile §3-§7/§9 content per ADR 0076 D-8 + 2026-04-28). Vendor-adapter content (§10/§11) lives in `docs/adapters/integration-extensions.md`. Schema: `schemas/wos-workflow.schema.json` (`bindings` field). Within-section §-references below cite the legacy `specs/profiles/integration.md` anchors retained as redirect stubs.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Integ §3.1 Overview | Load integration profile | wos-workflow (`bindings`) | `GET /api/integration/:url/profile` | full |  |
| Integ §3.4 Request-response | HTTP invocation | — | `POST /api/integration/:url/invoke/:binding` | **stub** | Echoes binding + inputs |
| Integ §3.5 Arazzo sequence | Multi-step orchestration | — | `POST /api/integration/:url/invoke/:binding` | **stub** | Same endpoint; real sequencing not wired |
| Integ §3.6 Tool binding | CWL-informed tool call | — | `POST /api/integration/:url/invoke/:binding` | **stub** | Same |
| Integ §3.7 Event binding | Emit CloudEvent | — | `task:assigned` / Socket.IO | partial | Only task events flow; generic event-emit not wired |
| Integ §5.3 Inbound event processing | Accept CloudEvent | — | `POST /api/events/inbound` | full | Idempotent via `integration_inbound` table; `validate_ingress` enforced |
| Integ §5.4 Idempotent consumption | Dedupe on CloudEvent id | — | included above | full | Duplicate IDs return `deduplicated: true` |
| Integ §6 Correlation | Correlation tokens | — | — | **none** | Callback correlation is the one real gap in this layer. **User value: high** — any meaningful request/response with async completion needs it |
| Integ §7 Idempotency keys | Idempotency on outbound invocations | — | in `submit_task_response` | partial | Task-binding layer only; integration-binding layer doesn't honour idempotency tokens |
| Integ §8 Policy engine bridge | XACML / OPA / Cedar decisions | — | — | **none** | Plan had `POST /api/policy/evaluate`; not yet implemented. **User value: medium** — real deployments use OPA as a sidecar; inlining adds little |

---

## Semantic Profile

Spec: `specs/profiles/semantic.md`. Schema: `schemas/sidecars/wos-ontology-alignment.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Semantic §2 Doc structure | Load semantic profile | wos-ontology-alignment | `GET /api/bundles/:url` | partial | Served as part of the bundle; no dedicated `/semantic/:url` projection |
| Semantic §3 JSON-LD context | Serve JSON-LD context | — | — | **none** | Plan had `GET /api/semantic/jsonld-context`. **User value: medium** — needed by RDF consumers but can be shipped as static file |
| Semantic §4 SHACL | SHACL validation | — | — | **none** | Requires a SHACL engine. **User value: medium** — overlaps with our lint surface; real RDF shops want this |
| Semantic §5 PROV-O mapping | Export provenance as PROV-O | — | `GET /api/instances/:id/provenance/export?format=prov-o` | full |  |
| Semantic §5 XES mapping | Export as XES | — | `GET /api/instances/:id/provenance/export?format=xes` | full |  |
| Semantic §5 OCEL mapping | Export as OCEL | — | `GET /api/instances/:id/provenance/export?format=ocel` | full |  |
| Semantic §6 SPARQL queries | SPARQL query endpoint | — | — | **none** | Plan flagged as stub with `WOS_TRIPLESTORE=none` returning 501. Not implemented. **User value: low-medium** — export-to-triplestore is the usual flow; in-server SPARQL is convenient but not load-bearing |

---

## Signature Profile

Spec: `specs/profiles/signature.md`. Schema: `schemas/wos-workflow.schema.json` (`signature` embedded block, `$defs/Signature`).

_Product shortcuts may exist only as workflow-lite paths over the same `SignatureAffirmation` semantics; no second meaning of "signed." (Signature shortcut rule, `wos-spec/CLAUDE.md`.)_

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Signature §2 | Serve signature profile | wos-workflow (`signature`) | `GET /api/bundles/:url` | full | Part of bundle join |
| Signature §2.5 | Consent-evidence capture on submit | wos-workflow (`signature`) | `POST /api/tasks/:id/response` | stub | Rides `ContractValidator` → `PermissiveValidator` accepts without consent-evidence check |
| Signature §2.7 | Document-binding (digest) on submit | wos-workflow (`signature`) | `POST /api/tasks/:id/response` | stub | Same `ContractValidator` seam |
| Signature §2.8 | SignatureAffirmation emission | — | runtime emission; read via `GET /api/instances/:id/provenance` (filter on kind) | partial | Emission wired at `crates/wos-runtime/src/runtime/signature.rs:447` + `tasks.rs:364`; no dedicated `GET /signature-affirmations` read surface. Pre-flight gate 3+4 proves this is current |
| Signature §2.9 | Reassignment MUST NOT erase accountability for original assignment | — | — | none | No dedicated reassignment endpoint; provenance trail observability unclear |
| Signature §2.10 | Witness / notary in-person authentication method | wos-workflow (`signature`) | `POST /api/tasks/:id/response` | stub | `ContractValidator` gate |
| Signature §signer-roles | Signer role declaration | wos-workflow (`signature`) | via bundle | spec-side | |
| Signature §signing-flow | Signing flow declaration | wos-workflow (`signature`) | via bundle | spec-side | |
| Signature §identity-binding | Identity-binding policy hook | wos-workflow (`signature`) | via bundle | spec-side | Consumer-provided policy |

---

## Extension Registry

Spec: `specs/registry/extension-registry.md`. Schema: `schemas/wos-tooling.schema.json` (`kind: "extensionRegistry"` sub-view).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Registry §2 | Load extension registry | wos-tooling (`kind: "extensionRegistry"`) | `GET /api/bundles/:url` | full | Bundle join |
| Registry §4.3 | Reject runtime registry conflict on composition | — | `POST /api/lint/document` | none | Grep `crates/wos-lint/src` for `composition`; no lint rule. Multi-registry conflict obligation |
| Registry §5.3 | Reject replacedBy cycles | — | `POST /api/lint/document` | none | Grep `crates/wos-lint/src` for `replacedBy`; no lint rule. Partial if lint rule exists, none otherwise |
| Registry §6.1 (per-obligation) | Enumerated MUST statements | — | — | classify per row | Six MUST behaviours: (1) reject invalid registry doc — none; (2) reject retired entry — none; (3) reject replacedBy cycles — none; (4) reject conflicting composition — none; (5) opaque seam identifiers — spec-side; (6) preserve x- keys — spec-side |

---

## Sidecars

### Business Calendar

Spec: `specs/sidecars/business-calendar.md`. Schema: `schemas/sidecars/wos-delivery.schema.json` (`calendar` embedded block).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| BusCal §compute | Snap-forward deadline | wos-delivery (`calendar`) | `POST /api/calendar/:url/compute-deadline` | full | Delegates to `wos_core::business_calendar::next_business_moment` |
| BusCal §business-days-between | Business-day delta | — | — | partial | Plan had `POST /api/calendar/:url/business-days-between`; the spec only obligates the deadline op, so this is optional. **User value: low** — trivial helper; clients can compose two `compute-deadline` calls |

### Notification Template

Spec: `specs/sidecars/notification-template.md`. Schema: `schemas/sidecars/wos-delivery.schema.json` (`notifications` embedded block).

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Notif §render | Template render with placeholder substitution | wos-delivery (`notifications`) | `POST /api/notifications/:url/render` | full | `${var}` + dotted `${nested.path}` supported |
| Notif §channels | Per-channel dispatch | — | — | spec-side | Delivery is out of scope for the server; template render returns declared channel list |

---

## User-value critique

Rows where the spec obligates a surface but the user value is questionable, and what we recommend.

### Low value — defer

1. **Semantic §6 SPARQL in-server.** In-process SPARQL requires an embedded triplestore and doesn't pay off for the usual "export → external tool" workflow. Users who need SPARQL have Apache Jena / Oxigraph already. Recommend: keep as optional feature behind `triplestore-oxigraph`; don't mark as MUST.
2. **AI §6 Fallback chain retrieval.** Fallback chains are typically driven by the agent registry at runtime, not queried by clients. The endpoint would have no real consumer. Recommend: leave as spec-side data on the AI integration doc; no dedicated endpoint.
3. **Runtime §Suspend / resume.** No evidence anyone uses these in practice. Recommend: lazy-implement when a real case comes in; don't build eagerly.
4. **Kernel §Correspondence template application.** Overlaps semantically with Notification template render (both shape outbound content). Recommend: clarify the boundary in the specs (correspondence = audit trail of _received_ communication, notification = _outbound_ content) — not a deletion case, but the surface area suggests merging or sharper delineation.

### High value — the real gaps

Rows where the spec is right and the missing surface is a concrete user-value block:

- **Runtime §9 / Gov §3.3 Explanation assembly.** Runtime §9 specifies the deterministic algorithm; Gov §3.3 specifies what must be delivered (individualised / categorical / aggregate by impact level). The two are a contract+implementation pair, not duplication. Server provides a _partial_ surface today via the applicant-determination view; the dedicated `/instances/:id/explain` per Runtime §9.1 is missing. `ReportRenderer` seam is now wired (WS-025) — blocked only on Runtime §9.1 algorithm.
- **Gov §5.4 Pipeline validation.** Assertion-gate pipelines have no run-against-inputs endpoint.
- **PolicyParam §1.3 As-of resolution.** Date-indexed policy resolution is the _whole point_ of the policy-parameters sidecar and has no endpoint.
- **Integ §6 Correlation.** Async request/response (most interesting integrations) need correlation tokens; currently absent.
- **Assurance §3 Subject continuity.** Continuity-hash validation absent; chain endpoint exists but doesn't prove the chain.

### Spec smells

Ambiguities worth flagging on the spec side, but **not** grounds for unilateral server-side dismissal:

1. **Overlap between `correspondence-metadata` and `notification-template`.** Both define outbound content shapes. The boundary should be tightened in the specs — recommend an editorial pass, not a deletion.
2. **`assertion-library.md`** defines a reusable assertion shape but no spec actually declares how to _invoke_ one. The `invokeAssertion` obligation is missing from `workflow-governance.md` §5.4. Recommend adding the invoke binding spec-side.

The previous version of this document also flagged Advanced §5 multi-step sessions, Advanced §11 circuit breakers, and Drift §1.3 as over-reach. Re-reading the specs more carefully:

- **Multi-step sessions (Advanced §5)** specify cumulative-confidence gating across DAG steps with intervention-point checkpoints — distinct from kernel compound states (which have no confidence semantics). Different abstractions; both have a place.
- **Circuit breakers (Advanced §11)** are agent-semantic (error rate of agent invocations feeding agent lifecycle state), not network-semantic. Service mesh breakers don't know what an agent's error predicate is. Defer if there's no consumer, but don't treat as over-reach.
- **Drift §1.3** only defines the _config shape_ for drift metrics; nothing in the spec obligates the processor to compute them. The earlier "the processor structurally can't do this" critique was solving a non-problem.

---

## Asymmetries

### Schemas without specs

None — every schema under `/schemas` has a matching spec.

### Specs without schemas

None — every spec under `/specs` has a matching schema.

### Specs that define a shape but imply no server surface

These are document-shape specs that are (correctly) not exposed as resources; they flow through the generic validation and bundle-read surfaces. Per ADR 0076 schema consolidation, all formerly-separate per-block schemas are now embedded blocks or embedded blocks within the 6-schema canonical family; the authoritative canonical location is listed for each.

- `schemas/sidecars/wos-delivery.schema.json` (`$wosDelivery`) — correspondence metadata validated via `/lint/document`; calendar + notifications via bundle read
- `schemas/wos-workflow.schema.json` (`governance.dueProcess`) — bundle projection
- `schemas/wos-workflow.schema.json` (`governance.policyParameters`) — bundle projection
- `schemas/wos-workflow.schema.json` (`governance.assertionLibrary`) — bundle projection
- `schemas/wos-workflow.schema.json` (`agents[*]`) — bundle projection + agent registration
- `schemas/wos-workflow.schema.json` (`agents[*].driftMonitoring`) — bundle projection
- `schemas/wos-provenance-log.schema.json` — output envelope from `/verification/verify`
- `schemas/wos-workflow.schema.json` (`advanced.equity`) — bundle projection
- `schemas/wos-workflow.schema.json` (`assurance`) — embedded in identity facts

### Schema-slug asymmetries (authoring smell, not correctness gap)

Tooling artifacts (formerly standalone schemas, no governing spec) live as `$views` of the merged tooling schema at `schemas/wos-tooling.schema.json` per ADR 0076 D-5: `$views/conformanceTrace`, `$views/lintDiagnostic`, `$views/mcpToolCatalog`, `$views/synthTrace`, `$views/extensionRegistry`. Documents in any of these shapes carry the `$wosTooling` envelope marker; the legacy slugs (`conformance-trace`, `wos-lint-diagnostic`, `wos-mcp-tools`, `wos-synth-trace`, `wos-extension-registry`) and their per-shape markers are retired.

Provenance: `wos-provenance-log.schema.json` is the top-level runtime artifact for audit logs; record-shape `$def`s (`FactsTierRecord`, `CapabilityInvocationRecord`, `CaseFileSnapshot`, etc.) are promoted into `wos-workflow.schema.json` and `$ref`'d across the cross-schema registry. The legacy `wos-provenance-record.schema.json` is gone — its content split between the workflow `$defs` (record shapes) and the provenance-log envelope (export wrapper).

Post-ADR-0076 slug notes: legacy per-block slugs (`wos-assertion-gate`, `wos-integration-profile`, `wos-semantic-profile`, `wos-advanced`, `wos-equity`, `wos-due-process`, `wos-workflow-governance`, `wos-ai-integration`, `wos-agent-config`, `wos-drift-monitor`, `wos-signature-profile`, `wos-extension-registry`) are absorbed into `wos-workflow` or the appropriate sidecar. Remaining spec↔slug asymmetries on canonical schemas: `wos-case-instance` ↔ `runtime`, `wos-assurance` ↔ `assurance`.

Recommendation: regenerate parity-check tooling against the 6-schema canonical family; track as candidate for `TODO.md §4.7`.

---

## Gap ranking — priority × complexity × tech-debt burden

Every gap scored on three independent axes. **Priority** is user impact × urgency. **Complexity** is effort to close. **Debt burden** is the compounding cost of deferring — an isolated addition scores 1; a gap where every additional day spreads workarounds across the codebase or ossifies breaking-change exposure scores 5.

**Rubric.**

- **Priority (P)**: 5 = blocks conformance or legal-sufficiency gate · 3 = real consumer asks exist · 1 = spec curiosity.
- **Complexity (C)**: 1 = <1 hr · 2 = <1 day · 3 = 1-2 days · 4 = 3-5 days · 5 = multi-week or external adapter.
- **Debt burden (D)**: 5 = every week of delay compounds (consumers build on absence, retrofit is breaking) · 3 = downstream reinvention starts · 1 = pure addition.

### Ranked table

Sorted by ROI (= P × D / C; higher is more value-per-effort). **DI seam sprint applied** (2026-04-25): six rows closed — `ProvenanceSigner` wiring, `ReportRenderer` wiring, `PolicyLayeredValidator`, `RoleBasedAccessControl`, chain-integrity verify endpoint, event-idempotency HTTP-layer dedup. Table drops from 22→16 rows. Remaining items unchanged.

| Gap | Spec § | P | C | D | ROI | Shape |
|---|---|---|---|---|---|---|
| `/instances/:id/explain` handler | Runtime §9 / Gov §3.3 | 5 | 2 | 5 | 12.5 | ~50 lines — `ReportRenderer` seam now wired (WS-025); blocked on Runtime §9.1 deterministic algorithm landing in `wos-runtime` (TODO #2) |
| Legal-sufficiency disclosure on exports | Assurance §6 | 2 | 1 | 4 | 8.0 | One-liner in `semantic_service.rs`. _P lowered from 5 (2026-04-24): §6.1 obligation is conditional on the implementation making evidentiary claims; today server makes none, so technically compliant. Re-score to P=5 when attestation surface (§5) ships._ |
| Pipeline validation endpoint | Gov §5.4 | 4 | 3 | 5 | 6.7 | Depends on TODO #38. _Assertion Library spec-side protocol landed (§4.4); TODO #38 G-064 resolution lint still open; complexity unchanged._ |
| `IntegrationDispatchService` + correlation tokens | Integ §3, §6 | 4 | 3 | 5 | 6.7 | Replace `EchoExternalService` — last stubbed DI seam |
| Policy-parameters as-of resolution | PolicyParam §1.3 | 4 | 2 | 3 | 6.0 | Date-indexed lookup |
| Hold create / release CRUD | Gov §3.6 | 3 | 2 | 3 | 4.5 |  |
| Subject continuity-hash validation | Assurance §3 | 3 | 2 | 2 | 3.0 | Extends existing `/assurance-chain` |
| Calibration expiry enforcement | AI §5.4 | 3 | 2 | 2 | 3.0 | Background job |
| Real drift detection (write-side) | Drift §1.3 | 3 | 5 | 4 | 2.4 | `POST /agents/:id/drift` for external detectors |
| JSON-LD context endpoint | Semantic §3 | 2 | 1 | 1 | 2.0 | Static serve |
| Multi-step sessions | Advanced §5 | 2 | 3 | 3 | 2.0 | Defer until consumer demand |
| SHACL validation | Semantic §4 | 2 | 3 | 2 | 1.3 | Optional feature |
| Counterfactual explanation | Gov §3.4 | 2 | 4 | 2 | 1.0 | Depends on FEL trace |
| Migration endpoint | Gov §2.9 | 2 | 3 | 1 | 0.7 | Wrap `WosRuntime::migrate` |
| Agent circuit breakers | Advanced §11 | 2 | 3 | 1 | 0.7 | Defer |
| Real SMT verification | Advanced §8 | 2 | 5 | 1 | 0.4 | External adapter; stub shape durable |
| SPARQL in-server | Semantic §6 | 1 | 5 | 1 | 0.2 | Defer indefinitely |

**Rows closed in 2026-04-25 sprint (moved from ranked table to completed):**

- Wire `ProvenanceSigner` seam (was ROI 25.0) → `NoopSigner` default wired (WS-024)
- Wire `ReportRenderer` seam (was ROI 25.0) → `JsonRenderer` default wired (WS-025)
- `PolicyLayeredValidator` (was ROI 12.5) → §15.7 ledger-gating wired (WS-026)
- `RoleBasedAccessControl` (was ROI 12.5) → separation-of-duties wired (WS-027)
- Chain-integrity verify endpoint (was ROI 8.0) → `GET /provenance/verify` wired (WS-031)
- Event idempotency on `POST /events` (was ROI 8.0) → HTTP-layer dedup cache wired (WS-032)

**Rows dropped from prior ranking:**

- **"Provenance attestation" (was ROI 2.0).** Not a server gap. The `ProvenanceSigner` seam exists in `wos-core::traits`; once wired (top row of new ranking), consumers inject whatever signer they have — Ed25519 local key, HSM, cloud KMS, or the Formspec Respondent Ledger (which provides the cryptographic checkpoint primitive per Formspec S13). The server's responsibility is seam composition, not attestation primitives.

### Top by debt burden (D = 5)

Under the DI framing, every D=5 row is about **seam locks**: the longer a stubbed seam stays stubbed, the more consumers depend on the stub behaviour and the more breaking any tightening becomes.

Four D=5 items were closed in the 2026-04-25 sprint (WS-024–027, WS-031–032). Remaining D≥4 items:

1. **`/instances/:id/explain` handler (D=5).** `ReportRenderer` seam is now wired; handler blocked only on Runtime §9.1 deterministic algorithm (TODO #2). Each day without the endpoint, adverse-decision workflows use the partial `applicant_service` surface.
2. **Pipeline validation (Gov §5.4, D=5).** Without a server-side gate evaluator, handlers hand-roll assertion logic.
3. **Integration correlation (Integ §6, D=5).** `ExternalService::invoke` is the last stubbed seam; adding correlation later is a trait-signature break. This is now the top remaining compounding-cost gap.
4. **Real drift detection write-side (Drift §1.3, D=4).** External detectors have nowhere to persist reports; each detector builds its own persistence.

### Actionable items → [`TODO.md`](TODO.md)

The per-item "do this" checklist that used to live here (Day 1 / Day 2 / Day 3 / Week 2 / demand-gated / deferred-indefinitely) has moved to [`TODO.md`](TODO.md) §`Spec surface — DI seams and endpoints` so a single file owns "what to do next." PARITY keeps the **status matrix**, **user-value critique**, **ranked table**, **top-by-debt-burden** rollup, and **compounding-costs rationale** — i.e. the analysis that justifies the prioritisation. TODO keeps the entries.

Reading order: ranked table below → click through to [`TODO.md`](TODO.md) entry → work starts. A stubbed `Decision matrix` cross-tabulation previously sat here; it restated the ranked table without adding information and was dropped in the 2026-04-24 PARITY→TODO migration.

### The compounding costs of deferral (DI seams)

Under the DI framing, the compounding costs cluster around seam state. A stubbed seam is worse than an unwired one: consumers build on the stub's behaviour; retrofitting the real impl then breaks them.

Four seams were unwired or stubbed and are now wired-real (2026-04-25 sprint):

1. ~~**Unwired `ProvenanceSigner` seam.**~~ **Closed (WS-024).** `NoopSigner` wired as default; `Ed25519FileKeySigner` available behind feature flag.
2. ~~**Unwired `ReportRenderer` seam.**~~ **Closed (WS-025).** `JsonRenderer` wired as default; unblocks `/explain` endpoint.
3. ~~**Stubbed `PolicyLayeredValidator` (§15.7 ledger-gating).**~~ **Closed (WS-026).** `PolicyLayeredValidator` enforces §15.7 rights/safety-impacting ledger-gating; `PermissiveValidator` demoted to inner test double.
4. ~~**Stubbed `RoleBasedAccessControl` (separation-of-duties).**~~ **Closed (WS-027).** Self-review rejected; delegation chains honoured.

Remaining compounding-cost items:

5. **Stubbed `IntegrationDispatchService` (`EchoExternalService`).** `ExternalService::invoke` signature doesn't model correlation tokens; adapters written against the current shape break on the real impl. Now the **top remaining compounding-cost gap**.

6. **Pipeline assertion scatter.** Without a server-side `validate-pipeline` endpoint, handlers hand-code assertion checks.

The remaining gaps are **additive** — deferring them creates no compounding cost. They're pure feature work that can happen whenever a concrete consumer arrives.

---

## Notes for future readers

- The "stub" status is load-bearing: consumers can integrate today against spec-correct response shapes. Swapping to real adapters (Z3 for SMT, a real drift detector, a real SHACL engine) doesn't change the wire protocol. Stubs are a feature, not a compromise, for a reference implementation.
- The server intentionally does NOT implement the Lifecycle Detail Companion as HTTP endpoints — it's an internal algorithm reference. Conformance tests cover it.
- Every sidecar that's marked entirely "spec-side" (due-process-config, policy-parameters, assertion-library, agent-config, verification-report, equity-config) is served through the existing `/api/bundles/:url` bundle join. Adding dedicated endpoints would fragment the surface.
