# Bundle the wholesale ADR 0082 greenfield landing

**Status:** WS-1 Phases A–C landed + 11 test failures repaired (210/210 green); WS-2 portal not started
**Anchor:** [`thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md`](/Users/mikewolfd/Work/formspec-stack/thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md)
**Closes:** PLN-0401, PLN-0402, PLN-0405; remaining TODO-STACK ADR 0082 still-open boxes; case-portal/TODO.md Phase 1 + Phase 2

---

## Context

ADR 0082 ratified the public REST API contract — schemas + specs landed for 12 domains, OpenAPI snapshot covers 65 paths, 6 CI gates wired, typify build resolver works. **What's missing is the implementation pair on both sides for 11 domains** (everything except `notification`), plus utoipa auto-emission (PLN-0401), legacy render-route deletion (PLN-0402), and portal regen (PLN-0405).

The orthodox path was per-domain incremental ratification with a long-lived `// adr-0082-d13: internal — pending PLN-040x` bridge across 60 internal-marked routes plus `domain/*.rs` legacy API DTOs surviving as the active type system on the server. The user has chosen the heterodox path: **wholesale rebuild, no legacy, one bundle**. Reasons: (1) the platform thesis is that schemas are the source of truth; carrying parallel legacy types contradicts the discipline ADR 0082 exists to enforce. (2) 11 domains × per-handler "annotate utoipa later" doubles the per-handler work. (3) The internal-marker bridge normalizes "ratify halfway" — once it lasts six months, it lasts forever.

End state of this bundle: zero internal markers, zero `domain/*.rs` API DTOs, OpenAPI doc auto-emitted from `utoipa::path(...)` annotations, portal `src/ports/types.ts` reduced to a re-export barrel for portal-only types only, every CI gate green including PLN-0401's Gate 3 (snapshot staleness).

---

## End-state assertions (definition of done)

- [x] `grep -rn "// adr-0082-d13: internal" workspec-server/crates/wos-server/src` returns zero matches — **was 49, now 0**
- [x] `ls workspec-server/crates/wos-server/src/domain/` contains only internal model types — no API DTOs — **`domain/` deleted entirely**
- [x] Every public handler under `workspec-server/crates/wos-server/src/http/` carries `#[utoipa::path(...)]` — **98 annotations across all handlers**
- [ ] `cargo build -p wos-server` regenerates `work-spec/api/wos-public-api.openapi.json`; `git diff --exit-code work-spec/api/` clean in CI (PLN-0401e Gate 3 active)
- [x] `case-portal/src/types/wos/` contains 16 generated `api-*.ts` modules
- [x] `case-portal/src/ports/types.ts` declares only portal-only types — **190-line re-export barrel**
- [x] `npm run types:check` no diff — **`npx tsc --noEmit` clean**
- [x] `cargo nextest run -p wos-server --tests` green — **210/210 pass ✅**
- [ ] `npx vitest run` green (including new `tests/integration/fixture-conformance.test.ts`)
- [ ] Gates 1–6 green; Gate 7 (response conformance) green per ratified domain
- [ ] `oasdiff` reports no breaking changes vs main on the OpenAPI snapshot
- [ ] PLN-0401, PLN-0402, PLN-0405 closed in `PLANNING.md`

---

## Execution log

### WS-1 Server — Phase A (infrastructure) ✅

All pre-bundle infrastructure landed prior to this execution session:
- A1: `utoipa::ToSchema` derive in `build.rs` ✅
- A2: utoipa primitive adapters — handled via `#[schema(value_type = ...)]` on individual fields ✅
- A3: Service trait reshape — 6/11 domains (auth, audit, dashboard, applicant, correspondence, reports) already return `api::types::*` ✅
- A4: `IdempotencyStore` trait + `InMemoryIdempotencyStore` impl + `http/idempotency.rs` middleware ✅
- A5: `bin/emit_openapi.rs` binary (scaffold mode) ✅

### WS-1 Server — Phase B (domain handlers) ✅

