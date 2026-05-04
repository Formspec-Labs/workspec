# WOS Case Portal: Architecture Review & Remediation Handoff

**Date:** 2026-04-16
**Status:** Review complete, implementation pending
**Reviewer context:** Semi-formal code review + Formspec Architecture Scout perspective (product-minded systems architect, nothing assumed correct)

> **Renamed 2026-05-02:** previously `WOS Studio` / `@formspec-org/wos-studio` at `/studio`. Renamed to `WOS Case Portal` / `@formspec-org/wos-case-portal` at `/case-portal` to free the `/studio` path for the WOS Studio (Authoring) layer. Path references throughout this document are pre-rename; treat each occurrence of `wos-spec/studio/` as `wos-spec/case-portal/`.

---

## What This Is

A standalone React + Express application (`@formspec-org/wos-case-portal`, formerly `@formspec-org/wos-studio`) that serves as a visual tool for managing WOS (Workflow Orchestration Standard) kernel documents — specifically a state benefits management system called "GovFlow." Originally scaffolded by Google AI Studio, now being transitioned to a reference implementation with a real backend server.

**This document is the complete context needed to execute the 55-task remediation plan.** It includes all findings, the dependency-ordered task list, review gate specifications, and architectural guidance for the transition from fixture-backed PoC to reference implementation.

---

## Project Structure

```
wos-spec/studio/
  server.ts              — Express + Socket.IO server, serves kernel from JSON fixture
  src/
    App.tsx              — Root component, 10-view state machine (ViewState union)
    main.tsx             — Entry point, wraps app in WosProvider + ToastProvider + ErrorBoundary
    types.ts             — Shared types (Notification, DashboardMetrics — NOTE: duplicate of WosPorts)
    context/
      WosContext.tsx      — Dependency injection via React context, hex-architecture ports
      ToastContext.tsx    — Custom motion-based toast (DUPLICATE: sonner also used)
    services/
      WosPorts.ts         — Port interfaces (IInboxPort, IWorkflowDesignPort, IGovernancePort, etc.)
      WosBackend.ts       — Backend interfaces (IWosBackend, WosDocumentBundle)
      FixtureAdapter.ts   — In-memory fixture implementations of ALL ports
      SocketIORealtimePort.ts — Socket.IO client implementing IRealtimePort
      KernelToDesigner.ts — Bidirectional WOS kernel <-> designer model transform
    components/
      designer/
        WorkflowDesigner.tsx  — 1792-line monolithic designer component
      workspace/
        FormWorkspace.tsx    — Case workspace with resizable reference panel
        CaseForm.tsx         — Renders case fields from kernel caseFile + instance state
        ActionBar.tsx        — Decision bar (approve/reject/hold/submit)
      viewer/
        CaseViewer.tsx       — Case detail with 5 tabs (timeline, case-file, related, review-history, documents)
      dashboard/
        ProcessDashboard.tsx — Operations dashboard with KPIs, heatmap, drift chart
      admin/
        AdminConsole.tsx     — 1352-line admin console with 10 tabs
      ui/
        ConfirmationModal.tsx, ErrorBoundary.tsx
    types/wos/           — Auto-generated TypeScript types from JSON schemas (19 files)
  e2e/
    journeys/            — Playwright test specs (applicant, service-design, extended, mobile)
    pages/               — Page Object Model (InboxPage, DesignerPage, DashboardPage, etc.)
    utils/               — Test utilities (mobile-nav)
  scripts/
    generate-wos-types.ts — JSON Schema -> TypeScript type generator
```

**Parent workspace:**
```
wos-spec/
  schemas/              — WOS JSON Schema files (kernel, governance, AI, companions, etc.)
  fixtures/             — 40+ fixture JSON files for various WOS scenarios
  specs/                — WOS specification documents
  crates/               — Rust crates (formspec-wos, etc.)
```

---

## Architecture Pattern (Dependency Inversion)

The studio uses a **hexagonal architecture** via port/adapter pattern:

```
React Components
    ↓ (consume ports via React context)
WosContext (DI container)
    ↓ (injects port implementations)
WosPorts.ts (interfaces: IWosBackend, IInboxPort, IWorkflowDesignPort, etc.)
    ↓ (implemented by)
FixtureAdapter.ts (current)  →  HttpWosBackend (target)
```

**Key seam:** `WosProvider` accepts optional `ports` prop to swap implementations. The real backend transition means creating `HttpWosBackend implements IWosBackend` and swapping it in — zero component changes needed.

**This is the most important architectural invariant to preserve.** Every task in the remediation plan reinforces this seam rather than bypassing it.

---

