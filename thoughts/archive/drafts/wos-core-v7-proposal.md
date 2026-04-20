# WOS v7 Change Proposal: Profile-Based Architecture with Minimal Kernel

**Author:** Mike (TealWolf Consulting LLC)
**Date:** 9 April 2026
**Status:** Proposal for Working Draft
**Applies to:** WOS Core Specification v6.0.0

---

## 1. Problem Statement

WOS v6 is designed around the threat model of a general-purpose LLM making or influencing high-stakes determinations. The governance machinery — deontic constraints, SMT verification, four-layer audit, structured oversight protocols — is calibrated to the scenario where an agent touches the decision surface.

In practice, AI agents participate in workflows across a wide spectrum. At one end, agents perform preparation work — extracting data, checking completeness, routing cases, pre-populating forms — upstream of any human determination. At the other end, agents influence or make consequential decisions requiring full governance. Most deployments begin at the preparation end.

The current spec applies uniform governance regardless of where on this spectrum the agent operates. This makes the governance cost disproportionate to the risk, prices out the safest use cases, and undermines the spec's stated goal of incremental adoption.

This proposal restructures WOS around a **minimal kernel** with **two independently adoptable profiles** — a Preparation Profile for agents that touch information, and a Governance Profile for agents that touch outcomes. Both are optional. Both compose onto the same kernel. Both compose with each other.

---

## 2. Core Architectural Principle

**The kernel orchestrates. Profiles govern. Contracts validate.**

The WOS kernel knows that work flows through states, data accumulates, agents participate, and everything gets logged. It is agnostic about *what* agents do. It does not impose governance semantics — no autonomy levels, no deontic constraints, no oversight protocols. It provides the substrate onto which governance profiles compose.

Two profiles define the governance semantics for different modes of agent participation:

- **Preparation Profile** — for agents that touch information. Grounded pipelines, assertion gates, Formspec-as-validator, agent provenance annotations. Agents prepare; contracts validate; humans decide.
- **Governance Profile** — for agents that touch outcomes. Deontic constraints, structured oversight, autonomy enforcement, SMT verification, dual-readability narratives, due process machinery. The full v6 governance envelope, available when needed.

Both are optional. Both compose onto the same kernel. Both compose with each other — a workflow can have a grounded pipeline (Preparation Profile) feeding into a governed determination step (Governance Profile), with the contract boundary between them being a Formspec definition.

---

## 3. Design Principle: Verifiability as the Automation Boundary

The decision of whether to automate a task is governed by a single test:

> **Can a second, independent system cheaply verify whether the agent performed this task correctly?**

- **Yes → automate aggressively.** Extraction (does the value appear in the source?), completeness checking (is the field present?), arithmetic (do the numbers add up?), format validation (does this parse as a date?), classification against known categories.
- **No → this is a judgment task.** If verifying the output requires the same expertise as producing it, the task is a determination wearing a prep-work costume. It stays with the human.

This principle replaces the current autonomy-level framework as the primary design heuristic for task assignment.

---

## 4. Proposed Architecture

### 4.1 The Minimal Kernel

The kernel is everything that's true regardless of how agents participate. It contains no governance semantics — no autonomy levels, no deontic constraints, no oversight protocols. It is the substrate.

**Kernel responsibilities:**

- **Lifecycle topology.** States, transitions, events, constraint zones. How work progresses.
- **Case state.** Typed data container with immutable mutation history. What data accumulates.
- **Provenance (Layer 1 only).** Who did what, when, with what version, with what inputs and outputs. The immutable factual record.
- **Contract validation.** Formspec definition in, Formspec definition out, pass or fail. The universal interface boundary.
- **Agent registration.** Agent id, version, type (deterministic / statistical / generative). No governance semantics — just identity and classification.
- **Durable execution.** Crash recovery, persistent state, deterministic replay, durable timers.

**What the kernel does NOT include:**

- Autonomy levels, deontic constraints, structured oversight, confidence routing, fallback chains (→ Governance Profile)
- Grounded pipelines, assertion gates, preparation task catalog, decision drift detection (→ Preparation Profile)
- Four-layer provenance, dual-readability narratives, counterfactual requirements (→ Governance Profile)

An implementation that supports only the kernel can orchestrate workflows with agent participation, validate contracts, and produce a complete audit trail — but it imposes no opinion on what agents are allowed to do.

