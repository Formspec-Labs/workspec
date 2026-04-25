# FlowSpec Counter-Proposal — Disposition

**Date:** 2026-04-24
**Subject:** `counter-proposal.md` (FlowSpec v0.1 Working Draft, dated 2026-04-09) and `counter-proposal-extra.md` (X31–X34)
**Verdict:** FlowSpec is not a WOS replacement. **Thirteen sequenced absorption items across five waves** (plus a Wave 0 cross-cutting pattern note) land adopt/adapt work. Feature tables use **one row per traced FlowSpec surface** for traceability; the same logical change may appear twice (for example §3.5 `retry` and extension **E3**). Together with held/reject posture, that closes measurable declarative-surface gaps without collapsing WOS's governance depth, AI constraints, provenance tiers, intake-handoff boundary, or one-signing-pipeline posture. **Two structural questions** (artifact taxonomy collapse, seam vocabulary drift) are flagged in appended sections and should resolve *before* Wave 2+ landings — they affect where each wave's work lives, not whether it lands.

Each substantive feature in both files is classified into one of **four** buckets:

- **ADOPT** — absorb as-is; closes a real gap
- **ADAPT** — take idea, reshape to WOS conventions
- **REJECT** — incompatible with stack invariants, or WOS has better
- **HELD** — already covered by WOS/Formspec stack, no action — **including** concerns that belong in another normative layer (e.g. presentation/layout in Theme) rather than in kernel workflow topology

## Summary

Row counts below match **data rows** in the disposition tables in this file (excluding this summary block). One logical adoption may appear in more than one row. **Mechanical rule:** scan every markdown table body row from `## Design principles` through the **Conformance** table; assign the row to the first cell whose trimmed text starts with `ADOPT`, `ADAPT`, `REJECT`, or `HELD` (so `ADOPT (shape only)` → ADOPT, `HELD (stronger)` → HELD). If a second tool disagrees, diff parsers before changing numbers.

| Bucket | Count |
|---|---|
| ADOPT | 25 |
| ADAPT | 8 |
| REJECT | 18 |
| HELD | 49 |

## Design principles (FlowSpec §1)

| Ref | Principle | Disposition | Rationale |
|---|---|---|---|
| FP-01 | Spec defines structure/state, not execution | HELD | Rust spec-authority + `DurableRuntime` trait |
| FP-02 | Formspec is the type system | HELD | Kernel `contractHook` seam permits pluggable case data; Formspec is the recommended binding (Kernel §10.2, §11; Runtime Companion S15.1). **Caveat (Response path):** normative kernel + S15 text today centers **Definition pins + Response instances** and **explicit mapping** into case fields (`responseMappingRef`, etc.); without mapping, S15 forbids ad-hoc Response→case projection. Do not read later HELD/ADOPT rows as “every Formspec artifact tier is already native kernel surface” beyond what `contractHook` and companions define. |
| FP-03 | Composed FormSpec via modular composition | HELD | Formspec Core modular composition |
| FP-04 | AI extracts, rules decide, humans confirm | HELD (stronger) | L2 enforces via deontic prohibitions + autonomy caps; FlowSpec is guidance-only |
| FP-05 | AI generation is a first-class constraint | HELD | Spec → schema → lint → conformance loop. **WOS:** use `wos-spec/README.md` / `RELEASE-STREAMS.md` for current schema-stream and lint counts (repo numbers move; older “18 / 116” snapshots are not normative). |
| FP-06 | FEL is the expression language | HELD | Exact alignment; `fel-core` reuse |
| FP-07 | Layout is a separate concern | HELD | **HELD (other layer):** normative page/grid layout lives in the **Formspec Theme** companion; Core still allows advisory item `presentation` where applicable. Same separation FlowSpec describes via LayoutSpec — no kernel workflow change. |

## Top-level schema, edges, bindings (FlowSpec §2.1–2.4)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| `$flowspec` / `id` / `version` / `name` / `description` (optional) / `status` / `extensions` | HELD | WOS workflow docs have these |
| `formRef` URI\|version pattern | HELD | `contractRef` + Formspec binding |
| Flat `nodes[]` + `edges[]` arrays | REJECT | Statechart is deliberate; authoring simplification is integration-profile concern |
| Edge `condition` (FEL) | HELD | Guards on transitions |
| Edge `trigger` ("timeout"/"error"/"normal") | REJECT | Typed `TransitionEvent` kinds already first-class |
| `$field.{path}` binding syntax | HELD | Formspec FEL uses definition/instance paths (`$fieldKey`, `$parent.child`, `@instance()` — Core §3.2). **`caseFile.*` is a WOS host projection for case data in workflow FEL, not a Formspec Core prefix.** Same role as FlowSpec’s `$field.{path}` binding into the composed form. |
| `$exec.{property}` metadata binding | HELD | `@case`, `@transition`, `@agent` context vars |