## Findings Summary

### BLOCKER (2)

| ID | Finding | Location |
|----|---------|----------|
| A1 | `designerToKernel` loses compound/parallel state hierarchy on round-trip. Line 224: `states[localId] = state` extracts only the leaf key from dot-delimited IDs, flattening all nested states to top-level. A compound state with substates round-trips as disconnected atomic states. | `KernelToDesigner.ts:183-248` |
| T1 | E2E tests reference hard-coded IDs that don't match fixture data. `service-design.spec.ts:20` uses `'CASE-2026-89A2'` but fixture IDs are `'urn:wos:instance:benefits-adj:...'`. All E2E tests will fail at runtime. | `e2e/journeys/*.spec.ts` |

### WARNING (16)

| ID | Finding | Location |
|----|---------|----------|
| A2 | WorkflowDesigner is a 1792-line god component with 20+ useState hooks and 8 inline sub-components | `WorkflowDesigner.tsx:251` |
| A5 | IRealtimePort callback pattern is fragile — only one callback per event, no unsubscribe mechanism | `SocketIORealtimePort.ts:7-10` |
| C1 | `getDetermination` returns `'pending'` which is not in the decision union type, hidden by `as any` | `FixtureAdapter.ts:399-403` |
| C2 | Dashboard metrics use `Math.random()` — non-deterministic fixtures cause flaky rendering | `FixtureAdapter.ts:378,387` |
| C3 | `submitEvent` ignores all parameters — returns hardcoded result regardless of input | `FixtureAdapter.ts:160-162` |
| C4 | `getAvailableTransitions` returns same hardcoded transitions for ALL instances | `FixtureAdapter.ts:164-169` |
| C5 | `kernelToDesigner` accesses `kernel.lifecycle.states` without null guard | `KernelToDesigner.ts:106` |
| C7 | FormWorkspace hard-codes instance ID, ignores the `taskId` prop entirely | `FormWorkspace.tsx:51` |
| S1 | Server holds kernel state in mutable variable with no persistence | `server.ts:22,31` |
| S2 | CORS is `origin: "*"` with no authentication on any endpoint | `server.ts:16` |
| S3 | Socket.IO broadcasts kernel state without validation or conflict resolution | `server.ts:57` |
| T2 | E2E page objects use selectors that don't match rendered DOM (missing data-testids) | `e2e/pages/*.ts` |
| T3 | Zero tests for KernelToDesigner round-trip fidelity | — |
| T4 | Unit tests cover only loading/rendering, not interaction | `WorkflowDesigner.test.tsx` |
| X1 | Gemini API key exposed in client JS bundle via vite define | `vite.config.ts:12` |
| Y1 | Fixture casts use `as unknown as T` extensively, bypassing all type checking | `fixtures.ts:39-53` |

### NIT (11)

C6, C8, U1, U2, U3, U5, B1, B2, B3, B4, B5

### OBSERVATION (5)

A4, A8, C9, C10, B5

---

## Task Execution Plan

### Phase 0 — Interface Contracts (enables everything else)

These define the seams the real backend will implement. Do them first because every other task builds on clean interfaces.

| # | Task | ID | Priority |
|---|------|----|----------|
| 1 | Split IGovernancePort into read/write interfaces (IGovernanceReader / IGovernanceWriter) | A4 | medium |
| 2 | Rewrite IRealtimePort — use event emitter or subscription array with unsubscribe | A5 | high |
| 3 | Add IAuthPort to WosPorts — login/logout/getCurrentUser/role-based access | R4 | high |
| 4 | Deduplicate DashboardMetrics type — keep canonical in WosPorts.ts | Y3 | low |

**Review Gate 0:** Subagent semi-formal-code-review scoped to `WosPorts.ts`, `WosContext.tsx`, new `IAuthPort`, modified `IGovernancePort`, modified `IRealtimePort`. Verify: interface completeness, no missing methods from old interfaces, clean read/write separation, unsubscribe mechanism works.

---

### Phase 1 — Correctness Bugs (break demos, block real backend)

These produce wrong data. Fix before building a real adapter on top of broken transforms.

| # | Task | ID | Priority |
|---|------|----|----------|
| 5 | Fix designerToKernel round-trip data loss — preserve compound/parallel hierarchy | A1 | high |
| 6 | Add null guard in kernelToDesigner for missing kernel.lifecycle.states | C5 | high |
| 7 | Fix FormWorkspace hard-coded instance ID — use taskId prop | C7 | high |
| 8 | Fix 'pending' not in decision union type, remove `as any` | C1 | medium |
| 9 | Fix listDelegations `as any` cast — add delegations to governance type | Y2 | medium |
| 10 | Replace Math.random() with seeded/fixed values in fixture metrics | C2 | low |
| 11 | Make submitEvent simulate actual state transitions | C3 | medium |
| 12 | Make getAvailableTransitions instance-aware | C4 | medium |

