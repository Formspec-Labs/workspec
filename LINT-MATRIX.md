# WOS Verification Matrix

> 🚨 **UNRECONCILED (2026-04-17).** This matrix claims 197 rules across T1/T2/T3. The code-side rule registry seeded at commit `1f8eae5` contains only 97 entries (91 wos-lint + 6 wos-conformance). The ~100-rule gap is expected to be rules that exist in prose here but are not yet reified in `crates/wos-lint/src/rules/registry.rs` / `crates/wos-conformance/src/rules.rs`. Reconciliation is §4.2 Task 2 of [`thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md`](thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md). Until then, treat the 197 count as an aspirational normative catalog, not a coverage metric.

197 normative constraints from WOS specs that JSON Schema cannot enforce. Each constraint maps to one of three verification tiers:

```text
┌─────────────────────────────────────────────────────────────────┐
│  Tier 1: wos-lint (static)           37 rules                  │
│  Single-document structural checks. Pattern matching and graph  │
│  walks over the JSON document tree. No parsing, no cross-doc.   │
├─────────────────────────────────────────────────────────────────┤
│  Tier 2: wos-lint --project (cross)  55 rules                  │
│  Multi-document resolution + FEL AST analysis. Loads a project  │
│  directory (kernel + governance + AI + sidecars), resolves       │
│  cross-references, parses FEL expressions into ASTs.            │
│  (42 cross-doc + 12 AST)                                       │
├─────────────────────────────────────────────────────────────────┤
│  Tier 3: wos-conformance (dynamic)   105 rules                 │
│  Event-driven test fixtures. Feeds event sequences through      │
│  WosRuntime, asserts on observed state transitions, provenance  │
│  records, timer behavior, compensation ordering, deontic        │
│  enforcement, and autonomy caps.                                │
└─────────────────────────────────────────────────────────────────┘
```

**Column legend:**

- **Tier** — which verification tool checks this rule
  - `T1` — `wos-lint` static, single-document
  - `T2-xdoc` — `wos-lint --project`, cross-document resolution
  - `T2-ast` — `wos-lint --project`, FEL AST analysis
  - `T3` — `wos-conformance`, dynamic fixture
- **Category** — semantic domain of the constraint
- **Tested?** — which test files exercise this rule
  - `T1` — `crates/wos-lint/tests/tier1_rules.rs`
  - `T2` — `crates/wos-lint/tests/tier2_rules.rs`
  - `AST` — `crates/wos-lint/src/rules/fel_analysis.rs` `#[cfg(test)]` module
  - `E2E` — `crates/wos-conformance/tests/kernel_conformance.rs`, `profile_conformance.rs`, `processor_conformance.rs`, `provenance_tests.rs`, or `stub_integration.rs`
  - `—` — no test yet
  - Multiple types separated by `+` (e.g., `T1+E2E`)

---

## Kernel + Lifecycle Detail + Correspondence Metadata

