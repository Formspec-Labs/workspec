# WOS Feature & Requirements Matrix

**Last updated:** 2026-04-10
**Spec version:** 1.0.0-draft.1 (all specs)
**Purpose:** Non-technical reference for evaluating WOS capabilities against competing platforms

---

## How to Read This Document

**WOS Implementation Status:**

| Icon | Meaning |
|------|---------|
| ✅ | Implemented and tested (engine + fixtures/lint) |
| 🟦 | Specified and partially implemented (engine code exists, not fully tested) |
| 🟡 | Specified in prose and schema, not yet implemented in engine |
| ⚪ | Referenced/planned, not yet fully specified |

**Competitor Support:**

| Icon | Meaning |
|------|---------|
| ✔ | Full native support |
| ~ | Partial or limited support |
| ✘ | Not supported |
| -- | Not applicable |

**Competitors Evaluated (16):**

| Abbrev | Platform | Type |
|--------|----------|------|
| SNow | ServiceNow | Commercial. 100+ US federal agencies. |
| Pega | Pegasystems | Commercial. CMS, SSA, IRS, USCIS. |
| Appn | Appian | Commercial. FedRAMP. DoD + civilian. |
| SFGov | Salesforce Government Cloud | Commercial. Agentforce AI. FedRAMP. |
| Cam | Camunda 8 | Open source (Apache 2.0 / SSPL). Zeebe. |
| KIE | Apache KIE | Open source (Apache 2.0). BPMN+DMN+Drools+OptaPlanner. |
| Flow | Flowable | Open source (Apache 2.0). BPMN+CMMN+DMN. |
| Temp | Temporal | Open source (MIT). Deterministic replay. |
| Palnt | Palantir AIP | Commercial. DoD, IC, HHS, VA. |
| MSPow | Microsoft Power Platform | Commercial. Power Automate + Copilot Studio. FedRAMP. |
| LGrph | LangGraph / LangChain | Open source. AI agent orchestration. |
| StepFn | AWS Step Functions | Commercial. Amazon States Language. GovCloud. |
| Tyler | Tyler Technologies | Commercial. Courts, finance, permitting, public safety. |
| Bonit | Bonita | Open source (GPL v2). European government. |
| PMkr | ProcessMaker | Open source (AGPL v3). LATAM government. |

Tables below show the 8 most differentiated competitors per category. Full 16-competitor ratings are in the companion spreadsheet (`wos-formspec-competitive-feature-matrix.xlsx`).

---

## 1. Process Orchestration

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Flow | Temp | Palnt | LGrph |
|---|------------|-------------|-----|------|------|-----|-----|------|------|-------|-------|
| 1.1 | **Sequential/parallel/choice composition** | Define workflows as sequences, branches, and parallel tracks | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ~ |
| 1.2 | **Hierarchical states (nested, compound)** | States contain sub-states; a "Review" phase has its own internal steps | 🟦 | ~ | ~ | ~ | ✔ | ~ | ~ | ✘ | ✘ |
| 1.3 | **History states** | When a case returns to a previous phase, it resumes where it left off | 🟡 | ~ | ~ | ✘ | ~ | ~ | ✔ | ✘ | ✘ |
| 1.4 | **Parallel regions (orthogonal)** | Multiple concurrent aspects of a single state, each progressing independently | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 1.5 | **Parallel completion policies** | Wait-all, cancel-siblings, or fail-fast when one parallel track finishes first | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.6 | **Declarative constraint zones (DCR-style)** | Adaptive case management using condition/response/include/exclude relations instead of explicit transitions | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.7 | **CMMN case management** | Discretionary items, sentries, ad-hoc work within structured stages | 🟡 | ~ | ✔ | ✘ | ✘ | ✔ | ✘ | ~ | ✘ |
| 1.8 | **Milestones (data-driven checkpoints)** | Named conditions that fire when data reaches a threshold, independent of workflow state | 🟦 | ~ | ✔ | ✘ | ~ | ~ | ✘ | ~ | ✘ |
| 1.9 | **Cancellation regions** | Cancel a set of activities when one completes or fails | 🟡 | ~ | ~ | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 1.10 | **Process definition as declarative data** | Workflow defined as a JSON document, not imperative code | ✅ | ✘ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ | ✘ |

---

## 2. Lifecycle & Durable Execution

