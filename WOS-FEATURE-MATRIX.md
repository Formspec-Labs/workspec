# WOS Feature & Requirements Matrix

**Last updated:** 2026-04-10
**Spec version:** 1.0.0-draft.1 (all specs)
**Purpose:** Non-technical reference for evaluating WOS capabilities against competing platforms

---

## How to Read This Document

**WOS Implementation Status Icons:**

| Icon | Meaning |
|------|---------|
| ✅ | Implemented and tested (engine + fixtures/lint) |
| 🟦 | Specified and partially implemented (engine code exists, not fully tested) |
| 🟡 | Specified in prose and schema, not yet implemented in engine |
| ⚪ | Referenced/planned but not yet specified |

**Competitor Support Icons:**

| Icon | Meaning |
|------|---------|
| ✔ | Full native support |
| 🟠 | Partial or limited support |
| ✘ | Not supported |
| -- | Not applicable to this platform |

**Competitor Abbreviations:**

| Abbrev | Platform |
|--------|----------|
| BPMN | Business Process Model and Notation 2.0 (OMG/ISO) |
| CMMN | Case Management Model and Notation 1.1 (OMG) |
| Temporal | Temporal.io durable execution platform |
| StepFn | AWS Step Functions / Amazon States Language |
| SWF | CNCF Serverless Workflow 1.0 |
| SCXML | W3C State Chart XML 1.0 |
| WS-HT | WS-HumanTask 1.1 (OASIS) |
| DMN | Decision Model and Notation 1.4 (OMG) |

---

## 1. Workflow Lifecycle & Execution

Foundational capabilities for defining and running workflows.

| # | Requirement | Description | WOS | BPMN | CMMN | Temporal | StepFn | SWF | SCXML |
|---|-----------|-------------|-----|------|------|----------|--------|-----|-------|
| 1.1 | **Workflow State Machine** | Define a workflow as a series of states with rules for moving between them | ✅ | ✔ | 🟠 | 🟠 | ✔ | ✔ | ✔ |
| 1.2 | **Nested/Hierarchical States** | States can contain sub-states (a "Review" phase has its own internal steps) | 🟦 | 🟠 | 🟠 | ✘ | ✘ | ✘ | ✔ |
| 1.3 | **Parallel Execution** | Multiple tracks of work happening simultaneously (e.g., background check AND reference check) | ✅ | ✔ | 🟠 | ✔ | ✔ | ✔ | ✔ |
| 1.4 | **Parallel Completion Policies** | Control what happens when one parallel track finishes before others: wait for all, cancel siblings, or fail-fast on error | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.5 | **History States** | When a case returns to a previous phase, it resumes where it left off instead of starting over | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✔ |
| 1.6 | **Deterministic Execution** | Two different systems running the same workflow with the same inputs always produce the same result | ✅ | ✘ | ✘ | ✔ | ✘ | ✘ | ✔ |
| 1.7 | **Milestone Tracking** | Named checkpoints that fire when data conditions are met, independent of workflow state | 🟦 | ✘ | ✔ | ✘ | ✘ | ✘ | ✘ |
| 1.8 | **Compensation / Undo** | When a step fails, previously completed steps can be rolled back in reverse order | 🟡 | ✔ | ✘ | ✔ | ✘ | 🟠 | ✘ |
| 1.9 | **Forward & Backward Recovery** | Choose between undoing completed work or skipping ahead to an alternate path when things go wrong | 🟡 | 🟠 | ✘ | 🟠 | ✘ | ✘ | ✘ |
| 1.10 | **Semantic Tags on States** | Label states with meanings like "determination," "review," "appeal" so governance rules attach to categories | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.11 | **Related Case Events** | When a linked case changes status, the current case automatically receives a notification event | 🟦 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.12 | **Cascade Depth Limits** | Prevent chain reactions where case A notifies B notifies C -- capped at configurable depth (default 3) | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.13 | **Continuous Evaluation Mode** | Guards re-evaluate automatically whenever data changes, not just when events arrive | 🟡 | ✘ | ✔ | ✘ | ✘ | ✘ | ✘ |
| 1.14 | **Convergence Cap** | Safety limit (100 cycles) preventing infinite loops when continuous evaluation triggers cascading transitions | 🟡 | -- | -- | -- | -- | -- | -- |

