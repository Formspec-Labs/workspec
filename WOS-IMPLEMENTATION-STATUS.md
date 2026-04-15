# WOS Implementation Status & Roadmap

**Last updated:** 2026-04-14
**Status:** Certified Reference Implementation (Draft 1)

This document tracks crate maturity, test coverage, and the technical roadmap. For a high-level feature comparison, see `WOS-FEATURE-MATRIX.md`.

---

## 1. Crate Maturity Matrix

| Component | Status | Detail |
|-----------|--------|--------|
| **wos-core** | ✅ | Implements the typed evaluation kernel, deontic/autonomy modules, and explanation assembly. |
| **wos-lint** | ✅ | Executes 197 rules (36 T1 + 55 T2 + 105 T3 + I-001) against typed models. |
| **wos-conformance** | ✅ | Manages 144 fixtures and handles Batch 16 processor reporting. |
| **wos-runtime** | ✅ | Orchestrates generic persistence, durable execution, and event queues. |
| **wos-formspec-binding** | ✅ | Implements the S15 protocol, task prefill, and response validation. |
| **wos-assurance** | 🟡 | Spec complete; reference implementation pending. Attaches via provenanceLayer and custodyHook seams. |

---

## 2. Verification Progress (LINT-MATRIX)

WOS verifies 197 normative constraints across three tiers.

| Tier | Type | Rules | Verified | Gap |
|------|------|-------|----------|-----|
| **Tier 1** | Single-Doc | 37 | 37 | 0 |
| **Tier 2** | Cross-Doc / AST | 55 | 55 | 0 |
| **Tier 3** | Runtime | 105 | 105 | 0 |
| **Total** | | **197** | **197** | **0** |

**NB:** Tier counts above reflect the baseline kernel+governance+AI rule set. Assurance layer rules (S2.9, S4.9, S7.15, §14.x) add approximately 9 Tier 1/2 rules, to be authored alongside the reference implementation.

---

## 3. Reference Implementation Details

### Formspec Coprocessor (S15)
Runtime Companion S15 specifies the handoff between WOS tasks and Formspec forms.
*   **Handoff Protocol:** Orchestrated by `wos-runtime`.
*   **Mapping DSL:** Synchronizes `caseFile` data via `wos-formspec-binding`.
*   **Validation Ordering:** Prioritizes contract validation before assertion gates in `wos-core`.

### Durable Execution
`wos-core` implements runtime behavior while `wos-runtime` manages the persistence layer.
*   **Instance Loading:** Resolves kernel versions strictly.
*   **Atomic Saves:** Guarantees state consistency via the `RuntimeStore` interface.
*   **Timer Materialization:** Checks tolerances during simulated time advancement.

---

## 4. Technical Architecture & Standards Alignment

WOS employs a linked-data architecture to ensure interoperability and AI-safety.

### 4.1 Semantic Interoperability
*   **JSON-LD Native:** Every WOS document functions as a valid RDF graph.
*   **SHACL Governance Shapes:** Validates semantic constraints beyond JSON Schema limits.
*   **SPARQL-Queryable:** Supports cross-workflow analysis via standard RDF queries.

### 4.2 Architecture Principles
*   **Layered Opt-in:** Kernel, Governance, AI, and Advanced layers remain independently adoptable.
*   **Sidecar Document Pattern:** Isolates configuration into separate, updatable documents.
*   **Extension Seams:** Provides named attachment points for actors, contracts, and lifecycle events.
*   **Separation of Concerns:** Segregates lifecycle, case state, and audit logs.
*   **Conformance Profiles:** Offers incremental tiers for engine adoption.

### 4.3 AI-Native Patching
*   **Typed Patch Operations:** AI proposes edits as statically analyzable AST operations.
*   **Four-stage Validation:** Checks every AI edit for schema, SHACL, soundness, and provenance.

---

## 5. Engineering Roadmap

### Phase 1: Engine Bindings (§1 Reference Blockers Complete)
§1 reference implementation blockers are complete as of 2026-04-14. Remaining Phase 1 work is engine adapter bindings:
*   [ ] **Camunda 8 Worker:** Delegates BPMN task execution to WOS governance.
*   [ ] **Temporal Workflow:** Maps WOS evaluation steps to deterministic replay.
*   [ ] **AWS Step Functions:** Bridges ASL states to WOS transitions.
*   [x] **Integration Profile Processor:** CloudEvents 1.0 (`event-emit`, `event-consume`, `callback`), Arazzo multi-step sequences, tool invocations, and policy engine bridges all implemented in `wos-runtime`. 13 INT-* conformance fixtures green. (NB.3 + NB.4 complete)
*   [x] **Business Calendar SLA Evaluation:** `wos-business-calendar` sidecar consumed for Governance S10.3 SLA deadline computation; lazy evaluation at check time; `calendarVersion` snapshot; 4 G-S10-* fixtures green. (BC.1 complete)

