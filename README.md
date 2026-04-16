# WOS — Workflow Orchestration Standard

**Machine-authorable governance for workflows where decisions affect people.**

Part of [Formspec](https://formspec.org). Licensed under [Apache-2.0](../LICENSE) (specs, schemas, runtime) and [BSL 1.1](../LICENSE-BSL) (studio). See [LICENSING.md](../LICENSING.md) for details.

---

## What is WOS?

WOS is a JSON-native specification for the **governance layer** of sensitive workflows — benefits adjudication, permit reviews, fraud investigations, any process where a decision affects someone's rights. It defines what protections apply, what constraints bind AI agents, what the audit trail must contain, and what the reasoning was behind each determination.

It is designed for **machine authoring and machine validation**: AI can generate WOS documents from schemas, static analysis checks 197 constraints that schemas alone cannot express, SMT solvers can prove governance properties safe for all inputs, and conformance fixtures verify runtime behavior matches the spec. This is not possible with BPMN XML, which was designed for visual editing by humans in a modeler canvas.

WOS does **not** replace your workflow engine. It targets Camunda, Temporal, Flowable, KIE, and Step Functions as execution substrates. The engine handles persistence, timers, crash recovery, and the full orchestration vocabulary. WOS governs the transitions that matter for rights, audit, and AI oversight.

---

## Why not extend BPMN?

BPMN is a visual process modeling language with an XML serialization. It excels at orchestration — who does what, in what order, under what conditions. WOS borrows orchestration concepts from BPMN, SCXML, WS-HumanTask, DCR Graphs, and Temporal's deterministic replay. The lifecycle model is not the invention.

The invention is the **governance semantics** — things no standard or platform currently covers:

1. **Deontic constraints on agents** — permitted, prohibited, obligated, right with fixed enforcement ordering and impact-dependent null propagation. No engine or standard implements this.
2. **Structured oversight as behavioral specification** — "independent-first" review means the system must suppress AI suggestions until the human has independently assessed. BPMN swim lanes say *who* does work, not *how* they must review it.
3. **Due process as a structural requirement** — every rights-impacting workflow must include appeal paths, adverse decision notice, and counterfactual explanation. BPMN validation checks diagram well-formedness, not whether you forgot the appeal path.
4. **Epistemic status separation in provenance** — verified facts, AI-generated narrative, and counterfactual analysis are recorded as distinct tiers. Every engine logs who/what/when. None separates "this is a verified fact" from "this is what the AI suggested" from "this is what would have changed the outcome."
5. **Authority-ranked reasoning traces** — statute > regulation > policy > guideline, recorded with every decision. No engine or standard ranks the authority of the rules that drove a decision.
6. **Impact-level-dependent behavior** — the same null value, the same fallback, the same autonomy cap behaves differently depending on whether the workflow is rights-impacting vs operational. No engine scopes behavior by impact classification.

These cannot be bolted onto BPMN as extensions. They require a different document type — which is why WOS exists as its own spec rather than a BPMN profile.

---

## Why JSON-native?

Three properties that BPMN XML cannot provide:

- **AI can generate it** — typed patch operations at the AST level, schema-constrained generation, conformance tests validating LLM-from-schema authoring. BPMN's visual-first representation doesn't support structured generation.
- **Tools can validate it** — JSON Schema for structural correctness, SHACL shapes for governance policy correctness, SPARQL for cross-workflow analysis, SMT solving for formal governance proofs. XML Schema validates structure, not governance semantics.
- **It's simultaneously linked data** — every WOS document is valid JSON, valid JSON-LD, and an RDF graph without transformation. BPMN requires a semantic lifting pipeline (RML mappings, ontology alignment) to achieve the same.

---

## A concrete example

A purchase-order approval: two people, three states, a guard on the dollar amount. This validates against the kernel schema and carries no governance beyond basic orchestration:

```json
{
  "$wosKernel": "1.0",
  "url": "https://agency.gov/workflows/purchase-order-approval",
  "version": "1.0.0",
  "status": "active",
  "impactLevel": "operational",
  "actors": [
    { "id": "requester", "type": "human", "description": "Submits purchase requests" },
    { "id": "approver", "type": "human", "description": "Reviews and approves requests" }
  ],
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

Add governance, AI constraints, or equity rules as separate layered documents when the risk profile requires them.

---

## How the layers work

WOS has one required layer and three optional ones. Each builds on the one below it. Two cross-cutting profiles and two companions attach without new kernel extension points.

### Layer 0 — Kernel (required)

States, transitions, guards, case data, actors, relationships. Every transition emits provenance. Two conformant processors given the same kernel and the same events produce the same result. The lifecycle model draws from SCXML (nested states, history states), WS-HumanTask (actor roles), and Temporal (deterministic replay).

### Layer 1 — Governance (optional)

Due process for adverse decisions, five structured review protocols (independent-first, consider-opposite, calibrated confidence, dual-blind, unassisted), validation pipelines, delegation of authority with legal instrument references, hold policies, and authority-ranked reasoning traces. This layer is where most of the genuine invention lives.

### Layer 2 — AI Integration (optional)

Agent registration with deontic constraints, autonomy levels capped by impact classification, confidence thresholds with decay across multi-step flows, mandatory fallback chains that terminate in human review, drift monitoring, and disclosure requirements for AI-assisted decisions (EU AI Act Article 13, OMB M-24-10 alignment).

### Layer 3 — Advanced Governance (optional)

DCR-style constraint zones for flexible investigation work (drawn from DCR Graphs — condition/response/include/exclude/milestone relations with three-state marking). Equity guardrails monitoring aggregate disparity without blocking individual cases. SMT verification reports proving governance constraints safe for all inputs.

### Cross-cutting profiles

- **Integration Profile** — Connects case data to external APIs (OpenAPI, Arazzo), event systems (CloudEvents), policy engines (OPA, Cedar), and tools. Maps data out, maps results back.
- **Semantic Profile** — JSON-LD contexts, PROV-O provenance alignment, SHACL governance shapes, XES process mining export.

### Companions

- **Lifecycle Detail** — Evaluation order, nested entry/exit, parallel region activation, compensation (saga), history-state resumption, and a bidirectional SCXML mapping.
- **Runtime** — Case instance serialization, event delivery contract, and the Formspec coprocessor handoff (how form submissions become case data).

---

## What to adopt

Only the kernel is mandatory. Add layers as the risk profile demands.

| Workflow type | Layers needed |
|--------------|---------------|
| Simple approval, low risk | Kernel |
| Human workflow with review and appeal | Kernel + Governance |
| AI-assisted decisions | Kernel + Governance + AI Integration |
| Adaptive case management, equity monitoring | + Advanced Governance |
| External APIs, policy engines, events | + Integration Profile |
| Linked data, PROV-O, process mining | + Semantic Profile |

The spec does not give AI a separate, weaker track — agents participate under the same governance structures as humans.

---

## Specification inventory

18 normative specs, 18 JSON schemas. Each document validates against its schema. Schemas live under `schemas/` in folders that mirror the spec structure.

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
| Advanced sidecar | [`equity-config.md`](specs/advanced/equity-config.md) | [`wos-equity`](schemas/advanced/equity-config.schema.json) |
| Advanced sidecar | [`verification-report.md`](specs/advanced/verification-report.md) | [`wos-verification-report`](schemas/advanced/wos-verification-report.schema.json) |
| Profile | [`integration.md`](specs/profiles/integration.md) | [`wos-integration-profile`](schemas/profiles/wos-integration-profile.schema.json) |
| Profile | [`semantic.md`](specs/profiles/semantic.md) | [`wos-semantic-profile`](schemas/profiles/wos-semantic-profile.schema.json) |
| Companion | [`lifecycle-detail.md`](specs/companions/lifecycle-detail.md) | [`wos-lifecycle-detail`](schemas/companions/wos-lifecycle-detail.schema.json) |
| Runtime | [`runtime.md`](specs/companions/runtime.md) | [`wos-case-instance`](schemas/companions/wos-case-instance.schema.json) |

---

## Reference implementation

Five Rust crates in this repository:

| Crate | What it does |
|-------|-------------|
| [`wos-core`](crates/wos-core/) | Typed models, lifecycle evaluation, deontic rules, provenance, contract ordering |
| [`wos-lint`](crates/wos-lint/) | Static analysis — 197 constraints across three tiers, all with test witnesses |
| [`wos-conformance`](crates/wos-conformance/) | Dynamic scenario runner — 146 JSON test fixtures that drive the runtime and assert correct behavior |
| [`wos-runtime`](crates/wos-runtime/) | Orchestration layer — persistence, queues, simulated time, milestone evaluation |
| [`wos-formspec-binding`](crates/wos-formspec-binding/) | Formspec coprocessor — prefill, response validation, mapping form data into case state |

### Running the tests

This tree is normally checked out as `formspec/wos-spec` inside the [Formspec](https://github.com/Formspec-org/formspec) repository. The workspace depends on `fel-core` at `../crates/fel-core`.

From `formspec/wos-spec`:

```bash
cargo test -p wos-core
cargo test -p wos-runtime
cargo test -p wos-conformance
```

---

## Intellectual ancestry

WOS did not invent lifecycle modeling. It combines proven concepts with novel governance semantics:

| Concept | Ancestor | What WOS adds |
|---------|----------|---------------|
| Nested states, history states, parallel regions | SCXML / Harel statecharts | Governance hooks on every transition, provenance emission |
| Actor roles, task lifecycle | WS-HumanTask | Impact-level-dependent behavior, structured override with authority ranking |
| Deterministic replay, durable execution | Temporal | Governance-constrained replay — deontic violations are replayed, not just state transitions |
| Condition/response relations, milestone marking | DCR Graphs | Constraint zones inside a statechart, with SMT satisfiability verification |
| Task assignment, SLA, escalation | BPMN / every engine | Breach policies tied to impact level, authority-ranked reasoning traces on escalation |
| Deontic constraints, structured oversight, due process, epistemic provenance | **WOS** | No prior standard or platform covers these. This is the invention. |

---

## Companion documents

| Document | What it covers |
|----------|---------------|
| [`TODO.md`](TODO.md) | Live backlog and sequencing |
| [`WOS-IMPLEMENTATION-STATUS.md`](WOS-IMPLEMENTATION-STATUS.md) | Crate maturity and technical roadmap |
| [`LINT-MATRIX.md`](LINT-MATRIX.md) | All 197 constraints with test status and citations |
| [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) | Sixteen-way competitive comparison |
| [`enterprise-feature-gaps.md`](enterprise-feature-gaps.md) | Formspec SaaS gaps vs enterprise vendors |
| [`enterprise-implementation-roadmap.md`](enterprise-implementation-roadmap.md) | Six-phase SaaS plan |
| [`wos-formspec-competitive-feature-matrix.xlsx`](wos-formspec-competitive-feature-matrix.xlsx) | Full spreadsheet comparison |

---

## Project status

WOS is maintained by Michael Deeb as part of Formspec under Apache-2.0 / BSL 1.1. The specification is **pre-release**; there are no production deployments yet.

**Shipped:** 18 specs, 18 schemas, 41 workflow samples, 146 dynamic conformance scenarios (all green), 197 lint constraints (all with test witnesses), five Rust crates, the Runtime S15 coprocessor protocol, and seven completed code-review rounds.

**Not shipped:** Production deployments, engine-specific bindings (Camunda, Temporal, Step Functions), runtime processors for the Integration Profile sidecars, WCAG/FedRAMP/NIST audits, a formal governance body beyond the maintainer.

**If development stopped:** Your workflow JSON is yours. The schemas are public. Any team can implement the spec independently. The product is the document, not a hosted service.

**Licensing:** Apache-2.0 applies to specs, schemas, and runtime crates. BSL 1.1 applies to the studio (authoring tooling), converting to Apache-2.0 in April 2030. Workflow JSON you author is your data. See [LICENSING.md](../LICENSING.md) for details.