### 4.2 Agent Type Taxonomy (Kernel)

The kernel defines agent types as a classification property. Profiles use this classification to determine applicable governance.

| Type | Characteristics | Kernel Responsibility |
|------|----------------|----------------------|
| `deterministic` | Rules engines, decision tables, lookup systems. Output is a pure function of input. | Registration, contract validation, provenance. |
| `statistical` | Classical ML. Fixed model, bounded output space, calibratable confidence. | Registration, contract validation, provenance. |
| `generative` | LLMs. Unbounded output space, non-deterministic, injection-vulnerable. | Registration, contract validation, provenance. |

The kernel treats all three identically. Profiles differentiate.

---

### 4.3 Preparation Profile (Optional)

**Adopters:** Agencies deploying agents for document processing, intake triage, form pre-population, completeness checking, case summarization, routing, and other preparation tasks where agents touch information but not outcomes.

**Core principle:** Agents prepare. Contracts validate. Humans decide.

#### 4.3.1 Grounded Pipelines

A `pipeline` is a staged processing chain where each stage constrains the degrees of freedom of subsequent stages. It is the primary orchestration construct for preparation work.

```yaml
pipeline:
  id: "income-verification"
  stages:
    - id: "extract"
      agentRef: "urn:wos:example.gov:agents:doc-extractor:2.0.0"
      outputContract: "urn:formspec:example.gov:income-extraction:1.0.0"
      
    - id: "validate"
      type: "contract-assertion"
      assertions:
        - type: "source-grounded"
          description: "Extracted values must appear in source document"
          fields: ["applicantIncome", "filingDate", "dependentCount"]
        - type: "arithmetic"
          expression: "$totalIncome = $wageIncome + $investmentIncome + $otherIncome"
        - type: "range"
          field: "filingDate"
          expression: "$filingDate >= @fiscalYearStart and $filingDate <= @fiscalYearEnd"
      onFailure: "flag-for-human"
      
    - id: "enrich"
      agentRef: "urn:wos:example.gov:agents:case-summarizer:1.0.0"
      inputConstraint: "stage:extract outputs only"
      outputContract: "urn:formspec:example.gov:case-summary:1.0.0"
      
    - id: "verify"
      type: "contract-assertion"
      assertions:
        - type: "consistency"
          description: "Summary must not contradict extracted facts"
          referenceStage: "extract"
          fields: ["applicantIncome", "eligibilityCategory"]
      onFailure: "flag-for-human"
      
    - id: "present"
      type: "human-review"
      activityRef: "urn:wos:example.gov:activities:eligibility-review:1.0.0"
      showProvenance: true
      highlightAgentTouched: true
```

Key semantics:

- Each stage's output is validated against a Formspec contract before the next stage executes.
- `contract-assertion` stages are deterministic validation gates derived from the contract definition, not trained models. They do not drift and do not require maintenance.
- `inputConstraint` restricts what data a generative agent can see, reducing injection surface.
- The pipeline's risk profile is determined by the weakest validation gate, not the most powerful model.
- Pipeline provenance is compositional: each stage records its inputs, outputs, and validation results. Confidence is a chain, not a single score.

#### 4.3.2 Formspec-as-Validator

**LLM output is untrusted input validated against the same Formspec contract a human would submit against.**

- A Formspec Definition used as an agent output contract MUST apply the same validation rules, bind expressions, and constraint shapes as the human-facing form.
- Validation failures on agent output are recorded as provenance and trigger fallback (flag for human review), not silent acceptance.
- Agent-touched fields MUST be annotated in the Formspec instance with `agentProvenance` metadata: which agent, which model version, what confidence, what source material. This annotation is surfaced in the UI, not buried in an audit log.

The deterministic grounding layer is not a separate classical ML pipeline. It is the Formspec contract itself, acting as a validation cage around agent output. This eliminates the maintenance burden of custom classical models while preserving structural verifiability.

#### 4.3.3 Preparation Task Catalog

A normative catalog of preparation task patterns with verifiability characteristics:

| Task Pattern | Verifiable? | Verification Method | Recommended Agent Type |
|-------------|-------------|--------------------|-----------------------|
| Field extraction from document | Yes | Source-grounding: value appears in source | `generative` with assertion gate |
| Completeness checking | Yes | Field presence/absence against schema | `deterministic` or `generative` |
| Document classification | Yes | Keyword/heuristic sanity check | `statistical` or `generative` |
| Arithmetic validation | Yes | Recomputation | `deterministic` |
| Date/format validation | Yes | Parse check | `deterministic` |
| Priority scoring | Partially | Range check, outlier detection | `statistical` |
| Case summarization | Partially | Consistency check against extracted facts | `generative` with assertion gate |
| Sufficiency judgment | No | Requires domain expertise | **Human task** |
| Credibility assessment | No | Requires domain expertise | **Human task** |

Tasks in the "No" rows are determinations. The Preparation Profile SHOULD flag any attempt to assign them to an agent without the Governance Profile being active.

#### 4.3.4 Provenance (Preparation Profile)

For preparation tasks validated by assertion gates, provenance is simplified:

- **Layer 1 (Facts)** is authoritative and complete: extraction source, extracted values, assertion results, model version.
- **Layer 2 (Assertion Trace)** is the validation record: which checks passed, which failed, what the deterministic validators found.
- **Layer 3 (Generated Narrative)** is genuinely optional.
- **Layer 4 (Counterfactual)** is not applicable.

#### 4.3.5 Decision Drift Detection

Structural guards against the pattern where preparation tasks gradually absorb determination logic:

- If a model's training data includes human determination outcomes (approvals, denials), the model is optimizing for outcome prediction, not task accuracy. This MUST be disclosed and triggers reclassification — the Governance Profile becomes required.
- Self-tuning models MUST declare their optimization objective. "Maximize extraction accuracy against ground truth" is preparation. "Maximize agreement with reviewer decisions" is determination.
- Periodic review of preparation-task agents SHOULD assess whether their outputs are being ratified without genuine human engagement (the Vaccaro rubber-stamp pattern).

---

### 4.4 Governance Profile (Optional)

**Adopters:** Agencies deploying agents that influence, recommend, or make consequential decisions — benefits adjudication, licensing determinations, compliance findings, risk assessments.

**Core principle:** Constraints are external to the agent. The agent is outside the trust boundary. Human authority is supreme.

The Governance Profile contains the full v6 agent governance machinery, relocated from the kernel:

#### 4.4.1 Autonomy Levels

| Level | Semantics |
|-------|-----------|
| `autonomous` | Output committed without human review. REQUIRES deontic constraints. PROHIBITED for `rights-impacting`/`safety-impacting` unless elevated with attestation. |
| `supervisory` | Provisionally committed. Human reviews within `reviewWindow`. |
| `assistive` | Recommendation only. Human reviews, modifies, confirms. |
| `manual` | Human performs. Agent assists on demand only. |

#### 4.4.2 Deontic Constraint Framework

Permissions, Prohibitions, Obligations, Rights — evaluated after Formspec contract validation and before commit. SHACL equivalence for every constraint. The full v6 framework (§9.4).

#### 4.4.3 Verifiable Constraint Subset

SMT-provable governance constraints within the decidable FEL fragment. The full v6 framework (Appendix F). Optional within the Governance Profile itself — available as an advanced capability.

#### 4.4.4 Behavioral Attestations

Independently issued evaluations verifying agent behavioral characteristics. The full v6 framework (§9.6).

#### 4.4.5 Structured Oversight Protocols

`independentFirst`, `considerOpposite`, `calibratedConfidence`, `dualBlind`, `unassisted`. The full v6 framework (§8.3).

#### 4.4.6 Provenance (Governance Profile)

The full four-layer audit architecture:

- **Layer 1 (Immutable Facts)** — authoritative.
- **Layer 2 (Structured Reasoning)** — authoritative for deterministic logic. Dual-readability narrative for adverse decisions in `rights-impacting` workflows.
- **Layer 3 (Generated Narrative)** — informational only. Non-authoritative.
- **Layer 4 (Counterfactual)** — required for adverse decisions in `rights-impacting` workflows.

#### 4.4.7 Due Process

Notice, explanation levels, appeal mechanisms, agent disclosure, continuation-of-service states. The full v6 framework (§16).

#### 4.4.8 Confidence Framework and Fallback Chains

ConfidenceReport, calibration status, confidence-based routing. Fallback chains terminating in `escalateToHuman` or `fail`. The full v6 framework (§9.7–9.8).

---

### 4.5 Profile Composition

The two profiles compose naturally because they share the kernel's contract boundary:

