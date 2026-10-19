# WOS Verification Matrix

> **Regenerated from code registries** (commit `89039bd5e96f21f8e555d6c34a8e17dd30add2cf`). 136 rules across 50 T1 / 77 T2 / 9 T3 (1 LoadBearing, 0 Stable, 20 Tested, 115 Draft).

```text
┌─────────────────────────────────────────────────────────────────┐
│  Tier 1: wos-lint (static)            50 rules                  │
│  Single-document structural checks. Pattern matching and graph  │
│  walks over the JSON document tree. No parsing, no cross-doc.   │
├─────────────────────────────────────────────────────────────────┤
│  Tier 2: wos-lint --project (cross)   77 rules                  │
│  Multi-document resolution + FEL AST analysis. Loads a project  │
│  directory, resolves cross-references, parses FEL expressions.  │
├─────────────────────────────────────────────────────────────────┤
│  Tier 3: wos-conformance (dynamic)     9 rules                  │
│  Event-driven test fixtures. Feeds event sequences through      │
│  WosRuntime, asserts on observed state transitions, provenance  │
│  records, timer behavior, compensation ordering, and autonomy.  │
└─────────────────────────────────────────────────────────────────┘
```

**Graduation ladder:**

| Tier | Meaning |
|------|---------|
| `draft` | No executable fixture linked yet |
| `tested` | ≥1 executable fixture exercises this rule |
| `stable` | Tested + unchanged across ≥3 consecutive releases |
| `load_bearing` | Removing breaks ≥2 executable fixtures (spec_ref + suggested_fix required) |