| # | Requirement | Description | WOS | SNow | Pega | Cam | Temp | StepFn | KIE | Flow |
|---|------------|-------------|-----|------|------|-----|------|--------|-----|------|
| 2.1 | **Crash recovery** | Running workflows resume from last saved point after system crash | 🟡 | ✔ | ✔ | ✔ | ✔ | ✔ | ~ | ~ |
| 2.2 | **Deterministic replay** | External service results saved; replay produces identical outcomes without re-calling services | 🟡 | ✘ | ✘ | ✘ | ✔ | ✘ | ✘ | ✘ |
| 2.3 | **Saga/compensation transactions** | Failed steps trigger reverse execution of previously completed steps | 🟡 | ~ | ~ | ✔ | ✔ | ✔ | ~ | ~ |
| 2.4 | **Timer management** | Absolute, relative, and recurring timers with durability guarantees and precision tolerances | 🟡 | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 2.5 | **Business-calendar-aware deadlines** | SLA deadlines computed in business days, excluding holidays and non-working hours | 🟦 | ✔ | ✔ | ~ | ✘ | ✘ | ~ | ~ |
| 2.6 | **Statutory deadline chains** | Interdependent government deadlines with automatic legal consequences when missed | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 2.7 | **Instance migration across versions** | Move running cases to a new workflow version with field mapping and state validation | 🟡 | ~ | ~ | ✔ | ✔ | ✘ | ~ | ~ |
| 2.8 | **Idempotent execution** | Deduplication keys prevent duplicate service calls during crash recovery | 🟦 | ~ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ |

> **Note on 2.6:** WOS has timeout categories and temporal parameters that can compose to model statutory deadline chains. This capability is achievable but not a single named feature in the spec. Rated 🟡 rather than ★ pending explicit specification.

---

## 3. Decision & Rules

| # | Requirement | Description | WOS | Pega | Cam | KIE | Flow | Temp | Palnt |
|---|------------|-------------|-----|------|-----|-----|------|------|-------|
| 3.1 | **Decision tables (DMN)** | Tabular decision logic with hit policies | 🟡 ^1 | ✔ | ✔ | ✔ | ✔ | ✘ | ~ |
| 3.2 | **Decision requirement graphs** | Visual composition of dependent decisions | 🟡 ^1 | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 3.3 | **FEEL expression language** | OMG standard expression language for decisions | ✘ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ |
| 3.4 | **FEL expression language** | PEG-parseable, deterministic, side-effect-free expressions with DAG dependency tracking | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.5 | **Business rules engine (DRL/Rete)** | Forward-chaining production rules | ✘ | ✔ | ✘ | ✔ | ✘ | ✘ | ✘ |
| 3.6 | **Defeasible rules** | General rules with exceptions that override them by priority | 🟡 ^2 | ~ | ✘ | ✘ | ✘ | ✘ | ~ |
| 3.7 | **Temporal parameters (date-effective values)** | Policy values indexed by effective date; correct value auto-applied based on case filing date | 🟡 | ✔ | ✘ | ~ | ✘ | ✘ | ✘ |
| 3.8 | **Regulatory version bindings** | External documents bound to specific versions by date; old cases keep original versions | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.9 | **Policy change propagation** | Grandfather, migrate, or review in-flight cases when regulations change | 🟡 | ~ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.10 | **Constraint solver / optimization** | OptaPlanner-style constraint solving for resource allocation | ✘ | ✘ | ✘ | ✔ | ✘ | ✘ | ✘ |

> ^1 **Decision tables / DRGs:** WOS delegates to external decision services via `invokeService` and the policy engine bridge (Integration Profile S8). The kernel spec explicitly states "WOS does not embed a decision table engine." DMN/DRG capabilities are available through integration, not natively. Rated 🟡 (specified integration path) rather than ■.
>
> ^2 **Defeasible rules:** WOS has authority ranking (statute > regulation > policy > guideline) and constraint composition with override mechanics, but not formal Catala-style defeasibility. Rated 🟡 (partial, via composition) rather than ■.

---

## 4. Human Task Management

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Flow | Temp | MSPow |
|---|------------|-------------|-----|------|------|-----|-----|------|------|-------|
| 4.1 | **WS-HumanTask lifecycle** | 8+ state lifecycle: created, assigned, claimed, completed, failed, delegated, escalated, skipped | 🟡 | ✔ | ✔ | ✔ | ✔ | ✔ | ✘ | ✔ |
| 4.2 | **Role-based assignment** | Owner, nominee, potential owner pool, administrator, excluded actors | 🟡 | ✔ | ✔ | ✔ | ✔ | ✔ | ✘ | ✔ |
| 4.3 | **Capability-based routing** | Route tasks based on actor skills/capabilities, not just roles | ⚪ ^3 | ~ | ✔ | ✘ | ~ | ~ | ✘ | ~ |
| 4.4 | **Separation of duties (four-eyes)** | Decision-maker and reviewer must be different people | ✅ | ~ | ✔ | ✘ | ~ | ~ | ✘ | ✘ |
| 4.5 | **Delegation with accountability chain** | Formal authority chains with legal instrument references and expiration dates | 🟡 | ✔ | ✔ | ✘ | ~ | ~ | ✘ | ~ |
| 4.6 | **Escalation (time + condition based)** | Automatic escalation when deadlines are missed or conditions met | 🟡 | ✔ | ✔ | ~ | ✔ | ✔ | ✘ | ✔ |
| 4.7 | **SLA tracking with deadline actions** | Configurable deadlines with auto-actions on breach: escalate, reassign, notify, extend | 🟡 | ✔ | ✔ | ~ | ~ | ~ | ~ | ✔ |
| 4.8 | **Override with structured rationale** | Reviewers override decisions with mandatory rationale, authority verification, immutable audit | 🟡 | ~ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ |

