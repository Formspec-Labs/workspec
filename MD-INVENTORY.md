# Markdown File Inventory — wos-spec

Generated 2026-04-18 by a swarm of 9 haiku sub-agents, each reading a grouped slice of the repo's 101 non-`node_modules` markdown files. Auto-generated `.llm.md` companions (19 pairs under `specs/`) are noted but not separately summarized — they mirror their canonical `.md` sibling.

**Totals:** 101 md files reviewed · ~80 unique source files · 19 `.llm.md` generated pairs.

**Status legend:** ✅ Implemented · 🟡 In‑Progress · 🔴 Active/Queued · ⏹ Superseded · 📚 Reference/Living · 🗄 Archived · ✨ Aspirational/Planning

---

## 1. Root-Level Docs

### `README.md`

**Summary:** Defines WOS as a JSON-native spec for governing sensitive workflows with AI-native authoring and agent-as-actor runtime. Introduces layered specs across Kernel, Governance, AI Integration, and Advanced Governance plus profiles and companions. Emphasizes that WOS targets LLM-authored workflows and agent governance (not BPMN parity). Reference implementation spans Rust crates, conformance tests, lint rules, and workflow samples.
**Relates to:** POSITIONING, TODO, LINT-MATRIX, WOS-IMPLEMENTATION-STATUS, WOS-FEATURE-MATRIX, enterprise-*.
**Status:** ✅ Implemented — specs shipped, crates green, pre-release.

### `LICENSING.md`

**Summary:** Open-core model: Apache-2.0 for specs/schemas/runtime libs; BSL 1.1 (converting to Apache-2.0 in April 2030) for authoring/tooling. User form definitions are user data, not derivative works. Covers trademark + contribution terms.
**Relates to:** README, enterprise-implementation-roadmap.
**Status:** 📚 Reference/Living Doc — stable terms.

### `POSITIONING.md`

**Summary:** Two-claim thesis: Claim A (LLM-authored workflows via spec→schema→lint→conformance loop); Claim B (agents as first-class runtime actors). Lists eight genuine inventions (deontic operators, structured oversight, due process encoding, epistemic provenance tiers, authority ranking, impact-level behavior, civil-rights monitoring, drift-governance binding).
**Relates to:** README, IDEA_SCRATCH, TODO, enterprise-feature-gaps.
**Status:** 📚 Reference/Living — strategic framing locked.

### `IDEA_SCRATCH.md`

**Summary:** Post-2026-04-16 trim: architectural framing only (no backlog). Five design axes, ground truth (closed vocabularies, six extension seams), non-goals, architectural decisions, audit trail. Tracks cross-project dependencies (Formspec Assist, Formspec Core, Trellis).
**Relates to:** TODO (all actionable items moved there), POSITIONING, README.
**Status:** 📚 Reference/Architectural Framing.

### `TODO.md`

**Summary:** Master backlog last audited 2026-04-18. 197 lint rules, 18 specs, 19 schemas, 146 conformance fixtures all green. Includes 2026-04-18 code review of 7 parallel sub-agents with 5 blockers (§5.3 teaching signal, T3 fixture repair, authoring command sealing, ActorKind mismatch, Custom variant panic) — most cleared by 8 commits that session. Priority sections §1–§7.
**Relates to:** All specs, roadmaps, LINT-MATRIX, IDEA_SCRATCH, enterprise-implementation-roadmap.
**Status:** 🟡 Active Scratch / Living Backlog.

### `LINT-MATRIX.md`

**Summary:** Canonical catalog of 197 constraints across three verification tiers: T1 (37 static single-doc rules), T2 (55 cross-doc + FEL AST rules), T3 (105 dynamic conformance fixtures). Each constraint maps to section, rule ID, why schema can't enforce it, test files. 100% test coverage.
**Relates to:** README, TODO, WOS-IMPLEMENTATION-STATUS, enterprise-feature-gaps.
**Status:** 📚 Reference/Living — canonical rule inventory.

### `WOS-FEATURE-MATRIX.md`

**Summary:** 16-way competitive feature matrix comparing WOS with 15 competitors (ServiceNow, Pegasystems, Appian, Salesforce Gov Cloud, Camunda, KIE, Flowable, Temporal, Palantir AIP, Microsoft Power Platform, LangGraph, AWS Step Functions, Tyler, Bonita, ProcessMaker) across 15 requirement categories. Highlights WOS-unique capabilities (deontic constraints, structured oversight, epistemic provenance tiers, authority-ranked reasoning, impact-level behavior, equity guardrails, Assist Governance Proxy). §15 positions WOS as governing existing engines rather than replacing them.
**Relates to:** README, enterprise-feature-gaps, POSITIONING.
**Status:** 📚 Reference/Comparative Analysis — updated 2026-04-14.

### `WOS-IMPLEMENTATION-STATUS.md`

