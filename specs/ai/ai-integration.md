---
title: WOS AI Integration Specification
version: 1.0.0-draft.2
date: 2026-10-19
status: draft
---

# WOS AI Integration Specification v1.0

**Version:** 1.0.0-draft.2
**Date:** 2026-10-19
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS AI Integration Specification defines Layer 2 of the Workflow Orchestration Standard: the governance structures for AI agent participation in WOS workflows. The `agents[]` and `aiOversight` embedded blocks of a `$wosWorkflow` document declare agent registration, deontic constraints on agent behavior, autonomy levels with impact-level caps, the Formspec-as-validator pattern, a confidence framework with decay and calibration, fallback chains guaranteeing graceful degradation, decision drift detection, AI-specific oversight extensions, volume constraints, agent-specific review sampling, agent disclosure for due process, the Narrative provenance tier, and the Assist Governance Proxy.

**Native ownership.** `agents[]` and `aiOversight` are part of the enclosing `$wosWorkflow`. They govern that workflow and MUST NOT declare independent `targetWorkflow`, `url`, or `version` fields. `actors[]` owns actor identity and kind; `agents[]` supplies policy and invocation settings for matching agent actors; `aiOversight` owns shared AI policy. This document defines their behavior; the Workflow schema defines their authored shape. A conforming processor MUST satisfy both. ADRs record the decision history and are not needed to interpret this document.

**ADR 0064 framing.** Per ADR 0064, agents are first-class actors (`ActorKind::Agent` in the Rust runtime) and invocation goes through a substrate-neutral `AgentInvoker` port. The `agents[].invoker` discriminator selects which adapter (`wos-agent-anthropic`, `wos-agent-claude-sdk`, `wos-agent-mcp`, `wos-agent-a2a`, `wos-agent-http`, `wos-agent-stub`) handles a given agent's invocations. WOS specifies the *governance contract* the runtime enforces — deontic constraints, confidence floor, autonomy caps, fallback chain — and treats the substrate as adapter-tier. Multi-agent orchestrators are *one kind* of `AgentInvoker`, not a special spec concept. See `crates/wos-core/src/agent/mod.rs` for the port definition.

AI governance is not a separate track. Every Layer 2 concept extends a Layer 1 (Workflow Governance) concept through the kernel's named seams. Review protocols gain AI-specific suppression. Data validation pipelines gain Formspec-as-validator. Due process gains agent disclosure. Structured audit gains the Narrative tier. Quality controls gain agent-specific sampling.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 and does not modify the kernel's processing model. It defines AI governance structures that attach to kernel workflows through the `actorExtension`, `contractHook`, `provenanceLayer`, and `lifecycleHook` seams. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

AI agents increasingly participate in high-stakes workflows. Empirical research constrains the design of any standard governing this participation:

- Naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024; meta-analysis of 106 experiments).
- Model-generated explanations are systematically post-hoc rationalizations, not faithful reasoning traces (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023).
- Behavioral drift between model versions can cause near-total performance collapse on specific tasks (Chen, Zaharia, and Zou, 2023).
- No single defense against prompt injection has proven robust against adaptive adversaries.

This specification encodes these findings as structural requirements. The agent is outside the trust boundary. The WOS Processor -- not the agent -- enforces constraints, validates outputs, and controls workflow progression.

### 1.2 Design Goals

1. **AI plugs into human governance, not alongside it.** Every AI governance structure extends a Layer 1 structure.
2. **Constraints are external to the agent.** The WOS Processor enforces governance. The agent cannot weaken its own constraints.
3. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.
4. **Formspec-as-validator.** Agent output is untrusted input validated against the same Formspec contract a human would submit against.
5. **Every normative claim is testable** (inherited from Kernel §1.2 Design Goal 6). Every AI-specific obligation in this spec — deontic constraint enforcement, autonomy-cap enforcement, confidence-decay semantics, fallback-chain execution, drift detection, assurance composition — MUST reduce to a conformance test or lint rule. An AI behavior that no test can falsify is not a governance guarantee. Authors adding a new AI obligation MUST also add the corresponding test artifact.

### 1.3 Scope

**Within scope:** agent registration via `actorExtension`; deontic constraints (permission, prohibition, obligation, right); autonomy levels with impact-level caps; Formspec-as-validator; confidence framework with decay; fallback chains; decision drift detection; AI-specific oversight extensions; volume constraints; agent-specific review sampling; agent disclosure; Narrative provenance tier; Assist Governance Proxy; conformance profiles.

**Out of scope:** SMT verification, equity guardrails, constraint zones, multi-step sessions, tool use governance, agent lifecycle state machine (Layer 3: Advanced Governance). Lifecycle topology, case state (Layer 0: Kernel). Due process, review protocols, data validation pipelines (Layer 1: Workflow Governance -- this layer extends them).

### 1.4 Relationship to Lower Layers

The `agents[]` and `aiOversight` embedded blocks govern the enclosing `$wosWorkflow` envelope's kernel surface. Per ADR 0063 §2.1, the blocks have no independent identity — the envelope's `url` and `version` are the sole identity boundary. Per ADR 0064, agents are first-class actors (`actors[]` entries with `type: "agent"`) joined to `agents[]` runtime declarations by `id` (lint `WOS-AGENT-XREF-001`). AI governance extends kernel and governance structures through four seams:

| Seam | What Layer 2 Attaches |
|------|----------------------|
| `actorExtension` | Agent taxonomy (deterministic/statistical/generative) on `agents[]` declarations; `ActorKind::Agent` is first-class per ADR 0064. |
| `contractHook` | Formspec-as-validator: agent output as untrusted input. |
| `provenanceLayer` | Narrative tier: non-authoritative model-generated explanation. |
| `lifecycleHook` | Deontic enforcement, autonomy enforcement, AI-specific oversight suppression, agent sampling, agent disclosure. |

**Substrate portability.** The `agents[].invoker` discriminator selects an `AgentInvoker` adapter (Anthropic SDK / Claude Agent SDK / MCP / A2A orchestrator / HTTP service / deterministic stub) at deploy time. The spec is portable across substrates because the runtime never names a concrete adapter; deployment binds the discriminator. See `crates/wos-core/src/agent/mod.rs` for the port and `crates/wos-agent-stub` for the canonical deterministic implementation used by conformance fixtures.

### 1.5 How AI Extends Human Governance

| Layer 1 Concept (Human) | Seam | Layer 2 Extension (AI) |
|---|---|---|
| Review protocols | `lifecycleHook` on `review` tags | AI-specific suppression: hide agent output until independent assessment. |
| Data validation pipelines | `contractHook` | Formspec-as-validator: agent output validated through same gates. |
| Due process (notice, appeal) | `lifecycleHook` on `adverse-decision` tags | Agent disclosure requirement added to notice. |
| Structured audit (Reasoning + Counterfactual tiers) | `provenanceLayer` | Narrative tier (non-authoritative, AI-specific). |
| Quality controls (review sampling) | `lifecycleHook` on `quality-check` tags | Agent-specific sampling: stratified, adversarial methods. |
| Separation of duties | `lifecycleHook` on `review` tags | Agent MUST NOT review its own output. |
| Override authority | `lifecycleHook` on `determination` tags | Agent output MUST NOT override human override. |
| Screener intake routing | `contractHook` | Agent-assisted screening: agent classification feeds Screener Determination Records via contractHook. |

### 1.6 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Conformance

### 2.1 Conformance Profiles

Three profiles are defined:

**AI Basic.** Agent registration (S3) and Formspec-as-validator (S6) are enforced. The processor correctly registers agents via the `actorExtension` seam and validates agent output against Formspec contracts.

**AI Governed.** Basic conformance plus: deontic constraints (S4), autonomy levels (S5), confidence framework (S7), fallback chains (S8), and agent disclosure (S12).

**AI Complete.** Governed conformance plus: decision drift detection (S9), AI-specific oversight extensions (S10), volume constraints (S11), agent-specific review sampling (S11), Narrative tier provenance (S13), and Assist Governance Proxy (S14).

### 2.2 Conformance Requirements

1. An AI Integration Processor MUST implement agent registration (S3).
2. An AI Integration Processor MUST implement the confidence framework (S7), including ConfidenceReport validation.
3. An AI Integration Processor MUST validate fallback chains at document load time, rejecting chains that cycle or lack a terminal action (S8).
4. An AI Integration Processor MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## 3. Agent Registration

This section is normative.

### 3.1 Agent as Actor Type

Layer 2 registers `agent` as an actor type via the kernel's `actorExtension` seam (Kernel S10.1). An agent actor has additional identity and provenance requirements beyond the kernel's `human` and `system` types.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique agent identifier within the workflow. |
| `agentType` | enum | REQUIRED | `deterministic`, `statistical`, or `generative`. |
| `modelIdentifier` | string | REQUIRED | Identifier of the model powering this agent. |
| `modelVersion` | string | REQUIRED | Version of the model. |
| `description` | string | OPTIONAL | Human-readable description of the agent's role. |
| `capabilities` | array of Capability | OPTIONAL | Declared capabilities with I/O contracts. |
| `modelVersionPolicy` | enum | OPTIONAL | `pinned`, `approved`, or `latest`. Default: `pinned`. |

Each declaration MUST join exactly one `actors[]` entry by `id`, and that actor's `type` MUST be `agent`. Conversely, every actor of type `agent` MUST have exactly one native declaration before admission. The declaration does not repeat `type`. Duplicate ids, missing matches, and matches to human or system actors are errors. The identity requirements above apply to every declared agent; a processor MUST NOT invent a model version or agent type to admit an incomplete declaration.

The declaration MAY also contain `invoker`, `deonticConstraints`, `confidenceFloor`, `confidenceDecay`, `fallbackChain`, `driftMonitoring`, `approvedVersions`, `calibration`, `autonomyPolicy`, `actionOverrides`, and `cascadingInvocations` as defined in this specification and the embedded [Agent Config specification](agent-config.md). Direct `agents[].autonomy` is invalid; §5.3 defines the single route for workflow, agent and action limits. Missing invocation configuration may remain an authoring draft; it cannot pass execution readiness for an action requiring that agent.

### 3.2 Agent Type Taxonomy

The kernel treats agents uniformly. Layer 2 differentiates governance by type:

| Type | Characteristics | Governance Implications |
|------|----------------|------------------------|
| `deterministic` | Rules engines, decision tables, lookup systems. Output is a pure function of input. | Confidence reporting is trivial (always 1.0). Drift detection focuses on input distribution shift. |
| `statistical` | Classical ML. Fixed model, bounded output space, calibratable confidence. | Confidence calibration is meaningful and required. Drift detection uses standard statistical methods. |
| `generative` | LLMs. Unbounded output space, non-deterministic, injection-vulnerable. | Confidence is difficult to calibrate. Formspec-as-validator is essential. Drift detection must account for prompt sensitivity. |

### 3.3 Capability Declaration

Each capability declares what the agent can do, with Formspec Definition references for I/O contracts:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Capability identifier. |
| `description` | string | OPTIONAL | What this capability does. |
| `inputContractRef` | string (URI) | OPTIONAL | Formspec Definition or JSON Schema for input. Uses headless contract pattern (Core S2.3). |
| `outputContractRef` | string (URI) | OPTIONAL | Formspec Definition or JSON Schema for output. |
| `preconditions` | array of string (FEL) | OPTIONAL | Boolean FEL expressions evaluated before invocation; see §3.3.1. |

Capabilities MUST be objects conforming to this table, with ids unique within the agent. Bare strings are not capabilities. Input and output references MUST resolve to the selected definitions before invocation; unresolved references are errors, not permission to skip validation. A capability declaration does not itself grant access to data, permission to write case fields, or authority to bypass human review.

### 3.3.1 Capability Preconditions

A capability MAY declare a `preconditions` array of FEL expressions (Kernel §7). Before the processor invokes the capability, every expression is evaluated against the current evaluation context -- `caseFile`, `@event`, `@agent`, and any other context a guard may reference (Kernel §7.4). Semantics:

1. All entries MUST evaluate to boolean `true` for the capability to be invocable.
2. If any entry evaluates to `false`, or fails to evaluate to a boolean, the capability is **skipped**: the processor does NOT invoke the agent and instead falls through to the selected agent fallback chain (§8).
3. Absent or empty `preconditions` (the default) means the capability is always invocable.
4. Every precondition evaluation MUST produce a provenance record with `recordKind: "capabilityInvocation"`. The record's `data.invocationBlocked` flag MUST be `true` when a precondition caused the processor to skip the agent, and the record's `outcome` MUST then be `preconditionNotSatisfied` (Kernel §8.2.2 reserved outcome literal) so that audit tooling can distinguish a declarative gate from an agent failure. This shape is enforced at schema-validation time by `$defs/CapabilityInvocationRecord` in `schemas/wos-workflow.schema.json` (promoted Facts-tier $defs per ADR 0076 step 5); persisted append-only logs validate against `schemas/wos-provenance-log.schema.json`, where `FactsTierRecord` composes `CapabilityInvocationRecord` via `allOf` so every conformant provenance log participates in the MUST.
5. Preconditions are evaluated **before** the agent input contract is rendered and before any guardrails run. They are the cheapest gate in the capability pipeline; use them to keep agents from being called against cases that are structurally wrong for them.

Preconditions are declarative: they say "only invoke this capability when the world looks like this." They do not relax deontic constraints (§4) -- a capability that passes its preconditions is still bound by every permission, prohibition, obligation, and right declared against it.

### 3.4 Model Version Policy

| Policy | Behavior |
|--------|----------|
| `pinned` | Agent uses exactly the declared `modelVersion`. Version changes require document update. |
| `approved` | Agent may use any version from an approved list. Version changes emit provenance but do not require document update. |
| `latest` | Agent uses the latest available version. Every invocation records the actual version used. |

When a version change is detected (for `approved` and `latest` policies), the processor MUST emit a provenance record of type `agentVersionChange`.

These are model-selection policies, not semantic-version compatibility ranges. The spellings `minor-allowed` and `major-allowed` are not accepted. A processor without the version-selection service required by `approved` or `latest` MUST report that configuration as unsupported; it MUST NOT silently substitute `pinned` or invent a resolved version.

### 3.5 Trust Boundary

The agent is outside the trust boundary of the governance envelope. The WOS Processor -- not the agent -- enforces constraints, validates outputs, and controls workflow progression. This separation ensures governance survives agent changes, prompt injection, and behavioral drift.

When capability declarations reference domain-specific concepts, agents SHOULD use Formspec Ontology terms (Ontology S3) to establish semantic contracts. Ontology terms provide stable, machine-readable identifiers for domain concepts that survive agent replacement -- when one agent is swapped for another, the ontology contract remains the same even if the underlying model changes.

