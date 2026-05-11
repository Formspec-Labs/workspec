# Validation: case-management.md

**Model:** zai-coding-plan/glm-5.1 (via opencode)
**Date:** 2026-05-10
**Subject:** `work-spec/thoughts/analysis/case-management.md`

---

## Overall Verdict

**Conceptually sound, technically grounded, needs targeted corrections and gap closure before ADR writing.**

Three parallel validation passes were run: conceptual alignment (vision, goals, ADRs), technical accuracy (schemas, specs, Rust code), and edge-case/gap completeness.

---

## 1. Conceptual Validation: PARTIAL (core thesis PASS, 3 critical gaps)

The central thesis — `CaseInstance` is a process instance, not a case aggregate — is **confirmed by the codebase** (Rust doc comment: "A running workflow instance"; kernel spec §11.1; API spec line 10). The proposed layering (Case above WOS) aligns with VISION.md's three-center architecture.

### 1.1 What passes

- **Vision alignment.** "WOS processes should be instruments operating on or within a Case. They should not be the Case" (line 140) aligns with VISION.md §III layer diagram positioning WOS as governance between Formspec and Trellis, and with `work-spec/CLAUDE.md` line 38: "WOS does NOT replace the workflow engine."
- **One portable case record.** The Case aggregate is designed to compose Formspec intake, WOS governance events, and Trellis integrity artifacts — matching VISION.md §V line 349.
- **Case initiation.** Correctly composes with ADR 0073 (WOS owns `case.created`; intake handoff produces artifacts that attach to Cases). Phase F (lines 367–374) correctly enumerates accepted, attachToExistingCase, and deferred modes.
- **ADR 0066 composition.** Edge case 12 explicitly states amendment/supersession concepts compose with CaseDecision lineage. Invariant 12 mirrors ADR 0066 D-3's verifier obligations.
- **ADR 0082 alignment.** Correctly adopts closed-taxonomy discipline, TypeID-in-URN identity, and `x-` vendor extension patterns. New `/api/v1/cases` routes follow URL versioning conventions.
- **No §VIII rejections triggered.** No FEEL/DMN/SHACL alternatives, dual-write, parallel hash chains, or other rejected anti-patterns proposed.
- **Architectural coherence.** Case positioned above WOS center — consumes WOS as governed process substrate, not replacement. Governed output paths (CaseStateMutation, CaseArtifact, CaseDecision, Timeline append) match existing kernel seams. No new kernel seams proposed.

### 1.2 Three gaps that must close before ADR

**Gap 1: Case aggregate ownership is unspecified.** The analysis doesn't state which center owns `Case`. VISION.md §VI and `work-spec/CLAUDE.md` line 106 establish "Case Ledger" (Trellis Core §1.2) as Trellis's term for the composed adjudicatory record, and WOS authors own `wos.*` event-type definitions. The analysis leaves ambiguous whether `Case` lives in WOS center, Trellis center, or is a new cross-cutting concept. VISION.md §VIII rejects "renaming or duplicating case ledger / Respondent Ledger / Subject Ledger / audit log in adjacent projects." **Recommendation:** Case is a WOS-center concept (new schemas in `work-spec/schemas/api/case.schema.json`, new Rust types in `wos-core`) that projects into Trellis's Case Ledger through the existing custody seam. This avoids creating a fourth center.

**Gap 2: Trellis Case Ledger relationship unaddressed.** VISION §VI line 437 defines the case ledger as composed sealed response-ledger heads + WOS governance events. The analysis mentions Trellis only in passing (line 855). The relationship must be pinned: one Trellis case ledger per Case; Case-level events outside any Process flow through `custodyHook` same as process events; case split opens new ledger per ADR 0066 supersession pattern.