## Tier 1 — Static Single-Document Rules (`wos-lint`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `AI-041` | AI | tested | Fallback chain MUST terminate in `escalateToHuman` or `fail`; MUST NOT cycle. | AI-041-negative-fallback-cycle.json |
| `AI-049` | AI | draft | Narrative records MUST have `authoritative: false`. | — |
| `CM-001` | CM | draft | Entry template `id` values MUST be unique within the sidecar. | — |
| `G-037` | G | draft | Assertion `id` values MUST be unique within the library. | — |
| `G-038` | G | draft | Assertions of type `arithmetic`/`range`/`temporal` SHOULD include `expression`. | — |
| `G-039` | G | draft | Assertions of type `source-grounded`/`consistency` SHOULD include `fields`. | — |
| `G-044` | G | draft | Delegation `expirationDate` MUST be strictly after `effectiveDate`. | — |
| `G-045` | G | draft | Delegation `revokedDate` MUST be on or after `effectiveDate`. | — |
| `G-047` | G | draft | Parameter `values` entries MUST be in ascending `effectiveDate` order. | — |
| `G-048` | G | draft | Binding `id` MUST match the key under which it appears in the `bindings` map. | — |
| `G-050` | G | draft | Resolved parameter value MUST be type-consistent with declared `type`. | — |
| `G-055` | G | draft | `expectedDuration` MUST be an ISO 8601 duration or the literal `"indefinite"`. | — |
| `G-057` | G | draft | Binding `values` entries MUST be in ascending `effectiveDate` order. | — |
| `G-058` | G | draft | Each Holiday entry MUST specify exactly one of `date` or `rule`. | — |
| `G-059` | G | draft | Operating hours `end` MUST be strictly after `start`. | — |
| `G-062` | G | draft | Adverse-decision templates MUST cover determination, reasons, rights, and instructions. | — |
| `G-065` | G | draft | Notification template section `id` values MUST be unique within a template. | — |
| `I-001` | I | draft | `outputBinding` JSONPath MUST NOT use filter expressions or recursive descent. | — |
| `K-001` | K | tested | Final states MUST NOT have outgoing transitions. | K-001-negative-final-transitions.json |
| `K-002` | K | draft | Compound states MUST have `initialState` and `states`. | — |
| `K-003` | K | draft | Parallel states MUST have `regions`. | — |
| `K-004` | K | draft | `cancellationPolicy` MUST only appear on `parallel` states. | — |
| `K-005` | K | draft | `historyState` MUST only appear on `compound` states. | — |
| `K-006` | K | draft | Transition `target` MUST reference an existing state. | — |
| `K-007` | K | draft | Event names MUST NOT use the `$` prefix (kernel-reserved). | — |
| `K-008` | K | draft | Parallel state outgoing transitions MUST use `$join` as event. | — |
| `K-009` | K | draft | Actor identifiers MUST be unique. | — |
| `K-014` | K | draft | Milestone `id` values MUST be unique. | — |
| `K-015` | K | draft | `setData` path MUST reference a declared `caseFile.fields` entry. | — |
| `K-016` | K | tested | `lifecycle.initialState` MUST key into `lifecycle.states`; compound `initialState` MUST key into its substates; `region.initialState` MUST key into `region.states`; forEach `body` is walked recursively. | — |
| `K-021` | K | draft | Provenance `actorId` MUST reference a declared actor. | — |
| `K-022` | K | draft | Digest present implies algorithm recorded in extensions. | — |
| `K-029` | K | draft | `startTimer` MUST specify exactly one of `duration` or `deadline`. | — |
| `K-030` | K | draft | Extension keys MUST be `x-` prefixed. | — |
| `K-031` | K | draft | `transition.actor` MUST match `actors[].id` pattern and name a declared `actors[].id`. | — |
| `K-032` | K | draft | `lifecycle.initialState` (root) and compound `initialState` MUST name a key in the relevant `states` map; diagnostic `path` distinguishes cases. | — |
| `K-048` | K | draft | Non-standard case relationship `type` values MUST use `x-` prefix. | — |
| `K-051` | K | tested | DecisionTableGuard ref/outputColumn/inputBindings MUST resolve. | K-051-resolved-decision-table-guard.json, K-051-negative-unresolved-table-ref.json, K-051-negative-unresolved-output-column.json, K-051-negative-missing-input-binding.json |
| `K-053` | K | tested | DecisionTable cell-shape: input cells boolean; transition-guard outputColumn boolean-typed; no collect hit policy on guards. | K-053-boolean-output-column.json, K-053-negative-non-boolean-output.json, K-053-negative-collect-hit-policy-on-guard.json |
| `K-FOREACH-001` | K-FOREACH | tested | ForEach states MUST declare a non-empty `collection` FEL expression. | — |
| `K-FOREACH-002` | K-FOREACH | tested | ForEach states MUST declare a `body` State to execute per iteration. | — |
| `K-FOREACH-003` | K-FOREACH | tested | ForEach `concurrency` MUST be at least 1 when present. | — |
| `K-FOREACH-004` | K-FOREACH | tested | Iteration fields (collection, itemVariable, indexVariable, concurrency, breakCondition, outputPath, mergeStrategy, body) are valid only on `foreach`-typed states. | — |
| `SCHEMA-DOC-001` | SCHEMA-DOC | draft | Schema leaf properties MUST carry sufficient `description` and `examples`. | — |
| `SCHEMA-OPEN-001` | SCHEMA-OPEN | tested | Open string leaves MUST declare `x-wos.openStringKind` unless `enum`/`const`/`pattern` already close the leaf. | — |
| `WOS-AI-NATIVE-001` | WOS-AI-NATIVE | draft | Native AI declarations must load through the shared owning policy model. | — |
| `WOS-EMBED-IDENTITY-001` | WOS-EMBED-IDENTITY | tested | Embedded blocks MUST NOT declare independent `url` or `version`. The enclosing $wosWorkflow envelope is the sole identity. | — |
| `WOS-EMBED-TARGET-001` | WOS-EMBED-TARGET | tested | Embedded blocks (governance, agents, aiOversight, signature, custody, advanced, assurance) MUST NOT declare `targetWorkflow`. | — |
| `WOS-SIDECAR-TARGET-001` | WOS-SIDECAR-TARGET | tested | Sidecar documents ($wosDelivery, $wosOntologyAlignment) MUST declare `targetWorkflow` as a non-empty workflow URI. | — |
| `WOS-VER-LEVEL-001` | WOS-VER-LEVEL | tested | Agents declaring `fallbackChain` SHOULD have at least one `verificationLevel` declared on output bindings. | — |

**T1 total: 50** (0 LoadBearing, 0 Stable, 14 Tested, 36 Draft)

---