| Domain | Handler | Service | utoipa | api::types | Markers removed |
|--------|---------|---------|--------|------------|-----------------|
| Auth (B1) | `http/auth.rs` | `auth_service.rs` ✅ | 6 ✅ | ✅ | Prior session |
| Audit (B2) | `http/audit.rs` | `audit_service.rs` ✅ | 4 ✅ | ✅ | Prior session |
| Dashboard (B2) | `http/dashboard.rs` | `dashboard_service.rs` ✅ | 4 ✅ | ✅ | Prior session |
| Applicant (B2) | `http/applicant.rs` | `applicant_service.rs` ✅ | 4 ✅ | ✅ | Prior session |
| Correspondence (B3) | `http/correspondence.rs` | `correspondence_service.rs` ✅ | 7 ✅ | ✅ | Prior session |
| Reports (B3) | `http/reports.rs` | `report_service.rs` ✅ | 6 ✅ | ✅ | Prior session |
| **Bundle (B4)** | `http/bundles.rs` | `bundle_service.rs` ✅ | 4 ✅ | Local types relocated | 3 → 0 |
| **Governance (B4)** | `http/governance.rs` | `governance_service.rs` ✅ | 13 ✅ | Generated + local | 13 → 0 |
| **Task (B5)** | `http/tasks.rs` | `task_service.rs` (new) ✅ | 5 ✅ | Generated | — |
| **Instance (B6)** | `http/instances.rs` | `instance_service.rs` ✅ | 16 ✅ | Generated | 6 → 0 |
| **Agents** | `http/agents.rs` | — | 7 ✅ | Service types | 7 → 0 |
| **Advanced** | `http/advanced.rs` | — | 4 ✅ | Service types | 4 → 0 |
| **Semantic** | `http/semantic.rs` | — | 1 ✅ | `serde_json::Value` | 1 → 0 |
| **AI Chat** | `http/ai_chat.rs` | — | 1 ✅ | `serde_json::Value` | 1 → 0 |
| **Lint** | `http/lint.rs` | — | 3 ✅ | Service types | 3 → 0 |
| **Integration** | `http/integration.rs` | — | 3 ✅ | Service types | 3 → 0 |
| **Deontic** | `http/deontic.rs` | — | 1 ✅ | `ProvenanceEnvelope` | 1 → 0 |
| **Conformance** | `http/conformance.rs` | — | 1 ✅ | Service types | 1 → 0 |
| **Calendar** | `http/calendar.rs` | — | 1 ✅ | Service types | 1 → 0 |
| **Assurance** | `http/assurance.rs` | — | 3 ✅ | Service types | 3 → 0 |
| **Signature** | `http/signature.rs` | — | 1 ✅ | `ProvenanceEnvelope` | 1 → 0 |
| **OpenAPI** | `http/openapi.rs` | — | 1 ✅ | `serde_json::Value` | 1 → 0 |

**Total: 98 `#[utoipa::path]` annotations. 49 → 0 internal markers.**

### WS-1 Server — Phase C (cleanup) ✅

- `domain/*.rs` — all 8 files deleted (applicant, auth, bundle, dashboard, governance, instance, provenance, mod) ✅
- `pub mod domain;` removed from `lib.rs` ✅
- `MigrateIdempotencyCache` retained in `lib.rs` (still referenced by existing runtime lifecycle tests) ⚠️
- `event_idempotency` field type changed from `HashMap<String, EvaluationResultView>` to `HashMap<String, EvaluationResult>` (generated) ✅
- `ProvenanceResponse` → `ProvenanceEnvelope` in provenance_service, semantic_service, signature, deontic ✅
- `governance_service.rs`: 13 view types without generated schema equivalents defined locally with `#[derive(utoipa::ToSchema)]` ✅
- `bundle_service.rs`: `BundleView`, `KernelSummaryView`, `ValidationResultView` relocated from domain to service file ✅
- `eval_service.rs`: `AvailableTransitionView` defined locally ✅
- `ProvenanceEnvelope`, `ExplainResponse`, `ChainVerifyResponse`, `MigrateInstanceRequest`, `DrainStepSummary`, `CreateHoldRequest`, `LegacyCreateInstanceBody`, `Format` — all carry `#[derive(utoipa::ToSchema)]` ✅
- `provenance_spec_shape.rs` test rewritten for `ProvenanceEnvelope` ✅

### Server test status

- `cargo check -p wos-server` ✅ clean (1 warning: dead field)
- `cargo nextest run -p wos-server --tests`: **210/210 pass ✅**

**Test repair session (11 failures → 0):**

Root causes and fixes across 3 submodules (8 files):

