# Designing an AI-native standard for high-stakes workflow orchestration

**The most durable workflow abstractions are hierarchical state machines, declarative guards, event-sourced audit, and the clean separation of process flow from decision logic — not the flowchart-centric paradigms that dominate today's tooling.** After surveying 50+ standards, specifications, academic models, and systems across four decades of workflow research, a clear architectural thesis emerges: the next standard should be **case-centric and state-machine-native**, combining Harel statecharts for lifecycle structure, declarative constraints for flexibility, artifact-centric data modeling, and first-class human task management — all designed for LLM generation and formal verification. Existing standards each solve real problems but none integrates the full stack required for high-stakes government workflows. BPMN dominates tooling but struggles with discretionary work. CMMN was purpose-built for case management but underadopted. Temporal.io proved durable execution is tractable. DMN showed decision logic can be separated cleanly. W3C PROV defined provenance but missed decision rationale. The gap isn't any single capability — it's the integration of process, decision, task, audit, and resilience into a coherent, AI-native whole.

---

## A. Executive synthesis: what the landscape reveals

### Strongest inspirations across the corpus

**Three abstractions have proven durable across every generation of workflow technology.** First, hierarchical state machines (Harel, 1987) remain the most expressive and verifiable way to model complex lifecycles. Their concepts — nested states, parallel regions, history states, guard conditions — appear in SCXML, UML, XState, and implicitly in every case management system. Second, the separation of decision logic from process routing (crystallized by DMN's decision services) appears in every mature system — XACML's PEP/PDP split, OPA's policy-as-code, Cedar's principal-action-resource model. Third, event sourcing as the foundation for both durable execution and audit trails unifies Temporal's replay model, Greg Young's CQRS patterns, and the compliance requirements of NIST SP 800-53.

**The workflow patterns corpus (van der Aalst et al.) remains the single most valuable analytical framework.** Its 43 control-flow, 40 data, and 43 resource patterns provide a completeness benchmark no standard has fully satisfied. The Deferred Choice pattern (environment determines the next step, not the process), the Milestone pattern (activity enabled only when a condition holds), and resource patterns for separation of duties and escalation are essential for high-stakes work yet commonly unsupported.

### Recurring failures

Three failure modes recur across generations. **Over-specification of control flow** plagues BPMN-heavy implementations where rigid flowcharts cannot accommodate the discretionary judgment that dominates real casework. **Conflation of concerns** appears when process flow, decision logic, authorization, and task management are entangled in a single model — BPEL tried to be both a programming language and a process language and collapsed under the weight. **Vendor-centric serialization** (BPEL's WSDL/SOAP dependency, AWS Step Functions' ASL license restrictions) creates lock-in that undermines the interoperability a standard must provide.

### Critical gaps in the existing landscape

No existing standard addresses **decision provenance** — recording not just what happened but why, which rule version fired, what data was evaluated, and what the human override rationale was. No standard integrates **temporal parameter versioning** (government rules change on specific dates; OpenFisca handles this but no workflow standard does). No standard treats **human judgment and override authority** as first-class modeled concepts rather than ad-hoc escape hatches. No standard provides **AI-native authoring affordances** — structured representations optimized for LLM generation, round-trip natural-language explanation, and automated soundness verification.

### Design implications

The standard should define **seven distinct conceptual layers**: process topology (statechart-based lifecycle), decision logic (DMN-style decision services with defeasibility), human task management (WS-HumanTask lifecycle with modern assignment), case state and evidence (CMMN case file model), eventing and integration (CloudEvents-based), provenance and audit (W3C PROV extended with decision records), and durable execution contracts (abstract guarantees, not implementation mechanisms). Each layer should be independently evolvable, with well-defined interfaces between them.

---

## B. Research corpus: 50+ standards and systems surveyed

### Process and orchestration standards

| Standard | Body/Year | Core Contribution | Seed? | Verdict |
|----------|-----------|-------------------|-------|---------|
| **BPMN 2.0** | OMG/ISO 19510, 2011 | Flowchart-based process modeling with events, gateways, user tasks, compensation, collaboration diagrams | Yes | Adapted — rich event/error taxonomy; weak for ad-hoc case work |
| **CMMN 1.1** | OMG, 2016 | Discretionary items, sentries, milestones, case file, planning tables for knowledge-intensive work | Yes | Adopted — case lifecycle and discretionary work concepts are essential |
| **DMN 1.4** | OMG, 2021 | Decision tables, FEEL expression language, DRDs separating decisions from process | Yes | Adopted — decision service pattern and hit policies |
| **SCXML 1.0** | W3C, 2015 | Harel statecharts in XML with data model, event processing, invoke for external services | Yes | Adapted — statechart semantics adopted; XML serialization replaced |
| **XPDL 2.2** | WfMC, 2012 | First XML interchange format for workflow; graphical+semantic preservation | Yes | Out of scope — superseded; interchange concept carries forward |
| **WS-BPEL 2.0** | OASIS, 2007 | Structured activities, compensation handlers, correlation sets, fault handling | Yes | Adapted — compensation and correlation concepts; SOAP coupling abandoned |
| **WS-HumanTask 1.1** | OASIS, 2010 | 10-state task lifecycle, 5 generic roles, escalation chains, composite tasks, delegation | Yes | Adopted — most complete human task model; needs modernization |
| **Serverless Workflow 1.0** | CNCF, 2024 | YAML/JSON DSL, CloudEvents integration, cloud-native task types, retry policies | Yes | Adapted — DSL design patterns and cloud-native eventing approach |
| **YAWL 4.x** | Academic, 2003 | Workflow-pattern-complete language, OR-join, cancellation regions, Worklets for dynamic adaptation | No | Adapted — formal pattern coverage; cancellation region concept |
| **S-BPM** | Community, 1994+ | Subject-oriented modeling (5 symbols), communication-first, CCS formal foundation | No | Adapted — subject/actor model for inter-organizational workflows |
| **AWS Step Functions/ASL** | AWS, 2016+ | Declarative state machine, .waitForTaskToken callback, Map for parallel iteration | No | Adapted — callback pattern; vendor lock-in is an anti-pattern |
| **GSM (Guard-Stage-Milestone)** | IBM Research, 2010 | Artifact-centric lifecycle, declarative guards on stages/milestones, data-driven progression | No | Adopted — foundational for case-centric architecture |

### Formal models and workflow theory

| Model | Origin | Core Contribution | Seed? | Verdict |
|-------|--------|-------------------|-------|---------|
| **Harel Statecharts** | Harel, 1987 | Hierarchical states, parallel regions, history states, guard conditions | Yes | Adopted — foundational lifecycle model |
| **Workflow Patterns** | van der Aalst et al., 2003 | 43+40+43 patterns across control, data, resources; completeness benchmark | Yes | Adopted — the evaluation framework |
| **Petri Nets / WF-nets** | Petri 1962; van der Aalst 1997 | Formal soundness (deadlock, liveness, proper termination), decidability results | Yes | Adapted — verification theory; undecidability limits with cancellation |
| **Saga Pattern** | Garcia-Molina & Salem, 1987 | Compensating transactions for long-running processes, forward/backward recovery | Yes | Adopted — essential for multi-step workflows |
| **Actor Model** | Hewitt, 1973 | Isolated state, message passing, supervision trees (Erlang/OTP) | Yes | Adapted — implementation architecture, not specification language |
| **DECLARE** | Pesic & van der Aalst, 2006 | LTL-based declarative constraints, open-world semantics, runtime monitoring | No | Adapted — constraint templates for compliance rules |
| **DCR Graphs** | Hildebrandt & Mukkamala, 2010 | Four primitive relations (condition, response, include, exclude), deployed in Danish government | No | Adapted — simplest formally grounded declarative model |
| **Process Algebras (CSP, π-calculus)** | Hoare 1978; Milner 1992 | Compositional specification, bisimulation, channel mobility | Yes | Out of scope — theoretical foundation, too abstract for practitioners |

### Decision, policy, and authorization systems

| System | Origin | Core Contribution | Seed? | Verdict |
|--------|--------|-------------------|-------|---------|
| **DMN** | OMG | Decision tables with hit policies, FEEL, DRDs | Yes | Adopted |
| **OPA/Rego** | CNCF | Policy-as-code, partial evaluation, decision logging | Yes | Adapted — decision logging pattern; Rego too developer-centric |
| **Cedar** | AWS, 2023 | Formally verifiable authorization, permit/forbid with principal-action-resource | Yes | Adapted — formal verification approach for policy safety |
| **XACML 3.0** | OASIS | Combining algorithms, obligations/advice, PEP/PDP/PAP architecture | Yes | Adapted — architecture pattern; XML verbosity abandoned |
| **Google Zanzibar/ReBAC** | Google, 2019 | Relationship-based authorization via tuples, global consistency | No | Adapted — relationship model for case-level authorization |
| **Catala** | Inria, 2020s | Default logic for legislation, literate programming for law, exception handling | No | Adapted — defeasibility pattern essential for regulatory rules |
| **OpenFisca** | Community | Temporal parameter versioning, tax-benefit microsimulation, reforms | No | Adapted — date-effective parameter model |
| **LegalRuleML 1.0** | OASIS, 2021 | Defeasible logic, deontic operators, isomorphism to legal source text | No | Adapted — deontic concepts and legal traceability |
| **Drools/Rete** | Forgy 1979; Red Hat | Forward-chaining production rules, Rete algorithm, DMN integration | No | Out of scope — implementation engine, not specification |

### Data contracts, eventing, and integration

| Standard | Origin | Core Contribution | Seed? | Verdict |
|----------|--------|-------------------|-------|---------|
| **JSON Schema 2020-12** | IETF | Validation, conditional schemas, vocabulary extension, $ref composition | Yes | Adopted — schema language for all workflow data |
| **OpenAPI 3.1** | Linux Foundation | Typed sync API interfaces, callbacks, webhooks, discriminators | Yes | Adopted — integration interface definition |
| **AsyncAPI 3.0** | Linux Foundation | Async message-driven interfaces, correlation IDs, protocol bindings | Yes | Adopted — event-driven integration surface |
| **CloudEvents 1.0** | CNCF Graduated | Vendor-neutral event envelope, protocol bindings, extensions, CESQL | Yes | Adopted — event envelope standard |
| **Standard Webhooks** | Community | Idempotency keys, signature verification, delivery patterns | No | Adopted — callback delivery pattern |
| **NIEM 6.0** | OASIS/NIEMOpen | Government data vocabulary (Justice, Human Services, Emergency Management) | No | Adapted — domain vocabulary for government case data |
| **HL7 FHIR R5** | HL7 | Definition→Request→Event pattern, Task resource, extension mechanism | No | Adapted — resource taxonomy pattern |
| **W3C Trace Context** | W3C | traceparent/tracestate for distributed tracing propagation | No | Adopted — observability correlation |
| **Enterprise Integration Patterns** | Hohpe & Woolf, 2003 | Process Manager, Routing Slip, Scatter-Gather, Correlation Identifier, Claim Check | No | Adapted — foundational integration vocabulary |

### Provenance, audit, and observability

| Standard | Origin | Core Contribution | Seed? | Verdict |
|----------|--------|-------------------|-------|---------|
| **W3C PROV** | W3C, 2013 | Entity-Activity-Agent triad, delegation, derivation, bundles | Yes | Adopted — provenance data model |
| **OpenTelemetry** | CNCF Graduated | Traces/spans/context, semantic conventions, baggage propagation | Yes | Adopted — infrastructure observability layer |
| **IEEE XES (1849-2016)** | IEEE | Event log standard for process mining (Log→Trace→Event) | No | Adapted — process mining interoperability |
| **Certificate Transparency / RFC 9162** | IETF | Merkle tree audit logs, inclusion/consistency proofs | No | Adapted — tamper-evidence mechanism |
| **PREMIS 3.0** | Library of Congress | Event-Agent-Object-Rights for digital preservation | No | Adapted — rights-aware audit metadata model |
| **NIST SP 800-53 AU controls** | NIST | Federal audit requirements (AU-2 through AU-11) | No | Adopted — compliance requirements baseline |

### Durable execution systems

| System | Origin | Core Contribution | Seed? | Verdict |
|--------|--------|-------------------|-------|---------|
| **Temporal.io** | Temporal Technologies, 2019 | Workflows-as-code, deterministic replay, durable timers, signals/queries/updates | Yes | Adapted — execution model concepts; code-first approach is implementation choice |
| **Restate** | Restate, 2023 | Embedded durable execution, virtual objects, journaling | Yes | Adapted — simpler durability model |
| **Azure Durable Functions** | Microsoft | Orchestrator/activity/entity functions, fan-out/fan-in | No | Adapted — pattern catalog |
| **AWS Step Functions** | AWS | Declarative state machine, callback tokens, Map state | No | Adapted — callback pattern |
| **Netflix Conductor** | Netflix/Orkes | Centralized orchestration, JSON workflow DSL, polling workers | No | Out of scope — traditional orchestrator |
| **Cadence** | Uber, 2017 | Pioneer of workflows-as-code with event-sourced replay | No | Out of scope — superseded by Temporal |
| **DBOS** | MIT/DBOS Inc | Postgres-backed embedded durable execution, minimal infrastructure | No | Adapted — embedded durability approach |
| **Inngest** | Inngest | Event-driven step memoization, no determinism constraints | No | Adapted — step memoization without replay complexity |

---

## C. Feature taxonomy: capabilities for high-stakes workflow

### Process topology and lifecycle

The standard must model **hierarchical state machines** (nested phases/sub-phases), **parallel regions** (concurrent review tracks), **sequential composition**, **conditional branching** (exclusive/inclusive), and **structured loops** with bounded iteration. BPMN's gateway taxonomy provides the branching vocabulary. Harel's hierarchy provides the nesting model. The critical addition is **milestone-gated progression** from GSM/CMMN — stages that advance based on data conditions, not just control flow completion.

**Deferred choice** (the environment determines the next step, not the designer) is the single most important pattern for case management. Traditional workflow forces the designer to predetermine routing; deferred choice lets incoming evidence, human judgment, or external events drive the path. DCR Graphs' include/exclude mechanism offers the most elegant formalization: activities can be dynamically enabled or disabled based on case evolution.

### States, transitions, and guards

Every state transition requires a **guard condition** (a boolean expression evaluated against case data and context), a **triggering event** (or explicit human action), and optional **entry/exit actions**. Guards should be expressible in a subset of FEEL (DMN's expression language) — it was designed to be "friendly enough" for business users while remaining executable. **History states** (shallow and deep) from Harel statecharts are non-negotiable for case management: when a suspended case resumes, it must restore its prior internal configuration, not restart from the beginning.

The standard should define a **core set of lifecycle states** for both workflows and tasks. For workflow instances: Created, Active, Suspended, Completed, Failed, Cancelled, Terminated. For human tasks, the WS-HumanTask lifecycle provides the most battle-tested model, which can be simplified to: **Available** (in queue), **Claimed** (assigned to worker), **InProgress** (actively worked), **Completed**, **Returned** (sent back for rework), **Escalated**, **Failed**, **Suspended**, **Cancelled**.

### Human tasks, queues, and assignment

This is the most **under-modeled capability** in existing standards. WS-HumanTask's five generic roles (task initiator, task stakeholders, potential owners, actual owner, business administrators) provide the right abstraction layer. The standard must model: **claim/unclaim** from shared queues, **delegation** (transfer to a specific person), **forwarding** (transfer to a group), **escalation** (automatic on SLA breach), **four-eyes principle** (separation of duties enforcement), **rework loops** (return for revision with iteration count tracking), and **supervisory override** (manual reassignment by administrators).

**Skill-based routing** should be a first-class concept: work items carry required competencies; workers declare capabilities; the matching engine assigns accordingly. **SLA management** requires modeling due dates with warning thresholds and automated escalation triggers. Priority should be computable from case data, not just statically assigned.

### Decisions and policy

DMN's decision service pattern — an encapsulated decision with defined inputs invocable from a process — is the right boundary between process and decision logic. The standard should support **decision tables with hit policies** (unique, first, priority, collect), **decision requirement diagrams** showing how decisions compose, and **FEEL expressions** for conditions and data transformation.

Beyond DMN, the standard must incorporate **defeasibility** from Catala/LegalRuleML: general rules with structured exceptions, where more specific provisions override general ones with explicit priority encoding. Government regulations pervasively follow this "general rule + exceptions" pattern. **Temporal parameter versioning** (from OpenFisca) is essential: rates, thresholds, and eligibility criteria change on specific dates, and the system must apply rules effective at the relevant time, not necessarily the current time.

Authorization requires a **layered model**: RBAC for structural roles (caseworker, supervisor, adjudicator), ABAC for contextual constraints (clearance, jurisdiction, conflict of interest), and ReBAC for relationship-aware access (the specific case's assigned reviewer). The **PEP/PDP separation** from XACML should be adopted architecturally: decision evaluation is centralized and auditable, enforcement is distributed at workflow execution points.

### Timers and temporal modeling

The standard should define **durable timers** that survive process restarts and consume no resources while waiting. Timer types include: **deadline timers** (absolute datetime), **duration timers** (relative to an event), **recurring timers** (CRON-based periodic checks), and **SLA timers** (computed from case data and policy). Temporal.io proved that durable timers spanning months are tractable; the standard should require this capability without mandating the implementation mechanism.

**Temporal expressions** should cover: absolute dates/times, durations (ISO 8601), CRON expressions for recurring schedules, and computed deadlines (e.g., "10 business days from submission, excluding federal holidays in the applicant's jurisdiction"). Business calendar support — handling weekends, holidays, and jurisdiction-specific non-working days — is essential for government SLA enforcement.

### Exceptions, compensation, and recovery

The saga pattern from Garcia-Molina & Salem provides the formal foundation. Every workflow step should optionally declare a **compensating action** — a semantically meaningful reversal. The standard should model both **backward compensation** (undo completed steps in reverse order) and **forward recovery** (retry or take alternative path). A **pivot step** concept distinguishes compensable steps (before the point of no return) from retryable steps (after).

**Exception taxonomy** should distinguish: anticipated exceptions (modeled in the workflow with explicit handling), unanticipated exceptions (handled by generic error boundaries and escalation to humans), timeout exceptions (SLA breach, activity timeout, heartbeat failure), and **cancellation** (cooperative propagation from parent to children with optional compensation). YAWL's cancellation regions — the ability to cancel all work items in a specified region of the process — are more expressive than BPMN's simple cancel event and should be supported.

### Case state, evidence, and data

Following CMMN and GSM, the standard should treat the **case artifact** as the primary entity: a typed data container that accumulates evidence, documents, decisions, and state over time. The case file is not merely workflow data — it is the subject of the workflow. Case file items should be typed (JSON Schema), versioned (every mutation recorded as an event), and accessible as context for guard conditions and decision evaluation.

**Evidence attachment** should be first-class: documents, forms, images, and structured data linked to specific workflow events with content hashing for integrity verification. The **claim check pattern** from Enterprise Integration Patterns handles large documents — the workflow carries a reference and content hash, not the document itself. Case data should support **selective visibility**: different roles see different subsets of the case file based on authorization policy.

### Integrations and eventing

**CloudEvents** should be adopted as the event envelope for all external interactions, with workflow-specific extension attributes: `workflowinstanceid`, `correlationkey`, `causationeventid`, `taskid`, and `workflowdefinitionversion`. The standard should define how workflows consume external events (correlation by business key to match events to running instances), produce events (status changes, notifications, callbacks), and integrate with external services (via OpenAPI for synchronous and AsyncAPI for asynchronous interfaces).

**Idempotency** must be required: every event carries a stable identifier; every step is designed for at-least-once execution. The **correlation architecture** should specify: workflow instance ID (globally unique, immutable), correlation keys (business identifiers for event matching), causation event ID (the event that triggered this action), and W3C Trace Context for observability.

### Provenance and audit

The standard must produce **complete, immutable, queryable audit records** satisfying NIST SP 800-53 AU-2/AU-3 content requirements: who, what, when, where, outcome, identity, and authority. W3C PROV's Entity-Activity-Agent triad provides the provenance data model, extended with **decision records** that capture: rule/policy version, input data snapshot at decision time, evaluation trace (which rules fired), confidence scores for AI-assisted decisions, and structured override rationale when humans override system recommendations.

**Tamper evidence** via Merkle tree hash-chaining (following Certificate Transparency / RFC 9162) should be specified: append-only event log, signed tree heads at configurable intervals, inclusion proofs for any audit event, and consistency proofs between any two points in time. This provides mathematical proof that audit records haven't been altered — a stronger guarantee than database-level access controls alone.

**Process mining interoperability** via IEEE XES-compatible event emission enables automated conformance checking: comparing actual case execution against defined process models to detect deviations, measure fitness, and identify compliance violations.

### Durability and runtime

The standard should define **abstract execution guarantees** without mandating implementation mechanisms. Required guarantees: workflow execution survives infrastructure failures (crash recovery), workflow progress is durable (persisted state), steps/activities execute at-least-once (with developer-responsible idempotency), and durable timers span arbitrarily long periods with zero resource consumption during waits.

**What belongs in the standard**: workflow execution states (Running, Completed, Failed, Cancelled, TimedOut, Terminated), retry policy schema (max attempts, backoff strategy, non-retryable error types), timeout categories (step execution, overall, heartbeat, queue wait), cancellation semantics (cooperative propagation), compensation registration, external signal/event ingestion, and query/inspection interface for workflow state.

**What stays implementation-specific**: the durability mechanism itself (event-sourced replay vs. checkpointing vs. journaling vs. direct persistence), whether developers must write deterministic code, the execution architecture (external server vs. embedded library vs. managed service), storage backend, scaling characteristics, and programming model (code-first vs. declarative vs. hybrid).

### Validation, conformance, and AI-native authoring

The standard's serialization format should be **YAML with strict JSON Schema validation underneath** — satisfying three requirements simultaneously: human-readable for review and audit, schema-validatable for correctness, and LLM-generatable for AI authoring. The JSON Schema vocabulary extension mechanism allows defining workflow-specific keywords validated by standard tooling.

**Automated soundness verification** should be specified as a conformance requirement: every workflow definition must be checkable for deadlock-freedom (no state from which completion is unreachable), livelock-freedom (no infinite non-productive cycles), proper termination (when the end state is reached, no orphaned work items remain), and reachability (every defined task can potentially execute). Petri net soundness theory provides the formal foundation, though verification must account for data-dependent guards (staying within decidable fragments).

**Round-trip natural language explanation** should be a design goal: every workflow definition should be translatable to a human-readable description, and natural language process descriptions should be translatable to valid workflow definitions. This requires a serialization format where the mapping between structured representation and natural language is learnable by LLMs — another argument for YAML with clear, self-documenting key names over terse or cryptic formats.

---

## D. Feature matrix: systems compared against the taxonomy

| Capability | BPMN | CMMN | SCXML | Serverless WF | Temporal | WS-HumanTask | DMN | CloudEvents | W3C PROV |
|---|---|---|---|---|---|---|---|---|---|
| Hierarchical states | ⚠️ Subprocess | ⚠️ Stages | ✅ Native | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Parallel regions | ✅ Gateway | ⚠️ Stages | ✅ Native | ✅ Fork | ✅ Child WF | ❌ | ❌ | ❌ | ❌ |
| Guard conditions | ⚠️ Conditional | ✅ Sentries | ✅ Native | ✅ Switch | ❌ (in code) | ❌ | ✅ FEEL | ❌ | ❌ |
| History states | ❌ | ❌ | ✅ Native | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Deferred choice | ⚠️ Event GW | ✅ Discretionary | ✅ Events | ⚠️ Listen | ✅ Signals | ❌ | ❌ | ❌ | ❌ |
| Milestones | ❌ | ✅ Native | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Human task lifecycle | ✅ User Task | ⚠️ Delegates | ❌ | ❌ | ⚠️ Signals | ✅ Full 10-state | ❌ | ❌ | ❌ |
| Claim/delegate/escalate | ⚠️ Tooling | ⚠️ Roles | ❌ | ❌ | ❌ | ✅ Full | ❌ | ❌ | ❌ |
| Separation of duties | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Decision tables | ⚠️ BRT | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ Full | ❌ | ❌ |
| Defeasible rules | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Compensation/saga | ✅ Native | ❌ | ❌ | ⚠️ Try/catch | ✅ Saga pattern | ❌ | ❌ | ❌ | ❌ |
| Durable timers | ⚠️ Timer event | ⚠️ Timer listener | ⚠️ Send delay | ✅ Wait | ✅ Unlimited | ⚠️ Deadlines | ❌ | ❌ | ❌ |
| Event correlation | ⚠️ Message | ⚠️ Sentry events | ⚠️ Send/receive | ✅ CloudEvents | ✅ Signals | ❌ | ❌ | ⚠️ Source+ID | ❌ |
| Audit/provenance | ❌ | ❌ | ❌ | ❌ | ✅ Event history | ❌ | ⚠️ DRD trace | ❌ | ✅ Full |
| Decision rationale | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ Table trace | ❌ | ❌ |
| Schema validation | ⚠️ XSD | ❌ | ⚠️ Datamodel | ✅ JSON Schema | ❌ | ❌ | ⚠️ FEEL types | ⚠️ dataschema | ❌ |
| Formal verification | ⚠️ Via Petri net mapping | ❌ | ✅ Model checking | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ PROV-CONSTRAINTS |
| AI-native format | ❌ XML-heavy | ❌ XML | ❌ XML | ✅ YAML/JSON | ❌ Code-first | ❌ XML | ⚠️ XML | ✅ JSON | ❌ RDF/OWL |

### Adopted/Adapted/Missing/Out-of-scope classification

**Adopted substantially intact:** WS-HumanTask lifecycle states and role model. CMMN case file and discretionary items. DMN decision tables with hit policies and FEEL. CloudEvents as event envelope. W3C PROV Entity-Activity-Agent triad. JSON Schema for data validation. Harel statechart hierarchical state semantics. Saga compensation pattern. NIST AU control requirements.

**Adapted — useful idea, redesigned for modern context:** BPMN event taxonomy (simplified, event-driven rather than flow-embedded). SCXML statechart XML (semantics kept, XML serialization replaced with YAML/JSON). XACML PEP/PDP architecture (without XML verbosity). Catala default logic (as a capability in the decision layer, not a separate language). OpenFisca temporal parameters (date-effective values in policy). GSM guard-stage-milestone (as the case lifecycle model). DCR Graph include/exclude (for dynamic process adaptation). Workflow patterns (as conformance checklist, not implementation prescription). Temporal durable execution concepts (as abstract guarantees, not code-first model). Certificate Transparency Merkle proofs (for audit tamper evidence).

**Missing — important, not well handled by any existing system:** Decision provenance records (which rule version, what data, what override rationale). Human override authority with structured accountability. AI-confidence annotation on automated/assisted decisions. Temporal parameter versioning in workflow context. Business calendar-aware deadline computation. Selective case file visibility by role. Round-trip natural language ↔ executable specification. Conformance testing framework with canonical fixtures. Dynamic process adaptation based on accumulated evidence. Policy-impact simulation (what happens if this rule changes?).

**Deliberately out of scope:** General-purpose programming constructs (the standard is not a programming language). Specific execution architecture (event sourcing vs. checkpointing). Storage and persistence mechanisms. UI rendering and form specification. Specific transport protocols beyond interface contracts. Process mining algorithms (the standard should emit compatible data, not include mining). Machine learning model training or inference. Full ontological reasoning (OWL/RDF).

---

## E. Important standards and models discovered during research

**DCR Graphs (Dynamic Condition Response Graphs)** from the IT University of Copenhagen deserve particular attention. With just four primitive relations — condition, response, include, exclude — DCR Graphs provide a declarative process model that is formally grounded, operationally executable, and deployed in production for Danish government case management. Their simplicity makes them the most AI-friendly formal model discovered: LLMs can generate and validate four-relation constraint specifications far more reliably than complex flowcharts. The include/exclude mechanism elegantly models processes that evolve based on findings — a case investigation that discovers fraud can dynamically include additional verification steps.

**Catala**, developed at Inria, addresses a gap no other system fills: formalizing legislation as executable code with **default logic as a first-class language feature**. Legal statutes universally follow a "general rule + exceptions" pattern, and Catala embeds this directly in syntax — base definitions can be overridden by more specific exceptions with structurally encoded priorities. Catala uncovered an actual bug in France's official family benefits implementation during formalization, demonstrating that formal specification of regulatory rules catches errors that traditional development misses.

**IEEE XES (1849-2016)** provides a standardized event log format (Log → Trace → Event with attributes) used by **40+ process mining tools**. A workflow standard that emits XES-compatible event data immediately enables conformance checking — automated comparison of actual case execution against defined process models. This is transformative for compliance: rather than manual audit sampling, every case can be automatically checked for process conformance.

**NIEM (National Information Exchange Model)**, now an OASIS Open Project, provides **10,000+ standardized data elements** across domains directly relevant to the target use cases: Justice (investigations, evidence), Human Services (benefits, child welfare), Emergency Management, Immigration. While NIEM is a data vocabulary rather than a process standard, any workflow standard for government use should align its case data model with NIEM domain vocabularies to enable interoperability across agencies.

**HL7 FHIR's Definition→Request→Event pattern** is a powerful generalization beyond healthcare. Definitions specify what can be done. Requests specify what should be done. Events record what was done. This trinity maps directly to workflow: process definitions, task assignments, and execution records. FHIR's Task resource as a first-class entity (not just an activity embedded in a process) validates the approach of treating human tasks as independently manageable objects.

**Guard-Stage-Milestone (GSM)** from IBM Research is the theoretical foundation behind CMMN, and in some ways more elegant than the standard it inspired. GSM's insight is that business processes are fundamentally **data-centric, not control-flow-centric**: a grant application is the artifact, data accumulates as it moves through stages, and milestones are achieved when data conditions are met. This inverts the BPMN paradigm where data is subordinate to flow.

**OpenFisca's temporal parameter model** solves a practical problem no workflow standard addresses: government rates, thresholds, and eligibility criteria change on specific dates, and the system must correctly apply the rules effective at the time relevant to each case. OpenFisca's approach — parameters with date-indexed values — should be adopted for the policy layer.

**Inngest's step memoization model** offers an alternative to Temporal's deterministic replay that eliminates the developer burden of writing deterministic code. Each step executes once; its result is persisted; on re-invocation, stored results are injected. This approach is more developer-friendly and demonstrates that durable execution does not require the complexity of full event-sourced replay.

**LegalRuleML's deontic operators** (obligation, permission, prohibition) provide formal vocabulary for what government workflows fundamentally regulate: what must, may, and must not be done. Combined with defeasible reasoning (rules that can be overridden by more specific exceptions) and temporal management (when rules take effect), LegalRuleML offers the most comprehensive model for representing regulatory constraints, despite its limited practical tooling.

**PREMIS 3.0** from the Library of Congress models digital preservation metadata with an Event-Agent-Object-Rights structure directly applicable to workflow audit. Its emphasis on rights — what actions are permitted on which objects by which agents under what conditions — adds an authorization dimension missing from W3C PROV.

---

## F. Design implications: conceptual layers and object model

### Seven-layer architecture

The standard should define seven distinct, independently evolvable layers with well-defined interfaces:

**Layer 1 — Lifecycle and Topology.** Defines the statechart-based process structure: hierarchical states, parallel regions, transitions with guards, history states, milestones as data-condition checkpoints. Serialized as a state machine definition. The primary influence is Harel statecharts formalized via SCXML semantics, augmented with CMMN's stage/milestone concepts and workflow pattern coverage.

**Layer 2 — Decision and Policy.** Encapsulates all routing logic, eligibility rules, and policy evaluation as independently versionable decision services following DMN's pattern. Supports decision tables with hit policies, FEEL expressions, defeasible rules with structured exception handling (Catala-style), and temporal parameter versioning (OpenFisca-style). The PEP/PDP separation from XACML ensures decisions are centralized, logged, and explainable.

**Layer 3 — Human Task and Work Management.** Defines the task lifecycle (based on WS-HumanTask, simplified), assignment model (roles, skills, queues), interaction patterns (claim, delegate, escalate, return for rework), SLA enforcement (durable timer-based deadlines with escalation chains), and separation-of-duties constraints. This layer is conspicuously absent from most modern workflow frameworks and represents the standard's key differentiation.

**Layer 4 — Case State and Evidence.** The data layer: typed case artifacts (JSON Schema-validated), evidence containers with content hashing, selective visibility by role, and event-sourced mutation history. Follows CMMN's case file concept and GSM's artifact-centric approach. Every case data mutation is an immutable event in the audit stream.

**Layer 5 — Integration and Eventing.** Defines how workflows interact with external systems: CloudEvents-based event consumption and production, OpenAPI/AsyncAPI interface contracts, webhook callbacks with Standard Webhooks verification, correlation architecture for matching external events to running instances, and the claim check pattern for document-heavy workflows.

**Layer 6 — Provenance and Audit.** The immutable record of everything that happened, built on W3C PROV's Entity-Activity-Agent model extended with decision records (rule version, input snapshot, evaluation trace, override rationale, AI confidence). Tamper evidence via Merkle tree hash-chaining. XES-compatible event emission for process mining. NIST AU-2/AU-3 compliant record content.

**Layer 7 — Durable Execution Contract.** Abstract guarantees that implementations must provide: crash recovery, durable state, at-least-once step execution, durable timers, external signal ingestion, and cancellation propagation. Defines the retry policy schema and timeout categories. Does NOT prescribe implementation mechanism.

### Likely object model

The core objects in the standard's metamodel would be: **WorkflowDefinition** (the template), **WorkflowInstance** (a running case), **State** (hierarchical, with parallel regions), **Transition** (event + guard → target state + actions), **Task** (human or automated work unit with lifecycle), **DecisionService** (encapsulated decision logic with inputs/outputs), **CaseFile** (typed data container), **CaseFileItem** (typed datum with schema), **Event** (CloudEvents-extended envelope), **AuditRecord** (PROV-based provenance entry), **PolicyRule** (defeasible rule with priority), **Timer** (durable temporal trigger), and **CompensationHandler** (saga step reversal).

### Serialization priorities

YAML is the primary human-authoring format; JSON is the canonical machine format (YAML is a superset of JSON, enabling lossless conversion). JSON Schema 2020-12 with a custom workflow vocabulary provides validation. All identifiers should be URIs for global uniqueness. Timestamps follow RFC 3339. Durations follow ISO 8601. Expression language is FEEL (for guards, conditions, data transformations) — chosen for business-user readability over developer expressiveness.

### Non-goals

The standard should explicitly exclude: general-purpose computation (it's not a programming language), UI rendering (it defines task data contracts, not form layouts), specific persistence mechanisms, specific transport protocols, machine learning model specification, and process mining algorithms. It should resist the temptation to become a "theory of everything" — BPMN's 500-page specification demonstrates the risk of over-inclusion.

### Key tradeoffs

**Expressiveness vs. verifiability.** More expressive workflow constructs (arbitrary cycles, data-dependent cancellation) make formal verification undecidable. The standard should define a verifiable core (structured control flow without reset arcs) and a pragmatic extension layer (arbitrary patterns that can be validated heuristically but not proven correct).

**Prescriptive vs. declarative.** Imperative process definitions (BPMN-style "do this, then that") are easier to understand for simple workflows but brittle for complex casework. Declarative constraints (DECLARE/DCR-style "these rules must hold") are more flexible but harder to comprehend. The standard should support both paradigms: imperative lifecycle structure with declarative compliance constraints layered on top.

**Standardization breadth vs. adoption feasibility.** A standard covering all seven layers is comprehensive but risks the XPDL fate of theoretical completeness with shallow adoption. A phased approach — core lifecycle and task management first, then decision and audit layers, then AI-native tooling — may be more practical.

---

## G. Conformance strategy

### Minimal conformance profiles

**Level 1 — Structural Conformance.** The implementation can parse and validate workflow definitions against the standard's JSON Schema. It can emit valid workflow definitions. No execution required. Enables tooling interoperability (editors, validators, migrators).

**Level 2 — Lifecycle Conformance.** The implementation correctly executes the statechart lifecycle semantics: state entry/exit, transition firing with guard evaluation, parallel region semantics, history state restoration. Tested via canonical workflow fixtures with expected state sequences.

**Level 3 — Task Management Conformance.** The implementation supports the human task lifecycle (Available → Claimed → InProgress → Completed/Returned/Escalated), assignment operations (claim, delegate, release, escalate), and SLA timer enforcement. Tested via task operation sequences with expected state transitions.

**Level 4 — Decision Conformance.** The implementation evaluates decision tables with hit policies and FEEL expressions correctly. Follows DMN's conformance levels model (already well-defined).

**Level 5 — Full Conformance.** All layers: lifecycle, tasks, decisions, case state, eventing, provenance, and durable execution guarantees. This is the target for production case management systems.

### Canonical fixtures

The conformance suite should include: a **grants processing workflow** (multi-stage review with parallel technical and financial evaluation, eligibility decision table, discretionary site visit, SLA-enforced deadlines, four-eyes approval, and appeal/rework loop), a **licensing workflow** (application intake with document verification, background check integration, conditional approval with remediation, renewal/modification lifecycle), and an **investigation workflow** (evidence-driven progression, discretionary activities based on findings, dynamic scope expansion, multi-agency coordination, suspension/reopen).

Each fixture should specify: the workflow definition in the standard's format, a set of input scenarios (normal path, exception paths, timeout paths, cancellation paths), expected state sequences for each scenario, expected audit records, and expected decision traces.

### Verification approaches

**Static validation** checks structural properties: schema conformance, soundness (deadlock-freedom, livelock-freedom, proper termination, no dead activities), guard completeness (every transition from a state either has a default or the guards are exhaustive), and timer consistency (no conflicting deadline specifications).

**Runtime behavior checks** verify execution semantics: correct state transitions, guard evaluation, parallel synchronization, history state restoration, compensation execution order, timer accuracy, event correlation, and task lifecycle operations.

**Portability testing** verifies that a workflow definition produces equivalent behavior across different compliant implementations. This requires canonical input/output pairs and deterministic execution for the verifiable core.

---

## H. Candidate architecture directions

### Direction 1: State-machine-centric (Harel-first)

The workflow is fundamentally a hierarchical state machine. States represent case lifecycle phases. Transitions are guarded by conditions and triggered by events. Parallel regions model concurrent activities. Human tasks and decisions are actions attached to states and transitions.

**Strengths:** Formally verifiable. Natural for case lifecycle modeling. Maps directly to visual representation. History states handle suspension/resumption. Well-understood semantics. AI-generatable (state machines have simple structure).

**Weaknesses:** Can become awkward for data-driven, ad-hoc processes where the lifecycle isn't well-defined in advance. Requires careful state decomposition for complex workflows. Less natural for pure "do these steps in order" scenarios.

### Direction 2: Case-centric artifact model (GSM-first)

The case artifact is primary. It has a data schema that accumulates over time. Guard conditions on stages and milestones drive progression based on data state. Activities (human and automated) produce data that advances the case. No explicit control flow — the data determines what happens next.

**Strengths:** Most natural for knowledge-intensive, evidence-driven casework. Declarative. Data changes drive behavior — closer to how caseworkers actually think. Integrates well with decision services. Guards ARE the process logic.

**Weaknesses:** Counterintuitive for developers accustomed to imperative flow. Difficult to visualize progression without explicit flow. Verification is harder (artifact systems with data are generally undecidable). Less tooling ecosystem.

### Direction 3: Hybrid process + policy + task (layered)

Three independently defined layers: a lifecycle layer (statechart-based), a policy layer (decision tables and rules), and a task layer (human work management). The lifecycle defines the overall case structure. The policy layer determines routing, eligibility, and constraints. The task layer manages human work assignment and completion. Each layer is authored, versioned, and tested independently.

**Strengths:** Clean separation of concerns. Each layer uses the best formalism for its problem (statecharts for lifecycle, DMN for decisions, WS-HumanTask for work management). Independently evolvable. Testable in isolation. Policy changes don't require process changes. Task assignment rules can change without touching lifecycle.

**Weaknesses:** Integration complexity between layers. Multiple formalisms to learn. Risk of impedance mismatch at layer boundaries. May feel over-engineered for simple workflows.

### Direction 4: Event-first orchestration (event-sourced)

Everything is an event. The workflow definition specifies event patterns and reactions. Case state is derived from the event stream. Activities produce events. Decisions consume events and produce events. Human tasks are modeled as "waiting for human-produced event." The event log IS the case history.

**Strengths:** Natural audit trail. Replay for debugging. Event-driven architecture alignment. Temporal decoupling. Process mining compatibility. Strong for integration-heavy workflows.

**Weaknesses:** Cognitive overhead (thinking in events rather than states or flows). Event schema evolution is the hardest practical problem. No obvious lifecycle visualization. Can feel like "programming with events" rather than "defining a process."

### Recommendation: Direction 3 (Hybrid) with Direction 1's lifecycle model

The hybrid layered architecture is the strongest candidate because it **respects the fundamental insight that process flow, decision logic, and task management are different concerns requiring different formalisms**. The lifecycle layer should use Direction 1's statechart approach (providing formal verifiability and natural case modeling). The event-first approach from Direction 4 should inform the audit and integration layers (event sourcing for provenance, CloudEvents for integration). Direction 2's artifact-centric insight should shape the case data model (the case file is the central artifact, and data conditions drive guard evaluation).

This architecture produces a standard where: a caseworker can understand the lifecycle (visual statechart), a policy analyst can manage the rules (decision tables), a supervisor can configure work assignment (task routing rules), an auditor can trace every action (event-sourced provenance), and an LLM can generate, validate, and explain all three layers independently.

---

## Four ranked lists

### Ten design principles for the new standard

1. **Separate concerns ruthlessly.** Process topology, decision logic, task management, case data, authorization, and audit are different problems requiring different formalisms. Never entangle them in a single model.

2. **Model the case, not just the process.** The case artifact — its data, evidence, and evolving state — is the primary entity. The process exists to serve the case, not the other way around.

3. **Human judgment is a feature, not a bug.** Discretionary actions, override authority, professional judgment, and exception handling are first-class concepts, not escape hatches bolted onto automation.

4. **Every action must be explainable.** Not just what happened, but why — which rule version, what data, what authority, what override rationale. Decision provenance is as fundamental as the decision itself.

5. **Design for verification and AI generation simultaneously.** The format must be formally verifiable (soundness checking) and reliably generatable by LLMs. These goals are complementary: structured, schema-validated formats serve both.

6. **Standardize guarantees, not mechanisms.** Define what durable execution means (crash recovery, persistent state, durable timers) without mandating how it's achieved (replay vs. checkpointing vs. journaling).

7. **Support both prescriptive and declarative styles.** Imperative lifecycle structure provides backbone for well-understood processes. Declarative constraints (compliance rules, eligibility conditions) provide flexibility for knowledge-intensive work. Both must coexist.

8. **Optimize for the 90% case while handling the 10%.** Common patterns (sequential approval, parallel review, SLA escalation) should be trivially expressible. Unusual patterns (dynamic scope expansion, multi-agency coordination, mid-process regulatory change) should be possible.

9. **Make conformance testable.** Every requirement in the standard must be verifiable via automated testing. If it can't be tested, it shouldn't be in the standard. Canonical fixtures make conformance concrete.

10. **Evolve gracefully.** Schema evolution, workflow definition versioning, and backward compatibility must be first-class design considerations. Long-running instances will outlive the definition version that created them.

### Ten underserved capabilities existing ecosystems handle poorly

1. **Decision provenance** — recording which rule version fired, what data was evaluated, what the override rationale was, and what AI confidence level informed the recommendation. No standard addresses this comprehensively.

2. **Temporal parameter versioning** — government rates, thresholds, and eligibility criteria change on specific dates. No workflow standard models date-effective policy parameters.

3. **Structured human override with accountability** — every existing system either prevents overrides (too rigid) or allows unconstrained overrides (no accountability). The standard needs structured override with mandatory rationale, authority verification, and audit trail.

4. **Business calendar-aware SLA computation** — "10 business days excluding federal holidays in the applicant's jurisdiction" is a routine government requirement that no standard handles natively.

5. **Case suspension with full context preservation** — suspending a case for months (pending legislation, court ruling, or investigation outcome) and resuming with complete state restoration including history states across all parallel regions.

6. **Dynamic process adaptation based on evidence** — investigation workflows that expand scope when fraud is discovered, or benefits workflows that require additional documentation based on preliminary assessment. DCR Graphs' include/exclude mechanism is the closest solution.

7. **Cross-organizational workflow coordination** — cases that span agencies with different systems, authorities, and data governance rules. BPMN's choreography diagrams modeled this but were never implemented. The problem remains unsolved.

8. **Separation of duties enforcement at the standard level** — the four-eyes principle, conflict of interest detection, and role-based constraints on who can perform which task combinations are implemented ad-hoc in every system.

9. **Defeasible regulatory rules** — government rules universally follow "general rule + exceptions" patterns. No workflow-adjacent standard handles defeasibility well. Catala and LegalRuleML offer concepts but not integration with process execution.

10. **Policy-impact simulation** — "what happens to pending cases if we change this eligibility rule?" requires linking process definitions, decision logic, case data, and simulation capabilities in a way no existing system supports.

### Ten traps to avoid

1. **The BPEL trap: coupling to a transport protocol.** BPEL's fatal dependency on WSDL/SOAP made it obsolete when REST arrived. The standard must be transport-agnostic.

2. **The BPMN trap: specification gigantism.** BPMN's 500-page spec with choreography diagrams, conversation diagrams, and three conformance classes resulted in partial implementations everywhere. Prefer a small, fully implementable core with optional extensions.

3. **The XPDL trap: interchange without execution.** A serialization format without execution semantics becomes an academic exercise. The standard must define behavior, not just structure.

4. **The XML trap: optimizing for machine parsing at the expense of human authoring.** XML-based workflow standards (BPEL, XACML, XPDL) suffer from verbosity that discourages adoption. YAML/JSON with clear naming is the modern baseline.

5. **The flowchart trap: assuming all work follows a predefined path.** High-stakes casework is fundamentally knowledge-intensive. A standard that can only model predetermined sequences will fail for investigations, complex eligibility, and exception-heavy intake.

6. **The abstraction trap: modeling theoretical elegance over practical necessity.** Process algebras and Petri net theory are valuable for analysis but must not leak into the practitioner-facing specification. Keep formal foundations in the verification layer, not the authoring surface.

7. **The completeness trap: trying to model everything.** UI rendering, document management, email notifications, and reporting are adjacent concerns that should be interfaced with, not standardized by, the workflow standard.

8. **The greenfield trap: ignoring existing systems.** Government agencies have existing case management systems. The standard must support incremental adoption, migration paths, and coexistence with legacy processes.

9. **The automation bias trap: assuming machines should decide.** In high-stakes workflows, human judgment isn't a fallback — it's the primary decision mechanism. AI assists; humans decide. The standard must make human-in-the-loop the default, not an exception.

10. **The single-paradigm trap: forcing either imperative or declarative.** Real workflows have well-structured phases (imperative) governed by flexible rules (declarative) with ad-hoc exceptions (discretionary). The standard must accommodate all three styles, coherently.

### Ten additional standards, papers, and models deserving special attention

1. **DCR Graphs** (Hildebrandt & Mukkamala, 2010) — The simplest formally grounded declarative process model. Four relations. Deployed in Danish government production. Most AI-friendly formal model discovered.

2. **Catala** (Inria) — Default logic as a first-class programming language feature for formalizing legislation. Found actual bugs in official French benefits implementation. The right approach to regulatory rules-as-code.

3. **IEEE XES (1849-2016)** — Standardized process mining event log format enabling automated conformance checking. Immediate compliance verification if the workflow standard emits compatible events.

4. **Guard-Stage-Milestone (GSM)** (Hull et al., IBM Research, 2010) — The theoretical foundation for artifact-centric process modeling. More elegant than CMMN in some respects. Essential reading for the case data model.

5. **NIEM 6.0** (NIEMOpen/OASIS) — 10,000+ standardized government data elements across Justice, Human Services, and Emergency Management domains. The vocabulary foundation for case data interoperability.

6. **Certificate Transparency / RFC 9162** with **Google Trillian** — Merkle tree-based tamper-evident logging with mathematical inclusion and consistency proofs. The right foundation for audit log integrity.

7. **HL7 FHIR's Definition→Request→Event pattern** — A generalizable resource taxonomy for any workflow domain. Task as a first-class resource. Extension mechanism for gradual standardization.

8. **OpenFisca** — Open-source tax-benefit microsimulation with temporal parameter versioning, reform modeling, and policy impact analysis. Production-deployed in France, Spain, and elsewhere.

9. **Van der Aalst's Workflow Patterns** (workflowpatterns.com, 2003-2016) — The completeness benchmark for workflow languages. 126 patterns across control, data, resources, and exceptions. Essential conformance evaluation framework.

10. **LegalRuleML 1.0** (OASIS, 2021) — Deontic operators (obligation/permission/prohibition), defeasible reasoning, temporal management, and isomorphism to legal source text. The most comprehensive formal model for regulatory constraint representation.