> ^3 **Capability-based routing:** WOS defines assignment roles (including potentialOwner pools) but task routing by actor capability/skill is implementation-defined. The kernel spec says actor-to-action routing is "implementation-defined in kernel-only deployments." Rated ⚪ (referenced concept, not normatively specified).

---

## 5. AI Agent Governance

No competing standard or platform offers comprehensive AI agent governance. Palantir AIP and LangGraph have fragments.

| # | Requirement | Description | WOS | Palnt | LGrph | SNow | Pega | Cam |
|---|------------|-------------|-----|-------|-------|------|------|-----|
| 5.1 | **Agent registration with type taxonomy** | Agents formally registered as deterministic, statistical, or generative with model ID and version | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.2 | **Agent as untrusted actor** | All agent outputs treated as untrusted input; system enforces all constraints | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.3 | **Deontic constraints (POPR)** | Permission/Prohibition/Obligation/Right framework governing agent behavior | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.4 | **SMT-verifiable governance constraints** | Mathematically prove constraints hold for all possible inputs before deployment | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.5 | **Four autonomy levels** | Autonomous, supervisory, assistive, manual -- per action, not per agent | 🟡 | ~ | ✘ | ✘ | ~ | ✘ |
| 5.6 | **Impact-level caps on autonomy** | Workflow consequence level (rights-impacting, safety, operational, informational) sets ceiling on agent autonomy | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.7 | **Dynamic autonomy (escalation/demotion)** | Autonomy raised with human approval + expiration, or auto-lowered on performance triggers | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.8 | **Confidence framework with decay** | Every output requires confidence report; confidence degrades over time as data changes | 🟡 | ~ | ~ | ✘ | ~ | ✘ |
| 5.9 | **Cumulative confidence tracking** | Multi-step agent interactions track compounding error; human review when floor breached | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.10 | **Mandatory fallback chains** | Every workflow using agents must work when agents are unavailable; chains validated at load time | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.11 | **Kill-switch / circuit-breaker** | Auto-fallback when error rates exceed thresholds; three-state breaker (closed/open/half-open) | 🟡 | ~ | ✘ | ~ | ~ | ✘ |
| 5.12 | **Drift monitoring with auto demotion** | Statistical drift detection (PSI, KS, chi-squared, accuracy) with automatic autonomy reduction | 🟡 | ~ | ✘ | ✘ | ~ | ✘ |
| 5.13 | **Volume rate limits** | Caps on autonomous agent actions per hour/day to prevent runaway automation | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.14 | **Agent lifecycle state machine** | Formal states: active, degraded, suspended, retired -- with auto-transitions on performance | 🟡 | ~ | ✘ | ✘ | ✘ | ✘ |
| 5.15 | **Model version pinning and policy** | Pinned (exact), approved list, or latest-with-tracking; version changes emit provenance | 🟡 | ~ | ✘ | ✘ | ~ | ✘ |
| 5.16 | **Tool use governance** | Controls which tools agents invoke, rate limits, side-effect policies, data mutation restrictions | 🟡 | ~ | ~ | ✘ | ✘ | ✘ |
| 5.17 | **Formspec-as-Validator** | Agent output validated against the same form contract as human-submitted data | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.18 | **Agent disclosure** | Affected individuals told when AI participated in decisions. Mandatory for rights-impacting. | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.19 | **Assist Governance Proxy** | Wraps AI assistant tool invocations with deontic constraints and provenance | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.20 | **CaMeL-compatible trust boundary** | Trust boundary model compatible with dual-LLM security architecture | 🟡 ^4 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.21 | **Equity guardrails (bias monitoring)** | Statistical fairness monitoring for human AND AI decisions by demographic group | 🟡 | ~ | ✘ | ✘ | ~ | ✘ |
| 5.22 | **Shadow / canary deployment** | New model versions run in shadow then canary before production, especially for high-stakes | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |

