Do not treat the named standards as exhaustive. Expand the landscape independently and surface important relevant standards, models, specifications, governance frameworks, safety architectures, adjacent formalisms, and domain standards that were not provided in the prompt. Treat agent participation in high-stakes workflows as part of the core conceptual landscape, not as an implementation detail or appendix.

You are conducting a deep landscape study to inform the design of a modern, open, AI-native standard for high-stakes workflows and case orchestration, including the governed participation of AI agents in consequential workflows.

This is not a generic workflow roundup, not a low-code tooling survey, and not a simple automation comparison. The purpose is to produce a serious research foundation for a new standard that can support consequential, long-running, human-in-the-loop workflows such as grants, licensing, benefits, inspections, investigations, case review, approvals, compliance processes, and exception-heavy intake.

Your job is to identify the best abstractions, the most durable lessons, the real gaps, and the most important traps across both old and new standards, systems, formal models, adjacent specifications, governance frameworks, legal constraints, and agent-safety research.

Use primary sources wherever possible. Prefer standards documents, official specifications, technical architecture docs, academic papers, standards-body materials, regulatory frameworks, case law, and authoritative project documentation over vendor marketing.

## Main objective

Produce a research report and feature matrix that answers:

1. What relevant standards, systems, formal models, governance frameworks, adjacent specifications, and domain interoperability standards already exist?
2. Which concepts should be adopted directly, adapted, treated as missing opportunities, or marked deliberately out of scope for a new standard?
3. What conceptual architecture would best support a standard that is:
   - machine-friendly
   - auditable
   - explainable
   - interoperable
   - suitable for long-running workflows
   - expressive for human tasks and decisions
   - safe for AI-agent participation in consequential workflows
   - amenable to conformance testing
   - optimized for AI-assisted authoring, validation, monitoring, and simulation

## Critical instruction: expand beyond the seed list

The standards, systems, and specifications named in this prompt are examples and starting points only. They are not exhaustive and must not define the boundary of the research.

You must independently discover and investigate additional relevant standards, specifications, academic models, interchange formats, policy languages, event models, audit/provenance standards, runtime models, governance frameworks, safety architectures, agent protocols, verification methods, and related systems that are not explicitly listed here.

Actively search for:

- older standards that may contain durable abstractions even if their tooling is outdated
- adjacent standards that solve subproblems workflow systems often handle badly
- governance frameworks for high-impact or rights-impacting AI
- legal and regulatory frameworks that impose design constraints on consequential automated decision systems
- agent-safety architectures and formal methods relevant to constraining tool-using agents
- standards-body work from organizations such as W3C, OMG, OASIS, IETF, CNCF, NIST, ISO, HL7, and similar bodies where relevant
- academic papers or pattern catalogs that became de facto reference points
- modern open-source specifications and execution models that may not yet be formal standards but are influential
- domain standards that demonstrate real-world interoperability for high-stakes tasks, audit, evidence, or case handling

Do not simply summarize the named examples. Perform genuine landscape expansion.

For every major research area, identify:

1. the canonical standards already known,
2. additional related standards, specifications, frameworks, or papers discovered during research,
3. whether those newly discovered artifacts add important concepts missing from the obvious sources.

Surface especially valuable discoveries that were not in the initial prompt.

## Expanded conceptual goal

This research must treat AI agent participation in high-stakes workflows as part of the conceptual landscape itself, not as a later implementation concern.

That means the standard should be researched not only as a workflow/process/case specification, but also as a possible governance layer for agent participation in consequential workflows.

You must therefore study:

- where AI agents can safely participate in workflows
- where agent autonomy must be constrained or prohibited
- how human oversight must be structured to avoid naive human-in-the-loop failure modes
- how confidence, escalation, abstention, and override should work
- how audit records should distinguish between factual execution records and model-generated narrative
- how prompt injection, tool abuse, behavioral drift, and hidden policy violations affect workflow design
- how emerging agent interoperability protocols should relate to governance constraints
- what must be represented in the standard versus what belongs in runtime implementations

Do not assume that “add a human approval step” is sufficient oversight. Evaluate whether research shows that human review patterns succeed or fail depending on structure, timing, confidence communication, and cognitive forcing design.

## Additional requirements surfaced by related research

