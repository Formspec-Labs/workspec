# Multi-Agent Validation Report: Case Management Architecture

> **Superseded validation artifact.** This R-file is retained as derivation history. The controlling source of truth for case-management decisions is [`case-boundary-decision-report.md`](case-boundary-decision-report.md); when this file disagrees with that report, the report controls.

**Original Document:** `work-spec/thoughts/analysis/case-management.md`
**Validating Model:** Gemini (CLI)

---

I have orchestrated the conceptual and technical validation of `work-spec/thoughts/analysis/case-management.md` through the requested specialized agent personas. Here is the synthesized architecture analysis:

### 1. `formspec-scout` (Formspec Architecture Scout)
**Symptom:** The current system uses `CaseInstance` as the root product abstraction, leading to bloated `caseState` variables and a conflation of real-world matters with running workflow executions.
**Trace:** Formspec MCP tool params → studio-core helpers → core handlers → WOS formspec-binding evaluating against a single lifecycle instance.
**Root Domino:** The architectural decision to make the WOS workflow instance (`CaseInstance`) the top-level entity, meaning any data collection (Formspec responses) currently has to live inside or be strictly bound to a workflow lifecycle state.
**Product Impact:** Prevents standalone Formspec responses (e.g., self-service intake that doesn't immediately trigger a workflow) and forces multi-workflow scenarios to share a brittle, monolithic `caseState` map. 
**Recommendation:** **Approved.** The separation of `Case` (durable matter) from `CaseProcess` (workflow instance) is structurally correct. A Formspec `Response` perfectly maps to the new `CaseArtifact` concept. Formspec does not require a workflow to exist; this boundary correction accurately restores Formspec's standalone value.

### 2. `wos-scout` (WOS Architecture Scout)
**Symptom:** WOS Kernel (L0) and Governance (L1) boundaries were leaking into the product domain.
**Trace:** WOS runtime evaluator → `lifecycleHook` → transition tags. If `CaseInstance` = `Case`, completing the process closes the real-world matter.
**Root Domino:** Conflating L0 Kernel topology state (e.g. `active`, `suspended`, `completed`) with L0+ product domain state (e.g. `open`, `pending-appeal`, `closed`).
**Product Impact:** Restricts L2 AI Integration and L1 Governance. If an L2 agent wants to issue a decision, it currently mutates `caseState` directly. 
**Recommendation:** **Approved.** The proposed target architecture enforces the Layered-Sieve invariant perfectly. 
- *Crucial Sieve Alignment:* The document requires that "Workflow writes to Case MUST use declared governed outputs: CaseStateMutation, CaseArtifact creation, CaseDecision creation". This closes a major trust-boundary gap, ensuring L2 agents operate *under* L1 governance contracts when interacting with the real-world `Case`. 
- *Action Item:* Add to the implementation plan that the L0 Kernel `CaseInstance` (to become `CaseProcess`) MUST include `caseId` as a mandatory linkage field, and sidecars (e.g. `BusinessCalendar`) must retain their URL bindings to the `WorkflowDocument`.

### 3. `trellis-scout` (Trellis Architecture Scout)
**Symptom:** Cryptographic anchoring and determinism breaks when multiple processes mutate the same Case simultaneously.
**Trace:** `verify_export_zip` verifier failure → Trellis Core §10 `eventHash` Chain → WOS `EventStore` → Formspec Respondent Ledger.
**Root Domino:** The document proposes "Provenance can be per-process and per-case," but Trellis §10 requires a deterministic, linear chain (or formally bounded sub-chains) for content addressing. 
**Product Impact:** If the `Case` is separated from `CaseProcess`, what does an export ZIP represent? The "stranger-test" verifier needs a coherent package. If a Case has three concurrent `CaseProcess` executions, their provenance events cannot be loosely associated.
**Recommendation:** **Approved with a critical constraint.** 
- The Trellis chain MUST anchor at the `Case` level, not the `CaseProcess` level. 
- All `CaseProcess` executions, `CaseArtifact`s, and `CaseDecision`s attached to a `Case` must write their events into the single deterministic `Case` ledger stream (interleaved if concurrent). This guarantees that a single Trellis Export ZIP can cryptographically prove the entire real-world matter to an offline verifier. 

### 4. `spec-expert` (Formspec & WOS Canonical Specification Expert)
**Canonical Verification:**
- **WOS Kernel §10 (Hooks):** The document's proposal cleanly isolates `processState` from `caseState`. The WOS `contractHook` and `lifecycleHook` evaluation will need updated canonical prose to reflect that variables evaluate against `CaseProcess.processCaseState` vs `Case.state`.
- **WOS L2 & L3 (Governance):** `CaseDecision` perfectly aligns with the required governance output gates. 
- **Formspec §2.1 (Form Response Data) & §6.2 (Respondent Ledger):** Existing Formspec schemas (`response.schema.json`) will plug directly into `schemas/api/case-artifact.schema.json` without schema drift.
- **Inconsistencies Surfaced:** The spec currently uses `$wosCaseInstance` as an internal marker. The document wisely recommends keeping this marker temporarily for backward compatibility before a hard rename to `$wosProcessInstance` in the canonical schemas (`instance.schema.json`).

**Final Conclusion:**
The boundary refactor proposed in `case-management.md` is architecturally sound. It resolves a fundamental conflation between workflow state machines and real-world domain aggregates without requiring a rewrite of the WOS or Trellis substrates.
