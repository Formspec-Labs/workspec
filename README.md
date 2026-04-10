# WOS -- Workflow Orchestration Standard

**A layered governance specification for workflows where decisions affect people.**

Part of the [Formspec](https://formspec.org) project. Free and open source under the [AGPL-3.0 License](../LICENSE).

---

WOS governs how work moves through organizations — government agencies processing benefits claims, insurers adjudicating coverage, hospitals managing care protocols, lenders underwriting loans, compliance teams tracking regulatory obligations. It answers the questions that matter when decisions affect people: who acted, what rules applied, whether governance held, and what happens when something breaks.

Adopt layers based on what your workflow demands -- not what technology it uses.

## Architecture

```text
PARALLEL SEAMS (cross-cutting, attach at any layer via existing kernel mechanisms)
┌─────────────────┬──────────────────┐
│  Integration     │  Semantic         │
│  (Arazzo, CWL,   │  (JSON-LD, SHACL, │
│   CloudEvents,   │   PROV-O, XES)    │
│   Policy Engines)│                   │
└────────┬────────┴────────┬──────────┘
         ▼                 ▼

VERTICAL LAYERS (each optional, each builds on the one below)
╔═════════════════════════════════════════════════════════════╗
║  Layer 3: ADVANCED GOVERNANCE (optional)                    ║
║  DCR constraint zones · Equity guardrails · SMT verification║
║  Multi-step sessions · Tool governance · Agent lifecycle    ║
║  Calibration · Drift detection · Shadow mode · Circuit break║
╠═════════════════════════════════════════════════════════════╣
║  Layer 2: AI INTEGRATION (optional)                         ║
║  Agent registration · Deontic constraints · Autonomy levels ║
║  Confidence framework · Fallback chains · Drift detection   ║
║  Formspec-as-validator · Disclosure · Narrative provenance  ║
╠═════════════════════════════════════════════════════════════╣
║  Layer 1: WORKFLOW GOVERNANCE (optional)                    ║
║  Due process · Review protocols · Data validation pipelines ║
║  Structured audit · Quality controls · Task management      ║
║  Delegation of authority · Typed holds · Temporal parameters║
╠═════════════════════════════════════════════════════════════╣
║  Layer 0: KERNEL (required)                                 ║
║  Lifecycle topology · Case state · Actor model · Provenance ║
║  Impact level · Contract validation · Durable execution     ║
║  5 named seams · Semantic tags · Case relationships         ║
╠═════════════════════════════════════════════════════════════╣
║  FORMSPEC SUBSTRATE                                         ║
║  Definitions · FEL · Mapping DSL · Screener · Assist        ║
║  References · Ontology · Respondent Ledger · Registry       ║
╚═════════════════════════════════════════════════════════════╝

COMPANION
┌─────────────────────────────────────────────────────────────┐
│  Lifecycle Detail: transition algorithm, compensation (saga),│
│  history states, parallel execution, timers, SCXML mapping  │
└─────────────────────────────────────────────────────────────┘
```

Dependencies flow downward only. Higher layers attach through five named kernel seams:

| Seam | Purpose | Who attaches |
| ---- | ------- | ------------ |
| `actorExtension` | Register actor types | L2 registers `agent`; L3 adds lifecycle states |
| `contractHook` | Inject data validation | L1 pipelines; L2 Formspec-as-validator; Integration Profile policy engines |
| `provenanceLayer` | Add audit tiers | L1 adds Reasoning + Counterfactual; L2 adds Narrative |
| `lifecycleHook` | Attach governance to transitions | L1 due process, review, quality, delegation, holds; L2 deontic constraints, oversight, sampling; L3 equity, drift, circuit breaker |
| `extensions` | Escape hatch | L3 binds constraint zones via `x-constraintZoneRef` |

The kernel tags transitions with their nature (`review`, `determination`, `adverse-decision`, `hold`). Governance documents declare rules matching those tags. Add a new review step, tag it `review`, and existing review protocols apply automatically.

## Specification

| Layer | Spec | Schema |
| ----- | ---- | ------ |
| Kernel | [`spec.md`](specs/kernel/spec.md) | [`wos-kernel`](schemas/wos-kernel.schema.json) |
| Governance | [`workflow-governance.md`](specs/governance/workflow-governance.md) | [`wos-workflow-governance`](schemas/wos-workflow-governance.schema.json) |
| Governance sidecar | [`due-process-config.md`](specs/governance/due-process-config.md) | [`wos-due-process`](schemas/wos-due-process.schema.json) |
| Governance sidecar | [`assertion-library.md`](specs/governance/assertion-library.md) | [`wos-assertion-gate`](schemas/wos-assertion-gate.schema.json) |
| Governance sidecar | [`policy-parameters.md`](specs/governance/policy-parameters.md) | [`wos-policy-parameters`](schemas/wos-policy-parameters.schema.json) |
| AI Integration | [`ai-integration.md`](specs/ai/ai-integration.md) | [`wos-ai-integration`](schemas/wos-ai-integration.schema.json) |
| AI sidecar | [`agent-config.md`](specs/ai/agent-config.md) | [`wos-agent-config`](schemas/wos-agent-config.schema.json) |
| AI sidecar | [`drift-monitor.md`](specs/ai/drift-monitor.md) | [`wos-drift-monitor`](schemas/wos-drift-monitor.schema.json) |
| Advanced | [`advanced-governance.md`](specs/advanced/advanced-governance.md) | [`wos-advanced`](schemas/wos-advanced.schema.json) |
| Advanced sidecar | [`equity-config.md`](specs/advanced/equity-config.md) | [`wos-equity`](schemas/wos-equity.schema.json) |
| Advanced sidecar | [`verification-report.md`](specs/advanced/verification-report.md) | [`wos-verification-report`](schemas/wos-verification-report.schema.json) |
| Profile | [`integration.md`](specs/profiles/integration.md) | [`wos-integration-profile`](schemas/wos-integration-profile.schema.json) |
| Profile | [`semantic.md`](specs/profiles/semantic.md) | [`wos-semantic-profile`](schemas/wos-semantic-profile.schema.json) |
| Companion | [`lifecycle-detail.md`](specs/companions/lifecycle-detail.md) | [`wos-lifecycle-detail`](schemas/wos-lifecycle-detail.schema.json) |
| Kernel sidecar | [`correspondence-metadata.md`](specs/kernel/correspondence-metadata.md) | [`wos-correspondence-metadata`](schemas/wos-correspondence-metadata.schema.json) |

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

This validates against `wos-kernel.schema.json`. No governance, no AI, no advanced features -- just orchestration. Add layers when the workflow demands them.

## What each layer lets you do

### Layer 0: Kernel

**When a case arrives, what happens to it?**

The kernel defines the path work takes. A benefits application moves from intake to review to determination to notice. An insurance claim moves from filing to investigation to settlement. A loan application moves from submission to underwriting to closing. The kernel tracks where every case is, what data it carries, who touched it, and how cases relate to each other. Every action produces a provenance record. Two systems given the same document and the same events produce the same state transitions -- the lifecycle is deterministic.

### Layer 1: Workflow Governance

**When the stakes are high, what protections apply?**

A caseworker denies a benefits claim. An underwriter declines a loan. An insurer rejects a coverage appeal. Governance ensures the affected person receives written notice, an individualized explanation, and instructions for appeal to an independent adjudicator. A reviewer evaluates a case using dual-blind protocol -- forming an independent assessment before seeing anyone else's recommendation. Delegation of authority ensures the person signing the determination has authorization within their scope. Cases pending external verification enter typed holds with defined timeouts and escalation paths.

Data arrives from an external source — income verification for a benefits case, property appraisal for a mortgage, medical records for a coverage determination. A validation pipeline runs it through assertion gates: does the extracted value match the source document? Does the total equal the sum of components? Is the filing date within the valid window? Failed gates route to structured remediation -- retry with corrections, escalate to a supervisor, or hold pending resolution.

Every determination produces a reasoning trace: which rules applied, which evidence was consulted, which thresholds governed the decision. For adverse decisions, a counterfactual trace records what the applicant could change to qualify and confirms that protected characteristics played no part in the outcome.

### Layer 2: AI Integration

**When an AI agent assists, what keeps it honest?**

A document extraction agent reads pay stubs, medical records, or property appraisals and pulls structured data. The system validates its output against the same Formspec contract a human would submit against -- no special treatment. A prohibition prevents the agent from issuing final denials on high-stakes cases. An obligation requires every extracted value to cite its source location.

The agent operates at assistive autonomy: it recommends, a human confirms. Confidence below 0.80 triggers escalation. Failure triggers a fallback chain: retry once, then create a human task. Accuracy drift over 30 days triggers demotion to a lower autonomy level until recalibration.

Adverse decision notices disclose that an AI system assisted, consistent with OMB M-24-10 and EU AI Act Article 13 (transparency). The agent's natural-language explanation enters provenance marked non-authoritative -- model-generated explanations are systematically unfaithful to actual reasoning.

WOS addresses AI Act Articles 13 (transparency) and 14 (human oversight) through deontic constraints, autonomy levels, and disclosure requirements. Article 9 (risk management system) and Article 51 (registration) are organizational obligations outside WOS's scope -- the spec provides the technical substrate (audit trails, drift monitoring, confidence tracking) but does not replace the conformity assessment or registration processes the Act requires.

### Layer 3: Advanced Governance

**When the workflow itself must adapt, what keeps it safe?**

A fraud investigation opens — whether at a government agency, an insurance carrier, or a financial institution. The investigator interviews witnesses, requests documents, consults experts, issues subpoenas -- in any order, subject to constraints. The final report requires at least one completed interview. Every interview triggers an obligation to request supporting documents. Filing the report closes interviews and document requests. These are declarative constraints on a flexible process, modeled as DCR (Dynamic Condition Response) relations adapted from Hildebrandt and Mukkamala's work, proven at government scale in Danish central government (65-70% institutional adoption). The current integration uses DCR's five relation types as a governance overlay on kernel compound states. The interaction semantics between DCR constraints, deontic constraints (Layer 2), and statechart transitions (kernel) are an active area -- the spec defines each formalism's evaluation independently; a unified verification model covering all three remains future work.

Equity guardrails monitor outcome rates across demographic groups — approval rates by region for a benefits program, denial rates by age for an insurance product, loan rejection rates by neighborhood for a lending institution. They run asynchronously -- they never block individual cases, because aggregate disparity does not indict any single decision. When disparity exceeds a threshold, the equity officer receives an alert.

A subset of deontic constraints -- those expressible in decidable logic -- submit to formal verification by an SMT solver before the workflow runs. The report states: "the prohibition on agent final denial holds for all possible inputs" (proven safe) or "this constraint references a temporal parameter unresolvable at static time" (inconclusive -- runtime enforcement still applies). Verification covers individual constraints, not full workflow soundness -- deadlock-freedom and termination of composed DCR + statechart models require techniques beyond what the current spec addresses.

### Integration Profile

**When the workflow talks to external systems, how does the conversation work?**

A workflow checks eligibility against a policy engine — OPA for a benefits program, a proprietary rules engine for an insurance product, Cedar for a lending platform. The profile maps case data into the engine's input format, calls it, and maps the permit/deny decision back into the evaluation context. A legacy mainframe check runs a command-line tool with typed inputs and outputs. An external system sends a CloudEvents notification carrying WOS extension attributes that route it to the correct workflow instance.

### Semantic Profile

**When machines beyond WOS must understand the workflow, how does meaning travel?**

A JSON-LD context maps every WOS property to an RDF term. Provenance records become PROV-O triples -- activities, entities, agents -- queryable from any SPARQL endpoint. SHACL shapes enforce that high-stakes workflows carry due process, that every data exchange point names a contract, and that agent provenance records include required fields. Process mining tools consume XES exports derived from provenance, revealing how cases actually flowed versus how the workflow was designed.

### Lifecycle Detail Companion

**When the execution engine needs exact instructions, what does it follow?**

The kernel declares a deterministic pure function. The companion supplies the pseudocode: guard evaluation in document order, exit and entry paths through nested states via least common ancestor, atomic parallel region activation, independent event routing to concurrent regions.

When a compensable scope fails, the companion defines the saga: compensation in reverse order, the pivot step excluded, backward recovery (full reversal) or forward recovery (retry without undoing). History states resume compound states where they left off. Timers survive restarts, reset on reentry, cancel when their region is cancelled. A bidirectional SCXML mapping enables interoperability with existing statechart engines -- with honest documentation of what translates and what does not.

## Composition

| What you need | What you adopt |
| ------------- | -------------- |
| Track work through states with actors and audit trail | Kernel |
| Governed human workflows with due process and review protocols | Kernel + Governance |
| AI-assisted workflows with agent constraints and confidence routing | Kernel + Governance + AI Integration |
| Adaptive case management, equity monitoring, formal verification | + Advanced Governance |
| External system integration (APIs, policy engines, events) | + Integration Profile |
| Linked data, PROV-O export, process mining | + Semantic Profile |
| Detailed execution engine guidance | + Lifecycle Detail |

Every layer except the kernel is optional. A purchase order needs only the kernel. A rights-impacting benefits adjudication needs the kernel and governance. AI plugs into governance structures that already exist for humans -- it has no separate track.

## How the documents relate

Four patterns: vertical layers, sidecars, parallel seams, and companions. Every document is one of these.

### Vertical layers

Each layer targets the kernel by URL. Layer 1 says "I govern this workflow." Layer 2 says "I add AI to this workflow." Neither requires the other -- Layer 2 works without Layer 1 for low-stakes automation. But rights-impacting workflows effectively require Layer 1 because due process lives there.

Dependencies flow downward only. A higher layer may reference a lower layer's concepts. A lower layer never references a higher one. The kernel knows nothing of governance, AI, or advanced concepts -- it publishes seams and tags; higher layers attach through them.

### Sidecars

```text
Kernel ← Policy Parameters, Due Process Config, Assertion Library,
         Correspondence Metadata

AI Integration ← Agent Config, Drift Monitor

Advanced Governance ← Equity Config, Verification Report
```

A sidecar enriches its parent without affecting processing. Each carries its own `$wos*` marker, version, and lifecycle. All target the kernel workflow via `targetWorkflow`.

Sidecars exist for independent update cadence. The eligibility threshold changes every January -- update Policy Parameters, leave governance untouched. The agent's calibration schedule shifts -- update Agent Config, leave AI Integration untouched. Equity parameters evolve with case law -- update Equity Config, leave Advanced Governance untouched.

### Parallel seams

Parallel seams cut across layers. The Integration Profile connects workflows to external APIs and policy engines whether or not AI is involved. The Semantic Profile adds linked data interpretation to any WOS document regardless of adopted layers.

Parallel seams use existing kernel mechanisms (`invokeService`, `emitEvent`, `extensions`). They introduce no new extension points. A workflow functions without them. Adding one changes interpretation or interoperability, never processing.

### Companion

The Lifecycle Detail is neither layer nor seam. It elaborates kernel semantics without adding concepts. The kernel declares a deterministic lifecycle; the companion provides the algorithm. The kernel declares a compensation seam; the companion provides the saga.

A Kernel Structural processor validates documents without it. A Kernel Complete processor executes workflows with it.

### What depends on what

- **Kernel** depends on nothing.
- **Governance** depends on kernel (tags, seams). Independent of AI and Advanced.
- **AI Integration** depends on kernel (seams) and extends governance (review protocols, validation pipelines, audit tiers, quality controls). Adoptable without governance for low-stakes workflows.
- **Advanced Governance** depends on kernel (seams, extensions) and extends both governance and AI. Adoptable without AI for human-only advanced governance.
- **Sidecars** depend on their parent document type and the kernel.
- **Integration Profile** depends on kernel (`invokeService`, `emitEvent`, correlation keys). Independent of all layers.
- **Semantic Profile** depends on kernel (provenance structure). Independent of all layers.
- **Lifecycle Detail** depends on kernel (elaborates its semantics). Independent of all layers.

No circular dependencies. No lateral dependencies. The graph is a DAG flowing downward through layers and outward through seams.

## Conformance Fixtures

22 fixtures demonstrate every layer and composition:

| Fixture | Layer | What it proves |
| ------- | ----- | -------------- |
| Purchase order approval | Kernel | Kernel stands alone -- 3 states, 2 actors, zero governance |
| Medicaid redetermination | Kernel | Cyclical lifecycle -- periodic review without a `final` state |
| Benefits adjudication (kernel) | Kernel | Full workflow with parallel dual-blind review |
| Case relationship appeal | Kernel | Parent/child case link with correlationKey interaction |
| Benefits correspondence metadata | Kernel sidecar | 9 correspondence entry templates |
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

## Status and Governance

WOS is part of the Formspec project, maintained by Michael Deeb under AGPL-3.0. The specification is pre-release with no production deployments.

**What exists today:** 16 specs, 16 schemas, 22 conformance fixtures covering all four vertical layers, two parallel seam profiles, one companion, and one correspondence metadata sidecar. All schemas pass LLM-authoring validation (an LLM given only the schema and a one-paragraph workflow description produces valid, semantically correct documents). Seven rounds of semi-formal code review have been completed with all findings addressed.

**What does not exist yet:** Production deployments, a conformance test suite beyond fixture validation, formal accessibility or compliance audits (WCAG, FedRAMP, NIST 800-53), or an organizational governance body beyond the maintainer. The `every`/`some`/`duration` FEL functions referenced by the specs are not yet implemented in the Formspec codebase. Federation and Learning profiles are planned but not started.

**Sustainability model:** WOS documents are JSON files under your control, validated against published JSON Schemas. The specification is a public document any team can implement independently. If the project stopped today, your workflow documents remain usable by any JSON-capable system -- the spec is the product, not a service.

**Licensing:** AGPL-3.0 applies to implementation code, not to workflow documents you create. Your kernel documents, governance configurations, and sidecar files are your data. Dual licensing is on the Formspec roadmap for organizations where AGPL is a procurement blocker.