**Summary:** Crate maturity tracker (wos-core/lint/conformance/runtime/formspec-binding/export ✅, wos-assurance 🟡). 197/197 rules verified. Covers Formspec Coprocessor S15, durable execution, timer materialization, JSON-LD native, SHACL shapes, SPARQL queryability, layered opt-in design, sidecar pattern. Phase roadmap (Phase 1 engine bindings, Phase 2 Merkle provenance, Phase 3 adoption) with Appendix A (6 feature upticks) and Appendix B (standards lineage).
**Relates to:** README, TODO, LINT-MATRIX, enterprise-implementation-roadmap.
**Status:** ✅ Implemented / 📚 Reference — phase roadmap forward-looking.

### `enterprise-feature-gaps.md`

**Summary:** Strategic gap analysis (2026-04-07) comparing Formspec + WOS against Adobe AEM Forms, DocuSign, ServiceNow across 13 gap domains. Key insight: WOS spec suite closes case management gap from "no spec" to "comprehensive specs exist — implement SaaS layer." True critical gaps are reviewer dashboard, document storage, FedRAMP/SOC 2/GSA Schedule.
**Relates to:** enterprise-implementation-roadmap, WOS-FEATURE-MATRIX, README.
**Status:** 📚 Reference/Strategic Analysis.

### `enterprise-implementation-roadmap.md`

**Summary:** Six-phase implementation plan (12–24 months to government pilot, 24+ to enterprise scale). Phase 1 Core Intake Loop (authoring studio, conversational runtime, WOS Coprocessor S15). Phase 2 Trustworthy Intake (crypto audit ledger, SOC 2, WCAG/VPAT). Phase 3 Intelligent Intake (References/Ontology, AI governance). Phase 4 Government-Ready (FedRAMP Moderate 3PAO, GSA Schedule). Phase 5 Operator Leverage. Phase 6 Ecosystem. Includes ADR-0001→0016 dependency matrix + 11 assumptions.
**Relates to:** enterprise-feature-gaps, TODO, WOS-IMPLEMENTATION-STATUS, all specs.
**Status:** ✨ Aspirational/Planning — supply-driven bet, no customers yet.

---

## 2. Canonical Specs (`specs/`)

Each file below has a paired `.llm.md` (auto-generated, identical content structure) unless noted. Only canonical `.md` summarized.

### Kernel — `specs/kernel/`

#### `specs/kernel/spec.md`

**Summary:** Minimal orchestration substrate for WOS. Covers lifecycle topology (statecharts: states, transitions, guards), case state (append-only typed data), actor model (human + system), impact-level classification, evaluation context, Facts provenance tier, durable execution (crash recovery, deterministic replay), and six extension seams for governance/custody/provenance.
**Relates to:** `schemas/kernel/wos-kernel.schema.json`; Formspec Core §1.4; all other specs attach via seams.
**Status:** ✅ Canonical Spec (v1.0.0-draft.1, 2026-04-09).
**LLM pair:** ✅

#### `specs/kernel/correspondence-metadata.md`

**Summary:** Sidecar defining structured metadata schema for correspondence entries (mail, phone, email, portal, fax) stored in case state. Declares channels, direction, actor types, required fields per entry template. Pure metadata — does not modify kernel event semantics.
**Relates to:** Kernel §5.2 case state.
**Status:** ✅ Canonical Sidecar (draft).
**LLM pair:** ✅

### Profiles — `specs/profiles/`

#### `specs/profiles/integration.md`

**Summary:** Parallel seam for WOS service integration. Defines six binding types (request-response, arazzo-sequence, tool, event-emit, event-consume, callback, policy-engine) with interface refs, I/O mappings, retry policies, and optional Formspec Definition contracts. Specifies CloudEvents extension attrs, correlation rules, idempotency, and external policy engine bridge (XACML/OPA/Cedar).
**Relates to:** Kernel §9.2/§10.2; Formspec Core; CloudEvents 1.0, Arazzo, CWL.
**Status:** ✅ Canonical Profile (parallel seam).
**LLM pair:** ✅

#### `specs/profiles/semantic.md`

**Summary:** Parallel seam for linked-data interpretation and provenance mapping. Declares JSON-LD `@context`, SHACL shape refs (8 categories), PROV-O mapping, XES/OCEL export. Pure interpretation overlay — does not transform WOS processing semantics.
**Relates to:** Kernel §8.4/§10.3; W3C PROV-O, JSON-LD 1.1, SHACL; IEEE XES, OCEL 2.0, LegalRuleML.
**Status:** ✅ Canonical Profile.
**LLM pair:** ✅

#### `specs/profiles/signature.md`

**Summary:** Parallel seam for signature workflow semantics. Declares signer roles, sequential/parallel/routed/free-for-all signing flows, consent and identity-binding evidence, document hashes, reminders, expiry, decline, void, reassignment, and `SignatureAffirmation` provenance emitted through `custodyHook`.
**Relates to:** Kernel actors/tasks/timers/provenance; Formspec canonical response fields; Trellis custody/export bundle.
**Status:** Draft Profile.
**LLM pair:** — (pending)

