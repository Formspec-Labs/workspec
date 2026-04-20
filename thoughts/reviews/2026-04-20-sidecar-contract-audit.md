---
title: WOS Sidecar Normative-Contract Audit
date: 2026-04-20
status: review
head: abe3c76
todo_ref: §4.6 #45
---

# WOS Sidecar Normative-Contract Audit

**Date:** 2026-04-20 · **HEAD:** abe3c76 · **Backlog item:** TODO §4.6 #45 (Imp 6 / Cx 5 / Debt 5)

## 1. Scope and Methodology

### 1.1 Files audited

Nine sidecars (or sidecar-like documents) are in scope. Every one declares a `$wos*` document-type marker, a `target*` pointer, and is described as a "sidecar" in its own front matter. Directory placement diverges from this uniform self-description (see §5).

| Sidecar | Spec file | Schema file |
|---|---|---|
| business-calendar | `specs/sidecars/business-calendar.md` | `schemas/sidecars/wos-business-calendar.schema.json` |
| notification-template | `specs/sidecars/notification-template.md` | `schemas/sidecars/wos-notification-template.schema.json` |
| policy-parameters | `specs/governance/policy-parameters.md` | `schemas/governance/wos-policy-parameters.schema.json` |
| due-process-config | `specs/governance/due-process-config.md` | `schemas/governance/wos-due-process.schema.json` |
| assertion-library | `specs/governance/assertion-library.md` | `schemas/governance/wos-assertion-gate.schema.json` |
| agent-config | `specs/ai/agent-config.md` | `schemas/ai/wos-agent-config.schema.json` |
| drift-monitor | `specs/ai/drift-monitor.md` | `schemas/ai/wos-drift-monitor.schema.json` |
| equity-config | `specs/advanced/equity-config.md` | `schemas/advanced/wos-equity.schema.json` |
| verification-report | `specs/advanced/verification-report.md` | `schemas/advanced/wos-verification-report.schema.json` |

### 1.2 Rubric

Per `CONVENTIONS.md` §"Sidecar Normative-Contract Audit Rubric (TODO #45)":

- **Step 0 — Independent existence.** Distinct semantic model OR distinct artifact lifecycle. If neither, merge into the closest host spec.
- **Structure.** Does a schema surface match the prose? Are escape hatches (`additionalProperties`, open unions) justified and bounded?
- **Semantics.** Are processor obligations explicit (MUSTs, not author affordances)? Are failure modes defined (reject / warn / ignore / provenance)?
- **Composition.** Explicit attachment point? Deterministic precedence and conflict rules with other sidecars?

Verdict vocabulary: **KEEP** (independent existence earned), **MERGE** (absorb into named host), **RESHAPE** (keep identity but restructure), **RETIRE** (delete without replacement).

Every claim is grounded in a `file:§` or `file:line` citation.

---

## 2. Per-Sidecar Assessment

### `business-calendar`

**Files:** `specs/sidecars/business-calendar.md` · `schemas/sidecars/wos-business-calendar.schema.json`

**Step 0 — independent existence?** PASS. Distinct semantic model (work week + holiday rules + operating hours + multi-calendar selection via `appliesWhen`) that neither Kernel nor Governance carries (business-calendar.md §2, §7).
**Structure?** PASS. Schema mirrors prose; `additionalProperties:false` at every `$def` except the terminal `extensions` map (wos-business-calendar.schema.json:18,152,193,226). `nthWeekday` / `lastWeekday` rule grammar is enumerated (§4.2) but encoded as a free string in the schema — authoring-surface gap, not a shape problem.
**Semantics?** PASS. Processor MUSTs enumerated: business-day definition (§3.3), timezone-conflict error (§7.2 item 3), empty applicable-set fallback with provenance warning (§7.1 item 6), expired-calendar ignore (§8.1 item 5).
**Composition?** PASS. Explicit attachment at Governance S10.3 and S13.3; multi-calendar composition rule is deterministic ("non-working if **any** calendar says so"; "most-restrictive intersection" for operating hours; reject on timezone mismatch) (§7.2).