| Root cause | Files changed | Tests fixed |
|---|---|---|
| Runtime rejects URN-format instance IDs, mints TypeID replacements | `wos-core/src/instance.rs` (+`is_instance_urn`, `extract_urn_parts`), `wos-runtime/src/runtime/instance.rs` (accept URNs + URN tenant extraction) | 6 (runtime_lifecycle ×4, event_submit ×2) |
| `extract_tenant()` can't parse URN namespace (splits by `_`) | `wos-runtime/src/runtime/instance.rs` (fall back to `extract_urn_parts`) | 2 (tenant_passthrough ×2) |
| `task_urn()`/`instance_urn()` generate invalid URNs (empty scope) | `task_service.rs`, `instance_service.rs` (+`urn_scope_and_date` helpers) | 3 (tasks_lifecycle ×3) |
| `TaskOutcomeKind` lacks `Completed` variant | `task.schema.json` (+`"completed"`), `task_service.rs` (map Completed→Completed) | 1 |
| Handler default URN has empty scope | `instances.rs` (`"default"` scope) | preventive |
| `save_record` doesn't update `definition_version` column | `runtime_store.rs` (+`current.definition_version = ...`) | 1 (migrate cross-version) |
| `submit_task` calls `get_task` after runtime removes it from `active_tasks` | `task_service.rs` (look up task BEFORE submitting) | 2 (submit, dismiss-then-respond) |
| Test fixtures: invalid URN + missing `status: "completed"` | `http_tasks_lifecycle.rs` (valid URN in seed, `"status": "completed"` in responses) | 3 |

### What's left (WS-1 server)

- [ ] Write per-domain conformance tests (B-step checklist item 5 — none exist yet)
- [ ] Wire OpenAPI emit binary to CI — `cargo build` regenerates `work-spec/api/wos-public-api.openapi.json`; `git diff --exit-code` guard (PLN-0401e Gate 3)
- [ ] Run `oasdiff` breaking-change check vs main (Gate 5)

### What's left (WS-2 portal)

- [ ] WS-2 Phase A1 (type regen) — already done; 16 `api-*.ts` modules exist
- [ ] WS-2 Phase A2 (barrel rewrite) — already done; `types.ts` is 190-line re-export barrel
- [ ] WS-2 Phase B1 (port interface migration) — import paths flip to `types/wos/api-*`, provenance methods update signatures
- [ ] WS-2 Phase B2 (HTTP adapter rewrite) — return types flip, stubs stay as `NotImplementedError`
- [ ] WS-2 Phase B3 (fixture adapter rewrite) — provenance literals flatten from envelope shape to tier-discriminated union
- [ ] WS-2 Phase B4 (`safeCall` audit) — wrap unratified endpoint calls
- [ ] WS-2 Phase C1 (fixture-conformance test) — new `tests/integration/fixture-conformance.test.ts`
- [ ] WS-2 Phase C2 (component spot-check) — `AuditViewer.tsx` discriminated-union narrowing
- [ ] WS-2 Phase C3 (`npx vitest run` + `npm run build` clean)

### What's left (WS-3 cross-stack bookkeeping)

- [ ] Close PLN-0401, PLN-0402, PLN-0405 in `PLANNING.md`
- [ ] Update `TODO-STACK.md` ADR 0082 row
- [ ] Update `case-portal/TODO.md` Phase 1 + 2

### Portal — partial

- TS compilation errors fixed: `ProcessDashboard.tsx`, `TaskList.test.tsx`, `DocumentsTab.tsx`, `RelatedCasesTab.tsx` ✅
- `npx tsc --noEmit` clean ✅
- WS-2 port interface migration: not started
- WS-2 adapter rewrite: not started
- WS-2 fixture-conformance test: not started

---

## Approach

Two parallel workstreams + a cross-submodule commit cluster. **Schemas at `work-spec/schemas/api/` are the only source of truth** — both sides codegen from them; neither blocks the other.

| Workstream | Owner | Critical-path bottleneck |
|---|---|---|
| **WS-1: Server greenfield rebuild** | workspec-server | Service trait reshape (single atomic PR unblocks 11 domains) |
| **WS-2: Portal greenfield rebuild** | case-portal | Type regen + `ports/types.ts` barrel rewrite (unblocks all port/adapter migration) |
| **WS-3: Cross-submodule coordination** | stack-root | Submodule pointer bumps in atomic train |

WS-1 and WS-2 land independently. The portal does not wait for server endpoints to be implemented — stubbed methods that throw `NotImplementedError` keep their existing portal-side bridge (the `safeCall(promise, fallback)` pattern) until the server endpoint goes live, at which point the stub flips to a real fetch with **zero portal-side type change** because the response shape is already the generated typify shape.

---

## WS-1: Server greenfield rebuild

### WS-1 Phase A — Pre-bundle infrastructure (sequential, single PR cluster)

Land before any handler ratifies.

