# Audit — Provenance emission completeness (2026-04-28)

Closes `TODO.md` Do-next item #2 *Provenance emission completeness audit*. Distinct from the lint-matrix evidence ratchet (`every_promoted_*_rule_has_executable_or_annotated_evidence`) — that gate covers rule→fixture; this audit covers spec MUST-emit → live emission site.

**Method.**

1. Enumerated 99 `ProvenanceKind` variants from `crates/wos-core/src/provenance/kind.rs` (count at audit time, pre-Session-14 `CapabilityInvocation` addition; 100 at HEAD).
2. For each variant, counted live emission sites in `crates/wos-runtime/src` (`R+B`), `crates/wos-formspec-binding/src`, and `crates/wos-core/src` (excluding the provenance module itself — definitions, audit-tier mapping, tests). Pattern: `ProvenanceKind::<Variant>\b` plus snake_case constructor where one exists in `wos-core/src/provenance/record.rs`.
3. Cross-checked: extracted every `recordKind: "..."` mention from `specs/**/*.md` and verified each named kind maps to an enum variant.
4. Spot-checked spec MUST→emit clauses (`MUST.*(emit|produce|record|append).*provenance`) against emission paths.

**Audit script:** `/tmp/prov_audit4.sh` (file count per variant across crate axes; run inside `wos-spec/`).

## Findings

### Gap 1 — `CapabilityInvocation` recordKind has no Rust variant or emission path (HIGH)

**Update 2026-04-28 (same day, post-audit):** typed Rust slice landed in Session 14 — variant + audit-tier=Facts arm + `ProvenanceRecord::capability_invocation(...)` constructor + `CapabilityInvocationInput<'a>` + 5 unit tests (including a serde round-trip added per review F3 item 1) + module export. Reviewed by `formspec-specs:wos-scout` (semi-formal-code-review): verdict *Land it*, drift survey clean across both schemas, two doc-and-test refinements applied in-session (F1, F3 item 1). Schema MUST is now fulfillable from the typed path. AI-runtime wiring (precondition evaluation site that actually fires the constructor), `wos-export` smoke-test extension, JSON conformance fixture pair, and ergonomic constructor variant grouped under `TODO.md` Do-next #2 (gated on AI-runtime invocation seam design) — the cross-stack typed-path debt is gone; remaining work is local AI-runtime wiring.


- **Spec MUST:** `specs/ai/ai-integration.md:159` — *"Every precondition evaluation MUST produce a provenance record with `recordKind: "capabilityInvocation"`."* + `specs/kernel/spec.md:460` reserves the `preconditionNotSatisfied` outcome paired with this discriminator.
- **Schema:** `schemas/wos-workflow.schema.json:1265-1310` defines `$defs/CapabilityInvocationRecord` with two examples; `FactsTierRecord` composes via `allOf` so every conformant log validates against the MUST.
- **Rust enum:** **no variant exists.** `ProvenanceKind` (`crates/wos-core/src/provenance/kind.rs`) has 99 variants at audit time, none for capability invocation. No constructor on `ProvenanceRecord`. No reference in `wos-core` or `wos-runtime` other than a doc comment at `record.rs:147` mentioning the `preconditionNotSatisfied` outcome literal.
- **Consequence:** any conformant runtime that needs to emit this record from typed Rust would have to bypass the enum (manual JSON construction, custom serde rename, or a new adapter type). The schema-level MUST is unfulfillable through the canonical typed path.
- **Fix shape:**
  - Add `ProvenanceKind::CapabilityInvocation` (variant + audit-tier classification = `Facts` per AI §3.3.1 + Kernel §8.2.2).
  - Add `ProvenanceRecord::capability_invocation(...)` constructor with required-field discipline matching `$defs/CapabilityInvocationRecord` (`invocationBlocked` flag, `outcome: "preconditionNotSatisfied"` when blocked, `capabilityId`, optional resolved-precondition data).
  - Wire emission in the AI-integration runtime path (where preconditions are evaluated — likely `wos-core/src/event_handler.rs` or a dedicated AI module). Until the AI runtime fires, an authoring-tier conformance fixture demonstrating the typed construction path closes the typed gap.
  - Conformance fixture: at least one record with `invocationBlocked: true` and one with `false`.