**Gap 3: Prod-MVP phasing not explicit.** The 8-phase plan + 35 edge cases exceeds GOAL.md scope. GOAL.md line 243: "prefer work that makes the seed deployment more real." GOAL.md line 250: "one complete production cell over three aspirational deployment tiers." The ADR should explicitly phase: minimum for prod-MVP (Case exists, CaseInstance aliased as CaseProcess, `caseId` added, one-Case-one-Process), and full ontology (split/merge, multiple processes, artifacts/decisions as top-level resources) as post-MVP.

### 1.3 GOAL.md alignment

- Does not block any of the 7 critical-path steps. The separation is additive — introduces new concepts without requiring changes to existing WOS kernel semantics.
- Correctly scoped as pre-release work (line 298: "Because this is pre-release, prefer clean naming where possible").
- Recommended alias strategy (`pub type CaseProcess = CaseInstance;`, line 361) preserves backward compatibility.
- Risk: executing all 8 phases before prod-MVP would violate GOAL.md's task selection rule.

---

## 2. Technical Validation: PARTIAL (diagnosis accurate, field enumeration incomplete, 9 typos)

### 2.1 Schema accuracy

**Confirmed claims:**
- `$wosCaseInstance` marker exists (schema line 28-34), pinned to `"1.0"`
- Required fields `definitionUrl`, `definitionVersion`, `configuration`, `caseState`, `provenancePosition`, `timers`, `activeTasks`, `status`, `createdAt`, `updatedAt` all present
- `governanceState`, `volumeCounters` exist as optional properties
- `pendingEvents` exists (schema lines 381-400)

**Incomplete enumeration.** Analysis lists ~10 fields; schema + Rust have ~20+. Missing: `instanceId`, `tenant`, `historyStore`, `compensationLogs`, `stalledSince`, `declineReason`, `voidedAt`, `expiredAt`, `voidedBy`, `pendingCallbacks`, `extensions`, `nextTaskSequence`, `firedMilestones`.

**Status enum incomplete.** Analysis implies 6 values at line 555; actual has 9. Missing: `declined`, `voided`, `expired`.

**API vs runtime schema relationship.** Directionally correct but underspecified. `wos-case-instance.schema.json` is the kernel runtime artifact; `api/instance.schema.json` is the public API projection (ADR 0082 D-3). API schema's `$defs/CaseInstance` renames fields (`id` vs `instanceId`, `workflowUrl` vs `definitionUrl`, `lifecycleState` vs `status`), drops runtime-only fields, and adds API-only fields (`impactLevel`, `outcomeCode`, `milestonesFired`, `continuationOfServicesActive`, `dcrZones`, `correlationKey`). Any refactor must handle both surfaces.

### 2.2 Rust model accuracy

- `instance.rs` line 22: `CaseInstance` struct with doc comment "A running workflow instance (Runtime Companion S3.1)" — directly confirms the analysis's core claim.
- `case_state: serde_json::Value` (line 45) — confirms `caseState` is opaque JSON, not a typed case-domain struct. The naming collision concern is valid.
- `InstanceStatus` enum has 9 variants: `Active`, `Suspended`, `Migrating`, `Completed`, `Terminated`, `Stalled`, `Declined`, `Voided`, `Expired`.
- No separate `Case` struct or `CaseState` type exists anywhere in the codebase — confirming the gap the analysis identifies.

### 2.3 Spec accuracy

- Kernel spec §11.1 (line 1849-1851): "A CaseInstance is the serialization format for a running workflow instance." Directly confirms the analysis.
- Instance API spec line 10: "`CaseInstance` is the public projection of a running WOS workflow instance." Confirmed.
- "Files to inspect first" (lines 821-838): all 12 paths exist and are relevant. No incorrect paths.

### 2.4 Naming collision assessment

- Schema: `caseState` described as "Current case file field values. The keys are field names declared in the Kernel Document's caseFile.fields." — workflow-defined data, not case-domain.
- API schema: `CaseStateValue` described as "opaque JSON value (typically an object whose keys are case-file field names declared in the governing Workflow Document's caseFile.fields)." — again workflow-defined.
- Rust: `case_state: serde_json::Value` — untyped JSON.
- **Verdict:** The naming collision concern at lines 770-772 is valid and well-grounded.