```
┌─────────────────────────────────────────────────┐
│           Future Profiles (conceptual)           │
│  ┌──────────────┐ ┌─────────┐ ┌──────────────┐  │
│  │ Coordination │ │Learning │ │  Federation  │  │
│  └──────────────┘ └─────────┘ └──────────────┘  │
├─────────────────────────────────────────────────┤
│              Governance Profile                  │
│  Deontic constraints · Oversight · Due process   │
│  Autonomy levels · Attestations · 4-layer audit  │
├─────────────────────────────────────────────────┤
│             Preparation Profile                  │
│  Grounded pipelines · Assertion gates            │
│  Formspec-as-validator · Drift detection         │
├─────────────────────────────────────────────────┤
│                   Kernel                         │
│  Lifecycle · Case state · Provenance (L1)        │
│  Contract validation · Agent registration        │
│  Durable execution                               │
├─────────────────────────────────────────────────┤
│  Formspec · FEL · JSON-LD / RDF Foundation       │
└─────────────────────────────────────────────────┘
```

**Kernel only.** Workflows with agents, contract validation, and basic audit. No governance opinion.

**Kernel + Preparation.** Document processing, intake triage, form pre-population. Agents prepare; contracts validate; humans decide. The adoption beachhead.

**Kernel + Governance.** AI-assisted adjudication with full governance. The v6 use case, now explicitly scoped.

**Kernel + Preparation + Governance.** A grounded pipeline feeds into a governed determination step. The preparation stages use assertion gates; the determination step uses deontic constraints and structured oversight. The contract boundary between them is a Formspec definition. This is the mature deployment model.

### 4.6 Future Profiles (Conceptual)

The kernel-plus-profiles architecture is designed for extensibility. The following profiles are anticipated but not yet specified. Each composes onto the kernel independently and may compose with the Preparation and Governance profiles.

#### 4.6.1 Coordination Profile

**Problem:** As agent deployments mature, workflows will involve multiple agents collaborating — one extracts, another validates, a third summarizes, a fourth cross-references against external data. The current spec treats each agent invocation as independent. It has no concept of agents communicating with each other, negotiating task boundaries, or coordinating on shared state.

**Scope:**

- **Multi-agent pipeline governance.** Extends the Preparation Profile's grounded pipelines to scenarios where stages are performed by different agents that must coordinate — not just execute sequentially. Defines handoff contracts between agents, not just between agents and the kernel.
- **Delegation chains.** When an agent determines it needs a capability it lacks, it may request delegation to another agent. The Coordination Profile governs delegation: who can delegate to whom, with what authority ceiling, to what depth. Bounded delegation depth is declared in the workflow definition, not discovered at runtime.
- **Agent-to-agent communication governance.** Aligns with the A2A protocol (already referenced in v6 §11.3) but adds governance: agents communicate through the kernel, not directly. The kernel mediates, logs, and enforces contracts on inter-agent messages. No agent-to-agent channel bypasses the provenance stream.
- **Shared state protocols.** When multiple agents operate on the same case file concurrently (e.g., parallel extraction of different document sections), the Coordination Profile defines conflict resolution: last-write-wins, merge strategies, or escalation to human resolution.
- **Consensus patterns.** For tasks where multiple agents independently produce outputs (e.g., three classifiers vote on document type), the profile defines consensus rules: majority, unanimous, weighted by confidence, or escalate on disagreement.

**Composition:** Coordination Profile + Preparation Profile enables multi-agent document processing pipelines. Coordination Profile + Governance Profile enables supervised multi-agent adjudication where delegation authority is constrained by deontic rules.

#### 4.6.2 Learning Profile

**Problem:** The Preparation Profile's decision drift detection (§4.3.5) addresses a specific failure mode — preparation tasks absorbing determination logic. But the broader challenge of governing models that learn, adapt, or self-tune over time requires a dedicated framework. A model that improves its extraction accuracy against ground truth is safe. A model that begins predicting reviewer decisions is not. The boundary between these is subtle, continuous, and requires ongoing monitoring rather than one-time classification.

**Scope:**