### Phase 2: Advanced Provenance (Future)
*   [x] **History State Semantics:** DeepHistory (full state snapshot) and ShallowHistory (exit point only) implemented in `wos-core`. 9 K-H-* conformance fixtures covering depth-1, depth-2, depth-3, and parallel-exit re-entry. (KS.1 complete)
*   [x] **Milestone Firing:** Data-driven milestone firing independent of workflow state implemented in `wos-runtime`. Ordering pinned: data write durable → `MilestoneFired` → reactive transitions evaluated. 5 K-M-* conformance fixtures. (KS.2 complete)
*   [ ] **Merkle Provenance Chains:** Adds cryptographic hash-chaining for tamper-proof logs.
*   [ ] **Provenance Export Formats:** Serializes internal provenance to W3C PROV-O, OCEL 2.0, and IEEE 1849 XES for external tooling. `provenance.rs` implements 30+ provenance kinds; export serialization is the gap.
*   [ ] **Simulation Trace Format:** Standardizes formats for replaying simulation runs.
*   [ ] **Federation Profile:** Enables cross-processor migration and signal routing.

### Phase 3: Adoption Artifacts
*   [ ] Kubernetes and Serverless reference deployment patterns.
*   [ ] Processor certification narratives for procurement.
*   [ ] Sector-specific competitive analysis (Health, Defense, Benefits).

---

## Appendix A: Audit Corrections

| Feature | xlsx Rating | Corrected To | Reason |
|---------|------------|--------------|--------|
| Decision tables (DMN) | ■ | 🟡 (integration) | Kernel requires no embedded decision engine. |
| Decision requirement graphs | ■ | 🟡 (integration) | Defined by external integration only. |
| Merkle tree logging | ■ | 🟡 (per-record) | Published spec uses per-record digests; Merkle chains deferred. |
| RO-Crate packaging | ■ | ⚪ | Draft only; omitted from published specs. |
| MCP agent-tool protocol | ■ | -- (Formspec) | Specified as a Formspec package, not a WOS feature. |
| CaMeL dual-LLM | ■ | 🟡 (informative) | Specified as optional guidance in S3.6. |
| Capability routing | ■ | ⚪ | Remains implementation-defined in the kernel. |
| Defeasible rules | ■ | ✅ | Implemented via authority-ranked assembly. |
| Business calendar SLA | 🟡 | ✅ | Lazy evaluation at check time; `calendarVersion` snapshot; 4 G-S10-* fixtures. (BC.1) |
| CloudEvents binding | 🟡 | ✅ | `event-emit`, `event-consume`, `callback` with subject correlation and full envelope provenance; 6 INT-* fixtures. (NB.3) |
| Arazzo orchestration | 🟡 | ✅ | Per-step `invokeService` invocations with step-level provenance; pause/resume across sequence; 3 INT-ARAZZO-* fixtures. (NB.4) |
| Policy engine bridge | 🟡 | ✅ | `PolicyDecision` normalized to `{decision, reasons, obligations}` at binding boundary; OPA adapter; 4 INT-POLICY-* fixtures. (NB.4) |
| History states | 🟡 | ✅ | DeepHistory + ShallowHistory implemented; 9 K-H-* fixtures covering depth-1, depth-2, parallel-exit, depth-3. (KS.1) |
| Milestone firing | 🟡 | ✅ | Milestone firing with pinned ordering (write → MilestoneFired → transitions); 5 K-M-* fixtures. (KS.2) |
| PROV-O / OCEL / XES export | 🟡 | 🟡 (internal) | Internal provenance complete; export serialization not implemented. |

---

## Appendix B: Standards Lineage

*   **Adopted Intact:** WS-HumanTask, CMMN Case File, DMN (integration), CloudEvents, W3C PROV, JSON Schema.
*   **Adapted:** BPMN events (taxonomy), SCXML (statechart semantics), XACML (PEP/PDP), Catala (default logic), OpenFisca (temporal parameters), GSM (milestones), DCR (constraint zones).
*   **Evaluated but Rejected:** WS-BPEL, XPDL, YAWL, Azure Durable Functions, Netflix Conductor, Google Zanzibar.