**A1. typify `utoipa::ToSchema` derive** (PLN-0401a)
- Edit: [`workspec-server/crates/wos-server/build.rs`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/build.rs) — add `settings.with_derive("utoipa::ToSchema".to_string())` next to existing `schemars::JsonSchema` derive
- Edit: [`workspec-server/crates/wos-server/Cargo.toml`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/Cargo.toml) — add `utoipa = { version = "5", features = ["chrono", "url"] }`
- Verify: `cargo check -p wos-server` clean

**A2. utoipa primitive adapters** (PLN-0401b)
- New: `workspec-server/crates/wos-server/src/api/types/adapters.rs` — `ToSchema` impls or `#[schema(value_type = ...)]` wrappers for `chrono::DateTime<Utc>`, `url::Url`, typify URN newtypes
- Edit: `workspec-server/crates/wos-server/src/api/types/mod.rs` — `pub mod adapters;` + `pub use common::{ActorRef, WosResourceUrn, ...}`

**A3. Service trait reshape** (single atomic PR — the unblock)
- Touch every file under [`workspec-server/crates/wos-server/src/services/*.rs`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/src/services/) (14 files)
- Every trait flips return types from `crate::domain::*` to `crate::api::types::*` — **no `From<>` shims**, the typify type is the only API truth
- New service files (5):
  - `services/auth_service.rs` — sessions, credentials, MFA, scope binding
  - `services/correspondence_service.rs` — render pipeline, calendar, delivery integration; subsumes legacy notification render (PLN-0402)
  - `services/report_service.rs` — cross-case query executor, server pagination
  - `services/task_service.rs` — draft/response/dismissal (extracted from inline `http/tasks.rs`)
  - `services/idempotency_store.rs` — see A4
- Handler bodies stub to `unimplemented!()` to keep `cargo check` green; bodies fill in WS-1 Phase B

**A4. IdempotencyStore middleware** (PLN-0401-adjacent, ADR D-16)
- New: `workspec-server/crates/wos-server/src/services/idempotency_store.rs`
- Trait: `async fn check(&self, scope: &str, key: &str) -> Option<StoredResponse>` + `record(...)`. `scope = "{method}:{path_template}"`
- Backing: **Postgres** (table `idempotency_records (scope, key, response_blob, created_at)` with `(scope,key)` PK; 24-hour TTL via background sweeper attached to existing `services::timer_task`). Reuses `StorageHandle` Postgres pool — no Redis dependency added
- New: `workspec-server/crates/wos-server/src/http/idempotency.rs` — `axum::middleware::from_fn_with_state` middleware
- Apply selectively via `route_layer(...)` to the 11 ADR D-16 endpoints
- Delete: `crates/wos-server/src/lib.rs` `MigrateIdempotencyCache` (lines 41–90) and `event_idempotency` `HashMap` field — the new store replaces them

**A5. OpenAPI emit binary** (PLN-0401d)
- New: `workspec-server/crates/wos-server/src/bin/emit_openapi.rs` — collects every `utoipa::path` annotation and writes to `../../work-spec/api/wos-public-api.openapi.json` (cross-submodule write per CLAUDE.md "Cross-stack scripts" pattern)
- Edit: stack-root `Makefile` — add `make openapi-emit` target invoked by Gate 3
- Edit: `workspec-server/.github/workflows/*.yml` — `cargo build -p wos-server` regenerates the snapshot; `git diff --exit-code work-spec/api/` fails CI if stale (activates PLN-0401e Gate 3)

**Phase A exit gate:** `cargo check -p wos-server` clean, idempotency-store unit tests pass, no domain handlers ratified yet.

### WS-1 Phase B — Domain handler implementations

Order minimizes cross-domain blast radius.

**B0.** Delete legacy notification render route (PLN-0402) — coordinates with B3 correspondence ratification. `crates/wos-server/src/http/notifications.rs` and `notifications_service.rs` lose the render path.

**B1. Auth (6 routes)** — first; nothing else blocks but transitively depends on `AuthCtx` shape stability
- File: `workspec-server/crates/wos-server/src/http/auth.rs`
- Idempotency: 3 of 11 endpoints (POST login, POST scope-swap, POST refresh)
- Conformance: `tests/integration/auth_conformance.rs`

**B2. Audit + Dashboard + Applicant (12 routes total)** — three-way parallel, low-LOC, no idempotency
- Files: `http/audit.rs` (new — split from existing surface), `http/dashboard.rs`, `http/applicant.rs`

**B3. Correspondence + Reports (10 routes)** — two-way parallel; correspondence absorbs legacy render route deletion
- Files: `http/correspondence.rs` (new), `http/reports.rs` (new)
- Idempotency: 2 of 11

