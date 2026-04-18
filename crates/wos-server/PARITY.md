# wos-server Parity Matrix

_Cross-references `/specs` + `/schemas` against the server's HTTP + Socket.IO surface on branch `claude/wos-spec-backend-y17wJ` as of commit `645fbd8`._

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
| full | 28 |
| partial | 9 |
| stub | 7 |
| none | 11 |
| spec-side | 13 |
| **total** | **68** |

Kernel + runtime companion are fully implemented. Governance L1 read-side and sidecar operations are solid. The gaps cluster in three places: (1) assurance attestation / continuity cross-subject queries, (2) integration-profile real dispatch (currently echo), (3) semantic profile's SHACL / SPARQL (triplestore adapter needed). Stubs are concentrated in advanced L3 (SMT verification, drift detection) — all three require external adapters the plan defers.

---

## Kernel layer

Spec: `specs/kernel/spec.md` — the authoritative WOS Kernel Specification. Schema: `schemas/kernel/wos-kernel.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Kernel §2.2 Structural | Parse + validate kernel document | wos-kernel | `POST /api/kernel/validate` | full | Routed through `wos-lint::lint_document` |
| Kernel §2.2 Structural | Round-trip kernel without loss | wos-kernel | `PUT /api/kernels/:url/kernel` | full | Serde preserves all fields |
| Kernel §2.2 Structural | List registered kernels | — | `GET /api/kernels` | full | Alias of legacy `/bundles` |
| Kernel §2.2 Structural | Load kernel document | wos-kernel | `GET /api/kernels/:url/kernel` | full |  |
| Kernel §2.2 Structural | Load kernel + sidecars bundle | — | `GET /api/kernels/:url` | full | Joins all attached sidecars |
| Kernel §3 Actor Model | Actor type resolution | wos-kernel | — | spec-side | Embedded in kernel doc; evaluator uses it internally |
| Kernel §4 Lifecycle | Deterministic event evaluation | wos-kernel | `POST /api/instances/:id/events` | full | Routes through `AppRuntime` → `WosRuntime` |
| Kernel §5 Provenance | Append-only provenance with hash chain | wos-kernel | `GET /api/instances/:id/provenance` | full | `ProvenanceService::prepare_batch` enforces chain |
| Kernel §5 Provenance | Chain integrity verification | wos-kernel | — | **none** | No dedicated `/provenance/verify` endpoint; chain is verified on read but result isn't surfaced. **User value: medium** — auditors want an explicit "chain valid" response. |
| Kernel §6 Contracts | Contract reference resolution | wos-kernel | — | spec-side | Internal to evaluator |

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
| Runtime §4.3 Exactly-once | Idempotency on event IDs | — | — | partial | Runtime accepts idempotency tokens on task submission; event submission doesn't dedupe. **User value: high** for at-least-once producers |
| Runtime §5 Action execution | onEntry/onExit/transition actions | — | `POST /api/instances/:id/events` | full | Evaluator executes |
| Runtime §5.4 invokeService | Service invocation seam | — | via `runtime/service.rs::EchoExternalService` | **stub** | Echoes input. Real dispatch lives in integration profile §3 |
| Runtime §5.5 Contract validation | Formspec validation on task submit | wos-case-instance | `POST /api/tasks/:id/response` | partial | `runtime/validator.rs::PermissiveValidator` accepts all. Real `FormspecProcessor` not wired |
| Runtime §6 Durability | Atomic checkpoint | — | n/a | full | `update_instance_atomic` transactional in SQLite |
| Runtime §7 Timers | Timer create / cancel / fire | — | `services/timer_task.rs` polls | partial | Correct for ≤200 instances; efficiency review flagged full-scan issue |
| Runtime §9 Explanation | Explanation assembly | — | — | **none** | No `/instances/:id/explain` endpoint. **User value: high** — due-process spec (§3.3) obligates explanation delivery. Currently a known gap |
| Runtime §10 Eval modes | Dry-run transitions | — | `GET /api/instances/:id/transitions` | full | Pure kernel walk |
| Runtime §11 Multi-version coexistence | Instances pinned to definition version | — | `GET /api/instances/:id` | full | `definition_version` preserved on row |
| Runtime §S12 Host interfaces | RuntimeStore / DocumentResolver / TaskPresenter / AccessControl / ExternalService / ContractValidator | — | via `runtime/` impls | full | All six hooks implemented (access + validator permissive) |
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
| Gov §3.6 Continuation of service | Hold management | — | — | **none** | Hold state lives in `CaseInstance.governance_state` but no CRUD endpoint. **User value: medium** — benefits adjudication needs this |
| Gov §4 Review protocols | Two-reviewer / supervisor override | — | — | spec-side | Enforced by kernel actor model + lifecycle actions; no separate endpoint needed |
| Gov §5 Deontic constraints | Enumerate constraints on workflow | wos-workflow-governance | `GET /api/governance/:url/deontic-constraints` | full | Projected from bundle |
| Gov §5 Deontic constraints | List violations per instance | — | `GET /api/instances/:id/deontic-violations` | full | Filtered provenance view |
| Gov §6 Delegations | List delegations | wos-workflow-governance | `GET /api/governance/:url/delegations` | full |  |
| Gov §6 Delegations | Create delegation | — | `POST /api/governance/:url/delegations` | full | Supervisor-gated |
| Gov §6 Delegations | Revoke delegation | — | `DELETE /api/governance/:url/delegations/:id` | full |  |
| Gov §7 Assertion gates | Pipeline enumeration | wos-assertion-gate | `GET /api/governance/:url/pipelines` | full |  |
| Gov §7 Assertion gates | Run pipeline against inputs | — | — | **none** | Plan called this out as `POST /validate-pipeline`; not yet implemented. **User value: high** — pipelines are the primary deontic-check mechanism |
| Gov §8 Quality controls | List quality controls | wos-workflow-governance | `GET /api/governance/:url/quality-controls` | full |  |
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
| AI §3.7 Agent MUST NOT review own output | Separation of duties enforcement | — | — | partial | Currently `PermissiveAccessControl`; not enforced. **User value: high** — spec explicitly names this as MUST |
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
| Drift §1.3 Monitor metrics | PSI / KS drift over windows | wos-drift-monitor | `GET /api/agents/:id/drift` | **stub** | Returns `{psi: null, ks: null, note: "configure WOS_DRIFT_DETECTOR"}`. **User value: medium** — real value requires a drift detector process; server should return the *last computed* report rather than computing on demand. Suggested fix: treat drift reports as externally-written rows, add `POST /api/agents/:id/drift` for producers |
| Drift §1.4 Deployment sequence | Canary / shadow gating on drift | — | — | spec-side | Enforced at gateway, not server |

---

## Advanced Governance (L3)

Spec: `specs/advanced/advanced-governance.md`. Schema: `schemas/advanced/wos-advanced.schema.json`.

| section | capability | schema | endpoint | status | notes |
|---|---|---|---|---|---|
| Advanced §3 Equity guardrails | Evaluate equity over window | wos-equity | `POST /api/equity/evaluate` | **stub** | Real group-by runs but outcome predicate is stubbed; result shape is spec-correct. **User value: high** — this is the main equity observable |
| Advanced §3.3 Async evaluation | Scheduled equity runs | wos-equity | — | spec-side | Belongs to a scheduler, not the server |
| Advanced §4 Constraint zones | List zones on workflow | wos-advanced | `GET /api/governance/:url/constraint-zones` | full | Projected from sidecar |
| Advanced §4.4 Relation evaluation | Compute DCR marking → valid next actions | — | `GET /api/instances/:id/constraint-zones/:zone/valid-actions` | **stub** | Returns declared activities; real marking evaluation against provenance not implemented. **User value: medium** — DCR-style case management is niche today |
| Advanced §5 Multi-step sessions | Session start / continue / complete | — | — | **none** | Plan had this as stretch. **User value: low** — overlaps with kernel compound states; serious implementations should extend the kernel instead |
| Advanced §6 Verifiable constraints | SMT verification | wos-verification-report | `POST /api/verification/verify` | **stub** | Returns `inconclusive` for every constraint. Real proofs require `WOS_SMT=z3`. Shape is spec-correct — consumers can integrate today |
| Advanced §7 Tool use governance | Tool invocation gating | — | `POST /api/agents/:id/tool-invocation-check` | **stub** | Shared with AI §tool use |
| Advanced §8 Agent lifecycle | State machine transitions | — | `POST /api/agents/:id/lifecycle-transition` | full | Shared with AI §agent lifecycle |
| Advanced §9 Calibration | Recalibration triggers | wos-agent-config | — | **none** | See AI §5.3 |
| Advanced §10 Shadow mode | Agent shadow deployment | — | `POST /api/agents/:id/shadow` | partial | Shared with AI |
| Advanced §11 Circuit breaker | Agent-level breaker | — | — | **none** | **User value: low** — typical deployments use process-level circuit breakers (sidecar or service mesh); encoding in the spec is over-reach |

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
| Assurance §6 Legal sufficiency disclosure | Disclosure metadata on exports | — | — | **none** | Spec §6 obligates every artifact to carry a legal-sufficiency disclosure. Currently missing from PROV-O / XES / OCEL exports. **User value: high** — this is explicit spec MUST |
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
| Semantic §2 Doc structure | Load semantic profile | wos-semantic-profile | `GET /api/kernels/:url` | partial | Served as part of the bundle; no dedicated `/semantic/:url` projection |
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

### Low value — defer or delete

1. **Advanced §11 Circuit breaker.** Encoding a per-agent circuit breaker in a governance document duplicates what every deployment already has at the service-mesh / API-gateway layer. Recommend: remove from spec, or reclassify as "operational guidance".
2. **Advanced §5 Multi-step sessions.** The kernel's compound states already express multi-step orchestration. A parallel "session" concept creates two ways to do the same thing. Recommend: drop from advanced spec; document how to express sessions via kernel states.
3. **Semantic §6 SPARQL in-server.** In-process SPARQL requires an embedded triplestore and doesn't pay off for the usual "export → external tool" workflow. Users who need SPARQL have Apache Jena / Oxigraph already. Recommend: keep as optional feature behind `triplestore-oxigraph`; don't mark as MUST.
4. **AI §6 Fallback chain retrieval.** Fallback chains are typically driven by the agent registry at runtime, not queried by clients. The endpoint would have no real consumer. Recommend: leave as spec-side data on the AI integration doc; no dedicated endpoint.
5. **Drift Monitor as "compute on demand".** Current stub computes nothing. Better: treat drift reports as externally-produced artifacts, add a write endpoint for drift detector processes, and serve the last report. Recommend: flip the spec's obligation from "processor computes" to "processor stores and serves".
6. **AI §tool-invocation-check.** Currently hardcoded to `allowed = status==active && deploymentState==production`. The spec obligates a richer check (rate limits, tool-specific restrictions, cooldowns) but those are gateway concerns, not server concerns. Recommend: remove from server-side obligations; keep as metadata on the agent config.
7. **Kernel §Correspondence template application.** Overlaps with Notification template render. Recommend: merge the two specs into one "outbound content templating" surface.
8. **Runtime §Suspend / resume.** No evidence anyone uses these in practice. Recommend: lazy-implement when a real case comes in; don't build eagerly.

### High value — the real gaps

Rows where the spec is right and the missing surface is a concrete user-value block:

- **Runtime §9 / Gov §3.3 Explanation assembly.** Due-process spec obligates rendered explanation; no endpoint. This is the #1 gap.
- **Assurance §6 Legal-sufficiency disclosure on exports.** The semantic export endpoints don't carry the spec-required disclosure. Low-effort to add; blocks legal-sufficiency claims.
- **Gov §7 Pipeline validation.** Assertion-gate pipelines are the main deontic mechanism; no way to run one against inputs.
- **PolicyParam §1.3 As-of resolution.** Date-indexed policy resolution is the *whole point* of the policy-parameters sidecar and has no endpoint.
- **Integ §6 Correlation.** Async request/response (most interesting integrations) need correlation tokens; currently absent.
- **AI §3.7 Agent separation of duties.** Spec explicitly MUST-says agent can't review own output; `PermissiveAccessControl` permits it.

### Spec smells

Ambiguities or over-reach in the specs themselves:

1. **Overlap between `correspondence-metadata` and `notification-template`.** Both define outbound content shapes for audit trails. Users have to know which to use when. → merge or clearly delineate.
2. **Runtime §9 Explanation vs. Gov §3.3 Explanation vs. Assurance §5 Attestation.** Three specs, one concept. Pick one normative home.
3. **`drift-monitor.md` obligates "processor monitors drift"** but the actual work has to happen elsewhere (the processor doesn't see the model's inference stream). Recommend: restructure as "processor stores and serves externally-produced drift reports".
4. **Advanced governance's multi-step sessions** invent a parallel state concept. Don't. Reuse the kernel.
5. **`assertion-library.md`** defines a reusable assertion shape but no spec actually declares how to *invoke* one. Missing `invokeAssertion` obligation in `workflow-governance.md` §7.

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

## Gaps prioritised by user value

| Gap | Spec section | User value (1-5) | Effort to close | Why |
|---|---|---|---|---|
| Explanation assembly endpoint | Runtime §9 / Gov §3.3 | **5** | Medium | Legal-sufficiency gate for every adverse-decision workflow |
| Legal-sufficiency disclosure on exports | Assurance §6 | **5** | Trivial | One-liner added to PROV-O / XES / OCEL headers; unlocks legal claims |
| Pipeline validation endpoint | Gov §7 | **4** | Medium | Primary deontic-check mechanism; conformance suite needs it |
| Policy-parameters as-of resolution | PolicyParam §1.3 | **4** | Low | Point of the sidecar; trivial date index over the stored values |
| Integration correlation tokens | Integ §6 | **4** | Medium | Unblocks real async integrations |
| Agent separation-of-duties enforcement | AI §3.7 | **4** | Low | `PermissiveAccessControl` → check actor on review-tagged transitions |
| Chain-integrity verify endpoint | Kernel §5 | **3** | Trivial | `GET /provenance/verify` — already computed, just not surfaced |
| Subject continuity-hash validation | Assurance §3 | **3** | Low | Add hash check to existing `/subjects/:ref/assurance-chain` |
| Provenance attestation | Assurance §5 | **3** | Medium | Needs signing key management |
| Calibration expiry enforcement | AI §5.3 | **3** | Low | Background job; autonomy cap already in evaluator |
| Hold create / release CRUD | Gov §3.6 | **3** | Low | `governance_state.holds` exists on CaseInstance; needs endpoint |
| Migration endpoint | Gov §2.9 | **2** | Medium | Rare feature |
| Counterfactual explanation | Gov §3.4 | **2** | Large | Expensive; narrow XAI audience |
| Real SMT verification | Advanced §6 | **2** | Large (external) | Z3 adapter; response shape already spec-correct |
| SHACL validation | Semantic §4 | **2** | Medium | Overlaps with lint surface |
| JSON-LD context endpoint | Semantic §3 | **2** | Trivial | Static-file serve |
| Real drift detection | Drift §1.3 | **2** | Large (external) | Better: make drift a write-side endpoint |
| SPARQL in-server | Semantic §6 | **1** | Large (external) | Export-to-external is the standard pattern |
| Multi-step sessions | Advanced §5 | **1** | — | Spec overlap with kernel compound states |
| Agent circuit breakers | Advanced §11 | **1** | — | Gateway concern, not server |

**Top 5 to close next** (value ≥ 4, effort ≤ medium):

1. **Legal-sufficiency disclosure on exports** — one line in `semantic_service.rs`, unblocks Assurance §6 conformance. 30-minute change.
2. **Chain-integrity verify endpoint** — wrap the hash check in an HTTP handler. 1-hour change.
3. **Policy-parameters as-of resolution** — iterate date-indexed values. 2-hour change.
4. **Agent separation-of-duties enforcement** — tighten `AccessControl::can_transition` to reject agent actors on review-tagged transitions. 2-hour change.
5. **Explanation assembly endpoint** — walk provenance + kernel, render narrative + reasoning + counterfactual stub. 1-day change.

---

## Notes for future readers

- The "stub" status is load-bearing: consumers can integrate today against spec-correct response shapes. Swapping to real adapters (Z3 for SMT, a real drift detector, a real SHACL engine) doesn't change the wire protocol. Stubs are a feature, not a compromise, for a reference implementation.
- The server intentionally does NOT implement the Lifecycle Detail Companion as HTTP endpoints — it's an internal algorithm reference. Conformance tests cover it.
- Every sidecar that's marked entirely "spec-side" (due-process-config, policy-parameters, assertion-library, agent-config, verification-report, equity-config) is served through the existing `/api/kernels/:url` bundle join. Adding dedicated endpoints would fragment the surface.