### 3.6 Security Patterns (informative)

The trust boundary model is compatible with the CaMeL (Capability-Limited Model) dual-LLM security architecture: a privileged controller validates whether an untrusted capability model's output should be committed. In WOS terms, the WOS Processor plays the controller role, and the agent plays the capability model role. Deontic constraints (S4) and Formspec-as-validator (S6) provide the validation mechanism. Implementations MAY adopt the CaMeL pattern as an implementation strategy for the trust boundary defined in S3.5.

### 3.7 Normative Constraints

1. Agents MUST NOT override human decisions.
2. Provenance records for `agent` actors MUST include model identifier, model version, confidence, and input summary.
3. Cascading autonomous agents (an agent at `autonomous` level invoking another agent at `autonomous` level) MUST be explicitly declared in the invoking agent's `cascadingInvocations` property. Each entry is a nonempty native agent id, unique within the array, and MUST resolve in the same Workflow revision. The list names targets this agent may invoke at `autonomous` level; it does not grant the target additional permissions or replace invocation checks.
4. An actor's type is immutable for a given action. A human using an AI tool remains a `human` actor if the human reviews and commits the output.

---

## 4. Deontic Constraints

This section is normative.

### 4.1 Overview

Agent constraints use four deontic types adopted from OASIS LegalRuleML. Rights are checked before invocation; output constraints are checked after interface validation and before output is committed to workflow state. The WOS Processor enforces both phases.

At each scope, `deonticConstraints` is one array of tagged objects. Every entry has a required, scope-unique `id` and `kind`, whose value is `permission`, `prohibition`, `obligation`, or `right`. The kind selects exactly one of the payloads in §4.2–§4.5; fields from another kind are invalid. The grouped object with `permissions`/`prohibitions`/`obligations`/`rights` arrays and an untyped common `expression` property are not alternative formats.

An entry MAY carry `citationRefs`, an array of nonempty descriptive strings naming supporting sources, and `x-*` metadata. A citation may be a URI or a conventional source citation; it is not an executable rule or an automatic resolver request. These preserve the legal basis and other author annotations; they do not change execution. Permission, prohibition and obligation entries MAY declare `bypassable` under §4.7; rights cannot be bypassed. A processor MUST NOT infer defeasibility, precedence or waiver authority from metadata. The former `defeasible` flag has no defined execution semantics and is not accepted. Constraints are combined by §4.6–§4.7, not array order. Shared schema definitions and a single native policy model MUST describe the same representation at every scope.

### 4.2 Permission

A Permission bounds what the agent is allowed to produce. Agent outputs within permission bounds are accepted; outputs outside are violations.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique permission identifier. |
| `allowedFields` | array of JSON Pointer | OPTIONAL | Permitted paths in the proposed output, as defined below. |
| `bounds` | string (FEL) | OPTIONAL | FEL expression defining value bounds. |
| `onViolation` | enum | REQUIRED | `reject`, `escalateToHuman`, `switchToAssistive`, or `flag`. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation: `pass`, `deny`, or `escalate`. |

A permission MUST declare `allowedFields`, `bounds`, or both. `allowedFields` uses RFC 6901 pointers into the proposed output: `/assessment` permits that property and its descendants, but not `/assessmentOther`; `""` permits the whole output. Every terminal value in the proposed output MUST be covered by a permitted path. A terminal value is a scalar (including null) or an empty object or array. A scalar root or an empty root object or array has pointer `""` and counts as a root value, not an empty proposal. Thus an empty list permits no output, including `{}` or `[]`. Path coverage compares decoded RFC 6901 segments, not string prefixes: escaped slash and tilde characters remain part of their property names, and array indices are individual segments. Omission adds no field restriction. All applicable permission lists and bounds apply together; one permission cannot widen another. `bounds` evaluates against the full output and case context and is satisfied only by boolean `true`, subject to §4.9.

### 4.3 Prohibition

A Prohibition forbids specified agent outputs or actions regardless of confidence.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique prohibition identifier. |
| `condition` | string (FEL) | REQUIRED | FEL expression defining the forbidden condition. When true, the prohibition is triggered. |
| `reason` | string | OPTIONAL | Human-readable explanation. Included in provenance when triggered. |
| `onViolation` | enum | REQUIRED | Enforcement action. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation. |

### 4.4 Obligation

An Obligation requires the agent to perform a specified action or include specified content. Checked after output, before commit.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique obligation identifier. |
| `requirement` | string (FEL) | REQUIRED | FEL expression defining the required condition. When false, the obligation is unmet. |
| `reason` | string | OPTIONAL | Human-readable explanation. |
| `onViolation` | enum | REQUIRED | Enforcement action. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation. |

### 4.5 Right

A Right specifies what the agent is entitled to receive as input context. The WOS Processor has an Obligation to provide data specified in agent Rights. A Rights violation MUST NOT be attributed to the agent and MUST trigger a system-level error.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique right identifier. |
| `entitlement` | JSON Pointer | REQUIRED | Path into the resolved input context, rooted at `/input` or `/caseFile`. |
| `description` | string | OPTIONAL | Human-readable description. |

Rights are input checks, performed before handing data to an invoker. An entitlement is satisfied when its selected value exists and is not null; `false`, `0`, and an empty string are present values. A missing or null entitlement prevents invocation and produces a processor-attributed failure and provenance identifying the right. It MUST NOT become an ordinary agent policy violation. A right does not use `onViolation` or `nullBehavior`, and its entitlement is a data locator, not a boolean FEL predicate.

### 4.6 Enforcement Ordering

Before invocation, the processor resolves the selected actor, capability, action context and inputs, evaluates capability preconditions, checks required input rights, and validates the input definition. A failure in these stages MUST NOT call the invoker. Rights failures are processor errors; failed preconditions follow §3.3.1.

After invocation, the processor validates the untrusted output against its selected definition. It then evaluates output policy in the following order before any case-file write. Validation failure MUST NOT be treated as a policy pass. A fixture-provided output can exercise these policy stages, but it does not prove invocation or input/output validation.

1. **Permissions** -- Is the agent permitted to perform this action? Structural bounds on allowed outputs (S4.2).
2. **Prohibitions** -- Is the agent prohibited from this action? Forbidden output patterns (S4.3).
3. **Obligations** -- Has the agent fulfilled its obligations? Required output elements (S4.4).
4. **Confidence floor** -- Does the agent's confidence meet the floor? (S7.4).
5. **Volume constraints** -- Has the agent exceeded volume constraints? Rate limits on autonomous actions (S11.1).
6. **Human review sampling** -- Is this action selected for quality review? Quality-assurance selection (S11.2; Governance S7).

When multiple constraints are violated simultaneously, the processor MUST apply the most restrictive enforcement action. Restriction ordering, from least to most restrictive: `flag` < `switchToAssistive` < `escalateToHuman` < `reject`. The `requireReview` and `log` actions exposed by external policy-engine bridges (see below) compose at the same precedence as `escalateToHuman` and `flag` respectively.

Rule truth is kind-specific: permission bounds `false` violates a permission; prohibition condition `true` violates a prohibition; obligation requirement `false` violates an obligation. Null handling is a disposition under §4.9, never a substituted boolean. Rule results MUST identify the scope, rule id, observed outcome, and selected enforcement action. A required evaluation error or unsupported enforcement action prevents acceptance; it cannot fall through as success.