| ID | Section | Rule | Why schema can't enforce | Tier | Category | Tested? |
|---|---|---|---|---|---|---|
| K-001 | Kernel S4.3 | Final states MUST NOT have outgoing transitions. | Conditional validation on `type` omitted from schema. | T1 | lifecycle-soundness | T1 |
| K-002 | Kernel S4.3 / S13.3 | Compound states MUST have `initialState` and `states`. | Conditional required properties. | T1 | lifecycle-soundness | T1 |
| K-003 | Kernel S4.3 / S13.3 | Parallel states MUST have `regions`. | Conditional required properties. | T1 | lifecycle-soundness | T1 |
| K-004 | Kernel S4.3 | `cancellationPolicy` MUST only appear on `parallel` states. | Property restricted to state type. | T1 | lifecycle-soundness | T1 |
| K-005 | Kernel S4.3 | `historyState` MUST only appear on `compound` states. | Property restricted to state type. | T1 | lifecycle-soundness | T1 |
| K-006 | Kernel S4.5 | Transition `target` MUST reference an existing state. | Cross-path reference resolution. | T1 | cross-reference | T1 |
| K-007 | Kernel S4.10 / S13.3 | Event names MUST NOT use the `$` prefix (kernel-reserved). | Event name pattern not enforced. | T1 | lifecycle-soundness | T1 |
| K-008 | Kernel S4.8 | Parallel state outgoing transitions MUST use `$join` as event. | Conditional event constraint by parent type. | T1 | lifecycle-soundness | T1+E2E |
| K-009 | Kernel S3.3 | Actor identifiers MUST be unique. | Uniqueness of nested `actors[].id` values is not enforced by schema. | T1 | actor-consistency | T1 |
| K-010 | Kernel S3.3 | createTask `assignTo` MUST reference a declared kernel actor. | Cross-path reference. | T2-xdoc | actor-consistency | tier2_rules.rs |
| K-011 | Kernel S4.2 / LCD S2.1 | Same document + events MUST produce same transitions (determinism). | Runtime behavioral property. | T3 | determinism | E2E |
| K-012 | Kernel S4.6 | Guards MUST be valid FEL expressions. | Opaque string in schema. | T2-ast | expression-validity | AST |
| K-013 | Kernel S4.13 | Milestone conditions MUST be valid FEL expressions. | Opaque string in schema. | T2-ast | expression-validity | AST |
| K-014 | Kernel S4.13 | Milestone `id` values MUST be unique. | Nested property uniqueness. | T1 | cross-reference | T1 |
| K-015 | Kernel S5.2 | `setData` path MUST reference a declared `caseFile.fields` entry. | Cross-path reference. | T1 | cross-reference | T1 |
| K-016 | Kernel S5.4 | Mutation history MUST be append-only. | Temporal invariant. | T3 | provenance | E2E |
| K-017 | Kernel S5.5 | FEL guards MUST NOT reference related case state. | FEL AST variable analysis. | T2-ast | expression-validity | AST |
| K-018 | Kernel S5.5 | Case relationship changes MUST produce provenance. | Runtime obligation. | T3 | provenance | E2E |
| K-019 | Kernel S7.4 | FEL MUST use only built-in and extension functions. | FEL function catalog cross-ref. | T2-ast | expression-validity | AST |
| K-020 | Kernel S8.2 | Every state/case mutation MUST produce Facts tier provenance. | Runtime completeness. | T3 | provenance | E2E |
| K-021 | Kernel S8.2 | Provenance `actorId` MUST reference a declared actor. | Cross-path reference. | T1 | actor-consistency | T1 |
| K-022 | Kernel S8.3 | Digest present implies algorithm recorded in extensions. | Conditional inter-field dependency. | T1 | provenance | T1 |
| K-023 | Kernel S9.1 (G1) | Non-terminal instances MUST resume after crash. | Runtime durability. | T3 | lifecycle-soundness | E2E |
| K-024 | Kernel S9.1 (G3) | Non-deterministic output MUST persist before advancing state. | Runtime execution order. | T3 | determinism | E2E |
| K-025 | Kernel S9.1 (G4) | Timers MUST survive restarts; tolerance > duration is violation. | Runtime timing. | T3 | timer | E2E |
| K-026 | Kernel S9.3 | IdempotencyKey MUST deduplicate invocations. | Runtime deduplication. | T3 | determinism | E2E |
| K-027 | Kernel S9.5 / LCD S5.2 | Compensation log MUST be append-only. | Runtime data structure. | T3 | compensation | E2E |
| K-028 | Kernel S9.6 | Instance migration MUST produce provenance. | Runtime obligation. | T3 | provenance | E2E |
| K-029 | Kernel S9.7 / LCD S6.2 | `startTimer` MUST specify exactly one of `duration` or `deadline`. | Mutual exclusivity not enforced. | T1 | timer | T1 |
| K-030 | Kernel S10.6 | Extension keys MUST be `x-` prefixed. | Pattern enforcement. | T1 | cross-reference | T1 |
| K-031 | Kernel S11.1 | Contract validation MUST produce structured results. | Runtime integration contract. | T3 | lifecycle-soundness | E2E |
| K-032 | Kernel S12 | Lifecycle state MUST be separated from case state. | Architectural invariant. | T3 | state-type-semantics | E2E |
| K-033 | LCD S2.1 | Guard evaluation: document order, first match wins. | Runtime evaluation rule. | T3 | determinism | E2E |
| K-034 | LCD S2.4 | Compound entry: initialState if target, skip if descendant. | Runtime entry-path computation. | T3 | lifecycle-soundness | E2E |
| K-035 | LCD S3.4 | History cleared on parent exit or region cancellation. | Runtime clearing rule. | T3 | lifecycle-soundness | E2E |
| K-036 | LCD S4.1 | All parallel regions initialized atomically. | Runtime concurrency. | T3 | lifecycle-soundness | E2E |
| K-037 | LCD S4.3 | Fail-fast `$join` fires only on error final state. | Transition-graph reachability. | T2-xdoc | lifecycle-soundness | tier2_rules.rs |
| K-038 | LCD S4.4 | Cancelled region timers MUST be cancelled. | Runtime timer scoping. | T3 | timer | E2E |
| K-039 | LCD S5.4 | Compensation in reverse of forward completion order. | Runtime ordering. | T3 | compensation | E2E |
| K-040 | LCD S5.5 | Pivot step MUST NOT receive compensation. | Runtime identification. | T3 | compensation | E2E |
| K-041 | LCD S5.8 | Inner scope compensation MUST NOT trigger outer. | Runtime scope boundary. | T3 | compensation | E2E |
| K-042 | LCD S5.9 | `$compensation.complete` processed like any event. | Runtime dispatch. | T3 | compensation | E2E |
| K-043 | LCD S6.4 | Re-entered state timers: cancel and recreate. | Runtime reset rule. | T3 | timer | E2E |
| K-044 | LCD S6.5 | Timer events routed to creating region only. | Runtime scoping. | T3 | timer | E2E |
| K-045 | LCD S6.6 | Timer tolerance > duration is violation. | Runtime timing. | T3 | timer | E2E |
| K-046 | LCD S6.7 | Timer lifecycle MUST produce provenance records. | Runtime obligation. | T3 | provenance | E2E |
| K-047 | Kernel S5.5 | Case relationships MUST NOT affect lifecycle evaluation (metadata only). | Semantic constraint on processor behavior. | T3 | lifecycle-soundness | E2E |
| K-048 | Kernel S5.5 | Case relationship `type` extensibility MUST use `x-` prefix for non-standard values. | Schema enum does not enforce prefix on extension values. | T1 | cross-reference | T1 |
| CM-001 | CorrMeta S1.2 | Entry template `id` values MUST be unique within the sidecar. | Uniqueness of nested `entryTemplates[].id` values is not enforced by schema. | T1 | correspondence-validity | T1 |

**Kernel + Correspondence:** 49 constraints — 17 T1, 6 T2, 26 T3. **Tested: 49 of 49** (17 T1, 4 AST, 2 T2, 26 E2E).

---

## Governance + Sidecars