---

## 2. Durable Execution & Reliability

Guarantees that workflows survive crashes, restarts, and infrastructure failures.

| # | Requirement | Description | WOS | BPMN | Temporal | StepFn | SWF |
|---|-----------|-------------|-----|------|----------|--------|-----|
| 2.1 | **Crash Recovery** | Running workflows resume from their last saved point after a system crash | 🟡 | ✘ | ✔ | ✔ | ✘ |
| 2.2 | **Persistent State** | Workflow progress, case data, and timer registrations are durably saved | 🟡 | ✘ | ✔ | ✔ | ✘ |
| 2.3 | **Deterministic Replay** | External service results are saved so replaying produces identical outcomes without re-calling services | 🟡 | ✘ | ✔ | ✘ | ✘ |
| 2.4 | **Durable Timers** | Timers survive restarts and fire on schedule even after prolonged outages, with precision tolerances | 🟡 | 🟠 | ✔ | ✔ | ✔ |
| 2.5 | **Signal Queuing** | Messages sent to suspended workflows are saved and delivered when the workflow resumes | 🟡 | ✘ | ✔ | ✘ | ✘ |
| 2.6 | **Idempotent Service Calls** | Deduplication keys prevent the same external service from being called twice during crash recovery | 🟦 | ✘ | ✔ | ✘ | ✘ |
| 2.7 | **Exactly-Once Event Processing** | Events processed once and only once per workflow instance, even with at-least-once delivery | 🟡 | ✘ | ✔ | ✘ | ✘ |
| 2.8 | **Atomic Checkpoints** | Workflow state and its audit trail are saved together -- never one without the other | 🟡 | ✘ | ✔ | ✘ | ✘ |

---

## 3. Case Data & Evidence Management

How the workflow manages the data it operates on.

| # | Requirement | Description | WOS | BPMN | CMMN | Temporal | StepFn |
|---|-----------|-------------|-----|------|------|----------|--------|
| 3.1 | **Typed Case File** | Every case has a structured data container with named, typed fields | 🟦 | ✘ | ✔ | ✘ | 🟠 |
| 3.2 | **Immutable Mutation History** | Every change to case data is permanently recorded: who, what, from, to, when, and in what state | 🟡 | ✘ | ✘ | ✔ | ✘ |
| 3.3 | **Case Relationships** | Formal typed links between cases: parent/child, sibling, related, supersedes -- with provenance | 🟦 | ✘ | ✘ | ✘ | ✘ |
| 3.4 | **Cross-Case Isolation** | Related cases can react to each other's status but cannot read each other's data directly | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 3.5 | **Contract-Validated Data** | Data entering the workflow is validated against a formal contract before being accepted | 🟦 | ✘ | ✘ | ✘ | 🟠 |
| 3.6 | **Two Contract Bindings** | Supports rich form-based validation (Formspec) and simple structural validation (JSON Schema) | 🟡 | ✘ | ✘ | ✘ | 🟠 |

---

## 4. Human Task Management

How the system manages work assigned to people.