### Assurance — `specs/assurance/`

#### `specs/assurance/assurance.md`

**Summary:** Normative companion defining identity and attestation semantics for rights-impacting/safety-impacting workflows. Assurance-level taxonomy (L1 self-asserted → L4 in-person equivalent), subject-continuity primitives, Invariant 6 (disclosure posture independent from assurance level), provider-neutral attestations, legal-sufficiency disclosure obligations.
**Relates to:** Kernel §10.3/§10.5 seams; Formspec Response signing; Trellis.
**Status:** ✅ Canonical (normative companion).
**LLM pair:** — (none — prose-only)

### Governance — `specs/governance/`

#### `specs/governance/workflow-governance.md`

**Summary:** Core governance spec (Layer 1) for regulated high-stakes human workflows. Declares due process (notice/explanation/appeal/continuation-of-service), review protocols (independentFirst, dualBlind, calibratedConfidence), data validation pipelines, structured audit (Reasoning + Counterfactual tiers), quality controls, rejection/remediation, task management, delegation of authority, typed hold policies, temporal parameter resolution.
**Relates to:** Kernel §10 seams; Formspec Core; Equity Config, Agent Config, AI Integration (Layer 2).
**Status:** ✅ Canonical Draft (Layer 1).
**LLM pair:** ✅

#### `specs/governance/assertion-library.md` — ABSORBED 2026-05-07

**Summary:** Merged into `workflow-governance.md` §14 (Named Assertions). Assertion `$def`s live in `wos-workflow.schema.json`.
**Status:** ❌ Retired (absorbed).

#### `specs/governance/due-process-config.md` — ABSORBED 2026-05-07

**Summary:** Merged into `workflow-governance.md` §15 (Appeal Routing). Schema surface at `Governance.dueProcess` in `wos-workflow.schema.json`.
**Status:** ❌ Retired (absorbed).

#### `specs/governance/policy-parameters.md`

**Summary:** Sidecar providing date-indexed parameter values for temporal parameter resolution (income thresholds, appeal deadlines) and regulatory version bindings. Follows OpenFisca model.
**Relates to:** workflow-governance §13; Kernel §7.3/§10.2.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

### Advanced — `specs/advanced/`

#### `specs/advanced/advanced-governance.md`

**Summary:** Layer 3 spec for formally verifiable constraints, statistical fairness monitoring, adaptive case management, multi-step agent interactions, operational resilience. Declares verifiable constraint subset (SMT-amenable FEL fragment), equity guardrails (disparity monitoring), constraint zones (DCR-style), multi-step sessions with checkpoints, tool-use governance, agent lifecycle state machines, calibration/drift detection, shadow mode, circuit breakers.
**Relates to:** Kernel v1.0; workflow-governance (Layer 1); ai-integration (Layer 2, extended); equity-config sidecar.
**Status:** ✅ Canonical Draft (Layer 3). **LLM pair:** ✅

#### `specs/advanced/equity-config.md`

**Summary:** Sidecar for equity monitoring configuration — protected categories, disparity calculation methods, automated reporting, remediation triggers.
**Relates to:** advanced-governance §3.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

#### `specs/advanced/verification-report.md` — ABSORBED (ADR 0076 D-4)

**Summary:** Runtime half absorbed into `wos-provenance-log.schema.json`; standalone spec doc never existed at HEAD.
**Status:** ❌ Retired (absorbed).

### AI — `specs/ai/`

#### `specs/ai/ai-integration.md`

**Summary:** Layer 2 governance spec defining agent registration, deontic constraints (permission/prohibition/obligation/right), autonomy levels with impact caps, Formspec-as-validator pattern, confidence framework with decay/calibration, fallback chains, decision drift detection, AI-specific oversight (suppression, sampling), volume constraints, agent disclosure, Narrative provenance tier, Assist Governance Proxy.
**Relates to:** Kernel seams (actorExtension, contractHook, provenanceLayer, lifecycleHook); workflow-governance (Layer 1, extended); Formspec Core; agent-config + drift-monitor sidecars.
**Status:** ✅ Canonical Draft (Layer 2, v1.0.0-draft.1). **LLM pair:** ✅

#### `specs/ai/agent-config.md`

**Summary:** Operational sidecar — agent endpoints, approved model versions, calibration requirements, autonomy escalation/demotion rules, per-action overrides. Separates runtime parameters from governance structure (enables credential rotation without governance changes).
**Relates to:** ai-integration §3–§7/§14; drift-monitor.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

#### `specs/ai/drift-monitor.md`

**Summary:** Operational sidecar for drift detection — tracks agent behavior via metrics (accuracy, confidence distribution, rubber-stamp rates), supports shadow/canary/production deployment, alert thresholds trigger autonomy demotion.
**Relates to:** ai-integration §9/§5; agent-config; Runtime Companion.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