## Timeouts (FlowSpec §2.5)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| ISO 8601 `duration` + `action` | HELD | `stateTimeout` |
| Ordered escalation array | ADOPT | Covered by `escalationLevels` (see §3.4.2 row) |
| X31 — timeout actions require explicit edges | REJECT | Statechart routes timers structurally via **`TransitionEvent`** with `kind: “timer”` (Kernel §4.5), so timer outcomes flow through the same guard-composable transition surface as every other event kind. That is a **stronger invariant** than FlowSpec's explicit-edge rule because **guards compose over `TransitionEvent`, not over edge annotations** — WOS can express “timer-fires AND guard-holds → target” as one predicate; FlowSpec's flat edge form cannot. FlowSpec §2.5 / §5.1 already require explicit edges for timeout targets in the working draft; `counter-proposal-extra.md` X31 is IMP-1 feedback that landed there. This REJECT is therefore a cross-reference to the flat-edge-graph REJECT above (same invariant expressed on a different surface), not an independent call. |

## Node type: start (FlowSpec §3.1)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Single start node, one entry point | HELD | WOS initial state |
| `requiredPayload` intake field list | REJECT | Duplicate validation surface; ADR 0073 + Formspec binding already validate intake |

## Node type: end (FlowSpec §3.2)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Multiple terminal outcomes | HELD | Multiple final states with `tags` |
| `terminalStatus` machine-readable code | ADAPT | Add `outcomeCode` to final states (distinct from governance `tags`) |
| `onComplete` trigger array | REJECT | Duplicates `correlationKey` + relationship events; bypasses provenance |

## Node type: ai (FlowSpec §3.3)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| `model` / `temperature` / `systemPrompt` | REJECT | **Node-local AI provider config** is an adapter concern (E5). **Clarification:** this REJECT is about *node-local provider selection* (picking a chat-completion provider per workflow node), **not** about model identity as such. Model identity/version *is* a WOS governance concern — tracked at L2 `CapabilityDeclaration` registration for drift monitoring, disclosure (EU AI Act Art. 13 / OMB M-24-10), and autonomy-cap anchoring. Workflow topology does not choose the provider per node; L2 registers the agent + its model, and adapters route calls. |
| `promptTemplate` with placeholders | REJECT | Adapter concern — couples to chat-completion modality |
| `inputBindings` (named aliases → caseField) | ADOPT (shape only) | Optional on `CapabilityDeclaration` for self-documenting capability |
| `outputSchema` (inline JSON Schema) | ADAPT | Prefer `outputContractRef`: content-addressable schema makes `verificationLevel` + Trellis custody coherent across capability invocations (same shape → same hash → same anchoring, so provenance compares cleanly across calls and audit export). Inline `outputSchema` permissible for single-use shapes; lint MAY warn and suggest promotion to `outputContractRef` when reuse is detected. |
| `outputBindings` (JSON pointer → caseField) | ADOPT | Closes real gap on `CapabilityDeclaration`; processor applies after validation, emits `agent-extracted` mutations |
| `requiredFields` gate | HELD | Formspec contract validation |
| `tools` array (MCP) | REJECT | Adapter concern (E5), same layer as every other §3.3 provider knob. A tool granted to an agent crosses into L2 autonomy-cap territory and is policed via `CapabilityDeclaration` permissions + deontic constraints, not via workflow-node config. |

## Node type: human (FlowSpec §3.4)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| `formSections` / `readOnlyFields` / `editableFields` | ADOPT | Runtime Companion S15 Formspec-backed task |
| Structured `actions[]` with `fieldBinding` + `fieldValue` | ADOPT | `taskActions` on S15 |
| Reveal-on-action `formSections` + `requiredFields` | ADOPT | Same block |
| Action `confirm` dialog string | ADOPT | Same block |
| `assignee` (role/pool/user) | HELD | Actor references |
| **X34 — escalation `levels[]` config** | ADAPT | `escalationLevels` on L1 Governance review protocol |
| Per-level `assignee` / `timeout` / `additionalFormSections` / `additionalReadOnlyFields` / `additionalEditableFields` | ADAPT | Same |
| `entryConditions` + `defaultEntry` | ADAPT | Same |
| `levelHistory[]` in execution state | ADAPT | `escalation_advanced` provenance records |

