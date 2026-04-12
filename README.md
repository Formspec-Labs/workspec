# WOS -- Workflow Orchestration Standard

**A governance specification for workflows where decisions affect people.**

Part of the [Formspec](https://formspec.org) project. Free and open source under the [AGPL-3.0 License](../LICENSE).

---

WOS governs how work moves through organizations -- government agencies processing benefits claims, insurers adjudicating coverage, hospitals managing care protocols, lenders underwriting loans. It answers the questions that matter when decisions affect people: who acted, what rules applied, whether governance held, and what happens when something breaks.

WOS is a governance standard, not an execution engine. It defines what constraints must hold. Existing engines -- Camunda, Temporal, Apache KIE, Flowable -- enforce them. You own the logic. They manage the execution.

## Architecture

```text
PARALLEL SEAMS (cross-cutting, attach at any layer via existing kernel mechanisms)
+-----------------+------------------+
|  Integration     |  Semantic         |
|  (Arazzo, CWL,   |  (JSON-LD, SHACL, |
|   CloudEvents,   |   PROV-O, XES)    |
|   Policy Engines)|                   |
+--------+--------+--------+---------+
         v                 v

VERTICAL LAYERS (each optional, each builds on the one below)
+============================================================+
|  Layer 3: ADVANCED GOVERNANCE (optional)                    |
|  DCR constraint zones - Equity guardrails - SMT verification|
|  Multi-step sessions - Tool governance - Agent lifecycle    |
|  Calibration - Drift detection - Shadow mode - Circuit break|
+============================================================+
|  Layer 2: AI INTEGRATION (optional)                         |
|  Agent registration - Deontic constraints - Autonomy levels |
|  Confidence framework - Fallback chains - Drift detection   |
|  Formspec-as-validator - Disclosure - Narrative provenance  |
+============================================================+
|  Layer 1: WORKFLOW GOVERNANCE (optional)                    |
|  Due process - Review protocols - Data validation pipelines |
|  Structured audit - Quality controls - Task management      |
|  Delegation of authority - Typed holds - Temporal parameters|
+============================================================+
|  Layer 0: KERNEL (required)                                 |
|  Lifecycle topology - Case state - Actor model - Provenance |
|  Impact level - Contract validation - Durable execution     |
|  5 named seams - Semantic tags - Case relationships         |
+============================================================+

EXECUTION SUBSTRATE (external -- WOS governs, engines execute)
+------------------------------------------------------------+
|  Camunda / Temporal / Apache KIE / Flowable / custom       |
|  State persistence - Timers - Crash recovery - Replay      |
+------------------------------------------------------------+

FORMSPEC SUBSTRATE
+------------------------------------------------------------+
|  Definitions - FEL - Mapping DSL - Screener - Assist       |
|  References - Ontology - Respondent Ledger - Registry      |
+------------------------------------------------------------+

COMPANION
+------------------------------------------------------------+
|  Lifecycle Detail: transition algorithm, compensation (saga)|
|  history states, parallel execution, timers, SCXML mapping |
+------------------------------------------------------------+
```

### Why not build our own engine?

Temporal, Camunda, and KIE have spent years hardening state persistence, timer durability, crash recovery, and deterministic replay. WOS adds what they lack: deontic constraints on AI agents, structured human oversight protocols, due process for adverse decisions, four-layer audit with epistemic status, and formal verification of governance properties. No existing engine offers these capabilities, and none can bolt them on -- they require structural changes to how transitions are evaluated and violations are recorded.

The deployment model:

| Concern | Who handles it |
|---------|---------------|
| **Governance logic** (what's allowed, what's required, what audit trail is produced) | WOS |
| **Execution infrastructure** (state persistence, timers, crash recovery, replay) | Camunda / Temporal / KIE / Flowable |
| **Intake and data collection** (forms, validation, AI-assisted filling) | Formspec |
| **Decision logic** (business rules, eligibility, policy evaluation) | DMN / OPA / Cedar / OpenFisca (via Integration Profile) |

WOS bindings -- interceptors, task listeners, job workers -- enforce governance on each engine. An agency running Camunda gets WOS governance. An agency running Temporal gets the same governance. The governance is portable; the execution is not.

### Kernel seams

Dependencies flow downward only. Higher layers attach through five named kernel seams:

| Seam | Purpose | Who attaches |
|------|---------|--------------|
| `actorExtension` | Register actor types | L2 registers `agent`; L3 adds lifecycle states |
| `contractHook` | Inject data validation | L1 pipelines; L2 Formspec-as-validator; Integration Profile policy engines |
| `provenanceLayer` | Add audit tiers | L1 adds Reasoning + Counterfactual; L2 adds Narrative |
| `lifecycleHook` | Attach governance to transitions | L1 due process, review, quality, delegation, holds; L2 deontic, oversight, sampling; L3 equity, drift, circuit breaker |
| `extensions` | Escape hatch | L3 binds constraint zones via `x-constraintZoneRef` |

The kernel tags transitions with their nature (`review`, `determination`, `adverse-decision`, `hold`). Governance documents declare rules matching those tags. Add a new review step, tag it `review`, and existing review protocols apply automatically.

## Specification

18 specs, 18 schemas. Every document validates against its schema.

Schemas live under `schemas/` grouped like the normative specs: `kernel/`, `governance/`, `sidecars/`, `ai/`, `advanced/`, `profiles/`, and `companions/`. Published `$id` URIs (for example `https://wos-spec.org/schemas/kernel/1.0`) are unchanged.

| Layer | Spec | Schema |
|-------|------|--------|
| Kernel | [`spec.md`](specs/kernel/spec.md) | [`wos-kernel`](schemas/kernel/wos-kernel.schema.json) |
| Kernel sidecar | [`correspondence-metadata.md`](specs/kernel/correspondence-metadata.md) | [`wos-correspondence-metadata`](schemas/kernel/wos-correspondence-metadata.schema.json) |
| Governance | [`workflow-governance.md`](specs/governance/workflow-governance.md) | [`wos-workflow-governance`](schemas/governance/wos-workflow-governance.schema.json) |
| Governance sidecar | [`due-process-config.md`](specs/governance/due-process-config.md) | [`wos-due-process`](schemas/governance/wos-due-process.schema.json) |
| Governance sidecar | [`assertion-library.md`](specs/governance/assertion-library.md) | [`wos-assertion-gate`](schemas/governance/wos-assertion-gate.schema.json) |
| Governance sidecar | [`policy-parameters.md`](specs/governance/policy-parameters.md) | [`wos-policy-parameters`](schemas/governance/wos-policy-parameters.schema.json) |
| Governance sidecar | [`business-calendar.md`](specs/sidecars/business-calendar.md) | [`wos-business-calendar`](schemas/sidecars/wos-business-calendar.schema.json) |
| Governance sidecar | [`notification-template.md`](specs/sidecars/notification-template.md) | [`wos-notification-template`](schemas/sidecars/wos-notification-template.schema.json) |
| AI Integration | [`ai-integration.md`](specs/ai/ai-integration.md) | [`wos-ai-integration`](schemas/ai/wos-ai-integration.schema.json) |
| AI sidecar | [`agent-config.md`](specs/ai/agent-config.md) | [`wos-agent-config`](schemas/ai/wos-agent-config.schema.json) |
| AI sidecar | [`drift-monitor.md`](specs/ai/drift-monitor.md) | [`wos-drift-monitor`](schemas/ai/wos-drift-monitor.schema.json) |
| Advanced | [`advanced-governance.md`](specs/advanced/advanced-governance.md) | [`wos-advanced`](schemas/advanced/wos-advanced.schema.json) |
| Advanced sidecar | [`equity-config.md`](specs/advanced/equity-config.md) | [`wos-equity`](schemas/advanced/wos-equity.schema.json) |
| Advanced sidecar | [`verification-report.md`](specs/advanced/verification-report.md) | [`wos-verification-report`](schemas/advanced/wos-verification-report.schema.json) |
| Profile | [`integration.md`](specs/profiles/integration.md) | [`wos-integration-profile`](schemas/profiles/wos-integration-profile.schema.json) |
| Profile | [`semantic.md`](specs/profiles/semantic.md) | [`wos-semantic-profile`](schemas/profiles/wos-semantic-profile.schema.json) |
| Companion | [`lifecycle-detail.md`](specs/companions/lifecycle-detail.md) | [`wos-lifecycle-detail`](schemas/companions/wos-lifecycle-detail.schema.json) |
| Runtime | [`runtime.md`](specs/companions/runtime.md) | [`wos-case-instance`](schemas/companions/wos-case-instance.schema.json) |

## Example

A minimal kernel document -- a purchase order approval with three states and two actors:

```json
{
  "$wosKernel": "1.0",
  "url": "https://agency.gov/workflows/purchase-order-approval",
  "version": "1.0.0",
  "status": "active",
  "impactLevel": "operational",
  "actors": {
    "requester": { "type": "human", "description": "Submits purchase requests" },
    "approver":  { "type": "human", "description": "Reviews and approves requests" }
  },
  "lifecycle": {
    "initialState": "submitted",
    "states": {
      "submitted": {
        "type": "atomic",
        "transitions": [
          { "event": "approve", "target": "approved", "guard": "caseFile.amount <= 5000" },
          { "event": "reject",  "target": "rejected" }
        ]
      },
      "approved":  { "type": "final" },
      "rejected":  { "type": "final" }
    }
  },
  "caseFile": {
    "fields": {
      "amount":      { "type": "number" },
      "description": { "type": "string" },
      "requestDate": { "type": "date" }
    }
  }
}
```

This validates against `schemas/kernel/wos-kernel.schema.json`. No governance, no AI, no advanced features -- just orchestration. Add layers when the workflow demands them.

## What each layer provides

### Layer 0: Kernel -- What happens to a case?

The kernel defines the path work takes. A benefits application moves from intake to review to determination to notice. The kernel tracks where every case is, what data it carries, who touched it, and how cases relate to each other. Every action produces a provenance record. Two systems given the same document and the same events produce the same state transitions -- the lifecycle is deterministic.

### Layer 1: Governance -- What protections apply?

A caseworker denies a benefits claim. Governance ensures the affected person receives written notice with individualized reasons and instructions for appeal to an independent adjudicator. A reviewer evaluates a case using dual-blind protocol -- forming an independent assessment before seeing anyone else's recommendation. Delegation of authority ensures the person signing the determination holds authorization within their scope.

Data arrives from an external source -- income verification, property appraisal, medical records. A validation pipeline runs it through assertion gates: does the extracted value match the source document? Does the total equal the sum of components? Failed gates route to structured remediation.

Every determination produces a reasoning trace: which rules applied, which evidence was consulted, which thresholds governed the decision. For adverse decisions, a counterfactual trace records what the applicant could change to qualify and confirms that protected characteristics played no part in the outcome.

### Layer 2: AI Integration -- What keeps the agent honest?

A document extraction agent reads pay stubs and pulls structured data. The system validates its output against the same Formspec contract a human would submit against. A prohibition prevents the agent from issuing final denials on high-stakes cases. An obligation requires every extracted value to cite its source location.

The agent operates at assistive autonomy: it recommends, a human confirms. Confidence below 0.80 triggers escalation. Failure triggers a fallback chain: retry once, then create a human task. Accuracy drift over 30 days triggers demotion to a lower autonomy level until recalibration.

Adverse decision notices disclose that an AI system assisted, consistent with OMB M-24-10 and EU AI Act Article 13.

### Layer 3: Advanced Governance -- What keeps adaptive work safe?

A fraud investigation opens. The investigator interviews witnesses, requests documents, consults experts, issues subpoenas -- in any order, subject to constraints. The final report requires at least one completed interview. Every interview triggers an obligation to request supporting documents. These are DCR (Dynamic Condition Response) relations adapted from Hildebrandt and Mukkamala's work, proven at government scale in Danish central government.

Equity guardrails monitor outcome rates across demographic groups. They run asynchronously -- they never block individual cases, because aggregate disparity does not indict any single decision. When disparity exceeds a threshold, the equity officer receives an alert.

A subset of deontic constraints submits to formal verification by an SMT solver before the workflow runs. The report states: "the prohibition on agent final denial holds for all possible inputs" (proven safe) or "this constraint references a temporal parameter unresolvable at static time" (inconclusive -- runtime enforcement still applies).

### Integration Profile -- How does the workflow talk to external systems?

A workflow checks eligibility against a policy engine -- OPA for a benefits program, Cedar for a lending platform. The profile maps case data into the engine's input format, calls it, and maps the permit/deny decision back. Seven binding types: request-response (OpenAPI), multi-step API orchestration (Arazzo), tool invocation, event-emit, event-consume, callback, and policy engine bridge.

### Semantic Profile -- How does meaning travel beyond WOS?

A JSON-LD context maps every WOS property to an RDF term. Provenance records become PROV-O triples queryable from any SPARQL endpoint. SHACL shapes enforce that high-stakes workflows carry due process and that agent provenance records include required fields. Process mining tools consume XES exports derived from provenance.

### Lifecycle Detail Companion -- What does the engine follow?

The kernel declares a deterministic pure function. The companion supplies the pseudocode: guard evaluation in document order, exit and entry paths through nested states, atomic parallel region activation, independent event routing to concurrent regions. Compensation in reverse order, the pivot step excluded. History states resume compound states where they left off. A bidirectional SCXML mapping enables interoperability with existing statechart engines.

## Composition

| What you need | What you adopt |
|---------------|----------------|
| Track work through states with actors and audit trail | Kernel |
| Governed human workflows with due process and review protocols | Kernel + Governance |
| AI-assisted workflows with agent constraints and confidence routing | Kernel + Governance + AI Integration |
| Adaptive case management, equity monitoring, formal verification | + Advanced Governance |
| External system integration (APIs, policy engines, events) | + Integration Profile |
| Linked data, PROV-O export, process mining | + Semantic Profile |
| Detailed execution engine guidance | + Lifecycle Detail |

Every layer except the kernel is optional. A purchase order needs only the kernel. A rights-impacting benefits adjudication needs the kernel and governance. AI plugs into governance structures that already exist for humans -- it has no separate track.

## What no competitor offers

WOS was designed after surveying 50+ standards, specifications, and platforms (BPMN, CMMN, SCXML, Temporal, DMN, XACML, W3C PROV, and others). Ten capabilities exist in no competing system:

1. **Decision provenance with authority ranking.** Every determination records which rules fired, ranked by authority: statute, regulation, policy, guideline. No workflow standard tracks decision rationale at this granularity.

2. **Temporal parameter versioning.** Policy values (eligibility thresholds, rates, limits) are indexed by effective date. The system applies the rules in effect when the case was filed, not today's rules. Only OpenFisca does this, and it is a microsimulation tool, not a workflow standard.

3. **Structured human override with accountability.** Overrides require rationale, authority verification, and supporting evidence -- all permanent record. Existing systems either block overrides or allow them without accountability.

4. **Five empirically-grounded review protocols.** Independent-first (form judgment before seeing the recommendation), consider-opposite, calibrated confidence display, dual-blind, and unassisted. Based on published research on automation bias. No platform implements structured cognitive debiasing.

5. **Deontic constraints on AI agents.** Permission, prohibition, obligation, and right -- adapted from LegalRuleML -- governing agent behavior with fixed enforcement ordering and impact-dependent null propagation.

6. **Mandatory graceful degradation.** Every workflow using agents must function when agents are unavailable. Fallback chains are validated at document load time. Agent failure is a normal operating condition.

7. **AI confidence decay and cumulative tracking.** Agent output confidence degrades as underlying data changes. Multi-step interactions track compounding error. When combined confidence drops below the floor, the system pauses for human review.

8. **Rubber-stamp detection.** Statistical monitoring flags when reviewers blindly accept agent recommendations -- short review times, high agreement rates, low modification rates.

9. **Equity guardrails for human and AI decisions.** Disparity monitoring applies to all decision-makers. Bias is a human problem too.

10. **Parallel completion policies.** Wait-all, cancel-siblings, and fail-fast control what happens when concurrent tracks finish at different times. Only SCXML supports parallel regions natively; none offer completion policies.

See [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) for the full competitive comparison across 16 platforms.

## How the documents relate

Four patterns: vertical layers, sidecars, parallel seams, and companions. Every document is one of these.

**Vertical layers.** Each layer targets the kernel by URL. Layer 1 says "I govern this workflow." Layer 2 says "I add AI to this workflow." Neither requires the other -- Layer 2 works without Layer 1 for low-stakes automation. But rights-impacting workflows effectively require Layer 1 because due process lives there. Dependencies flow downward only. The kernel knows nothing of governance, AI, or advanced concepts.

**Sidecars.** A sidecar enriches its parent without affecting processing. Each carries its own `$wos*` marker, version, and lifecycle. Sidecars exist for independent update cadence. The eligibility threshold changes every January -- update Policy Parameters, leave governance untouched. The agent's calibration schedule shifts -- update Agent Config, leave AI Integration untouched.

```text
Kernel         <-- Correspondence Metadata, Business Calendar, Notification Template
Governance     <-- Policy Parameters, Due Process Config, Assertion Library
AI Integration <-- Agent Config, Drift Monitor
Advanced       <-- Equity Config, Verification Report
```

**Parallel seams.** The Integration Profile connects workflows to external APIs and policy engines whether or not AI is involved. The Semantic Profile adds linked data interpretation to any WOS document regardless of adopted layers. They use existing kernel mechanisms and introduce no new extension points.

**Companion.** The Lifecycle Detail elaborates kernel semantics without adding concepts. The kernel declares a deterministic lifecycle; the companion provides the algorithm.

## Conformance Fixtures

24 fixtures demonstrate every layer and composition:

| Fixture | Layer | What it proves |
|---------|-------|----------------|
| Purchase order approval | Kernel | Kernel stands alone -- 3 states, 2 actors, zero governance |
| Medicaid redetermination | Kernel | Cyclical lifecycle -- periodic review without a `final` state |
| Benefits adjudication (kernel) | Kernel | Full workflow with parallel dual-blind review |
| Case relationship appeal | Kernel | Parent/child case link with correlationKey interaction |
| Benefits correspondence metadata | Kernel sidecar | 9 correspondence entry templates |
| Benefits business calendar | Governance sidecar | Federal holiday schedule, Mon-Fri work week, operating hours |
| Benefits notification templates | Governance sidecar | Adverse decision, hold entry, appeal acknowledgment notices |
| Benefits adjudication (governance) | L1 | Due process, review protocols, pipelines, audit, quality |
| Benefits policy parameters | L1 | Temporal parameters + regulatory version bindings |
| Benefits AI integration | L2 | 2 agents, deontic constraints, Formspec-as-validator, fallback |
| Benefits advanced governance | L3 | Fraud investigation DCR zone, equity guardrails, SMT verification |
| Benefits equity config | L3 sidecar | Protected categories, disparity methods, remediation triggers |
| Verification report | L3 sidecar | SMT results: 2 proven-safe, 1 inconclusive |
| Integration profile | Profile | 7 binding types: Arazzo, CWL, CloudEvents, OPA policy engine |
| Semantic profile | Profile | JSON-LD context, SHACL shapes, PROV-O mapping, XES export |
| Lifecycle detail | Companion | Compensation config, timer config, SCXML mapping |
| 4 LLM-authoring tests | Validation | Sonnet authors valid documents from schemas alone |
| 8 invalid documents | Validation | Expected schema rejections |

## Implementation

Three Rust crates implement WOS:

| Crate | Purpose | Status |
|-------|---------|--------|
| `wos-core` | Typed domain models, evaluation algorithm, 9 host interface traits | Phase 3 -- typed models for kernel, governance, AI, and sidecars; evaluation algorithm implemented; migrating conformance engine |
| `wos-lint` | Static analysis of WOS documents | 189 rules (30 T1 single-doc + 50 T2 cross-doc + 101 T3 dynamic). 76 tested. |
| `wos-conformance` | Dynamic conformance test runner | 194 tests passing. Migrating to wos-core typed models. |

Engine bindings (Camunda, Temporal, KIE, Flowable) are planned but not started. The current priority is completing the wos-core typed model extraction and writing conformance fixtures for untested governance rules. See [`TODO.md`](TODO.md) for the full implementation roadmap.

### Critical gap: Formspec Coprocessor

No specified handoff protocol exists between Formspec forms and WOS workflows. When a Formspec form submission completes, the system must create a WOS case instance, map response data to case file fields, and validate the response before firing the workflow event. This protocol -- the Formspec Coprocessor -- is the missing piece connecting intake to governance. It blocks the enterprise SaaS roadmap and is the highest-priority spec to write next.

## Companion Documents

| Document | Purpose |
|----------|---------|
| [`TODO.md`](TODO.md) | Implementation roadmap: 7 phases from spec prose through engine capabilities |
| [`LINT-MATRIX.md`](LINT-MATRIX.md) | All 189 lint rules with test status and spec citations |
| [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) | Competitive comparison against 16 platforms with implementation status and audit corrections |
| [`enterprise-feature-gaps.md`](enterprise-feature-gaps.md) | Formspec SaaS platform gaps vs. enterprise competitors (Adobe, ServiceNow, DocuSign) |
| [`enterprise-implementation-roadmap.md`](enterprise-implementation-roadmap.md) | 6-phase SaaS build plan with WOS governance dependencies |
| [`wos-formspec-competitive-feature-matrix.xlsx`](wos-formspec-competitive-feature-matrix.xlsx) | Full 16-competitor spreadsheet with strategic analysis and open source engine comparison |

## Status

WOS is part of the Formspec project, maintained by Michael Deeb under AGPL-3.0. The specification is pre-release with no production deployments.

**What exists:** 18 specs, 18 schemas, 24 conformance fixtures covering all four vertical layers, two parallel seam profiles, one companion, one runtime companion, and five governance sidecars. 189 lint rules. 194 passing tests. All schemas pass LLM-authoring validation (an LLM given only the schema and a one-paragraph workflow description produces valid, semantically correct documents). Seven rounds of semi-formal code review have been completed with all findings addressed.

**What does not exist:** Production deployments. Engine bindings. A conformance test suite beyond fixture validation. The Formspec Coprocessor handoff protocol. Formal accessibility or compliance audits (WCAG, FedRAMP, NIST 800-53). An organizational governance body beyond the maintainer. Normative **FEL-RECORDS** semantics (quantifiers over array-of-record rows with a stable `$` story) and the **Runtime Companion S15** coprocessor handoff — see `TODO.md` Phase 11.

**Sustainability model:** WOS documents are JSON files under your control, validated against published JSON Schemas. The specification is a public document any team can implement independently. If the project stopped today, your workflow documents remain usable by any JSON-capable system -- the spec is the product, not a service.

**Licensing:** AGPL-3.0 applies to implementation code, not to workflow documents you create. Your kernel documents, governance configurations, and sidecar files are your data. Dual licensing is on the Formspec roadmap for organizations where AGPL is a procurement blocker.