### 2.5 Typographical/structural errors

| Line | Error | Correction |
|------|-------|------------|
| 92 | `├──  ├── notes` (double tree branch) | malformed tree syntax — should be `├── notes` on its own line |
| 128 | `├âme` | corrupted Unicode (mojibake) — likely intended field name like `├── mimeType` |
| 161 | `/casease-processes/{processId}/suspend` | `/case-processes/{processId}/suspend` |
| 324 | `CaseProcess  the process runtime object.` (missing `=`) | `CaseProcess = the process runtime object.` |
| 329 | `/specs/api/instae.md` | `/specs/api/instance.md` |
| 334 | `casschema.json` | `case.schema.json` |
| 347 | `processoutes` | `process routes` |
| 377 | `-pdate` | `Update` |
| 893 | `nexst` | `nextest` |

---

## 3. Edge Case & Gap Analysis: PARTIAL (strong first pass, 5 critical gaps)

### 3.1 Existing edge case quality

The 35 edge cases are well-structured: concrete scenarios, ordered simple-to-complex, most lead to specific design decisions. Edge cases 1-7, 17-20, 28 are the strongest. Edge cases 21, 27, 30, 31 are weaker (design notes rather than failure scenarios).

### 3.2 Critical missing edge cases

| # | Missing Edge Case | Severity | Rationale |
|---|---|---|---|
| M1 | **Case data model evolution (schema migration for case fields)** | CRITICAL | ADR 0071 pins version set at case open. When Case becomes a first-class aggregate with its own schema, what happens when that schema evolves? Case-level field additions, removals, type changes mid-case or across reopened cases are undefined. Case has its own durable state that evolves independently of any WorkflowDocument. |
| M2 | **Trellis case ledger binding — Case vs Trellis "case ledger" composition** | CRITICAL | VISION §VI and Trellis Core §1.2 define the case ledger as composed sealed response-ledger heads + WOS governance events. How does a Case relate to a Trellis case ledger? 1:1? Can a Case span multiple ledgers? When a case splits, does the ledger split? The mapping must be explicit. |
| M3 | **Process crash mid-case-mutation (durable execution recovery)** | CRITICAL | ADR 0070 defines Trellis local-append as commit point and `stalled` for exhausted retry. The analysis assumes atomic Case mutations. What happens when a process writes a `CaseStateMutation` and crashes after Trellis append but before Case aggregate's durable state updates? Two durable states (Case + Process) now inconsistent. Distinct from ADR 0070's single-instance crash. |

### 3.3 High-severity missing edge cases

| # | Missing Edge Case | Severity | Rationale |
|---|---|---|---|
| M4 | **Case-level time/SLA tracking vs process-level timers** | HIGH | ADR 0067 defines statutory clocks. Currently per-process. How do case-level SLAs compose across process boundaries? Does a case-level SLA pause when no process is active? |
| M5 | **Case transfer between organizations** | HIGH | Edge case 24 addresses tenant matching but not organizational transfer. ADR 0068 defines `(Tenant, DefinitionId, KernelId, LedgerId)` as case identity. Can a case transfer between orgs within a tenant? Between tenants? Implications for Trellis ledger scope, key-bag membership, provenance continuity. |
| M6 | **Bulk case operations (close, archive, reassign)** | HIGH | WOS TODO lists "Bulk Operations spec" as backlog. Analysis does not consider bulk operations. Bulk-close with active processes, bulk-archive with pending Trellis anchors, bulk-reassign with tenant consistency — exact scenarios where the Case/Process boundary matters most. |
| M7 | **Case archival and retention policy** | HIGH | Edge case 29 mentions archival briefly but does not address retention lifecycle. VISION §V defines lifecycle discipline per state type. What is retention policy for Case? For archived Case's CaseProcesses? How does archival interact with Trellis key-bag immutability and GDPR Art. 17 crypto-shredding? |
| M8 | **Evidence chain of custody (per VISION §V and ADR 0072)** | HIGH | VISION §V commits to evidence integrity. ADR 0072 defines evidence integrity and attachment binding. How does evidence move from Formspec response → CaseArtifact → Trellis anchor? Does evidence custody attach at Case or Process granularity? |
| M9 | **Case access after process completion (read-only process state?)** | HIGH | Process completion does not close Case. But what is the access model for a completed CaseProcess's state? Is it frozen? Re-materializable for audit? |
| M10 | **GDPR Art. 17 crypto-shredding impact on case artifacts** | HIGH | VISION §V and §VII commit to crypto-shredding via class-DEK destruction. When erasure request arrives for a Case's subject, what happens to CaseArtifacts, CaseDecisions, and CaseProcess state that reference that subject? How does Case aggregate track crypto-shredded artifacts? |
| M11 | **Legal hold on case artifacts** | HIGH | VISION §V lists legal hold as lifecycle state and mentions `legalHoldPlaced`/`legalHoldReleased`/`legalHoldDestructionRejected` Facts records. A case under legal hold must reject archival, reject crypto-shredding, and potentially reject certain CaseProcess transitions. |