> ^4 **CaMeL dual-LLM:** AI Integration S3.6 describes compatibility with the CaMeL pattern as informative guidance ("Implementations MAY adopt"). Not a normative requirement. The trust boundary model (S3.5) is normative; the CaMeL realization is optional.

---

## 6. Structured Human Oversight

Protocols ensuring genuine cognitive engagement during review. No competing platform offers any of these.

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 6.1 | **Independent-first protocol** | Reviewer forms their own assessment before seeing any recommendation; interface enforces ordering | 🟡 | ✘ |
| 6.2 | **Consider-opposite protocol** | Reviewer must articulate reasons the recommendation might be wrong before confirming | 🟡 | ✘ |
| 6.3 | **Calibrated confidence display** | Per-field confidence scores displayed alongside recommendations; low-confidence items highlighted | 🟡 | Palnt ~ |
| 6.4 | **Dual-blind review** | Two independent reviewers assess without seeing each other's work; results reconciled | 🟡 | Pega ~ |
| 6.5 | **Rejection-rate monitoring** | Track reviewer agreement/modification/disagreement rates for quality signals | 🟡 | Pega ~ |
| 6.6 | **Dynamic sampling (risk-based review allocation)** | Configurable percentage of decisions randomly selected for quality review; random or stratified | 🟡 | SNow ~ |
| 6.7 | **Rubber-stamp detection** | Monitoring for signs reviewers are blindly accepting recommendations (short review times, high agreement) | 🟡 | ✘ |

---

## 7. Due Process & Legal Compliance

Structural requirements for fairness in rights-impacting and safety-impacting workflows. No competing platform addresses these as workflow-level requirements.

| # | Requirement | Description | WOS | SNow | Pega | Palnt |
|---|------------|-------------|-----|------|------|-------|
| 7.1 | **Impact level classification** | Every workflow declares consequence level: rights-impacting, safety, operational, informational | 🟡 | ✘ | ✘ | ✘ |
| 7.2 | **Mandatory notice before adverse decisions** | Affected individuals receive notice with specific reasons, appeal rights, and deadlines | ✅ | ✘ | ✘ | ✘ |
| 7.3 | **Individualized explanation** | Adverse decisions include reasons specific to the individual's case | 🟡 | ✘ | ~ | ✘ |
| 7.4 | **Counterfactual explanation** | What the person could change (positive) and what irrelevant factors did NOT affect the decision (negative) | 🟡 | ✘ | ✘ | ✘ |
| 7.5 | **Dual-readability narrative** | Structured (machine) + prose (human) from same provenance; explanation assembly algorithm | 🟡 | ✘ | ✘ | ✘ |
| 7.6 | **Appeal with independent adjudicator** | Appeals reviewed by someone independent of the original decision-maker | 🟡 | ✘ | ~ | ✘ |
| 7.7 | **Continuation of service during appeal** | Adverse impacts frozen and current services maintained during appeal period | 🟡 | ✘ | ✘ | ✘ |
| 7.8 | **Agent disclosure requirement** | Affected individuals told when AI participated. Mandatory for rights-impacting workflows. | 🟡 | ✘ | ✘ | ✘ |
| 7.9 | **Respondent ledger** | Tracks notice delivery, receipt confirmation, and appeal deadlines per individual | 🟡 | ✘ | ✘ | ✘ |
| 7.10 | **Typed hold policies** | 6 standard suspension reasons with expected durations, resume triggers, and timeout actions | ✅ | ✘ | ✘ | ✘ |
| 7.11 | **Tag-based governance attachment** | Governance rules attach to semantic categories ("all determination steps"), not specific states | ✅ | ✘ | ✘ | ✘ |
| 7.12 | **Scoped governance rules** | Rules include FEL conditions so they only apply when relevant (e.g., "claims over $10,000") | 🟡 | ✘ | ✘ | ✘ |
| 7.13 | **EU AI Act high-risk alignment** | Spec designed consistent with EU AI Act Art. 13-14 requirements | 🟡 | ~ | ~ | ✘ |
| 7.14 | **OMB M-24-10 compliance support** | Agent disclosure and governance structures consistent with federal AI guidance | 🟡 | ✘ | ✘ | ~ |

---

## 8. Provenance & Audit