| # | Requirement | Description | WOS | BPMN | WS-HT | Temporal | StepFn |
|---|-----------|-------------|-----|------|-------|----------|--------|
| 4.1 | **Task Lifecycle** | 8-state lifecycle: created, assigned, claimed, completed, failed, delegated, escalated, skipped | 🟡 | 🟠 | ✔ | ✘ | 🟠 |
| 4.2 | **Assignment Roles** | 5 roles: owner, nominee, potential owner pool, administrator, and explicitly excluded actors | 🟡 | 🟠 | ✔ | ✘ | ✘ |
| 4.3 | **SLA with Breach Policies** | Configurable deadlines with auto-actions on breach: escalate, reassign, notify, or extend | 🟡 | 🟠 | ✔ | ✘ | ✘ |
| 4.4 | **Business Calendar Support** | SLA deadlines computed in business days, excluding holidays and non-working hours | ⚪ | ✘ | 🟠 | ✘ | ✘ |
| 4.5 | **Separation of Duties** | The person who makes a decision cannot be the same person who reviews it | ✅ | ✘ | ✘ | ✘ | ✘ |
| 4.6 | **Delegation of Authority** | Formal chains defining who is authorized to make decisions, with legal references and expiration | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 4.7 | **Sub-Delegation Controls** | Authority can be further delegated only if permitted, with maximum chain depth | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 4.8 | **Override Authority** | Reviewers can override decisions with mandatory rationale, authority verification, and immutable audit trail | 🟡 | ✘ | ✘ | ✘ | ✘ |

---

## 5. Governance & Due Process

Rules ensuring fairness, accountability, and legal compliance in decision-making.

| # | Requirement | Description | WOS | BPMN | CMMN | Temporal | DMN |
|---|-----------|-------------|-----|------|------|----------|-----|
| 5.1 | **Impact Level Classification** | Every workflow declares its consequence level: rights-impacting, safety-impacting, operational, or informational | ✅ | ✘ | ✘ | ✘ | ✘ |
| 5.2 | **Mandatory Notice** | Affected individuals must receive notice before an adverse decision takes effect | ✅ | ✘ | ✘ | ✘ | ✘ |
| 5.3 | **Individualized Explanation** | Adverse decisions must include reasons specific to the individual's case | 🟡 | ✘ | ✘ | ✘ | 🟠 |
| 5.4 | **Counterfactual Explanation** | Must explain what the person could change and what irrelevant factors did NOT affect the decision | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 5.5 | **Independent Appeal** | Appeals must be reviewed by someone independent of the original decision-maker | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 5.6 | **Continuation of Service** | When configured, adverse impacts are frozen and services maintained during the appeal period | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 5.7 | **Respondent Ledger** | Tracks notice delivery, receipt confirmation, and appeal deadlines per individual | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 5.8 | **Typed Hold Policies** | 6 standard suspension reasons with expected durations and timeout actions | ✅ | ✘ | ✘ | ✘ | ✘ |
| 5.9 | **Review Protocols** | 5 empirically-grounded protocols for ensuring genuine cognitive engagement during review | ✅ | ✘ | ✘ | ✘ | ✘ |
| 5.10 | **Quality Sampling** | Configurable percentage of decisions randomly selected for quality review | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 5.11 | **Tag-Based Governance** | Governance rules attach to semantic categories, not specific workflow states | ✅ | ✘ | ✘ | ✘ | ✘ |
| 5.12 | **Scoped Governance Rules** | Governance rules include conditions so they only apply when relevant | 🟡 | ✘ | ✘ | ✘ | ✘ |

---

## 6. Review Protocols

Specific methods for ensuring human reviewers genuinely evaluate work rather than rubber-stamping it.

| # | Protocol | Description | WOS | Any Competitor? |
|---|---------|-------------|-----|-----------------|
| 6.1 | **Independent-First** | Reviewer forms their own assessment before seeing any recommendation. Interface enforces the ordering. | 🟡 | ✘ |
| 6.2 | **Consider-Opposite** | After seeing a recommendation, reviewer must articulate reasons it might be wrong before confirming | 🟡 | ✘ |
| 6.3 | **Calibrated Confidence** | Confidence scores displayed alongside recommendations; low-confidence items highlighted | 🟡 | ✘ |
| 6.4 | **Dual-Blind** | Two independent reviewers assess without seeing each other's work; results reconciled | 🟡 | ✘ |
| 6.5 | **Unassisted** | No recommendation provided; reviewer uses professional judgment alone | 🟡 | ✘ |

---

## 7. Audit & Provenance

How the system records what happened, why, and by whom.