- **Optimization objective declaration.** Every self-tuning or continuously trained model MUST declare its optimization objective in machine-readable form. "Minimize character error rate on field extraction" is a preparation objective. "Maximize agreement with final case disposition" is a determination objective. The Learning Profile validates that the declared objective is consistent with the agent's assigned role (preparation vs. governance).
- **Training data provenance.** When a model is retrained, the Learning Profile records what data it was trained on, what version of ground truth was used, and whether the training data included human determination outcomes. Training on outcomes triggers reclassification review.
- **Performance baseline and drift monitoring.** The profile defines a baseline evaluation protocol: a held-out test set evaluated at each model version. Performance metrics are recorded as provenance. Drift beyond declared thresholds triggers alerts, autonomy reduction, or fallback to previous version.
- **A/B deployment governance.** When a new model version is deployed alongside the current version, the Learning Profile governs the comparison: what fraction of cases see the new version, how disagreements are resolved, what metrics determine promotion. Shadow deployment (new model runs but output is not used) is distinguished from canary deployment (new model output is used for a fraction of cases).
- **Feedback loop governance.** The most dangerous pattern: human reviewers correct agent output, corrections become training data, model learns to predict corrections, human reviewers trust model more, corrections decrease, model begins driving decisions. The Learning Profile requires that feedback loops be declared, monitored for convergence, and interrupted if reviewer engagement metrics (time spent, override rate, independent assessment variance) decline below thresholds.
- **Model retirement.** When a model version is deprecated, the Learning Profile records the retirement event, the replacement version, and the migration of in-flight cases. Provenance records retain the original model version for auditability.

**Composition:** Learning Profile + Preparation Profile is the primary composition — governing models that improve at preparation tasks. Learning Profile + Governance Profile governs models whose determination-influencing behavior evolves. Learning Profile + Coordination Profile governs scenarios where model updates affect multi-agent coordination (e.g., a retrained extractor produces different output structure, breaking downstream agents).

#### 4.6.3 Federation Profile

**Problem:** Government workflows frequently span organizational boundaries. A benefits application may require income verification from a tax authority, identity verification from a separate agency, and eligibility determination by a third. Each organization has its own agents, its own governance posture, its own trust boundaries, and its own data sovereignty requirements. The current spec assumes a single organizational context.

**Scope:**

