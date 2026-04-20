# WOS Completed

Archive of closed-out work items extracted from `TODO.md`. Active backlog and in-flight work continue to live in `TODO.md`; this file is append-only and is not read during planning.

---

## Specs and schemas

- [x] Kernel spec (S4.2, S4.10, S9.2) — concurrency, cascade depth, async actions.
- [x] Governance spec (S6.2) — source authority ranking.
- [x] Runtime companion (S5.3, S10, S12, S14) — parallel provenance, convergence cap, EventQueue interface.
- [x] Formspec integration gaps — version pinning, changelog migration, semantic contracts.
- [x] LINT-MATRIX rule count reconciled (197 total; I-001 added in NB.2).
- [x] Kernel schema — `evaluationMode`, `maxRelationshipEventDepth`.
- [x] Governance schema — `scope`, `sourceAuthority`, `ruleId`.
- [x] Case Instance schema — `pendingEvents`, `governanceState`, `volumeCounters`.

## Normative features (from IDEA_SCRATCH Shipped)

- [x] **Null behavior on deontic constraints** (formerly IDEA #11) — `nullBehavior` on Permission/Prohibition/Obligation with impact-level defaults. `ai-integration.md §4.2-4.5 + §5`; `NullBehavior` `$def`.
- [x] **Arazzo integration sequences** (formerly IDEA #14) — Multi-step API orchestration via Arazzo references. `integration.md §3.5`; fixtures `INT-ARAZZO-001..003`. (See NB.4.)
- [x] **Non-HTTP tool invocation** (formerly IDEA #15) — `tool` binding kind (`command-line`, `batch-file`, `database-procedure`, `graph-query`). `integration.md §3.6`; fixtures `INT-TOOL-001..002`. (See NB.4.)
- [x] **Assist Governance Proxy** (formerly IDEA #16) — Deontic constraint enforcement on Formspec Assist tool calls. `ai-integration.md §14`; schema `AssistGovernanceProxy`. Stabilizes with Assist layer upstream.

## wos-core and runtime capabilities

- [x] Typed deserialization — Kernel, Governance, AI fixtures round-trip.
- [x] Evaluator — deterministic algorithm from S2.
- [x] Host traits — nine interfaces in `traits/mod.rs`.
- [x] `instance.rs`, `explain.rs`.
- [x] Conformance harness wired to runtime (`WosRuntime` / evaluator path as landed).
- [x] T3 fixtures batches 1–17 (102) and batch 16 processor meta-rules.
- [x] Inline conformance documents — `run_fixture` and fixture parse checks support `documents.* = "inline"`.
- [x] Timer region scoping and tolerance validation.
- [x] `deontic.rs`, `autonomy.rs`, `confidence.rs`, `event_handler.rs`, `eval_mode.rs`, `explain.rs` behavior.

## wos-lint

- [x] T1/T2 on typed models (`KernelDocument`, `KernelCollections`).
- [x] Typed state-tree walks (replaced manual tag/event collection).
- [x] G-027 sub-delegation depth via typed models.
- [x] T1-TESTS (G-058, G-059, G-062, G-065), T1-K009, CM-001, T2-GAPS (G-060, G-063).
- [x] LINT-COVERAGE — 197 of 197 rules covered (see LINT-MATRIX.md; I-001 added in NB.2).

## Conformance harness hygiene

- [x] **CONF-META-MOVE** — Move `observe_proxy_behavior` / `observe_assist_governance_proxy` into `wos-core/src/proxy.rs`.
- [x] **CONF-AI050-DIFF** — `differential_check_passed` computed from actual severity + violation-id comparison instead of hard-coded `true`.
- [x] **CONF-AI004-EVIDENCE** — `observe_delegated_formspec_evaluation` sets `full_response_envelope_validated` from `validation_result.valid`.
- [x] **CONF-PROFILE-DEDUP** — `tests/profile_conformance.rs` now delegates to `run_profile_against_fixtures` in `meta.rs`.
- [x] **CONF-RUNTIME-POLICY** — Move deontic, autonomy, confidence, event-handler, and DCR fixture policy into `wos_runtime::ReferenceCompanionPolicy`; conformance only selects/configures it.
- [x] **CONF-RUNTIME-PROVENANCE** — Emit compensation, lifecycle/case separation, and history-cleared provenance from `wos-runtime` / `wos-core`; conformance asserts observed provenance instead of synthesizing it.
- [x] **CONF-EVENT-IDENTITY** — Runtime drain results report the processed event token; fixture draining no longer stops on event name alone.
- [x] **CONF-IDEMPOTENCY-SCOPE** — Scope reference companion idempotency tracking per instance.
- [x] **CONF-STORE-API** — Remove `InMemoryStore` from the conformance public API; engine uses `wos_runtime::InMemoryStore`.
- [x] **CONF-STUB-TESTS** — Document inline stub tests as harness verification, not spec behavior.
- [x] **CONF-BINDING-DOC** — Document `ConformanceBinding`: intentionally permissive, `compute_case_mutation` returns `None`.

## Documentation

- [x] `wos-spec/README.md`, root `context.md` WOS section, `wos-core/README.md`, `WOS-IMPLEMENTATION-STATUS.md`.

## Conformance profiles

- [x] Governance Basic/Complete aggregate tests.
- [x] Agent Registration / Confidence Framework aggregate tests.

## SMT / static analysis

- [x] AG010 finite-domain AST analysis, `finiteDomainDeclarations` in schema/linter, FEL filter rejection.

## Formspec coprocessor

- [x] FEL `every`/`some` in Formspec core.
- [x] Runtime Companion S15 interface and reference in-memory runtime path.
- [x] `wos-formspec-binding` — adapter surface plus prefill, validation, and mapping tests.
- [x] S15.3 pin re-validation on replay paths — `wos-formspec-binding::FormspecBinding::revalidate_submission` recomputes pin equality fresh on every replay/audit/review call.

## Coprocessor version discipline (S15)

- [x] S15.1 — register `FormspecBinding` alongside `ConformanceBinding`; real binding path exercised in conformance (61132c1).
- [x] S15.2 — author S15 validation fixtures through real `wos-formspec-binding` path; all 6 fixtures green (b0f3306).
- [x] S15.3 — delete `ConformanceBinding`; pin re-validation enforced on replay paths (0283740 + 0a3c369). `StubValidator` retained for service-invocation contract validation (`contract_outcomes` fixture field), which is a separate code path from the task-binding adapter.

## Kernel/runtime semantics (KS)

- [x] KS.1 — DeepHistory + ShallowHistory state semantics with conformance fixtures (D1 depth-1, D2 depth-2 + parallel-exit, D3 depth-3); `wos-core` capture/restore (c78848c).
- [x] KS.2 — Milestone firing with pinned ordering (data write durable → `MilestoneFired` → reactive transitions evaluated); 5 conformance fixtures K-M-001 through K-M-005 (521bd54).

## Business calendar (BC)

- [x] BC.1 — Business Calendar SLA runtime integration: lazy deadline evaluation at check time, `calendarVersion` snapshot, `DidNotConverge` error on convergence failure; 4 fixtures G-S10-001 through G-S10-004 green (c93052f).

## Provenance export (PE)

- [x] PE.1 — `wos-export` crate: PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); `timestamp` added to `ProvenanceRecord`; 3 SP-EXPORT-* conformance fixtures green (9daf447, 7cedfae, d8fbcf0, 7cd3cd3, 3ed010e, bd4e52f, b55b67e). Known limitations: higher-tier PROV-O bundles (§5.4) not emitted; OCEL events link to instance object only (per-case-file-item E2O links deferred); SHACL validation out of scope.
- [x] PE.2 — `ProvenanceRecord` schema extension + full SP §5.3/§5.5/§6.3 emission (2026-04-16, branch `feat/provenance-export` at `0fb895d` — unmerged). Eight optional SP-mandated fields added to `ProvenanceRecord`: `audit_layer`, `actor_type`, `lifecycle_state`, `definition_version`, `inputs`, `outputs`, `input_digest`, `output_digest`. Runtime populates all eight at stamp time via new `populate_provenance_record_fields` helper (wired at all 9 append sites; 1:1 with `provenance_log.push`/`.extend` invariant verified). Exporters emit the full §5.3/§5.5/§6.3 mappings: PROV-O `prov:used`/`prov:wasGeneratedBy` Entity nodes, `wos:atLifecycleState`, `wos:definitionVersion`, §5.5 actor-type subclass pairs (`[prov:Person, wos:HumanAgent]` / `[prov:SoftwareAgent, wos:SystemAgent]` / `[prov:SoftwareAgent, wos:AIAgent]`); XES `org:group`, repeated-key `wos:input`/`wos:output`, trace-level `wos:definitionVersion`, `wos:lifecycleState`, per-event digests; OCEL uniform `eventTypes` schema + indexed `inputs.{i}`/`outputs.{i}` scalar attrs (OCEL 2.0 compliance — no array-valued attributes). §6.5 Facts-tier filter applied uniformly via shared `is_facts_tier` helper; exhaustive `audit_layer_for_kind` match (93/93 variants) compile-gates future tier additions. New SP-EXPORT-004 fixture locks the filter. SHA-256 digests via new `sha2` crate dep. 407 tests passing, zero TODO(spec-upstream) markers remaining. Four rounds of semi-formal code review; all findings addressed (da20e80, d33b3ef, 32e453f, d86709b + 10 findings-fix commits: 8f3583a, 8cf6802, 0357b26, 1c86299, 418c0f9, 5ee7291, 2809393, 0f2a4a0, b735923, 0fb895d). Known limitations remaining: higher-tier PROV-O bundle wrapping (§5.4 — requires export API redesign to accept tier-discriminated output); OCEL case-file-item objects + per-item E2O/O2O links (§6.4 — requires case state snapshot protocol); SHACL validation (needs RDF library dependency); `ActorKind::Agent` mapping (`actor_type = "agent"`) pending AI Integration agent-registry threading through runtime context. Follow-up plan at `thoughts/plans/2026-04-16-wos-provenance-record-schema-extension.md`.

## Integration Profile binding kinds (NB)

- [x] NB.1 — typed `IntegrationBindingKind` enum + `IntegrationBindingHandler` trait; replaced stringly-typed dispatch (f017910).
- [x] NB.2 — outputBinding RFC 9535 profile pinned (wildcard + slice; filter/recursive-descent rejected); lint rule I-001; spec §3.3.1 (e6e916d).
- [x] NB.3 — CloudEvents bindings (`event-emit`, `event-consume`, `callback`) with subject correlation `{instanceId}:{bindingId}:{invocationId}`; full envelope captured in provenance; 6 fixtures INT-EMIT/CONSUME/CALLBACK-001–003 (75c8b21).
- [x] NB.4 — Arazzo, tool, and policy-engine bindings; `PolicyDecision` normalized to `{decision, reasons, obligations}`; 7 fixtures INT-ARAZZO/TOOL/POLICY-001–004 (d79c02b).

## Security / architecture docs

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).

## Session 4 (2026-04-18) — wos-synth scaffold + §4.1 chain unblocking

- [x] **§5.4 wos-synth Tasks 1-6 scaffold** (`6409006` + review fixes `b824927`) — four-crate split: `wos-synth-core` (loop + `Prompter` trait + `ToolContext` trait + prompt templates + `DirectToolContext` stopgap), `wos-synth-mock` (deterministic test prompter), `wos-synth-anthropic` (streaming-callback Anthropic provider), `wos-synth-cli` (binary `wos-synth` with `generate` / `dry-run` / `explain`). DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal | grep -E 'reqwest|tokio|anthropic'`. CLI `dry-run` produces a kernel doc that lints clean without touching the network. Plan Task 7 (synth-trace JSON Schema + drift test) deferred to follow-up. Review fixes: AnthropicPrompter `Arc::try_unwrap` → `mem::take` (no more discarded paid completions); `strip_fences` no-newline regression; `LintFinding.suggested_fix` + `related_docs` plumbing into repair prompt; ScriptedPrompter/Tools converted to VecDeque + pop_front; trace explain prints "unknown" instead of misleading 0/0/0 token totals; OverrideRecord orphan-`$def` annotated; `anyhow_lite` rationale documented.
- [x] **§4.1 NoticeTemplate reconciliation** (`dfd9189`) — dropped thin `NoticeTemplate` `$def` from `wos-due-process.schema.json`; rich `TemplateSection`-based shape in `wos-notification-template.schema.json` is canonical. Zero in-tree consumers. `noticeTemplateRef` (Governance §3.1) and `notificationTemplateRef` (Governance §12.2) both already routed through the Notification Template sidecar via lint rule G-063.
- [x] **§4.1 #23 OverrideRecord schema** (`62b1561` + pytest contract `b824927`) — typed `OverrideRecord` + `EvidenceReference` `$def`s in `wos-workflow-governance.schema.json` with 1:1 mapping to OverrideAuthority policy switches (`requireStructuredRationale` ↔ `rationale`, `requireAuthorityVerification` ↔ `authorityVerification`, `requireSupportingEvidence` ↔ `supportingEvidence`). Authority-verification typed via 4-variant `method` enum (`roleAssignment | delegationGrant | supervisorAttestation | externalAuthority`) + `actorId` + `verifiedAt`. Spec §7.3 prose links to typed shape. EvidenceReference enforces "MUST be locatable" structurally via `required: ["kind"]` + `anyOf: [{required: ["caseFieldPath"]}, {required: ["uri"]}]`. Pytest contract `tests/schemas/test_override_record_shape.py` (12 cases: 6 EvidenceReference + 6 OverrideRecord with parameterized missing-field rejection + empty-supporting-evidence rejection) added in the review-fix commit. OverrideRecord is intentionally orphan (shape catalog for runtime provenance) — annotated via `$comment`.
- [x] **§4.1 #31 Jurisdiction-aware business calendar selection** (`44ac44c`) — replaced "implementation-defined" §7.1 selection with deterministic 6-step algorithm via optional `appliesWhen` FEL on each Business Calendar (matches `DueProcess.scope` pattern). Multi-jurisdiction rights-impacting workflows (e.g., national benefits with one calendar per US state) now have a declarative selection mechanism. Timezone disagreement among applicable calendars is a configuration error — surfaces modelling mistakes at evaluation time instead of silently picking one timezone. Spec §7 gained 7.1 (selection algorithm), 7.2 (composition + timezone-error rule), 7.3 (worked multi-state example).
- [x] **§4.2 #29a Milestone trigger-mode spec-lag closure** (`64b03a5`) — `Milestone.triggerMode: writeSettled` (default-only enum, extensible) names the runtime KS.2 behavior in authoring-visible form. Spec §4.13 gained "Trigger semantics" paragraph naming the three runtime invariants: fire-after-settled-write, at-most-once-per-instance, lexicographic id ordering for deterministic provenance. Wos-core `Milestone` struct picked up the optional field with `serde(skip_serializing_if = "Option::is_none")` so existing fixtures roundtrip byte-identically. Unblocks IDEA #29b reactive milestone firing — which can now extend `triggerMode` cleanly.

## Session 5 (2026-04-19) — §4.2 #37 / #46 closeout

- [x] **§4.2 #37 Drift Monitor demotion policy binding** — `AlertThreshold.policyRef` binds Drift Monitor alerts to named Agent Config `DemotionRule.id` semantics. Added executable T3 fixtures and expected traces for `AI-AUTO-001-escalation-expiry-revocation` and `AI-AUTO-002-drift-alert-demotion`; registered both as Tested conformance rules; parity + runtime-engagement tests prove escalation-expiry emits `autonomyDemotion`, while drift-alert demotion emits `autonomyDemotion` + `driftReclassification` and reroutes through `escalated` to human review. `LINT-MATRIX.md` regenerated to 99 rules / 8 T3.
- [x] **§4.2 #46 Schema-prose enum alignment batch** — closed enum/prose drift in `wos-kernel.schema.json` and `wos-workflow-governance.schema.json`: `CaseRelationship.type` and `HoldPolicy.holdType` now accept standard values or `x-` vendor extensions; `AppealMechanism.reviewerConstraint` is required and uses the due-process independence vocabulary; `AppealMechanism.continuationScope` uses the due-process continuation vocabulary; duration fields are constrained to the runtime-supported ISO 8601 grammar; `DelegationScope.conditions` cites the shared FEL evaluation contract. Drift Monitor `AlertThreshold.policyRef` prose/schema binding is covered by the #37 conformance slice.

## Session 5 (2026-04-19) — §4.1 #24a Facts-tier input snapshot

- [x] **§4.1 #24a Mandatory Facts-Tier input snapshot** — Kernel §8.2.1 now requires `transitionTags` plus `caseFileSnapshot` on Facts-tier state-transition provenance for determination-tagged transitions. `FactsTierRecord` / `CaseFileSnapshot` schema `$defs` and pytest contracts lock the shape.
- [x] **Runtime support** — `wos-core` snapshots use RFC 8785 JCS canonicalization plus SHA-256. The lifecycle evaluator captures snapshots at the exact transition firing point and persists transition tags on the provenance record, so recursive `$join` determinations receive the case-file state current to that transition rather than a stale per-drain snapshot.
- [x] **Executable conformance coverage** — registered T3 rule `K-DET-001` and added `k-det-001-determination-snapshot.json`, asserting transition tags, snapshot value, canonical JSON, and digest. `LINT-MATRIX.md` regenerated to 100 rules / 9 T3.

## Session 6 (2026-04-20) — active closeout

- [x] **§5.4 Task 7 synth-trace schema + drift test** — `schemas/synth/wos-synth-trace.schema.json` now publishes the `SynthTrace` and `SynthOutcome` artifact contract for `wos-synth explain`. `crates/wos-synth-core/tests/trace_schema_drift.rs` validates representative `SynthTrace`, converged `SynthOutcome`, and unconverged `SynthOutcome` serde output against the published schema, including optional `conformance`, `path`, `suggested_fix`, and `related_docs` fields. Local verification: `cargo test -p wos-synth-core --test trace_schema_drift -- --nocapture` passed 3/3; provider-DIP invariant remains clean (`cargo tree -p wos-synth-core --edges normal | rg 'reqwest|tokio|anthropic'` returned no matches).
- [x] **§5.4 synth review follow-up** — semi-formal review found two adjacent behavioral gaps: `wos-synth explain` omitted per-iteration conformance verdicts, and `strip_fences` did not honor its own non-JSON language-tag contract. Fixed both with tests: `cargo test -p wos-synth-core strip_fences -- --nocapture` (7 passed) and `cargo test -p wos-synth-cli render_trace_includes_iteration_conformance -- --nocapture` (1 passed).
- [x] **§4.1 #2 Deterministic adverse-decision notice (dual-form)** — `ReferenceCompanionPolicy` now detects active `adverse-decision` transitions with `noticeRequired`, captures the pre-transition Facts-tier case-file snapshot, resolves the Notification Template sidecar by `noticeTemplateRef`, renders deterministic human-readable prose, and emits a `noticeSent` record with `data.source = "deterministic"`, `machineReadable.kind = "adverseDecisionNotice"`, `snapshotSha256`, transition metadata, appeal configuration, and template reference. Governance §3.2 and schema prose now state the deterministic assembly contract. G-002 uses inline governance + notification-template documents and asserts the deterministic artifact plus snapshot digest. Verification: `cargo test -p wos-conformance g002_notice_before_adverse -- --nocapture`; `python3 -m pytest tests/schemas/test_fixture_validity.py tests/schemas/test_meta_validity.py -q`.
- [x] **§4.2 #21 Extension registry (seams-only MVP)** (`3550fad`) — `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md` catalog the six kernel seams (§10.1 actor-extension, §10.2 contract-hook, §10.3 provenance-layer, §10.4 lifecycle-hook, §10.5 custody-hook, §10.6 vendor-extension) plus the Trellis custody shape. `RegistryEntry` `$def` carries lifecycle (draft/stable/deprecated/retired), composition (merge/replace/augment), `since` / `replacedBy` / `vendorPrefix`. Descriptive, not enforcement; closes the `custodyHook` prose-only escape.
- [x] **§4.2 #39 ContinuationPolicy normative linkage** (`eaa678d`) — `AppealMechanism.continuationPolicyRef` (optional, `x-lm.critical`) resolves to `ContinuationPolicy.id` (now REQUIRED). `continuationOfServices: true` with neither ref nor scope resolving is a configuration error. Governance §3.6 prose added; misconfiguration-vs-error distinction spelled out.
- [x] **§4.2 #37 Drift Monitor demotion policy binding** (`b077613`) — `AlertThreshold.policyRef` (optional, `x-lm.critical`) resolves to `DemotionRule.id` (now REQUIRED) in the Agent Config sidecar. Named rule's structured semantics take precedence over the `action` enum; unresolvable ref falls back with a provenance warning. Drift-monitor §1.4.1 prose added. Combined with session-5 AI-AUTO-001/002 fixtures, this closes the full §4.2 #37 slice.
- [x] **§4.3 #13 Verifiability test principle** (`31a0e21`) — Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2. Doc-only.
- [x] **§4.3 #57 Assurance schema `x-lm.critical` coverage** (`a1100fe`) — Annotations on `assuranceLevel`, `subjectContinuity.{reference,scope}`, `disclosurePosture`, `attestation.{subject,predicate,basis}`. The only schema in the suite with zero annotations now has them; `schema_doc_zero_regression` stays green.

## Session 7 (2026-04-20) — DRAFTS triage + §4.3 close + v0 spike + review pass (8 commits)

- [x] **§4.1 DRAFTS triage** (`0d17f9f`) — 13 historical kernel drafts (v0.x through v7 plus tier-spec ancestors and a schema snapshot) moved from `DRAFTS/` to `thoughts/archive/drafts/` with a README classifying each file (superseded kernel iterations / v7 reframe ancestors / tier-spec ancestors / schema snapshot). MD-INVENTORY §6 rewritten to point at the archive; IDEA_SCRATCH reference updated. Unblocks §4.1 #20 typed event meta-vocabulary.
- [x] **§4.3 #56 K-049 continuous-mode cycle detection** (`4fd32e3` + review Finding 1 fix `2c6a2e2`) — new module `crates/wos-lint/src/rules/continuous_mode.rs`: parses each transition guard via `fel-core`, collects `setData` write-paths from transition actions plus source-state `onExit` plus target-state `onEntry` (Kernel §4.7 execution sequence), builds a directed write→read graph keyed by a per-path writer index (O(writes × reads)), runs iterative-DFS cycle detection, emits a T2 warning when `evaluationMode: continuous`. Exports `simple_access_path_string` + `walk_expr` as `pub(super)` from `fel_analysis.rs`. Registered `Tested` with spec_ref `specs/companions/runtime.md#s10-3`. 7 unit tests (self-loop, 2-node cycle, compound-nested cycle, entry/exit cycle, event-driven skip, acyclic control, unparseable guard).
- [x] **§4.3 #12 Capability preconditions + AI-057** (`19ad643`) — added `Capability.preconditions: array of FEL strings` (with `x-lm.critical`) to `schemas/ai/wos-ai-integration.schema.json`; normative semantics in new spec §3.3.1 (all entries MUST evaluate to boolean `true`; unsatisfied → skip to fallback chain §8; provenance `outcome: preconditionNotSatisfied`; preconditions do not relax deontic constraints). Wos-core `Capability` struct picked up `preconditions: Vec<String>` with `serde(default)`. New AI-057 T2 error lint enforces FEL parse validity per entry; 3 unit tests. LINT-MATRIX regenerated to 102 rules / 11 Tested / 58 T2.
- [x] **v0 spike Tasks 4–5** (`f6320c2` + `a80e37d`) — Task 4 conformance smoke-test gate: after lint passes, wraps the synthesized kernel in a minimal inline `ConformanceFixture` (empty `event_sequence`, empty `expected_transitions`) and calls `wos_conformance::run_fixture`; one repair round granted; budget-aware; `SpikeError::ConformanceFailure` isolates conformance-phase failures. Task 5 retrospective at `thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md` with plan propagations appended inline to `wos-synth-crate`, `wos-synthesis-benchmark`, `wos-mcp-crate` plans. Key findings: `wos-conformance` has no `run(&doc)` entry point (fixture wrapper required); `ToolContext` shipped without spike counter-example → provisional; structured repair-prompt with `rule_id` + `suggested_fix` + `spec_ref` recommended before `wos-bench` measures convergence; live Anthropic iteration counts (Q-V0-1..4) flagged as follow-up. Spike disposition: keep-with-deletion-horizon (2–3 months). 17 unit tests green.
- [x] **§4.3a K-049 / AI-057 review follow-ups filed + refined** (`64962ea` + `4ceddb7`) — background `semi-formal-code-review` agent ran over `0d17f9f` + `4fd32e3` + `19ad643`. Verdict APPROVE with 9 findings; Finding 1 (K-049 missing entry/exit actions) fixed in `2c6a2e2`; Findings 6/8/9 OBSERVATION-only. Remaining 4 filed as §4.3a items in TODO, then refined via parallel spec-expert + wos-expert consultations into six concrete work items: **#F2** structured `Vec<Segment>` path comparison under Core §3.6.4 reachability; **#F3a** K-049 message reword + `$continuous` fixture; **#F3b** ADR + rewrite `eval.rs:412-421` post-mutation re-scan to match Runtime §10.3; **#F4** AI-058 boolean-AST-root allowlist lint + upstream Formspec §3.8.1 normativity gap filing; **#F5a** kernel `$defs/ProvenanceOutcome` enum (closes both `preconditionNotSatisfied` and `convergenceCapReached` MUSTs in one schema change); **#F5b** AI schema `if/then` enforcement. Cross-cutting drift surfaced: `ProvenanceKind::ConvergenceCapReached` missing from `crates/wos-core/src/provenance.rs:44` despite being named as a `recordKind` in `runtime.md:517`.
- [x] **Validation at close** — `cargo test --workspace` (63 test binaries, 0 failures). SCHEMA-DOC-001 zero-regression gate passes. Python `python3 -m pytest tests/ -q` 121 passed / 11 skipped / 1 xfailed.