| # | Requirement | Description | WOS | SNow | Pega | Cam | Temp | Palnt |
|---|------------|-------------|-----|------|------|-----|------|-------|
| 8.1 | **Facts tier (immutable action records)** | Every state transition and action produces an immutable record: who, what, when, inputs, outputs | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 8.2 | **Reasoning tier** | Records which rules were applied, evidence consulted, criteria checked -- for every determination | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.3 | **Counterfactual tier** | Records what would have changed the outcome and what did not affect it | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.4 | **Narrative tier (non-authoritative)** | AI-generated explanations recorded separately and explicitly marked non-authoritative | 🟡 | ✘ | ✘ | ✘ | ✘ | ~ |
| 8.5 | **Epistemic status tagging** | Every assertion tagged: verified fact, system record, agent-generated, or human judgment | 🟡 | ✘ | ✘ | ✘ | ✘ | ~ |
| 8.6 | **Authority-ranked explanations** | Rules ranked by authority: statute > regulation > policy > guideline | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.7 | **Tamper detection (per-record digests)** | Optional cryptographic digests on inputs/outputs for detecting modification | 🟡 ^5 | ✘ | ✘ | ✘ | ✔ | ~ |
| 8.8 | **W3C PROV-O export** | Provenance records export to standard W3C PROV Ontology format | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.9 | **OCEL 2.0 event logging** | Object-centric event log format for process mining | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.10 | **XES process mining export** | Provenance exports to IEEE 1849 XES format for process mining tools | 🟡 | ~ | ~ | ✔ | ✘ | ~ |

> ^5 **Tamper detection:** The published spec defines per-record `inputDigest`/`outputDigest` for tamper detection. Merkle tree hash-chaining was in DRAFTS v2-v7 but is listed as "Deferred / Not started" in TODO.md. The xlsx's "Merkle tree" claim refers to the DRAFT-era design; the published spec has per-record digests only.

---

## 9. Data Collection & Interface Contracts

These are primarily Formspec capabilities that WOS leverages through the contract validation interface.

| # | Requirement | Description | WOS | SNow | Pega | Appn | MSPow |
|---|------------|-------------|-----|------|------|------|-------|
| 9.1 | **Universal interface contract model** | One contract spec for human forms, agent I/O, decision services, and integrations | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 9.2 | **Headless contract pattern** | Formspec Definition with no presentation layer used as pure typed data contract | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 9.3 | **Reactive form behavior** | Computed fields, conditional visibility, cross-field dependencies | ✅ | ~ | ✔ | ✔ | ✔ |
| 9.4 | **Cross-field validation (Shapes)** | Validation rules spanning multiple fields | ✅ | ~ | ✔ | ~ | ~ |
| 9.5 | **Structured validation results** | Severity, field path, message, constraint kind | ✅ | ~ | ~ | ~ | ✘ |
| 9.6 | **Mapping DSL for bidirectional data flow** | Versioned, auditable transforms between case file and external formats | 🟦 | ✘ | ✘ | ✘ | ✘ |
| 9.7 | **Ontology-driven semantic field identity** | Fields identified by semantic meaning, not just name | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 9.8 | **Version-pinned responses** | Immutable contract binding at submission time | 🟡 | ✘ | ~ | ✘ | ✘ |
| 9.9 | **Changelog with impact classification** | Structured change records with severity and migration guidance | 🟦 | ✘ | ✘ | ✘ | ✘ |

---

## 10. Data Validation Pipelines

Staged processing chains for validating untrusted data before it influences decisions.

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 10.1 | **Four pipeline stage types** | Contract validation, assertion gates, data transformation, human review | 🟡 | ✘ |
| 10.2 | **Seven assertion gate types** | Source-grounded, arithmetic, range, consistency, format, cross-document, temporal | ✅ | ✘ |
| 10.3 | **Reusable assertion libraries** | Named assertions shared across pipelines and governance documents | 🟡 | ✘ |
| 10.4 | **Pipeline risk profile** | Reliability determined by weakest gate, not strongest stage | 🟡 | ✘ |
| 10.5 | **Four rejection policies** | Retry with corrections, escalate to supervisor, hold pending data, fail with explanation | 🟡 | ✘ |
| 10.6 | **Rejection provenance** | Every rejection records what failed, the input, the threshold, and what would pass | 🟡 | ✘ |

---

## 11. Case State & Evidence

| # | Requirement | Description | WOS | SNow | Pega | Cam | Palnt |
|---|------------|-------------|-----|------|------|-----|-------|
| 11.1 | **Typed case file** | Structured data container with named, typed fields | 🟦 | ✔ | ✔ | ✘ | ✔ |
| 11.2 | **Immutable mutation history** | Every change permanently recorded: who, what, from, to, when, in what state | 🟡 | ~ | ~ | ✘ | ~ |
| 11.3 | **Case relationships** | Typed links: parent/child, sibling, related, supersedes -- with provenance | 🟦 | ✘ | ✘ | ✘ | ✘ |
| 11.4 | **Cross-case isolation** | Related cases react to status changes but cannot read each other's data | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 11.5 | **Claim check pattern** | Content hash + URI reference for evidence documents (not stored in case state) | 🟡 | ✘ | ✘ | ✘ | ~ |
| 11.6 | **Role-based field-level visibility** | Access control per field per actor role | 🟡 | ✔ | ✔ | ✘ | ✔ |