**Verdict:** KEEP.
**Rationale:** Highest-quality sidecar in the set. Clear semantic model, clean schema, deterministic composition, absence behaviour spelled out (§8.2). Lint enforcement exists (G-023 T2 Warning, G-061 at registry.rs:748).
**Dependent work items:** TODO §4.5 item "Due Process Config partial merge" depends on this sidecar remaining stable (SLA calendar semantics referenced from governance).

---

### `notification-template`

**Files:** `specs/sidecars/notification-template.md` · `schemas/sidecars/wos-notification-template.schema.json`

**Step 0 — independent existence?** PASS. Distinct authoring lifecycle (templates are versioned/localised/audited separately from governance — §1) and distinct semantic model (category → required sections mapping with due-process rejection rule).
**Structure?** PASS. Schema matches prose; adverse-decision constraint is encoded (NotificationTemplate $def `sections` x-lm intent). `additionalProperties:false` consistent; only terminal `extensions` open (wos-notification-template.schema.json:252).
**Structure note (PARTIAL caveat).** The prose in §4.4 requires a processor to reject `adverse-decision` templates missing due-process sections; the schema does not express this conditional-required rule directly (enforcement is delegated to lint G-063 / G-065, registry.rs:765,778). Declared as a gap in §4.4 itself — acceptable per CONVENTIONS "Normative Contract" rule.
**Semantics?** PASS. MUST reject malformed adverse-decision template (§4.4); MUST NOT send notification with missing required variables (§5.3 item 3, §6.1 item 6); MUST record failure in provenance (§6.1 item 5). Missing-template is a warning, not a fatal (§5.2) — explicit.
**Composition?** PASS. `noticeTemplateRef` (Governance §3.1) and `notificationTemplateRef` (Governance §12.2) resolve through this sidecar; lint rule G-063 enforces the seam. Due-Process-Config §1.1 explicitly defers notice authoring here.

**Verdict:** KEEP.
**Rationale:** Load-bearing: six governance reference sites collapse to one canonical template surface. Retiring it would force governance to carry six inline template shapes. `M-2` merge is already rejected in TODO §4.5.
**Dependent work items:** §4.5 M-2 (rejected). TODO #39 (notification linkage hardening) complements.

---

### `policy-parameters`

**Files:** `specs/governance/policy-parameters.md` · `schemas/governance/wos-policy-parameters.schema.json`

**Step 0 — independent existence?** PASS. Two distinct semantic models (scalar parameters in §1.2 and regulatory version bindings in §1.5), both date-indexed, neither expressible in the kernel or governance document without inventing a parallel construct. OpenFisca lineage gives it independent authorial provenance (§abstract).
**Structure?** PASS. Schema consistently `additionalProperties:false` (wos-policy-parameters.schema.json:17,145,207,233,299); `parameters` and `bindings` are typed maps (lines 61,99); only terminal `extensions` open (line 332).
**Semantics?** PASS. Resolution mechanism (§1.4) is a three-step MUST; binding resolution (§1.5.4) mirrors it; processors MUST NOT alter resolution based on `bindingType` (§1.5.4 final paragraph). Failure mode for missing resolution date is implicit — see Composition gap.
**Composition?** PARTIAL. Attachment declared (Kernel S7.3, Governance S13.2), but: (a) no stated precedence when `parameters.X` and `bindings.X` collide on name — the namespace is shared ("injected into the evaluation context under `parameters.[parameterName]`" vs "`parameters.[bindingId]`"); (b) missing `resolutionDateRef` field on the case file — failure mode not spelled out (reject? warn? null?).

**Verdict:** KEEP (with RESHAPE follow-up).
**Rationale:** Semantics are load-bearing and distinct, but the parameter/binding namespace collision and missing-date failure mode are genuine composition gaps. They are fixable in-place, not by merging.
**Dependent work items:** File a follow-up for namespace collision rule and missing-`resolutionDateRef` failure-mode. Non-blocking; Cx 2.

---

### `due-process-config`

**Files:** `specs/governance/due-process-config.md` · `schemas/governance/wos-due-process.schema.json`