| # | Requirement | Description | WOS | BPMN | Temporal | W3C PROV | DMN |
|---|-----------|-------------|-----|------|----------|----------|-----|
| 7.1 | **Facts Tier** | Every state transition and action produces an immutable record: who, what, when, inputs, outputs | ✅ | ✘ | ✔ | ✔ | ✘ |
| 7.2 | **Tamper Detection** | Optional cryptographic digests on inputs/outputs for detecting modification | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 7.3 | **Reasoning Tier** | Records which rules were applied, what evidence was consulted, and criteria checked | 🟡 | ✘ | ✘ | ✘ | 🟠 |
| 7.4 | **Authority-Ranked Explanations** | Rules ranked by authority: statute > regulation > policy > guideline | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 7.5 | **Counterfactual Tier** | Records what would have changed the outcome and what did not affect it | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 7.6 | **Narrative Tier** | AI-generated explanations recorded separately, explicitly marked non-authoritative | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 7.7 | **W3C PROV-O Export** | Provenance exports to standard W3C PROV Ontology format | 🟡 | ✘ | ✘ | ✔ | ✘ |
| 7.8 | **Process Mining Export** | Provenance exports to XES (IEEE 1849) and OCEL 2.0 for process mining tools | 🟡 | ✘ | ✘ | ✘ | ✘ |

---

## 8. AI Agent Governance

How the system manages AI/ML agents participating in workflows.

| # | Requirement | Description | WOS | Any Competitor? |
|---|-----------|-------------|-----|-----------------|
| 8.1 | **Agent Registration** | AI agents formally registered with type (deterministic/statistical/generative), model ID, version | ✅ | ✘ |
| 8.2 | **Agent as Untrusted Actor** | All agent outputs treated as untrusted input; the system enforces all constraints | 🟡 | ✘ |
| 8.3 | **Deontic Constraints** | Four behavioral rule types: permissions, prohibitions, obligations, rights | ✅ | ✘ |
| 8.4 | **Constraint Enforcement Ordering** | Fixed evaluation order: permissions > prohibitions > obligations > confidence > volume > sampling | 🟡 | ✘ |
| 8.5 | **Formspec-as-Validator** | Agent output validated against the same form contract as human-submitted data | 🟡 | ✘ |
| 8.6 | **Four Autonomy Levels** | Autonomous (no review), supervisory (review window), assistive (human confirms), manual (advises only) | 🟡 | ✘ |
| 8.7 | **Impact-Level Caps** | Workflow consequence level sets ceiling on agent autonomy | 🟡 | ✘ |
| 8.8 | **Dynamic Autonomy** | Autonomy raised (with approval + expiration) or auto-lowered based on performance | 🟡 | ✘ |
| 8.9 | **Confidence Framework** | Every agent output must include confidence report with per-field scores and calibration | 🟡 | ✘ |
| 8.10 | **Confidence Decay** | Agent output confidence degrades over time as data changes | 🟡 | ✘ |
| 8.11 | **Cumulative Confidence** | Multi-step interactions track compounding error; human review required when floor breached | 🟡 | ✘ |
| 8.12 | **Mandatory Fallback Chains** | Every workflow using agents must work when agents are unavailable | ✅ | ✘ |
| 8.13 | **Volume Rate Limits** | Caps on autonomous agent actions per hour/day to prevent runaway automation | 🟡 | ✘ |
| 8.14 | **Rubber-Stamp Detection** | Monitoring for signs reviewers are blindly accepting agent recommendations | 🟡 | ✘ |
| 8.15 | **Drift Detection** | Statistical monitoring for when agent behavior silently shifts (PSI, KS, chi-squared, accuracy) | 🟡 | ✘ |
| 8.16 | **Shadow / Canary Deployment** | New model versions run in shadow then canary before full production | 🟡 | ✘ |
| 8.17 | **Agent Disclosure** | Affected individuals told when AI participated. Mandatory for rights-impacting workflows. | 🟡 | ✘ |
| 8.18 | **Model Version Management** | Three policies: pinned (exact version), approved list, or latest-with-tracking | 🟡 | ✘ |
| 8.19 | **Agent Lifecycle State Machine** | Formal states: active, degraded, suspended, retired -- with auto-transitions on performance | 🟡 | ✘ |
| 8.20 | **Tool Use Governance** | Controls which tools agents can invoke, rate limits, side-effect policies | 🟡 | ✘ |
| 8.21 | **Assist Governance Proxy** | Wraps AI assistant tool invocations with deontic constraints and provenance | 🟡 | ✘ |