---

## 12. Integration & Eventing

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Temp | StepFn | LGrph |
|---|------------|-------------|-----|------|------|-----|-----|------|--------|-------|
| 12.1 | **HTTP/REST (OpenAPI)** | Synchronous service invocation via OpenAPI binding | 🟡 | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 12.2 | **CloudEvents 1.0 native** | Standard event envelopes with WOS extension attributes for routing and causal chains | 🟡 | ~ | ~ | ✔ | ✔ | ✔ | ✔ | ✘ |
| 12.3 | **Multi-step API orchestration (Arazzo)** | Reference Arazzo documents for multi-step API sequences | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.4 | **Non-HTTP tool invocation** | CLI, batch, database procedure, graph query bindings | 🟡 | ✘ | ✘ | ✘ | ✘ | ✔ | ✔ | ✔ |
| 12.5 | **Policy engine bridge** | Direct integration with XACML, OPA, and Cedar authorization engines | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.6 | **Form-level agent interaction (Assist)** | Governance proxy for AI assistant tool invocations at form interaction level | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.7 | **SCXML interoperability mapping** | Bidirectional translation between WOS and W3C SCXML documents | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |

> **Note on MCP:** The xlsx lists MCP agent-tool protocol as ■. MCP is a Formspec capability (`formspec-mcp` package), not specified in WOS. WOS integrations use `invokeService` bindings and the Assist Governance Proxy. MCP is available to WOS deployments via Formspec but is not a WOS spec feature.

---

## 13. Semantic Interoperability

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 13.1 | **JSON-LD native serialization** | WOS documents are valid JSON-LD and RDF graphs without transformation | 🟡 | ✘ |
| 13.2 | **SHACL governance shapes** | Semantic validation using W3C SHACL for constraints JSON Schema cannot express | 🟡 | ✘ |
| 13.3 | **Domain vocabulary extension** | Incorporate NIEM, FHIR, Schema.org vocabularies via @context extension | 🟡 | ✘ |
| 13.4 | **SPARQL-queryable workflow graphs** | Cross-workflow analysis via standard RDF queries | 🟡 | ✘ |

---

## 14. AI-Native Authoring

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 14.1 | **Typed patch operations** | AI proposes structural edits as statically analyzable AST-level operations | 🟡 | ✘ |
| 14.2 | **Four-stage patch validation** | JSON Schema, SHACL, soundness, provenance -- every AI edit validated before commit | 🟡 | ✘ |
| 14.3 | **Compositional authoring** | Reuse by URI reference; documents compose without copy-paste | ✅ | Cam ~ |
| 14.4 | **Workflow definition as data** | Declarative JSON, not imperative code -- AI-friendly for generation and analysis | ✅ | Cam ✔, KIE ✔, Flow ✔ |

---

## 15. Architecture & Extensibility

| # | Requirement | Description | WOS | Cam | KIE | Flow | Temp |
|---|------------|-------------|-----|-----|-----|------|------|
| 15.1 | **Layered opt-in** | Four layers independently adoptable; Kernel-only is valid | ✅ | ✘ | ✘ | ✘ | ✘ |
| 15.2 | **Sidecar document pattern** | Configuration in separate documents, independently updatable | ✅ | ✘ | ✘ | ✘ | ✘ |
| 15.3 | **Five extension seams** | Named attachment points (actor, contract, provenance, lifecycle, extensions) | ✅ | ~ | ✘ | ✘ | ✘ |
| 15.4 | **Four separation principles** | Lifecycle / case state / decision logic / audit / governance cleanly separated | ✅ | ~ | ~ | ~ | ✘ |
| 15.5 | **JSON-native format** | All documents JSON -- human-readable, machine-parseable, AI-friendly | ✅ | ✘ | ✘ | ✘ | ✘ |
| 15.6 | **Conformance profiles** | Multiple tiers per layer for incremental adoption | ✅ | ✘ | ✘ | ✘ | ✘ |
| 15.7 | **Open specification** | No vendor lock-in; any engine can implement | ✅ | ✔ | ✔ | ✔ | ~ |
| 15.8 | **181 lint rules** | Static analysis: 30 T1 + 50 T2 + 101 T3 | 🟦 | ~ | ✘ | ✘ | ✘ |
| 15.9 | **Open source engine available** | Production-ready open source execution engine | ⚪ | ✔ | ✔ | ✔ | ~ |
| 15.10 | **Production government deployment** | Deployed in government production today | ✘ | ~ | ~ | ~ | ~ |
| 15.11 | **FedRAMP / GovCloud authorization** | Cloud deployment with federal security authorization | ✘ | ✘ | ✘ | ✘ | ✘ |