### Sidecars — `specs/sidecars/`

#### `specs/sidecars/business-calendar.md`

**Summary:** Calendar sidecar — business-day work weeks, holiday schedules (fixed + floating), operating hours. Controls SLA evaluation + temporal parameter resolution (working days vs. wall-clock).
**Relates to:** Kernel §9.7 timers; governance §10.3/§13.3; runtime companion.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

#### `specs/sidecars/notification-template.md`

**Summary:** Template sidecar for government notice generation — template categories, required sections, placeholder variable resolution, delivery channels. Ensures legally adequate notice with individualized reasoning + filing deadlines.
**Relates to:** governance §3/§12; business-calendar; Kernel; runtime companion.
**Status:** ✅ Canonical Sidecar (draft). **LLM pair:** ✅

### Companions — `specs/companions/`

#### `specs/companions/lifecycle-detail.md`

**Summary:** Companion elaborating kernel lifecycle execution with detailed algorithms: transition evaluation pseudocode, history state semantics (shallow/deep), parallel execution (region sync, nested parallelism, join/cancel policies), compensation execution (reverse ordering, pivot steps, recovery modes), timer lifecycle, SCXML interoperability mapping.
**Relates to:** Kernel §4/§9; runtime companion; Formspec Core convergence.
**Status:** ✅ Canonical Draft (v1.0.0-draft.1). **LLM pair:** ✅

#### `specs/companions/runtime.md`

**Summary:** Comprehensive runtime contract between WOS engine and host. Covers WorkflowProcess serialization, instance ops (create/process/suspend/migrate), event delivery (serial, exactly-once dedup), action execution, durability checkpoints, timer management, governance enforcement ordering, explanation assembly, evaluation modes (event-driven + continuous with convergence cap), multi-version coexistence, host interfaces (InstanceStore, DocumentResolver, ContractValidator, ExternalService, etc.), security model, relationship-triggered events, Formspec task coprocessor (S15).
**Relates to:** Kernel §4–§9; lifecycle-detail; governance enforcement ordering; Formspec Core (task binding, validation); mapping spec; Respondent Ledger.
**Status:** ✅ Canonical Draft. **LLM pair:** ✅

---

## 3. Crate / Package READMEs

### `crates/wos-conformance/README.md`

**Summary:** Dynamic conformance test runner executing event sequences against WOS kernel documents to verify 104 T3 constraints (determinism, provenance completeness, timer behavior, compensation, deontic enforcement, autonomy caps, confidence, hold/resume, DCR zones). Covers runtime guarantees static lint cannot detect.
**Relates to:** wos-lint (T1/T2), wos-core (evaluation kernel), fel-core, LINT-MATRIX (187-rule catalog).
**Status:** ✅ Implemented.

### `crates/wos-core/README.md`

**Summary:** Typed domain model + evaluation kernel for WOS. Rust types for kernel/governance/AI/sidecar documents; pure evaluator with typed state; shared provenance/timer/deontic/autonomy/confidence/explanation logic; host trait interfaces.
**Relates to:** wos-lint (types consumer), wos-conformance (shared evaluator), future runtime adapters, fel-core, lifecycle-detail companion §2, runtime companion §12–§13.
**Status:** ✅ Implemented.

### `crates/wos-lint/README.md`

**Summary:** Static linter enforcing 83 normative constraints in two tiers — T1 (32 single-doc structural) + T2 (51 cross-doc + FEL AST). Complements JSON Schema validation.
**Relates to:** JSON Schema, wos-core, fel-core, wos-conformance, LINT-MATRIX.
**Status:** ✅ Implemented.

### `crates/wos-mcp/README.md`

**Summary:** MCP adapter exposing WOS authoring ops as JSON-RPC 2.0 tools for external clients (Claude Desktop, Cline via stdio) and in-process Rust callers. Hand-rolled transport (~130 LOC, no SDK) replaces rust-mcp-sdk to avoid hyper/axum/reqwest bloat.
**Relates to:** wos-synth-core, wos-bench, tests, MCP clients, runtime helpers.
**Status:** 🟡 Partial — scaffold + transport + ping tool exist; ProjectRegistry marked stub.

### `case-portal/README.md`

**Summary:** Browser-based case management portal for WOS state — inbox, form workspace, case viewer, process dashboard, workflow designer, admin console, audit trail, applicant portal, report builder. React 19 + Express + Socket.IO. Hexagonal architecture (port/adapter); auto-generated TS types from JSON schemas. Renamed 2026-05-02 from `studio/` (was `@formspec-org/wos-studio`); the `/studio` path now hosts the WOS Studio (Authoring) layer.
**Relates to:** WOS schemas, fixtures, Formspec contract refs, WosDocumentBundle contract.
**Status:** 🟡 Implemented (PoC) — transitioning from fixture-backed to reference backend.

### `case-portal/HANDOFF.md`