**Review Gate 1:** Subagent semi-formal-code-review scoped to `KernelToDesigner.ts`, `FormWorkspace.tsx`, `FixtureAdapter.ts`. Trace the full `kernel → designer → kernel` path for a compound-state fixture. Verify `as any` casts are eliminated. Verify state transitions actually change instance state.

---

### Phase 2 — Test Infrastructure (safety net for remaining work)

| # | Task | ID | Priority |
|---|------|----|----------|
| 13 | Add KernelToDesigner round-trip tests for all fixture kernels | T3 | high |
| 14 | Fix E2E hard-coded IDs — align with fixture data or use dynamic selectors | T1 | high |
| 15 | Fix E2E page object selectors — add data-testid attributes to components | T2 | high |

**Review Gate 2:** Subagent semi-formal-code-review scoped to new test files + modified components (data-testid additions). Verify round-trip test assertions actually check structural equivalence (not just key count). Verify E2E selectors match rendered DOM. Run the test suite and confirm all tests pass.

---

### Phase 3 — Structural Refactor (makes codebase maintainable)

| # | Task | ID | Priority |
|---|------|----|----------|
| 16 | Extract WorkflowDesigner into separate files | A2 | medium |
| 17 | Add per-view ErrorBoundary wrappers in App.tsx | A3 | medium |
| 18 | Cap undo/redo history, debounce position updates | C6 | low |
| 19 | Replace `as unknown as T` fixture casts with schema-validated loading | Y1 | medium |
| 20 | Wire ActionBar.confirmDecision to backend.submitEvent | C9 | medium |

**Review Gate 3:** Subagent semi-formal-code-review scoped to all extracted component files, `App.tsx` ErrorBoundary changes, `fixtures.ts` schema validation. Verify extracted components maintain correct props/data flow. Verify ErrorBoundary actually catches errors in each view. Verify schema validation catches malformed fixture data.

---

### Phase 4 — Security Hardening (blocks any shared deployment)

| # | Task | ID | Priority |
|---|------|----|----------|
| 21 | Move Gemini API calls server-side — never expose key in client | X1 | high |
| 22 | Add auth middleware to server | S2 | medium |
| 23 | Add express.json size limit + schema validation on server endpoints | X2 | medium |
| 24 | Add schema validation + size limit on Socket.IO updates | S3 | medium |

**Review Gate 4:** Subagent semi-formal-code-review scoped to `server.ts`, `vite.config.ts`, new server-side AI route. Trace the API key path — confirm it never reaches client bundle. Verify auth middleware covers all endpoints. Verify input validation rejects oversized/invalid payloads.

---

### Phase 5 — Reference Backend Adapters (the actual transition)

| # | Task | ID | Priority |
|---|------|----|----------|
| 25 | Create HttpWosBackend implements IWosBackend — REST client adapter | R1 | high |
| 26 | Add schema validation at port adapter boundary | R2 | high |
| 27 | Define WosDocumentBundle as primary backend API contract | R9 | medium |
| 28 | Add transaction semantics to saveKernel | R10 | medium |
| 29 | Abstract real-time transport behind pluggable IRealtimePort | R6 | medium |
| 30 | Implement cursor-based pagination in inbox port + TaskList | R5 | medium |
| 31 | Add optimistic update + rollback pattern to ports | R3 | medium |
| 32 | Extract WOS types to shared @formspec-org/wos-types package | R7 | medium |

**Review Gate 5:** Subagent semi-formal-code-review scoped to `HttpWosBackend` (new), `WosPorts.ts` (modified), `WosDocumentBundle` type, real-time abstraction. Verify adapter faithfully implements `IWosBackend` with no missing methods. Verify schema validation catches malformed server responses. Verify transaction semantics handle failure cases (network error, validation failure, broadcast failure).

---

### Phase 6 — Test Expansion

| # | Task | ID | Priority |
|---|------|----|----------|
| 33 | Add WorkflowDesigner interaction tests | T4 | medium |
| 34 | Expand AdminConsole test coverage | T5 | low |
| 35 | Add integration test suite against HTTP API via port interfaces | R8 | medium |

**Review Gate 6:** Subagent semi-formal-code-review scoped to all new test files. Verify interaction tests exercise real DOM events (not mocked). Verify integration tests start server, seed data, and test through HTTP stack. Identify any remaining coverage gaps.