| ID | Section | Rule | Why schema can't enforce | Tier | Category | Tested? |
|---|---|---|---|---|---|---|
| G-001 | WG S3 / S3.7 | Due process requirements MUST be enforced when kernel declares `impactLevel` of `rights-impacting` or `safety-impacting`. | Cross-document: schema cannot read the kernel's `impactLevel`. | T2-xdoc | due-process-completeness | T2 |
| G-002 | WG S3.2 | Affected individual MUST receive notice before the adverse decision takes effect. | Runtime execution ordering. | T3 | due-process-completeness | E2E |
| G-003 | WG S3.2 | Notice MUST include specific determination, individualized reason codes, and appeal instructions. | Schema validates structure but cannot verify content is individualized. | T2-xdoc | due-process-completeness | tier2_rules.rs |
| G-004 | WG S3.3 | Explanation level MUST be `individualized` when kernel's `impactLevel` is `rights-impacting`. | Cross-document conditional requirement. | T2-xdoc | cross-reference | T2 |
| G-005 | WG S3.4 | Adverse decisions MUST include positive and negative counterfactuals when `impactLevel` is `rights-impacting`. | Cross-document condition. | T2-xdoc | due-process-completeness | T2 |
| G-006 | WG S3.5 | Appeal MUST be reviewed by an adjudicator independent of the original determination. | Runtime actor identity verification. | T3 | separation-of-duties | E2E |
| G-007 | WG S3.5 | Filing an appeal MUST produce a provenance record. | Runtime requirement. | T3 | pipeline-validity | E2E |
| G-008 | WG S3.6 | When `continuationOfServices` is true, workflow topology MUST freeze adverse impacts during appeal. | Cannot verify kernel topology implements continuation. | T2-xdoc | cross-reference | tier2_rules.rs |
| G-009 | WG S3.7 | When a transition tagged `adverse-decision` fires, processor MUST enforce due process policy. | Runtime tag-matching and enforcement. | T2-xdoc | tag-matching | T2 |
| G-010 | WG S4.2 | `independentFirst` MUST enforce reviewer records independent assessment before any recommendation is accessible. | Runtime UI ordering constraint. | T3 | due-process-completeness | E2E |
| G-011 | WG S4.3 | Review protocol tags MUST match tags that actually appear in the target kernel document. | Cross-document tag resolution. | T2-xdoc | tag-matching | T2 |
| G-012 | WG S5.5 | Each pipeline stage MUST record inputs, outputs, and gate results in provenance. | Runtime execution requirement. | T3 | pipeline-validity | E2E |
| G-013 | WG S5.5 | Pipeline risk profile MUST be determined by the weakest validation gate. | Semantic computation over gate types. | T3 | pipeline-validity | E2E |
| G-014 | WG S6.5 | Reasoning tier MUST be present for all `determination`-tagged transitions. | Cross-document + runtime. | T2-xdoc | tag-matching | T2 |
| G-015 | WG S6.5 | Counterfactual tier MUST be present for `adverse-decision` transitions in `rights-impacting` workflows. | Cross-document + runtime. | T2-xdoc | cross-reference | T2 |
| G-016 | WG S7.1 | Configurable percentage of decisions MUST be randomly selected for quality review. | Runtime enforcement. | T3 | pipeline-validity | E2E |
| G-017 | WG S7.2 | Reviewer MUST NOT be the same actor who made the original decision. | Runtime actor identity constraint. | T3 | separation-of-duties | E2E |
| G-018 | WG S7.3 | Override MUST include structured rationale, authority verification, and supporting evidence. | Runtime content requirements. | T3 | separation-of-duties | E2E |
| G-019 | WG S7.3 | Override records are immutable provenance entries. | Storage/runtime property. | T3 | pipeline-validity | E2E |
| G-020 | WG S8.2 | Every rejection MUST record: which gate failed, the input, the threshold, what would pass. | Runtime provenance content requirement. | T3 | pipeline-validity | E2E |
| G-021 | WG S10.1 | All task state transitions MUST be recorded in provenance. | Runtime execution requirement. | T3 | pipeline-validity | E2E |
| G-022 | WG S10.2 | When actor appears in both `potentialOwner` and `excludedOwner`, `excludedOwner` MUST override. | Processor-side semantics. | T2-xdoc | separation-of-duties | T2 |
| G-023 | WG S10.3 | SLA evaluation MUST use business calendar days when a Business Calendar sidecar is present. | Cross-document runtime dependency. | T2-xdoc | temporal-resolution | tier2_rules.rs |
| G-024 | WG S11.4 | When `determination`-tagged transition fires, processor MUST verify acting actor has valid delegation. | Runtime identity check. | T2-xdoc | delegation-validity | tier2_rules.rs |
| G-025 | WG S11.4 | Determinations without valid delegation are conformance errors. | Runtime actor-identity check. | T3 | delegation-validity | E2E |
| G-026 | WG S11.4 | Delegation used MUST be referenced in provenance record. | Runtime content requirement. | T3 | delegation-validity | E2E |
| G-027 | WG S11.5 | Sub-delegation MUST respect `maxDelegationDepth`. | Requires traversing delegation chain. | T2-xdoc | delegation-validity | T2 |
| G-028 | WG S12 | Hold policies MUST attach to kernel states tagged `hold`. | Cross-document tag resolution. | T2-xdoc | tag-matching | T2 |
| G-029 | WG S12.2 | `resumeTrigger` event name MUST correspond to an event in the target kernel document. | Cross-document event resolution. | T2-xdoc | cross-reference | T2 |
| G-030 | WG S12.4 | On entering `hold` state, processor SHOULD start timer; on resume trigger, timer MUST be cancelled. | Runtime timer management. | T3 | hold-validity | E2E |
| G-031 | WG S13.2 | `resolutionDateRef` MUST refer to a field path that exists in the kernel's case state. | Cross-document path resolution. | T2-xdoc | temporal-resolution | T2 |
| G-032 | WG S13.2 | Resolution MUST select the most recent entry whose `effectiveDate` is on or before resolution date. | Runtime resolution algorithm. | T3 | temporal-resolution | E2E |
| G-033 | WG S13.2 / PP S1.4 | When no entry covers the resolution date, behavior is undefined. | Schema allows any ordering; cannot detect coverage gaps. | T2-xdoc | temporal-resolution | T2 |
| G-034 | WG S14 | `targetWorkflow` MUST match `url` of the target kernel document. | Cross-document URI resolution. | T2-xdoc | cross-reference | T2 |
| G-035 | DP S1 | `targetGovernance` MUST reference a valid governance document. | Cross-document type resolution. | T2-xdoc | cross-reference | T2 |
| G-036 | DP S1.3 | `independenceConstraint` MUST encode a mechanism preventing the original decision-maker from reviewing. | Prose constraint on string content. | T2-xdoc | separation-of-duties | tier2_rules.rs |
| G-037 | AL S1.1 | Assertion `id` values MUST be unique within the library. | Uniqueness on nested property across array. | T1 | assertion-validity | T1 |
| G-038 | AL S1.2 | When type is `arithmetic`/`range`/`temporal`, `expression` SHOULD be present. | Conditional field presence based on type. | T1 | assertion-validity | T1 |
| G-039 | AL S1.2 | When type is `source-grounded`/`consistency`, `fields` array SHOULD be present. | Conditional field presence based on type. | T1 | assertion-validity | T1 |
| G-040 | AL S1.2 | When type is `consistency`, `referenceStage` MUST refer to an earlier pipeline stage. | Cross-document reference to governance pipeline. | T2-xdoc | cross-reference | T2 |
| G-041 | AL S1.3 | Every assertion `id` referenced by a pipeline stage MUST exist in the targeted library. | Cross-document reference integrity. | T2-xdoc | cross-reference | T2 |
| G-042 | AL S1.2 | FEL expressions in assertion `expression` fields MUST be syntactically valid. | Requires FEL parser. | T2-ast | expression-validity | AST |
| G-043 | WG S11.3 | FEL expressions in delegation `conditions` MUST be syntactically valid. | Requires FEL parser. | T2-ast | expression-validity | AST |
| G-044 | WG S11.2 | Delegation `expirationDate` MUST be strictly after `effectiveDate`. | Cross-field temporal ordering. | T1 | delegation-validity | T1 |
| G-045 | WG S11.2 | `revokedDate` MUST be on or after `effectiveDate`. | Cross-field temporal ordering. | T1 | delegation-validity | T1 |
| G-046 | WG S11.4 | `delegator` and `delegate` MUST correspond to actors in the target kernel document. | Cross-document actor resolution. | T2-xdoc | delegation-validity | T2 |
| G-047 | PP S1.3 | Parameter `values` entries MUST be in ascending `effectiveDate` order. | Array ordering constraint. | T1 | temporal-resolution | T1 |
| G-048 | PP S1.5.2 | Binding `id` MUST match the key under which it appears in the `bindings` map. | Object key must equal property value. | T1 | cross-reference | T1 |
| G-049 | PP S1.5.4 | Processors MUST NOT alter resolution mechanism based on `bindingType`. | Processor behavior rule. | T3 | pipeline-validity | E2E |
| G-050 | PP S1.4 | Resolved parameter value MUST be type-consistent with declared `type`. | Type validation across DateValue.value and ParameterDefinition.type. | T1 | assertion-validity | T1 |
| G-051 | WG S2.1 | Governance Basic processor MUST enforce due process (S3) and review protocols (S4). | Processor conformance claim. | T3 | due-process-completeness | E2E |
| G-052 | WG S2.1 | Governance Complete processor MUST enforce all normative sections. | Processor conformance claim. | T3 | pipeline-validity | E2E |
| G-053 | WG S11.5 | Sub-delegation MUST only be permitted if the original delegation explicitly allows it. | Requires traversing delegation chain and checking per-delegation permissions. | T2-xdoc | delegation-validity | T2 |
| G-054 | WG S12.4 | When `resumeTrigger` event arrives before hold timer fires, the timer MUST be cancelled. | Runtime timer management behavior. | T3 | hold-validity | E2E |
| G-055 | WG S12.2 | `expectedDuration` MUST be a valid ISO 8601 duration or the literal string `"indefinite"`. | Union type validation (duration or specific string) not enforced by schema. | T1 | hold-validity | T1 |
| G-056 | PP S1.5.2 | Binding `resolutionDateRef` MUST reference a field path that exists in the kernel's case state. | Cross-document path resolution (same as G-031 but for bindings). | T2-xdoc | temporal-resolution | T2 |
| G-057 | PP S1.5.3 | Binding `values` entries MUST be in ascending `effectiveDate` order. | Array ordering constraint (same as G-047 but for bindings). | T1 | temporal-resolution | T1 |
| G-058 | BC S3.3 / S4.1 | Each Holiday entry MUST specify exactly one of `date` or `rule`. | Mutual exclusivity across two optional fields. | T1 | calendar-validity | T1 |
| G-059 | BC S5.2 | Operating hours `end` MUST be strictly after `start`. | Cross-field temporal ordering within OperatingHours. | T1 | calendar-validity | T1 |
| G-060 | BC S6.1 | When a Business Calendar sidecar targets a workflow, SLA evaluation MUST use business days. | Cross-document sidecar presence check + runtime behavior. | T2-xdoc | temporal-resolution | tier2_rules.rs |
| G-061 | BC S8.1 | Processor MUST ignore an expired calendar (`expirationDate` in the past). | Runtime temporal check. | T3 | calendar-validity | E2E |
| G-062 | NT S4.4 | Adverse-decision templates MUST include sections addressing determination, reason codes, appeal rights, and appeal instructions. | Content-level validation of section coverage by category; lint uses id / `contentType` heuristics (see `check_adverse_decision_template_sections` in `tier1.rs`). | T1 | notification-validity | T1 |
| G-063 | NT S5.1 | `notificationTemplateRef` and `noticeTemplateRef` values MUST resolve to a template key in a Notification Template sidecar targeting the same workflow. | Cross-document reference resolution. | T2-xdoc | cross-reference | tier2_rules.rs |
| G-064 | NT S5.3 | Processor MUST NOT send notification when `requiredVariables` are missing from rendering context. | Runtime variable resolution check. | T3 | notification-validity | E2E |
| G-065 | NT S4.1 | Section `id` values MUST be unique within a template. | Nested property uniqueness within array. | T1 | notification-validity | T1 |