---

## 9. Data Validation Pipelines

Staged processing chains for validating untrusted data before it influences decisions.

| # | Requirement | Description | WOS | Any Competitor? |
|---|-----------|-------------|-----|-----------------|
| 9.1 | **Pipeline Stages** | Four stage types: contract validation, assertion gates, data transformation, human review | 🟡 | ✘ |
| 9.2 | **Seven Assertion Gate Types** | Source-grounded, arithmetic, range, consistency, format, cross-document, temporal | ✅ | ✘ |
| 9.3 | **Reusable Assertion Libraries** | Named assertion definitions shared across multiple pipelines and governance documents | 🟡 | ✘ |
| 9.4 | **Pipeline Risk Profile** | Reliability determined by weakest validation gate, not strongest processing stage | 🟡 | ✘ |
| 9.5 | **Four Rejection Policies** | Retry with corrections, escalate to supervisor, hold pending data, or fail with explanation | 🟡 | ✘ |
| 9.6 | **Rejection Provenance** | Every rejection records what failed, the input, the threshold, and what would pass | 🟡 | ✘ |

---

## 10. Temporal & Policy Management

How the system handles rules and parameters that change over time.

| # | Requirement | Description | WOS | OpenFisca | DMN | Any Workflow Std? |
|---|-----------|-------------|-----|-----------|-----|-------------------|
| 10.1 | **Date-Indexed Parameters** | Policy values indexed by effective date; correct value auto-applied based on case filing date | 🟡 | ✔ | ✘ | ✘ |
| 10.2 | **Resolution Date References** | Different parameters can resolve against different dates (application, determination, effective) | 🟡 | ✔ | ✘ | ✘ |
| 10.3 | **Regulatory Version Bindings** | External documents bound to specific versions by date; old cases keep original versions | 🟡 | ✘ | ✘ | ✘ |
| 10.4 | **Instance Version Pinning** | Running instances bound to creation-time definition; updates never change in-flight cases | 🟡 | -- | -- | 🟠 |
| 10.5 | **Explicit Migration** | Moving a case to a new workflow version is deliberate, audited, with field mapping and validation | 🟡 | -- | -- | ✘ |

---

## 11. Equity & Fairness

Statistical monitoring and enforcement for equitable outcomes.

| # | Requirement | Description | WOS | Any Competitor? |
|---|-----------|-------------|-----|-----------------|
| 11.1 | **Equity Guardrails** | Statistical monitoring for demographic disparities -- applies to human AND AI decisions | 🟡 | ✘ |
| 11.2 | **Disparity Metrics** | Configurable metrics tracked by demographic group with maximum disparity thresholds | 🟡 | ✘ |
| 11.3 | **Asynchronous Enforcement** | Produces alerts and reports, not per-decision blocks -- avoids masking systemic issues | 🟡 | ✘ |
| 11.4 | **Remediation Triggers** | Auto-triggers structured remediation review when disparity thresholds are breached | 🟡 | ✘ |
| 11.5 | **Protected Category Configuration** | Separate sidecar for demographic groupings and monitoring parameters, independently updatable | 🟡 | ✘ |

---

## 12. Formal Verification

Mathematical proofs about workflow correctness.