## Tier 2 — Cross-Document + FEL AST Rules (`wos-lint --project`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `AG-008` | AG | draft | Side-effect tools at `autonomous` autonomy MUST declare a `sideEffectPolicy`. | — |
| `AG-010` | AG | draft | Verifiable constraints MUST satisfy all SMT subset restrictions (parse failures). | — |
| `AG-011` | AG | draft | `let` bindings in verifiable expressions MUST NOT be recursive. | — |
| `AG-012` | AG | draft | Quantifiers MUST quantify over finite domains (non-standard every/some arity). | — |
| `AG-013` | AG | draft | Verifiable arithmetic MUST be linear (no variable*variable products). | — |
| `AG-014` | AG | draft | Verifiable subset MUST NOT include extension function calls. | — |
| `AG-017` | AG | draft | Shadow mode is RECOMMENDED for rights-impacting workflows. | — |
| `AI-007` | AI | draft | Cascading autonomous agents MUST be declared via `cascadingInvocations`. | — |
| `AI-018` | AI | draft | `autonomous` actions MUST have associated deontic constraints. | — |
| `AI-020` | AI | draft | `supervisory` actions MUST define `reviewWindow`. | — |
| `AI-023` | AI | draft | Every agent invocation MUST have a reachable path to completion without any agent. | — |
| `AI-024` | AI | draft | Escalation conditions MUST be valid FEL referencing `@agent` context. | — |
| `AI-026` | AI | draft | Escalation MUST have `escalationExpiry`; agent reverts when expired. | — |
| `AI-031` | AI | draft | Agent output contract MUST apply same rules as human-facing form. | — |
| `AI-042` | AI | draft | Agent config MUST disclose training data characteristics. | — |
| `AI-043` | AI | draft | Agent config MUST disclose optimization objective. | — |
| `AI-046` | AI | draft | `rights-impacting` workflows MUST have `discloseThatAgentAssisted: true`. | — |
| `AI-056` | AI | draft | Autonomy is an action-site property, not an agent property. | — |
| `AI-057` | AI | draft | Capability `preconditions` entries MUST be valid FEL. | — |
| `AI-058` | AI | tested | Capability `preconditions` AST root MUST be boolean-shaped (no truthy coercion). | — |
| `DM-002` | DM | draft | Rights/safety workflows SHOULD follow shadow/canary/production sequence. | — |
| `G-001` | G | draft | Due process MUST be enforced for `rights-impacting` or `safety-impacting` kernels. | — |
| `G-003` | G | draft | Notice MUST include specific determination, reason codes, and appeal instructions. | — |
| `G-004` | G | draft | Explanation level MUST be `individualized` when kernel impact is `rights-impacting`. | — |
| `G-005` | G | draft | Adverse decisions MUST include positive and negative counterfactuals when rights-impacting. | — |
| `G-008` | G | draft | `continuationOfServices: true` requires kernel topology to freeze adverse impacts. | — |
| `G-009` | G | draft | Transitions tagged `adverse-decision` MUST trigger due process policy enforcement. | — |
| `G-011` | G | draft | Review protocol tags MUST match tags declared in the target kernel. | — |
| `G-014` | G | draft | Reasoning tier MUST be present for `determination`-tagged transitions. | — |
| `G-015` | G | draft | Counterfactual tier MUST be present for `adverse-decision` transitions in rights-impacting workflows. | — |
| `G-022` | G | draft | `excludedOwner` MUST override `potentialOwner` when actor appears in both. | — |
| `G-023` | G | draft | SLA evaluation SHOULD use business calendar when BC sidecar is present. | — |
| `G-024` | G | draft | Determination-tagged transitions MUST verify the actor has valid delegation. | — |
| `G-027` | G | draft | Sub-delegation MUST respect `maxDelegationDepth`. | — |
| `G-028` | G | draft | Hold policies MUST attach to kernel states tagged `hold`. | — |
| `G-029` | G | draft | `resumeTrigger` event name MUST reference an event in the target kernel. | — |
| `G-031` | G | draft | `resolutionDateRef` MUST refer to a field path in the kernel's case state. | — |
| `G-033` | G | draft | Parameter `values` SHOULD cover every resolution date (no coverage gap). | — |
| `G-034` | G | draft | `targetWorkflow` MUST match the `url` of the target kernel document. | — |
| `G-035` | G | draft | `targetGovernance` MUST reference a valid governance document. | — |
| `G-036` | G | draft | `independenceConstraint` MUST describe a mechanism preventing self-review. | — |
| `G-040` | G | draft | `consistency` assertions `referenceStage` MUST refer to an earlier pipeline stage. | — |
| `G-041` | G | draft | Pipeline-stage assertion ids MUST exist in the targeted assertion library. | — |
| `G-042` | G | draft | FEL expressions in assertion `expression` fields MUST be syntactically valid. | — |
| `G-043` | G | draft | FEL expressions in delegation `conditions` MUST be syntactically valid. | — |
| `G-046` | G | draft | Delegation `delegator`/`delegate` MUST reference declared kernel actors. | — |
| `G-053` | G | draft | Sub-delegation is only permitted if the original delegation explicitly allows it. | — |
| `G-056` | G | draft | Binding `resolutionDateRef` MUST reference a field path in kernel case state. | — |
| `G-060` | G | draft | Business Calendar target requires SLA evaluation in business days. | — |
| `G-063` | G | draft | Notification template keys MUST resolve to a template in a targeting sidecar. | — |
| `G-066` | G | draft | BreachPolicy escalationStepId MUST resolve within the task pattern. | — |
| `K-010` | K | draft | createTask `assignTo` MUST reference a declared kernel actor. | — |
| `K-012` | K | draft | Guards MUST be valid FEL expressions. | — |
| `K-013` | K | draft | Milestone conditions MUST be valid FEL expressions. | — |
| `K-017` | K | draft | FEL guards MUST NOT reference related case state. | — |
| `K-019` | K | draft | FEL functions MUST be declared built-ins or registered extensions. | — |
| `K-037` | K | draft | Fail-fast `$join` fires only on an error final state. | — |
| `K-049` | K | load_bearing | Continuous-mode kernels MUST NOT contain `setData` → guard dependency cycles. | k-049-load-bearing-self-loop.json, k-049-load-bearing-two-node-cycle.json |
| `K-050` | K | draft | Final state `outcomeCode` MUST NOT duplicate any entry in `tags` (Kernel S4.3). | — |
| `K-052` | K | tested | DecisionTable rows for hitPolicy=unique/priority MUST be pairwise disjoint (or have distinct priorities under priority). | K-052-disjoint-unique-rows.json, K-052-negative-overlapping-unique-rows.json, K-052-negative-priority-tie.json |
| `K-EXT-002` | K-EXT | tested | `x-wos-*` namespace is reserved for future normative WOS use. | x-wos-reserved-warn.json, x-vendor-custom-ok.json |
| `SIG-001` | SIG | draft | Signature Profile `targetWorkflow.url` MUST match the loaded kernel URL. | — |
| `SIG-002` | SIG | draft | Signature Profile roles MUST reference declared kernel actors. | — |
| `SIG-003` | SIG | draft | Signature Profile roles MUST bind to human kernel actors. | — |
| `SIG-004` | SIG | draft | Signature role authenticationPolicyKey values MUST resolve. | — |
| `SIG-005` | SIG | draft | Signature signing-step roleId values MUST resolve. | — |
| `SIG-006` | SIG | draft | Signature signing-step documentId values MUST resolve. | — |
| `SIG-007` | SIG | draft | Signature signing-step dependencies MUST resolve and MUST NOT cycle. | — |
| `SIG-008` | SIG | draft | Routed signing guards MUST parse as valid FEL. | — |
| `SIG-009` | SIG | draft | Signature lifecycle tags SHOULD appear in the target kernel. | — |
| `SIG-010` | SIG | draft | Signature reminder and expiry events MUST map to kernel events. | — |
| `SIG-011` | SIG | draft | SignatureAffirmation evidence inputs MUST be satisfiable. | — |
| `SIG-012` | SIG | draft | Signature Profile fields MUST follow Ref/Key/Id naming conventions. | — |
| `VR-003` | VR | draft | `counterexample` MUST be present when result is `proven-unsafe`. | — |
| `WOS-AGENT-XREF-001` | WOS-AGENT-XREF | tested | Every actor with `type=='agent'` MUST have a matching `agents[].id`. | — |
| `WOS-AI-FEL-001` | WOS-AI-FEL | draft | Every native output-policy predicate must be valid FEL. | — |
| `WOS-SIG-COVER-001` | WOS-SIG-COVER | tested | Signature-gated transitions MUST be covered by a `signature.signers[]` entry. | — |

