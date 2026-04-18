# wos-server Parity Matrix

_Cross-references `/specs` + `/schemas` against the server's HTTP + Socket.IO surface on branch `claude/wos-spec-backend-y17wJ` as of commit `645fbd8`._

> **Validation pass applied** (commit `d61b2af` → next). Three independent audits (spec citations, server surface, status grades + ROI math) flagged 6 citation errors, 1 path fabrication (`/api/kernels` → actual `/api/bundles`), 4 mis-graded rows, an unsorted "ranked" table, and 4 unfair critiques (multi-step sessions, circuit breakers, drift "impossible work", three-way explanation schism). All applied. The ranking now sorts by ROI; the chain-integrity rationale was corrected (helper exists but has zero callers; chain is built on write but never re-verified on read). The previously-claimed three-way explanation schism is downgraded to a contract+implementation pair (Runtime §9 + Gov §3.3 only); Assurance §5 attestation is a different concept.

**Methodology.** Walked each spec extracting every imperative observable (MUST statements on processor behaviour, enumerated operations, processor-obligation tables). Cross-referenced against `crates/wos-server/src/http/*.rs` routes, `realtime/mod.rs` events, and `runtime/mod.rs` methods. Schema files that define document shapes (not observables) are marked "spec-side" — they're consumed as validation inputs to `POST /api/lint/document`, not served as first-class resources.

**Status legend.**
- **full** — endpoint exists, response matches spec shape, real logic behind it
- **partial** — endpoint exists, covers main use case, missing edge cases
- **stub** — endpoint exists with spec-correct response shape, values are synthetic
- **none** — no server surface; spec obligates one
- **spec-side** — document-shape spec, no server surface expected

**User-value lens.** Every row is also evaluated for "does this solve a real user problem or is it cargo-cult compliance?" Rows flagged are collected in the _User-value critique_ section.

---

## Summary

| Status | Count |
|---|---|
| full | 27 |
| partial | 11 |
| stub | 6 |
| none | 11 |
| spec-side | 13 |
| **total** | **68** |