### 3.4 Medium-severity missing edge cases

| # | Missing Edge Case | Rationale |
|---|---|---|
| M12 | **Process-to-process signaling within same case** | How do processes communicate within a case? Through the Case aggregate, correlation, or new mechanism? |
| M13 | **Case search/query/indexing** | Case-level search (by subject, status, participant, decision type) is a product concern distinct from process-level event routing. What projections are needed? |
| M14 | **Case data export/FOIA compliance** | Is Case export a projection of aggregate state, a Trellis export-bundle composition, or both? How does per-class encryption interact with FOIA export? |
| M15 | **Respondent/participant notification on case events** | Case-level notifications (subject notified of decision) distinct from process-level (participant notified of task assignment). Existing notification sidecar is process-scoped. |
| M16 | **Case state machine deadlock scenarios** | Case requires closure for statutory deadline (ADR 0067) but a process is stalled (ADR 0070 D-4.1). Case cannot close (active process policy), process cannot complete (stalled). Deadlock. |
| M17 | **Case-level vs process-level webhook events** | Should there be case-level webhooks (`case.statusChanged`, `case.decisionIssued`) distinct from process-level webhooks? |

### 3.5 Invariant assessment

**Weak invariants needing tightening:**

| Inv | Issue | Fix |
|-----|-------|-----|
| 5 | "Explicit or policy-derived" is ambiguous | Must enumerate closure policy types (as edge case 35 attempts) |
| 8 | "Governed outputs" is a term of art not defined here | Reference specific output types from the interaction model. Also: do governed outputs respect ADR 0074 access classes? |
| 14 | "Remain separable" — what does this mean concretely? | Define the interface boundary. If Case reads require joining with process state, they are not separable at query time. |
| 15 | "Rooted in Case" is product guidance, not mechanically testable | Reframe: "Public API MUST provide Case-scoped endpoints that do not require knowledge of CaseProcess lifecycle to retrieve Case status, artifacts, or decisions." |

**Missing invariants:**

| # | Missing Invariant | Severity |
|---|---|---|
| I1 | **CaseID never changes.** Case identity stability. Split/merge creates ambiguity about what "immutable" means. | CRITICAL |
| I2 | **Case provenance completeness.** Every case-level mutation (status change, artifact add, decision issue, relationship change) MUST produce a provenance record. Distinct from process-level provenance. | CRITICAL |
| I3 | **Case aggregate is the query root for case-domain views.** Case MUST be the aggregate root for queries about status, participants, decisions, artifacts. CaseProcess is query root for runtime state. | HIGH |
| I4 | **Cross-case reference integrity.** When Case A references Case B, the reference MUST remain valid even if Case B is closed, archived, or crypto-shredded. Relationship implies identity continuity, not data availability. | HIGH |
| I5 | **Case-to-Trellis ledger binding is 1:1.** Each Case corresponds to exactly one Trellis case ledger. If not 1:1, the mapping must be declared. | HIGH |
| I6 | **Evidence custody is Case-scoped, not Process-scoped.** `CaseArtifact.custodyRef` anchors to the Case's Trellis case ledger, not to any individual CaseProcess's provenance. | HIGH |
| I7 | **Case status transitions are append-only.** `open → on-hold → open → closed` produces a status history, not a status overwrite. | MEDIUM |

