# WOS TODO

**Last audited:** 2026-04-20 (session 6 close — §5.4 Task 7 synth-trace schema + drift test, §4.1 #2 deterministic adverse-decision notice, §4.2 #21 extension-registry MVP, §4.2 #39 continuationPolicyRef, §4.2 #37 Drift-Monitor policyRef, §4.3 #13 verifiability principle, §4.3 #57 assurance x-lm.critical all landed; metrics below reflect current tree, not last session's)

**Snapshot**

| Metric | Value |
|---|---|
| Specs / schemas | 20 specs · 25 schemas (21 production + 4 meta: conformance / lint / mcp / synth) · 0 SCHEMA-DOC-001 violations across all (`all_production_schemas_have_zero_schema_doc_violations` CI gate) |
| Fixtures | 53 document + 150 conformance (147 top-level + 3 export); T3 green (`kernel_conformance` 133 passed, `trace_parity` 20 passed) |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) + 6 MVP (`wos-authoring` @ 50 tests, `wos-mcp` @ 22 tools, **`wos-synth-core` @ 13 tests, `wos-synth-mock` @ 3, `wos-synth-anthropic` @ 2, `wos-synth-cli`** — DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal \| grep -E 'reqwest\|tokio\|anthropic'`) + 1 spike (`wos-synth-spike`) |
| Lint matrix | 100 rules in `LINT-MATRIX.md` (35 T1 · 56 T2 · 9 T3 · 10 Tested · 90 Draft; regenerated from code registries) |
| CI gates | `schema_doc_zero_regression` (all 21 production schemas) · `every_promoted_*_rule_has_executable_or_annotated_evidence` (Tested/LoadBearing) · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Links:** [Core extraction plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Completed archive](COMPLETED.md) · [ADR 0065](../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

---

## Next actionable work items (ordered by ROI)

> Session 6 landed 2026-04-20: §4.1 #2 deterministic adverse-decision notice (uncommitted at time of audit but test-green end-to-end — `cargo test -p wos-conformance --test kernel_conformance g002_notice_before_adverse` passes), §5.4 Task 7 synth-trace JSON Schema + drift test (`f7c6c86`), §4.2 #21 extension-registry MVP (`3550fad`), §4.2 #39 continuationPolicyRef binding (`eaa678d`), §4.2 #37 Drift-Monitor policyRef (`b077613`), §4.3 #13 verifiability principle (`31a0e21`), §4.3 #57 assurance `x-lm.critical` coverage (`a1100fe`). §4.1 critical path now reduces to DRAFTS triage + #20 typed events.

1. **§4.1 #20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union. Closes the kernel's last load-bearing openness. DRAFTS triage complete (2026-04-20) — unblocked.
3. **§4.4 Split release trains** — unblocked since §4.2 close. See [plan](thoughts/plans/2026-04-16-wos-release-trains.md).
4. **§5.5 Synthesis benchmark (`wos-bench`)** — unblocked since §5.4 scaffold complete (Tasks 1–7). See [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md).
5. ~~**v0 spike Tasks 4-5**~~ — landed 2026-04-20 (`f6320c2` + `a80e37d`). Conformance smoke-test gate lands inline-fixture wrapper pattern; retrospective at [`thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md`](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md). Live-run iteration counts (Q-V0-1..4) flagged as follow-up.
6. **§4.3 cheap batch complete 2026-04-20** — #12 capability preconditions (schema + spec §3.3.1 + wos-core model + AI-057 parse-validity lint) and #56 isolation-invariant lint rule (K-049) both landed. #34 already covered by `SCHEMA-DOC-001`; #42 landed in session 5.

---

## 2026-04-20 session 6 — §5.4 Task 7 + §4.1 #2 + §4.2/§4.3 batch closure

Eight commits + in-flight #2 working tree.

### §5.4 Task 7 synth-trace schema + drift test (`f7c6c86`)

`schemas/synth/wos-synth-trace.schema.json` publishes the `SynthTrace` and `SynthOutcome` contract for `wos-synth explain` artifacts. `crates/wos-synth-core/tests/trace_schema_drift.rs` validates representative `SynthTrace`, converged `SynthOutcome`, and unconverged `SynthOutcome` serde output against the published schema, including optional `conformance`, `path`, `suggested_fix`, and `related_docs` fields. Verified locally with `cargo test -p wos-synth-core --test trace_schema_drift` (3 passed). The provider-DIP invariant also remains clean: `cargo tree -p wos-synth-core --edges normal | rg 'reqwest|tokio|anthropic'` produced no matches.

Semi-formal review follow-up fixed two adjacent synth issues before moving on: `wos-synth explain` now prints each iteration's conformance status, and `strip_fences` strips non-JSON language tags such as `wos`.

### §4.2/§4.3 single-commit items

- **`3550fad`** — **#21 Extension registry (seams-only MVP).** `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md` catalog the six kernel seams (§10.1 actor-extension, §10.2 contract-hook, §10.3 provenance-layer, §10.4 lifecycle-hook, §10.5 custody-hook, §10.6 vendor-extension) plus the Trellis custody shape. `RegistryEntry` `$def` has lifecycle (draft/stable/deprecated/retired), composition (merge/replace/augment), `since`/`replacedBy`/`vendorPrefix`. Descriptive, not enforcement.
- **`eaa678d`** — **#39 ContinuationPolicy normative linkage.** `AppealMechanism.continuationPolicyRef` (optional, `x-lm.critical`) resolves to `ContinuationPolicy.id` (now REQUIRED). `continuationOfServices: true` with neither ref nor scope resolving is a configuration error. Governance §3.6 prose added.
- **`b077613`** — **#37 Drift Monitor demotion policy binding.** `AlertThreshold.policyRef` (optional, `x-lm.critical`) resolves to `DemotionRule.id` (now REQUIRED) in the Agent Config sidecar. Named rule's structured semantics take precedence over the `action` enum; unresolvable ref falls back with a provenance warning. Drift-monitor §1.4.1 prose added. Combined with session-5 AI-AUTO-001/002 fixtures, this closes the full §4.2 #37 slice.
- **`31a0e21`** — **#13 Verifiability test principle.** Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2. Doc-only.
- **`a1100fe`** — **#57 Assurance schema `x-lm.critical` coverage.** Annotations on `assuranceLevel`, `subjectContinuity.{reference,scope}`, `disclosurePosture`, `attestation.{subject,predicate,basis}`. The only schema in the suite with zero annotations now has them; `schema_doc_zero_regression` stays green.

### §4.1 #2 Deterministic adverse-decision notice (in-flight at audit time)

**Status: test-green end-to-end, not yet committed as of 2026-04-20.** The working tree carries the full emitter: `ReferenceCompanionPolicy::deterministic_adverse_decision_notice_input` (new, +358 lines in `crates/wos-runtime/src/companion.rs`) detects active `adverse-decision` transitions covered by `dueProcess.adverseDecisionPolicy.noticeRequired`, captures the pre-transition case-file snapshot, resolves the Notification Template sidecar by `noticeTemplateRef`, renders deterministic human-readable prose, and passes `AdverseDecisionNoticeInput` (new type in `wos-core/src/event_handler.rs`) into `evaluate_event`. The emitted `noticeSent` provenance record carries `data.source="deterministic"`, `data.machineReadable.kind="adverseDecisionNotice"`, `snapshotSha256`, `templateRef`, transition metadata, appeal configuration, and the human-readable notice. Governance §3.2 schema + spec updated. `g-002-notice-before-adverse.json` expanded to inline governance + notification-template documents and asserts the deterministic artifact + snapshot digest (`aedb224cf7...`). `cargo test -p wos-conformance --test kernel_conformance g002_notice_before_adverse` passes. Commit pending.

---

## 2026-04-19 session 5 — §4.2 #37 / #46 / §4.1 #24a closure (10 commits, landed)

Single-stream closeout of the resumable §4.2 work paused in session 4.

### #37 Drift Monitor demotion policy binding

Added two executable T3 fixtures and expected traces for autonomy demotion behavior: `AI-AUTO-001-escalation-expiry-revocation` and `AI-AUTO-002-drift-alert-demotion` (commit `4f10241`). The fixtures lock the two missing autonomy-lifecycle revocation paths: escalation-expiry demotion emits `autonomyDemotion`, and drift-alert demotion emits `autonomyDemotion` + `driftReclassification` while rerouting through `escalated` to land on human review. `crates/wos-conformance/src/rules.rs` registers both as Tested, `trace_parity.rs` runs both parity and honest-runtime-engagement checks, `crates/wos-runtime/src/companion.rs` gained `resolve_drift_demotion_policy_ref` + `annotate_autonomy_demotion_policy_ref` to bind the DriftMonitor `AlertThreshold.policyRef` onto `autonomyDemotion` records, and `fixtures/conformance/expected-traces/README.md` documents why the AI-AUTO-002 golden is intentionally thin (drift-alert demotion does not emit policy-application records). `LINT-MATRIX.md` is regenerated to 100 rules / 9 T3 after all three new T3 rules land.

### #46 Schema-prose enum alignment batch

Tightened the schema side of the alignment batch: `CaseRelationship.type` and `HoldPolicy.holdType` now enforce standard enums or `x-` vendor extensions; `AppealMechanism.reviewerConstraint` is required and closed to the shared independence vocabulary; `AppealMechanism.continuationScope` is closed to the due-process continuation vocabulary; duration fields are constrained to the runtime-supported ISO 8601 grammar; `DelegationScope.conditions` cites the shared FEL evaluation contract. The Drift Monitor `AlertThreshold.policyRef` prose/schema binding had already landed before this closeout and is now covered by AI-AUTO-002.

### #24a Facts-tier input snapshot

Closed the last prerequisite for #2 / #5: Kernel §8.2.1 requires `transitionTags` plus `caseFileSnapshot` for determination-tagged transitions (commit `3de0a89`); the new `schemas/kernel/wos-provenance-record.schema.json` owns `FactsTierRecord` / `CaseFileSnapshot` `$defs` with a pytest contract at `tests/schemas/test_facts_tier_snapshot.py` (commits `7dffcd7`, `cef083b`); `CaseFileSnapshot` in `wos-core` uses RFC 8785 JCS canonicalization (`serde_json_canonicalizer` 0.3) plus SHA-256 (commit `9080f56`); `Evaluator` threads `transitionTags` through `ObservedTransition` and captures a per-transition pre-action snapshot at `case_file_snapshot_for_transition` (commit `91fe462`); `wos-runtime` emits the snapshot via `ProvenanceRecord::tagged_state_transition` (commit `5df72c4`). Added executable T3 rule `K-DET-001` plus `k-det-001-determination-snapshot.json`, asserting transition tags, snapshot value, canonical JSON, and digest (commit `a004e8e`).

### Review cycle + parser hardening

Two independent semi-formal code reviews ran over the WIP. Round 1 (9 findings) surfaced misleading claims about JCS canonicalization, orphan schema `$defs`, and missing per-transition snapshot coverage — all either resolved by verification tests or already fixed in the tree. Round 2 (5 findings) caught a real blocker: this WIP had expanded `crates/wos-conformance/tests/fixtures/g-002-notice-before-adverse.json` to expect a deterministic `noticeSent` shape (`data.source="deterministic"`, `machineReadable.kind`, `humanReadable` prose, `snapshotSha256`) that no runtime emitter produces. The fixture expansion was reverted; see the scope note under #2 below. Round 2 also flagged that `crates/wos-core/src/eval.rs` `parse_iso_duration_to_ms` silently parsed `P20BD` / `PT5Q` as 0ms — fixed to return `Err` on unknown unit letters (commit `ba50102`), along with threading `current_time_ms` into `simulated_time_ms` (pre-existing clock-zeroing bug). `fixtures/validation/foia-kernel-llm-test.json` switched `P20BD` → `P20D` with `x-durationNote` vendor extensions pending a business-calendar sidecar binding.

Validation at close: `cargo test --workspace` green (33 test binaries, 0 failures); `python3 -m pytest tests/ -q` green (121 passed / 11 skipped / 1 xfailed).

---

## 2026-04-18 session 4 — wos-synth scaffold + §4.1 chain unblocking (7 commits)

Single-stream session, ROI-prioritized after a "user value, not chores" steer. Six work-product commits + one review-fix commit. Semi-formal review verdict went from REQUEST CHANGES → APPROVE after the fix commit.

### §5.4 wos-synth four-crate scaffold (Plan Tasks 1-6)

`6409006` — created `wos-synth-core` (loop + `Prompter` trait + `ToolContext` trait + prompt templates + `DirectToolContext` stopgap), `wos-synth-mock` (deterministic test prompter), `wos-synth-anthropic` (streaming-callback Anthropic provider), `wos-synth-cli` (binary `wos-synth` with `generate` / `dry-run` / `explain`). DIP invariant: `cargo tree -p wos-synth-core --edges normal | grep -E 'reqwest|tokio|anthropic'` returns empty. End-to-end `dry-run` produces a kernel doc that lints clean without touching the network. Plan Task 7 (synth-trace JSON Schema + drift test) deferred — tracked as next-actionable item #3.

### §4.1 chain — two of three blockers cleared for #5

`dfd9189` — **NoticeTemplate reconciliation.** Dropped the thin `NoticeTemplate` `$def` from `wos-due-process.schema.json`; rich `TemplateSection`-based shape in `wos-notification-template.schema.json` is canonical. Zero in-tree consumers. `noticeTemplateRef` (Governance §3.1) and `notificationTemplateRef` (Governance §12.2) both already routed through the Notification Template sidecar via lint rule G-063. Spec prose updated with redirect note.

`62b1561` — **#23 OverrideRecord schema.** Typed `OverrideRecord` + `EvidenceReference` `$def`s in workflow-governance schema. Three required fields (rationale + authorityVerification + supportingEvidence) map 1:1 to OverrideAuthority policy switches. Authority verification typed via 4-variant method enum (roleAssignment | delegationGrant | supervisorAttestation | externalAuthority). Spec §7.3 prose links to typed shape. Reachable as a "shape catalog" `$def` (intentional unreachability annotated via `$comment`).

### §4.1 + §4.2 discrete user-value items

`44ac44c` — **#31 Jurisdiction-aware business calendar.** Replaced "implementation-defined" calendar selection (§7.1) with deterministic 6-step algorithm via optional `appliesWhen` FEL expression on each Business Calendar. Multi-jurisdiction rights-impacting workflows (one calendar per US state) now have a declarative selection mechanism. Timezone disagreement among applicable calendars is a configuration error — surfaces modelling mistakes at evaluation time.

`64b03a5` — **#29a Milestone trigger-mode spec-lag closure.** Promoted runtime KS.2 behavior into authoring-visible form: `Milestone.triggerMode: writeSettled` (default-only enum, extensible). Spec §4.13 gained "Trigger semantics" paragraph naming the three runtime invariants (fire-after-settled-write, at-most-once-per-instance, lexicographic id ordering). Wos-core `Milestone` struct picked up the optional field. Unblocks #29b reactive milestone firing.

### Semi-formal code review (REQUEST CHANGES → APPROVE)

`b824927` — addressed review findings from sub-agent review of the four scaffold/schema commits. **Blocker (Finding 1)** fixed: `EvidenceReference` schema now has `required: ["kind"]` and `anyOf: [{required: ["caseFieldPath"]}, {required: ["uri"]}]` — the prose contract "evidence MUST be locatable" is structurally enforced. New pytest file `tests/schemas/test_override_record_shape.py` (12 cases) locks both EvidenceReference and OverrideRecord required-field contracts. **Warnings (Findings 2/3/4/5/6/7)** fixed: AnthropicPrompter `Arc::try_unwrap` → `mem::take` (no more discarded paid completions); `strip_fences` no-newline regression repaired with 6 new tests; `LintFinding` extended with `suggested_fix` + `related_docs` plumbed into the repair prompt; ScriptedPrompter/Tools converted to VecDeque + pop_front; trace explain prints "unknown" instead of misleading 0/0/0 token totals; OverrideRecord orphan-`$def` annotated. **Nit (Finding 10)** fixed: `anyhow_lite` rationale documented inline. **Deferred:** Findings 8/9/11/12/13/14 (lower-impact nits and observations).

---

## 2026-04-18 session 3 — parallel-agent close (~50 commits)

Five concurrent work streams executing simultaneously; all 5 previous "Next actionable items" from session 2 complete. Aggregate: ~50 commits on `main`. Working tree clean at close.

### §5.1 Schema description audit — all tiers at 0 violations

Companions tier (18 violations → 0): 6 commits `4d88dfc` → `3384049` — lifecycle-detail, case-instance top-level, ActiveTask, FormspecTaskContext, ValidationOutcome/Compensation/PendingEvent, GovernanceState/Delegation/Hold/Volume.

Governance tier (159 violations → 0): 4 commits `a9e400b` (assertion-gate, 17 → 0), `a62c30b` (policy-parameters, 23 → 0), `e0d2637` (due-process, 27 → 0), `18e7831` (workflow-governance, 92 → 0).

AI tier (148 violations → 0): 3 commits `595e6f3` (agent-config, 35 → 0), `7339e99` (drift-monitor, 25 → 0), `4d22710` (ai-integration, 88 → 0).

Profiles/sidecars/assurance/advanced (299 violations → 0): 8 commits — `4b530a7` (business-calendar, 14 → 0), `65e3acd` (notification-template, 16 → 0), `61cf062` (assurance, 13 → 0), `bdcb082` (semantic-profile, 28 → 0), `68b07b8` (integration-profile, 48 → 0), `39a7519` (verification-report, 26 → 0), `d42c25c` (equity-config, 32 → 0), `ccd1199` (advanced-governance, 122 → 0).

CI gate: `3493f92` — `schema_doc_zero_regression` test walks `schemas/**/*.schema.json` and fails if any production schema has violations. Covers all 18 production schemas. `wos-mcp-tools.schema.json` was initially excluded, then backfilled (`a87cf30`) and added to the gate. A kernel schema (`wos-correspondence-metadata.schema.json`) had 17 latent violations discovered by the gate and fixed inline.

Reviews: all tiers APPROVED (companions, governance, rest); minor nits all resolved inline.

### §5.3 Trace-emitting conformance — Tasks 4-5 done

Task 4 CLI subcommands `wos-conformance-explain` + `wos-conformance-diff` landed: `842c9ee` (library: trace rendering + diffing) + `09de276` (binaries). Review nits resolved: source_actor comparison added with new `DivergenceCause::ActorMismatch` variant (`103b6af`), whitespace normalized (`de3e848`), binary exit-code contract pinned via `assert_cmd` (`27c4f00`).

Task 5 `schemas/conformance/conformance-trace.schema.json` published (`96c6acf`) with Python pytest validation of all 7 golden traces.

### §4.2 Rule-coverage conformance — Tasks 4-7 done

Task 4 `wos-rule-coverage` coverage library + binary: `6b51bb2` (library) + `ecd4218` (binary). Review nits resolved: `--help` exits 0 (`c5cb23f`), `path_is_referenced` tightened (`b160dee`), `is_expected_traces_path` hardened (`d1d7749`).

Task 5 LINT-MATRIX regen tool landed + matrix regenerated from 197 aspirational rows → 97 code-registry reality: `082fa06` (--generate-matrix flag) + `e2be40a` (matrix regenerated).

Task 6 LoadBearing ≥2-fixture CI gate: `e5eecee`.

Task 7 promotion-candidate discovery + GitHub Actions workflow: `679bcd7`.

### wos-mcp crate — Tasks 3-6 complete (22 tools, round-trip tested)

Task 3 document management: `064099f` (new_kernel() + from_document() factory methods) + `1fdb321` (project registry + document-management tools). Post-review nits: `63c01c3` (dead dep, handler signatures, ai_agent_count TODO, path load test).

Task 4 lifecycle/actor: `fb9cde8` (set_initial_state, add_actor_extension, richer add_state) + `4d66d0a` (lifecycle + actor tools). Post-review: `4bb411c` (self-loop transition count in apply_remove_state), `1e11940` (missing-arg vs. invalid-arg classification in require_*_arg helpers), `92741a0` (advertise Task 4 tools in tools/list discovery for 6 new tools).

Tasks 5-6 governance/AI + validation/query + tool catalog: `3d6cb6c` (governance + AI authoring helpers in wos-authoring), `9a97ecf` (governance + AI tools), `09926aa` (validation + query tools + tool catalog schema), `f1a4537` (round-trip integration test). Post-review blocker: `307d55e` (`dueProcesePaths` → `dueProcessPaths` spec-fidelity rename, touched 7 files across wos-authoring + wos-mcp), plus `a23ddd1` (Engine error classification as Internal not InvalidArguments), `11065f6` (remove dead unwrap_or in wos_search constraint branch), `681cf21` (useless_format clippy warning + test intent clarification).

Final state: 22 tools, round-trip test at `crates/wos-mcp/tests/round_trip.rs`, tool catalog schema `schemas/mcp/wos-mcp-tools.schema.json` at 0 SCHEMA-DOC-001 violations.

### §5.2 Structured lint diagnostics — Tasks 3-6 done

Task 3 rule migration (largest task): 92 unique rule IDs / 110 push sites migrated across `tier1.rs` (41 rules), `tier2.rs` (38 rules), `fel_analysis.rs` (12 rules), `schema_doc.rs` (1 rule). Commits: `0a25e4b` (Tier 1 migration), `d635a6d` (Tier 2 migration), `262ed7a` (FEL analysis migration), `220bee6` (SCHEMA-DOC-001 migration).

Task 4 output formatters (text / JSON / SARIF 2.1.0): `ab89dae` landed the `LintDiagnostic` constructors + text/json/sarif formatters alongside §5.2 Tasks 2-4; the session 3 rule migration (below) consumed them.

Task 5 `schemas/lint/wos-lint-diagnostic.schema.json` published: `d2a3fc1`.

Task 6 `crates/wos-lint/MIGRATION.md` migration guide: `5be48b8`.

Plan status updated: `03d9768`.

---

## 2026-04-18 session 2 — code review of 2026-04-17/18 parallel-agent batch (6 APPROVE, 1 REQUEST CHANGES → resolved)

Seven parallel semi-formal code reviews on the 14-commit batch delivered by 7 sonnet sub-agents executing v0 scopes of wos-authoring / wos-mcp / wos-synth-spike / §4.2 Task 2 / §5.1 Task 2 / §5.2 Tasks 1-2 / §5.3 Tasks 2-3. **Aggregate: 6 of 7 REQUEST CHANGES, 1 APPROVE (§5.1 triage).** Five real blockers across three work units, plus supporting warnings.

### Blockers (ordered by damage if not fixed)

> **Status (session 2 close, 2026-04-18):** ALL FIVE blockers cleared + post-review follow-ups + cleanup-review nits landed. §5.3 teaching signal end-to-end. Verdict on the follow-up review: APPROVE.

**Blockers cleared**

- **Blocker 1 — §5.3 teaching signal** (`b0b9ac5` + `95b88e9` + `b28f610` + `120086e`, follow-up `742373c`). Evaluator now captures `GuardEvaluation` records (including short-circuited false guards); `DrainOnceResult.guard_evaluations` plumbs them through runtime; `TraceStep.guards_evaluated` populated per step; `Delta::GuardFalse` enriched when an expected transition's guard blocks; `TraceStep.policies_applied` synthesized from governance/AI provenance via canonical `ProvenanceKind::is_policy_application()`. Single source of truth: `wos-conformance::trace::GuardEvaluation` re-exports `wos_core::eval::GuardEvaluation` (with `source_state` / `target_state` / `event` fields added to carry the teaching signal). Seven goldens regenerated (additive).
- **Blocker 2 — §5.2 `SuggestedFix::Custom` panic** (`0f6f049`, plan synced `935dce9`). Tuple variant `Custom(String)` converted to struct variant `Custom { hint: String }` with serde round-trip test.
- **Blocker 3 — wos-authoring `Command` visibility** (`9470b14` + `a42c281`). Enum sealed `pub(crate)`; `lib.rs` re-export dropped; `dispatch` moved onto inherent `pub(crate)` method of `RawWosProject`; `AppliedCommand::inverse` / `with_inverse` tightened. `cargo check -p wos-authoring --tests` warning-clean. Closes the prior session-review Finding 1.
- **Blocker 4 — wos-authoring plan used fictitious enum variants.** Plan realigned to real `ActorKind = Human | System` (AI agents route through `x-wos-ai.agents`; custom kinds through §10.6 `actorExtension` seam) and real `ImpactLevel = RightsImpacting | SafetyImpacting | Operational | Informational`.
- **Blocker 5 — §5.3 T3 fixture data-path mismatch** (6 happy-path fixtures across two passes: `0f7e27b` + `0b61e96` + `ddd25d3` + `56369bf`). (a) `initial_case_state` bridges added so guards evaluate; (b) `expected_provenance` migrated from legacy `type`/`from`/`to` to serde `recordKind`/`fromState`/`toState`; K-046 + G-030 `expected_transitions` expanded to full sequences. All six happy-path goldens now `outcome: pass` with honest step counts (1, 5, 2, 1, 7, 12); K-001 correctly stays `fail/0` as a negative lint fixture. Six new `happy_path_*` tests assert runtime engagement + `outcome=Pass`.

**Follow-up fixes from the post-blocker review cycles**

- **`742373c` — §5.3 semi-formal-review fixes.** (a) `policies_applied` was silently empty on real fixtures because governance constructors set `event: None` and the trace extractor filtered on event match; runtime `drain_once` now stamps the drain's event onto policy-kind records. (b) Policy-id extractor widened from `ruleId`/`policyId` to also include `constraintId` (what governance actually emits), `id`, `tool`, with camelCase-kind fallback for aggregate records (DeonticResolution etc.). (c) `Delta::GuardFalse` carries `expression` for disambiguation when two transitions share `(from, target, event)`. (d) `build_guard_inputs` handles `[*]` wildcard paths (guards using `every()` / `some()`). (e) Stale "opaque wos_runtime" docstring replaced. (f) `DrainOnceResult` struct-literal uses `..Default::default()`. Four new tests including end-to-end AI-014 governance fixture that would have been red before the stamping fix.
- **`ef0da3c` — cleanup-review nits.** `wos-lint::document` got the TODO comment that `wos-synth-spike::loop_mod::MISSING_MARKER_SENTINEL` references. `wos-mcp/src/server.rs` header no longer asserts rust-mcp-sdk is "too heavy" — points at the `Cargo.toml` TODO with the corrected feature-flag analysis.
- **`b203c29` + `6e83cdf` — §4.2 Task 3 CI ratchet.** `every_promoted_*_rule_has_executable_or_annotated_evidence` test added to both `crates/wos-conformance/tests/rule_registry.rs` and `crates/wos-lint/tests/rule_registry.rs`: every `Tested`/`LoadBearing` entry requires a resolvable executable fixture OR an evidence-annotation comment (mirroring AI-004 / AI-050 / K-EXT-002 / G-052 pattern). AI-041 evidence annotation added as a TDD discovery.

**What this unblocks**

- §5.4 repair prompts now have a real teaching signal (guards + policies are populated end-to-end).
- §4.2 `Tested` → `LoadBearing` promotions are now CI-gated.
- wos-mcp Tasks 3-6 can safely depend on the sealed `WosProject` façade.

### Session 2 warnings — all resolved in session 3

All warnings from the original review (wos-mcp JSON-RPC hygiene, v0 spike model + error classification, §4.2 fixture-link evidence gaps, §5.2 LintDiagnostic latent issues) were addressed before session 3 close. See the "New work items added by this review" list below for resolution commits.

### Non-blocker observations worth knowing

- **§5.1 triage** (`1e37b56`) — APPROVE. Minor factual corrections (K-023 is a crash-recovery conformance test, not a url lint rule; `$schema` descriptions vary more than "all missing or 47-char" claimed; 20 kernel fixtures reference `url` not 14; `title` count 15 not 16). Classifications are defensible; reshape wins validated (ExtensionsMap x30, JsonSchemaUri x18, bare items x39).
- Pre-bump of workspace `Cargo.toml` (commit `b5cb7e2`) successfully prevented the parallel-dispatch race the practices doc was written for. Validated discipline.

### New work items added by this review

- ✅ **§5.3 runtime instrumentation** — landed `b0b9ac5` + `95b88e9` + `b28f610` + `120086e` + follow-up fix `742373c`. DrainOnceResult.guard_evaluations carries GuardEvaluation records from the evaluator; build_trace_from_result populates TraceStep.guards_evaluated per-step and enriches Delta::GuardFalse (with `expression`) when an expected transition's guard evaluated false. PolicyApplication synthesized from governance/AI provenance records via the canonical `ProvenanceKind::is_policy_application()` method in wos-core; policy-id lookup covers `ruleId`/`policyId`/`constraintId`/`id`/`tool` with camelCase-kind fallback. Wildcard path dependencies (`[*]`) handled. Seven T3 goldens regenerated (additive only). §5.4 repair prompts now have a real teaching signal end-to-end.
- ✅ **§5.3 fixture repair** — resolved in session 2 (six happy-path T3 fixtures repaired with `initial_case_state` seeding + `expected_provenance` shape migration; see session-1 summary above).
- ✅ **§5.2 `Custom` variant fix** — landed `0f6f049`, plan sketch updated `935dce9`.
- ✅ **wos-authoring pre-Task-4 fixes** — `Command` sealed `pub(crate)` (`9470b14` + `a42c281`); plan realigned to real `Human | System` ActorKind and `RightsImpacting | SafetyImpacting | Operational | Informational` ImpactLevel.
- ✅ **wos-mcp hygiene pass** — 6 commits `bf86853` + `d6377ce` + `732f848` + `68f5d26` + `5394d15` + `e1530d9`: notifications, JSON-RPC error codes, dead `ServerError` removed, test timeout, handler signature aligned, rust-mcp-sdk TODO with feature-flag analysis. Review-approved with 2 doc nits (fixed `ef0da3c`).
- ✅ **v0 spike warning fixes** — 3 commits `47677fa` + `e165dd7` + `add6796`: claude-sonnet-4-6 model bump; `SpikeError::MissingWosMarker` / `LintFailure` distinct variants; API-key guard rejects empty / whitespace-only. 15/15 tests green. Review-approved.
- ✅ **§4.2 registry annotations** — 3 commits `d86f3df` + `f96f6c5` + `91602de`: K-EXT-002 (wos-lint tier2 inline evidence); AI-001 / AI-002 (indirect fixture linkage via batch number); G-052 (expanded to exhaustive 20-fixture listing). Review-approved.
- ✅ **§4.2 Task 3 CI ratchet** — 2 commits `b203c29` + `6e83cdf`: `every_promoted_*_rule_has_executable_or_annotated_evidence` in both conformance and lint registry tests; AI-041 annotation added as a TDD discovery.
- ✅ **COMP-001 companion drift lint rule blocker cleared** — §5.2 Task 3 rule migration landed in session 3; COMP-001 is now unblocked for implementation.

---

## Current plan status (2026-04-18)

Legend: ✅ landed · 🟡 partial · 🔴 not started · 🚨 has blocker from review

- ✅ **§4.1 Extension fix** — 19 schemas patched at every nested level. §10.6 amended. K-EXT-002 lint rule landed (`5689d3c`) with review finding: linked fixtures not executed by harness. K-EXT-001 subsumed by schema `patternProperties`.
- ✅ **§4.3 Precedence clause** — Added to both companions. COMP-001 drift-detection rule now unblocked (§5.2 Task 3 landed in session 3).
- ✅ **§4.2 Rule-coverage conformance** — [plan](thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md). All 7 tasks done. **Task 1** (metadata registry, 97 entries) `1f8eae5`. **Task 2** (fixture-link backfill) `45e654d` + `bcaa294`. **Task 3** CI ratchet `6e83cdf`. **Task 4** coverage CLI `6b51bb2` + `ecd4218` + fixes `c5cb23f` + `b160dee` + `d1d7749`. **Task 5** LINT-MATRIX regen `082fa06` + `e2be40a` (197 → 97 reconciled rows). **Task 6** LoadBearing CI gate `e5eecee`. **Task 7** promotion-candidate discovery `679bcd7`. §4.4 is now unblocked.
- ✅ **§4.2 #46 Schema-prose enum alignment batch** — session 5 closeout. `CaseRelationship.type` and `HoldPolicy.holdType` enforce standard enums or `x-` vendor extensions; `AppealMechanism.reviewerConstraint` is required and closed to the due-process independence vocabulary; `AppealMechanism.continuationScope` is closed to `currentLevel | reducedLevel | emergencyOnly`; duration fields are constrained to the runtime-supported ISO 8601 grammar; `DelegationScope.conditions` cites the shared FEL evaluation contract; Drift Monitor `AlertThreshold.policyRef` table/prose already landed and is covered by #37.
- 🔴 **§4.4 Split release trains** — [plan](thoughts/plans/2026-04-16-wos-release-trains.md). Changesets + per-stream git tags mirroring ADR 0063. **§4.2 is now complete — this is unblocked.**
- ✅ **§5.1 Schema description audit** — [plan](thoughts/plans/2026-04-16-wos-schema-description-audit.md). All tiers at 0 violations, CI gate live. **Task 1** `03973e3`. **Task 2** triage `1e37b56`. **Task 3** reshape pre-pass `34eafe7` (901 → 815). **Kernel tier** `8f30886` + `bc064d5` + `c06cb40` + `5edb4ae` + `410b2d1` + `078c955` + `29d1ef6` (74 → 0). **Companions** `4d88dfc` → `3384049` (118 → 0). **Governance** `a9e400b` + `a62c30b` + `e0d2637` + `18e7831` (159 → 0). **AI** `595e6f3` + `7339e99` + `4d22710` (148 → 0). **Profiles/sidecars/assurance/advanced** `4b530a7` + `65e3acd` + `61cf062` + `bdcb082` + `68b07b8` + `39a7519` + `d42c25c` + `ccd1199` (299 → 0). **CI gate** `3493f92` + `a87cf30` (mcp-tools backfill). 22 total schemas at 0 violations (19 production + 3 meta).
- ✅ **§5.2 Structured lint diagnostics** — [plan](thoughts/plans/2026-04-16-wos-structured-lint-diagnostics.md). All 6 tasks done. **Tasks 1-2** `cfedab3` + `a71a154`. **Custom panic fix** `0f6f049`. **Task 2-4** (formatters: text/JSON/SARIF 2.1.0) `ab89dae`. **Task 3** rule migration — 92 rule IDs / 110 push sites: `0a25e4b` (Tier 1, 41 rules), `d635a6d` (Tier 2, 38 rules), `262ed7a` (FEL analysis, 12 rules), `220bee6` (SCHEMA-DOC-001, 1 rule). **Task 5** JSON schema `d2a3fc1`. **Task 6** migration guide `5be48b8`. Plan closed `03d9768`. COMP-001 companion drift lint rule now unblocked.
- ✅ **§5.3 Trace-emitting conformance** — [plan](thoughts/plans/2026-04-16-wos-trace-emitting-conformance.md). All 5 tasks done. **Tasks 1-3** (ConformanceTrace type + runner emission + golden traces) `bb1d323` → `d961c9f` → fixture repair → `b0b9ac5` + `95b88e9` + `b28f610` + `120086e` + `742373c`. Teaching signal fully operational: `guards_evaluated` + `policies_applied` populated; `Delta::GuardFalse` carries `guard_id` + `expression` + `inputs`. **Task 4** CLI subcommands `842c9ee` (library) + `09de276` (binaries) + `103b6af` (ActorMismatch) + `de3e848` (whitespace) + `27c4f00` (exit-code contract). **Task 5** conformance-trace schema `96c6acf`. §5.4 repair prompts can consume the full teaching signal end-to-end.
- ✅ **`wos-authoring` crate MVP** — [plan](thoughts/plans/2026-04-17-wos-authoring-crate.md). **All 8 Tasks landed.** Tasks 1-3 (`a33094d` + `f9c879c` + `daec5b8`) + sealing (`9470b14` + `a42c281`) + Tasks 4-8 (12 commits `0a124ca` → `46fc45a`) + review nits (`d46e26c`). Final state: 13-variant `pub(crate)` Command enum; 10 handlers (AddState/AddTransition/AddActor/RemoveActor/SetImpactLevel/AddContract/AddExtensionKey/AddActorDeontic/SetTimer/RenameState/AddMilestone/RemoveMilestone); snapshot-based undo/redo with `UNDO_DEPTH = 100` eviction cap (tested); public `WosProject` façade over private `RawWosProject`; README; end-to-end integration test covering guarded fork + multi-domain compose + undo/redo interleave + save→load roundtrip. 49 unit + 1 integration tests; `cargo check --tests` warning-clean. Public surface: only `WosProject`, `AuthoringResult`, `AuthoringDiagnostic`, `Severity`, and re-exported kernel enums. Review verdict APPROVE WITH NITS — F1-F4 (pub(crate) sealing, UNDO_DEPTH test, comment correction, plan milestone backfill) fixed in `d46e26c`. Unblocks wos-mcp Task 3 (can now safely depend on `WosProject`) and §5.4 synth-core.
- ✅ **`wos-mcp` crate** — [plan](thoughts/plans/2026-04-17-wos-mcp-crate.md). All 6 tasks done. 22 tools, round-trip test at `crates/wos-mcp/tests/round_trip.rs`. **Tasks 1-2** `cde0b04` + `53eb25f` + hygiene pass `bf86853` + `d6377ce` + `732f848` + `68f5d26` + `5394d15` + `e1530d9` + `ef0da3c`. **Task 3** `064099f` + `1fdb321` + nits `63c01c3`. **Task 4** `fb9cde8` + `4d66d0a` + fixes `4bb411c` + `1e11940` + `92741a0`. **Tasks 5-6** `3d6cb6c` + `9a97ecf` + `09926aa` + `f1a4537` + post-review `307d55e` (spec-fidelity rename) + `a23ddd1` + `11065f6` + `681cf21`. Tool catalog schema `schemas/mcp/wos-mcp-tools.schema.json` at 0 violations.
- ✅ **§5.4 `wos-synth-core` + providers** — [plan](thoughts/plans/2026-04-16-wos-synth-crate.md). **Plan Tasks 1-6 landed `6409006` (session 4):** four-crate split (`wos-synth-core` + `-mock` + `-anthropic` + `-cli`); DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal | grep -E 'reqwest|tokio|anthropic'`; 12+3+2 tests; CLI `dry-run` lints clean end-to-end without network. **Review fixes** `b824927`: EvidenceReference blocker (governance schema) + AnthropicPrompter Arc::try_unwrap fragility + strip_fences no-newline regression + lint suggested_fix/related_docs plumbing + VecDeque test pattern + token-count "unknown" display. **Task 7 verified 2026-04-20:** published `schemas/synth/wos-synth-trace.schema.json` plus drift test `crates/wos-synth-core/tests/trace_schema_drift.rs` covering `SynthTrace`, converged `SynthOutcome`, and unconverged `SynthOutcome`.
- 🔴 **§5.5 Synthesis benchmark (`wos-bench`)** — [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Depends on wos-synth-core.
- ✅ **§4.1 NoticeTemplate reconciliation** — `dfd9189` (session 4). Dropped thin `NoticeTemplate` `$def` from due-process schema; rich Notification Template sidecar canonical. Zero in-tree consumers; lint G-063 already routed both refs through the sidecar.
- ✅ **§4.1 #23 OverrideRecord schema** — `62b1561` (session 4). Typed `OverrideRecord` + `EvidenceReference` `$def`s in workflow-governance with 1:1 mapping to OverrideAuthority policy switches. Authority-verification typed via 4-variant method enum. Spec §7.3 prose links to typed shape. Pytest contract `tests/schemas/test_override_record_shape.py` (12 cases) added in `b824927` to lock the EvidenceReference locator + OverrideRecord required-field invariants.
- ✅ **§4.1 #2 Deterministic adverse-decision notice (dual-form)** — session 6 closeout. Runtime companion policy now emits deterministic `noticeSent` provenance for adverse-decision transitions with `noticeRequired`: machine-readable `adverseDecisionNotice`, `snapshotSha256`, transition metadata, appeal configuration, template ref, and human-readable prose rendered from the same Facts-tier snapshot + Notification Template inputs. G-002 asserts the deterministic artifact. `cargo test -p wos-conformance --test kernel_conformance g002_notice_before_adverse` green. **Working tree at audit time carries the full landing; commit pending.**
- ✅ **§4.2 #21 Extension registry (seams-only MVP)** — `3550fad` (session 6). Six kernel-seam enum values (actor-extension / contract-hook / provenance-layer / lifecycle-hook / custody-hook / vendor-extension) + Trellis custody shape in `wos-extension-registry.schema.json` and `specs/registry/extension-registry.md`. Catalog, not enforcement; closes the `custodyHook` prose-only escape.
- ✅ **§4.2 #39 ContinuationPolicy normative linkage** — `eaa678d` (session 6). `AppealMechanism.continuationPolicyRef` resolves to `ContinuationPolicy.id` (both marked `x-lm.critical`). Unresolvable refs emit configuration warnings with fallback to `continuationScope`; missing both is a configuration error. Governance §3.6 prose updated.
- ✅ **§4.3 #13 Verifiability test principle** — `31a0e21` (session 6). Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2. Doc-only.
- ✅ **§4.3 #57 Assurance schema `x-lm.critical` coverage** — `a1100fe` (session 6). Annotations on assuranceLevel, subjectContinuity, disclosurePosture, attestation triple. `schema_doc_zero_regression` stays green.
- ✅ **§4.3 #34 `x-lm.critical` enforcement gate** — effectively covered by `SCHEMA-DOC-001` + `all_production_schemas_have_zero_schema_doc_violations` CI gate. The rule requires `description ≥ 140 chars` + `examples ≥ 2` on every `x-lm.critical: true` leaf and the gate rejects any PR that regresses. No separate gate needed.
- ✅ **§4.3 #42 Autonomy-lifecycle conformance fixture batch** — session 5 closeout via AI-AUTO-001 (escalation-expiry revocation) and AI-AUTO-002 (drift-alert-triggered demotion). The two exact fixtures this item requested, both registered as Tested in `crates/wos-conformance/src/rules.rs`.
- ✅ **§4.1 #31 Jurisdiction-aware business calendar** — `44ac44c` (session 4). Replaced "implementation-defined" §7.1 selection with deterministic 6-step algorithm via optional `appliesWhen` FEL on each Business Calendar. Multi-jurisdiction rights-impacting workflows now have a declarative selection mechanism. Timezone disagreement among applicable calendars raises a configuration error.
- ✅ **§4.2 #29a Milestone trigger-mode spec-lag closure** — `64b03a5` (session 4). `Milestone.triggerMode: writeSettled` (default-only enum) names the runtime KS.2 behavior. Spec §4.13 gained "Trigger semantics" paragraph naming fire-after-settled-write + at-most-once-per-instance + lexicographic id ordering. Wos-core struct picks up the optional field (roundtrip safe). Unblocks #29b reactive milestone firing.
- ✅ **§4.2 #37 Drift Monitor demotion policy binding** — session 5 closeout. Drift Monitor `AlertThreshold.policyRef` resolves to Agent Config `DemotionRule.id`; `AI-AUTO-002` locks the drift-alert demotion path (`autonomyDemotion` + `driftReclassification` + event reroute through `escalated`), and paired `AI-AUTO-001` locks escalation-expiry revocation (`autonomyDemotion` on next invocation). Registry now has 99 rules / 8 T3; T3 green at 148/148.
- ✅ **§5.6 Repositioning docs** — README + POSITIONING lead with Claim A / Claim B framing.
- ✅ **§8 Open questions** — all 6 resolved 2026-04-17; doc archived at `thoughts/archive/reviews/2026-04-16-architecture-review-open-questions.md`.
- ✅ **Schema regression tests** — [plan](thoughts/plans/2026-04-17-wos-schema-regression-tests.md). 6 commits (`793e2e8` through `59bf25b`); 72 pytest cases pass, 2 skip, 1 xfail. Meta-validity + fixture validity + spec-example validity + negative fixtures + CI gate.
- ✅ **v0 spike** — [plan](thoughts/plans/2026-04-17-wos-synth-v0-spike.md). **Tasks 1-3** landed `26c7eaa` + `d2bb234` + `58fb369`: 529 LOC across 4 files (under 800 cap), lint-driven repair loop, 9 unit tests green. **All 3 warnings fixed 2026-04-18** (`47677fa` + `e165dd7` + `add6796` + `ef0da3c`). **Task 4** landed 2026-04-20 (`f6320c2`): conformance smoke-test gate using inline-fixture wrapper (empty event_sequence + empty expected_transitions); 2 new unit tests covering minimal-valid + dangling-initialState branches. **Task 5** landed 2026-04-20 (`a80e37d`): retrospective at [`thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md`](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md); plan propagations appended inline to wos-synth-crate, wos-synthesis-benchmark, and wos-mcp-crate. Open follow-ups: live Anthropic run for Q-V0-1..4; upstream `wos_conformance::smoke_test_document` helper; structured repair-prompt with rule_id + suggested_fix + spec_ref.

---

## 1 — Reference implementation blockers

> §1 closed 2026-04-14 — see [COMPLETED.md](COMPLETED.md).

---

## 2 — Foundational (zero external dependencies)

- [x] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES. Landed 2026-04-15 — see [COMPLETED.md § Provenance export](COMPLETED.md#provenance-export-pe).
- [ ] **Ontology field identity** *(design not started — do not sequence as active work)* — `ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and regulatory specs in §6, but cannot be scheduled until the spec is drafted. Prerequisite design work: JSON-LD `@context` decision (see Deferred #9), semantic-field-identity protocol, cross-document alignment mechanism. Move to active only once a draft exists.

---

## 3 — Engine adapters (open question — sequencing unresolved)

> **Status:** sequencing unresolved. TODO previously placed engine adapters as a near-term priority; IDEA_SCRATCH #49 marked them Defer with trigger "first commercial deployment requesting a specific adapter." No arbitrating document. Items kept in the backlog below but **not** scheduled until this question is resolved.

- [ ] **#49 Camunda 8 Worker** `[Imp 5 / Cx 8 / Debt 3]` — Delegate BPMN task execution under WOS governance. Most common BPMN target; broadest external fixture diversity.
- [ ] **#49 Temporal Workflow** `[Imp 5 / Cx 8 / Debt 3]` — Map WOS evaluation steps to deterministic replay. Natural fit with WOS evaluator determinism.
- [ ] **#49 AWS Step Functions** `[Imp 5 / Cx 8 / Debt 3]` — Bridge ASL states to WOS transitions. Broadest commercial reach; narrowest semantic fit.

---

## 4 — Active backlog (priority-ordered)

Previously split across "schema closures" and "behavioral specs." Collapsed and re-sorted 2026-04-16 by cost-to-defer + first-adopter enablement.

### Priority logic and scoring rubric

**Priority logic (2026-04-16 re-sort).** Two goals drive order: (A) reduce architectural lock-in while it's still cheap, (B) make WOS immediately usable by a first real adopter. Items are ranked by cost-to-defer, not cost-to-do. Cheap-and-cheap-forever items are bundled separately so they don't crowd the critical path. The prior Urgency formula from IDEA_SCRATCH (`(Imp+Debt)/Cx`) is retired — it over-rewarded low-Cx regression-prevention items. Scores `[Imp/Cx/Debt]` are preserved per item as metadata — they inform relative weight within each tier but do not override cross-tier ordering.

**Score definitions (0–10 scale):**

- **Imp** — **Importance.** How much does this item move the project forward (architectural leverage, first-adopter enablement, civil-rights/compliance weight). Higher = do it.
- **Cx** — **Complexity.** How much real work (design + implementation + test) this takes. Higher = bigger lift.
- **Debt** — **Architectural tech debt if deferred.** How much extra rework lands later if we don't do it now. Higher = cheaper now than later. Confined-scope fixes score low; load-bearing foundational items (0/N fixtures, unclosed escape hatches) score high.

**Score validation (2026-04-16).** Scores audited in parallel by four code-scout agents against live schemas, specs, crates, and fixtures. Adjustments applied: DRAFTS Debt 7→5, #24a Cx 3→4, #20 Cx 6→7, #46 Cx 2→3, #39 Cx 2→1, #12 Cx 2→3, #56 Debt 3→2, #35 Debt 5→4, #40 Debt 5→4, #30 Cx 4→5, #28 Debt 3→2, Assertion-Library merge Cx 3→2, #22 Cx 6→4, #48 Debt 4→6, #51 Debt 3→5. Factual corrections applied to #22 (runtime.rs lives in wos-runtime at 4451 lines, not wos-core at 3821; binding-inversion already landed), #28 (inputDigest/outputDigest already wired through export crate, not prose-only), #56 (continuous_reevaluate has 4 in-crate test callers, not "dead code").

### 4.1 — Critical path (lock-in + usable)

Items that get materially more expensive if deferred, or that block a first real adopter. Do these first.

- [x] **DRAFTS triage** — 2026-04-20. All 13 historical kernel drafts moved to `thoughts/archive/drafts/` with a README classifying each file (superseded kernel iterations, v7 reframe ancestors, tier-spec ancestors, schema snapshot). MD-INVENTORY §6 updated; IDEA_SCRATCH reference updated. Unblocked #20.
- [x] **#24a Mandatory Facts-Tier input snapshot** `[Imp 8 / Cx 4 / Debt 7]` — Tighten Facts Tier §8.2: case-file input snapshot MANDATORY and typed at `determination`-tagged transitions. Closed 2026-04-19 with schema/spec/runtime support plus executable K-DET-001 fixture coverage. Silent dependency of #2 now resolved.
- [x] **#23 OverrideRecord schema** — landed `62b1561` (session 4); see Current plan status. Pytest contract added `b824927`.
- [x] **NoticeTemplate reconciliation** — landed `dfd9189` (session 4); see Current plan status.
- [x] **#2 Deterministic adverse-decision notice (dual-form)** — session 6 closeout; see Current plan status. Runtime emitter + fixture + spec prose landed; `cargo test -p wos-conformance --test kernel_conformance g002_notice_before_adverse` green. Working-tree commit pending at audit time.
- [ ] **#20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. No `named` wrapper; no escape hatch. Co-type `Action.event` for `startTimer`. Closes kernel's last load-bearing openness. Migration surface is ~168 fixtures containing `"event":` strings (much larger than originally framed); plus schema + Rust model + K-007 lint promotion to schema validation. **Depends on DRAFTS triage.**
- [x] **#31 Jurisdiction-aware business calendar selection** — landed `44ac44c` (session 4); see Current plan status.

### 4.2 — Next (mostly closed — #22a is the only open item)

Five of six §4.2 items landed across sessions 4–6 (#21, #37, #39, #29a, #46). Only #22a remains, and it is lock-in-sensitive only in the sense that new `ProvenanceKind` variants must declare their tier; the "ossification" pressure was relieved by PE.2's `audit_layer_for_kind` exhaustive match.

- [ ] **#22a ProvenanceKind tier-typing** `[Imp 4 / Cx 4 / Debt 3]` *(extracted from #22; re-scored 2026-04-16 post-PE.2)* — Replace the 93-variant `ProvenanceKind` monolith enum (`crates/wos-core/src/provenance.rs`) with a tier-typed record (kernel / governance / ai / advanced). **Debt lowered 5→3:** PE.2 added the `audit_layer` field and an exhaustive `audit_layer_for_kind` match, so new variants must now explicitly declare their tier at compile time — the "ossification" pressure is partly relieved. Remaining value is data-shape cleanliness: separating record payloads by tier so each tier's struct carries only the fields it can populate. Still load-bearing for the broader #22 crate split but no longer urgent. The rest of #22 (directory split, runtime.rs split, CI fence) remains organizational and stays in §4.6.
- [x] **#46 Schema-prose enum alignment batch** — session 5 closeout; see Current plan status. `CaseRelationship.type`, `HoldPolicy.holdType`, `AppealMechanism.reviewerConstraint`, `AppealMechanism.continuationScope`, duration patterns, FEL context citation, and Drift Monitor `AlertThreshold` prose/schema alignment are closed.
- [x] **#21 Extension registry (seams-only MVP)** — `3550fad` (session 6); see Current plan status. Six kernel-seam enum + Trellis custody shape + lifecycle/composition/vendorPrefix metadata. Catalog-only (no enforcement).
- [x] **#29a Milestone spec-lag closure** — landed `64b03a5` (session 4); see Current plan status. Unblocks #29b.
- [x] **#37 Drift Monitor demotion policy binding** — session 5 closeout; see Current plan status. `AlertThreshold.policyRef` binds to named `DemotionRule.id`, with T3 evidence for drift-alert demotion and escalation-expiry revocation.
- [x] **#39 ContinuationPolicy normative linkage** — `eaa678d` (session 6); see Current plan status. `AppealMechanism.continuationPolicyRef` → `ContinuationPolicy.id`; misconfiguration-vs-error distinction spelled out in Governance §3.6.

### 4.3 — Cheap batch (ship together in one sprint)

Low-cost, low-risk, no lock-in. Independent of critical-path work — can land in parallel. Ordering within the batch doesn't matter.

- [x] **#34 `x-lm.critical` enforcement gate** — already enforced by `SCHEMA-DOC-001` (requires `description ≥ 140` chars + `examples ≥ 2` on every `x-lm.critical: true` leaf) combined with the `all_production_schemas_have_zero_schema_doc_violations` CI gate. No separate gate needed; see Current plan status.
- [x] **#57 Assurance schema `x-lm.critical` coverage** — `a1100fe` (session 6); see Current plan status.
- [x] **#13 Verifiability test principle** — `31a0e21` (session 6); see Current plan status. Kernel §1.2 + Governance §6.1 + AI Integration §1.2.
- [x] **#12 Capability preconditions** — 2026-04-20. Added `Capability.preconditions: array of FEL strings` with `x-lm.critical` annotation to `schemas/ai/wos-ai-integration.schema.json`. Spec §3.3 gained the property row; §3.3.1 (new) pins the normative semantics: all entries MUST evaluate to boolean `true`; unsatisfied → skip, fall through to fallback chain (§8); every evaluation emits a provenance record with `outcome: preconditionNotSatisfied` when blocked; preconditions do not relax deontic constraints. Wos-core `Capability` struct picked up `preconditions: Vec<String>` with `serde(default)`. AI-057 landed as a T2 error lint rule (parse-validity on each precondition entry) with 3 unit tests.
- [x] **#56 Runtime §2 isolation-invariant lint rule** — 2026-04-20. K-049 landed as a T2 warning: parses each transition guard via `fel-core`, collects `setData` write-paths, builds a directed write→read graph, runs iterative-DFS cycle detection, and flags any cycle when `evaluationMode: continuous`. Lives in `crates/wos-lint/src/rules/continuous_mode.rs` with 6 unit tests (self-loop, 2-node cycle, compound-nested cycle, event-driven skip, acyclic control, unparseable-guard graceful skip). Wired into `tier2::check`; registry entry added as `Tested`; LINT-MATRIX regenerated (101 rules / 11 Tested / 57 T2).
- [x] **#42 Autonomy-lifecycle conformance fixture batch** — session 5 via AI-AUTO-001 + AI-AUTO-002 (the two exact fixtures this item requested — escalation-expiry revocation + drift-alert demotion). Both registered as Tested rules in `crates/wos-conformance/src/rules.rs`.

### 4.4 — Behavioral backlog (after §4.1–§4.3 stabilize)

Specifies processor behavior, governance semantics, or runtime obligations. Not usability-critical, not foundational lock-in — schedule once the critical path and cheap batch have landed. Dependencies noted where they exist.

- [ ] **#26a `AccessControl.canRead` enforcement semantics** `[Imp 6 / Cx 3 / Debt 4]` — Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact / return `null` / raise error / skip action. Conformance fixtures per branch. Interface exists as pure stub today (defaults `true`, zero call sites). **Prerequisite to #26b.**
- [ ] **#26b `caseFieldPolicy` schema** `[Imp 6 / Cx 6 / Debt 4]` — `caseFieldPolicy` `$def` in workflow-governance schema; per-field read/write scopes by actor role. Governance-layer.
- [ ] **#36 Equity RemediationTrigger expression language** `[Imp 6 / Cx 4 / Debt 4]` — FEL extension vs. restricted DSL vs. FEL + windowing. **Prerequisite to #35.**
- [ ] **#35 Equity Config enforcement semantics** `[Imp 7 / Cx 5 / Debt 4]` — Specify processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime per `ReportingSchedule`; define "suspended workflow" behaviorally. Applies to human AND AI decisions. Runtime seam partially in place (`ProvenanceKind::EquityAlert`, lifecycle emission in `event_handler.rs`); behavioral enforcement still absent.
- [ ] **#24b + #25 joint design** *(rule-firing trace + defeasibility)* `[#24b: 7/6/4 · #25: 6/7/6]` — Reasoning Tier gains ordered rule list, intermediate state, outcome; Catala-style default logic with declared rule priorities. Load-bearing coupling — evaluation order requires defeasibility answer. Must compose with `sourceAuthority` rank (§6.2) and Integration Profile §11.2 ("restrict, never relax").
- [ ] **#43 Assurance × impact-level composition rule** `[Imp 6 / Cx 5 / Debt 4]` — Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact. Respect Invariant 6.
- [ ] **#38 Assertion Library cross-document reference protocol** `[Imp 5 / Cx 3 / Debt 3]` — `assertionId` on `PipelineStage.assertions[]`; resolution semantics. The library concept exists in prose; the reference mechanism doesn't.
- [ ] **#40 Task SLA authoring surface** `[Imp 6 / Cx 5 / Debt 4]` — Add schema properties for §10.3 normative prose (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`). Currently spec'd as normative processor behavior with no schema surface. Adjacent scaffolding exists (`sla-warning` category in notification-template schema; SLA-aware business calendar schema), which reduces retrofit cost if deferred.
- [ ] **#30 WS-HumanTask lifecycle completion** `[Imp 5 / Cx 5 / Debt 2]` — Extend 8-state model: task-level `Suspended`, distinct `Cancelled` terminal, explicit `Return` with rework counter, group-forwarding distinct from person-delegation.
- [ ] **#27 Cancellation regions** `[Imp 4 / Cx 6 / Debt 3]` — YAWL-style named region spanning arbitrary structural levels, fireable as a unit. Distinct from existing `cancellationPolicy` join policy.
- [ ] **#28 Claim-check artifact references** `[Imp 4 / Cx 4 / Debt 2]` — Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` as case-field value with normative integrity-check at retrieval. `inputDigest`/`outputDigest` fields are already wired through `ProvenanceRecord` and the export crate (`wos-export/src/{ocel,xes,prov_o}.rs`); remaining work is the `ExternalArtifactRef` type and population/retrieval contract.
- [ ] **#29b Milestone reactive transition firing (GSM-style)** `[Imp 6 / Cx 5 / Debt 2]` — `MilestoneFired` enqueues event, or `$milestone.*` FEL boolean for guards. Ships after #29a.
- [ ] **#3 Policy-based migration routing** `[Imp 5 / Cx 6 / Debt 2]` — `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. Composes with Governance §2.9. **Open sub-questions:** `tenant`-scope behavioral contract undefined (0 code matches); version pinning on provenance records.

### 4.5 — Structural merges (schema consolidation)

Absorbed from IDEA_SCRATCH. Schedule alongside whichever critical-path item naturally touches them.

- [ ] **Assertion Library → Workflow Governance** `[Imp 4 / Cx 2 / Debt 3]` — Absorb as "Named Assertions" section. Library without #38 reference protocol is incomplete; absorb rather than fix. Source is a thin 55-line spec + 139-line schema; merge is mechanical.
- [ ] **Verification Report → Advanced Governance** `[Imp 3 / Cx 2 / Debt 2]` — Absorb as "Output Artifacts" section. Thin sidecar.
- [ ] **Due Process Config partial merge → Workflow Governance** `[Imp 5 / Cx 3 / Debt 4]` (pending #45 step 0) — If thin NoticeTemplate drops (per #2) and AppealRouting + ContinuationPolicy remain, the merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage gap structurally.
- **M-1 Drift Monitor + Agent Config — BLOCKED.** Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone. Ship #37 standalone binding instead; reconsider merge if fixture is revised.
- **M-2 Notification Template + Due Process Config — REJECTED.** 4 non-due-process categories. Ship #39 standalone linkage instead.

### 4.6 — Engineering hygiene (deprioritized)

Organizational debt, not architectural. First adopter won't notice. Schedule when the relevant code is actively being touched for another reason.

- [ ] **#22 Crate split along tier boundaries** `[Imp 5 / Cx 3 / Debt 3]` *(ProvenanceKind tier-typing extracted to §4.2 as #22a)* — Split `wos-core` → `wos-kernel | wos-governance | wos-ai | wos-advanced`. Split `wos-runtime/src/runtime.rs` (now 4451 lines, up from 3821) along action-kind dispatch. Add CI dependency fence. Remaining scope is purely organizational; first adopter won't notice. **Note:** `wos-formspec-binding → wos-runtime` inversion is already landed (`wos-formspec-binding/Cargo.toml:10-13`); `runtime.rs` lives in `wos-runtime`, not `wos-core`.
- [ ] **#45 Sidecar normative-contract audit** `[Imp 6 / Cx 5 / Debt 5]` — Retrofit all sidecars against CONVENTIONS.md: Step 0 (does this sidecar deserve independent existence?) + three-question rubric (Structure / Semantics / Composition).

---

## 5 — Audit and evidence products

Build on the stable provenance export surface from §2. Schedule after §4.1 lands.

- [ ] **#48 Merkle provenance chains** `[Imp 6 / Cx 6 / Debt 6]` — Cryptographic hash-chaining for tamper-evident logs. Attaches via Assurance `provenanceLayer` seam. Hash-chaining only initially; full SCITT / RFC 9162 transparency-service integration as later ADR. **Debt raised:** PROV-O / XES / OCEL exports shipped 2026-04-15 without hash-chain hooks — every adopter of those formats now consumes unlinkable output; retrofitting means versioning three export surfaces simultaneously.
- [ ] **#52 Simulation trace format** `[Imp 4 / Cx 3 / Debt 2]` — Normative replay semantics for simulation runs. Event log format is XES (already shipped via `wos-export::xes`). Remaining work: normative replay contract + conformance fixtures.

---

## 6 — Regulatory alignment

External-deadline-driven. Benefits from ontology (§2) landing first.

- [ ] **#50 EU AI Act alignment** `[Imp 7 / Cx 5 / Debt 4]` — Art. 13–14 alignment spec: draft → 1.0.0. Watchlist — external compliance deadlines can force escalation.
- [ ] **#50 OMB M-24-10 compliance** `[Imp 6 / Cx 4 / Debt 3]` — Compliance support spec: draft → 1.0.0. Narrower than EU AI Act; overlaps existing assurance / impact-level plumbing. More process-documentation-shaped than structural, so Debt is lower.

---

## 7 — Interoperability and speculative research

Pick up when §§2–6 stabilize.

- [ ] **SCXML interoperability** `[Imp 3 / Cx 6 / Debt 2]` — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **#51 Statutory deadline chains** `[Imp 4 / Cx 7 / Debt 5]` — Interdependent government deadlines and automated legal consequences. Architecturally expensive — wrong abstraction here is expensive. **Debt raised:** once #31 jurisdiction-aware calendars and #20 typed events land, deadline chains must compose with both; deferring past those without at least a sketch risks an incompatible construct.

---

## Deferred (with triggers)

Items captured but not active; re-score when the named trigger fires.

| IDEA # | Item | Imp | Cx | Debt | Trigger |
|---|---|---:|---:|---:|---|
| #1 | Agent Behavioral Attestations | 2 | 7 | 1 | SLSA-style AI-agent attestation ecosystem matures OR specific deployment demands capability attestation. |
| #4 | Tripartite Object Model | 2 | 9 | 3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1 | 8 | 0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2 | 9 | 5 | Multi-object mutation patterns emerge, or flat→OCEL export shows systematic semantic loss. |
| #9 | JSON-LD Export Surface | 5 | 5 | 3 | `ontology-spec.md` drafts begin OR shipped PROV-O export pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6 | 7 | 5 | #20 lands. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3 | 5 | 2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions | Sustained deployments above 100 cases/minute |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS |
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding). Reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN-expression-language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; SHACL would duplicate. Shipped PROV-O is JSON-LD; if output-shape validation is needed, scope a dedicated item — don't resurrect SHACL wholesale. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Topology rejected; event taxonomy adopted normatively via #20. |

---

## Parked

- [ ] Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- [ ] JSON Patch for fine-grained provenance.
- [ ] FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open questions

1. **Engine-adapter sequencing** — TODO §3 ↔ IDEA Deferred. Defer until first commercial request, or schedule now to validate runtime against production-shape workloads?
2. **Ontology-spec authoring ownership** — who drafts, when?
3. **Timer semantics** (#20). Wall-clock or business-days for `noticeGracePeriod` legal compliance? Business calendar reference opt-in or required?
4. **Registry composition** (#21). Two L1 governance docs attaching rules to the same tag — declaration order, explicit priority, or conflict rejection?
5. **Multi-instance design** (#32). Events, arrays, or both? Governance hooks per-instance vs. per-iteration.
6. **Version migration declaration surface** (#3). Kernel carries governance version or each case? `tenant`-scope behavioral contract?
7. **Canonical forms.** Enforce "simple sequential workflow MUST be expressed as compound state with ordered children, not flat atomic sequence"?
8. **Defeasibility layer** (#25). `workflow-governance` or distinct companion? Priority encoding? Compose with `sourceAuthority` AND Integration Profile §11.2.
9. **Case-field policy vs. L2 `Right`** (#26b). Compose or supersede? Hold policy interaction. Assurance Invariant 6 independence.
10. **Cancellation region semantics** (#27). Set or predicate? Event / guard / explicit action? Compensation run / skip / author's choice?
11. **`ExternalArtifactRef` shape** (#28). 9th case-field type or `$def`? Retrieval contract — sync / deferred / action-body?
12. **Task suspension reducibility** (#30). Always reducible to `holdType: task-suspended`, or independent task state needed?
13. **Equity expression language** (#36). FEL extension, restricted DSL, or FEL + windowing?
14. **Assurance-level composition** (#43). Minimum floor per impact level, disclosure-only, or implementation-defined?
15. **JSON-LD authoring surface** (Deferred #9). Should `@context` land in authoring or stay export-only?
16. **#29b firing mechanism.** Event-based (enqueue synthetic event) or guard-based (`$milestone.*` FEL boolean)?

---

## Completed

Closed-out work items are archived in [`COMPLETED.md`](COMPLETED.md). New completions should be appended there, not tracked here.

---

## Notes

**ADR references (resolved 2026-04-18).** `ADR-0057 (wos-core-implementation-boundary)` and `ADR-0058 (wos-core-gap-analysis)` live in `thoughts/archive/adr/` (implemented). A prior audit looked only in active `thoughts/adr/` and incorrectly flagged them as missing. Citations in `enterprise-implementation-roadmap.md:257`, `thoughts/plans/2026-04-13-wos-runtime-crate.md:423`, `thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md:302`, and `specs/companions/runtime.md:51,:906` all resolve against the archive copies. No action pending — retained here so future audits don't re-raise the same flag.
