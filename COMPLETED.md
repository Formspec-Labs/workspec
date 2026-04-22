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
- [x] WOS-T2 — ADR-0060 cross-reference taxonomy revisit: Workflow Governance now uses `templateKey`, `noticeTemplateKey`, `notificationTemplateKey`, and `escalationStepId`; stale `noticeTemplateRef` governance fixtures/runtime surfaces were removed; G-063 enforces Notification Template keys; G-066 enforces `BreachPolicy.escalationStepId` resolution within the same `TaskPattern`; Studio WOS types regenerated from schemas.
- [x] WOS-T3 — `DurableRuntime` extraction + Temporal/Restate spike: public backend-neutral trait, in-memory `WosRuntime` adapter, runtime module split (`tasks`, `actions`, `timers`, `provenance`, `support`, `drain`, `instance`, `durable_impl`), Restate selected as first production backend, Temporal deferred pending Rust workflow API stability, and tenant-scope contract recorded in `thoughts/reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md`.
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

## Session 8 (2026-04-20) — 8-agent parallel dispatch (~23 commits)

Largest parallel dispatch to date. Three batches: (1) uncommitted session-6 work committed + review-finding fixes; (2) eight concurrent agents on disjoint file sets; (3) four concurrent semi-formal code reviews.

### §4.1 #2 Deterministic adverse-decision notice — commit-split of uncommitted session-6 work (4 commits)