**Summary:** Architecture review + 55-task remediation plan transitioning Case Portal (formerly Studio) from fixture-backed PoC to reference backend. 2 blockers (designer round-trip data loss, hard-coded E2E IDs), 16 warnings, 11 nits. 7 phases with review gates.
**Relates to:** Case Portal codebase (WosPorts, hexagonal, FixtureAdapter, HttpWosBackend target), E2E framework, schema-driven type generation.
**Status:** 🔴 Planning — review complete 2026-04-16, implementation pending.

### `case-portal/e2e/README.md`

**Summary:** Playwright E2E framework with Human-Driven Design philosophy — behavior-driven scenarios (Given/When/Then) rather than implementation details. Three journeys: Efficient Triage, Workflow Evolution, Transparency & Trust.
**Relates to:** Case Portal UI, fixtures, Playwright config.
**Status:** 🟡 Partial — framework present; HANDOFF.md flags broken E2E tests as Phase 2 blocker.

### `case-portal/src/types/wos/README.md`

**Summary:** Auto-generated TS types from WOS JSON Schema (json-schema-to-typescript). Corresponds to spec; planned extraction to shared `@wos/types` package.
**Relates to:** WOS schemas, Case Portal frontend/server, generate-wos-types.ts script.
**Status:** ✅ Implemented — shared-package extraction pending.

### `tests/README.md`

**Summary:** Pytest suite protecting WOS schema surface under `tests/schemas/`. Meta-validity (schemas valid JSON Schema 2020-12), fixture validity (fixtures validate against declaring schema), negative fixtures (invalid docs rejected), spec-example tests (fenced JSON in specs passes). Auto-registration via conftest.py marker mapping.
**Relates to:** schemas/, fixtures/, specs/, JSON Schema 2020-12.
**Status:** ✅ Implemented.

### `benchmarks/problems/purchase-order-approval.md`

**Summary:** Problem spec for procurement workflow — requester submits PO, approver reviews with $50K threshold fork (direct approval below / director above), rejection at any point. Terminal states: approved/rejected/cancelled/completed. Requires audit trail. Maps to `fixtures/kernel/purchase-order-approval.json`.
**Relates to:** WOS kernel spec, conformance fixtures, synthesizer success criteria.
**Status:** 📚 Reference.

---

## 4. `thoughts/plans/` — Dated Implementation Plans

(All paths under `thoughts/plans/`; dates are filename prefixes.)

### `0059-unified-ledger-as-canonical-event-store.md`

Stack **narrative lock** (2026-04-22): ADR-0059 is the **Phase 3+ architecture target** (single append-only case spine); delivery is phased per Trellis `product-vision.md`; full technical ADR lives in parent repo `thoughts/adr/0059-unified-ledger-as-canonical-event-store.md`.
**Status:** 🔒 Narrative locked — not a dated implementation sprint; cross-repo program intent.

### `2026-04-10-wos-core-extraction.md`

Extract wos-core from wos-conformance; scaffold typed KernelDocument/State/Transition, evaluator, timer, provenance. wos-lint + wos-conformance depend inward.
**Status:** ✅ Implemented — wos-core crate 9,701 LOC, 146 fixtures green.

### `2026-04-13-wos-runtime-crate.md`

Build wos-runtime for instance lifecycle, event processing, action dispatch, timer management, provenance coordination. Implements §12 (host traits) + §15 (Formspec coprocessor).
**Status:** ✅ Implemented — 8,730 LOC; DrainOnceResult carries guard_evaluations (4 commits Apr 18).

### `2026-04-14-wos-spec-section-1-implementation.md`

Close §1 TODO via 11 slices: S15.1-3 (binding-backed conformance), KS.1-2 (history + milestones), BC.1 (business calendar SLA), NB.1-4 (six integration-profile binding kinds). TDD-driven, 10.5 engineer-days.
**Status:** 🟡 In-Progress — Slice 1 (§5.3 guard evaluation) landed Apr 18; slices 2-11 pending.

### `2026-04-15-wos-custody-and-assurance.md`

Adds custodyHook seam (§10.6), Assurance layer (identity/attestation/continuity), Governance extensions (schema upgrade, quorum delegation, legal hold), feature-matrix updates. Invariant 6 (disclosure ≠ assurance) normative here.
**Status:** 🔴 Active/Queued — 787-line plan, 14 tasks; no artifacts yet; prerequisite for Trellis integration.

### `2026-04-15-wos-provenance-export.md`

Implement provenance export from wos-core ProvenanceLog to W3C PROV-O (JSON-LD), IEEE 1849 XES (XML), OCEL 2.0 (JSON). Implements Semantic Profile §§5–6.
**Status:** ✅ Implemented — wos-export crate 1,965 LOC with prov_o/xes/ocel modules; 3 conformance fixtures.

### `2026-04-16-wos-provenance-record-schema-extension.md`