---

## 16. Open Source Engine Comparison

WOS is a governance standard, not an execution engine. These engines are deployment targets for WOS bindings.

| Property | Camunda 8 | Apache KIE | Flowable | Temporal | Bonita | ProcessMaker |
|----------|-----------|------------|----------|----------|--------|--------------|
| **License** | Apache 2.0 / SSPL | Apache 2.0 (incubating) | Apache 2.0 | MIT (server) | GPL v2 | AGPL v3 |
| **Process model** | BPMN 2.0 | BPMN 2.0 + CMMN (partial) | BPMN 2.0 + CMMN 1.1 | Workflow-as-code | BPMN 2.0 | BPMN 2.0 |
| **Decision engine** | DMN 1.3 (FEEL) | DMN 1.5 + Drools DRL | DMN 1.3 (FEEL) | None | None | None |
| **Human tasks** | Tasklist UI, forms | WS-HumanTask, forms | WS-HumanTask, forms | None | Task mgmt UI | Task mgmt UI |
| **Durable execution** | Yes (Zeebe event log) | Partial (JPA) | Partial (DB) | Yes (event sourcing) | DB persistence | DB persistence |
| **Case management** | Dropped CMMN | Partial | Yes (CMMN 1.1) | No | No | No |
| **AI governance** | None | None | None | None | None | None |
| **WOS binding priority** | High -- largest community | High -- broadest features | Medium -- CMMN migration | High -- best execution | Low -- smaller community | Low -- PHP ecosystem |

---

## 17. Strategic Position

### WOS is not competing with workflow engines. It is the governance layer above them.

Existing engines define HOW work flows. WOS defines HOW WORK IS GOVERNED.

**What engines provide that WOS gets for free via bindings:**
1. BPMN visual notation with mature tooling
2. FEEL/DMN decision tables with production implementations
3. WS-HumanTask lifecycle with task management UIs
4. Process persistence and crash recovery
5. Existing government deployments and developer communities

**What WOS provides that cannot be bolted onto existing engines:**

1. **Deontic governance** -- Permission/Prohibition/Obligation/Right requires structural changes to how transitions are evaluated and violations recorded. BPMN extensions cannot express deontic semantics.
2. **Four-layer audit with epistemic status** -- Separating verified facts from AI narrative from counterfactuals requires a different provenance model than BPMN's flat event log.
3. **Structured oversight protocols** -- `independentFirst` requires the UI to suppress agent output until independent assessment is recorded. BPMN has no mechanism for this.
4. **Due process as structural requirement** -- Appeal topology, continuation-of-service, counterfactual explanation are workflow topology constraints BPMN validation cannot enforce.
5. **Universal interface contract** -- One spec for human forms, agent I/O, decision services, and integrations. BPMN uses XML Schema with no reactive behavior.
6. **Constraint zones** -- DCR-style declarative case management within a statechart. BPMN's ad-hoc sub-process has no formal relations or satisfiability verification.
7. **Formally verifiable constraints** -- SMT-provable governance properties. Neither BPMN nor DMN supports this.
8. **JSON-LD native** -- Every WOS document is simultaneously valid JSON and an RDF graph. BPMN requires the sBPMN semantic lifting pipeline.

### The fundamental architectural difference

> **Existing engines:** Process definitions are configurations for an execution engine. BPMN XML is meaningful only in the context of its engine.
>
> **WOS:** Workflow definitions are self-describing linked data documents that can be validated by JSON Schema, verified by SHACL, queried via SPARQL, proven by SMT, authored by AI via typed patches, and executed by any conformant processor on any engine.

This is not "BPMN in JSON." It is a governance document that happens to be executable, rather than an execution artifact that happens to have governance metadata.

---

## 18. Implementation Status Summary

| Component | Status | Detail |
|-----------|--------|--------|
| **18 Specifications** | ✅ | All normative prose written, all at v1.0.0-draft.1 (includes Business Calendar and Notification Template sidecars) |
| **18 JSON Schemas** | ✅ | All document types have JSON Schema 2020-12 validation |
| **24+ Fixtures** | 🟦 | Example documents for all layers; conformance fixtures for core scenarios |
| **wos-core crate** | 🟦 | Typed models for kernel/governance/AI/sidecars; evaluation algorithm; 9 traits; default runtime |
| **wos-lint crate** | 🟦 | 189 rules defined; 76 tested; T1 & T2 mostly covered; T3 needs fixtures |
| **wos-conformance crate** | 🟦 | Dynamic test runner; 194 tests passing; migrating to wos-core typed models |
| **Formspec Coprocessor** | ⚪ | Handoff protocol between Formspec forms and WOS workflows not yet specified (see below) |
| **Engine capabilities** | 🟡 | Deontic, compensation, confidence, DCR, delegation, autonomy, due process, pipelines |
| **Engine bindings** | ⚪ | Camunda, KIE, Flowable, Temporal bindings not yet started |
| **Documentation** | ⚪ | README updates, architecture docs, API docs |