**Step 0 — independent existence?** FAIL. The spec is 53 lines; the schema is 195. After the thin `noticeTemplates` shape was removed (§1.1 boxed note), what remains is `explanationTemplates`, `appealRouting`, `continuationPolicies` — three fields that Workflow Governance S3.1/S3.5 already declares structurally. The independent-lifecycle claim in the abstract ("update grace period without modifying governance") is real but solved by `policy-parameters` (date-indexed values), not by carrying a second sidecar.
**Structure?** PASS. `additionalProperties:false` consistent (wos-due-process.schema.json:16,77,112,140,183).
**Semantics?** PARTIAL. `independenceConstraint` is REQUIRED but the prose doesn't say what the processor MUST do with it — it is authoring metadata, not processor obligation. `appealRouting` and `continuationPolicies` lack the "processor MUST" language that CONVENTIONS demands; this reads as "authors can express" not "processors must enforce."
**Composition?** PARTIAL. `targetGovernance` attaches the sidecar, but no precedence rule exists for when `continuationPolicies` here conflict with `continuationOfServices` flag on the host `AppealMechanism` — TODO §4.5 already flags this as a structural linkage gap.

**Verdict:** MERGE INTO Workflow Governance.
**Rationale:** Step 0 fails. Ratifies TODO §4.5 "Due Process Config partial merge → Workflow Governance" (Imp 5 / Cx 3 / Debt 4). The merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage structurally. Notice templates already moved out (§1.1 boxed note). Remaining three fields absorb cleanly.
**Dependent work items:** §4.5 entry already present — this audit upgrades it from "pending #45 Step 0" to "Step 0 confirmed FAIL, proceed."

---

### `assertion-library`

**Files:** `specs/governance/assertion-library.md` · `schemas/governance/wos-assertion-gate.schema.json`