`flag` records the violation and permits further checks; it never grants a write. `switchToAssistive` requires human confirmation before committing the proposal. `escalateToHuman` holds the proposal for human resolution. `reject` prevents its commit. If the selected processor cannot create or resume the required review, it MUST report unsupported execution and preserve the proposal without committing it. Subsequent review and commit use the maintained task and governed-output operations; policy evaluation is not a second case-file writer.

> **Editorial note (ADR 0076 absorption, 2026-04-28):** prior to this absorption, the Runtime Companion's §8.3 specified `log < flag < requireReview < reject < escalateToHuman` (with `escalateToHuman` most restrictive). The two documents disagreed on whether `reject` or `escalateToHuman` was the strictest action. Reconciliation here adopts the AI Integration spec's local ordering (`reject` strictest), bridges `requireReview`→`escalateToHuman` and `log`→`flag` for cross-doc compatibility, and treats the resulting four-action ordering as canonical. The decision rationale: `reject` is a final answer that terminates the agent invocation; `escalateToHuman` is recoverable (the human may approve), so it cannot logically dominate `reject`.

**Composition with external policy engines (deny-overrides-permit).** When a workflow integrates an external policy engine (XACML, OPA, Cedar, or equivalent) via the integration binding mechanism (Workflow §3, §8), the engine's decision composes with this deontic pipeline as a strict restrictor: a `deny` decision from a policy engine overrides any `permit` produced by S4.2-S4.4 evaluation. **External policy engines are more restrictive, never more permissive.** A policy-engine `permit` does not override a deontic `prohibition`, an unmet `obligation`, a confidence-floor failure, a volume-constraint violation, or a sampling selection. The engine's decision is recorded as a Facts-tier provenance record (Kernel §8) and made available under the binding's `outputBinding` path so guard expressions on subsequent transitions (Kernel §4.5) can reference it.

### 4.7 Constraint Composition

Deontic constraints compose across three levels. All applicable constraints at all levels are evaluated for every agent invocation:

1. **Workflow-level** -- constraints in `aiOversight.deonticConstraints`. Apply to all agent invocations in the workflow.
2. **Agent-level** -- constraints declared on a specific `AgentDeclaration`. Apply to all invocations of that agent.
3. **Action-level** -- constraints in `agents[].actionOverrides[].deonticConstraints`, selected by the executing lifecycle action's `actionRef`. Apply to that invocation point.

When constraints at different levels conflict, the most restrictive enforcement action wins (same restriction ordering as S4.6).

The processor obtains actor and action identity from the admitted execution context, not from an agent output or an untrusted event's claimed action id. An action override adds restrictions and MUST NOT replace or remove workflow or agent constraints. A processor unable to evaluate a declared action-level policy MUST refuse execution requiring it; it MUST NOT claim the action scope passed after checking only the agent scope.

**Consistency constraints** detect contradictions between agent output and case data or prior outputs (e.g., `not (output.eligible = true and output.riskScore > 90)`).

**Scope constraints** ensure output stays within declared capability boundaries by restricting which fields the agent may produce (Permission with `allowedFields`).

**Guardrail bypass:** A rationale is not authority. A processor MAY support an explicit, authenticated waiver for a named rule marked `bypassable: true`. The waiver MUST bind the authorized actor, admitted Workflow revision, one invocation, selected agent and action, and the exact native JSON Pointer of the rule occurrence. The authenticated authorization operation MUST resolve that occurrence; a readable rule id alone is insufficient because ids are unique only within a scope. Provenance MUST retain that locator, rule id, authorization decision and rationale. It skips only that occurrence; same-named rules in other scopes remain active. It MUST NOT bypass rights, input/output definition validation, human review requirements, or other rules. A processor without that authorization operation MUST reject every waiver request, including one containing purported approval metadata; event fields such as `bypass`, `deonticBypass`, or `rationale` alone MUST NOT suppress policy evaluation. The default for `bypassable` is false.

### 4.8 Examples (non-normative)

**Permission** -- bound agent output to allowed fields and value ranges:

```
bounds: "output.eligibilityScore >= 0 and output.eligibilityScore <= 100"
```

**Prohibition** -- forbid agent from issuing final denial on rights-impacting case:

```
condition: "output.eligible = false and instance.impactLevel = 'rights-impacting'"
```

**Obligation** -- require agent to provide justification for every extracted value:

```
requirement: "every(output.extractedFields, $.sourceReference != null)"
```

(Formspec Core §3.5.1: the second argument is a FEL predicate expression; `$` is rebound to each element of `extractedFields`, so `$.sourceReference` reads the property on each record.)

**Cross-layer constraint** -- prohibition referencing kernel case state, Layer 1 temporal parameters, and Layer 2 agent output simultaneously:

```
condition: "output.eligible = true and caseFile.verification.totalIncome > parameters.eligibilityThreshold * 1.5"
```

This last example demonstrates the evaluation context enrichment model (Kernel S7.3): `output` is injected by Layer 2, `caseFile` is kernel base context, and `parameters` is injected by Layer 1 via temporal resolution.

### 4.9 Null Propagation

When a deontic constraint expression evaluates to `null` (due to missing data or unresolvable references), the behavior is determined by the kernel's `impactLevel`:

| Impact Level | Null Behavior | Rationale |
|---|---|---|
| `rights-impacting` | `escalateToHuman` | Unknown constraint state on a rights-affecting decision requires human review. |
| `safety-impacting` | `escalateToHuman` | Safety constraints that cannot be evaluated must not silently pass. |
| `operational` | `pass` | A null result adds no violation under this default. |
| `informational` | `pass` | A null result adds no violation under this default. |

A constraint MAY override the default with an explicit `nullBehavior` property.

`pass` means this rule adds no violation, independently of its predicate polarity. `deny` applies the rule's `onViolation`; `escalate` requires human resolution. A malformed expression, evaluator failure or non-boolean predicate result is an evaluation error, not null or boolean false. The processor MUST retain the error and prevent acceptance. This section does not apply to a right's missing entitlement, whose behavior is fixed by §4.5.

---

## 5. Autonomy Levels

This section is normative.

### 5.1 Overview

Every agent action operates at a declared autonomy level. Autonomy is a property of the **action site** -- the point in the workflow where an agent is invoked -- not a property of the agent itself. The same agent MAY operate at different autonomy levels in different contexts.

Human review and write permission are separate: an autonomous action still cannot write outside its permissions, and an assistive action cannot commit before confirmation even when its proposed fields are permitted. `readOnly` is a permission restriction, not an autonomy level. `supervised` is not an alias for `supervisory`.

### 5.2 Level Definitions

| Level | Semantics | Fallback Requirement |
|-------|-----------|---------------------|
| `autonomous` | Agent output committed without human review. REQUIRES deontic constraints. | MUST define fallback to human. |
| `supervisory` | Agent output provisionally committed. Human reviews within a `reviewWindow`. If the window expires without intervention, the output becomes final with a provenance record noting implicit confirmation. | MUST define fallback to human. |
| `assistive` | Agent produces a recommendation. Human reviews, may modify, and explicitly confirms. The human owns confirmed output. | MUST define fallback to human. |
| `manual` | Action performed entirely by a human. Agent MAY provide contextual assistance on demand. | N/A. |

### 5.3 Autonomy Constraints