### Critical Dependency: Formspec Coprocessor

The most significant architectural gap for SaaS deployment: there is no specified handoff protocol between WOS tasks and Formspec forms. This gap blocks the enterprise implementation roadmap (Phase 1.3). Needs to address:

- How `createTask` with a Formspec `contractRef` causes a form to be presented to an actor
- How a completed Response flows back into WOS `caseFile`
- How Response data maps to case file fields (normatively via Mapping DSL)
- Whether Response is validated before the workflow event fires
- Respondent Ledger integration for rights-impacting workflows

Best addressed as a "Formspec Coprocessor" section in the Runtime Companion or a new sidecar spec.

### What's Next (by phase)

1. **Phase 3** (in progress): Finish typed models in wos-core; consolidate timers; migrate wos-conformance
2. **Phase 4**: Write ~94 failing conformance fixtures for untested T3 rules
3. **Phase 5**: Implement 10 engine capability modules
4. **Phase 6**: Migrate wos-lint to typed model field access
5. **Phase 7**: Documentation
6. **Future**: Formspec Coprocessor spec; engine bindings (Camunda, KIE, Temporal); reference implementation; adoption wedge document

---

## Appendix A: Audit Corrections from Spreadsheet

The following features from the companion xlsx were corrected in this document based on spec verification:

| Feature | xlsx Rating | Corrected To | Reason |
|---------|------------|--------------|--------|
| Decision tables (DMN) | ■ | 🟡 (integration) | Kernel: "WOS does not embed a decision table engine." Available via integration. |
| Decision requirement graphs | ■ | 🟡 (integration) | Not defined in WOS specs; DMN concept available through policy engine bridge. |
| Merkle tree tamper-evident logging | ■ | 🟡 (per-record only) | Merkle trees in DRAFTS only; TODO: "Deferred / Not started." Published spec has per-record digests. |
| RO-Crate audit packaging | ■ | ⚪ | In DRAFTS v3-v7 only; not in any published spec. |
| MCP agent-tool protocol | ■ | -- (Formspec, not WOS) | MCP is a Formspec package, not specified in WOS. Available via Formspec integration. |
| CaMeL dual-LLM | ■ | 🟡 (informative) | S3.6: "Implementations MAY adopt." Informative, not normative. |
| Capability-based routing | ■ | ⚪ | Kernel: routing is "implementation-defined." Assignment roles exist; capability matching does not. |
| Sustainable caseload limits | ■ | ⚪ | Volume constraints exist for agents (S11.1); no human caseload limits in spec. |
| Defeasible rules | ■ | 🟡 (partial) | Authority ranking and constraint composition, not formal Catala-style defeasibility. |
| Statutory deadline chains | ★ | 🟡 | Achievable by composing timeouts + temporal parameters, but not a named spec feature. |
| Canada Directive alignment | ■ | (removed) | Not mentioned in any spec. Only OMB M-24-10 and EU AI Act cited. |
| A2A agent-to-agent protocol | ■ | (removed) | Not mentioned in any WOS spec. |
| Evidence lifecycle | ■ | (removed) | No formal evidence state machine in spec. Mutation history covers change tracking. |
| Work queue management | ■ | (removed) | EventQueue exists for events; no normative work queue feature for tasks. |

---

## Appendix B: Standards Lineage

**Adopted substantially intact:** WS-HumanTask lifecycle, CMMN case file model, DMN decision table concepts (via integration), CloudEvents envelope, W3C PROV Entity-Activity-Agent triad, JSON Schema, Harel statechart semantics, Saga compensation pattern, NIST AU control requirements.

**Adapted (concept redesigned):** BPMN event taxonomy, SCXML statechart semantics (JSON not XML), XACML PEP/PDP architecture, Catala default logic (as authority ranking), OpenFisca temporal parameters, GSM guard-stage-milestone, DCR include/exclude, Temporal durable execution guarantees (declarative not code-first), Certificate Transparency (per-record digests).

**Evaluated but not adopted:** WS-BPEL, XPDL, YAWL, S-BPM, Azure Durable Functions, Restate, Inngest, Cadence, Netflix Conductor, Drools/Rete, Google Zanzibar/Cedar, NIEM, HL7 FHIR.