## Node type: integration (FlowSpec §3.5)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| `service` / `method` / `parameters` | HELD | `invokeService` action |
| `outputBindings` from response | ADOPT | Same pattern as AI node; extends `invokeService` |
| `outputSchema` (inline) | ADAPT | Prefer `serviceRef` contract — same provenance-anchoring rationale as the AI-node `outputContractRef` row (content-addressable → `verificationLevel` + custody coherent across calls). Inline permissible for single-use shapes; lint promotes to `serviceRef` on reuse. |
| `retry` policy (maxAttempts/backoffRate/retryableErrors) | ADOPT | `retryPolicy` on `invokeService`; provenance per attempt |

## Node type: conditional (FlowSpec §3.6)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Multi-way routing | HELD | Guarded transitions on atomic states |
| X33 — conditional-node vs. conditional-edge heuristic | REJECT | Semantic `tags` on transitions (Kernel §4.12) is strictly superior for WOS — governance attaches to transitions, not merely to documentation. FlowSpec §3.6 already documents both routing mechanisms plus a heuristic; `counter-proposal-extra.md` X33 asks for more appendix guidance. This REJECT means **do not mirror FlowSpec’s dual-mechanism story in WOS**, not that FlowSpec failed to address it. |

## Node type: transform (FlowSpec §3.7)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| FEL calculations with target field writes | HELD | Transition actions with FEL |

## Node type: parallel (FlowSpec §3.8)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Fan-out branches | HELD | Parallel state regions |
| `join` = "all"/"any"/n | HELD | `cancellationPolicy` |
| `mergeStrategy` (shallow/deep/collect) | ADOPT | New enum on parallel states; closes implicit "last-write" default |
| `collectPath` | ADOPT | Paired with `mergeStrategy: "collect"` |

## Node type: foreach (FlowSpec §3.9)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Iterate over array (structural primitive) | ADOPT | New **`foreach`** topology (**fifth** state kind alongside **atomic**, **compound**, **parallel**, and **final**). **Final is terminal, not composite** — do not lump it with compound/parallel when counting “composite” kinds. |
| `collection` + `itemVariable` + `indexVariable` | ADOPT | Same |
| `concurrency` integer or null | ADOPT | Same |
| `breakCondition` FEL | ADOPT | Same |
| `outputPath` aggregation | ADOPT | Inherits `mergeStrategy` |

## Node type: wait (FlowSpec §3.10)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Pause for event | HELD | Signal/message transitions + `correlationKey` |
| Pause for FEL condition | HELD | Guarded transitions |
| `event.type` identifier | HELD | Signal/message event names |
| `event.payloadSchema` inline | ADOPT | `eventContract` URI on signal-waiting states |
| `outputBindings` from event payload | ADOPT | `eventOutputBindings` JSON pointer map |
| Mandatory timeout (no indefinite waits) | ADOPT | Promote `stateTimeout` to MUST-level on signal/message wait substates in WOS normative text (Kernel §9.7) — matches FlowSpec §3.10's indefinite-wait prohibition. One-sentence prose addition in Wave 1; no schema change (the `stateTimeout` surface already exists). |

## Execution state — node statuses (FlowSpec §4.2)

| Status | Disposition | Landing / rationale |
|---|---|---|
| pending/running/waiting/completed | HELD | WOS lifecycle |
| failed / skipped / cancelled | HELD | WOS equivalents |
| `invalidated` (stale pre-commitment output) | ADOPT | New provenance `recordKind: capabilityOutputInvalidated` |
| `quarantined` (MUST NOT auto-retry) | ADOPT | **Absorption target (not current L2 text):** extend AI Integration §8 fallback / output-validation story + provenance kinds (`capabilityQuarantined`). There is **no §8.5** in `ai-integration.md` today; do not cite it as existing. Resume requires authorized-actor reset (provenance-recorded). |
| `visitCount` per node | HELD | WOS has iteration counts |
| `timeoutDeadline` in state | HELD | Derivable from `stateTimeout` |