**B4. Bundle + Governance (10 routes)** — two-way parallel
- Files: `http/bundles.rs`, `http/governance.rs`
- Idempotency: 2 of 11

**B5. Task (5 routes)** — depends on Auth + Idempotency
- File: `http/tasks.rs` (refactored — inline service moves to `services/task_service.rs`)
- Idempotency: 2 of 11 (draft, response)

**B6. Instance (16 routes)** — last, highest LOC, includes provenance subresource
- File: `http/instances.rs`
- Idempotency: 3 of 11 (migrate, event-emit, hold)

**Per-domain checklist** (every B step):
1. Replace handler bodies — return `crate::api::types::<domain>::*` directly
2. Add `#[utoipa::path(...)]` annotations on every public handler (PLN-0401c lands per-handler — every new handler is born annotated, no later sweep)
3. Strip `// adr-0082-d13: internal` markers; delete bridge-only routes
4. Apply idempotency middleware via `route_layer` on qualifying routes
5. Write per-domain conformance test `tests/integration/<domain>_conformance.rs` reusing the `ApiSchemaFamilyResolver` from existing `tests/integration/adr_0082_response_conformance.rs`
6. Verify: `cargo nextest run -p wos-server --test integration <domain>` green AND `cargo check -p wos-server` clean

### WS-1 Phase C — Cleanup + verification

**C1. `domain/*.rs` purge.** Delete entirely: `domain/applicant.rs`, `auth.rs`, `bundle.rs`, `dashboard.rs`, `governance.rs`, `instance.rs`, `provenance.rs`. Keep only internal model types — relocate `EvaluationResultView` (currently used in `lib.rs:33` cache state) to `crates/wos-server/src/runtime/eval_view.rs`. Then delete `domain/` directory entirely. Edit `crates/wos-server/src/lib.rs` line 10 — remove `pub mod domain;`.

**C2. Verification gates** (all green to declare done):
- `cargo check -p wos-server` clean
- `cargo nextest run -p wos-server --tests` all pass (10 new per-domain conformance tests + existing notification + 179 baseline = 190+ tests)
- Gate 3 (route coverage): `make wos-api-route-coverage` reports 0 internal markers
- Gate 4 (`$ref` discipline): green
- Gate 5 (oasdiff vs main): no breaking changes
- Gate 6 (mirror parity): green
- Gate 7 (response conformance): green per-domain
- PLN-0401e (snapshot staleness): `cargo build` regenerates snapshot; `git diff --exit-code work-spec/api/` clean
- `grep -r "impl From<crate::api::types" crates/wos-server/src` empty
- `ls crates/wos-server/src/domain` errors with "No such file" OR contains only internal types

---

## WS-2: Portal greenfield rebuild

### WS-2 Phase A — Type regen + portal-only types salvage

**A1. Run `npm run types:gen`** (closes PLN-0405)
- Auto-discovery in [`case-portal/scripts/generate-wos-types.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/scripts/generate-wos-types.ts) lines 35–44 picks up all 16 schemas in `work-spec/schemas/api/`
- Output: 16 `api-*.ts` modules at `case-portal/src/types/wos/` (was 3)
- Sharp edge: `_common.schema.json` produces `api-_common.ts` — verify generator filename mangling tolerates leading underscore. If not, rename schema to `common.schema.json` (one ADR D-1 file table edit, no semantic change)

**A2. Rewrite `src/ports/types.ts` as re-export barrel**
- File: [`case-portal/src/ports/types.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/ports/types.ts) (currently 421 lines)
- **Delete** (~23 API-shaped types — now generated): `Notification`, `NotificationFeedOptions` (already re-exported), `ProvenanceRecord`, `ProvenanceTier`, `ReasoningBlock`, `AINarrativeBlock`, `CounterfactualBlock`, `ActiveTaskView`, `TimerView`, `DelegationView`, `HoldView`, `CaseInstanceView`, `EvaluationResult`, `AvailableTransition`, `KernelSummary`, `TaskListItem`, `AgentView`, `DelegationEntry`, `PolicyVersionView`, `CalendarEventView`, `ServiceHealthView`, `DeonticConstraintView`, `QualityControlsView`, `PipelineStageView`, `PipelineView`, `EquityCategoryView`, `EquityRemediationTriggerView`, `EquityConfigView`, `DashboardMetrics`, `StageMetricView`, `AlertView`, `DriftDataPoint`, `PipelineDataPoint`, `ApplicantDeterminationView`, `AuthUser`, `SignatureProfileSummary`, `OutboundNotification`, `ReportTemplate`. Re-export from corresponding `api-*.ts`
- **Keep** (~14 portal-only types): `SortConfig`, `BackgroundJob`, `SavedView`, `BulkActionImpact`, `WosDocumentBundle` (composite kernel — UI-only), `InstanceFilter` (request envelope, not yet in API schema), `WosValidationResult`, `WosValidationIssue`, `ReportConfig` (request envelope)
- **Request shapes living portal-side until per-domain spec adds them** (stay colocated in their port interfaces, NOT in `types.ts`): `TaskDraft`, `FieldReviewDecision`, `TaskSubmissionResult`, `BatchTaskAction`, `BatchActionResult`, `HoldRequest`, `RelatedCaseView`, `CreateDelegationRequest`, `RegisterAgentRequest`, `ResolvedPolicyView`, `CorrespondenceSearchQuery`, `ReportGenerationRequest`, `GeneratedReport`, `ChainVerificationResult`, `ProvenanceSearchQuery`