- [x] **`02ca0c1` style(runtime): rustfmt import-sort + assert! macro wrap** — split rustfmt churn out of the semantic commit per review Finding 6.
- [x] **`a041433` feat(runtime): thread current_time_ms + now_iso through drain context** — adds `now_ms: u64` + `now_iso: String` to `RuntimeEventContext`, populated once per drain from `self.clock.now_ms()`. No silent-zero path; missing populates surface at compile time. Prerequisite for the adverse-decision emitter's deterministic timestamps.
- [x] **`25026dd` feat(runtime): deterministic adverse-decision notice emission (§4.1 #2)** — `ReferenceCompanionPolicy::deterministic_adverse_decision_notice_input` + `AdverseDecisionNoticeInput`. Digest `7c6c9f04…f8a749` verified via both Rust + Python JCS implementations. Schema `if/then` requires `noticeTemplateRef` when `noticeRequired: true` (closes F8). Resolver returns typed `NoticeTemplateResolution` enum; audit signal surfaces as `resolvedTemplateKey` / `templateResolution` on the emitted record (closes F4). Spec §3.2 enumerates "transition-firing-timestamp" as a determining input (closes F3). Fixture `initial_case_state` cleaned up to realistic pre-transition state (closes F7). Two new unit tests pin `humanReadable` byte-identity under a fixed clock (closes F2).
- [x] **`abe3c76` fix(synth): strip non-JSON fence language tags; render per-iteration conformance in explain** — §5.4 synth review follow-up: `strip_fence_language` heuristic + `render_trace` pure function.

### §4.3a K-049 / AI-057 review follow-ups — 5 of 6 closed (8 commits)

- [x] **#F3a K-049 message reword + `$continuous` fixture** (`e15bd80`) — diagnostic now spec-faithful; `$continuous`-event fixture added.
- [x] **#F4 AI-058 boolean-AST-root lint** (`8855591`) — `is_boolean_shaped(&Expr)` pub(super) in `fel_analysis.rs`; 3 unit tests.
- [x] **#F5a Kernel `$defs/ProvenanceOutcome`** (`2d890d3`) — open-enum with `preconditionNotSatisfied` + `convergenceCapReached` reserved, `^x-` vendor pattern; optional `outcome` on `FactsTierRecord`; Rust `ProvenanceKind::ConvergenceCapReached` variant. Closes both §3.3.1 and §10.3 MUSTs in one schema change.
- [x] **#F2 K-049 structured-path reachability** (`ee05cec`) — `Vec<Segment>` + `reaches()` per Core §3.6.4; 2 regression fixtures; 10 new tests (`normalize_setdata_path` helpers + cycle regressions).
- [x] **#F5b AI schema `if/then` preconditionNotSatisfied** (`ae3589f`) — `CapabilityInvocationRecord` $def enforces `outcome = "preconditionNotSatisfied"` when `data.invocationBlocked: true`.
- [x] **LINT-MATRIX regen** (`d46d172`) — 102 → 103 rules; T2 Tested 2 → 3 (AI-058 added); K-049 later promoted LoadBearing in `f03ca40` after F3b.
- **#F3b ADR 0059 drafted** (`fcd2c19`) — Runtime §10.3 conformance plan; 5 tasks, ~3-5 engineer-days; preconditions satisfied by F5a. Implementation deferred.

### §4.4 Release trains Tasks 1-3 (4 commits)

- [x] **`78283ae` docs(release-trains): stream path mapping (§4.4 Task 1)** — `RELEASE-STREAMS.md`: kernel / governance / ai / advanced with paths, cadence, stability; sidecar attribution (lint/conformance/rule-coverage follow kernel); tag convention.
- [x] **`2c53f62` docs(changelogs): four per-stream changelog files (§4.4 Task 2)** — seeded with stability commitments per stream (kernel/governance semver-strict, ai pre-1.0, advanced research).
- [x] **`49de6c0` docs(release-trains): COMPATIBILITY-MATRIX + README pointer (§4.4 Task 3)** — `COMPATIBILITY-MATRIX.md` with `1.0.x / 1.0.x / 0.5.x / 0.1.x` row, `x-` known-broken convention, vendor-claim pattern.
- [x] **`9aee9be` docs(todo): mark §4.4 as partial after Tasks 1-3** — TODO state updated.

### §4.4 #40 Task SLA authoring surface (3 commits)

- [x] **`8b466fa` feat(governance): Task SLA authoring schema** — four OPTIONAL properties on `TaskPattern` (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`) + four supporting `$def`s.
- [x] **`bc5de5f` docs(governance): Task SLA authoring spec subsection** — §10.4 + §10.4.5 future-work lint deferrals.
- [x] **`130a51e` test(schemas): contract tests for Task SLA shape** — 27 parametrized tests + happy-path fixture.

### §4.4 #38 Assertion Library cross-document reference protocol (3 commits)

- [x] **`77695eb` feat(governance): Assertion Library cross-document reference shape** — `AssertionReference` / `AssertionInlineUse` / `AssertionUse` three-$def `oneOf` split.
- [x] **`f862d1f` docs(governance): Assertion Library cross-reference protocol** — new spec §2.3/§2.4 with resolution semantics + G-064 design sketch.
- [x] **`21e9195` test(schemas): AssertionReference shape contract** — 12 tests covering hybrid-mix rejection + URI validation + `assertionId` pattern.

### §4.6 #45 Sidecar normative-contract audit (1 commit)

- [x] **`9900e39` docs(reviews): sidecar normative-contract audit** — 9 sidecars audited against CONVENTIONS.md (Step 0 + Structure / Semantics / Composition rubric). Verdict: 3 KEEP / 3 MERGE / 3 RESHAPE / 0 RETIRE. Ratifies the §4.5 three-merge plan. Six open questions filed for user verdict.

### Plans + ADR (2 commits)

- [x] **`6cad36e` docs(plans): draft implementation plan for #20 typed event meta-vocabulary** — 9 sections, 10 ordered tasks, grep-verified fixture counts (185 files / 844 occurrences), four open questions (OQ1 `$join` + OQ4 vendor kinds are load-bearing blockers).
- [x] **`fcd2c19` docs(adr): continuous-mode post-mutation re-scan driver (F3b)** — ADR 0059. All preconditions satisfied; 5-task implementation plan; READY TO EXECUTE.

### Semi-formal review pass (4 parallel reviews)

- [x] **Review A — wos-lint cluster** (F3a + F4 + F2): APPROVE WITH FOLLOW-UPS. 1 WARNING (AI-058 allowlist drift — missing `every`/`some`/`boolean`, bogus `isBoolean`) + 1 NIT (guard-walker short-circuit regression test) + observations. Filed in TODO §4.3b as #F4a + #F2a.
- [x] **Review B — schema cluster** (F5a + F5b): APPROVE WITH FOLLOW-UPS. 3 WARNINGs: F5b's `CapabilityInvocationRecord` is orphan `$def` with no composer (#F5d); F5a Rust emission not wired (`ProvenanceRecord` lacks `outcome` field; runtime still emits `CaseStateMutation`) (#F5c, rolls into F3b Task 3); vendor-extension regex drift from lowercase-kebab convention (#F5e).
- [x] **Review D — #40 Task SLA**: APPROVE WITH FOLLOW-UPS. 2 WARNINGs + 2 NITs: `expectedDuration` `indefinite` semantics (#40a); `startEvent` pattern allows `$continuous` (#40b); `EscalationStep.id` drift (#40c); enum-rejection test gaps.
- [x] **Review H — #38 Assertion Library**: APPROVE WITH FOLLOW-UPS. 3 WARNINGs: stale `.llm.md` regen (#38a); TODO #38 text stale (fixed inline); "one-line $ref" adoption claim understated — adoption requires cross-schema `$ref` plumbing or duplicate $defs (#38b; G+H concur the §4.5 merge is the natural landing).

### Validation at close

Final state: `cargo test --workspace` 1002+ passed / 0 failed · SCHEMA-DOC-001 zero-regression gate green · `pytest tests/schemas/ -q` 171 passed / 11 skipped / 1 xfailed (+50 vs session 7). 103 LINT-MATRIX rules (AI-058 added). All eight parallel agents + all four parallel reviews landed on disjoint file sets without conflict — validates the parallel-agent dispatch discipline from `thoughts/practices/2026-04-17-parallel-agent-dispatch.md`.

## Session 10 (2026-04-21) — WOS-T1 custodyHook execution closeout

- [x] **TypeID minting landed in code** — added stack-local [typeid.rs](crates/wos-core/src/typeid.rs) with UUIDv7/Crockford `{tenant}_{type}_{uuidv7_base32}` minting + validation; `ProvenanceRecord` now mints `prov` IDs at authoring time; `CaseInstance` now mints `case` IDs and preserves legacy request aliases for runtime compatibility.
- [x] **Kernel provenance records gained durable custody citation** — `ProvenanceRecord` now carries `canonicalEventHash`; runtime added `apply_custody_receipt(...)` and stamps `CustodyAppendReceipt { canonical_event_hash }` onto persisted provenance by `recordId`.
- [x] **`wos-runtime::custody` rewritten to ADR-0061** — removed the superseded JCS/wide-shape append surface; live runtime now emits the narrow four-field append input (`caseId`, `recordId`, `eventType`, `record`) with dCBOR-authored bytes, base64 JSON host serialization, canonical CBOR map ordering, oversize rejection, and 2-tuple idempotency `(caseId, recordId)`.
- [x] **Spec / schema / registry surfaces aligned** — [specs/kernel/custody-hook-encoding.md](specs/kernel/custody-hook-encoding.md), [schemas/kernel/wos-custody-hook-encoding.schema.json](schemas/kernel/wos-custody-hook-encoding.schema.json), registry ownership metadata, case/provenance/governance TypeID patterns, and the Trellis Operational Companion §24.9 now all point at the accepted ADR-0061 contract.
- [x] **Planning surfaces updated to closure state** — [T1-TODO.md](T1-TODO.md) now carries a closeout summary + verification log; [TODO.md](TODO.md) marks WOS-T1 complete.

### Validation at close

- [x] `cargo test -p wos-core --lib`
- [x] `cargo test -p wos-runtime --lib`
- [x] `cargo test -p wos-export --lib`
- [x] `cargo test -p wos-conformance --lib`
- [x] `pytest tests/schemas/test_custody_hook_encoding.py tests/schemas/test_extension_registry.py tests/schemas/test_facts_tier_snapshot.py tests/schemas/test_facts_tier_outcome.py tests/schemas/test_capability_invocation_record.py tests/schemas/test_override_record_shape.py tests/schemas/test_case_instance_typeid.py tests/schemas/test_meta_validity.py`
- [x] `npm run docs:check`

## Session 9 (2026-04-20) — 4-agent parallel sweep of review follow-ups (19 commits)

All §4.3b review follow-ups closed in a single 4-agent parallel dispatch. Disjoint file scopes kept conflict surface minimal despite shared-crate touches on `wos-core/src/provenance.rs`.

### Review A follow-ups — wos-lint cluster (6 commits)

- [x] **#F4a AI-058 allowlist drift** (`2d3132f` + `b0ec6e0`) — `is_boolean_shaped`'s boolean-returning builtin allowlist now derives from `fel_core::builtin_function_catalog()` via `std::sync::LazyLock<HashSet<&'static str>>`, filtering on signatures ending `→ boolean`. Adds `every`, `some`, `boolean` (three real builtins previously missing → false positives on valid FEL); removes bogus `isBoolean` (was never a registered builtin). Four new tests pin each branch.
- [x] **#F2a Guard-walker short-circuit regression test** (`196346c`) — direct test `k049_guard_walker_short_circuit_prevents_spurious_cycle` with inline rationale naming the `PostfixAccess(FieldRef("caseFile", []), [Dot("input")])` parse shape that motivated the short-circuit. Previously only indirect-tested via `k049_ignores_acyclic_continuous_kernel`.
- [x] **Review A Finding 4 — `NullCoalesce` admission** (`10bd3af`) — `is_boolean_shaped` now recurses into `Expr::NullCoalesce { left, right }` (both sides must be boolean-shaped). Closes a false-positive class for `$flag ?? true` precondition patterns.
- [x] **Review A Finding 5 — adversarial `normalize_setdata_path` coverage** (`6b448df`) — new test `normalize_adversarial_inputs_degrade_to_single_dot` covers 7 edge cases (`""`, `"."`, `"foo[]"`, `"foo[-1]"`, `"foo[[0]]"`, `"foo[a]"`, `"foo[ 1 ]"`). `[*]` deliberately excluded since the normalizer handles it as `[Wildcard]` (documented inline).
- [x] **Review A Findings 3/6/8 — narrative cleanup** (`45a97f3`) — `extract_read_paths` docstring names the PostfixAccess parse shape; `reaches()` gains a symmetry comment + regression test; module docstring normalizes "100-cycle cap" / "convergence cap" phrasing to match the emitted diagnostic. Zero behavior change; diagnostic test still passes.
- wos-lint unit tests: 88 → 97 (+9).

### Review B follow-ups — schema cluster (6 commits)

- [x] **Review B Finding 4 — `ProvenanceOutcome` shape simplification** (`3f4bce9`) — rework to match sibling open-enum convention at `wos-kernel.schema.json:803-818`: top-level `type: string` + bare `oneOf: [{enum}, {pattern}]`. No leaf-level duplication. Commit bundled the F5e vendor-regex change to avoid a transient-invalid intermediate shape.
- [x] **#F5d F5b composition story** (`504a48b` + `2e853b7`) — `CapabilityInvocationRecord` $def moved from `schemas/ai/wos-ai-integration.schema.json` to `schemas/kernel/wos-provenance-record.schema.json`. Kernel provenance schema is now the single validation point for the §3.3.1 MUST. AI schema retains only a `$comment` pointer. Spec prose (AI §3.3.1 + Kernel §8.2.2) updated to describe the moved enforcement accurately.
- [x] **#F5e Vendor-extension regex normalization** (`37347a5`, regression test only — regex flip itself landed in `3f4bce9`) — `^x-[a-zA-Z][a-zA-Z0-9-]*$` → `^x-[a-z][a-z0-9-]*$`, matching the established lowercase-kebab convention elsewhere. `x-Acme-Foo` now correctly rejected.
- [x] **#F5c F5a runtime-emission wiring / F3b Task 3** (`a683c03`) — `ProvenanceRecord` gained `pub outcome: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` (roundtrip-safe on existing fixtures). `eval_mode.rs` convergence-cap emission flipped from `ProvenanceKind::CaseStateMutation` + `data.convergenceCapReached: true` to the dedicated `ProvenanceKind::ConvergenceCapReached` variant with `outcome: Some("convergenceCapReached")` and clean `data` payload. **ADR 0059 Task 3 is complete** — F3b remaining scope shrinks to 4 tasks / ~2-3 engineer-days. Crossed the `wos-runtime` fence with mechanical `outcome: None` additions at ~29 literal-constructor sites plus spillover in `wos-core/{explain,event_handler,deontic,autonomy,confidence}` and `wos-conformance`. New regression test `convergence_cap_emits_dedicated_kind_and_outcome_field`.
- [x] **Review B Findings 5 + 6 — edge-case coverage + literal agreement** (`0eb14b2`) — 4 new Python contract tests: `test_outcome_rejects_bare_x_prefix`, `test_absent_invocation_blocked_not_required_outcome`, `test_non_capability_record_kind_with_blocked_flag_not_required_outcome`, plus a cross-schema grep-based smoke test that `preconditionNotSatisfied` agrees across the (now post-move) kernel $def and its `if/then` const. Finding 6 discharge: `const` retained for simplicity; agreement pinned by test.
- `cargo test --workspace`: 1006 → 1012 (+6 net across wos-core + the new regression).

### Review D follow-ups — #40 Task SLA (4 commits)

- [x] **#40a `expectedDuration` rejects `"indefinite"`** (`8b32330`) — drop `indefinite|` branch from `SlaDefinition.expectedDuration` regex; now matches sibling `WarningThreshold.beforeBreach` + `EscalationStep.gracePeriod` duration-only regex. Prose + examples updated; one new negative test. Semantic justification: "indefinite SLA" is an oxymoron since `warningThresholds` + `breachPolicy` have nothing to fire against.
- [x] **#40b `startEvent` kernel event-name pattern** (`d22038c`) — `"pattern": "^[a-zA-Z][a-zA-Z0-9_-]*$"` added. Rejects `$`-prefixed reserved names (`$continuous`, `$join`, `$timeout.*`), empty strings, whitespace. Two new negative tests.
- [x] **#40c `EscalationStep.id` OPTIONAL + `escalationChainRef` contract** (`dea7786`) — added OPTIONAL `id: string` with kernel identifier pattern; `BreachPolicy.escalationChainRef` description now concretely references how level-based vs id-based resolution works. Fixture gained `id: "supervisor"` on step 2.
- [x] **Review D Findings 3 + 4 — calendarRef convention comments + enum negatives** (`62c43cc`) — confirmed `HoldPolicy.notificationTemplateRef` precedent (plain `type: string` for in-document keys, `format: uri` for sidecar URI). Added one-line convention comments to `calendarRef` / `WarningThreshold.templateRef` / `BreachPolicy.templateRef` / `escalationChainRef`. 4 new enum negative tests (`calendarType`, `startAt`, `onExhaustion`, `timeoutPolicy.onRepeatedBreach`).
- Task SLA tests: 27 → 35 (+8).

### Review H follow-ups — #38 Assertion Library (3 commits)

- [x] **#38b + Review H F4/F5/F9 — adoption path + dual-role clarifications** (`c746e9c`) — `specs/governance/assertion-library.md` §2 rewritten with honest adoption story: adopting `AssertionUse` from a consumer schema requires either cross-schema URI `$ref` plumbing (untested territory) OR duplicating the three $defs OR the §4.5 merge which dissolves the choice. New paragraph §2.1 disambiguates `assertionId`'s dual role (inline-standalone vs. library-mirrored). G-064 check (c) tightened to "When an `assertionRef` resolves to a library body that carries its own `assertionId`, that `assertionId` MUST match the library `id`." §2 gained a forward-looking sentence on the §4.5 merge interaction.
- [x] **Review H Finding 7 — "Configuration error" glossary** (`2020c48`) — one-paragraph gloss at top of §2.2 defining "configuration error" as a load-time reject condition. Cross-linkable from any future sidecar spec.
- [x] **Review H Finding 8 — Edge-case negative tests** (`4b0e575`) — 3 new tests in `test_assertion_reference_shape.py`: `assertionRef: ""` rejected via `minLength: 1`; `assertionRef: null` rejected via type mismatch; `assertionRef: "#localFrag"` rejected via `format: uri` requiring absolute URI.
- **Review H Finding 1 (#38a)** — regen no-op (`npm run docs:generate` reported 3 updated artifacts but git saw no diff; `docs:check` was already exit-0 at session start because the 3 stale `.llm.md` files had been regenerated content-identically). No commit needed.
- AssertionReference tests: 12 → 15 (+3).

### Cross-agent coordination notes

- **Transient git churn** between Agents A and B on shared wos-core crate: one agent's `git reset` / `git stash` operations briefly touched the other's uncommitted work. Recovered cleanly via `git stash pop` + `git checkout HEAD --`; no scope-overlap damage. Parallel-agent dispatch on shared crates carries real friction — future sessions should sequence provenance.rs-touching work or introduce a coordination mutex.
- **F3b Task 3 landed ahead of F3b Tasks 1-2** — Agent B completed the emission wiring opportunistically while adding the `outcome` field. Order departed from the ADR's sequential plan but delivered the same end-state; ADR 0059 commit-message cross-reference notes Task 3 closed out-of-band.

### ADR 0059 F3b + Task 5 — Runtime §10.3 + K-049 LoadBearing (`bdf7063`, `f03ca40`)

- [x] **F3b continuous-mode post-mutation guard re-scan** (`bdf7063`) — `Evaluator::rescan_on_mutation`; guard-only transitions participate per Runtime Companion §10.3; `Transition::event` optional with trim-to-absent deserialization; kernel schema + spec alignment.
- [x] **ADR 0059 Task 5 — K-049 LoadBearing + greenfield cleanup** (`f03ca40`) — drop authored `"$continuous"` from `participates_in_continuous_rescan`; synthetic trace/provenance dispatch label `$postMutationRescan`; remove deprecated `try_fire_guardless_transition`; K-049 warning cites §10.3 + `CONVERGENCE_CAP`; rule promoted **LoadBearing** with two `fixtures/validation/k-049-load-bearing-*.json` + `tier2_rules` harness; governance/kernel schema descriptions updated.

### #22a — ProvenanceKind tier-typing (`1240745`, `916d6db`)

- [x] **`wos-core` provenance module split** — `provenance.rs` replaced by `provenance/{mod,snapshot,kind,audit_tier,record,log,tests}.rs`; `ProvenanceAuditTier` (`Facts` | `Narrative`) with `From<ProvenanceKind>`; `audit_layer_for_kind` retained as a string bridge; crate-root re-export.
- [x] **`wos-runtime` stamp path** — `populate_provenance_record_fields` sets `audit_layer` via `ProvenanceAuditTier::from(record.record_kind).as_str()` (typed tier at emission site).

### #20 — Typed event meta-vocabulary (`TransitionEvent`)

- [x] **Kernel JSON Schema** — `$defs/TransitionEvent` + five branch shapes; `Transition.event` and `Action.event` (`startTimer`) reference the union; `signal.name` pattern allows `$join` and `$compensation.complete`.
- [x] **`wos-core`** — `TransitionEvent` with lowercase `kind` tag, camelCase JSON field renames on variants, `from_legacy_string` / `runtime_dispatch_label` / `matches_runtime_dispatch`; optional `Transition.event`; legacy string deserialization on transitions and actions.
- [x] **Eval / runtime / lint / authoring / MCP** — dispatch and lint rules updated; K-007 retained on typed model for `$` misuse on `message` / disallowed `$` signals; K-008 join signal check.
- [x] **Fixtures + migration script** — `scripts/migrate-transition-events.py`; kernel fixtures including `$compensation.complete` as `signal` name with `$` prefix (matches `process_event`).
- [x] **Spec prose** — `specs/kernel/spec.md` §4.5–§4.6, §4.8, §4.10, §9.2; governance `startEvent` reserved-name note.
- [x] **Plan doc** — `thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md` aligned with shipped serde and compensation signal spelling.

### Validation at close

`cargo test --workspace`: **1012 passed / 0 failed**. SCHEMA-DOC-001 zero-regression gate green. `pytest tests/schemas/ -q`: **188 passed / 11 skipped / 1 xfailed** (+17 vs session 8). `npm run docs:check`: exit 0. `git status`: clean.