| # | Requirement | Description | WOS | SCXML | BPMN | Other |
|---|-----------|-------------|-----|-------|------|-------|
| 12.1 | **SMT Verification of Constraints** | Constraints mathematically proven to hold for all inputs before the workflow goes live | 🟡 | 🟠 | 🟠 | 🟠 |
| 12.2 | **Verifiable Constraint Subset** | Clear rules for provability: linear arithmetic, finite domains, no recursion, no external calls | 🟡 | -- | -- | ✘ |
| 12.3 | **Verification Reports** | Immutable records of results: proven safe / proven unsafe with counterexample / inconclusive | 🟡 | ✘ | ✘ | ✘ |
| 12.4 | **DCR Constraint Zones** | Declarative adaptive case management using include/exclude/condition/response/milestone relations | 🟡 | ✘ | ✘ | 🟠 |

---

## 13. Integration & Interoperability

How WOS connects to external systems and standards.

| # | Requirement | Description | WOS | BPMN | Temporal | SWF |
|---|-----------|-------------|-----|------|----------|-----|
| 13.1 | **Seven Integration Binding Types** | Request-response, Arazzo, tool, event-emit, event-consume, callback, policy engine | 🟡 | 🟠 | 🟠 | 🟠 |
| 13.2 | **CloudEvents 1.0 Native** | Standard CloudEvents envelopes with WOS extension attributes for routing and causal chains | 🟡 | ✘ | ✘ | ✔ |
| 13.3 | **Policy Engine Bridge** | Direct integration with XACML, OPA, and Cedar authorization engines | 🟡 | ✘ | ✘ | ✘ |
| 13.4 | **Formspec Contract Validation** | Integration data validated against Formspec Definitions before sending or accepting | 🟡 | ✘ | ✘ | ✘ |
| 13.5 | **SCXML Interoperability** | Bidirectional mapping between WOS documents and W3C SCXML | 🟡 | -- | ✘ | ✘ |
| 13.6 | **JSON-LD / Linked Data** | WOS documents interpretable as RDF/linked data via JSON-LD context, enabling SPARQL queries | 🟡 | ✘ | ✘ | ✘ |
| 13.7 | **SHACL Shape Validation** | Semantic policy-level validation using W3C SHACL for constraints JSON Schema cannot express | 🟡 | ✘ | ✘ | ✘ |
| 13.8 | **External Vocabulary Support** | Incorporate NIEM, FHIR, or other domain vocabularies without translation middleware | 🟡 | ✘ | ✘ | ✘ |
| 13.9 | **SCXML History State Mapping** | Direct mapping to SCXML shallow/deep history -- only WOS and SCXML support this | 🟡 | ✘ | ✘ | ✘ |

---

## 14. Architecture & Extensibility

Structural properties of the specification itself.

| # | Requirement | Description | WOS | BPMN | CMMN | Temporal |
|---|-----------|-------------|-----|------|------|----------|
| 14.1 | **Layered Opt-In** | Four layers each independently adoptable; Kernel-only is a valid deployment | ✅ | ✘ | ✘ | ✘ |
| 14.2 | **Sidecar Document Pattern** | Configuration lives in separate documents, independently updatable | ✅ | ✘ | ✘ | ✘ |
| 14.3 | **Five Extension Seams** | Named attachment points through which layers add behavior without modifying the kernel | ✅ | 🟠 | ✘ | ✘ |
| 14.4 | **Four Separation Principles** | Lifecycle / case state / decision logic / audit / governance all cleanly separated | ✅ | 🟠 | 🟠 | ✘ |
| 14.5 | **JSON-Native Format** | All documents are JSON -- human-readable, machine-parseable, AI-friendly | ✅ | ✘ | ✘ | ✘ |
| 14.6 | **Conformance Profiles** | Multiple tiers per layer (Structural, Complete, Governed) for incremental adoption | ✅ | ✘ | ✘ | -- |
| 14.7 | **181 Lint Rules** | Static analysis: 30 T1 single-doc, 50 T2 cross-doc, 101 T3 dynamic | 🟦 | 🟠 | ✘ | ✘ |

---

## 15. Unique Capabilities (No Competitor Coverage)

These capabilities exist in no competing standard, framework, or platform surveyed (50+ reviewed):