## Execution state — events + provenance (FlowSpec §4.3–4.4)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Append-only event log | HELD | Provenance records are append-only |
| `actor` format (user:id, system:ai:model) | HELD | `actorRef` schema |
| Field-level provenance on every write | HELD | **WOS Kernel §5.4** mutation history (append-only). *Not* Formspec Core §5.4 (that section is the validation-report schema). |
| `fieldsWritten[]` with value + previousValue | HELD | Mutation records |
| `provenance` source enum (human-entered / human-corrected / ai-extracted / system-fetched / computed / self-attested) | ADOPT | `mutationSource` closed enum on **WOS Kernel §5.4** mutation record; WOS renames `ai-extracted` → `agent-extracted` for actor-vocabulary alignment |
| `verificationLevel` enum (independent / attested / corroborated / authoritative) | ADOPT | **Highest leverage (absorption target).** OPTIONAL field on mutation record. Tying it to **`determination`-tagged transitions** is **policy-shaped** (governance profile / attachment pattern), not a current blanket L1 MUST in prose searched for this disposition. |
| `x-` prefixed provenance strings on field writes (§4.4) | HELD | Vendor-specific sourcing labels belong in extension or profile space; closed `mutationSource` enum remains the interoperable baseline |
| Event type taxonomy (execution_started / node_entered / etc.) | HELD | Covered by provenance record kinds |

## Validation rules (FlowSpec §5)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Graph structure validation | HELD | Kernel lint + schema |
| Binding validity (cross-doc refs) | HELD | WOS Tier-2 lint for cross-document FEL integrity; Formspec side uses Core vs Extended conformance (§1.4), composition/registry/mapping where applicable — not the same “tier” vocabulary. |
| Node-type config schema validation | HELD | Per-type schema + lint |
| FEL parse + reference validity | HELD | `fel-core` + K-lint |
| §5.2 — `outputBindings` MUST NOT overwrite applicant `x-flowspec.scope: "form"` fields unless a human task lists those paths in `editableFields` | ADAPT | Same invariant expressed via Formspec Core/Extended validation + bind semantics plus `taskActions` projection on Runtime Companion §15. `taskActions` is a Wave 3 absorption target, not current normative WOS text — so this row binds to work that has not landed, hence ADAPT not HELD. No silent workflow writes around governed human confirmation. |

## X32 — JSON Schema 2020-12 + discriminator

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Adopt 2020-12 discriminator where it improves tooling errors | ADOPT (meta/publishing) | Not a normative spec change; tighten canonical schema publication where it helps actionable validation errors |

## Extension points (FlowSpec §6)

| Ref | Extension | Disposition | Landing / rationale |
|---|---|---|---|
| E1 | Custom node types | HELD | **Six** named extension seams in Kernel **§10** (10.1–10.6). Kernel **intro** prose may still say “five”; treat the **§10** enumeration as authoritative until the intro is reconciled. |
| E2 | Assignee resolution | HELD | Actor references |
| E3 | Retry/error policies | ADOPT | `retryPolicy` on `invokeService` |
| E4 | Service adapters | HELD | `serviceRef` pattern |
| E5 | AI provider adapters | HELD | L2 adapter-friendly |
| E6 | Expression extensions | HELD | FEL extension mechanism |
| E7 | Execution hooks | HELD | `lifecycleHook` seam |
| E8 | End node `onComplete` triggers | REJECT | Duplicates correlation mechanism |
| E9 | Field type extensions | HELD | `caseFieldExtension` seam |
| E10 | Validation extensions | HELD (fragmented) | Formspec shapes, bind `constraint`, §5.7 external validation, and §8 extensions — four seams serve one conceptual job (custom validator plug-in point). HELD is accurate; consolidation into a single named seam is future-ADR work. Asking “where does a custom validator plug in?” should not yield four answers. |
| E11 | Trigger mechanisms | HELD | ADR 0073 intake handoff + workflowInitiated / publicIntake |

## Conformance (FlowSpec §7)

| Feature | Disposition | Landing / rationale |
|---|---|---|
| Two-tier Core/Extended model | REJECT | WOS four-stream model (`wos-kernel` / `wos-governance` / `wos-ai` / `wos-advanced`) preserves rights-proportional claims |