Add 8 optional fields to ProvenanceRecord (audit_layer, actor_type, lifecycle_state, definition_version, inputs/outputs, digests) closing gap vs. Semantic Profile mapping tables. Runtime stamp pass populates; exporters emit full mappings.
**Status:** ✅ Implemented — all 8 fields + stamp pass + exporter emissions landed Apr 16-18.

### `2026-04-16-wos-release-trains.md`

Split WOS releases into four independent streams (kernel/governance/ai/advanced) via Changesets + per-stream git tags (ADR 0063). Vendors claim `wos-kernel@1.0 + wos-ai@0.5`.
**Status:** 🔴 Active/Queued — 227-line plan, 5 tasks; no Changesets infra yet; depends on rule-coverage completion.

### `2026-04-16-wos-rule-coverage-conformance.md`

Replace fixture-count metric with rule-coverage metric — every LINT-MATRIX rule links to ≥1 passing fixture; rule-graduation ladder (Draft → Tested → Stable → LoadBearing); CI enforces link.
**Status:** 🟡 In-Progress — Tasks 1–2 landed (7 rules promoted to Tested); 97/197 reified; Tasks 3–7 pending.

### `2026-04-16-wos-schema-description-audit.md`

Every schema property: description ≥60 chars + ≥1 example. Critical props (x-lm.critical=true): ≥140 chars + ≥2 examples. SCHEMA-DOC-001 lint rule enforces. Enables LLM authoring (Claim A).
**Status:** 🟡 In-Progress — SCHEMA-DOC-001 + triage (901→815 violations) + reshape pre-pass landed; per-tier backfill pending (~2 engineer-weeks).

### `2026-04-16-wos-structured-lint-diagnostics.md`

Turn wos-lint output from strings → LintDiagnostic JSON (ruleId, severity, tier, path, message, suggestedFix, relatedDocs, source). CLI gets `--format=text|pretty|json|json-lines`. Prerequisite for §5.4 (LLM repair prompts).
**Status:** 🟡 In-Progress — LintDiagnostic type + golden tests landed; custom-variant panic fixed; 91 rules still to migrate.

### `2026-04-16-wos-synth-crate.md`

Build wos-synth as 4 crates (core + anthropic + mock + cli) per DIP. Core owns loop + Prompter + ToolContext traits. Loop: generate → diagnostics → targeted repair → diagnostics → stop.
**Status:** 🔴 Queued — 387-line plan; v0 spike lands first to validate architecture.

### `2026-04-16-wos-synthesis-benchmark.md`

Pair every fixture with NL problem statement. Benchmark tracks convergence rate + step accuracy + T3-pass-rate per run. Separate wos-bench crate. Monthly BENCHMARK.md leaderboard.
**Status:** 🔴 Queued — 156-line plan, 4 tasks; depends on wos-synth-core.

### `2026-04-16-wos-trace-emitting-conformance.md`

ConformanceTrace captures expected vs. actual per step, guard evaluations, policy applications, deltas. Runtime tests emit traces into `target/conformance-traces/`. CLI explain/diff subcommands.
**Status:** 🟡 In-Progress — type + runner + goldens landed; teaching-signal blocker cleared Apr 18; explain/diff CLI + schema publication pending.

### `2026-04-17-wos-authoring-crate.md`

Intent-driven authoring API over wos-core via private command pipeline (dispatch → handler → state update → diagnostics → undo). Public surface: 28 intent-level helpers (add_state, add_transition, add_actor…). Mirrors formspec-studio-core.
**Status:** 🟡 In-Progress — 943 LOC; Tasks 1-3 landed, Apr 18 blockers cleared (Command sealed pub(crate), ActorKind/ImpactLevel aligned); Tasks 4-8 pending.

### `2026-04-17-wos-mcp-crate.md`

Dual-entry MCP adapter over wos-authoring: JSON-RPC-2.0 stdio + in-process dispatch for wos-synth-core ToolContext. Zero business logic — every handler delegates. 22 tools across 6 families.
**Status:** 🟡 In-Progress — 459 LOC; Tasks 1-2 landed; 6 review warnings open; Tasks 3-6 pending (depends on authoring Tasks 4+).

### `2026-04-17-wos-schema-regression-tests.md`

Three pytest layers protect WOS schemas — meta-validity, fixture validity, spec-example validity. CI enforces regression.
**Status:** ✅ Implemented — 6 commits; 72 pytest pass / 2 skip / 1 xfail; CI gate active.

### `2026-04-17-wos-synth-v0-spike.md`

2–3 day disposable spike validating wos-synth architecture before full build. Single crate, imperative code. Output: retrospective answering whether Prompter/ToolContext/dual-entry MCP/28 helpers are necessary vs. YAGNI.
**Status:** 🟡 In-Progress — 529/800-LOC cap; 9 unit tests green; Apr 18 review warnings fixed; Tasks 4-5 (conformance gate + retrospective) pending.

---

## 5. `thoughts/` — Archive, Examples, Practices, Research, Reviews, Specs

### `thoughts/archive/adr/0057-wos-core-implementation-boundary.md`