**Step 0 — independent existence?** FAIL. The spec is 55 lines, one `$def`. The library IS a reference mechanism; its prose says "Pipelines reference library assertions by `id`" (§1.3), but the reference wire — `assertionId` on `PipelineStage.assertions[]` — does not exist in the governance schema (TODO §4.4 #38 confirms the gap). A library whose reference protocol is unshipped is not an independent spec; it is a stub.
**Structure?** PASS. Schema shape is clean and tight (164 lines, `additionalProperties:false`).
**Semantics?** FAIL. No processor obligations defined. §1.3 "The pipeline stage inherits the assertion's type, expression, fields, and rejection policy unless overridden" uses "inherits" without specifying the resolution order, override precedence, or failure mode when an id is unresolvable.
**Composition?** FAIL. No composition mechanism ships; the resolution point is undefined because the reference property doesn't exist on the consumer side.

**Verdict:** MERGE INTO Workflow Governance (as "Named Assertions" section).
**Rationale:** Ratifies TODO §4.5 entry verbatim ("Absorb as 'Named Assertions' section. Library without #38 reference protocol is incomplete; absorb rather than fix. Source is a thin 55-line spec + 139-line schema; merge is mechanical."). CONVENTIONS Step 0 is unambiguously failed.
**Dependent work items:** TODO §4.5 "Assertion Library → Workflow Governance" `[Imp 4 / Cx 2 / Debt 3]`. Closes TODO §4.4 #38 by construction.

---

### `agent-config`

**Files:** `specs/ai/agent-config.md` · `schemas/ai/wos-agent-config.schema.json`

**Step 0 — independent existence?** PASS. Distinct operational lifecycle (credential rotation, model-version approval) explicitly motivates the split (§abstract). The `DemotionRule.id` that Drift Monitor cites by `policyRef` cannot live inside AI Integration without inverting the seam.
**Structure?** PASS. Schema is thorough; `additionalProperties:false` throughout except terminal `extensions` (wos-agent-config.schema.json:16,95,129,163,191,225,270,307). Enum-and-ref shape for `EndpointConfig`, `AutonomyPolicy`, `ActionOverride` is clean.
**Semantics?** PASS. Explicit processor MUSTs: calibration-expired autonomy cap (§1.3 last paragraph: "effective autonomy MUST be capped at `assistive`"), DemotionRule resolution contract (§1.4.1).
**Composition?** PASS. Named seam: `Drift Monitor.AlertThreshold.policyRef` → `AgentConfig.DemotionRule.id`. Resolution is a four-step MUST (drift-monitor.md §1.4.1) with defined failure mode (config warning in provenance, fallback to action enum).

**Verdict:** KEEP.
**Rationale:** Semantic model is distinct and the seam with Drift Monitor is load-bearing. M-1 merge (Drift+Agent) is already BLOCKED in TODO §4.5 by the benefits-drift-monitor fixture.
**Dependent work items:** None blocking.

---

### `drift-monitor`

**Files:** `specs/ai/drift-monitor.md` · `schemas/ai/wos-drift-monitor.schema.json`

**Step 0 — independent existence?** PASS. Distinct semantic model (PSI/KS/chi² drift, rubber-stamp detection, shadow/canary/production deployment sequence) with no home in AI Integration.
**Structure?** PASS. Schema consistently closed (wos-drift-monitor.schema.json:17,79,117,145,180,230,273). Monitor/metric/threshold shape follows authoring prose.
**Semantics?** PARTIAL. `policyRef` resolution (§1.4.1) is a proper MUST contract. But: the `action` enum at §1.4 is explicitly "implementation-defined" for `notify`/`demoteToAssistive`/`demoteToManual`/`suspend` — three of those four actions have structured runtime meaning elsewhere in the spec and should reference them rather than hiding behind "implementation-defined."
**Composition?** PASS. Attachment is the `policyRef` seam to Agent Config. Deployment-sequence SHOULD (§1.5) is advisory for rights-impacting / safety-impacting workflows — composes with Kernel impact-level classification without overriding it.

**Verdict:** KEEP (with RESHAPE follow-up on `action` semantics).
**Rationale:** Independence is earned; the `action` enum's "implementation-defined" escape hatch should be tightened, but that is a clarification, not a merge. M-1 merge blocked in TODO §4.5.
**Dependent work items:** File follow-up to tighten `action` enum semantics. Cx 2. Non-blocking.

---

### `equity-config`

**Files:** `specs/advanced/equity-config.md` · `schemas/advanced/wos-equity.schema.json`

**Step 0 — independent existence?** PARTIAL. Distinct semantic model (protected categories, disparity methods, remediation triggers) that applies to human AND AI decisions (§abstract). But the spec body is 42 lines and the processor-obligation story is unshipped — TODO §4.4 #35 ("Equity Config enforcement semantics") and #36 ("RemediationTrigger expression language") are both open. Until those ship, independence is structural only, not semantic.
**Structure?** PASS. Schema is complete (224 lines); `additionalProperties:false`.
**Semantics?** FAIL. Zero processor MUSTs in the spec body. `RemediationTrigger.action` has no defined behaviour (TODO #35). `DisparityMethod` is declared but its runtime wire-up is "applies to" rather than "processor MUST compute." The spec table is pure authoring surface.
**Composition?** PARTIAL. Attachment exists (`ProvenanceKind::EquityAlert` wired in `crates/wos-core/src/event_handler.rs` per TODO #35 commentary), but composition with `policy-parameters` (for disparity thresholds) or `notification-template` (for remediation notices) is not declared.

**Verdict:** RESHAPE (do not merge).
**Rationale:** Step 0 is PARTIAL because civil-rights framing gives it independent authorial provenance distinct from Advanced Governance's SMT/verification framing. Merging into Advanced Governance conflates civil-rights law (applies to human decisions) with verification research (optional, AI-adjacent). Fix semantics via TODO §4.4 #35 and #36, don't merge.
**Dependent work items:** TODO §4.4 #35 `[Imp 7 / Cx 5 / Debt 4]` and #36 `[Imp 6 / Cx 4 / Debt 4]`. Both on the Active backlog.

---

### `verification-report`

**Files:** `specs/advanced/verification-report.md` · `schemas/advanced/wos-verification-report.schema.json`

**Step 0 — independent existence?** FAIL. The spec is 40 lines, one table. It describes an output artifact (per-constraint SMT result, solver info, counterexamples) that is fundamentally a provenance record, not a configuration document. Every other sidecar is **input** to the processor; this one is **output**. It belongs in the Advanced Governance "Output Artifacts" section.
**Structure?** PASS. Schema is clean.
**Semantics?** PARTIAL. No processor MUSTs — because it's a record format, not a processor input. CONVENTIONS §Normative Contract assumes processor obligations; a pure output artifact needs a different contract section ("What the writer MUST emit") that doesn't exist here.
**Composition?** FAIL. No composition story at all. There is no declared attachment point for "when a verification run produces a report, where does it go, who reads it?"

**Verdict:** MERGE INTO Advanced Governance.
**Rationale:** Ratifies TODO §4.5 entry. Output artifacts are architecturally different from configuration sidecars; collapsing it into Advanced Governance as "§Output Artifacts" resolves the CONVENTIONS-contract mismatch.
**Dependent work items:** TODO §4.5 "Verification Report → Advanced Governance" `[Imp 3 / Cx 2 / Debt 2]`.

---

## 3. Cross-Cutting Findings

**3.1 Reference-resolution pattern is duplicated across three sidecars.**
`noticeTemplateRef` (governance → notification-template), `policyRef` (drift-monitor → agent-config `DemotionRule.id`), and the absent `assertionId` (governance → assertion-library) all describe the same pattern: "look up a named target in a sidecar whose `target*` matches the current workflow; fall back on unresolvable; record provenance." A shared `$def` for "TargetedLookupRef { sidecarKind, id, fallbackBehaviour }" would let lint reuse G-063's resolution logic across sidecars. **Finding, not recommendation** — a follow-up, not part of #45.

**3.2 "Sidecar vs. output artifact" is a category error.** Eight of nine sidecars are processor **inputs**. `verification-report` is a processor **output**. CONVENTIONS is written around processor inputs (MUST/SHOULD/MAY processor behaviour). This mis-category is part of why verification-report fails Step 0; the remedy is structural placement, not rubric change.

**3.3 Step 0 failures cluster at the "governance absorption" boundary.** `assertion-library`, `due-process-config`, and `verification-report` all fail or partially fail Step 0, and all three have a host spec (Workflow Governance or Advanced Governance) that already declares the concept structurally. TODO §4.5 already captures all three as merge candidates. This audit's single strongest signal: **§4.5 is correctly scoped and all three entries should proceed.**

**3.4 Escape-hatch audit — schemas are not over-permissive.** The pattern across all nine schemas is `additionalProperties:false` everywhere except a terminal `extensions` object whose inner `additionalProperties:{}` is the intentional `x-` extension seam. No schema is silently more permissive than its prose. One minor exception: `notification-template.md` §3.3 enumerates `deliveryChannels` values while the schema also permits custom items via extension keys — acceptable, but the prose could cite the extension seam explicitly.

**3.5 Lint-rule coverage is uneven.** Only G-023 (business-calendar), G-061 (business-calendar target), G-063 / G-065 (notification-template) explicitly cover sidecar seam enforcement (registry.rs:458,748,765,778). No lint rule covers `policyRef` resolution (drift-monitor § 1.4.1), no rule covers `bindings[*].value` URI resolvability (policy-parameters §1.5), and no rule covers equity remediation-trigger wiring. These are CONVENTIONS "Conformance" gaps; filing them is **out of scope for this audit** (per task guardrail) but belongs under the K-049 / AI-057 follow-ups or a new §4.4 item.

**3.6 Contradiction check against 2026-04-16 handoff.** The handoff's §3 retraction explicitly supports keeping named sidecars (`correspondence`, `policy`, `assertion`) because they carry semantic intent. This audit's MERGE verdicts for `assertion-library` and `due-process-config` are not contradictions: the handoff ratifies `assertion-library` because it is *semantically distinct*, but Step 0 requires *independent existence*, and an unshipped reference protocol (TODO #38) defeats independence structurally. I read the handoff as "keep named seams, not necessarily named sidecar documents" — the seam of "reusable assertion definitions" survives the merge into Workflow Governance as a `namedAssertions` map.

---

## 4. Recommended Actions

1. **Merge `assertion-library` → Workflow Governance.** Cx 2 / Debt 3. Register under **TODO §4.5** (entry already present — this audit ratifies Step 0 FAIL).
2. **Merge `verification-report` → Advanced Governance "Output Artifacts".** Cx 2 / Debt 2. Register under **§4.5** (already present — ratified).
3. **Merge `due-process-config` → Workflow Governance (partial — the three remaining sections: `explanationTemplates`, `appealRouting`, `continuationPolicies`).** Cx 3 / Debt 4. Register under **§4.5** (already present as "pending #45 Step 0" — this audit confirms Step 0 FAIL, unblocks the merge).
4. **Reshape `equity-config` semantics (processor MUSTs on `RemediationTrigger.action`, `DisparityMethod` runtime wire-up).** Cx 5 / Debt 4. Already tracked as **§4.4 #35** and **§4.4 #36** — no new work item; this audit ratifies their priority.
5. **Reshape `drift-monitor.AlertThreshold.action` enum** — replace "implementation-defined" language with references to AI Integration autonomy semantics. Cx 2. File under **§4.3a** as a new review follow-up.
6. **Reshape `policy-parameters`** — define precedence when parameter and binding share a name; define failure mode for missing `resolutionDateRef`. Cx 2. File under **§4.3a**.

Total: 3 MERGE, 3 RESHAPE, 3 KEEP, 0 RETIRE.

---

## 5. Placement Anomalies

Items that self-describe as "sidecar" but live outside `specs/sidecars/`:

| Sidecar | Current home | Should move to `sidecars/`? |
|---|---|---|
| policy-parameters | `specs/governance/` | **No** — keep with Governance (tight semantic coupling to temporal parameter resolution §S13.2). |
| due-process-config | `specs/governance/` | **N/A** — MERGING into Workflow Governance. |
| assertion-library | `specs/governance/` | **N/A** — MERGING into Workflow Governance. |
| agent-config | `specs/ai/` | **No** — named seam with Drift Monitor justifies AI grouping. |
| drift-monitor | `specs/ai/` | **No** — same. |
| equity-config | `specs/advanced/` | **No** — civil-rights framing differentiates from verification research. |
| verification-report | `specs/advanced/` | **N/A** — MERGING into Advanced Governance as output-artifacts section. |

Conclusion: the directory layout is **correct**. Moving everything into `specs/sidecars/` would erase the layer-provenance signal that currently lets readers find sidecars in the spec that "hosts" their seam. MD-INVENTORY §6 already uses this layout; no restructuring needed.

---

## 6. Open Questions

1. **Is "sidecar" the right umbrella term for both inputs and outputs?** `verification-report` is a processor output; the other eight are inputs. Yes → keep rubric; No → split into "input sidecars" and "output artifacts" in CONVENTIONS. *One-line verdict form: "Yes, unified" / "No, split."*
2. **Does the `targetedLookupRef` pattern (Finding 3.1) warrant extraction into a shared `$def` now, or wait until a fourth instance lands?** *One-line verdict: "Extract now" / "Wait for fourth instance."*
3. **For `equity-config`: civil-rights framing OR Advanced Governance consolidation?** This audit said "civil-rights framing — reshape, don't merge." If the consolidation win is bigger than the framing signal, reverse to MERGE. *One-line verdict: "Keep independent (civil rights)" / "Merge (consolidation wins)."*
4. **For `drift-monitor.action`: is 'implementation-defined' acceptable for `notify`/`suspend`, or must the spec wire them to named Kernel/AI constructs?** *One-line verdict: "Implementation-defined OK" / "Must wire to named semantics."*
5. **Does `policy-parameters` parameter↔binding namespace actually collide in any shipped fixture?** If no and no plausible author would shadow, the §4.3a follow-up can drop priority. *One-line verdict: "Collision realistic" / "Theoretical only."*
6. **Do the three §4.5 merges ship as one PR or three?** Mechanical, ~Cx 7 total. One PR is faster but harder to review; three are clearer but risk partial-merge states. *One-line verdict: "One PR" / "Three PRs."*

---

## 7. Summary Stats

- **Sidecars audited:** 9
- **Step 0 PASS:** 5 (business-calendar, notification-template, policy-parameters, agent-config, drift-monitor)
- **Step 0 PARTIAL:** 1 (equity-config)
- **Step 0 FAIL:** 3 (assertion-library, due-process-config, verification-report)
- **Verdicts:** 3 KEEP · 3 MERGE · 3 RESHAPE · 0 RETIRE
- **New TODO work items created by this audit:** 0 (ratifies existing §4.5 entries and §4.4 #35/#36; files two §4.3a follow-ups)