## Architectural / philosophical posture

| Item | Disposition | Landing / rationale |
|---|---|---|
| `formRef` names Formspec as case data model | HELD | Compatible — `contractHook` seam permits pluggable case data, Formspec is recommended |
| No `IntakeHandoff` typed artifact | REJECT | ADR 0073 D-3 requires named `IntakeHandoff` seam |
| No `case.created` governed event | REJECT | ADR 0073 D-1 requires WOS-owned case identity |
| No `workflowInitiated` / `publicIntake` mode vocabulary | REJECT | ADR 0073 D-4 requires both first-class |
| No governance layer (L1/L2 analog) | REJECT | Load-bearing for rights-impacting workflows |
| No signature / Trellis custody story | REJECT | Stack commits to one signing pipeline (STACK.md commitment 3) |
| Event log as audit ledger | REJECT | Platform decision register: durable execution checkpoints the ledger, does not become it; Trellis owns custody |
| Hierarchical statechart → flat node-graph | REJECT | Statechart is deliberate; lint catches nesting errors (LLM-authorability is not a gap) |

## Sequenced absorption plan

Dependency-ordered. Schema-only adds first, behavioral additions second, new structural primitives last. **Normative today vs plan:** bullets here are **targets** until ratified in spec/schema; several rows in the tables above are likewise forward-looking (e.g. `taskActions`, `retryPolicy` on `invokeService`, `mergeStrategy` / `foreach`, new provenance kinds).

### Wave 0 — Cross-cutting pattern: unified declarative output-commit pipeline

Every external-work surface in WOS — agent output, service response, event payload, human task action, parallel branch result — routes mutations through **one governed output-commit pipeline**:

- **Inputs:** actor/action output + contract ref (`outputContractRef` | `eventContract` | service response contract) + output bindings (`outputBindings` | `eventOutputBindings` | `taskActions`) + allowed write scope (editable field paths) + `mutationSource` (`agent-extracted` | `system-fetched` | `human-entered` | `human-corrected` | `computed` | `self-attested`) + `verificationLevel` (`independent` | `attested` | `corroborated` | `authoritative`).
- **Output:** validated case mutations + provenance records (Kernel §5.4, one record per mutation).
- **Reuse targets:** `CapabilityDeclaration` (L2 AI Integration §3.3), `invokeService` (Kernel §9.2), signal/message wait (Kernel §9.7), `taskActions` (Runtime Companion §15), parallel `mergeStrategy` / `collectPath` (Kernel §4.4).

Name the pipeline as one abstraction before Waves 2–4 land. Otherwise those waves produce four parallel artifacts for one pattern — `outputBindings` in AI Integration, `retryPolicy` in Kernel actions, `taskActions` in Runtime Companion, `mergeStrategy` in Kernel states — and require a consolidation pass later. Same shape across surfaces; different authority rules per surface (who writes, what write scope, what verification level, what `mutationSource`).

**Statechart topology stays.** Wave 0 absorbs the *external-work surface* (every seam uses one declarative-I/O shape), not FlowSpec's flat `nodes[] / edges[]` topology. Wave 0 is a cross-cutting note, not a separate landing — Waves 1–5 implement it.

### Wave 1 — Pure schema adds, zero processor behavior change

1. **Mutation record extensions** (Kernel §5.4, `wos-kernel.schema.json`). Add OPTIONAL `mutationSource` + `verificationLevel` enums on `MutationRecord`. Unlocks everything else that writes fields.
2. **Final state `outcomeCode`** (Kernel §4.3). OPTIONAL string, domain-defined. Lint: MUST NOT duplicate a `tags` entry.
3. **JSON Schema 2020-12 discriminator publication pass.** Tighten canonical schema publication. No normative change.
4. **Provenance record-kind slots** — `capabilityQuarantined`, `capabilityOutputInvalidated` on `wos-kernel.schema.json`. Slots declared here; processor semantics land in Wave 4. Splitting schema and behavior across waves preserves the "schema-first" sequencing contract.
5. **`stateTimeout` → MUST on signal/message wait substates** (Kernel §9.7). One-sentence prose addition. Closes FlowSpec §3.10's indefinite-wait prohibition without schema change (the `stateTimeout` surface already exists).

### Wave 2 — Capability + service declaration surface (L2 + Kernel)