ADR defining WOS core vs. implementation-adapter boundaries — 7 core behaviors vs. 7 impl traits. Alternative 3 (Runtime Companion spec) adopted and already landed in `specs/companions/runtime.md`.
**Status:** 🗄 Archived — decisions implemented in Runtime Companion.

### `thoughts/archive/adr/0058-wos-core-gap-analysis.md`

Gap analysis identifying 7 spec constructs missing (case linking, regulatory effective dating, delegation of authority, redetermination cycles, batch operations, correspondence events, typed holds). 4 accepted, 2 rejected, 1 accepted with modification.
**Status:** 🗄 Archived — proposals implemented.

### `thoughts/archive/reviews/2026-04-16-architecture-review-handoff.md`

Maintainer-guided architecture review — affirms 18 schemas + 3-tier verification justified. Identifies 4 hygiene bugs + 6 AI-native positioning work items (~1 engineer-month backlog).
**Status:** 🗄 Archived — decisions flowed into follow-up plans.

### `thoughts/archive/reviews/2026-04-16-architecture-review-open-questions.md`

Six open questions from architecture review with synthesized answers from 3 reviewers. Resolves Q1 (Claim A first-class), Q2 (wos-synth in-tree), Q3 (Changesets + per-stream tags → ADR 0063), Q4 (K-012/K-017 audit), Q5 (COMPAT.md matrix), Q6 (wos-synth + wos-bench separate crates).
**Status:** 🗄 Archived — decisions propagated into plans + ADRs 0063/0064/0065.

### `thoughts/archive/specs/2026-04-11-wos-s15-formspec-coprocessor-proposal.md`

Concrete proposal for Runtime Companion §15 (Formspec Coprocessor Protocol). Defines task pinning, FormspecTaskContext shape, TaskPresenter interface, coprocessor validation algorithm, submitTaskResponse steps, kernel/workflow-process schema updates, Respondent Ledger integration.
**Status:** 🗄 Archived — merged into Phase 11 master spec 2026-04-11; remains as §15 source.

### `thoughts/examples/medicaid-redetermination-user-stories.md`

End-to-end 6-act narrative: County Medicaid redetermination workflow — administrator designs form+workflow+AI agent rules → respondent intake → caseworker review with independent-first protocol + AI checks → adverse notice → appeal → supervisor → equity/drift monitoring → federal audit. Demonstrates spec value proposition.
**Status:** 📚 Active Reference — validates spec completeness.

### `thoughts/examples/temporal-reference-implementation.md`

Technical architecture for Formspec + WOS + Temporal integration — wos-temporal crate structure, workflow event loop, governance evaluation, activities, coprocessor, human task integration. Demonstrates Temporal providing crash recovery + state persistence + durable timers, WOS providing governance.
**Status:** 📚 Active Reference — informs runtime-trait design (ADR-0057).

### `thoughts/practices/2026-04-17-parallel-agent-dispatch.md`

Team discipline for parallel sub-agent dispatching in shared git repos. Five rules: one writer per file, no parallel retries without confirmation, worktrees for high-risk parallelism, hot-file sequencing, post-dispatch reconciliation check.
**Status:** 📚 Active Practice.

### `thoughts/practices/README.md`

Index of practices documents — currently only parallel-agent dispatch.
**Status:** 📚 Active Reference.

### `thoughts/research/2026-04-17-k012-k017-load-bearing-audit.md`

Mechanical audit whether lint rules K-012 (guard FEL syntax) + K-017 (no cross-case guard refs) qualify for LoadBearing promotion. Finding: correct + tested, but zero shipped fixtures carry violations. Hold at Stable until negative-pattern fixtures exist.
**Status:** 📚 Active Reference — executed per open-questions Q4.

### `thoughts/research/2026-04-17-wos-schema-doc-audit-triage.md`

Comprehensive audit of SCHEMA-DOC-001 violations — 901 violations across 603 pointers across 19 schemas. 56% Backfill / 44% Reshape (boilerplate extensions/$schema/version/title/description) / <2% Delete. Per-schema breakdown + Task 3 sequencing.
**Status:** 📚 Active Reference — triage input to schema-description-audit plan.

### `thoughts/reviews/2026-04-09-wos-core-companion-review.md`

Implementation plan derived from ADR-0057 + ADR-0058 gap analysis. Maps 7 spec additions to phases, success criteria, content recovery. Roadmap linking ADRs → landed specs.
**Status:** 📚 Active Reference.

### `thoughts/specs/2026-04-11-formspec-wos-phase11-integration-master.md`

Master integration spec merging 3 Phase 11 sources (gap analysis, coprocessor proposal, FEL plan). Documents 3 gaps: Formspec coprocessor protocol (→ Runtime §15 + Kernel §9.2/§11.3), FEL quantifiers `every`/`some`/`duration` (→ Core §3.5), record predicates + `$` semantics (→ Core §3.5.1 + ADR-0060). Defines 8 review biases.
**Status:** 📚 Active Reference — live master handoff for Phase 11; normative source for coprocessor semantics.