In addition to surveying standards and adjacent models, determine whether the new standard requires explicit workflow-kernel governance primitives that cannot be delegated to agent protocols or provider APIs.

Specifically investigate whether the standard should include first-class support for:

- actor kinds, for example human, system, and agent
- authority modes or transfer-of-control semantics
- proposal versus commit as distinct workflow events
- durable replay-safe agent steps, where nondeterministic agent outputs are persisted as step results rather than embedded in orchestration logic
- idempotent tool-call envelopes with schema references, result digests, approval boundaries, and replay semantics
- provenance hooks linking decisions and actions to evidence, policy, rule versions, confidence, and input/output digests
- evidence as a governed artifact distinct from model memory or prompt context
- trust labeling and policy enforcement for untrusted content
- delegation chains and accountability-preserving multi-agent handoffs
- structured clarification and input-required states for ambiguous or incomplete cases
- counterfactual, reason-code, and appeal-related artifacts for adverse decisions
- multi-layer audit records that distinguish immutable facts, structured reasoning, generated narrative, and counterfactuals

Research whether modern tool and agent protocols should be treated as interoperability layers rather than governance layers, and determine which semantics belong in:

1. the stable core standard,
2. protocol or provider profiles,
3. optional audit/security profiles,
4. implementation-specific runtime behavior

Evaluate whether a generalized tool capability descriptor is needed beyond HTTP-centric API descriptions, including inspiration from affordance-oriented models for non-HTTP and long-running tools.

Treat conformance as at least three separate problems:

- protocol interoperability
- governance correctness
- durability/replay correctness

Include scenario-based and adversarial conformance designs for:

- overrides
- delegation
- delayed callbacks
- replay without re-execution
- untrusted evidence ingestion
- prompt-injection resistance
- policy version changes in long-running cases
- confidence-threshold escalation
- abstention
- kill switches
- continuation-of-service during appeal
- appeal and reversal flows

Assess whether verifiable audit extensions or profiles should be part of the ecosystem, including provenance-aligned audit records and optional cryptographic or transparency-based attestation models.

Explicitly determine what should remain out of scope for the standard, especially any attempt to standardize model-internal cognition, chain-of-thought, or provider-specific prompt formats.

## Neutral comparison baseline

Do not compare only standards documents and products. Also use neutral comparison frameworks where possible, especially:

- workflow patterns literature
- workflow resource patterns
- workflow data patterns where relevant
- declarative process modeling literature
- process mining and event-log interchange literature
- formal semantics and conformance literature

Use these neutral baselines to assess expressive power, feature coverage, and interoperability requirements.

## Required research scope

Survey at least the following domains.

### 1. Process and orchestration standards

Research traditional and modern process standards and related specifications, including but not limited to:

- BPMN
- CMMN
- DMN
- SCXML
- XPDL
- BPEL
- WS-HumanTask / BPEL4People
- Serverless Workflow
- Amazon States Language
- Arazzo
- YAWL and similar systems
- other relevant workflow/process standards you independently discover

### 2. Human task and case-oriented systems

Study systems and standards that treat discretionary work, incomplete information, evolving context, and human judgment as first-class concerns.

Focus on concepts such as:

- task lifecycle
- assignment
- claim/unclaim
- delegation
- substitution
- review chains
- escalation
- queues and inboxes
- SLAs
- rework
- suspension and reopen
- discretionary branching
- notes, evidence, and annotations
- task history retrieval
- authority to add, skip, or alter work items
- skill-based routing
- separation of duties
- supervisory override
- selective visibility of case materials

Also identify any formal or quasi-formal standards, models, or interoperability formats related to human task management or case work that are not already named above.

### 3. Formal execution and state models

Study formal or semi-formal models relevant to workflow semantics, including:

- finite state machines
- hierarchical state machines / statecharts
- event-driven models
- actor-style orchestration
- saga / compensation patterns
- workflow patterns literature
- Petri-net-inspired approaches where relevant
- declarative and constraint-based process models such as DECLARE, DCR Graphs, and similar approaches
- artifact-centric and milestone-based models such as GSM and related systems

Focus on what these models clarify about:

- correctness
- liveness
- dead ends
- concurrency
- cancellation
- compensation
- partial completion
- long-running behavior
- flexibility under exceptions
- deferred choice
- milestone-gated progression
- history-state restoration
- constraint satisfaction versus explicit path enumeration