**Sole structural transition** — `ProvenanceRecord` flat envelope → tier-discriminated union. Current portal shape is one envelope with optional `reasoning?`/`aiNarrative?`/`counterfactual?` blocks plus mandatory nested `actor: {id, type, name}`. Generated shape is a closed `oneOf` over `FactsRecord | ReasoningRecord | CounterfactualRecord | NarrativeRecord` keyed by `tier`, with URN-typed `actorRef`. Everything else is mechanical rename + import path change.

**Phase A exit gate:** `npm run types:check` no diff. `npx tsc --noEmit` shows the breakage list — that list IS the WS-2 Phase B work order.

### WS-2 Phase B — Port + adapter rewrite

**B1. Port interface migration** ([`case-portal/src/ports/I*.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/ports/) — 13 files)
- Type-import paths flip from `'../../ports'` to `'../../types/wos/api-*'`
- Method signatures change only on `ProvenanceRecord`-touching methods (`ICaseViewerPort.getProvenance/getTimeline`, `IAuditPort.getProvenance/searchProvenance`, `IWorkspacePort.submitEvent` returning `EvaluationResult` carrying optional `provenanceRecord`)
- Other 10 ports: import-path-only changes

**B2. HTTP adapter rewrite** ([`case-portal/src/adapters/http/ports.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/adapters/http/ports.ts) — 265 lines)
- Every method updates its return-type import to the typify-generated shape
- Stubs stay as `throw new NotImplementedError(...)` — they DO NOT switch on portal landing. When server WS-1 ratifies the endpoint, the stub flips to a real fetch with **zero portal-side type change** because the response is already the generated shape
- Stub inventory: `inbox.batchAction`, `workspace.recordFieldReview`, `caseViewer.getRelatedCases`, `signatureProfile` (all 4), `audit.searchProvenance`, `correspondence.listOutbound/getOutbound/sendNotification/retryNotification/exportLogs`, `reports` (all 5)

**B3. Fixture adapter rewrite** ([`case-portal/src/adapters/fixture/ports.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/adapters/fixture/ports.ts) — 435 lines)
- Every fixture literal reshapes to the generated shape
- Bulk of the diff: every `ProvenanceRecord` literal flattens from `{tier: 'reasoning', reasoning: {...}, actor: {id, type, name}}` to tier-variant flat shape `{tier: 'reasoning', actorRef: 'actor:human:cw-001', rulesApplied: [...], ...}`

**B4. `safeCall` audit** ([`case-portal/src/hooks/use*.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/hooks/) and component sites)
- Wrap calls to unratified endpoints (the stub inventory above) at the **caller** site, not in the port: `safeCall(audit.exportProvenance(...), new Blob())`. Ratified endpoints (bundle, inbox.list, workspace core, caseViewer core, governance, dashboard, applicant, auth, notification) stay un-wrapped

### WS-2 Phase C — Fixture conformance + component sweep + verification