- **Cross-agency workflow coordination.** A federated workflow is a composition of sub-workflows, each owned and operated by a different organization. The Federation Profile defines the contract between organizations: what data is shared, what provenance is exchanged, what governance guarantees each party provides. Inspired by Estonia's X-Road data exchange layer (already referenced in v6 Appendix I).
- **Trust boundary management.** Each organization defines its trust boundary. Agents from Organization A are outside Organization B's trust boundary by default. The Federation Profile defines trust elevation mechanisms: attestation exchange, governance profile compatibility verification, and bilateral trust agreements recorded as provenance.
- **Data sovereignty.** Case file data may be subject to different jurisdictional requirements. The Federation Profile defines data residency constraints (this field must not leave this jurisdiction), data minimization rules (share only what the receiving organization's contract requires), and purpose limitation (shared data may only be used for the declared purpose). These constraints are expressed as FEL expressions on the integration contract.
- **Provenance chain continuity.** When a case crosses organizational boundaries, provenance must be continuous. The Federation Profile defines provenance exchange formats: what the sending organization shares (Layer 1 facts, contract validation results), what it withholds (internal deliberation, agent implementation details), and how the receiving organization incorporates external provenance into its own chain. Merkle tree hash chaining extends across organizational boundaries with signed cross-references.
- **Federated attestation.** An agent certified by Organization A's evaluation board may not be recognized by Organization B. The Federation Profile defines attestation interoperability: mutual recognition agreements, attestation equivalence mappings, and the fallback when no mutual recognition exists (the agent operates at `manual` autonomy in the receiving organization).
- **Conflict resolution.** When two organizations' governance profiles conflict (Organization A permits autonomous agent classification, Organization B requires human review of all classifications), the Federation Profile defines resolution: the more restrictive policy wins by default, with explicit bilateral override available.

**Composition:** Federation Profile + Preparation Profile enables cross-agency document processing (e.g., one agency extracts, another validates against its records). Federation Profile + Governance Profile enables cross-agency adjudication with coordinated due process. Federation Profile + Coordination Profile enables multi-organization agent ecosystems with delegated authority across trust boundaries.

#### 4.6.4 Profile Composition Matrix

The full composition space, including future profiles:

| Composition | Use Case |
|-------------|----------|
| Kernel only | Basic workflow orchestration with agents, contract validation, audit trail |
| + Preparation | Document processing, intake triage, form pre-population |
| + Governance | AI-assisted adjudication with full governance |
| + Preparation + Governance | Grounded pipelines feeding governed determinations |
| + Coordination | Multi-agent collaboration within a single organization |
| + Learning | Continuously improving models with drift governance |
| + Federation | Cross-agency workflow coordination |
| + Preparation + Learning | Self-tuning document processing with feedback loop governance |
| + Preparation + Coordination | Multi-agent document processing pipelines |
| + Governance + Federation | Cross-agency adjudication with coordinated due process |
| + All | Mature multi-organization deployment with full governance, multi-agent coordination, and learning governance |

Each row is a valid deployment configuration. No profile requires any other profile except where explicitly stated. The kernel is always required.

---

## 5. What Moves from Kernel to Profiles

The following v6 kernel constructs are relocated:

| v6 Location | Construct | v7 Location |
|-------------|-----------|-------------|
| Kernel (§9) | Autonomy levels | Governance Profile |
| Kernel (§9) | Deontic constraint framework | Governance Profile |
| Kernel (§9) | Verifiable constraint subset | Governance Profile (optional within) |
| Kernel (§9) | Behavioral attestations | Governance Profile |
| Kernel (§9) | Confidence framework & fallback chains | Governance Profile |
| Kernel (§9) | CaMeL isolation pattern | Subsumed by Preparation Profile grounded pipelines |
| Kernel (§8) | Structured oversight protocols | Governance Profile |
| Kernel (§12) | Four-layer provenance (Layers 2–4) | Governance Profile |
| Kernel (§12) | Dual-readability narrative | Governance Profile |
| Kernel (§16) | Due process requirements | Governance Profile |
| New | Grounded pipelines | Preparation Profile |
| New | Assertion gates | Preparation Profile |
| New | Formspec-as-validator pattern | Preparation Profile |
| New | Preparation task catalog | Preparation Profile |
| New | Decision drift detection | Preparation Profile |

The kernel retains: lifecycle topology, case state, Layer 1 provenance, contract validation, agent registration (with type taxonomy), durable execution, the actor model, FEL, JSON-LD serialization, SHACL structural validation, versioning, extensibility.

---

## 6. What This Elevates

- **Formspec-as-validator** becomes the central architectural insight: the form contract is the deterministic cage.
- **Grounded pipelines** become a first-class orchestration construct, not an implementation detail.
- **Agent type taxonomy** becomes a kernel-level classification that profiles use to determine applicable governance.
- **Verifiability test** becomes the normative design principle for task assignment within the Preparation Profile.
- **Profile composition** becomes the primary adoption and scaling model — start with what you need, add governance as agent responsibilities increase.
- **Decision drift detection** addresses the most dangerous failure mode — not an agent making a bad decision, but an agent silently becoming the decision-maker through accumulated trust.

---

## 7. Migration Path from v6

This proposal is backward-compatible with v6 documents:

- Existing workflows implicitly adopt Kernel + Governance Profile. All v6 governance constructs remain valid within the Governance Profile.
- Existing Agent Configurations without `agentType` default to `generative`.
- Existing deontic constraints, oversight protocols, and due process configurations continue to function — they are now scoped to the Governance Profile rather than the kernel.
- New constructs (`agentType`, `pipeline`, `contract-assertion`, `agentProvenance` annotations) are additive.
- A v6 document with no agent governance can be re-read as Kernel-only, with profiles adopted later.

---

## 8. Recommended Next Steps

1. **Draft the minimal kernel schema** — extract the kernel constructs from v6 and verify they stand alone without governance dependencies.
2. **Draft the Preparation Profile schema** — grounded pipelines, assertion gate types, Formspec-as-validator integration.
3. **Draft the Governance Profile schema** — relocate the v6 governance constructs, verify no kernel dependencies are missing.
4. **Prototype a grounded pipeline** against a real government form (CSBG Tribal Plan is a candidate) to validate the preparation architecture against actual documents.
5. **Restructure conformance profiles** around the kernel + profile model: Kernel, Kernel + Preparation, Kernel + Governance, Full (Kernel + both profiles).
6. **Extract the Preparation Profile as a standalone adoption guide** — this is the beachhead. Agencies don't need to understand deontic constraints or SMT verification to deploy safe AI-assisted document processing.