- **Owner:** stack ADR not required — internal Rust + schema alignment, the spec MUST is already prose-stable.

### Gap 2 — `TaskSkipped` variant defined but never emitted (MEDIUM, already tracked)

- **Spec MUST:** `specs/companions/runtime.md:863` — *"…records `taskSkipped` provenance with the structured rationale required by Governance S10.1, emits no `completionEvent` or `failureEvent`, and is removed from `activeTasks`."* + outcome-table row at `:929` + `specs/governance/workflow-governance.md:496` *"All task state transitions MUST be recorded in provenance."*
- **Rust enum:** `ProvenanceKind::TaskSkipped` defined at `kind.rs:132`; audit-tier classification at `audit_tier.rs:110` (Facts); unit-test reference at `tests.rs:380`. No `task_lifecycle()` branch nor any other emission site in `wos-runtime`, `wos-formspec-binding`, or `wos-core`.
- **Consequence:** runtime parity gap — declared lifecycle outcome with no implementation.
- **Already tracked:** `TODO.md` open backlog item **#66e** (*"Abandonment + skip semantics … skip path → `skipped` + `taskSkipped` + removal from `activeTasks` without completion/failure events (backlog §G P11-BL-040)"*). No new TODO entry needed; this audit confirms the gap.

### Gap 3 — Configuration-warning MUSTs emit nothing (LOW, under-specified)

Four spec MUSTs require provenance emission for an unresolvable configuration reference, but the spec does not bind a specific `recordKind` and the codebase has no emission path:

- `specs/ai/drift-monitor.md:77` — *"…processors MUST emit a configuration warning in provenance and fall back…"* (unresolvable `policyRef`).
- `specs/governance/workflow-governance.md:154` — *"An unresolvable reference MUST emit a configuration warning in provenance"* (`continuationPolicyRef`).
- `specs/sidecars/notification-template.md:199` — *"…the processor MUST record a warning in provenance"* (template key not found).
- `specs/sidecars/notification-template.md:222` — *"MUST record notification rendering failures in provenance."*

Existing fallback kinds (`CalendarIgnored`, `NotificationSuppressed`) cover related sidecar-fallback semantics but not these four MUSTs. Greps for `configuration warning`, `configurationWarning`, `ConfigurationWarning` in `crates/wos-core/src` and `crates/wos-runtime/src` return zero hits.

Two viable shapes for closure:

- **(a) Generic `ConfigurationWarning` ProvenanceKind** with `data.subject` enum (`drift-monitor.policyRef`, `governance.continuationPolicyRef`, `notification-template.key`, `notification-template.render`) plus `data.unresolvedRef` payload. Lowest schema surface, highest cohesion.
- **(b) Per-site recordKind discipline** — `DriftMonitorPolicyUnresolved`, `ContinuationPolicyUnresolved`, `NotificationTemplateKeyUnresolved`, `NotificationRenderFailed`. Higher schema surface, easier to filter in audit tooling.

Lean (a). Either way, this gap is lower-stakes than #1 (CapabilityInvocation) because the spec leaves the discriminator implementer-choice and consumers won't break if it lands later. Tracked as a new low-priority item below.

## Findings — coverage table summary

- **99 ProvenanceKind variants** total at audit time (100 at HEAD post-Session 14).
- **98 with at least one live emission site** (across `wos-core` event_handler / deontic / autonomy / proxy / eval / ai logic + `wos-runtime` + `wos-formspec-binding`).
- **1 with zero live emission:** `TaskSkipped` (Gap 2).
- **0 spec recordKind names without an enum variant** EXCEPT `capabilityInvocation` (Gap 1; closed in Session 14).