1. **`outputBindings` + `inputBindings` on `CapabilityDeclaration`** (AI Integration §3.3). Processor applies after `outputContractRef` validation, emits `agent-extracted` mutations.
2. **`retryPolicy` on `invokeService` action** (Kernel §9.2). Provenance record per attempt.
3. **`eventContract` + `eventOutputBindings` on signal-waiting states** (Kernel **signal/message wait topology** + **§9.7** timer categories such as `signalTimeout` — **not** §9.4, which is **correlation keys** only). Declarative callback surface.

### Wave 3 — Governance + Runtime Companion

1. **`escalationLevels` on L1 review protocol** (Governance §4). Processor emits `escalation_advanced` facts; last-level timeout falls to standard `stateTimeout`. Carry per-level **additional** form visibility plus read-only/editable surface (FlowSpec §3.4.2 `additionalFormSections`, `additionalReadOnlyFields`, `additionalEditableFields` analog).
2. **`taskActions` on Runtime Companion S15.** Formspec-binding task shape; processor writes `fieldBinding`/`fieldValue`, triggers Formspec processing model.

### Wave 4 — Capability disposition + parallel merge

1. **`quarantined` fallback disposition processor semantics** (record-kind slots already declared in Wave 1; this wave adds behavior). Extend AI Integration §8 + provenance registration — **no §8.5 today**. Processor MUST NOT auto-retry; resume requires authorized-actor reset (provenance-recorded).
2. **`mergeStrategy` + `collectPath` on parallel states** (Kernel §4.4). Closes implicit last-write default.

### Wave 5 — New structural primitive

1. **`foreach` state shape** (Kernel §4.3). **Fifth topology kind** (atomic, compound, parallel, final, foreach) — **final remains terminal, not composite.** Uses `mergeStrategy` for output aggregation. Provenance per iteration. Ordered last so Waves 3–4 land first (avoids second pass).

## Rejects requiring explicit spec documentation

Four rejects warrant explicit spec text so future contributors don't re-raise them:

- **`onComplete` end-node triggers** — use `correlationKey` + relationship events. Note in Kernel §4.3 final-state prose.
- **Flat edge-graph vs. statechart** — statechart is deliberate; integration-profile authoring simplification is the correct layer. Note in Kernel §4 overview.
- **Timeout-action-requires-explicit-edge (X31)** — statechart makes timeout routing structural via **`TransitionEvent`** (`kind: "timer"`). Note in Kernel **§4.10 / §9.7** (not §9.3 — that section is idempotency keys).
- **Event log as audit ledger** — durable execution checkpoints, does not replace Trellis custody. Note in Runtime Companion introduction or pointer to platform decision register.

## Red flags — stack-invariant conflicts, not absorbable in any form

These are the load-bearing architectural commitments FlowSpec conflicts with. They are the reason FlowSpec is not a WOS replacement:

1. **No intake-session/governed-case boundary.** FlowSpec has no `IntakeHandoff`, no `case.created`, no mode vocabulary. ADR 0073 closes this.
2. **No governance layer.** FlowSpec has no L1/L2 equivalent. WOS governance is load-bearing for rights-impacting workflows.
3. **Event log is the ledger.** Conflicts with Trellis boundary and platform decision register.
4. **AI non-determination is guidance-only.** WOS L2 enforces via deontic schema + autonomy caps. For rights-impacting adjudication, guidance is insufficient.

## On LLM-authorability

WOS's authoring loop (spec → schema → lint → conformance) is fully operative for all user-authored content. **Three narrow WOS subsurfaces** produce higher iteration counts than FlowSpec's equivalent:

1. **Parallel statechart topology** — Kernel §4.3 / §4.8 `$join` wiring. Reduced by **Wave 2** (declarative signal-wait + `eventContract` surface reduces hand-written transition wiring).
2. **Cross-layer FEL reference integrity** — WOS Tier-2 lint, fixable by `wos-lint --project`. Reduced by **Wave 3** (`taskActions` on Runtime Companion §15 — fewer manually-authored FEL pointers per task projection).
3. **Four-stream conformance model** — pair-version claims (`wos-kernel@X` + `wos-governance@Y` + `wos-ai@Z` + `wos-advanced@W`) are genuinely harder to author than FlowSpec's two-tier model. **This is an accepted cost of rights-proportional compliance claims, not a gap to close.** Canned stream-pairing templates (kernel-only, kernel+governance, kernel+governance+AI) mitigate authoring cost without collapsing streams.