---

### Phase 7 — Polish (parallel, low risk)

| # | Task | ID | Priority |
|---|------|----|----------|
| 36 | Persist kernel to disk on server PUT | S1 | medium |
| 37 | Add error handling + graceful shutdown to server.ts | S4 | low |
| 38 | Load all fixture kernels in server, expose via bundle list API | S5 | low |
| 39 | Remove duplicate toast system — pick sonner, delete ToastContext.tsx | U1 | low |
| 40 | Make ActionBar AI fields count dynamic | U2 | low |
| 41 | Create z-index scale constants file | U3 | low |
| 42 | Add per-section loading skeletons | U4 | medium |
| 43 | Connect version comparison modal to real version data | U5 | low |
| 44 | Fix Header search | C8 | low |
| 45 | Make CaseForm inputs controlled | C10 | low |
| 46 | Remove vite from dependencies | B1 | low |
| 47 | Remove unused tsconfig options | B2 | low |
| 48 | Update index.html title | B3 | low |
| 49 | Rewrite README.md | B4 | low |
| 50 | Add GEMINI_API_KEY startup validation | B5 | low |

**Review Gate 7 (Final):** Subagent semi-formal-code-review — full regression sweep of all changed files. Verify no regressions from cleanup, no dead code remains, codebase is ready for reference backend connection.

---

## Key Architectural Invariants (DO NOT BREAK)

1. **Port interfaces are the only seam.** Components import from `WosContext`, never from adapter implementations. `WosPorts.ts` defines the contract; adapters implement it.

2. **`WosDocumentBundle` is the primary data contract.** The backend serves and accepts bundles (kernel + governance + AI + policy + etc.). This is the correct granularity.

3. **Adapters are swappable via `WosProvider` props.** `<WosProvider ports={{...}}>` injects all port implementations. The app should work identically with `FixtureAdapter` or `HttpWosBackend`.

4. **No component imports from adapter implementations.** `import { FixtureBackend } from '../services/FixtureAdapter'` should ONLY appear in `WosContext.tsx` (the DI container).

5. **Write operations need different handling than reads.** This is why IGovernancePort splits into read/write. Real backends need auth, optimistic updates, error rollback on writes — not on reads.

6. **Types come from schemas, not from hand-authoring.** The `generate-wos-types.ts` script compiles `schemas/*.json` → `src/types/wos/*.ts`. Never hand-edit generated types. If a type is wrong, fix the schema and regenerate.

---

## Subagent Review Instructions

Each review gate should be dispatched as a Task with `subagent_type: "general"` and the following framing:

```
You are performing a semi-formal code review of the WOS Studio project.
Load the semi-formal-code-review skill first.

Scope: [list specific files changed in this phase]

Context: This project is transitioning from a fixture-backed PoC to a reference
implementation with a real backend. The architecture uses hex-ports (dependency
inversion via WosPorts.ts interfaces). The key invariant is that components never
import from adapter implementations — only from port interfaces via WosContext.

Focus areas for this review:
- [2-3 specific concerns from the findings list above]

Original findings that motivated these changes:
- [paste relevant findings with file:line evidence]

Verify each finding was actually resolved. Cite file:line evidence for your claims.
Do not claim a finding is resolved without reading the changed code and tracing the
relevant data path.
```

---

## File Reading Guide (for incoming agents)

**Start here:**
1. `wos-spec/studio/package.json` — dependencies, scripts
2. `wos-spec/studio/src/services/WosPorts.ts` — all port interfaces (the most important file)
3. `wos-spec/studio/src/context/WosContext.tsx` — DI container
4. `wos-spec/studio/src/services/WosBackend.ts` — backend interfaces + WosDocumentBundle

**For the round-trip bug:**
5. `wos-spec/studio/src/services/KernelToDesigner.ts` — bidirectional transform
6. `wos-spec/fixtures/kernel/benefits-adjudication.json` — the main fixture kernel

**For the component decomposition:**
7. `wos-spec/studio/src/components/designer/WorkflowDesigner.tsx` — the 1792-line monolith
8. `wos-spec/studio/src/App.tsx` — view state machine

**For the test infrastructure:**
9. `wos-spec/studio/e2e/journeys/service-design.spec.ts` — E2E with wrong IDs
10. `wos-spec/studio/e2e/pages/InboxPage.ts` — page objects with wrong selectors
11. `wos-spec/studio/src/components/designer/WorkflowDesigner.test.tsx` — existing tests

**For the security issues:**
12. `wos-spec/studio/server.ts` — Express server
13. `wos-spec/studio/vite.config.ts` — API key exposure
