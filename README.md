# WOS — Workflow Orchestration Standard

**Governance for workflows where decisions affect people.**

Part of [Formspec](https://formspec.org). Open source under the [AGPL-3.0 License](../LICENSE).

---

## If you are not technical

WOS is a **written standard** for how organizations run sensitive work: who may act, which rules apply, what must be recorded when someone is denied a benefit, approved for a loan, or flagged in a fraud review. It does **not** replace your workflow engine. It tells the engine what obligations to enforce and what to write into an audit trail.

**You get:** portable rules (JSON documents you can version and inspect), deterministic lifecycle behavior when the spec is followed, and optional layers for human due process, AI oversight, and formal checks on policy logic.

**You still run:** Camunda, Temporal, KIE, Flowable, or another engine for timers, persistence, and replay. WOS binds to those engines so governance stays consistent when you change the substrate.

---

## If you are technical

WOS normatively defines kernel lifecycle, case state, actor model, provenance, impact levels, contract validation ordering, and five named extension seams. Optional tiers add workflow governance, AI integration (deontic constraints, autonomy, confidence), advanced governance (DCR-style relations, equity guardrails, SMT-backed verification), and profiles for integration (APIs, policy engines, events) and semantics (JSON-LD, SHACL, PROV-O, XES). The **Lifecycle Detail** and **Runtime** companions pin down evaluation order, compensation, parallel regions, and case-instance behavior. Everything that can be machine-checked has a JSON Schema under `schemas/`; published `$id` URIs (for example `https://wos-spec.org/schemas/kernel/1.0`) are stable.

---

## Why WOS instead of “just BPMN”?

Mature engines already solve durability, timers, and replay. They do not standardize **governance**: deontic rules on agents, structured review protocols, due process for adverse decisions, authority-ranked reasoning traces, or proof obligations on subsets of constraints. WOS defines those structures so the same governance documents can attach to different execution products.

| Concern | Owner |
|--------|--------|
| What is allowed, what is required, what is proven | WOS |
| Persistence, timers, crash recovery, replay | Camunda / Temporal / KIE / Flowable / … |
| Forms, validation, AI-assisted intake | Formspec |
| Business policy evaluation (DMN, OPA, Cedar, OpenFisca, …) | Integration Profile bridges |

Bindings (interceptors, listeners, workers) enforce WOS on each engine. Governance stays portable; execution stays vendor-specific.

---

## Architecture (layers and substrates)

Plain reading: **read upward from the bottom.** The kernel is required. Everything above it is optional. Formspec handles definitions and field logic. Your engine runs the state machine. WOS sits between policy documents and the engine, and profiles attach cross-cutting integration and semantics without new kernel extension points.

```text
PARALLEL SEAMS (cross-cutting; attach via existing kernel mechanisms)
+-----------------+------------------+
|  Integration     |  Semantic         |
|  (Arazzo, CWL,   |  (JSON-LD, SHACL, |
|   CloudEvents,   |   PROV-O, XES)    |
|   Policy Engines)|                   |
+--------+--------+--------+---------+
         v                 v

VERTICAL LAYERS (optional above Layer 0; each builds on the layer below)
+============================================================+
|  Layer 3: ADVANCED GOVERNANCE (optional)                    |
|  DCR constraint zones - Equity guardrails - SMT verification|
|  Multi-step sessions - Tool governance - Agent lifecycle    |
|  Calibration - Drift detection - Shadow mode - Circuit breaker |
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

EXECUTION SUBSTRATE (external — WOS governs; engines execute)
+------------------------------------------------------------+
|  Camunda / Temporal / Apache KIE / Flowable / custom       |
|  State persistence - Timers - Crash recovery - Replay      |
+------------------------------------------------------------+

FORMSPEC SUBSTRATE
+------------------------------------------------------------+
|  Definitions - FEL - Mapping DSL - Screener - Assist       |
|  References - Ontology - Respondent Ledger - Registry      |
+------------------------------------------------------------+

COMPANION (normative algorithm and runtime shapes)
+------------------------------------------------------------+
|  Lifecycle Detail: transition order, compensation (saga),   |
|  history states, parallel execution, SCXML mapping          |
+------------------------------------------------------------+
```

### Kernel seams (dependency direction: down only)

Higher layers attach through five named seams:

| Seam | Purpose | Typical attachers |
|------|---------|-------------------|
| `actorExtension` | Register actor types | L2 (`agent`); L3 (lifecycle on agents) |
| `contractHook` | Inject validation | L1 pipelines; L2 Formspec-as-validator; Integration policy engines |
| `provenanceLayer` | Add audit tiers | L1 Reasoning + Counterfactual; L2 Narrative |
| `lifecycleHook` | Governance on transitions | L1 due process, review, quality; L2 deontic, oversight; L3 equity, drift, circuit breaker |
| `extensions` | Escape hatch | L3 constraint zones via `x-constraintZoneRef` |

The kernel tags transitions (`review`, `determination`, `adverse-decision`, `hold`, …). Governance rules match those tags so new steps pick up existing protocols when tagged consistently.

---

## Specification inventory

**18** normative specs and **18** JSON Schemas. Each document validates against its schema. Schemas live under `schemas/` in folders that mirror the specs (`kernel/`, `governance/`, `sidecars/`, `ai/`, `advanced/`, `profiles/`, `companions/`).

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

---

## Minimal example (kernel only)

A small purchase-order approval: two human actors, three states, a guard on amount. This JSON validates against [`wos-kernel.schema.json`](schemas/kernel/wos-kernel.schema.json). It carries no governance, AI, or advanced documents—only orchestration. Add layers when the risk profile requires them.

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

---

## What each layer adds (plain terms, then detail)

### Layer 0 — Kernel: where is the case, and what happened?

The kernel defines states, events, guards, case file fields, actors, and relationships between cases. Every transition emits provenance. Two conformant processors given the same kernel document and the same ordered events yield the same transitions (deterministic lifecycle).

### Layer 1 — Governance: what protections apply?

Governance adds due process (notice, individualized reasons, appeal paths), review protocols (including dual-blind and independent-first patterns), validation pipelines with assertion gates on external data, structured audit tiers, delegation of authority, and reasoning traces for determinations—including counterfactual guidance on adverse decisions where the spec requires it.

### Layer 2 — AI integration: what constrains the agent?

Agents register with types and models. **Deontic** rules express permission, prohibition, obligation, and right with fixed enforcement ordering. Output is validated like a human submission (Formspec-as-validator). Autonomy levels, confidence thresholds, fallback chains, drift monitoring, and disclosure for assisted adverse decisions (OMB M-24-10 and EU AI Act Article 13 alignment in the spec) live here.

### Layer 3 — Advanced governance: adaptive work under constraints

Dynamic condition–response style relations govern flexible investigations (interviews, document requests, expert consults) with obligations and completion rules. Equity guardrails watch aggregate disparity without blocking individual cases. A subset of constraints can ship with **SMT** verification reports (“proven safe for all inputs” vs “inconclusive—enforce at runtime”).

### Integration Profile — talking to the outside world

Maps case data into policy engines and APIs, then maps results back. Binding types include OpenAPI-style request/response, Arazzo multi-step flows, CWL, CloudEvents, tools, callbacks, and policy-engine bridges (OPA, Cedar, XACML, …).

### Semantic Profile — meaning outside WOS JSON

JSON-LD contexts map properties to RDF. Provenance can align with PROV-O; SHACL shapes can require due-process and agent fields on high-stakes workflows; XES supports process mining.

### Lifecycle Detail companion — the evaluation recipe

The kernel declares a pure deterministic lifecycle. The companion specifies guard order, nested entry/exit, parallel region activation, per-region event routing, compensation order (pivot excluded), history-state resumption, and a bidirectional SCXML mapping for interoperability.

---

## What to adopt (composition)

| Need | Adopt |
|------|--------|
| States, actors, audit trail | Kernel |
| Human workflow with due process and review | Kernel + Governance |
| AI with constraints and confidence routing | Kernel + Governance + AI Integration |
| Adaptive case management, equity, formal verification | + Advanced Governance |
| External APIs, events, policy engines | + Integration Profile |
| Linked data, PROV-O, process mining | + Semantic Profile |
| Engine implementers need step-by-step semantics | + Lifecycle Detail |

Only the kernel is mandatory. A simple purchase order may need nothing else. Rights-impacting adjudication typically needs governance. AI attaches to the same governance structures as humans; the spec does not give AI a separate, weaker track.

---

## Differentiators (survey-backed)

WOS was designed after reviewing **50+** standards and platforms (BPMN, CMMN, SCXML, Temporal, DMN, XACML, W3C PROV, and others). [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) compares **16** platforms in depth. Ten capabilities are uncommon or absent elsewhere as a combined package:

1. **Decision provenance with authority ranking** — Rules and sources ranked (statute, regulation, policy, guideline).
2. **Temporal parameter versioning** — Thresholds and limits keyed by effective date; rules at filing time, not “whatever changed today.”
3. **Structured human override** — Rationale, authority check, and evidence on the permanent record.
4. **Five evidence-based review protocols** — Independent-first, consider-opposite, calibrated confidence display, dual-blind, unassisted (automation-bias literature reflected in the spec).
5. **Deontic constraints on agents** — LegalRuleML-inspired modalities with defined enforcement order and impact-dependent null propagation.
6. **Mandatory graceful degradation** — Agent-free operation is required where agents are used; fallback chains validated at load time.
7. **Confidence decay and cumulative error tracking** — Multi-step flows compound uncertainty; floors force human pause.
8. **Rubber-stamp detection** — Statistical signals when reviewers accept agent output without engagement.
9. **Equity guardrails** — Disparity monitoring across decision-makers, human and AI.
10. **Parallel completion policies** — Wait-all, cancel-siblings, fail-fast beyond “regions exist” alone.

---

## How documents compose

Four patterns cover every artifact:

1. **Vertical layers** — Governance, AI, and Advanced documents target a kernel by URL. Dependencies point **down** only; the kernel does not import higher concepts.
2. **Sidecars** — Separate versioned documents (policy parameters, agent config, business calendar, …) that enrich a parent without redefining the parent’s processing core.
3. **Parallel seams** — Integration and Semantic profiles attach through existing kernel mechanisms; they do not add new extension seams.
4. **Companions** — Lifecycle Detail and Runtime elaborate behavior without inventing new lifecycle concepts in the kernel JSON itself.

```text
Kernel         <-- Correspondence Metadata, Business Calendar, Notification Template
Governance     <-- Policy Parameters, Due Process Config, Assertion Library
AI Integration <-- Agent Config, Drift Monitor
Advanced       <-- Equity Config, Verification Report
```

---

## Conformance (what “green” means)

**95** JSON scenarios under [`crates/wos-conformance/tests/fixtures/`](crates/wos-conformance/tests/fixtures/) drive the reference runtime with event sequences and assert transitions, provenance, timers, compensation, deontic behavior, and related rules. Workflow payloads often load from [`fixtures/`](fixtures/) via paths declared in each scenario.

| Theme | Layer | What it exercises |
|-------|-------|-------------------|
| Purchase order | Kernel | Kernel alone — three states, two actors |
| Medicaid redetermination | Kernel | Cyclic lifecycle without a `final` state |
| Benefits adjudication | Kernel | Parallel dual-blind review |
| Case relationship appeal | Kernel | Parent/child case with `correlationKey` |
| Correspondence metadata | Kernel sidecar | Nine correspondence entry templates |
| Business calendar | Governance sidecar | Federal holidays, work week, hours |
| Notification templates | Governance sidecar | Adverse decision, hold, appeal notices |
| Benefits governance | L1 | Due process, reviews, pipelines, audit |
| Policy parameters | L1 | Temporal parameters and regulatory bindings |
| Benefits AI | L2 | Two agents, deontic rules, Formspec validator, fallback |
| Advanced governance | L3 | DCR-style zone, equity, SMT verification |
| Equity config | L3 sidecar | Categories, disparity methods, triggers |
| Verification report | L3 sidecar | SMT: two proven-safe, one inconclusive |
| Integration profile | Profile | Arazzo, CWL, CloudEvents, OPA, … |
| Semantic profile | Profile | JSON-LD, SHACL, PROV-O, XES |
| Lifecycle detail | Companion | Compensation, timers, SCXML |
| LLM authoring | Validation | Model authors valid JSON from schema alone |
| Invalid bundles | Validation | Expected schema rejections (`invalid-documents.json`) |

---

## Implementation (this repository)

Five Rust crates ([`Cargo.toml`](Cargo.toml)):

| Crate | Role | Notes |
|-------|------|--------|
| `wos-core` | Typed models, lifecycle evaluation, deontic/autonomy, provenance, contract ordering | Aligned to published schemas; semantics exercised by lint and conformance. |
| `wos-lint` | Static analysis (single document and `--project`) | [`LINT-MATRIX.md`](LINT-MATRIX.md): **196** constraints (**36** T1, **55** T2, **105** T3). **196** have test witnesses; no open matrix rule-coverage gaps remain. |
| `wos-conformance` | Dynamic scenario runner | Harness over **`wos-runtime`** / **`wos-core`**; **95** scenarios plus profile and processor-report integration tests. |
| `wos-runtime` | Orchestration seam | Persistence, queues, simulated time; Runtime Companion behavior used by conformance and embedders. |
| `wos-formspec-binding` | Formspec coprocessor (**Runtime S15**) | Prefill, response validation, Mapping DSL sync into `caseFile`. |

**Not shipped here:** Camunda 8, Temporal, and AWS Step Functions bindings; a full Integration Profile processor in runtime (schemas and fixtures exist—consumption is pending). See [`WOS-IMPLEMENTATION-STATUS.md`](WOS-IMPLEMENTATION-STATUS.md) and [`TODO.md`](TODO.md).

### Formspec coprocessor (S15)

Runtime Companion **S15** defines how a Formspec-backed task is presented, drafted, submitted, validated, mapped into case state, and recorded. **`wos-formspec-binding`** implements the protocol; **`wos-runtime`** participates in orchestration. Near-term work is harness integrity and runtime boundaries ([`TODO.md`](TODO.md)), not redefining the wire protocol.

---

## Companion documents (navigation)

| Document | Use |
|----------|-----|
| [`TODO.md`](TODO.md) | Live backlog and sequencing |
| [`WOS-IMPLEMENTATION-STATUS.md`](WOS-IMPLEMENTATION-STATUS.md) | Crate maturity and technical roadmap |
| [`LINT-MATRIX.md`](LINT-MATRIX.md) | All 196 constraints, test status, citations |
| [`WOS-FEATURE-MATRIX.md`](WOS-FEATURE-MATRIX.md) | Sixteen-way competitive comparison |
| [`enterprise-feature-gaps.md`](enterprise-feature-gaps.md) | Formspec SaaS vs enterprise vendors |
| [`enterprise-implementation-roadmap.md`](enterprise-implementation-roadmap.md) | Six-phase SaaS plan with WOS dependencies |
| [`wos-formspec-competitive-feature-matrix.xlsx`](wos-formspec-competitive-feature-matrix.xlsx) | Full spreadsheet and engine comparison |

---

## Project status

WOS is maintained by Michael Deeb as part of Formspec under AGPL-3.0. The specification is **pre-release**; there are **no** production deployments yet.

**Shipped today:** 18 specs, 18 schemas, **41** workflow samples in [`fixtures/`](fixtures/), **95** dynamic conformance scenarios, Lifecycle Detail and Runtime companions, five crates (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`), **196** tracked constraints with **196** test-backed, Runtime **S15** and CaseInstance schema landed, LLM-from-schema authoring checks on samples, seven completed semi-formal code-review rounds with findings addressed.

**Not shipped:** Production deployments, engine-specific bindings, binding-backed S15 task-submission conformance fixtures, runtime processors for Integration Profile and business-calendar sidecars (artifacts exist; wiring pending), WCAG/FedRAMP/NIST 800-53 audits, a multi-stakeholder governance body beyond the maintainer.

**If development stopped:** Your workflow JSON remains yours. Schemas are public; any team can implement the spec independently. The product is the **document**, not a hosted service.

**Licensing:** AGPL-3.0 applies to **this codebase**. Workflow JSON you author is your data. Dual licensing for AGPL-sensitive procurement is on the Formspec roadmap.