The remaining WOS complexity is structural — it exists because WOS expresses semantics FlowSpec cannot (concurrent review tracks, governance tagging, deontic constraints, provenance tiers, stream-tier compliance claims).

## Artifact taxonomy: layers, profiles, companions

WOS lands work across three artifact kinds: **layers** (kernel, governance, AI, advanced), **profiles** (Integration, Semantic, Signature), **companions** (Runtime, Lifecycle Detail). Formspec has one kind: core + sidecars. The extra dimension may carry cost without carrying meaning.

### Collapse candidates

- **Runtime Companion → Kernel Part B.** Mandatory-wait-timeout prose (Wave 1), S15 `taskActions` projection (Wave 3), timeout categories (§9.7) are kernel-grade processing-model content. Formspec keeps processing-model rules in Core §6–§7, not a peer document. If conformant-processing rules live outside the kernel doc, the "two conformant processors produce the same result" invariant (`CLAUDE.md` Claim A) cannot be checked against one authoritative source.
- **Lifecycle Detail Companion → Kernel chapter.** If it elaborates kernel transition semantics, it is §4.X content, not a peer artifact. "Companion" should mean "ships independently of the kernel"; if nothing qualifies, the category is vestigial.
- **Integration Profile → implementation appendix.** Restate / Temporal / Camunda / Step Functions guidance is non-normative — `docs/adapters/` or an appendix, not a peer artifact.
- **Semantic Profile — investigate scope.** Transition-tag vocabulary (Kernel §4.12) is kernel content; collapse up. Broader semantic alignment (ontology, cross-workflow concept mapping) may deserve standalone sidecar status — not "profile."

### Do NOT collapse

- **Four release streams** (`wos-kernel` / `wos-governance` / `wos-ai` / `wos-advanced`) and **L0/L1/L2/L3 layer split.** Both encode rights-proportional compliance claims (vision-model Q2–Q4). A jurisdiction adopting kernel+governance (no AI) must claim that pair cleanly; collapsing breaks that.
- **Signature Profile.** Active T4 track, separate conformance suite, Trellis-custody-aligned — genuinely independent concern.
- **Advanced Governance (L3).** DCR constraint zones, equity guardrails, SMT reports — genuinely advanced and genuinely optional.

### Target shape

Kernel is Core (processing model included). Governance / AI / Advanced are tiered sidecars (Formspec analog: Theme / Components / Mapping). Profiles collapse into the tier they serve, or into an appendix. One artifact kind, one axis (tier), no companion/profile distinction.

### Resolve before Wave 2+

If the artifact map is not cleaned up first, the Wave 0 declarative-I/O pipeline lands across four artifact homes — `taskActions` in Runtime Companion, `outputBindings` in AI Integration, `escalationLevels` in Governance, `mergeStrategy` in Kernel — for one abstraction. Collapse first, absorb second.

## Seam vocabulary drift

`wos-spec/CLAUDE.md` names six kernel seams as the only extension surface: `actorExtension`, `attachmentExtension`, `caseFieldExtension`, `eventExtension`, `outcomeExtension`, `sidecarExtension`.

This disposition's prose references `contractHook` (FP-02 row; architectural-posture row on `formRef`) and `lifecycleHook` (E7 row) as seams. Neither appears in `CLAUDE.md`'s six. Row E1 cites Kernel §10 as enumerating six seams (§10.1–§10.6) without naming them; §10 may or may not match `CLAUDE.md`.

Two possibilities:

- `CLAUDE.md` is stale against current Kernel §10, and the true seam set includes `contractHook` / `lifecycleHook`.
- The disposition uses "seam" loosely for advisory hooks at non-kernel layers, and `CLAUDE.md`'s six are the canonical set.

**Action.** Reconcile before Wave 1 schema work lands. The named-seams invariant (`CLAUDE.md` Q3 heuristic: "Inventing new seams is a Q3 violation") requires one canonical enumeration. Resolution pass: read Kernel §10 normative prose, align `CLAUDE.md`'s six-name list to match, align disposition row references (FP-02, E7, architectural-posture) to match. Wave 2+ absorptions declare new extension points (`outputBindings`, `eventContract`, `taskActions`); without a canonical seam list, those landings are ambiguous about whether they create new seams or attach to existing ones.