### 3.6 Missing acceptance criteria

| # | Missing Criterion | Severity |
|---|---|---|
| AC1 | **Case reopen preserves prior process outcomes and provenance.** No test asserts reopening does not mutate old process records. | HIGH |
| AC2 | **Case split produces lineage-preserving relationships.** No test asserts split-from/split-into relationships are created and original artifacts remain on source. | HIGH |
| AC3 | **Concurrent process writes to same case field are detected.** Edge case 20 identifies the concern; no test asserts conflict detection. | HIGH |
| AC4 | **IntakeHandoff creates Case via both `workflowInitiated` and `publicIntake` modes.** ADR 0073 defines two modes; no test asserts correct Case/Process boundary for both. | HIGH |
| AC5 | **Case status is NOT derived from CaseProcess lifecycle state.** Invariant 4 is the core separation; no test asserts that changing CaseProcess lifecycle state does NOT change Case status. | HIGH |
| AC6 | **Trellis case ledger anchors Case-level provenance records.** No test verifies Case-level events flow through `custodyHook`. | HIGH |
| AC7 | **Generated Case types are distinct from CaseProcess types.** Criterion 12 checks types are updated but does not verify distinct shapes. | MEDIUM |

### 3.7 Interaction with existing WOS features

| Feature | Assessment |
|---------|-----------|
| **DCR Constraint Zones** (Advanced Governance §4) | Edge case 27 underspecified. Zone activity provenance — Case-scoped or Process-scoped? Adaptive case management at Case level needs a provenance path distinct from DCR zone provenance inside a CaseProcess. |
| **Correlation Groups** (Kernel S6, S9.4) | Edge case 15 asks but does not decide. **Recommendation:** correlation remains process-scoped; Case-level event fan-out is a separate mechanism. |
| **IntakeHandoff** (ADR 0073) | Well-aligned. One decision needed: deferred intake should create a Case with `status: pending` (no CaseProcess) so deferred intake has a durable domain object to attach notes, evidence, or communications to. |
| **Trellis Case Ledger** | Weakest area. Must pin: one ledger per Case; process provenance goes on Case's ledger via `custodyHook`; Case-level events outside any Process also go on Case's ledger; case split opens new ledger. |
| **Signature Profile** | Correctly composes. Invariant 6 (process completion does not close Case) means completed signature process does not close case. Multiple parallel signing processes are correctly isolated by CaseProcess boundary. |

---

## 4. Recommended Next Steps

1. **Fix 9 typos** and complete `CaseInstance` field enumeration to include all current fields from schema and Rust
2. **Pin ownership:** Case is a WOS-center concept, projects into Trellis Case Ledger via `custodyHook`, not a fourth center
3. **Pin Trellis binding:** 1:1 Case-to-ledger, Case-level events flow through `custodyHook`, case split opens new ledger
4. **Pin MVP phasing:** minimum viable Case (exists, `CaseInstance` aliased as `CaseProcess`, `caseId` added, one-Case-one-Process); full ontology post-MVP
5. **Add critical edge cases** M1 (case schema evolution), M2 (Trellis ledger binding), M3 (dual-state crash recovery)
6. **Add critical invariants** I1 (CaseID immutability), I2 (Case provenance completeness)
7. **Add missing acceptance criteria** AC1–AC6
8. **Strengthen weak invariants** 5, 8, 14, 15 per the tightening guidance above
9. **Then** proceed to ADR writing per the document's Step 1