Also look for additional formal models and execution semantics not explicitly named here.

### 4. Decision, policy, and authorization systems

Research:

- decision models
- rules engines
- policy languages
- authorization models and standards
- RBAC / ABAC / ReBAC ideas where relevant
- analyzable policy systems
- legal-rule representation systems

Focus on:

- routing rules
- approvals
- eligibility
- exceptions
- explainability
- versioning
- override handling
- conflict resolution
- obligations and advice
- separation of policy from process
- testability and schema compatibility
- defeasible rules
- deontic concepts such as obligation, permission, and prohibition
- temporal parameter versioning for date-effective policies
- business-calendar-aware deadlines and SLA computation

You must independently identify additional policy and authorization standards beyond the obvious examples.

Determine whether the new standard should define a language-agnostic policy evaluation interface with:

- structured inputs
- requested action
- context
- decision output
- explanation output
- obligations/advice
- policy/version references
- conflict resolution semantics
- test fixture compatibility

Also evaluate whether the architecture should preserve a PEP/PDP-style separation between policy evaluation and enforcement.

### 5. Data contracts and interface standards

Research standards and conventions relevant to schemas and interoperability, including:

- JSON Schema
- OpenAPI
- AsyncAPI
- typed envelopes
- schema evolution patterns
- contract/versioning patterns
- government or domain data vocabularies where relevant

Focus on how workflow inputs, outputs, tasks, decisions, evidence, and integrations should be structured and validated.

Also search for adjacent interface and contract standards that may be useful for workflow specification or portability, including non-HTTP and affordance-oriented models.

### 6. Eventing and integration boundaries

Research:

- CloudEvents
- webhook conventions
- idempotency patterns
- replay semantics
- correlation and causality
- message delivery guarantees
- event envelopes
- trace-context propagation
- enterprise integration patterns where relevant

Focus on what a workflow standard should borrow for triggers, callbacks, handoffs, and external system coordination.

Also identify any additional interoperability or event standards that could matter here.

### 7. Provenance, audit, and observability

Research standards and systems relevant to:

- provenance
- auditability
- immutable history
- trace semantics
- operational telemetry
- process-mining logs
- event sourcing where relevant
- object-centric event logs where relevant
- tamper-evident logs where relevant

Focus on representing:

- what happened
- why it happened
- who or what caused it
- which rule or event triggered it
- what evidence or state was used
- what changed
- what was overridden by a human
- how multiple related objects participate in the same case

Research not only provenance capture but also:

- provenance constraints
- validity rules
- semantic equivalence
- portability to audit and analysis tools
- tamper evidence and integrity verification

Determine whether the standard should support object-centric execution and audit logs rather than only flat per-case traces.

### 8. Durable execution and runtime resilience

Study modern runtime systems and patterns for:

- retries
- timers
- backoff
- resumability
- durable state
- replay
- idempotency
- compensation
- waiting on external callbacks
- long-running execution
- failure recovery
- versioning of running instances
- alternative durability strategies such as replay, journaling, checkpointing, or memoized step execution

Focus on what should belong in the standard versus what should be left to implementations.

Also identify any formal runtime or recovery models that are relevant even if they are not marketed as workflow standards.

Pay special attention to the interaction between durable orchestration and nondeterministic AI-agent behavior. Determine whether agent invocations should be modeled as external, recorded step results rather than inline orchestration logic, and what semantics are required for replay, re-evaluation, compensation, idempotency, versioning, and auditability.

### 9. AI-native authoring, validation, and simulation

Research what an actually AI-native standard would require beyond historical workflow specs.

Cover:

- structured representations optimized for LLM generation and editing
- typed authoring operations rather than freeform prompt blobs
- semantic validation
- static analysis
- ambiguity detection
- simulation and what-if analysis
- bottleneck detection
- policy-risk analysis
- explainable translation from natural language to structured workflow definitions
- round-tripping between human-readable and machine-executable representations
- canonical serialization suitable for hashing/signing
- typed patch/edit operations
- semantic diff and refactoring operations
- language-server-style diagnostics and code actions
- canonical simulation trace formats
- round-trip natural-language explanation of workflow definitions

Do not treat “AI-native” as “add prompts.” Be rigorous about what properties make a standard easier to generate, inspect, diff, validate, simulate, test, and explain using AI systems.