**T2 total: 77** (1 LoadBearing, 0 Stable, 5 Tested, 71 Draft)

---

## Tier 3 — Dynamic Runtime Rules (`wos-conformance`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `AI-001` | AI | draft | Processor MUST implement agent registration (AI S3). | — |
| `AI-002` | AI | draft | Processor MUST implement the confidence framework (AI S7). | — |
| `AI-004` | AI | draft | Processor MUST delegate Formspec evaluation to a conformant processor. | — |
| `AI-050` | AI | draft | Assist Governance Proxy MUST NOT modify conformance requirements. | — |
| `AI-AUTO-001` | AI-AUTO | draft | Escalation expiry MUST revoke elevated autonomy and emit an autonomyDemotion record (AI S5.5). | — |
| `AI-AUTO-002` | AI-AUTO | draft | Drift-alert thresholds with action=demoteToAssistive MUST emit autonomyDemotion + driftReclassification and reroute the event through escalation (AI S5.5). | — |
| `G-051` | G | draft | Governance Basic processor MUST enforce due process and review protocols. | — |
| `G-052` | G | draft | Governance Complete processor MUST enforce all normative sections. | — |
| `K-DET-001` | K-DET | tested | Determination-tagged transitions MUST emit the pre-transition case-file snapshot in Facts-tier provenance. | native-determination-snapshot.json |

**T3 total: 9** (0 LoadBearing, 0 Stable, 1 Tested, 8 Draft)

---

## Summary

| Tier | Total | LoadBearing | Stable | Tested | Draft |
|------|-------|-------------|--------|--------|-------|
| T1 | 50 | 0 | 0 | 14 | 36 |
| T2 | 77 | 1 | 0 | 5 | 71 |
| T3 | 9 | 0 | 0 | 1 | 8 |
| **Total** | **136** | **1** | **0** | **20** | **115** |