| # | Capability | Why It Matters |
|---|-----------|----------------|
| 15.1 | **Decision provenance with authority ranking** | Know exactly which rule (statute > regulation > policy > guideline) drove every decision, with full evidence chain |
| 15.2 | **Temporal parameter versioning in workflow context** | Apply the correct eligibility rules for the date the case was filed, not today's rules |
| 15.3 | **Structured human override with accountability** | Allow expert overrides without losing accountability -- rationale, authority, and evidence are permanent record |
| 15.4 | **AI confidence decay and cumulative tracking** | Agent reliability degrades over time; multi-step interactions track compounding error |
| 15.5 | **Empirically-grounded review protocols** | Five methods based on published research for ensuring genuine cognitive engagement |
| 15.6 | **Rubber-stamp detection** | Flags when reviewers blindly accept recommendations -- a known failure mode in human-AI teaming |
| 15.7 | **Equity guardrails for human AND AI decisions** | Fairness monitoring for all decision-makers, not just AI -- bias is a human problem too |
| 15.8 | **Deontic constraints on AI agents** | Permission/prohibition/obligation/right framework from legal reasoning, applied to agent behavior |
| 15.9 | **Mandatory graceful degradation** | Every workflow must function when AI is unavailable -- agent failure is normal, not exceptional |
| 15.10 | **Parallel completion policies** | Wait-all, cancel-siblings, fail-fast control concurrent track behavior |

---

## 16. Implementation Status Summary

| Component | Status | Detail |
|-----------|--------|--------|
| **16 Specifications** | ✅ | All normative prose written, all at v1.0.0-draft.1 |
| **16 JSON Schemas** | ✅ | All document types have JSON Schema 2020-12 validation |
| **22 Fixtures** | 🟦 | Example documents for all layers; conformance fixtures for core scenarios |
| **wos-core crate** | 🟦 | Typed models for kernel/governance/AI; evaluation algorithm; 9 traits; default runtime |
| **wos-lint crate** | 🟦 | 181 rules defined; 76 tested; T1 & T2 mostly covered; T3 needs fixtures |
| **wos-conformance crate** | 🟦 | Dynamic test runner; 185 tests passing; migrating to wos-core typed models |
| **Engine capabilities** | 🟡 | Deontic, compensation, confidence, DCR, delegation, autonomy, due process, pipelines -- all Phase 5 |
| **Documentation** | ⚪ | README updates, architecture docs, API docs -- Phase 7 |

### What's Next (by phase)

1. **Phase 3** (in progress): Finish extracting typed models into wos-core; consolidate timers; migrate wos-conformance
2. **Phase 4**: Write ~94 failing conformance test fixtures for untested Tier 3 rules
3. **Phase 5**: Implement engine capabilities (10 modules)
4. **Phase 6**: Migrate wos-lint from raw JSON walking to typed model field access
5. **Phase 7**: Documentation

---

## Appendix A: Standards Surveyed

The following 50+ standards were analyzed. WOS adopted concepts from many, adapted others, and identified gaps no existing system addresses.

**Adopted substantially intact:** WS-HumanTask lifecycle, CMMN case file model, DMN decision table concepts, CloudEvents envelope, W3C PROV Entity-Activity-Agent triad, JSON Schema, Harel statechart semantics, Saga compensation pattern, NIST AU control requirements.

**Adapted (concept redesigned for WOS):** BPMN event taxonomy, SCXML statechart semantics (JSON not XML), XACML PEP/PDP architecture, Catala default logic, OpenFisca temporal parameters, GSM guard-stage-milestone, DCR include/exclude, Temporal durable execution guarantees (declarative not code-first), Certificate Transparency Merkle proofs.

**Evaluated but not adopted:** WS-BPEL (obsolete SOAP coupling), XPDL (interchange format only), YAWL (academic), S-BPM (limited adoption), Azure Durable Functions / Restate / Inngest / Cadence / Netflix Conductor (vendor-specific or code-first), Drools/Rete (rule engine only), Google Zanzibar/Cedar (authorization only), NIEM (data vocabulary only), HL7 FHIR (healthcare-specific).