**Governance:** 65 constraints — 14 T1, 29 T2, 22 T3. **Tested: 65 of 65** (14 T1, 2 AST, 27 T2, 22 E2E).

---

## AI Integration + Advanced Governance

| ID | Section | Rule | Why schema can't enforce | Tier | Category | Tested? |
|---|---|---|---|---|---|---|
| AI-001 | AI S2.2 | Processor MUST implement agent registration (S3). | Processor conformance claim. | T3 | agent-consistency | E2E |
| AI-002 | AI S2.2 | Processor MUST implement confidence framework (S7). | Processor conformance claim. | T3 | confidence-validity | E2E |
| AI-003 | AI S2.2 | Processor MUST validate fallback chains at load time, rejecting cycles or missing terminal actions. | Cycle detection requires graph traversal. | T1 | fallback-chain-validity | T1 (AI-041) |
| AI-004 | AI S2.2 / S6.2 | Processor MUST delegate Formspec evaluation to a conformant processor (Core S1.4). | Runtime architectural requirement. | T3 | cross-layer-reference | E2E |
| AI-005 | AI S3.7 | Agents MUST NOT override human decisions. | Runtime workflow state knowledge required. | T3 | autonomy-constraint | E2E |
| AI-006 | AI S3.7 | Agent provenance MUST include model identifier, version, confidence, input summary. | Runtime output completeness. | T3 | agent-consistency | E2E |
| AI-007 | AI S3.7 | Cascading autonomous agents MUST be declared via `cascadingInvocations`. | Cross-referencing runtime invocation graph against declarations. | T2-xdoc | autonomy-constraint | T2 |
| AI-008 | AI S3.7 | Actor type is immutable for a given action. | Runtime execution constraint. | T3 | agent-consistency | E2E |
| AI-009 | AI S4.2 | Permission bounds evaluated against live output. | Runtime FEL evaluation. | T3 | deontic-validity | E2E |
| AI-010 | AI S4.3 | Prohibition condition evaluated against live output. | Runtime FEL evaluation. | T3 | deontic-validity | E2E |
| AI-011 | AI S4.4 | Obligation requirement evaluated against live output. | Runtime FEL evaluation. | T3 | deontic-validity | E2E |
| AI-012 | AI S4.5 | Rights violation MUST NOT be attributed to the agent. | Runtime data availability. | T3 | deontic-validity | E2E |
| AI-013 | AI S4.6 | Deontic constraints MUST be evaluated in order: Permissions, Prohibitions, Obligations, Confidence, Volume, Sampling. | Runtime evaluation ordering. | T3 | deontic-validity | E2E |
| AI-014 | AI S4.6 | Most restrictive enforcement action applies when multiple constraints violated simultaneously. | Runtime conflict resolution. | T3 | deontic-validity | E2E |
| AI-015 | AI S4.7 | All constraints at all three composition levels MUST be evaluated. | Runtime multi-level aggregation. | T3 | deontic-validity | E2E |
| AI-016 | AI S4.7 | Cross-level conflicts resolved by most restrictive action. | Runtime conflict resolution. | T3 | deontic-validity | E2E |
| AI-017 | AI S4.9 | Null deontic expression in rights/safety workflow MUST escalate to human. | Runtime null evaluation. | T3 | deontic-validity | E2E |
| AI-018 | AI S5.3 | `autonomous` actions MUST have associated deontic constraints. | Partially lintable within-document. | T2-xdoc | autonomy-constraint | T2 |
| AI-019 | AI S5.3 | `assistive` actions MUST create a human task for confirmation. | Runtime execution outcome. | T3 | autonomy-constraint | E2E |
| AI-020 | AI S5.3 | `supervisory` actions MUST define `reviewWindow`. | Partially lintable within-document. | T2-xdoc | autonomy-constraint | T2 |
| AI-021 | AI S5.3 | Effective autonomy MUST NOT exceed impact-level cap. | Cross-document minimum computation. | T3 | autonomy-constraint | E2E |
| AI-022 | AI S5.3 | Effective autonomy = minimum of 4 sources. | Cross-document computation. | T3 | autonomy-constraint | E2E |
| AI-023 | AI S5.3 | Every agent invocation MUST have a reachable path to completion without any agent. | Graph reachability analysis over lifecycle. Conservative global approximation; per-invocation compliance requires manual review. | T2-xdoc | fallback-chain-validity | T2 |
| AI-024 | AI S5.4 | Escalation conditions MUST be valid FEL referencing `agent` context. | FEL AST analysis. | T2-ast | expression-validity | AST |
| AI-025 | AI S5.4 | Human approval required for escalation. | Runtime workflow event. | T3 | autonomy-constraint | E2E |
| AI-026 | AI S5.4 | Escalation MUST have `escalationExpiry`; agent reverts when expired. | Runtime temporal constraint. | T2-xdoc | autonomy-constraint | tier2_rules.rs |
| AI-027 | AI S5.4 | Escalation does NOT bypass deontic constraints. | Runtime enforcement behavior. | T3 | deontic-validity | E2E |
| AI-028 | AI S5.5 | Demotion takes effect for next invocation; in-flight annotated. | Runtime timing rule. | T3 | autonomy-constraint | E2E |
| AI-029 | AI S5.5 | `pendingRecalibration` keeps demoted level until escalation conditions met. | Runtime state tracking. | T3 | autonomy-constraint | E2E |
| AI-030 | AI S5.6 | Dynamic autonomy MUST NOT exceed effective cap. | Runtime FEL computation. | T3 | autonomy-constraint | E2E |
| AI-031 | AI S6.2 | Agent output contract MUST apply same rules as human-facing form. | Cross-form semantic equivalence. | T2-xdoc | agent-consistency | tier2_rules.rs |
| AI-032 | AI S6.2 | Validation failures MUST trigger fallback, not silent acceptance. | Runtime enforcement behavior. | T3 | fallback-chain-validity | E2E |
| AI-033 | AI S6.2 | Agent-touched fields MUST be annotated with `agentProvenance`. | Runtime output annotation. | T3 | agent-consistency | E2E |
| AI-034 | AI S7.1 | Every agent output MUST have a ConfidenceReport. | Runtime output requirement. | T3 | confidence-validity | E2E |
| AI-035 | AI S7.2 | `modelNative` confidence MUST be calibrated. | Operational/empirical property. | T3 | confidence-validity | E2E |
| AI-036 | AI S7.4 | Confidence below floor MUST invalidate output. | Runtime enforcement. | T3 | confidence-validity | E2E |
| AI-037 | AI S7.5 | DecayTrigger multiplies confidence; below floor triggers escalation. | Runtime state tracking. | T3 | confidence-validity | E2E |
| AI-038 | AI S7.7 | Cumulative confidence below floor MUST pause for human review. | Runtime computation. | T3 | confidence-validity | E2E |
| AI-039 | AI S8.2 | Every fallback attempt MUST produce provenance. | Runtime behavioral requirement. | T3 | fallback-chain-validity | E2E |
| AI-040 | AI S8.2 | Terminal fallback MUST produce result or human task. | Runtime execution property. | T3 | fallback-chain-validity | E2E |
| AI-041 | AI S8.4 | Fallback chain MUST terminate in `escalateToHuman` or `fail`; MUST NOT cycle. | Graph traversal over chain array. | T1 | fallback-chain-validity | T1 |
| AI-042 | AI S9.2 | Agent config MUST disclose training data characteristics. | Empirical accuracy of disclosure. | T2-xdoc | drift-config-validity | T2 |
| AI-043 | AI S9.2 | Agent config MUST disclose optimization objective. | Empirical accuracy of disclosure. | T2-xdoc | drift-config-validity | T2 |
| AI-044 | AI S9.3 | Training data contamination triggers reclassification to determination. | Runtime operational decision. | T3 | drift-config-validity | E2E |
| AI-045 | AI S10.2 | `independentFirst` suppression MUST hide agent output until independent assessment recorded. | Runtime UI behavior. | T3 | agent-consistency | E2E |
| AI-046 | AI S12.2 | `rights-impacting` workflows MUST have `discloseThatAgentAssisted: true`. | Cross-document conditional. | T2-xdoc | disclosure | T2 |
| AI-047 | AI S13.2 | Narrative tier provenance MUST be labeled non-authoritative. | Runtime emission requirement. | T3 | agent-consistency | E2E |
| AI-048 | AI S13.2 | Narrative tier MUST NOT be treated as dispositive evidence. | Downstream usage behavior. | T3 | disclosure | E2E |
| AI-049 | AI S13.3 | `authoritative` field MUST be `false` on Narrative records. | Lintable as const check. | T1 | agent-consistency | T1 |
| AI-050 | AI S14.2 | Assist Governance Proxy MUST NOT modify conformance requirements. | Implementation behavior. | T3 | cross-layer-reference | E2E |
| AI-051 | AI S14.2 | Proxy MUST apply deontic constraints to tool invocations. | Runtime enforcement. | T3 | deontic-validity | E2E |
| AI-052 | AI S14.2 | Proxy MUST produce provenance per governed invocation. | Runtime behavioral requirement. | T3 | agent-consistency | E2E |
| AI-053 | AI S3.4 | Version change MUST emit `agentVersionChange` provenance. | Runtime monitoring. | T3 | agent-consistency | E2E |
| AI-054 | AI S4.7 | Bypass applies to single invocation only; MUST produce provenance with rationale. | Runtime scope enforcement. | T3 | deontic-validity | E2E |
| AI-055 | AI S4.7 | Consistency constraints MUST detect contradictions between output and case data. | Runtime FEL evaluation. | T3 | deontic-validity | E2E |
| AI-056 | AI S5.1 | Autonomy is action-site property, not agent property. | Structural modeling constraint across document graph. | T2-xdoc | autonomy-constraint | T2 |
| AI-057 | AI S3.5 | WOS Processor enforces constraints; agent cannot weaken its own constraints. | Architectural trust-boundary enforcement. | T3 | agent-consistency | E2E |
| AC-001 | AgentConfig S1.3 | Expired calibration caps autonomy at `assistive`. | Runtime calendar comparison. | T3 | autonomy-constraint | E2E |
| AC-002 | AgentConfig S1.4 | `maxAutonomy` participates in cross-document minimum. | Cross-document computation. | T3 | autonomy-constraint | E2E |
| DM-001 | DriftMonitor S1.1 | Extension keys MUST be prefixed with `x-`. | Pattern check. | T1 | drift-config-validity | T1 |
| DM-002 | DriftMonitor S1.4 | Rights/safety workflows SHOULD follow shadow/canary/production sequence. | Cross-document semantic requirement. | T2-xdoc | drift-config-validity | T2 |
| AG-001 | AdvGov S3.3 | Equity guardrails MUST NOT block individual actions. | Runtime semantics of `suspend`. | T3 | equity-validity | E2E |
| AG-002 | AdvGov S4.4 | Excluding a pending activity MUST raise resolution error. | Runtime DCR marking state. | T3 | dcr-soundness | E2E |
| AG-003 | AdvGov S4.5 | Zone satisfied when all pending executed and no included activity pending+unexecuted. | Runtime marking state computation. | T3 | dcr-soundness | E2E |
| AG-004 | AdvGov S5.4 | Cumulative confidence below floor MUST pause session at next checkpoint. | Runtime state tracking. | T3 | confidence-validity | E2E |
| AG-005 | AdvGov S6.1 | Agent MUST NOT invoke tools not in permitted list. | Runtime invocation enforcement. | T3 | autonomy-constraint | E2E |
| AG-006 | AdvGov S6.1 | Agent MUST NOT write to case file directly. | Runtime architectural enforcement. | T3 | autonomy-constraint | E2E |
| AG-007 | AdvGov S6.1 | Tool invocations MUST respect rate limits. | Runtime invocation counting. | T3 | autonomy-constraint | E2E |
| AG-008 | AdvGov S6.1 | Side-effect tools at `autonomous` MUST have `sideEffectPolicy`. | Cross-referencing tool flags, autonomy level, and policy presence. | T2-xdoc | autonomy-constraint | T2 |
| AG-009 | AdvGov S7.2 | Agent state transitions MUST produce provenance; suspension pauses in-flight sessions. | Runtime processor obligation. | T3 | agent-consistency | E2E |
| AG-010 | AdvGov S8.2 | Verifiable constraints MUST satisfy all SMT subset restrictions. | FEL AST analysis (parse, let, linear, extensions, **finite equality**). JSONPath filter expressions are rejected by the FEL parser (S8.2 r6) — no separate lint. **Severity:** parse failures → **Error**; finite-domain equality heuristic → **Warning** (same rule id — use `severity` in tooling). | T2-ast | smt-compatibility | AST |
| AG-011 | AdvGov S8.2 | `let` bindings MUST NOT create recursive definitions. | FEL AST cycle detection. | T2-ast | smt-compatibility | AST |
| AG-012 | AdvGov S8.2 | Quantifiers MUST quantify over finite domains. | **Partial (T2-ast):** warns when `every` / `some` are called with arity ≠ 2 (non-standard). Core `every(array, expr)` / `some(array, expr)` iterates a concrete array — not flagged. Full finiteness proof still needs type/ontology knowledge. | T2-ast | smt-compatibility | T2 |
| AG-013 | AdvGov S8.2 | Arithmetic MUST be linear (no variable*variable). | FEL AST analysis. | T2-ast | smt-compatibility | AST |
| AG-014 | AdvGov S8.2 | Verifiable subset MUST NOT include extension function calls. | Cross-reference against Core S3.5 catalog. | T2-ast | smt-compatibility | AST |
| AG-015 | AdvGov S8.3 | Proven-unsafe constraint MUST be corrected before workflow reaches `active`. | Runtime lifecycle gating. | T3 | smt-compatibility | E2E |
| AG-016 | AdvGov S9.3 | Every review provides ground-truth label. | Operational data completeness. | T3 | confidence-validity | E2E |
| AG-017 | AdvGov S11.1 | Shadow mode RECOMMENDED for rights-impacting before granting operational authority. | Cross-document + operational history. | T2-xdoc | autonomy-constraint | T2 |
| EQ-001 | EquityConfig S1 | Extension keys MUST be prefixed with `x-`. | Pattern check. | T1 | equity-validity | T1 |
| VR-001 | VerifReport S1 | Verification report is immutable once produced. | Storage/lifecycle property. | T3 | smt-compatibility | E2E |
| VR-002 | VerifReport S1 | Proven-unsafe MUST prevent workflow activation until corrected. | Cross-document lifecycle gating. | T3 | smt-compatibility | E2E |
| VR-003 | VerifReport S1 | `counterexample` MUST be present when result is `proven-unsafe`. | Conditional field presence based on sibling value. | T2-xdoc | smt-compatibility | T2 |