1. An action with `autonomous` autonomy MUST have associated deontic constraints (S4). Autonomous actions without deontic constraints are a structural error.
2. An action with `assistive` autonomy MUST create a human task for confirmation.
3. An action with `supervisory` autonomy MUST define a `reviewWindow` (ISO 8601 duration).
4. The effective autonomy MUST NOT exceed the workflow's impact-level cap. For `rights-impacting` and `safety-impacting` workflows, the cap is `assistive`; for `operational` and `informational` workflows it is `autonomous`. This specification defines no field or approval that raises the impact cap.
5. The **authored autonomy ceiling** is the minimum of `aiOversight.defaultAutonomy` (default `assistive`), `agents[].autonomyPolicy.maxAutonomy` (if present), the selected `agents[].actionOverrides[].autonomy` (if present), and the impact-level cap from constraint 4. The order from most to least human control is `manual`, `assistive`, `supervisory`, `autonomous`. Omitted agent or action limits add no further restriction. The **effective autonomy** is the minimum of the processor-owned current or requested level, the authored ceiling, and every active operational reduction, including expired or missing required calibration and pending recalibration. At first invocation without prior operational state, the processor initializes the retained agent operational level from the minimum of workflow, agent and impact limits and active operational reductions; a particular action's ceiling is excluded from that initialization. New operational demotion or calibration reductions lower the retained agent level. A lower action-site ceiling or a request for lower autonomy limits that invocation only and MUST NOT persistently demote the agent at other action sites. Removing an operational reduction never raises the retained level; restoring a higher level requires a fresh applicable approval under §5.4, even after successful recalibration. Raising an authored `assistive` ceiling requires a Workflow revision; human approval cannot change it.
6. Every agent invocation MUST have a reachable path to workflow completion that does not require any agent to succeed.

### 5.4 Autonomy Escalation