---

## 6. `thoughts/archive/drafts/` — Historical Core Drafts (archived 2026-04-20)

The 13 files in this archive (12 markdown + 1 schema snapshot) were moved from
the top-level `DRAFTS/` directory during TODO §4.1 DRAFTS triage (2026-04-20).
All content is superseded by `specs/kernel/spec.md` v1.0.0-draft.1 and the
canonical tier specs under `specs/`. See
[`thoughts/archive/drafts/README.md`](thoughts/archive/drafts/README.md)
for the full classification table and rationale.

Highlights retained for historical reference:

- **Superseded kernel iterations** — `wos-core-spec.md` (v0.1 baseline),
  `wos-core-v2.md` (8-layer, 21 profiles), `wos-core-v3.md` (JSON-LD + SHACL),
  `wos-core-v4.md` (constraint-enhanced layered kernel), `wos-core-v5.md`
  (Formspec-as-interface-contract), `wos-core-v6.md` (community-review draft),
  `wos-core-agent-amendments.md` (agent terminology added to v0.1).
- **v7 reframe drafts** — `wos-core-v7-kernel.md` and `wos-core-v7-proposal.md`
  shaped the canonical kernel/profile split.
- **Tier-spec ancestors** — `wos-agent-tier-spec.md` + `wos-agent-tier-v7.md`
  (near-duplicates; content now in `specs/ai/`), `wcos-lifecycle-spec.md`
  (Harel statechart semantics folded into kernel §4 lifecycle topology).
- **Schema snapshot** — `wos-core-v7.schema.json` replaced by the 21 production
  schemas under `schemas/`.

---

## 7. Research

### `research/compass_artifact_wf-91189436-c8d3-4e27-9159-57565301cb69_text_markdown.md`

Comprehensive landscape study of 50+ workflow standards/systems (BPMN, CMMN, DMN, SCXML, Temporal.io, W3C PROV, etc.) — executive synthesis, feature taxonomy, feature matrix, architecture recommendations (hybrid state-machine-centric with layered concerns), four ranked lists (design principles, underserved capabilities, traps to avoid, standards to watch). Load-bearing rationale for wos-spec architectural direction.
**Status:** 📚 Reference — informs architecture decisions.

### `research/prompts/research-prompt.md`

Research directive/RFP that prompted the landscape study — >50 standards across 12 domains, Adopted/Adapted/Missing/Out-of-scope classification, feature taxonomy, AI-agent oversight/provenance/audit/conformance, architecture recommendations, ranked lists.
**Status:** 📚 Planning/Reference — methodology charter. Candidate for `thoughts/archive/research/` once design phase closes.

---

## Cross-Cutting Observations

**Spec maturity ladder:** Canonical specs under `specs/` are at v1.0.0-draft.1 unless marked otherwise. Signature Profile is a newer draft profile added by WOS-T4. The core layers (kernel + profiles + governance + advanced + ai + companions) are Canonical Draft; sidecars compose with them.

**Implementation progress (crates):** wos-core + wos-lint + wos-conformance + wos-runtime + wos-export + wos-formspec-binding are ✅ Implemented. wos-assurance + wos-mcp + wos-authoring + wos-synth-spike are 🟡 In-Progress. wos-synth-core/anthropic/mock/cli + wos-bench are 🔴 Queued.

**Active planning clusters (April 2026):**

1. **AI authoring pipeline** — schema-description-audit → structured-lint-diagnostics → trace-emitting-conformance → wos-synth-v0-spike → wos-synth-core → wos-bench (Claim A reference implementation).
2. **Authoring/MCP layer** — wos-authoring (28 intent helpers) → wos-mcp (22 tools, dual-entry) → wos-synth-core ToolContext.
3. **Release hygiene** — rule-coverage-conformance (replace fixture count with rule coverage) → release-trains (4 independent streams per ADR 0063).
4. **Identity/Trellis prep** — custody-and-assurance seam + Assurance layer (prerequisite for Trellis integration).

**Technical debt / legacy:**

- Historical kernel drafts (13 files) were archived to `thoughts/archive/drafts/` on 2026-04-20 — all superseded by current `specs/kernel/` + tier specs.
- `thoughts/archive/` contains 5 files already archived by convention.
- Research prompt is a candidate for archival once design phase closes.

**File relationships (high-level graph):**

- Every spec/plan/review ultimately traces back to `specs/kernel/spec.md` (the canonical substrate) via one of six extension seams.
- LINT-MATRIX is the denominator for all conformance claims (197 rules); rule-coverage plan makes this metric normative.
- TODO.md is the live heartbeat; IDEA_SCRATCH is the architectural compass; POSITIONING is the strategic frame.
- Studio is the reference UI; its HANDOFF.md is the largest active remediation plan (55 tasks) in the project.