**C1. New `tests/integration/fixture-conformance.test.ts`** (file: `case-portal/tests/integration/fixture-conformance.test.ts`)
- Mirror the existing [`adr-0082-response-conformance.test.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/tests/integration/adr-0082-response-conformance.test.ts) pattern (lines 84–117 expose `validateAgainstApiSchema(schemaBasename, defName, value)`)
- Loop every fixture method through its schema family — 10+ validation cases (bundle list, inbox list, workspace getInstance, caseViewer getProvenance, governance listAgents, dashboard getMetrics, applicant getDetermination, auth getCurrentUser, audit getProvenance, notification listNotifications)

**C2. Component spot-check**
- Highest-risk: `case-portal/src/components/audit/AuditViewer.tsx` — switches on `record.tier`. TS narrowing works in our favor; verify each switch arm body accesses tier-specific fields (e.g., `record.tier === 'reasoning' && record.rulesApplied` instead of legacy `record.reasoning?.rulesApplied`)
- Second-highest: anywhere `EvaluationResult.provenanceRecord` is consumed — likely `ActionBar.tsx` post-decision flow
- Other components consume non-structural types — `tsc --noEmit` will flag any narrowing issues; no design ahead-of-time required

**C3. Verification gates**
- `npm run build` clean
- `npm run types:check` no diff (PLN-0405 closed)
- `npx tsc --noEmit` clean
- `npx vitest run` all pass (existing + new `fixture-conformance.test.ts`)
- Fixture-mode dev server renders without runtime type errors (manual smoke)
- `grep -rn "WosBackend" case-portal/src` empty (already done per DI parity refactor; re-verify)
- `grep -nE "^export interface.*View|^export type.*View" case-portal/src/ports/types.ts` returns only kernel-composite types (`WosDocumentBundle`, `WosValidationResult`)

---

## WS-3: Cross-stack coordination

Submodule sequence — three commits across three submodules + parent pointer bump:

1. **work-spec** — if any schemas need touch-ups during WS-1/WS-2 (e.g., `_common.schema.json` rename per WS-2 A1 sharp edge). Most likely: zero schema commits — schemas are stable.
2. **workspec-server** — WS-1 bundle lands. CI runs `emit_openapi` binary, regenerating `work-spec/api/wos-public-api.openapi.json` via cross-submodule write. Submodule pointer commits in workspec-server include this regeneration.
3. **work-spec** (second commit) — pin the regenerated OpenAPI snapshot.
4. **case-portal** — WS-2 bundle lands. Submodule already consumes `work-spec/schemas/api/*.schema.json` directly via path coupling; pointer bump on work-spec.
5. **stack-root** — atomic commit advancing all three submodule pointers.

**WS-1 and WS-2 land independently** before this train assembles. Either can land first. The train is the synchronization point.

**Bookkeeping commits** (in the train):
- `case-portal/TODO.md` — Phase 1 (P1-01..P1-16) and Phase 2 (P2-01..P2-36) close as "subsumed by typify regen + service-trait reshape, see commit SHA"
- `PLANNING.md` — close PLN-0401, PLN-0402, PLN-0405; mark TODO-STACK ADR 0082 still-open boxes done
- `TODO-STACK.md` ADR 0082 row — flip "Still open" boxes to landed, add brief landing summary

---

## Critical files

**Server (5):**
- [`workspec-server/crates/wos-server/build.rs`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/build.rs) — utoipa derive activation
- `workspec-server/crates/wos-server/src/services/mod.rs` — service trait reshape (the unblock)
- `workspec-server/crates/wos-server/src/services/idempotency_store.rs` (new) — middleware backing
- `workspec-server/crates/wos-server/src/bin/emit_openapi.rs` (new) — OpenAPI auto-emission
- `workspec-server/crates/wos-server/src/lib.rs` — domain/ deletion + idempotency cache removal

**Portal (5):**
- [`case-portal/scripts/generate-wos-types.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/scripts/generate-wos-types.ts) — regen entry; verify `_common.schema.json` filename handling
- [`case-portal/src/ports/types.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/ports/types.ts) — barrel rewrite
- [`case-portal/src/adapters/fixture/ports.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/adapters/fixture/ports.ts) — provenance flattening
- [`case-portal/src/components/audit/AuditViewer.tsx`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/components/audit/AuditViewer.tsx) — discriminated-union narrowing
- `case-portal/tests/integration/fixture-conformance.test.ts` (new) — proof-of-correctness gate

**Cross-stack (1):**
- `work-spec/api/wos-public-api.openapi.json` — regenerated by emit binary; CI staleness gate enforces freshness

---

## Reused functions / patterns

- typify build resolver: [`workspec-server/crates/wos-server/build.rs`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/build.rs) substitute-and-feed (PLN-0403 path a, landed)
- Schema family pre-load + cross-`$ref` resolution: `ApiSchemaFamilyResolver` in [`workspec-server/crates/wos-server/tests/integration/adr_0082_response_conformance.rs`](/Users/mikewolfd/Work/formspec-stack/workspec-server/crates/wos-server/tests/integration/adr_0082_response_conformance.rs)
- Portal AJV pre-load: `loadApiSchemaFamily`/`buildAjvWithFamily` in [`case-portal/tests/integration/adr-0082-response-conformance.test.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/tests/integration/adr-0082-response-conformance.test.ts)
- Cross-submodule write pattern: [`scripts/generate-wos-core-references.py`](/Users/mikewolfd/Work/formspec-stack/scripts/generate-wos-core-references.py) (CLAUDE.md "Cross-stack scripts" §)
- `safeCall(promise, fallback)`: [`case-portal/src/ports/safeCall.ts`](/Users/mikewolfd/Work/formspec-stack/case-portal/src/ports/safeCall.ts) (DI parity refactor)
- Auth extractors (shape-agnostic, reusable): `crates/wos-server/src/auth/middleware.rs` `AuthCtx`, `RequireRole`, `Supervisor`, `Adjudicator`
- Error → RFC 7807 conversion: `crates/wos-server/src/error.rs` `ApiError → Problem` (already typify-driven)

---

## Verification (end-to-end)

After WS-3 train lands:

```bash
# Server
cd workspec-server
cargo check -p wos-server
cargo nextest run -p wos-server --tests
cargo build -p wos-server  # regenerates work-spec/api/wos-public-api.openapi.json
git diff --exit-code ../work-spec/api/  # PLN-0401e Gate 3

# Schema discipline
cd ../work-spec
python3 -m pytest tests/schemas -q  # 376+ pass, 1 xfail
node scripts/check-api-schema-validity.mjs  # Gate 1
python3 scripts/check-openapi-staleness.py  # Gate 2
python3 scripts/check-route-coverage.py  # Gate 3 — 0 internal markers
oasdiff breaking api/wos-public-api.openapi.json @main HEAD  # Gate 5
python3 scripts/check-api-mirror-parity.py  # Gate 6

# Portal
cd ../case-portal
npm run types:gen  # PLN-0405 — no diff
npm run types:check
npx tsc --noEmit
npx vitest run  # includes fixture-conformance + adr-0082-response-conformance
npm run build

# Manual smoke
npm run dev  # fixture mode renders all components without console errors
```

End-state grep checks (all should return empty):
```bash
grep -rn "// adr-0082-d13: internal" workspec-server/crates/wos-server/src
grep -r "impl From<crate::api::types" workspec-server/crates/wos-server/src
grep -rn "^export interface.*View" case-portal/src/ports/types.ts | grep -v "WosDocumentBundle\|WosValidationResult\|WosValidationIssue"
ls workspec-server/crates/wos-server/src/domain/  # error or only internal types
```

---

## Out of scope

- ADR 0068 D-3.1 (per-tenant authority shape) — auth handler implements against the ADR's Proposed shape per ADR 0082 D-15 greenfield discipline; promotion of ADR 0068 to Accepted is a separate effort
- Portal UI design refresh — components rebuild against new types but visual design unchanged
- Trellis integration beyond bundle export — Bundle handler composes existing trellis-export crate; no new Trellis work
- SDK generation (Python, mobile) — `openapi-generator` configured but not run; deferred until first external consumer
- Performance/benchmarking — gates verify correctness; performance work is post-bundle if needed
- ADR 0068 ratification, PLN-0381 (identity attestation), PLN-0406 follow-up Rust variants beyond what already landed

---

## Risk register

1. **typify `oneOf` ergonomics on tier-discriminated `ProvenanceRecord`** — the load-bearing risk per ADR 0082 Notes section. Mitigation: notification's `bundle-completed` discriminated union (PLN-0403 path-a) already proved the pattern; provenance follows the same shape
2. **utoipa derive propagation through typify-generated nested types** — PLN-0401 was deferred specifically because this was unproven. Mitigation: WS-1 A1 verifies `cargo check` clean before committing; if ToSchema doesn't propagate, fall back to per-type manual derives in `adapters.rs`
3. **Cross-submodule emit binary write timing** — server CI regenerates the OpenAPI snapshot; if this races with concurrent work-spec edits, the snapshot becomes a merge conflict. Mitigation: WS-3 train serializes the writes; emit binary is idempotent on stable handler annotations
4. **IdempotencyStore Postgres TTL sweeper** — 24-hour TTL via background sweeper attached to `services::timer_task`. If sweeper fails silently, table grows unbounded. Mitigation: alert on table size; fall back to per-request `DELETE WHERE created_at < now() - '24 hours'` in `record(...)` if monitoring unavailable
5. **Portal fixture reshape volume** — 435 lines of fixture code touch every API resource. Mitigation: `fixture-conformance.test.ts` lands in WS-2 C1 as the proof-of-correctness gate; fixture diff lands incrementally with green tests at each step