Escalation (raising an agent's effective autonomy level) requires human approval:

1. The agent's `autonomyPolicy.escalation` MUST define the selected `EscalationRule` using Agent Config §1.4.2. Its condition is evaluated against processor-owned operational state and admitted case data.
2. The human bound to the rule's `requiredApproverRef` actor slot MUST review and approve the escalation through the maintained authenticated actor-binding and authorization operation. A descriptive actor role label is not authority.
3. The rule's positive `escalationExpiry` duration bounds the approval. The processor records approval time, expiry, actor, selected rule and prior level using its maintained clock and authorization operation. At or after expiry the approval no longer elevates the level; all active reductions still apply. A replacement approval requires a new authorization decision.
4. Escalation produces a provenance record of type `autonomyEscalation` including the approving actor, the escalation conditions that were met, and the expiry.
5. Escalation can restore a level only within the authored ceiling and cannot override an active reduction. It does NOT bypass deontic constraints (S4) or any other governance mechanism. An agent escalated to `autonomous` still has all its constraints evaluated. A processor without the required approval, state or expiry operation MUST report this execution as unsupported; `escalationActive` or similar claims in an ordinary event or model output are not approval.

### 5.5 Autonomy Demotion

Demotion (lowering an agent's effective autonomy level) MAY be automatic:

1. The agent's `autonomyPolicy` MUST define demotion triggers. Standard triggers:
   - Calibration accuracy falls below threshold.
   - Guardrail violation rate exceeds threshold within evaluation window.
   - Model version change (pending recalibration).
   - Drift detection alert (S9).
2. When a demotion trigger fires, the demotion takes effect for the next invocation. In-flight sessions are not retroactively affected but are annotated with the demotion event in provenance.
3. When `pendingRecalibration` is true, the processor retains that reduction until its calibration operation records successful recalibration. Removing that reduction does not erase another active reduction, revive an expired approval, or authorize a rise above the authored ceiling. An elevation requiring approval still follows §5.4.
4. Demotion produces a provenance record of type `autonomyDemotion` including the trigger that fired and the new effective level.

### 5.6 Dynamic Autonomy Selection

The processor resolves the current or requested level, active approvals, demotions and calibration state before each invocation, using its maintained state and clock. An action may request a lower level; a request cannot remove an applicable limit or elevate the retained current level without §5.4 approval. The calculation in §5.3 then determines effective autonomy. This section introduces no additional authored FEL selector or second autonomy property. Ordinary event and model-output fields cannot supply trusted approval, calibration or demotion state. A processor unable to resolve required operational state MUST refuse execution rather than assume a favorable state.

---

## 6. Formspec-as-Validator

This section is normative.

### 6.1 Principle

**Agent output is untrusted input validated against the same Formspec contract a human would submit against.**

This extends Layer 1's data validation pipelines (Governance S5) via the `contractHook` seam (Kernel S10.2). The Formspec Definition serves as a validation cage around agent output, eliminating the maintenance burden of custom validation while preserving structural verifiability.

### 6.2 Requirements

1. A Formspec Definition used as an agent output contract MUST apply the same validation rules, Bind expressions, and constraint Shapes as the human-facing form.
2. Validation failures on agent output MUST be recorded as provenance and trigger fallback (S8), not silent acceptance.
3. Agent-touched fields MUST be annotated in the Formspec instance with `agentProvenance` metadata: which agent, which model version, what confidence, what source material.
4. WOS processors MUST delegate contract evaluation to a Formspec-conformant processor (Core S1.4). Deontic constraint evaluation (S4) occurs outside the Formspec processing model -- the WOS Processor evaluates deontic constraints against the contract processor's output. The Formspec processing model (Core S2.4) is a closed system; WOS governance wraps it.

### 6.3 Agent Provenance Metadata

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agentId` | string | REQUIRED | Which agent produced this value. |
| `modelVersion` | string | REQUIRED | Model version at time of production. |
| `confidence` | number (0-1) | OPTIONAL | Confidence for this specific field. |
| `sourceRef` | string | OPTIONAL | Reference to the source material used. |

---

## 7. Confidence Framework

This section is normative.

### 7.1 Confidence Report

Every agent output MUST be accompanied by a ConfidenceReport.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `overall` | number (0.0-1.0) | REQUIRED | Estimated probability that the output is correct. |
| `method` | enum | REQUIRED | How confidence was derived. |
| `explanation` | string | OPTIONAL | Human-readable explanation of confidence factors. |
| `fieldLevel` | object | OPTIONAL | Per-output-field confidence values, keyed by field name. |

The `overall` value represents the agent's estimated probability that its output would be accepted without modification by a competent human reviewer performing the same task. `ConfidenceReport` is one closed object, also used by the maintained invoker result as `AgentResult.confidenceReport`, explicit stub-adapter responses and supplied-output policy checks. A scalar `confidence` is not an alternative report, and a processor MUST NOT invent a missing `method`. `overall` and every `fieldLevel` value MUST be finite numbers in [0,1]. Descriptive `x-*` metadata conveys no authority. A claimed calibration method describes the report; it does not establish trusted calibration evidence or alter the operational limits in §5. Missing required calibration operations remain unsupported.

API and provenance fields named `confidenceReport` MUST reference this same report definition and retain the reported values. Processor-computed decay, cumulative confidence and floor decisions are separate evaluation evidence, identified by their observed output and native policy scope; they MUST NOT replace report fields or masquerade as another input report. Absent evaluation evidence means that stage was not observed, never an invented successful check.

### 7.2 Confidence Methods

| Method | Definition | Calibration Requirement |
|--------|-----------|------------------------|
| `modelNative` | Derived from the model's own probability estimates. | MUST be calibrated. |
| `calibrated` | Post-hoc calibration applied to model-native scores using historical accuracy data. | Calibration is inherent. |
| `heuristic` | Derived from structural properties of the output (consistency checks, output stability). | SHOULD be calibrated. |
| `conformal` | Conformal prediction sets with guaranteed coverage. | Calibration is inherent. |
| `declared` | Manually assigned by the agent developer. | MAY be calibrated. |

### 7.3 Per-Field Confidence

When per-field confidence is available, guard expressions and routing logic MAY reference individual fields. This enables targeted routing: a classification output with high confidence on document type but low confidence on an extracted amount routes only the low-confidence field to human review.

### 7.4 Confidence Floor

A `ConfidenceFloor` is the same closed object at workflow (`aiOversight.confidenceFloor`), agent (`agents[].confidenceFloor`) and selected action (`agents[].actionOverrides[].confidenceFloor`) scopes. It adds a restriction; omission adds no floor at that scope. Scalar floors are invalid.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `threshold` | number (0.0-1.0) | REQUIRED | Minimum acceptable confidence. |
| `onViolation` | enum | REQUIRED | Action when confidence is below threshold: `reject` (discard output and trigger fallback) or `escalateToHuman` (route to human review with full context). |

Thresholds and reported confidence MUST be finite numbers in [0,1]. A floor is violated only when confidence is strictly below its threshold; equality passes. The processor MUST evaluate every applicable floor and combine all violated effects using §4.6. It MUST NOT select only the highest threshold and discard a lower floor's stricter effect. For example, a workflow floor of 0.8 with `reject` and an action floor of 0.9 with `escalateToHuman` reject confidence 0.7, require human resolution at 0.85, and pass both floors at 0.9. Missing or invalid required confidence is an error, not an implicit score. Each result identifies its native scope. Confidence floors participate at step 4 of §4.6, after output rules and before volume constraints.

### 7.5 Confidence Decay

Agent outputs become less reliable as underlying case data changes. Confidence decay models this degradation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `enabled` | boolean | REQUIRED | Whether confidence decay is active. |
| `halfLife` | string (duration) | OPTIONAL | Positive ISO 8601 duration after which effective confidence halves, using the maintained clock/timer operation. |
| `triggers` | array of DecayTrigger | OPTIONAL | Events causing immediate confidence reduction. |

Supported positive ISO 8601 durations MAY use a fractional final component (for example `PT0.5S` or `PT1.5H`) and omit zero-valued components (`P1Y1D` or `PT1H1S`). A week component stands alone, a time separator requires a time component, and whitespace is invalid. The maintained timer operation still MUST accept the represented positive duration; schema acceptance does not establish clock or monitoring support.

A `DecayTrigger` is a closed object with required `event` (a nonempty native event-name string) and `decayFactor` (a finite number in [0,1]). Enabled decay requires at least one trigger or a positive supported `halfLife`; an absent required clock, history or decay operation is unsupported execution. A trigger binds its event to its multiplicative `decayFactor`. When the trigger fires, the effective confidence of all outputs produced by that agent for the current case is multiplied by the factor. After decay, the processor reevaluates every applicable floor using §7.4 and retains the combined disposition; decay cannot replace a required rejection with escalation.

### 7.6 Temporal Confidence Thresholds

Confidence thresholds used in autonomy decisions and routing SHOULD reference `governance.policyParameters` (Governance S13) so they can be adjusted based on operational experience.

### 7.7 Cumulative Confidence

In multi-step agent interactions, errors compound. The cumulative confidence after step *n* is the product of individual step confidences by default. A four-step process where each step has 0.9 confidence yields cumulative confidence of approximately 0.66.

A conformant processor MUST track cumulative confidence. It evaluates that confidence against every applicable floor using §7.4; a rejected result remains rejected, and an escalation holds the interaction for human resolution. Missing tracking for a required multi-step interaction is unsupported execution.

---

## 8. Fallback Chains

This section is normative.

### 8.1 Principle

Every workflow that uses agents MUST function correctly when agents are unavailable (Kernel S1.2). Agent unavailability is a regular operating condition, not an edge case. A rejected proposal is never committed by fallback. Human fallback receives unresolved work and refusal evidence and must create its own authorized resolution; it does not relabel rejected output as accepted.

### 8.2 Fallback Chain Execution

When an agent invocation fails (error, timeout, guardrail rejection, or unavailability), the fallback chain is executed:

1. Each level is attempted in order.
2. A successful level stops the chain.
3. A failed level advances to the next.
4. Every attempt MUST produce a provenance record.
5. A terminal human step hands the unresolved work to its declared task; a terminal `fail` records failure and does not produce or commit agent output. Neither is a successful agent invocation.

### 8.3 Fallback Steps

Both `aiOversight.fallbackChain` and `agents[].fallbackChain` use the same array of closed, tagged `FallbackStep` objects. Each step has required `action`; its value selects exactly the fields below. Strings, cross-kind fields and undeclared operational fields are invalid. Descriptive `x-*` metadata is permitted and carries no authority.

| `action` | Required fields | Meaning |
|----------|-----------------|---------|
| `retry` | `maxRetries`, `backoff`, `initialInterval` | Retry the same selected capability with the same admitted input. `maxRetries` is an integer from 1 through 4294967295, excluding the initial invocation; this finite bound is shared by the native type and schema. `backoff` is `fixed`, `linear` or `exponential`; `initialInterval` is a positive ISO 8601 duration supported by the maintained timer operation. |
| `alternateAgent` | `alternateAgentRef` | Invoke the same capability id on the referenced native agent. That agent must declare the capability; its own input, policy, review and output checks apply. |
| `escalateToHuman` | `taskActionRef` | RFC 6901 pointer to an exact native lifecycle `createTask` action in the admitted Workflow revision. Use its task configuration and maintained assignment rules. |
| `fail` | none | Terminate the failed action with recorded failure. |

A human fallback pointer MUST select an actual native lifecycle action occurrence, not an action-shaped object in metadata. The selected action's `assignTo` MUST resolve to a declared human actor through the maintained assignment and authenticated binding operation. Its contract, mappings, completion/failure events and lifetime settings remain intact. This reference selects task configuration only: it does not enter the containing state, fire its transition, execute sibling actions or establish that the containing guards ran. Creation uses the admitted current context and maintained task operation, including its timer capture and contract checks. Missing operations or context make execution unsupported; resolving a pointer or emitting provenance does not create a task. Repeated `createTask.taskRef` labels are permitted and disambiguated by the pointer; the fallback step does not accept a `taskRef` alias or invent a task-definition registry.

For retry number `n` starting at one, the delay is `initialInterval` for fixed backoff, `n × initialInterval` for linear backoff, and `2^(n−1) × initialInterval` for exponential backoff. Invalid or unrepresentable schedules are configuration errors. Exhausting the retry count advances to the next step. A successful retry stops the chain and follows the ordinary validation, policy, review and commit path.

### 8.4 Selection and Chain Constraints

A declared agent chain selects the complete chain for that agent; an omitted agent chain inherits the workflow chain. Chains are never concatenated. An explicit empty chain is invalid, rather than a request to inherit or suppress fallback. If both scopes omit the chain, the document may remain an authoring draft, but execution readiness for that agent fails. There is no capability-specific fallback field.

Every selected chain MUST end in `escalateToHuman` or `fail`, with no earlier terminal step. An alternate-agent step invokes the target through its maintained invocation route; if that target's invocation and selected fallback chain fail, the caller advances to its next step. A successful target output is still subject to the caller's applicable output validation and restrictions. A target's handoff to human review remains a held result, never a completed agent output. Every reference MUST resolve in the same admitted Workflow revision. Processors MUST validate the actual graph of selected chains for direct and indirect alternate-agent cycles and reject cycles at load time.

Every chain intended for `assistive`, `supervisory` or `autonomous` work MUST include a reachable human handoff as required by §5.2; a failure-only terminal does not satisfy that separate requirement. A processor without a required retry, invoker, task, validation or continuation operation MUST identify the unsupported operation and refuse execution readiness. It MUST NOT translate an actor string into an invented task or treat emitted task provenance as an actual created task.

---

## 9. Decision Drift Detection

This section is normative.

### 9.1 Overview

Structural guards against patterns where agent behavior silently drifts from its intended purpose.

### 9.2 Drift Indicators

| Indicator | Description | Detection Method |
|-----------|-------------|-----------------|
| Training data contamination | Agent was trained on human determination outcomes (approvals, denials) rather than task-specific ground truth. The agent optimizes for outcome prediction, not task accuracy. | Declaration: the agent configuration MUST disclose training data characteristics. |
| Optimization objective misalignment | Agent optimizes for "maximize agreement with reviewer decisions" rather than "maximize extraction accuracy against ground truth." | Declaration: the agent configuration MUST disclose its optimization objective. |
| Rubber-stamp detection | Human reviewers are ratifying agent output without genuine cognitive engagement at increasing rates. | Monitoring: track reviewer engagement metrics (time spent, modifications made, disagreement rate). |

### 9.3 Governance Implications

When training data contamination is detected, the agent's task MUST be reclassified from preparation to determination, and appropriate governance (deontic constraints, autonomy caps) MUST be applied.

Per-agent monitoring has the closed authored shape defined in [Drift Monitor](drift-monitor.md). The shared `aiOversight.driftDetection` setting and the training-data/optimization disclosure input shape are not defined by this revision. `driftDetection` is reserved and MUST be rejected at native admission; `agents[].modelConfig` is not an alternate location. Implementations MUST report the required disclosure or monitoring operation as unsupported when it lacks defined inputs and an owning operation. They MUST NOT invent configuration from runtime examples or claim complete AI conformance from declaration parsing. A lossless editor may retain such raw drafts, but they cannot pass admission or readiness.

---

## 10. AI-Specific Oversight Extensions

This section is normative.

### 10.1 Overview

Layer 2 extends Layer 1's review protocols (Governance S4) with AI-specific presentation and suppression controls. These attach via `lifecycleHook` on `review`-tagged transitions.

### 10.2 Independent-First Suppression

When the `independentFirst` protocol (Governance S4.1) is active on a transition where an agent has produced output, the agent's output MUST be hidden until the reviewer's independent assessment is recorded. The interface MUST enforce this ordering.

### 10.3 Presentation Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `showConfidence` | boolean | `false` | Display the agent's confidence score alongside the recommendation. |
| `showAlternatives` | boolean | `false` | Display alternative recommendations the agent considered. |
| `highlightLowConfidenceFields` | boolean | `false` | Visually highlight fields where agent confidence is below threshold. |
| `showDiffFromIndependent` | boolean | `false` | After independent assessment is recorded, highlight differences between reviewer and agent. Only meaningful with `independentFirst`. |

---

## 11. Volume Constraints and Review Sampling

This section is normative.

### 11.1 Volume Constraints

Rate limits on autonomous agent actions prevent runaway automation:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAutonomousPerHour` | integer | OPTIONAL | Maximum autonomous actions per hour. |
| `maxAutonomousPerDay` | integer | OPTIONAL | Maximum autonomous actions per day. |

Each declared limit is a nonnegative integer; zero permits no autonomous actions within that window. When volume limits are reached, subsequent agent invocations are escalated to human review regardless of confidence. Counts and window boundaries come from maintained processor state and time; event/model assertions cannot supply them. Required counters, clock or review operations that are absent make execution unsupported.

### 11.2 Agent-Specific Review Sampling

Extends Layer 1's review sampling (Governance S7.1) with agent-specific methods:

| Method | Description |
|--------|-------------|
| `random` | Standard random sampling (same as Layer 1). |
| `stratified` | Stratified by agent type, confidence level, or case characteristics. |
| `adversarial` | Targeted sampling of cases likely to reveal agent weaknesses. |

Actions selected for sampling are routed to human review regardless of the autonomy level. The review outcome feeds the calibration pipeline.

---

## 12. Agent Disclosure

This section is normative.

### 12.1 Overview

Layer 2 extends Layer 1's due process notice requirements (Governance S3) with agent disclosure. This attaches via `lifecycleHook` on `adverse-decision`-tagged transitions.

### 12.2 Disclosure Requirements

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `discloseThatAgentAssisted` | boolean | REQUIRED | Disclose that an AI system assisted in the determination. |
| `discloseModelIdentity` | boolean | OPTIONAL | Disclose the model identifier. Default: `false`. |
| `discloseConfidence` | boolean | OPTIONAL | Disclose the agent's confidence score. Default: `false`. |

For `rights-impacting` workflows, `discloseThatAgentAssisted` MUST be `true`. This requirement is consistent with OMB M-24-10 and EU AI Act Art. 13.

---

## 13. Narrative Provenance Tier

This section is normative.

### 13.1 Overview

Layer 2 adds the **Narrative tier** to the kernel's provenance model via the `provenanceLayer` seam (Kernel S10.3). This is the only AI-specific provenance tier.

### 13.2 Epistemic Status

The Narrative tier is **NON-AUTHORITATIVE**. Every provenance record that includes Narrative tier content MUST label it non-authoritative. This requirement exists because model-generated explanations are systematically unfaithful to actual reasoning processes (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023).

A conformant implementation MUST NOT treat Narrative tier content as dispositive evidence in any adjudicative, audit, or due process context.

### 13.3 Narrative Tier Record

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `narrative` | string | REQUIRED | The model's natural language explanation. |
| `authoritative` | boolean | REQUIRED | MUST be `false`. Explicitly marks this content as non-authoritative. |
| `modelIdentifier` | string | REQUIRED | Model that generated this narrative. |
| `modelVersion` | string | REQUIRED | Model version. |

### 13.4 Relationship to Layer 1 Tiers

Layer 1 provides the Reasoning tier (rules applied, evidence consulted) and Counterfactual tier (what would change the outcome). Layer 2 extends these existing tiers with agent-specific metadata (model version, confidence scores, input summaries) but does not add new tiers beyond the Narrative tier. The Reasoning and Counterfactual tiers remain authoritative; the Narrative tier is informational only.

---

## 14. Assist Governance Proxy

This section is normative.

### 14.1 Overview

The WOS Assist Governance Proxy is a **WOS-defined governance construct** that consumes the Formspec Assist protocol (Assist S2.1). It sits between the Assist Consumer and the Assist Provider (Assist S2.1), intercepting tool invocations and applying the WOS deontic constraint framework.

The proxy is NOT a Formspec concept. It is a WOS construct that wraps Assist protocol interactions with governance.

### 14.2 Requirements

1. The proxy MUST NOT modify either role's Assist conformance requirements.
2. The proxy MUST apply deontic constraints (S4) to tool invocation results.
3. The proxy MUST produce provenance records for each governed tool invocation.
4. Per-tool-category governance is supported: different tool categories may have different constraint sets.
5. Transport bindings follow Assist S7 (Transport Bindings).

---

## 15. AI Governance in the Workflow Envelope

AI governance has one authored identity: the enclosing Workflow's `url` and `version`. Agent settings and shared oversight do not have independent document identities. The following fields have one owner:

| Native path | Meaning |
|---|---|
| `actors[].id/type` | Identity and kind of the actor performing work. |
| `agents[]` | Matching agent declarations, structured capabilities, per-agent rules and operational settings. |
| `aiOversight.defaultAutonomy` | Shared action autonomy limit, defaulting to `assistive`. |
| `aiOversight.deonticConstraints` | Workflow-wide rules, using the same tagged rule definition as agent and action scopes. |
| `aiOversight.confidenceFloor/fallbackChain` | Shared confidence and fallback policy defined in §7–§8. |
| `aiOversight.volumeConstraints` | Shared count limits defined in §11.1. |
| `aiOversight.disclosure` | Shared disclosure settings defined in §12.2. |

The settings `aiOversight.oversightExtensions`, `reviewSampling`, `narrativeTier`, `driftDetection` and `assistGovernanceProxy` are reserved: their behavioral requirements are retained in §9–§14, but this revision does not define complete authored configuration shapes. Their presence MUST fail native admission; their canonical schema properties MUST reject configuration rather than accept an untyped object. An implementation cannot infer accepted fields from earlier runtime structs or samples. This does not relax required independent-first review, sampling, narrative labeling, drift disclosure or proxy behavior: required behavior without a supported owning operation prevents readiness and complete AI conformance. Raw draft storage is distinct from admission.

The names `agentDisclosure` and `x-transportAgentDetails` are not alternate authored locations for `disclosure` and `agents`. Unknown non-extension properties are invalid, including retired standalone-document identity fields. Implementations MAY create an internal read-only view of these native fields; that view is not another authored or serializable AI document. All consumers MUST use the same native loading and policy interpretation.

This example shows the policy-bearing native structure. Execution additionally requires resolution of referenced definitions and a bound invoker; the example alone is not execution readiness evidence.

```json
{
  "$wosWorkflow": "1.0",
  "url": "urn:example:assessment-review",
  "version": "1.0.0",
  "title": "Review an agent assessment",
  "impactLevel": "operational",
  "actors": [
    {"id": "reviewer", "type": "human"},
    {"id": "assessor", "type": "agent"}
  ],
  "agents": [{
    "id": "assessor",
    "agentType": "deterministic",
    "modelIdentifier": "assessment-rules",
    "modelVersion": "1.0.0",
    "capabilities": [{
      "id": "assess",
      "inputContractRef": "urn:example:assessment-input",
      "outputContractRef": "urn:example:assessment-output"
    }],
    "deonticConstraints": [
      {"id": "input-required", "kind": "right", "entitlement": "/input/application"},
      {"id": "assessment-only", "kind": "permission", "allowedFields": ["/assessment"], "onViolation": "reject"},
      {"id": "reason-required", "kind": "obligation", "requirement": "output.assessment.reason != null", "onViolation": "reject"}
    ]
  }],
  "aiOversight": {"defaultAutonomy": "assistive"},
  "lifecycle": {
    "initialState": "review",
    "states": {"review": {"type": "atomic"}}
  }
}
```

When the Workflow carries both `governance` and AI blocks, their restrictions compose. Impact-level and behavior-specific presence requirements still apply. Absence of a block does not excuse a check required by the selected behavior.

### 15.1 Normative Contract

The authored schema, typed loading, semantic validation and runtime MUST agree on these fields and meanings. Schema success establishes structural validity only. Execution readiness additionally requires supported semantics, resolved inputs and services, and all required behavioral checks. A processor MUST identify each unsupported required feature and MUST NOT report readiness or a successful skipped check. It MUST preserve the authored input and the exact refusal or failure evidence.

The processor owns invocation, output validation, policy, review and commit. The invoker returns untrusted output; it does not authorize its own writes. MCP, CLI, synthesis and Studio consumers MUST call maintained owner operations rather than implement their own parser, policy evaluator or readiness rule. No compatibility format, synthetic missing field or fixture-only conversion may change the candidate being checked.

### 15.2 Composition

The actor join uses the Kernel actor model. Input and output definitions use the maintained contract-validation operation, delegating Formspec evaluation to a Formspec-conformant processor. Agent invocation uses `AgentInvoker`. A supplied-output policy exercise obtains its admitted actor, action, input and case context through the test harness or processor interface, never from the output being evaluated. An operation that cannot establish a required context must refuse that operation and report the unchecked stage; policy-only output checks do not establish that rights or input validation ran before an invocation. Policy uses one native evaluator across workflow, agent and action scopes. Human confirmation uses maintained tasks and reviews. Case-file changes use the governed output-commit operation, which checks the actual write effects, including deletions; a permitted output field is not by itself authority for a case-file write. Provenance records the observed stages and dispositions.

Policy and review decisions MUST bind the exact proposal, actor, action, Workflow revision and relevant case state. If those inputs change before commit, the governed commit operation MUST recheck them or refuse the stale decision. These operations may be separate modules, but they form one execution route. A caller may not omit a required stage, replace a missing result with an expectation, or build an alternate route to obtain a pass.

### 15.3 Conformance

Schema tests MUST cover every rule variant, structured capabilities, identity requirements and native scope. They MUST reject bare capability strings, grouped legacy constraints, cross-kind fields, retired enum spellings and transport-only agent injection. Maintained examples MUST validate against their owning definitions.

Independent runtime controls MUST demonstrate permission bounds and field scope; prohibition and obligation polarity; present, absent, null, false and zero-valued rights; all null dispositions; errors distinct from null; root scalars and empty containers; decoded pointer boundaries; composition across scopes; enforcement precedence; and refusal of unauthorized bypass. Each test MUST assert observed decisions, unchanged state when blocked, and rule-specific provenance. Tests generated solely from the same schema or catalog are insufficient.

Autonomy controls MUST pair fresh and expired calibration, active and expired approval, and pending and completed recalibration without exceeding authored limits. Confidence controls MUST exercise every scope and mixed effects below, between and at thresholds. Fallback controls MUST exercise variant validation, selection, reference resolution, cycle refusal and terminal requirements. Where the owning execution operation is absent, the expected result is explicit unsupported execution, not a simulated pass.

Invocation conformance additionally proves the invoker is called only after required preconditions, rights and input validation pass. Commit conformance additionally proves output validation, permissions and human review cannot be bypassed, and observes the actual case-file change. A deterministic invoker adapter is sufficient for local behavioral proof when the rest of this path is real. Tests supplying output directly MUST identify that fixture boundary and cannot claim invocation, Formspec validation, human review or governed commit was executed. Hosted qualification separately proves deployment services and durable behavior.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS Workflow Governance] Formspec Working Group, "WOS Workflow Governance Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [Assist] Formspec Working Group, "Formspec Assist Specification v1.0".

### Informative References

- Vaccaro, M. et al., "When combinations of humans and AI are useful: A systematic review and meta-analysis", Nature Human Behaviour, 2024.
- Turpin, M. et al., "Language Models Don't Always Say What They Think", NeurIPS 2023.
- Lanham, T. et al., "Measuring Faithfulness in Chain-of-Thought Reasoning", Anthropic, 2023.
- Chen, L., Zaharia, M., and Zou, J., "How is ChatGPT's behavior changing over time?", 2023.
- OMB Memorandum M-24-10, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", March 2024.
- EU AI Act, Regulation (EU) 2024/1689.
- OASIS LegalRuleML TC, "LegalRuleML Core Specification v1.0", 2021.