Variants with `R+B = 0` but non-zero `wos-core` emission (e.g., `DeonticViolation`, `AppealFiled`, `OverrideRecorded`, `PipelineStageCompleted`, `DcrActivityExecuted`, `EquityAlert`, `VerificationReportProduced`) all resolve through the `wos-core` event-handler path (`crates/wos-core/src/event_handler.rs` plus `deontic.rs` / `proxy.rs` / `event_handler.rs`). They are not gaps; the runtime composes the core emission rather than re-emitting in a separate site.

## Methodology caveats

- File-count heuristic, not call-count — a variant emitted once from a hot path counts equal to a variant emitted from one cold guard. The audit's purpose is *MUST-coverage*, not perf or completeness within a path; counts are sufficient signal for that purpose.
- `wos-export` mapping (`prov_o.rs`, `xes.rs`, `ocel.rs`) deliberately excluded — those are read-only translations of an existing record stream into PROV-O / XES / OCEL, not emission sites. A variant that only appears in `wos-export` would be a translation target with no producer (none observed).
- Conformance fixtures (`crates/wos-conformance/`, `fixtures/conformance/`) deliberately excluded — fixtures may declare `recordKind: "..."` literals but proving the runtime emits them is the runtime audit's job, not the fixture corpus's.
- Cross-tier audit-layer transitions (Facts → Narrative → Reasoning) not in scope; that is the domain of `audit_layer_for_kind` matrix tests in `wos-core/src/provenance/tests.rs`, which is a separate gate.

## Meta-finding — no CI gate enforces schema↔enum parity

Gap 1 was structurally avoidable. The ADR 0076 schema-promotion pass (2026-04-26) added `$defs/CapabilityInvocationRecord` to `schemas/wos-workflow.schema.json` with a fixed `recordKind: "capabilityInvocation"` literal and two normative examples; nothing in CI ensured the corresponding `ProvenanceKind` variant landed. The same failure mode could recur for any future schema $def whose `properties.recordKind.const` (or `enum: [literal]`) introduces a new wire-shape obligation.

A schema↔enum parity gate is the durable fix: walk `wos-workflow.schema.json` $defs, extract every record-shape $def whose `recordKind` is pinned to a literal string, and assert each literal has a matching `ProvenanceKind` variant under `serde(rename_all = "camelCase")` in `crates/wos-core/src/provenance/kind.rs`. Same pattern as `scripts/check-canonical-seams.py` landed 2026-04-28 for ADR 0077 (kernel extension-seam vocabulary lock). Filed in `TODO.md` Hygiene as **#68**.

## Follow-ups filed

| Gap | Item | Priority | Where | Score |
|---|---|---|---|---|
| 1 | Add `ProvenanceKind::CapabilityInvocation` variant + constructor + AI runtime emission + conformance fixture | HIGH | new `TODO.md` Do-next **#2** | `[6 / 3 / 4]` (24) |
| 2 | Wire `TaskSkipped` emission (skip-path lifecycle) | MEDIUM | already at `TODO.md` backlog **#66e** (audit cross-ref added) | covered |
| 3 | Configuration-warning emission discipline (4 MUSTs, lean generic `ConfigurationWarning` kind) | LOW | new `TODO.md` Behavioral / governance **#67** | `[4 / 3 / 3]` (12) |
| Meta | Schema↔enum drift lint for ProvenanceKind (catch future Gap-1-shape regressions) | MEDIUM | new `TODO.md` Hygiene **#68** | `[5 / 2 / 4]` (20) |

## Net change

`TODO.md` Do-next item *Provenance emission completeness audit* retired. Three new items filed (Gap 1 in Do-next, Gap 3 in Behavioral / governance, Meta in Hygiene). Existing backlog **#66e** cross-referenced. Verifiability-closure section's duplicate audit entry collapsed to a closed reference. No code change in this audit pass.