**AI + Advanced:** 82 constraints — 5 T1, 20 T2, 57 T3. **Tested: 82 of 82** (5 T1 including AI-003 shared coverage, 5 AST, 14 T2, 57 E2E, 1 shared-rule note).

---

## Integration Profile

| ID | Section | Rule | Why schema can't enforce | Tier | Category | Tested? |
|---|---|---|---|---|---|---|
| I-001 | Integration §3.3.1 | `outputBinding` JSONPath expressions MUST NOT use filter expressions (`[?(...)]`) or recursive descent (`..`). | JSONPath is an opaque string in the schema; only a parser can detect unsupported constructs. | T1 | integration-profile | T1 |

**Integration Profile:** 1 constraint — 1 T1, 0 T2, 0 T3. **Tested: 1 of 1** (1 T1).

---

## Summary

| Layer | Total | T1 (static) | T2 (cross-doc + AST) | T3 (dynamic) | Tested | Coverage |
| ----- | ----- | ----------- | -------------------- | ------------- | ------ | -------- |
| Kernel + Lifecycle Detail + Correspondence Metadata | 49 | 17 | 6 | 26 | 49 | 100% |
| Governance + Sidecars | 65 | 14 | 29 | 22 | 65 | 100% |
| AI + Advanced | 82 | 5 | 20 | 57 | 82 | 100% |
| Integration Profile | 1 | 1 | 0 | 0 | 1 | 100% |
| **Total** | **197** | **37** | **55** | **105** | **197** | **100%** |

