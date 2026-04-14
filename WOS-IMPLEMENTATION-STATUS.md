# WOS Implementation Status & Roadmap

**Last updated:** 2026-04-13
**Status:** Certified Reference Implementation (Draft 1)

This document tracks crate maturity, test coverage, and the technical roadmap. For a high-level feature comparison, see `WOS-FEATURE-MATRIX.md`.

---

## 1. Crate Maturity Matrix

| Component | Status | Detail |
|-----------|--------|--------|
| **wos-core** | ✅ | Implements the typed evaluation kernel, deontic/autonomy modules, and explanation assembly. |
| **wos-lint** | ✅ | Executes 196 rules (36 T1 + 55 T2 + 105 T3) against typed models. |
| **wos-conformance** | ✅ | Manages 134 fixtures and handles Batch 16 processor reporting. |
| **wos-runtime** | ✅ | Orchestrates generic persistence, durable execution, and event queues. |
| **wos-formspec-binding** | ✅ | Implements the S15 protocol, task prefill, and response validation. |

---

## 2. Verification Progress (LINT-MATRIX)

WOS verifies 196 normative constraints across three tiers.

| Tier | Type | Rules | Verified | Gap |
|------|------|-------|----------|-----|
| **Tier 1** | Single-Doc | 36 | 36 | 0 |
| **Tier 2** | Cross-Doc / AST | 55 | 55 | 0 |
| **Tier 3** | Runtime | 105 | 95 | 10 |
| **Total** | | **196** | **186** | **10** |

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

### Phase 1: Engine Bindings (In Progress)
Adapts the reference evaluator to established engines.
*   [ ] **Camunda 8 Worker:** Delegates BPMN task execution to WOS governance.
*   [ ] **Temporal Workflow:** Maps WOS evaluation steps to deterministic replay.
*   [ ] **AWS Step Functions:** Bridges ASL states to WOS transitions.

### Phase 2: Advanced Provenance (Future)
*   [ ] **Merkle Provenance Chains:** Adds cryptographic hash-chaining for tamper-proof logs.
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

---

## Appendix B: Standards Lineage

*   **Adopted Intact:** WS-HumanTask, CMMN Case File, DMN (integration), CloudEvents, W3C PROV, JSON Schema.
*   **Adapted:** BPMN events (taxonomy), SCXML (statechart semantics), XACML (PEP/PDP), Catala (default logic), OpenFisca (temporal parameters), GSM (milestones), DCR (constraint zones).
*   **Evaluated but Rejected:** WS-BPEL, XPDL, YAWL, Azure Durable Functions, Netflix Conductor, Google Zanzibar.