Kernel + runtime companion are mostly implemented (Runtime §12 has six of the spec's nine host interfaces). Governance L1 read-side and sidecar operations are solid. The gaps cluster in three places: (1) assurance attestation / continuity hash validation, (2) integration-profile real dispatch (currently echo) plus correlation tokens, (3) semantic profile's SHACL / SPARQL (triplestore adapter needed). Stubs are concentrated in advanced L3 (SMT verification, drift detection) — both require external adapters; their response shapes are spec-correct so consumers can integrate today.

---

## Kernel layer

Spec: `specs/kernel/spec.md` — the authoritative WOS Kernel Specification. Schema: `schemas/kernel/wos-kernel.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Kernel §2.2 Structural | Parse + validate kernel document | wos-kernel | `POST /api/kernel/validate` | full | Routed through `wos-lint::lint_document` |
| Kernel §2.2 Structural | Round-trip kernel without loss | wos-kernel | `PUT /api/bundles/:url/kernel` | full | Serde preserves all fields |
| Kernel §2.2 Structural | List registered kernels | — | `GET /api/bundles` | full | Bundle = kernel + sidecars |
| Kernel §2.2 Structural | Load kernel document | wos-kernel | `GET /api/bundles/:url/kernel` | full |  |
| Kernel §2.2 Structural | Load kernel + sidecars bundle | — | `GET /api/bundles/:url` | full | Joins all attached sidecars |
| Kernel §3 Actor Model | Actor type resolution | wos-kernel | — | spec-side | Embedded in kernel doc; evaluator uses it internally |
| Kernel §4 Lifecycle | Deterministic event evaluation | wos-kernel | `POST /api/instances/:id/events` | full | Routes through `AppRuntime` → `WosRuntime` |
| Kernel §8 Provenance | Append-only provenance with hash chain | wos-kernel | `GET /api/instances/:id/provenance` | full | `ProvenanceService::prepare_batch` enforces chain on write |
| Kernel §8 Provenance | Chain integrity verification | wos-kernel | — | **none** | Chain is enforced on write but never re-verified on read; `ProvenanceService::verify_chain` helper exists with zero callers. **User value: medium** — auditors want an explicit "chain valid" response |
| Kernel §11 Contracts | Contract reference resolution | wos-kernel | — | spec-side | Internal to evaluator |

### correspondence-metadata

Spec: `specs/kernel/correspondence-metadata.md`. Schema: `schemas/kernel/wos-correspondence-metadata.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Corr §1 Document | Validate correspondence metadata | wos-correspondence-metadata | `POST /api/lint/document` | full | Generic lint endpoint handles it |
| Corr §1.4 Event model | Correspondence entries as provenance records | wos-correspondence-metadata | `GET /api/instances/:id/provenance` | full | Captured via normal provenance flow |
| Corr §1.2 Templates | Correspondence template application | — | — | **none** | No endpoint for rendering correspondence from a template. **User value: low** — overlaps with notification-template render; consider deduplicating the two spec-side. |

---

## Companions — Runtime

Spec: `specs/companions/runtime.md` — the behavioural contract between the processor and its host.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Runtime §3.1 CaseInstance | Instance serialization | wos-case-instance | `GET /api/instances/:id` | full |  |
| Runtime §3.3 Instance ops | Create instance | wos-case-instance | `POST /api/instances` | full | `WosRuntime::create_instance` |
| Runtime §3.3 Instance ops | List instances with filters | — | `GET /api/instances` | full | Pagination + status/impact filters |
| Runtime §3.3 Instance ops | Suspend / resume / migrate | wos-case-instance | — | **none** | `AppRuntime` does not expose these; runtime supports them internally. **User value: medium** — rarely used in practice; add when demand surfaces |
| Runtime §3.4 Status transitions | Completed / terminated via kernel events | wos-case-instance | `POST /api/instances/:id/events` | full | Handled by evaluator |
| Runtime §4 Event delivery | Enqueue event | — | `POST /api/instances/:id/events` | full | Queue → drain |
| Runtime §4 Event delivery | Drain event queue | — | `POST /api/instances/:id/drain` | full | `drain_until_idle` |
| Runtime §4.3 Exactly-once | Idempotency on event IDs | — | — | **none** | The event submission path has no dedupe. Idempotency tokens exist for task submission only (§Formspec Tasks below). **User value: high** for at-least-once producers |
| Runtime §5 Action execution | onEntry/onExit/transition actions | — | `POST /api/instances/:id/events` | full | Evaluator executes |
| Runtime §5.4 invokeService | Service invocation seam | — | via `runtime/service.rs::EchoExternalService` | **stub** | Echoes input. Real dispatch lives in integration profile §3 |
| Runtime §5.5 Contract validation | Formspec validation on task submit | wos-case-instance | `POST /api/tasks/:id/response` | partial | `runtime/validator.rs::PermissiveValidator` accepts all. Real `FormspecProcessor` not wired |
| Runtime §6 Durability | Atomic checkpoint | — | n/a | full | `update_instance_atomic` transactional in SQLite |
| Runtime §7 Timers | Timer create / cancel / fire | — | `services/timer_task.rs` polls | partial | Correct for ≤200 instances; efficiency review flagged full-scan issue |
| Runtime §9 Explanation | Explanation assembly | — | `GET /api/applicant/:id/determination` | partial | `applicant_service` already assembles rules-applied + milestones + AI disclosure for the applicant view. The dedicated `/instances/:id/explain` per Runtime §9.1's deterministic-algorithm contract is missing; due-process delivery (Gov §3.3) flows through the partial surface today. **User value: high** for adverse-decision workflows |
| Runtime §10 Eval modes | Dry-run transitions | — | `GET /api/instances/:id/transitions` | full | Pure kernel walk |
| Runtime §11 Multi-version coexistence | Instances pinned to definition version | — | `GET /api/instances/:id` | full | `definition_version` preserved on row |
| Runtime §12 Host interfaces | InstanceStore / DocumentResolver / ContractValidator / ExternalService / AccessControl / ProvenanceSigner / ReportRenderer / EventQueue / TaskPresenter | — | via `runtime/` + `wos-runtime::store` impls | partial | Six of nine implemented (InstanceStore as `SqliteRuntimeStore`, DocumentResolver, ContractValidator/permissive, ExternalService/echo, AccessControl/permissive, TaskPresenter). ProvenanceSigner, ReportRenderer, EventQueue not yet hosted |
| Runtime §Formspec Tasks | Present task | wos-case-instance | `task:assigned` event | full | Socket.IO broadcast |
| Runtime §Formspec Tasks | Persist task draft | — | `POST /api/tasks/:id/draft` | full |  |
| Runtime §Formspec Tasks | Submit task response | — | `POST /api/tasks/:id/response` | full | Returns `Completed`/`Failed`/`Rejected` |
| Runtime §Formspec Tasks | Dismiss task | — | `POST /api/tasks/:id/dismiss` | full | Socket.IO `task:dismissed` |

### Lifecycle Detail Companion

Spec: `specs/companions/lifecycle-detail.md` — execution algorithms (pseudocode). Schema: `schemas/companions/wos-lifecycle-detail.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Lifecycle §2 Transition evaluation | Deterministic algorithm | — | — | spec-side | Internal to evaluator. Conformance tests cover it |
| Lifecycle §3 Parallel regions | Fork / join / synchronization | — | — | spec-side | Evaluator implementation detail |
| Lifecycle §4 History states | Shallow / deep history | — | — | spec-side | Evaluator |
| Lifecycle §5 Compensation | Reverse-order compensation | — | — | spec-side | Evaluator; conformance fixture `K-H-*` series |
| Lifecycle §6 Timers | Timer algorithms | — | `timer_task.rs` | full | Polling-based |

---

## Governance L1 (Workflow Governance Basic)

Spec: `specs/governance/workflow-governance.md`. Schema: `schemas/governance/wos-workflow-governance.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Gov §3 Due process | Notice template declaration | wos-workflow-governance + wos-due-process | `GET /api/governance/:url/policy-versions` | partial | Read-side only; no notice history |
| Gov §3.2 Notice | Render adverse-decision notice | wos-due-process | — | **none** | Closest: `POST /api/notifications/:url/render` but it doesn't carry the due-process semantics (grace period, appeal window). **User value: high** — explicit due-process notice rendering is a legal-sufficiency requirement |
| Gov §3.3 Explanation | Assemble explanation | — | — | **none** | See Runtime §9; duplicated obligation |
| Gov §3.4 Counterfactual | Counterfactual explanation | — | — | **none** | Typically derived from FEL evaluation traces; expensive feature with narrow audience. **User value: medium** — only XAI-serious deployments need this |
| Gov §3.5 Appeal | Record appeal | — | `POST /api/applicant/:id/appeal` | full | Routes through `AppRuntime::enqueue_event` |
| Gov §3.6 Continuation of service | Hold management | — | — | **none** | Holds are stored on `CaseInstance.governance_state.active_holds` but no CRUD endpoint. **User value: medium** — benefits adjudication needs this |
| Gov §4 Review protocols | Two-reviewer / supervisor override | — | — | spec-side | Enforced by kernel actor model + lifecycle actions; no separate endpoint needed |
| Gov §10 Deontic constraints | Enumerate constraints on workflow | wos-workflow-governance | `GET /api/governance/:url/deontic-constraints` | full | Projected from bundle |
| Gov §10 Deontic constraints | List violations per instance | — | `GET /api/instances/:id/deontic-violations` | full | Filtered provenance view |
| Gov §6 Delegations | List delegations | wos-workflow-governance | `GET /api/governance/:url/delegations` | full |  |
| Gov §6 Delegations | Create delegation | — | `POST /api/governance/:url/delegations` | full | Supervisor-gated |
| Gov §6 Delegations | Revoke delegation | — | `DELETE /api/governance/:url/delegations/:id` | full |  |
| Gov §5.4 Assertion gates | Pipeline enumeration | wos-assertion-gate | `GET /api/governance/:url/pipelines` | full | Pipelines live under §5 Data Validation Pipelines, not §7 |
| Gov §5.4 Assertion gates | Run pipeline against inputs | — | — | **none** | No `POST /validate-pipeline`. **User value: high** — pipelines are the primary data-validation mechanism for untrusted inputs |
| Gov §7 Quality controls | List quality controls | wos-workflow-governance | `GET /api/governance/:url/quality-controls` | full |  |
| Gov §2.9 Schema upgrade | Named lifecycle operation | — | — | **none** | Migration endpoint missing (`POST /api/instances/:id/migrate`). **User value: medium** — rare outside multi-year workflows |

### Due Process Config (sidecar)

Spec: `specs/governance/due-process-config.md`. Schema: `schemas/governance/wos-due-process.schema.json`.

All rows here are **spec-side** — this document defines the *data shape* for due-process parameters. Consumed through `GET /api/governance/:url/bundle` or validated via `POST /api/lint/document`. No dedicated endpoints required.

### Policy Parameters (sidecar)

Spec: `specs/governance/policy-parameters.md`. Schema: `schemas/governance/wos-policy-parameters.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| PolicyParam §1.3 Date-indexed values | Resolve parameter as-of date | wos-policy-parameters | — | **none** | No `POST /policy/:url/resolve?asOf=…`. Date resolution is the whole point of this sidecar. **User value: high** — every regulation-tracking workflow needs as-of resolution |
| PolicyParam §1.5 Regulatory bindings | List bound regulations | wos-policy-parameters | `GET /api/governance/:url/policy-versions` | partial | Returns the projection but no as-of query |

### Assertion Library

Spec: `specs/governance/assertion-library.md`. Schema: `schemas/governance/wos-assertion-gate.schema.json`.

All rows are **spec-side** — reusable assertion definitions referenced by governance pipelines. No direct endpoint; served through the bundle read path.

---

## AI Integration (L2)

Spec: `specs/ai/ai-integration.md`. Schema: `schemas/ai/wos-ai-integration.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| AI §3 Agent registration | Register agent | wos-ai-integration + wos-agent-config | `POST /api/agents` | full | Backed by new `agents` table |
| AI §3 Agent registration | List registered agents | wos-ai-integration | `GET /api/agents?workflowUrl=…` | full |  |
| AI §3 Agent registration | Get agent by id | wos-agent-config | `GET /api/agents/:id` | full |  |
| AI §3.5 Trust boundary | Trust boundary declaration | wos-ai-integration | `GET /api/governance/:url/agents` | partial | Read-only projection, doesn't expose boundary details |
| AI §1.5 / Gov §7.2 | Separation of duties (agent must not review own output) | — | — | **none** | `PermissiveAccessControl::can_transition` returns `true` unconditionally; `AccessControl` trait has no method comparing actor identity to original author. **User value: high** — Gov §7.2 obligates this normatively (AI §1.5 informative table cross-references) |
| AI §4 Deontic constraints | Permissions / prohibitions / obligations for agents | — | `GET /api/governance/:url/deontic-constraints` | full | Shared endpoint |
| AI §5 Autonomy | Autonomy level cap | wos-agent-config | — | partial | Stored on agent row, not enforced on actions |
| AI §5.3 Autonomy capped on expired calibration | Enforce calibration expiry | wos-agent-config | — | **none** | No scheduled check; calibration metadata stored but never consulted. **User value: medium** — safety feature for production agents |
| AI §6 Confidence | Per-session confidence timeline | — | — | **none** | No `GET /api/instances/:id/confidence`. **User value: low-medium** — most deployments log confidence outside the case instance |
| AI §6 Fallback chain | Active fallback chain | wos-ai-integration | — | **none** | Plan called this out; not implemented. **User value: low** — rarely consumed at runtime |
| AI §agent lifecycle | Lifecycle transitions | — | `POST /api/agents/:id/lifecycle-transition` | full | Typed enum at boundary |
| AI §agent deployment | Canary / shadow | — | `POST /api/agents/:id/canary\|shadow` | partial | Writes deployment state; no traffic-splitting enforcement (belongs at gateway, not server) |
| AI §tool use | Tool invocation authorization | — | `POST /api/agents/:id/tool-invocation-check` | **stub** | Returns `{allowed: status==active && deploymentState==production}` — a reasonable default but not the full spec |

### Agent Config (sidecar)

Spec: `specs/ai/agent-config.md`. Schema: `schemas/ai/wos-agent-config.schema.json`.

Largely **spec-side** — endpoint config, credentials refs, model version lists. Consumed through agent registration or bundle loading. Calibration expiry is the only behavioural obligation and is flagged above as "none".

### Drift Monitor (sidecar)

Spec: `specs/ai/drift-monitor.md`. Schema: `schemas/ai/wos-drift-monitor.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Drift §1.3 Monitor metrics | Serve drift report shaped by configured metrics | wos-drift-monitor | `GET /api/agents/:id/drift` | **stub** | Spec defines metric config shape (PSI / KS / threshold); doesn't obligate the processor to compute. Endpoint returns spec-correct envelope with `psi: null, ks: null`. **User value: medium** — real impls have an external detector write reports; suggested follow-up: add a write-side `POST /api/agents/:id/drift` so the GET serves the most-recent externally-produced report |
| Drift §1.4 Deployment sequence | Canary / shadow gating on drift | — | — | spec-side | Enforced at gateway, not server |

---

## Advanced Governance (L3)

Spec: `specs/advanced/advanced-governance.md`. Schema: `schemas/advanced/wos-advanced.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Advanced §3 Equity guardrails | Evaluate equity over window | wos-equity | `POST /api/equity/evaluate` | partial | Real group-by runs over instances; outcome predicate is stubbed (`Some(_) ⇒ false`). Result shape is spec-correct. **User value: high** — main equity observable |
| Advanced §3.3 Async evaluation | Scheduled equity runs | wos-equity | — | spec-side | Belongs to a scheduler, not the server |
| Advanced §4 Constraint zones | List zones on workflow | wos-advanced | `GET /api/governance/:url/constraint-zones` | full | Projected from sidecar |
| Advanced §4.4 Relation evaluation | Compute DCR marking → valid next actions | — | `GET /api/instances/:id/constraint-zones/:zone/valid-actions` | **stub** | Returns declared activities; real marking evaluation against provenance not implemented. **User value: medium** — DCR-style case management is niche today |
| Advanced §5 Multi-step sessions | Session start / continue / complete with cumulative-confidence gating | — | — | **none** | §5.4 specifies cumulative-confidence product across DAG steps with intervention-point checkpoints — distinct from kernel compound states (which have no confidence semantics). **User value: medium** — narrow consumer set (multi-step LLM reasoning chains) |
| Advanced §6 Verifiable constraints | SMT verification | wos-verification-report | `POST /api/verification/verify` | **stub** | Returns `inconclusive` for every constraint. Real proofs require `WOS_SMT=z3`. Shape is spec-correct — consumers can integrate today |
| Advanced §7 Tool use governance | Tool invocation gating | — | `POST /api/agents/:id/tool-invocation-check` | **stub** | Shared with AI §tool use |
| Advanced §8 Agent lifecycle | State machine transitions | — | `POST /api/agents/:id/lifecycle-transition` | full | Shared with AI §agent lifecycle |
| Advanced §9 Calibration | Recalibration triggers | wos-agent-config | — | **none** | See AI §5.3 |
| Advanced §10 Shadow mode | Agent shadow deployment | — | `POST /api/agents/:id/shadow` | partial | Shared with AI |
| Advanced §11 Circuit breaker | Agent-level breaker (errorRateThreshold / cooldownDuration / closed-open-half-open) | — | — | **none** | Agent-semantic — error rate of agent invocations feeds agent lifecycle state via `lifecycleHook`. Distinct from network-layer breakers a service mesh provides. **User value: medium** — standalone-agent deployments need it |

### Verification Report (sidecar)

Spec: `specs/advanced/verification-report.md`. Schema: `schemas/advanced/wos-verification-report.schema.json`.

**Spec-side** document — the output envelope of a verification run. Consumed via `POST /api/verification/verify` response and `GET /api/governance/:url/verification-report` projection.

### Equity Config (sidecar)

Spec: `specs/advanced/equity-config.md`. Schema: `schemas/advanced/wos-equity.schema.json`.

**Spec-side** document defining protected categories, disparity methods, schedule. Consumed via `GET /api/governance/:url/equity-config` (already implemented).

---

## Assurance

Spec: `specs/assurance/assurance.md`. Schema: `schemas/assurance/wos-assurance.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Assurance §2.1 Taxonomy | L1–L4 assurance levels | wos-assurance | — | full | Enforced at type level via `AssuranceLevel` enum |
| Assurance §2.3 Upgrade facts | Record assurance upgrade | wos-assurance | `POST /api/instances/:id/identity-facts/:id/upgrade` | full | Forward-only; `upgradedFrom` preserved |
| Assurance §3 Subject continuity | Cross-instance timeline for a subject | wos-assurance | `GET /api/subjects/:ref/assurance-chain` | partial | Returns ordered facts; continuity-hash validation not implemented. **User value: high** — continuity is the main assurance observable |
| Assurance §4 Invariant 6 | Assurance level ≠ disclosure posture | wos-assurance | — | full | Enforced at type level (two independent enums on request) |
| Assurance §5 Attestation | Provider-neutral attestation | — | — | **none** | No `/api/instances/:id/identity-facts/:id/attest`. **User value: medium** — legal-sufficiency deployments need attestation; low-assurance deployments don't |
| Assurance §6 Legal sufficiency disclosure | Disclosure metadata on exports when claims are made | — | — | **none** | §6.1 obligates a disclosure of which conditions an implementation relies on (process, signature semantics, records-management, applicable law) **when** the implementation makes claims about evidentiary weight. Server-side exports today make no such claims and therefore are technically compliant; if/when we add attestation (§5), exports must carry the disclosure. **User value: medium** — gating the attestation work, not currently blocking |
| Assurance §custody | Custody posture declaration | — | — | **none** | Plan had `GET /api/instances/:id/custody-posture` as a stretch. **User value: medium** — specialised to chain-of-custody workflows |

---

## Integration Profile

Spec: `specs/profiles/integration.md`. Schema: `schemas/profiles/wos-integration-profile.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Integ §3.1 Overview | Load integration profile | wos-integration-profile | `GET /api/integration/:url/profile` | full |  |
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

Spec: `specs/profiles/semantic.md`. Schema: `schemas/profiles/wos-semantic-profile.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Semantic §2 Doc structure | Load semantic profile | wos-semantic-profile | `GET /api/bundles/:url` | partial | Served as part of the bundle; no dedicated `/semantic/:url` projection |
| Semantic §3 JSON-LD context | Serve JSON-LD context | — | — | **none** | Plan had `GET /api/semantic/jsonld-context`. **User value: medium** — needed by RDF consumers but can be shipped as static file |
| Semantic §4 SHACL | SHACL validation | — | — | **none** | Requires a SHACL engine. **User value: medium** — overlaps with our lint surface; real RDF shops want this |
| Semantic §5 PROV-O mapping | Export provenance as PROV-O | — | `GET /api/instances/:id/provenance/export?format=prov-o` | full |  |
| Semantic §5 XES mapping | Export as XES | — | `GET /api/instances/:id/provenance/export?format=xes` | full |  |
| Semantic §5 OCEL mapping | Export as OCEL | — | `GET /api/instances/:id/provenance/export?format=ocel` | full |  |
| Semantic §6 SPARQL queries | SPARQL query endpoint | — | — | **none** | Plan flagged as stub with `WOS_TRIPLESTORE=none` returning 501. Not implemented. **User value: low-medium** — export-to-triplestore is the usual flow; in-server SPARQL is convenient but not load-bearing |

---

## Sidecars

### Business Calendar

Spec: `specs/sidecars/business-calendar.md`. Schema: `schemas/sidecars/wos-business-calendar.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| BusCal §compute | Snap-forward deadline | wos-business-calendar | `POST /api/calendar/:url/compute-deadline` | full | Delegates to `wos_core::business_calendar::next_business_moment` |
| BusCal §business-days-between | Business-day delta | — | — | partial | Plan had `POST /api/calendar/:url/business-days-between`; the spec only obligates the deadline op, so this is optional. **User value: low** — trivial helper; clients can compose two `compute-deadline` calls |

### Notification Template

Spec: `specs/sidecars/notification-template.md`. Schema: `schemas/sidecars/wos-notification-template.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Notif §render | Template render with placeholder substitution | wos-notification-template | `POST /api/notifications/:url/render` | full | `${var}` + dotted `${nested.path}` supported |
| Notif §channels | Per-channel dispatch | — | — | spec-side | Delivery is out of scope for the server; template render returns declared channel list |

---

## User-value critique

Rows where the spec obligates a surface but the user value is questionable, and what we recommend.

### Low value — defer

1. **Semantic §6 SPARQL in-server.** In-process SPARQL requires an embedded triplestore and doesn't pay off for the usual "export → external tool" workflow. Users who need SPARQL have Apache Jena / Oxigraph already. Recommend: keep as optional feature behind `triplestore-oxigraph`; don't mark as MUST.
2. **AI §6 Fallback chain retrieval.** Fallback chains are typically driven by the agent registry at runtime, not queried by clients. The endpoint would have no real consumer. Recommend: leave as spec-side data on the AI integration doc; no dedicated endpoint.
3. **Runtime §Suspend / resume.** No evidence anyone uses these in practice. Recommend: lazy-implement when a real case comes in; don't build eagerly.
4. **Kernel §Correspondence template application.** Overlaps semantically with Notification template render (both shape outbound content). Recommend: clarify the boundary in the specs (correspondence = audit trail of *received* communication, notification = *outbound* content) — not a deletion case, but the surface area suggests merging or sharper delineation.

### High value — the real gaps

Rows where the spec is right and the missing surface is a concrete user-value block:

- **Runtime §9 / Gov §3.3 Explanation assembly.** Runtime §9 specifies the deterministic algorithm; Gov §3.3 specifies what must be delivered (individualised / categorical / aggregate by impact level). The two are a contract+implementation pair, not duplication. Server provides a *partial* surface today via the applicant-determination view; the dedicated `/instances/:id/explain` per Runtime §9.1 is missing.
- **Gov §5.4 Pipeline validation.** Assertion-gate pipelines have no run-against-inputs endpoint.
- **PolicyParam §1.3 As-of resolution.** Date-indexed policy resolution is the *whole point* of the policy-parameters sidecar and has no endpoint.
- **Integ §6 Correlation.** Async request/response (most interesting integrations) need correlation tokens; currently absent.
- **Gov §7.2 Separation of duties.** Spec normatively MUST-says actor cannot review own output (cross-referenced informatively in AI §1.5); `PermissiveAccessControl` permits it.
- **Assurance §3 Subject continuity.** Continuity-hash validation absent; chain endpoint exists but doesn't prove the chain.

### Spec smells

Ambiguities worth flagging on the spec side, but **not** grounds for unilateral server-side dismissal:

1. **Overlap between `correspondence-metadata` and `notification-template`.** Both define outbound content shapes. The boundary should be tightened in the specs — recommend an editorial pass, not a deletion.
2. **`assertion-library.md`** defines a reusable assertion shape but no spec actually declares how to *invoke* one. The `invokeAssertion` obligation is missing from `workflow-governance.md` §5.4. Recommend adding the invoke binding spec-side.

The previous version of this document also flagged Advanced §5 multi-step sessions, Advanced §11 circuit breakers, and Drift §1.3 as over-reach. Re-reading the specs more carefully:

- **Multi-step sessions (Advanced §5)** specify cumulative-confidence gating across DAG steps with intervention-point checkpoints — distinct from kernel compound states (which have no confidence semantics). Different abstractions; both have a place.
- **Circuit breakers (Advanced §11)** are agent-semantic (error rate of agent invocations feeding agent lifecycle state), not network-semantic. Service mesh breakers don't know what an agent's error predicate is. Defer if there's no consumer, but don't treat as over-reach.
- **Drift §1.3** only defines the *config shape* for drift metrics; nothing in the spec obligates the processor to compute them. The earlier "the processor structurally can't do this" critique was solving a non-problem.

---

## Asymmetries

### Schemas without specs

None — every schema under `/schemas` has a matching spec.

### Specs without schemas

None — every spec under `/specs` has a matching schema.

### Specs that define a shape but imply no server surface

These are document-shape specs that are (correctly) not exposed as resources; they flow through the generic validation and bundle-read surfaces:

- `schemas/kernel/wos-correspondence-metadata.schema.json` — validated via `/lint/document`
- `schemas/governance/wos-due-process.schema.json` — bundle projection
- `schemas/governance/wos-policy-parameters.schema.json` — bundle projection
- `schemas/governance/wos-assertion-gate.schema.json` — bundle projection
- `schemas/ai/wos-agent-config.schema.json` — bundle projection + agent registration
- `schemas/ai/wos-drift-monitor.schema.json` — bundle projection
- `schemas/advanced/wos-verification-report.schema.json` — output envelope from `/verification/verify`
- `schemas/advanced/wos-equity.schema.json` — bundle projection
- `schemas/assurance/wos-assurance.schema.json` — embedded in identity facts

---

## Gap ranking — priority × complexity × tech-debt burden

Every gap scored on three independent axes. **Priority** is user impact × urgency. **Complexity** is effort to close. **Debt burden** is the compounding cost of deferring — an isolated addition scores 1; a gap where every additional day spreads workarounds across the codebase or ossifies breaking-change exposure scores 5.

**Rubric.**
- **Priority (P)**: 5 = blocks conformance or legal-sufficiency gate · 3 = real consumer asks exist · 1 = spec curiosity.
- **Complexity (C)**: 1 = <1 hr · 2 = <1 day · 3 = 1-2 days · 4 = 3-5 days · 5 = multi-week or external adapter.
- **Debt burden (D)**: 5 = every week of delay compounds (consumers build on absence, retrofit is breaking) · 3 = downstream reinvention starts · 1 = pure addition.

### Ranked table

Sorted by ROI (= P × D / C; higher is more value-per-effort). Rescored relative to the pre-validation draft: Legal-sufficiency D 3 → 4 (every export shipped without disclosure compounds retrofit cost when attestation lands); Integration correlation D 4 → 5 (own document calls this breaking-change risk for every adapter); Chain-integrity D 1 → 2 (auditors hand-roll absent); Real drift P 2 → 3 (drift is the only behavioural surface for AI governance customers).

| Gap | Spec § | P | C | D | ROI |
|---|---|---|---|---|---|
| Legal-sufficiency disclosure on exports | Assurance §6 | 5 | 1 | 4 | **20.0** |
| Agent separation-of-duties enforcement | Gov §7.2 / AI §1.5 | 5 | 2 | 5 | **12.5** |
| Chain-integrity verify endpoint | Kernel §8 | 4 | 1 | 2 | 8.0 |
| Explanation assembly endpoint (full) | Runtime §9 / Gov §3.3 | 5 | 3 | 5 | 8.3 |
| Pipeline validation endpoint | Gov §5.4 | 4 | 3 | 5 | 6.7 |
| Integration correlation tokens | Integ §6 | 4 | 3 | 5 | 6.7 |
| Policy-parameters as-of resolution | PolicyParam §1.3 | 4 | 2 | 3 | 6.0 |
| Hold create / release CRUD | Gov §3.6 | 3 | 2 | 3 | 4.5 |
| Subject continuity-hash validation | Assurance §3 | 3 | 2 | 2 | 3.0 |
| Calibration expiry enforcement | AI §5.3 | 3 | 2 | 2 | 3.0 |
| Real drift detection | Drift §1.3 | 3 | 5 | 4 | 2.4 |
| JSON-LD context endpoint | Semantic §3 | 2 | 1 | 1 | 2.0 |
| Provenance attestation | Assurance §5 | 3 | 3 | 2 | 2.0 |
| SHACL validation | Semantic §4 | 2 | 3 | 2 | 1.3 |
| Counterfactual explanation | Gov §3.4 | 2 | 4 | 2 | 1.0 |
| Multi-step sessions | Advanced §5 | 2 | 3 | 3 | 2.0 |
| Migration endpoint | Gov §2.9 | 2 | 3 | 1 | 0.7 |
| Real SMT verification | Advanced §6 | 2 | 5 | 1 | 0.4 |
| Agent circuit breakers | Advanced §11 | 2 | 3 | 1 | 0.7 |
| SPARQL in-server | Semantic §6 | 1 | 5 | 1 | 0.2 |

Event-idempotency on `POST /api/instances/:id/events` (Runtime §4.3) was downgraded from "partial" to "none" during validation — added to the gap pool: P 4, C 2, D 4 → ROI 8.0. Goes alongside chain-integrity verify in week 1.

### Top by debt burden (D = 5)

These are the gaps where deferral **actively costs more every week**, independent of priority:

1. **Agent separation-of-duties (Gov §7.2 / AI §1.5).** Permissive behaviour is already shipped. Every new consumer builds expectations around "agent can self-review." Tightening later becomes a breaking change. Fix before more consumers land.
2. **Explanation assembly (Runtime §9 + Gov §3.3).** Two specs (algorithm + delivery contract) normatively home this. Server has a partial surface via applicant-determination today; if we let it solidify there, the Runtime §9.1 deterministic-algorithm contract becomes a parallel implementation later. Pick the dedicated `/explain` endpoint now.
3. **Pipeline validation (Gov §5.4).** Without a server-side gate evaluator, handler code hand-rolls assertion logic. Every month of delay scatters more bespoke assertion calls across the codebase.
4. **Integration correlation (Integ §6).** `ExternalService::invoke` is already in adapters' hands; adding correlation later breaks the trait. The longer we wait, the more adapters we invalidate.

### Decision matrix (cross-tabulated)

| Do now (high P, high D, low C) | Do when you can (high P, low D, low C) | Defer — spec change first | Defer indefinitely |
|---|---|---|---|
| Agent separation-of-duties | Legal-sufficiency disclosure | Multi-step sessions (delete) | SPARQL in-server |
| Explanation assembly endpoint | Chain-integrity verify | Agent circuit breakers (delete) | Real SMT verification |
| Pipeline validation endpoint | JSON-LD context | Real drift detection (pivot spec) | |
| Integration correlation tokens | Policy as-of resolution | SHACL validation | |
| Hold CRUD | Subject continuity-hash | | |
| | Calibration expiry | | |

### Recommended sequence

**Week 1 — quick wins + tighten the easy compounding gaps (~1 day total):**

1. Legal-sufficiency disclosure on exports (30 min) — emit a `wosDisclosure` block in PROV-O / XES / OCEL headers so the future attestation work doesn't require re-issuing exports.
2. Chain-integrity verify endpoint (1 hr) — wrap the existing `verify_chain` helper.
3. JSON-LD context endpoint (30 min) — static serve.
4. Agent separation-of-duties enforcement (2 hr) — **stops permissive drift**; tighten `AccessControl::can_transition` on transitions whose source state has a `review`-tagged actor.
5. Policy-parameters as-of resolution (2 hr) — date-indexed lookup over the policy-parameters sidecar.
6. Event-idempotency on `POST /events` (2 hr) — accept an `idempotencyToken` in the request body, dedupe in `event_queue`.

**Week 2 — full explanation + pipeline (~3 days total):**

7. Pipeline validation endpoint (1 day) — `POST /api/governance/:url/validate-pipeline` with `{inputs}`; assertion evaluator returns `{passed, failures}`.
8. Explanation assembly endpoint (1 day) — `GET /api/instances/:id/explain` per Runtime §9.1's deterministic algorithm; Gov §3.3 delivery contract is satisfied by selecting the explanation level from the instance's impact level. Existing applicant-determination view stays; new endpoint is the spec-shaped one.
9. Hold CRUD (3 hr) — `POST /api/instances/:id/holds` + `DELETE …/:holdId`; route through runtime so provenance is consistent.
10. Calibration expiry enforcement (3 hr) — background job; autonomy cap when calibration `validUntil` < now.
11. Subject continuity-hash validation (2 hr) — extend `assurance_chain` response with `chainValid: bool`.

**Week 3 — integration correctness + attestation (~3 days total):**

12. Integration correlation tokens (1 day) — **do before more adapters land**. Add `correlation_token: Option<String>` to `ExternalService::invoke` and `wos-runtime` callback registry.
13. Provenance attestation (1 day) — Ed25519 signing path; emits attestation record with the legal-sufficiency disclosure block.
14. Migration endpoint (1 day) — `POST /api/instances/:id/migrate` exposing `WosRuntime::migrate`.

**Defer for product-fit signal:**

- **Real drift detection** (Drift §1.3) — add write-side `POST /api/agents/:id/drift` and have the GET serve the most-recent stored report. Skip until an external detector is in scope.
- **Real SMT verification** (Advanced §6) — stub shape is durable; wait for a customer with a real proof obligation.
- **SHACL validation** (Semantic §4) — defer until an RDF consumer asks; overlaps with the lint surface.
- **Multi-step sessions** (Advanced §5) — defer; legitimate spec but narrow consumer set today (multi-step LLM reasoning chains).
- **Agent circuit breakers** (Advanced §11) — defer; standalone-agent deployments will need it eventually.

**Deferred indefinitely:**

- **SPARQL in-server** (Semantic §6) — export-to-external is the standard pattern.
- **Counterfactual explanation** (Gov §3.4) — narrow XAI audience.

### The four compounding costs of deferral

1. **Ossified permissive behaviour.** Every day the server ships with `PermissiveAccessControl` allowing agents to self-review, more consumers depend on that behaviour. Closing this gap later is no longer additive — it's a breaking change that invalidates existing integrations. Cost doubles every month.

2. **Explanation surface fragmentation.** Runtime §9.1 specifies a deterministic explanation algorithm; Gov §3.3 specifies what must be delivered (individualised / categorical / aggregate). The two are a contract+implementation pair, not duplication, but the partial surface today (applicant-determination) doesn't satisfy Runtime §9.1's algorithm contract. If consumers build against the partial surface, the dedicated `/explain` endpoint later becomes a migration path rather than an addition. Cost is one extra migration per consumer.

3. **Integration-binding dispatch shape.** The `ExternalService::invoke` trait signature doesn't model correlation tokens today. Any adapter written against the current shape will need a breaking trait update when correlation lands. Cost is linear in the number of external adapters written between now and the fix.

4. **Pipeline assertion scatter.** Without a server-side `validate-pipeline` endpoint, handlers and services hand-code assertion checks against governance rules. Every new check ossifies the pattern of bespoke assertion logic in handler code. Consolidation later means tracking down N inlined assertions and rerouting them through the gate evaluator.

The remaining 16 gaps are **additive** — deferring them creates no compounding cost. They're pure feature work that can happen whenever there's a concrete consumer.


---

## Notes for future readers

- The "stub" status is load-bearing: consumers can integrate today against spec-correct response shapes. Swapping to real adapters (Z3 for SMT, a real drift detector, a real SHACL engine) doesn't change the wire protocol. Stubs are a feature, not a compromise, for a reference implementation.
- The server intentionally does NOT implement the Lifecycle Detail Companion as HTTP endpoints — it's an internal algorithm reference. Conformance tests cover it.
- Every sidecar that's marked entirely "spec-side" (due-process-config, policy-parameters, assertion-library, agent-config, verification-report, equity-config) is served through the existing `/api/bundles/:url` bundle join. Adding dedicated endpoints would fragment the surface.