> **Note on AI-023 coverage:** The T2 test for AI-023 is a conservative global approximation (BFS reachability from initial to final state excluding agent-only states). Per-invocation compliance — verifying that each individual agent state has an alternative non-agent path — requires manual review of the lifecycle topology.

**Test coverage by test type:**

| Test type | File | Rules covered |
| --------- | ---- | ------------- |
| T1 | `crates/wos-lint/tests/tier1_rules.rs` + `crates/wos-lint/tests/I_001.rs` | 37 (K-001..K-009, K-014, K-015, K-021, K-022, K-029, K-030, K-048, CM-001, G-037..G-039, G-044, G-045, G-047, G-048, G-050, G-055, G-057, G-058, G-059, G-062, G-065, AI-041, AI-049, DM-001, EQ-001, I-001) |
| T2 | `crates/wos-lint/tests/tier2_rules.rs` | 44 (K-010, K-037, G-001, G-003, G-004, G-005, G-008, G-009, G-011, G-014, G-015, G-022, G-023, G-024, G-027, G-028, G-029, G-031, G-033, G-034, G-035, G-036, G-040, G-041, G-046, G-053, G-056, G-060, G-063, AI-007, AI-018, AI-020, AI-023, AI-026, AI-031, AI-042, AI-043, AI-046, AI-056, AG-008, AG-012, AG-017, DM-002, VR-003) |
| AST | `crates/wos-lint/src/rules/fel_analysis.rs` | 11 (K-012, K-013, K-017, K-019, G-042, G-043, AI-024, AG-010, AG-011, AG-013, AG-014) |
| E2E | `crates/wos-conformance/tests/*.rs` | 105 T3 rules plus K-008 T1 join coverage; fixture, profile, processor-claim, and stub-integration tests pass through the runtime-backed conformance harness |