Also independently look for adjacent standards, schemas, or protocol ideas from AI tooling, DSL design, verification, and structured editing that could inform this.

### 10. AI agents in consequential workflows

Research the landscape of AI-agent participation in high-stakes workflows as a first-class design domain.

Cover:

- human-in-the-loop, human-on-the-loop, and human-over-the-loop oversight models
- autonomy tiering and graduated autonomy
- structured human review protocols versus naive review patterns
- confidence calibration, abstention, selective prediction, and escalation thresholds
- separation of reasoning from permission enforcement
- tool-use governance and permission models
- agent memory, checkpointing, and long-duration state handling
- multi-agent coordination and handoff models
- agent interoperability protocols
- defense-in-depth for prompt injection and indirect prompt injection
- formal verification or contract-based constraint systems for agent behavior
- runtime circuit breakers, kill switches, and degraded modes
- model/version drift monitoring and rollback strategies
- due-process and appeal implications when agents participate in consequential decisions
- counterfactual explanations, structured reason codes, and non-faithful model-generated explanations
- governance models for when agents may recommend, act, defer, or be prohibited from participation
- deliberation depth as a configurable property tied to decision criticality
- memory governance, retention, redaction, and privacy boundaries
- evidence synthesis and summarization safeguards
- long-context limitations, chunking strategies, and verification against “lost in the middle” failures

Treat this domain as central to the standard’s purpose whenever the standard may be used in environments where AI agents participate in real workflows.

### 11. Governance, rights-impact, and legal due-process frameworks

Research policy and legal frameworks relevant to consequential automated decisions and AI-assisted workflows.

Include:

- AI risk management frameworks
- public-sector AI governance guidance
- rights-impact or high-impact AI frameworks
- due-process requirements in automated or semi-automated decisions
- transparency, notice, appeal, and override obligations
- discrimination and bias-audit obligations
- administrative law implications when algorithmic systems function as de facto rules
- impact assessment frameworks
- global regulatory approaches where relevant
- public notice-and-comment requirements where relevant
- continuation-of-service protections during disputes or appeal
- rights to challenge, contest, or override automated outcomes

Do not treat these as peripheral policy background. Treat them as sources of hard design requirements for the standard.

### 12. Domain exemplars and interoperability proof points

Study domain-specific ecosystems that may provide reusable patterns for high-stakes interoperability, such as:

- healthcare workflow/task/audit standards
- government data-contract and conformance ecosystems
- identity, credential, or evidence-integrity standards where relevant
- domain-specific audit-event models
- cross-agency or inter-organizational exchange standards
- preservation or recordkeeping standards where relevant

Use these exemplars to understand what real interoperability, auditability, and conformance look like in practice.

## Required analytical lens

Approach this as standards archaeology plus modern systems synthesis.

Do not assume newer is better.
Do not assume older standards are irrelevant.
Do not assume current products represent the best abstractions.
Do not confuse tooling popularity with conceptual strength.
Do not assume “human in the loop” is automatically safe.
Do not assume agent protocols are sufficient governance standards.

For every major concept, evaluate:

- Is it modeling a real recurring workflow problem?
- Is it legible to both humans and machines?
- Can it be validated?
- Can it be simulated?
- Can it be tested for conformance?
- Is it composable?
- Is it interoperable?
- Is it explainable?
- Does it separate concerns cleanly?
- Is it robust under ambiguity, interruption, partial information, and exceptions?
- Is it suitable for AI-assisted authoring and review?
- Is it safe enough for agent participation in consequential workflows?
- Does it create meaningful accountability and appeal pathways?
- Can it constrain or monitor agent behavior without relying on model self-reporting?
- Does it support durable replay and long-running correctness?
- Does it handle multi-object cases rather than only single-instance traces?
- Does it preserve transport agnosticism and allow incremental adoption?

## Classification framework

For each feature or concept, classify it as one of:

- Adopted — should likely carry forward substantially intact
- Adapted — useful idea, but should be redesigned for a modern AI-native standard
- Missing — important capability not well handled by existing systems
- Deliberately out of scope — useful elsewhere, but should not be part of this standard

Be explicit and opinionated about why.

## Research questions to answer

Answer these with evidence and examples:

1. What abstractions have proven durable across generations of workflow and process systems?
2. Which abstractions are elegant in theory but consistently weak in real-world use?
3. What has been historically overemphasized?
4. What has been historically under-modeled?
5. How should a new standard distinguish among:
   - process flow
   - decision logic
   - task/work management
   - case state
   - evidence
   - data contracts
   - audit/provenance
   - runtime guarantees
   - agent governance
6. What capabilities are essential for high-stakes workflows but commonly underpowered in existing systems?
7. How should a new standard represent human judgment, discretionary action, and override authority without collapsing into unstructured chaos?
8. How should the standard represent AI-agent participation, abstention, escalation, prohibition zones, and appeal pathways?
9. What must be standardized for interoperability, and what should remain implementation-specific?
10. What would a credible conformance model look like?
11. What structure would best support AI-assisted authoring, migration, review, simulation, monitoring, and governance?
12. Should the architecture be centered on a small semantic kernel with layered interfaces, or on a monolithic DSL/modeling language?
13. Does the standard need a declarative constraint layer in addition to state/flow models?
14. Should audit and execution history be object-centric rather than only per-case?
15. Should the standard define a language-agnostic policy interface rather than standardize one policy language?
16. Should the standard be fundamentally case-centric and state-machine-native, or only support that style as one option among several?

## Specific issues the research must resolve

Address these explicitly:

- Whether naive human-in-the-loop review degrades outcomes in practice, and what structured alternatives are supported by evidence
- Whether model-generated reasoning or chain-of-thought can be treated as reliable audit evidence, or whether audit layers must separate facts from narrative
- Whether confidence calibration and abstention are mature enough to be standardized as part of escalation logic
- Whether agent interoperability protocols such as tool and agent communication standards should be treated as transport layers, while governance and accountability belong in the new standard
- Whether formal behavioral contracts, constraint systems, or temporal logics are practical for constraining agent participation
- Whether drift monitoring, shadow deployment, version pinning, rollback, and circuit breakers should be explicit conceptual features of the standard or implementation guidance
- How the standard should model appeal rights, adverse-action notice, override authority, and continuation-of-service requirements in rights-impacting contexts
- Whether durable replay-safe semantics for agent and tool steps should be mandatory in the core model
- Whether proposal, approval, and commit should be separate normative state transitions
- Whether evidence governance must be modeled separately from memory, context, or summaries
- Whether a generalized capability descriptor is needed for tools that are not well-described by HTTP-centric API contracts
- Whether a workflow-specific provenance profile with machine-checkable constraints should be required
- Whether object-centric event-log interoperability should be part of the audit model
- Whether declarative constraints and obligations should complement explicit process flow
- Whether AI-native authoring should include typed edits, semantic diffs, diagnostics, and refactors as first-class ecosystem expectations
- Whether simulation traces should be standardized for comparison, conformance, and audit reuse
- Whether tamper-evident audit-log profiles belong in the core ecosystem or optional profiles
- Whether business-calendar-aware deadline computation belongs in the core temporal model
- Whether separation-of-duties and skill-based routing should be first-class standard semantics
- Whether incremental adoption alongside legacy systems should shape the scope and layering of the standard

## Deliverables

Produce all of the following.

### A. Executive synthesis

A crisp narrative explaining:

- the most important findings
- the strongest inspirations
- the recurring failure modes
- the most important gaps
- the main design implications

### B. Expanded research corpus

A categorized list of the standards, systems, papers, governance frameworks, legal materials, domain standards, and projects reviewed.

For each one, include:

- what it is
- why it matters
- the most relevant concepts
- what it gets right
- where it falls short for a modern AI-native workflow standard
- whether it was part of the initial seed list or discovered during independent research

Make sure this corpus includes substantial material not explicitly named in the prompt.

### C. Feature taxonomy

Create a structured taxonomy of capabilities, grouped into categories such as:

- process topology
- states and transitions
- guards and conditions
- declarative constraints and obligations
- human tasks and queues
- decisions and policy
- timers and temporal behavior
- exceptions and compensation
- case state and evidence
- integrations and eventing
- provenance and audit
- object-centric history and log interoperability
- durability and runtime behavior
- validation and conformance
- agent governance and autonomy control
- safety and oversight controls
- AI-native authoring and simulation

### D. Feature matrix

Build a detailed matrix comparing the surveyed systems/standards against the taxonomy.

For each feature:

- define it clearly
- identify which systems support it
- classify it as Adopted / Adapted / Missing / Deliberately out of scope
- explain why

### E. Additional discoveries

Include a dedicated section titled “Important standards, models, and governance frameworks discovered during research” that highlights the most valuable relevant artifacts not explicitly listed in the original prompt.

For each discovery, explain:

- why it was not obvious
- what it contributes
- whether it meaningfully changes the proposed design direction

### F. Agent governance implications

Include a dedicated section titled “Design implications for AI agent participation in high-stakes workflows.”

It should address:

- autonomy tiering
- structured human review
- calibrated confidence and abstention
- agent permissioning
- prompt-injection resilience
- formal constraints and behavioral contracts
- audit architecture
- drift monitoring
- rollback and kill-switch design
- evidence versus memory boundaries
- where agents should be allowed, restricted, or forbidden

### G. Core vs profile boundary

Include a dedicated section titled “What belongs in the core standard versus profiles and mappings.”

It should determine which concepts belong in:

- the stable workflow kernel
- human-work profile(s)
- case/evidence profile(s)
- decision/policy profile(s)
- provenance/audit profile(s)
- agent interoperability profiles
- provider-specific mappings
- optional audit/security profiles
- implementation-specific runtime behavior

Discuss this specifically for:

- MCP
- A2A
- provider tool/function calling APIs
- telemetry/observability vocabularies
- verifiable audit mechanisms
- tool capability descriptors
- domain-specific interoperability artifacts

### H. Conceptual architecture and object model

Include a dedicated section titled “Conceptual architecture recommendation.”

It should:

- compare plausible architecture directions
- evaluate monolithic versus layered approaches
- determine whether a small semantic kernel with layered interfaces is preferable
- determine whether the standard should be case-centric and state-machine-native
- recommend a likely object model
- identify major conceptual layers
- describe separation of concerns
- identify likely non-goals
- describe serialization priorities
- discuss tradeoffs between expressiveness and simplicity

Make clear whether the standard should include a distinct governance layer for agent participation.

### I. Conformance strategy

Propose how implementations could be tested, including:

- canonical fixtures
- static validation
- semantic validation
- runtime behavior checks
- portability checks
- edge-case scenarios
- minimal conformance profiles
- agent-governance test cases
- audit-record test cases
- escalation and abstention test cases
- drift and rollback test cases
- replay and idempotency test cases
- adversarial tests for untrusted evidence and prompt injection
- semantic equivalence tests across engines
- simulation trace comparison tests
- soundness checks such as deadlock-freedom, livelock-freedom, and proper termination where feasible

Treat protocol interoperability, governance correctness, and durability/replay correctness as distinct conformance dimensions.

### J. Candidate architecture directions

Propose 2–4 plausible architecture directions and compare them, for example:

- state-machine-centric
- case-centric
- hybrid process + policy + task
- event-first orchestration
- workflow spec plus agent-governance overlay
- constraint-enhanced layered kernel

Recommend the strongest direction and explain why.

## Output requirements

Use clear sectioning and tables where useful.
Prefer comparative analysis over isolated summaries.
Include citations to primary or authoritative sources throughout.
Be explicit about tradeoffs.
Distinguish between ideas that are good in principle and ideas that are good in practice.
Where sources disagree, surface the disagreement.
Where you are inferring rather than directly citing, say so.

## Non-goals

Do not optimize for:

- simple if-this-then-that automation
- marketing automation
- generic low-code app builders
- RPA scripting
- diagramming for its own sake
- proprietary lock-in
- “BPMN but in JSON”
- superficial AI wrappers
- agent autonomy without governance
- audit designs that rely on model self-explanation as ground truth
- standardizing provider-specific prompt formats, chain-of-thought, or internal cognition traces
- full UI, form, or caseworker rendering standards
- embedding a single policy language as the required choice
- prescribing storage engines, brokers, or one vendor runtime
- specification gigantism that prevents incremental adoption

## Final section

End with six ranked lists:

1. Ten design principles that should guide the new standard
2. Ten underserved capabilities that existing ecosystems handle poorly
3. Ten traps to avoid when designing the standard
4. Ten additional standards, papers, or models discovered during research that deserve special attention
5. Ten governance requirements for safe AI-agent participation in consequential workflows
6. Ten conformance fixtures or scenario classes that any serious implementation should pass