**T3 rules (105 total, 105 with E2E coverage):** All Tier 3 rules are exercised by `wos-conformance` tests. Coverage spans runtime fixtures in `kernel_conformance.rs`, profile aggregation in `profile_conformance.rs`, processor-claim checks in `processor_conformance.rs`, regression tests in `stub_integration.rs`, and lower-level lifecycle provenance tests in `provenance_tests.rs` (K-008 also has E2E coverage but is classified as T1). Runtime governance policy and provenance semantics live in `wos-runtime` / `wos-core`; conformance configures the reference policy and asserts observed results.

## Verification Tools

### Tier 1: `wos-lint` (37 rules)

Single-document structural checks. Input: one JSON document + its `$wos*` type marker. Output: diagnostics with severity, path, rule ID. No dependencies beyond the document itself.

**Lifecycle soundness (8):** K-001 through K-008 — state type validation (final with transitions, compound without initialState, parallel without regions), event naming (`$` prefix reserved), parallel join events (`$join` required).

**Cross-reference integrity (6):** K-006, K-014, K-015, K-030, K-048, G-048 — transition targets exist in states map, milestone ids unique, setData paths reference declared fields, extension keys `x-` prefixed, case relationship `x-` prefix, binding id matches map key.

**Temporal ordering (4):** G-044, G-045, G-047, G-057 — delegation expiration after effective, revocation after effective, parameter values in ascending date order, binding values in ascending date order.

**Type consistency (4):** G-037, G-038, G-039, G-050 — assertion id uniqueness, conditional field presence by assertion type, parameter value matches declared type.

**Fallback chain validity (2):** AI-003, AI-041 — chains terminate in escalateToHuman or fail, no cycles.

**Timer (1):** K-029 — startTimer has exactly one of duration or deadline.

**Actor (2):** K-009, K-021 — actor ids unique, provenance actorId references declared actor.

**Correspondence validity (1):** CM-001 — correspondence entry template ids unique within the sidecar.

**Provenance (1):** K-022 — digest present implies algorithm in extensions.

**Disclosure (1):** AI-049 — narrative authoritative must be false.

**Extension keys (2):** DM-001, EQ-001 — `x-` prefix on extension object keys.

**Hold validity (1):** G-055 — expectedDuration must be valid ISO 8601 duration or `"indefinite"`.

**Calendar validity (2):** G-058, G-059 — holiday entry has exactly one of date/rule, operating hours end strictly after start.

**Notification validity (2):** G-062, G-065 — adverse-decision templates cover required sections, section ids unique within template.

**Integration profile (1):** I-001 — outputBinding JSONPath must not use filter expressions or recursive descent (parsed at definition load).

### Tier 2: `wos-lint --project` (55 rules)

Loads a project directory containing kernel + governance + AI + sidecars. Two sub-modes:

**Cross-document resolution (43 rules):** governance tags match kernel tags (G-011), `targetWorkflow` matches kernel `url` (G-034), `resolutionDateRef` points to kernel case file field (G-031), delegation actors exist in kernel (G-046), hold resumeTrigger is a kernel event (G-029), rights-impacting kernel requires disclosure (AI-046), autonomous actions have deontic constraints (AI-018), agent declarations match across AI doc and agent config sidecar.

**FEL AST analysis (12 rules):** guard expressions parse (K-012, K-013), delegation conditions parse (G-043), assertion expressions parse (G-042), no cross-case variable references (K-017), only built-in + extension functions (K-019), FEL escalation conditions reference agent context (AI-024), SMT subset restrictions — all SMT rules satisfied (AG-010, including variable-to-variable `==` / `!=` warnings + optional `finiteDomainDeclarations`), no recursion (AG-011), linear arithmetic (AG-013), no extension functions (AG-014), finite quantification (AG-012 — warns on non–two-arg `every`/`some` only).

**AG-010 and severity:** The same rule id is used for invalid verifiable FEL (**Error**) and for finite-domain equality heuristics (**Warning**). Downstream tools MUST not assume one severity per rule id for AG-010.

### Tier 3: `wos-conformance` (105 rules)

Event-driven test fixtures create a runtime instance, enqueue fixture events with event tokens, drain `WosRuntime`, and assert observed state and provenance. Each fixture declares:

```json
{
  "id": "K-011-determinism",
  "rule": "K-011",
  "description": "Same document + events produce same transitions",
  "documents": {
    "kernel": "fixtures/kernel/purchase-order-approval.json"
  },
  "eventSequence": [
    { "event": "approve", "actor": "approver", "data": { "amount": 3000 } }
  ],
  "expectedTransitions": [
    { "from": "submitted", "to": "approved", "event": "approve" }
  ],
  "expectedProvenance": [
    { "type": "stateTransition", "actorId": "approver" }
  ]
}
```

**What dynamic fixtures test:**

- **Determinism (4):** K-011, K-024, K-026, K-033 — same inputs produce same outputs, document-order guard tiebreaking, idempotency deduplication
- **Provenance completeness (7):** K-016, K-018, K-020, K-028, K-046, G-007, G-021 — every mutation, relationship change, timer event, and task transition produces a record
- **Timer behavior (5):** K-025, K-038, K-043, K-044, K-045 — survive restarts, cancel on region exit, reset on reentry, route to creating region, tolerance enforcement
- **Compensation (5):** K-027, K-039, K-040, K-041, K-042 — append-only log, reverse order, pivot excluded, nested independent, completion event processed
- **Deontic enforcement (12):** AI-009 through AI-017, AI-027, AI-054, AI-055 — evaluation order, most restrictive wins, null propagation by impact level, bypass scope
- **Autonomy caps (8):** AI-005, AI-019, AI-021, AI-022, AI-028, AI-029, AI-030, AC-001 — cross-document minimum, calibration expiry caps, demotion timing
- **Confidence (7):** AI-034 through AI-038, AG-004, AG-016 — every output has report, decay triggers, cumulative tracking, session pause
- **Hold/resume (1):** G-030 — timer starts on hold entry, cancels on resume trigger
- **DCR (2):** AG-002, AG-003 — exclude-pending raises error, zone satisfaction conditions
- **Lifecycle (10):** K-023, K-031, K-032, K-034, K-035, K-036, G-010, G-012, G-013, G-017 — crash recovery, separation principle, compound entry, history clearing, atomic region init, independent-first ordering, pipeline provenance, weakest-link risk, separation of duties
- **Remaining (44):** processor conformance claims, architectural invariants, operational data completeness, downstream usage constraints

Binding-specific note: lifecycle fixtures use a permissive `ConformanceBinding` where the fixture is about WOS lifecycle behavior rather than Formspec validation. Binding-backed S15 task-submission fixtures for `wos-formspec-binding` are tracked in `TODO.md` as product hardening, not as an uncovered rule in this matrix.

---

## Test Coverage Gaps

Audit date: 2026-04-14. Cross-referenced against all test files in `crates/wos-lint/tests/` and `crates/wos-conformance/tests/`.

| Metric | Count |
|--------|-------|
| Total rules in matrix | 197 |
| Rules with at least one test | 197 |
| Rules with **no** test | 0 |

### Tested rule inventory (197 rules)

**From `tier1_rules.rs` (36 rules) + `I_001.rs` (1 rule):** K-001, K-002, K-003, K-004, K-005, K-006, K-007, K-008, K-009, K-014, K-015, K-021, K-022, K-029, K-030, K-048, CM-001, G-037, G-038, G-039, G-044, G-045, G-047, G-048, G-050, G-055, G-057, G-058, G-059, G-062, G-065, AI-041, AI-049, DM-001, EQ-001, I-001.

**From `tier2_rules.rs` (44 rules):** K-010, K-037, G-001, G-003, G-004, G-005, G-008, G-009, G-011, G-014, G-015, G-022, G-023, G-024, G-027, G-028, G-029, G-031, G-033, G-034, G-035, G-036, G-040, G-041, G-046, G-053, G-056, G-060, G-063, AI-007, AI-018, AI-020, AI-023, AI-026, AI-031, AI-042, AI-043, AI-046 (cross-doc), AI-056, AG-008, AG-012, AG-017, DM-002, VR-003.

**From `fel_analysis.rs` inline tests (11 rules):** K-012, K-013, K-017, K-019, G-042, G-043, AI-024, AG-010 (parse **Error** vs finite `==`/`!=` **Warning**, declarations, deduped path pairs, FEL rejects JSONPath `[?]` filters), AG-011, AG-013, AG-014.

**From `wos-conformance` tests (105 T3 rules + K-008 join coverage):** All Tier 3 rules are covered by `kernel_conformance.rs`, `profile_conformance.rs`, `processor_conformance.rs`, `provenance_tests.rs`, and `stub_integration.rs`. K-008 is also exercised by conformance tests but remains classified as T1 because the normative check is static. Runtime-backed fixture assertions observe provenance emitted by `wos-runtime` / `wos-core`; conformance no longer synthesizes compensation, lifecycle/case separation, or history-cleared provenance records.

**AI-003:** Same static validation as **AI-041** (fallback chain termination and cycle detection). Tests assert `AI-041`; see gap section below.

### Gap list by tier

#### T1 gaps

No open Tier 1 lint coverage gaps. `K-009` and `CM-001` are now implemented in `tier1.rs` and covered in `tier1_rules.rs`.

#### AI-003 (fallback load-time validation)

Covered by the same implementation and tests as **AI-041** (`tier1_rules.rs`). The diagnostic uses rule id `AI-041`; AI-003 is the normative S2.2 umbrella for that static validation.

#### T2 calendar / notification gaps

**Resolved (Phase 8):** G-060 (scoped Business Calendar + SLA `calendarType`), G-063 (template ref resolution). Both run whenever a governance document is present **even if the kernel is missing or fails typed deserialization** (they only need `targetWorkflow` + sidecars). When SLA violates BC S6.1, **G-023** (Governance S10 SHOULD) and **G-060** (BC S6.1 MUST) both emit: warning for authoring guidance plus error for the normative obligation.

**Resolved (Phase 10):** AG-010 finite-domain equality heuristics — **Warning** when both sides of `==` or `!=` are simple non-literal field/context accesses, unless a side is a literal, or either path appears in `finiteDomainDeclarations`. (Comparisons such as `$instance.impactLevel == "x"` are decidable because one side is a literal, not because of a separate enum special-case in the linter.) AdvGov S8.2 restriction 6 (no JSONPath filter predicates in paths): **enforced by the FEL parser** — `PathSegment` has no filter variant and `$items[?(@.x > 1)]` does not parse; no dedicated lint rule.

#### T3 gaps

No open Tier 3 rule-coverage gaps remain. The former 90-rule runtime fixture backlog is now covered by `wos-conformance` fixture, profile, processor-claim, and stub-integration tests. Binding-backed S15 conformance is tracked in `TODO.md` as a runtime/product hardening item rather than a separate matrix rule gap.
