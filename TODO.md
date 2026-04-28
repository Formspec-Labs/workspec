# WOS TODO

Working backlog for the Workflow Orchestration Standard specification suite. Session narratives and all closed items live in [COMPLETED.md](COMPLETED.md); **stack-wide** architectural commitments and scope lines live in repo-root [`VISION.md`](../VISION.md) (WOS-specific settled positions in §X there; owner operating prefs remain in [`.claude/user_profile.md`](../.claude/user_profile.md)). This file indexes active work, blocked items, and trigger-gated future work.

**Last audited:** 2026-04-28 — **ADR 0076 substantively closed; workspace fully green** (`cargo test --workspace --no-fail-fast --tests` reports **1244 passed / 0 failed**, canonical-seams clean on 73 files). All 12 ADR 0076 architectural-plan steps + within-section expansion pass + un-migrated-tail disposition + citation sweep landed. **kernel/spec.md grew 741 → 2196 lines** (16 top-level chapters per **D-8 amendment** append-at-end §14/§15, preserving §2 + §6 anchors). **Five rounds of /semi-formal-code-review actioned**: wos-scout architectural F1/F2/F3, wos-expert spec-fidelity E1-E8 + D-8 amendment recommendation, wos-spec-author authoring-discipline F1-F7, wos-scout user-value Q1/Q2/Q3 ceremony trim, wos-expert + wos-spec-author full-pass review (sub-section renumber CRITICAL/BLOCKER fixed; ratchet doc-comment 40↔79 drift reconciled; FEL syntax hallucination in `State.collection.examples` fixed; `Transition.guard` description filled; `x-lm.critical` rebalanced — promoted `guard`/`actor`/`mutationSource`/`verificationLevel`, demoted `assurance`/`advanced`; CONVENTIONS scope extended to embedded-block specs; sidecars README anyOf gate documented; ratchet gameability concern documented for next-pass leaf-count companion). ADR 0077 fully implemented (D-5 follow-ups in `kernel/spec.md` + `wos-kernel.schema.json`; row E9 + "Seam vocabulary drift" section in `counter-proposal-disposition.md`; new vocabulary-drift CI gate at `scripts/check-canonical-seams.py` wired into `.github/workflows/schema-regression.yml`). **Architectural cut completed cleanly — no compat shims, no legacy markers:** `crates/wos-conformance/src/marker_shim.rs` deleted; six standalone Rust document types (`AIIntegrationDocument`, `GovernanceDocument`, `BusinessCalendarDocument`, `NotificationTemplateDocument`, `SignatureProfileDocument`, `IntegrationProfileDocument`) refactored to represent embedded-block content per ADR 0076 D-1 (marker fields stripped; type names retained for consumer compatibility); conformance engine extracts embedded blocks via `extract_embedded_block` + `embedded_block_for_role` mapping (kebab-case + camelCase variants); `drift_monitor` / `agent_config` consumers in `wos-runtime/companion.rs` updated to identify by shape (not legacy marker); `KernelDocument.wos_kernel` → `wos_workflow` rename propagated across 105 Rust files; `wos.dev` → `wos-spec.org` host unification across all schema $ids. **wos-scout semi-formal code review** dispatched and actioned — three findings (F1 standalone-doc legacy pin / F2 `inline_documents` skipped / F3 host inconsistency) all resolved. Initial 6 of 12 implementation-plan steps — governance spec absorption (`workflow-governance.md` 727→814 lines, runtime §8.4/§8.5/§9 absorbed into §3.8/§11.4/§12.4), AI spec absorption (`ai-integration.md` 680→682 lines, runtime §8.3 + integration §8.4 absorbed into §4.6 deontic ordering), sidecar split (`wos-delivery.schema.json` 676 lines merging calendar+notification+correspondence; `wos-ontology-alignment.schema.json` 385 lines renamed from semantic-profile; `wos-custody-hook-encoding.schema.json` deleted), runtime artifact migration ($defs `FactsTierRecord`/`MutationSource`/`VerificationLevel`/`CaseFileSnapshot`/`CapabilityInvocationRecord`/`ProvenanceOutcome` promoted into `wos-workflow.schema.json`; `wos-case-instance.schema.json` moved to `schemas/`; `wos-provenance-record.schema.json` renamed to `wos-provenance-log.schema.json`), tooling consolidation (`wos-tooling.schema.json` 2411 lines absorbing 5 sources via `$views`), fixture marker migration (91 fixtures rewritten — 35 root-level `$wosKernel`/`$wosWorkflowGovernance`/etc. → `$wosWorkflow` with embedded blocks + 56 inline_documents-nested; sidecar markers → `$wosDelivery`/`$wosOntologyAlignment`/`$wosTooling`/`$wosProvenanceLog`). Three new lint rules registered + lint logic implemented + 8 inline tests landed + promoted to `Tested` graduation: `WOS-AGENT-XREF-001` (T2; agent-typed actor ↔ `agents[].id` xref), `WOS-SIG-COVER-001` (T2; signature-gated transition coverage), `WOS-VER-LEVEL-001` (T1 warning; `fallbackChain` → `verificationLevel` hint); I-001 reanchored to kernel/spec.md §9.2. `schema_doc_zero_regression` gate updated with `EXCLUDED_SCHEMAS` for two in-flight schemas (`wos-workflow.schema.json` 160 inner-block leaves gated on PLN-0176..0207 spec absorption; `wos-delivery.schema.json` 1 leaf tracked follow-up). **Still deferred:** step 3a kernel spec absorption (blocked on D-8 §2/§6 disambiguation owner decision); step 7-8 Profiles+Companions deletion (depends on 3a — once spec absorption lands, source markdown + schema source files can be `git rm`); step 9 `COMPATIBILITY-MATRIX.md` rewrite to envelope-version model (`README.md` + `RELEASE-STREAMS.md` updated 2026-04-28 with claims-map); Python `tests/schemas/conftest.py` `MARKER_TO_SCHEMA` rewrite (currently still references the old 19-marker map; pytest passes today only because schema files at deleted-paths still exist on disk pending step 7-8 cleanup — the conftest rewrite + schema deletion land together). **Cross-stack new ADR doc references:** wos-spec `CLAUDE.md` `Six canonical kernel seams` line + `wos-workflow.schema.json` `$comment` block updated; wos-spec `Schema-structure section in CLAUDE.md` already aligned. — `wos-server` **Phase 1 sprint closed**: auth-tightening Track B (WS-002 + WS-008 + WS-009 + WS-057 + WS-058 + WS-083 const-generic `RequireRole<R: Role>`; WS-003 close-out — every mutator demands auth), WS-080 `AppRuntimeConfig` + `WOS_SIGNER` env wire-up, WS-082 typed `HoldServiceError` + Hold CRUD via typed `HoldService`, WS-052 session-table daily sweep, WS-034 policy resolve `GET` + assurance-chain validation surface, WS-011 + WS-012 task/event HTTP integration tests, WS-038 calibration + WS-041 semantic route mount. End-state architecture for the **reference server** lives in [`crates/wos-server/VISION.md`](crates/wos-server/VISION.md) (aligned 2026-04-27 with repo-root [`VISION.md`](../VISION.md): zero-trust data+workflow posture, ports/adapters, **`wos-server-eventstore-embedded`** sibling under `wos-spec/crates/` per §VI, `DurableRuntime` + Restate default, workload identity for inter-service auth, Federal-posture `processing-tee` gate, `CRYPTO_OWNER` / package fences, per-class encryption per [ADR-0074](../thoughts/adr/0074-formspec-native-field-level-transparency.md)). **Stack reconciliation (open):** root `VISION.md` §VIII rejects a durable **two-store** split (`Storage` + separate `AuditSink`); current Phase 4 plan in [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) still sequences WS-090 as an audit sink port until Trellis composition closes — converge plans toward a **single Trellis-backed `EventStore`** as the operational+integrity port without dual-write. Merged `wos-workflow` schema sketch + product-tier examples landed per [ADR 0076](../thoughts/adr/0076-product-tier-consolidation.md); [ADR 0079](../thoughts/adr/0079-formspec-native-intake-handoff-emission.md) (Formspec native `IntakeHandoff` emission) and [ADR 0080](../thoughts/adr/0080-governed-output-commit-pipeline.md) (governed output-commit pipeline) authored. Trellis landed **Wave 16** (store-postgres production hardening + memory-store transaction parity), **Wave 17** (HPKE ephemeral-uniqueness lint `TR-CORE-033` + `KeyEntry` taxonomy per Trellis ADR 0006), **Wave 18** (HPKE crate hardening + ADR 0009 promotion; §A.5.2 reason-code corpus reconciliation + R19 parity lint; `trellis-store-postgres` review follow-ups including IPv6-URI host-extraction bugfix + future-version migration guard; **ADR 0005 Stage 1 spec deltas** — Companion §20.6 + OC-141..146 + Core §6.7/§6.9/§19 step 6b/§19.1 enum + matrix TR-OP-105..109/113/114), **Wave 19** (AEAD nonce determinism Core §9.4 + §17 amendment per parent **PLN-0383** — signature-stack-relevant silent retry-determinism class on signed events), **Wave 20** (ADR 0008 interop sidecar reservation lock-off; empty crates `trellis-interop-{scitt,vc,c2pa,did}` + cargo-deny config; Trellis-side closure of parent **PLN-0313** reservation), and **Wave 21 closed 2026-04-28** (ADR 0005 Stages 2-5 fully landed — Rust 10-step verifier + Python parity + positive vectors `append/023..027` + tamper `017..019` + export bundle + CLI + Companion §27 + matrix promotion all committed; slot collision resolved by renaming `export/009-intake-handoffs-public-create-empty-outputs` → `export/013-...` preserving item #4 cert-of-completion's slot 010 reservation per ADR 0007 *Fixture plan*; **closes parent PLN-0312 foundational crypto execution bundle entirely**) — see `trellis/COMPLETED.md` Wave 18-21 entries. **Parent MVP foundation cluster decided 2026-04-27** ([parent PLANNING.md](../PLANNING.md) PLN-0331..0367) coordinates the wos-server adapter cluster as execution home; PLN-0368 (cross-submodule Cargo path-dep) and PLN-0369 (per-tenant DB scaling — VISION commitment vs schema-per-tenant) are open parent-stack architectural decisions gating WS-020 (eventstore-postgres) + multi-tenant deployment. **Parent stack closure cluster decided 2026-04-27** (PLN-0379..0398, synthesis-merge of the 2026-04-27 architecture brainstorm; archived at [`/thoughts/archive/specs/2026-04-27-architecture-synthesis-corrected.md`](../thoughts/archive/specs/2026-04-27-architecture-synthesis-corrected.md)) carries WOS-touching rows: **PLN-0380** (signature.md §1.3 scope reopen + signing-intent URI registry + signer-authority claim shape), **PLN-0381** (identity attestation stack ADR — supersedes PLN-0310), **PLN-0382** (external recipient lifecycle — `wos.governance.access-granted` / `access-revoked`), **PLN-0384** (highest leverage; ratifies `wos-event-types.md` taxonomy gating `wos.signing.*` / `wos.identity.*` / `wos.governance.access-*` namespace citations), **PLN-0385** (`custody-hook-encoding.md` companion v1.0 — the four-field append wire surface), **PLN-0387** (`wos-server-eventstore-embedded` sibling adapter — WS-095 execution home), **PLN-0388** (`agent-sdk` peer crate + CRYPTO_OWNER fence extending PLN-0340), **PLN-0398** (DocuSign 100% admin surface — Trigger; pulled back into 1.0 scope per VISION §X). 2026-04-24 — `wos-server`: `Storage` port documents `LIST_INSTANCES_PAGE_SIZE_MAX` (dependency-inversion; SQLite is one adapter), inbound CloudEvent dedupe via `INSERT OR IGNORE`, governance delegations errors propagate, timer task paginates all instance pages; follow-ups listed as **#62–#64** below. 2026-04-23 — WOS-T4 Signature Profile runtime/lint/conformance + SIG-* fixtures green; WOS-T1 ADR-0061 `custodyHook` four-field append + receipt stamping closed in code/schemas; semi-formal review follow-ups: `ProvenanceRecord.id` required on serde (no silent mint), `typeid::tenant_from_env_value` + env-free unit tests, `FixtureFormspecProcessor` dead_code reservation. Parent Formspec canonical signed-response / `authoredSignatures` fields, signed-response fixture, WOS-facing mapping seed, and server-side revalidation preservation landed. Trellis landed `append/019`, export `006` + verify/tamper `014`, Core extension `trellis.export.signature-affirmations.v1`, and `trellis-verify` catalog checks (202-04-22). **Landed 2026-04-23:** distinct host-side intake-acceptance seam in `wos-runtime` (`IntakeAcceptanceAdapter` / registry / durable command) plus first-class host `IntakeAcceptancePolicy` (default `NoopIntakeAcceptancePolicy`) and Formspec reference interpreter/finalizer in `wos-formspec-binding`. **Landed later 2026-04-23:** runtime-owned default intake library (`AutoCreatePublicIntakePolicy`, `ManualReviewIntakePolicy`, `PublicIntakeDisabledPolicy`), durable `accept_intake_handoff(...)` persistence/idempotency keyed by binding + intake id, canonical `intakeAccepted|Rejected|Deferred` provenance emission, and case-attach/create application in `wos-runtime`. **Still open:** shared cross-repo fixture bundle wiring (parent design at [`../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md`](../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md)), Studio authoring/validation UX, Trellis human certificate-of-completion composition per [Trellis ADR 0007](../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) ([T4-TODO.md](T4-TODO.md) T4-10/T4-11/T4-12). **Provenance emission completeness audit landed 2026-04-28** ([`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md)): 99 `ProvenanceKind` variants at audit time (100 at HEAD post-Session 14) checked against runtime + binding + core emission; 98/99 emit, 1/99 (`TaskSkipped`) is dead — confirmed at backlog **#66e**. **Gap 1 typed-path closed in-session** ([COMPLETED.md](COMPLETED.md) Session 14): `ProvenanceKind::CapabilityInvocation` variant + audit-tier=Facts + `ProvenanceRecord::capability_invocation(...)` constructor enforcing `outcome: "preconditionNotSatisfied"` when blocked + 5 unit tests (including serde round-trip); schema MUST now fulfillable from typed Rust path. Reviewed by `formspec-specs:wos-scout` semi-formal-code-review 2026-04-28 (verdict *Land it*; F1 doc grammar+rationale and F3 item 1 round-trip applied; F2 ergonomic variant + F3 items 2-3 exporter assertion + JSON fixture carried into Do-next #2). Re-filed Do-next **#2** as AI-runtime emission wiring (`[6 / 5 / 3]` — 18, gated on AI-runtime invocation seam design — `Capability.preconditions` is declarable but no runtime path evaluates it today). Behavioral / governance **#67** still open (Gap 3 — generic `ConfigurationWarning` kind for the 4 unresolvable-ref MUSTs at `drift-monitor.md:77`, `workflow-governance.md:154`, `notification-template.md:199,222`). Hygiene **#68** filed for the meta-finding: no CI gate enforces schema `recordKind:` literal ↔ `ProvenanceKind` variant parity; mirror the ADR 0077 `check-canonical-seams.py` pattern. **Custody / Assurance governance prose gap (former Do-next #2)** also closed in-session — 2026-04-24 audit verdict #17 was stale: §2.9 + §4.9 + §7.15 prose + schema all landed 2026-04-15 (commits `2f50812`, `2a5d89b`, `5d86839`); legal-sufficiency cross-ref to Assurance §6 at `workflow-governance.md:46`; Invariant 6 dedup completed in Plan 3 with `assurance.md` §4.4 declaring normative home and Trellis matrix removing legacy ULCR-112. **Cross-stack-scout validation pass 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 15) — 8 candidate "low-hanging" tickets validated against HEAD: surfaced 95→100 variant-count drift (corrected across 6 sites); rescoped #68 to all schemas + bidirectional; rescoped §4.1 to 2 schemas (third was deleted in ADR 0076); retired Kernel-Basic LoadBearing (profile doesn't exist as artifact) and Stack-level ADR cross-check lint (template convention change required first); split out exporter-parity carry-over as Hygiene **#69**. Validated execution order: **#69** (catches exporter bugs) → **#68** (locks the door) → **§4.1** (mechanical) → **#67** (proven pattern) → **#65d** + ADR 0073 terminology (doc cleanups).

## Snapshot

| Health | Value |
|---|---|
| Specs / schemas | 41 spec/docs under `specs/` · 27 schemas · 0 SCHEMA-DOC-001 violations |
| Crates | 6 production (`wos-core`, `wos-lint`, `wos-conformance`, `wos-runtime`, `wos-formspec-binding`, `wos-export`) · 6 MVP (`wos-authoring`, `wos-mcp`, `wos-synth-core/-mock/-anthropic/-cli`) · 1 spike (`wos-synth-spike`, keep-with-deletion-horizon) |
| Tests | Latest targeted gates: `cargo check --workspace` green; `cargo test -p wos-core --lib` green; `cargo test -p wos-runtime --lib` green; `cargo test -p wos-lint` green; `cargo test -p wos-conformance --test signature_profile` 13 green; `pytest tests/schemas -q` 255 passed / 12 skipped / 1 xfailed |
| Lint matrix | 116 rules (35 T1 · 72 T2 · 9 T3 · 1 LoadBearing · 11 Tested · 104 Draft) |
| CI gates | `schema_doc_zero_regression` · `every_promoted_*_rule_has_executable_or_annotated_evidence` · `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures` · `discover_and_report_promotion_candidates` ratchet |

**Navigation:** [**User profile** (read first)](../.claude/user_profile.md) · [**Stack vision**](../VISION.md) (canonical; WOS §X) · [`wos-spec/CLAUDE.md`](CLAUDE.md) · [LINT-MATRIX](LINT-MATRIX.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [Runtime Companion](specs/companions/runtime.md) · [ADRs](../thoughts/adr/) · [Plans](thoughts/plans/) · [Parallel-agent dispatch discipline](thoughts/practices/2026-04-17-parallel-agent-dispatch.md)

### Continuation — `trellis-store-postgres` ↔ wos-server `EventStore`

Trellis already ships the **Postgres canonical ledger** in [`trellis/crates/trellis-store-postgres/`](../trellis/crates/trellis-store-postgres/) (`LedgerStore`, versioned migrations, TLS, pool, **`append_event_in_tx`** for same-transaction writes with downstream `projections` — see that crate’s module docs). **No duplicate store implementation belongs in `wos-spec/`** except the thin **`wos-server-eventstore-postgres`** adapter (PLN-0332) that **composes** this crate + in-database projections per [`crates/wos-server/VISION.md`](crates/wos-server/VISION.md) §VI.

**Gating:** parent [`PLANNING.md`](../PLANNING.md) **PLN-0368** (cross-submodule Cargo path-dep for `wos-spec` → `trellis`) unblocks the adapter crate; **PLN-0369** remains the tenancy/schema topology decision. **`wos-server-eventstore-embedded`** (WS-095, [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md)) is a **sibling for tests/dev**, typically composing **`trellis-store-memory`** with identical envelope invariants — not a substitute for `trellis-store-postgres` in production.

**After** `eventstore-postgres` demonstrates one-transaction canonical + projection writes, **revisit WS-090** (`AuditSink`) and the interim `Storage` + audit split — converge toward a **single composed `EventStore`** path per root [`VISION.md`](../VISION.md) §VIII (details in `wos-server` TODO WS-090 / WS-093 stack notes). Cross-stack index: [`TODO-STACK.md`](../TODO-STACK.md).

---

## Do next

Pick from the top. Each item has a gate (what unblocks it) and a plan or ADR.

**Scoring note.** Per [`user_profile.md`](../.claude/user_profile.md) economic model: dev/time is free, architectural drift is expensive. Ordering uses **`Imp × Debt`**; Cx is preserved as a scheduling dimension but does not change priority. Debt values trend **up** between sessions on pre-1.0 work. Score notation: `[Imp / Cx / Debt]`; the number in parentheses is `Imp × Debt`.

1. **Signature Profile workflow semantics** `[7 / 5 / 5]` (**35**) — **WOS-T4 ACTIVE (cross-repo closeout).** Workflow-tier slice of the **DocuSign 100% parity bar** per VISION §X (parent PLN-0370 reframe holds the marketing line until full parity lands; PLN-0355 ESIGN/UETA gate Trigger): signer roles via `actorExtension`, sequential/parallel/routed/free-for-all flows, intent capture, identity binding, signer-authentication policy schema, reminders, expiry, decline, void, reassignment, and `SignatureAffirmation` provenance. Cryptographic integrity + certificate-of-completion live in Trellis; WOS only emits the semantic evidence record. **Path back to original DocuSign-100% framing** lands via parent **PLN-0380** (`signature.md` §1.3 scope reopen + signing-intent URI registry + signer-authority claim shape distinct from §2.6 authentication-method) + parent **PLN-0398** (Trigger — administrative surface: template libraries, bulk-send, send-for-signature dashboards, signer status views, reminder cadence configuration, audit history view). Trellis user-content Attestation primitive at parent **PLN-0379** (Trellis ADR 0010) composes for byte-level signing-intent URI carriage. **Execution plan:** [T4-TODO.md](T4-TODO.md). **Landed 2026-04-22 (WOS center):** [ADR-0062](thoughts/adr/0062-signature-profile-workflow-semantics.md), [Signature Profile spec](specs/profiles/signature.md), [Signature Profile schema](schemas/profiles/wos-signature-profile.schema.json), schema fixtures/tests, Studio generated type binding, `ProvenanceKind::SignatureAffirmation`, schema-constrained `SignatureAffirmation` payload, Rust constructor/helper, Facts-tier classification, custody append inclusion, SIG-001..SIG-012 lint, runtime profile loading, signing task evidence validation, `SignatureAffirmation` emission, sequential/parallel/routed/free-for-all/witness/notary/decline/void/reassignment/expiry semantics, and 13 SIG-* conformance tests. **Cross-repo landed 2026-04-22:** parent Formspec canonical signed-response / `authoredSignatures` fields, signed-response fixture, WOS-facing mapping seed, server-side revalidation preservation, and Trellis `append/019` + export `006` / verify `014` / tamper `014` + Core verifier extension for `062-signature-affirmations.cbor`. **Next T4 slice:** shared fixture bundle end-to-end (design doc at parent [`thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md`](../thoughts/specs/2026-04-24-shared-cross-seam-fixture-bundle-design.md)), Studio authoring/validation UX, Trellis human certificate-of-completion composition per [Trellis ADR 0007](../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) (accepted 2026-04-24) ([T4-TODO.md](T4-TODO.md) T4-10–T4-12).

   **WOS-T4 -COMPLETE- criteria:** Formspec captures signing/consent evidence; WOS routes signing through lifecycle/governance semantics; WOS emits `SignatureAffirmation`; Trellis accepts and anchors the record through `custodyHook`; conformance proves sequential, parallel, routed, free-for-all, expiry, decline, reassignment, witness/counter-signature, notary/in-person authentication, missing-consent rejection, and custody append inclusion.
2. **AI-runtime capability-precondition emission wiring** `[6 / 5 / 3]` (**18**) — typed Rust path landed 2026-04-28: `ProvenanceKind::CapabilityInvocation` variant, audit-tier=Facts (exhaustive `From<K>` arm), `ProvenanceRecord::capability_invocation(input)` constructor enforcing the `outcome: "preconditionNotSatisfied"` invariant when blocked, 5 unit tests (blocked-sets-outcome, permitted-omits-outcome, context-key-collision-protection, classifies-as-facts, serde-round-trip); `cargo test -p wos-core --lib` 66 passed; `python3 -m pytest tests/schemas/test_capability_invocation_record.py` 6 passed (schema if/then guard already covered). Reviewed by `formspec-specs:wos-scout` 2026-04-28 (semi-formal-code-review, verdict *Land it*; review applied F1 doc-grammar+rationale and F3 item 1 round-trip test). **Still open and grouped under this Do-next:** (a) runtime emission site — AI §3.3.1 step 1-3 specifies precondition evaluation but no runtime path actually evaluates `Capability.preconditions` (`crates/wos-core/src/model/ai.rs:189`); the field is declarable but not fired. Wiring requires AI-runtime architecture decisions (when does the processor invoke a capability? which seam fires? how does the fallback chain compose?). (b) JSON conformance fixture pair (blocked + permitted) under `fixtures/conformance/` — meaningful only once emitter exists, so co-lands with (a) (review F3 item 3). (c) Ergonomic constructor variant (`impl IntoIterator<Item=(String, Value)>` for `context`) once call-site count justifies (review F2). The exporter-parity carry-over (review F3 item 2) was confirmed independent of AI-runtime wiring per cross-stack-scout 2026-04-28; split out as Hygiene **#69**. **Gate: AI-runtime invocation seam design.** Discovered 2026-04-28 audit ([`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md) Gap 1).
3. **Actor authorization shape (`AuthorizationAttestation`)** `[7 / 4 / 5]` (**35**) — stack contract per [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) D-2. **Gate: ADR 0066 accepted.** Checklist: [ADR 0066 — execution checklist](#adr-0066-exec-checklist) (attestation record + policy bindings).
4. **ADR 0066 implementation — amendment / supersession / rescission / correction** `[7 / 6 / 5]` (**35**) — six provenance record kinds, `caseRelationship.type = supersedes`, Workflow Governance policy sections, exporter coverage. **Gate: ADR 0066 accepted.** Full WOS-scoped breakdown: [ADR 0066 — execution checklist](#adr-0066-exec-checklist).
5. **ADR 0067 implementation — statutory clocks** `[7 / 5 / 5]` (**35**) — `clockStarted` / `clockResolved` provenance kinds, `Clock` `$def`, four-kind runtime wiring, `#40` / `#51` composition, export mappings, conformance. **Gate: ADR 0067 accepted.** Execution checklist: [ADR 0067 — execution checklist](#adr-0067-exec-checklist).

### Agent task extract (from this file)

| Task ID | Tracks | Deliverable | Depends on |
|---------|--------|-------------|------------|
| **WOS-T2** | Do next **#2** | [ADR-0060](thoughts/adr/0060-cross-reference-naming-ref-key-id.md) accepted + governance schema/runtime/lint/test sweep | **-COMPLETE-** — Workflow Governance taxonomy closed; G-063/G-066 enforce remaining key/id resolution |
| **WOS-T3** | Do next **#3** | `DurableRuntime` trait + Temporal/Restate spike + tenant-scope notes | **-COMPLETE-** — Restate selected; Temporal deferred; tenant-scope contract recorded |
| **WOS-T4** | Do next **#1** | Signature Profile spec slice + schemas + runtime/conformance | **ACTIVE** — WOS + parent Formspec + Trellis machine-verifiable append/export/verify landed 2026-04-22; shared bundle + Studio + COC presentation remain |
| **WOS-B1** | Backlog | §4.5 structural merges (1 vs 3 PRs) | Owner packaging decision |
| **WOS-B2** | Backlog | Kernel-Basic profile **LoadBearing** declaration + lint-matrix wire | None |
| **WOS-B3** | Backlog | [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) authoring-stack closure — MCP↔synth `ToolContext` seam, spike/Q-V0 follow-ups, `wos-bench`, conformance/lint API hygiene | See **ADR 0065 — authoring stack closure** in Backlog |
| **WOS-B4** | Backlog | Runtime Companion **§15** / Phase 11 — `wos-runtime` parity vs published MUSTs (`#66`–`#66g`), PARITY drift (**WS-074**), HTTP §15 fixtures (**WS-075**); ADR 0066 server slice **WS-072**; ADR 0067 clock prove-out **WS-073** | [§ Runtime Companion §15 / Phase 11](#runtime-companion--15--phase-11--reference-wos-runtime-parity) · [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-011**, **WS-072–WS-073**, **WS-074–WS-075** |

*Falling off Do next at Imp × Debt < 30:* §4.5 structural merges (owner decision needed), **ADR 0065 authoring-stack closure** (Backlog — MCP/synth seam + spike follow-ups + `wos-bench`; consolidated 2026-04-24), §4.4 release-trains Tasks 4-5 (15). All live in Backlog.

### ADR 0066 — execution checklist (WOS center) {#adr-0066-exec-checklist}

**Gate:** [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) accepted (exit *Proposed*). Formspec Respondent Ledger work and Trellis vectors/verifier/export stay owned in parent [`TODO-STACK.md`](../TODO-STACK.md) and [`../trellis/TODO.md`](../trellis/TODO.md) item **17**; this block is the **WOS spec + schema + runtime + export** slice.

1. **Kernel / provenance**
   - [ ] Add six `ProvenanceKind` variants + `wos-provenance-record.schema.json` `recordKind` registrations: `correctionAuthorized`, `amendmentAuthorized`, `determinationAmended`, `rescissionAuthorized`, `determinationRescinded`, `authorizationAttestation` (camelCase per existing serde/schema convention — align names to ADR D-1/D-2 at authoring time).
   - [ ] Payload `$defs` / `allOf` guards: prior-event hashes (`supersedes_event_hash` on amended determination, etc.), mode-specific `reason`, pointer to authorizing provenance, `EvidenceReference` arrays, `AmendmentAuthorized` + `AuthorizationAttestation` predicate `"amendment-authority"` where D-1 requires it.
   - [ ] Tier map: **Facts** for `authorizationAttestation`; **Narrative** (or spec-chosen tier) for the five mode-specific kinds — match ADR Consequences §Negative and Runtime Companion audit-layer rules.
   - [ ] `wos-core` `audit_layer_for_kind` / conformance tests: every new kind appears in the same matrices as existing kinds (see `crates/wos-core/src/provenance/tests.rs` Finding 3 pattern).
2. **Workflow Governance**
   - [ ] Normative sidecar sections: `amendmentPolicy`, `rescissionPolicy`, and parallel sections for correction / supersession authority, each binding an `AppealMechanism`-shaped gate; impact-level assurance floor (rights-impacting → authorizing actor `Assurance ≥ high`) per D-2.
   - [ ] Lint rules + fixtures: misconfigured policy fails at validate time where MUST-level.
3. **`caseRelationship.type = supersedes`**
   - [ ] Schema enum already includes `supersedes` — close the loop: kernel spec prose, K-048 / companion examples, fixtures that emit a superseding case instance, runtime validation of `targetCase` URI shape.
4. **Runtime + binding**
   - [ ] `wos-runtime` (and `wos-formspec-binding` where intake/custody intersects): emit the new records on governed transitions; ensure intake paths never silently mutate prior responses when a correction lineage exists (ADR Context).
5. **Export**
   - [ ] `wos-export`: distinct PROV-O / OCEL / XES event types for all six kinds (ADR implementation plan).
6. **Conformance + docs**
   - [ ] `wos-conformance` fixtures per kind; update `WOS-IMPLEMENTATION-STATUS.md` / matrix rows as applicable.

**Related:** statutory “may this still be amended?” is [ADR 0067](../thoughts/adr/0067-stack-statutory-clocks.md) (separate acceptance). D-5 only requires composition, not 0067 implementation inside 0066.

### ADR 0067 — execution checklist (WOS center) {#adr-0067-exec-checklist}

**Gate:** [ADR 0067](../thoughts/adr/0067-stack-statutory-clocks.md) accepted (exit *Proposed*). Trellis `open-clocks.json`, verifier advisories, and append vectors **014–017** stay in parent [`TODO-STACK.md`](../TODO-STACK.md) and [`../trellis/TODO.md`](../trellis/TODO.md) item **18**; Formspec **StatuteClock** origination on respondent acts stays in parent [`TODO-STACK.md`](../TODO-STACK.md); reference-server prove-out is [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-073**.

1. **Kernel / provenance**
   - [ ] Add `ProvenanceKind` variants + `schemas/kernel/wos-provenance-record.schema.json` `recordKind` branches: `clockStarted`, `clockResolved` (camelCase per existing serde/schema convention — align literal names and payload keys to ADR D-1 at authoring time).
   - [ ] Payload `$defs` / `allOf`: **ClockStarted** — `clockId`, `clockKind`, `originEventHash`, `duration` (ISO 8601 duration), `calendarRef`, `statuteReference`, `computedDeadline` (RFC 3339, materialized once); **ClockResolved** — `clockId`, `originClockHash`, `resolution` (`satisfied` \| `elapsed` \| `paused` \| `cancelled`), `resolvingEventHash`, `resolvedAt`. `clockKind` open enum (`AppealClock`, `ProcessingSLA`, `GrantExpiry`, `StatuteClock`, `x-*`).
   - [ ] Facts-tier classification; `audit_layer_for_kind` + `wos-core` conformance matrices (`crates/wos-core/src/provenance/tests.rs` patterns).
2. **Runtime emission (ADR D-2 — four kinds)**
   - [ ] **AppealClock** — adverse-decision / deterministic notice path (composes with Gov §4.1 #2).
   - [ ] **ProcessingSLA** — intake accepted / intake-complete workflow event.
   - [ ] **GrantExpiry** — benefit award issued transition.
   - [ ] **StatuteClock** — WOS-owned triggers only on this slice; Formspec-originated statute clocks use the respondent-ledger path (parent repo).
   - [ ] **Pause / resume (D-4):** `clockResolved` with `resolution: paused` plus a new `clockStarted` carrying **residual** duration — no separate `ClockPaused` record kind.
3. **Task SLA (#40)**
   - [ ] Reference the clock contract where Task SLA durations overlap rights-impacting deadlines (ADR implementation plan).
4. **#51 statutory deadline chains**
   - [ ] Compose with §7.1 business calendars + typed kernel events; revisit trigger-gate once the center contract ships.
5. **`wos-export`**
   - [ ] Distinct PROV-O / OCEL / XES event types or annotations for `clockStarted` / `clockResolved` (parallel to other Facts-tier kinds).
6. **Conformance + normative closure**
   - [ ] `wos-conformance` fixtures (start, satisfied, elapsed, paused segment); kernel / companion prose for MUST-level behavior.
   - [ ] Encode or explicitly defer ADR §Open questions: envelope timestamp granularity; post-hoc synthetic `elapsed` vs leave-open + verifier-only; multi-jurisdictional independent emits vs single jurisdiction.

---

## Backlog

### Envelope-stack enablement (§4.7)

The DocuSign-class signature workflow is expressible with WOS's existing nine host-interface seams (Runtime §12) — `ProvenanceSigner`, `ReportRenderer`, `ContractValidator`, `AccessControl`, `ExternalService` — plus Formspec's Respondent Ledger (S13) for cryptographic checkpoint. The spec gaps below are the **surface concepts** that the DI composition can't fill on its own: instance-level envelope status values, a cross-system CloudEvent type catalog, and canonical reference patterns so integrators don't diverge.

Existing backlog items that serve envelope-stack composition once they land: **#2** (dual-form adverse-decision notice — §4.1 critical path) feeds the ReportRenderer seam for COC + notice rendering; **#20** (typed event meta-vocabulary — §4.1) normalises the kernel-internal event taxonomy that the §4.7 CloudEvent catalog mirrors to external systems; **#30** (task lifecycle completion) + **#40** (Task SLA runtime) cover task-level decline and reminders; **#38** (G-064 Assertion Library resolution lint) gates cross-doc `assertionRef` misconfigurations that would otherwise silently bypass signer-input validation; **#43** (assurance × impact composition) hosts the signature-class ↔ assurance-level binding; **#3** tenant-scope sub-question unblocks SaaS multi-tenancy. §4.7 annotations on each item live inline in *Behavioral / governance* below.

- [ ] **#58 Envelope (instance-level) status extension** `[Imp 7 / Cx 3 / Debt 5]` — Extend `CaseInstance.status` (or adjacent schema surface) with first-class `declined | voided | expired` discriminators, each carrying required metadata (`declineReason`, `voidedBy`, `voidedAt`, `expiredAt`). Current status taxonomy (`active | suspended | migrating | completed | terminated`) can't distinguish "envelope signer declined" from "processor terminated the instance" — a material legal distinction. Companions to #30: #30 is task-level, #58 is instance-level. **Debt 5** because every envelope shipped without this forces integrators to encode the distinction in case_state, creating diverging conventions that later have to be migrated.
- [ ] **#59 CloudEvent envelope-flow type catalog** `[Imp 6 / Cx 3 / Debt 4]` — Normative event-type catalog in `integration.md` for cross-system envelope coordination: `envelopeCreated`, `signerInvited`, `signerAuthenticated`, `signerSigned`, `signerDeclined`, `envelopeCompleted`, `envelopeVoided`, `envelopeExpired`, `reminderDue`. Distinct from #20 (which normalises **kernel-internal** event vocabulary per transition). #59 is the **cross-system wire contract** that identity providers, email adapters, and webhook consumers speak. Without it, every WOS-based signature stack defines its own event names and the integration ecosystem fragments.
- [ ] **#60 Envelope reference fixtures** `[Imp 5 / Cx 3 / Debt 3]` — Three to five canonical kernel documents under `fixtures/kernel/envelope-*.json` demonstrating the composition patterns: `envelope-2signer-sequential.json`, `envelope-parallel-witness.json`, `envelope-decline-reroute.json`, `envelope-with-approver.json`, `envelope-reminder-expire.json`. Plus matching conformance fixtures exercising the full lifecycle (create → invite → sign → complete; create → invite → decline → void). **Fixture-only work** — no new schema surface, but critical for lock-in: locked patterns prevent divergent re-inventions across vendors building on WOS. Depends on #20 typed events and #30 task-lifecycle for the decline fixture.
- [ ] **#61 Separation-of-duties conformance fixture batch** `[Imp 5 / Cx 2 / Debt 3]` — Two to three fixtures under `fixtures/conformance/` exercising the AccessControl seam's separation-of-duties rejection path: (1) agent attempts to review its own output → rejected; (2) delegated human attempts to re-review as the original author → rejected; (3) separation-of-duties bypass with authority override → recorded as provenance with `OverrideRecord`. Pairs with #23 OverrideRecord schema landing. Shape of the AccessControl seam is already in wos-core traits; what's missing is the conformance contract that reference processors MUST reject these attempts.

**Not adding to TODO (handled by DI, not spec):**

- Attestation / signing ceremony — `ProvenanceSigner` seam (Runtime §12.6) already exists; Formspec Respondent Ledger S13 provides the primitive. Implementation is "wire the seam," not "spec the surface."
- Explanation assembly endpoint — `ReportRenderer` seam (Runtime §12.7) already exists. Deterministic algorithm lands via #2.
- Separation-of-duties enforcement — `AccessControl` seam (Runtime §12.5) already exists. Implementation is policy-source composition, not a new spec concept.
- Integration correlation tokens — NB.3 CloudEvents bindings already ship subject correlation `{instanceId}:{bindingId}:{invocationId}`. Server-side surface gap, not spec gap.

### Structural merges (§4.5) — blocked on owner decision

Three merges ratified by the 2026-04-20 [sidecar audit](thoughts/reviews/2026-04-20-sidecar-contract-audit.md). **Gate: user decision — one PR or three?** `VISION.md` / practice recommendation: three discrete PRs for review hygiene; audit recommended one. Either is acceptable; owner picks.

- **Assertion Library → Workflow Governance** `[4 / 2 / 5]` (**20**) — `AssertionUse` seam already landed session 8; merge is mechanical file-move.
- **Verification Report → Advanced Governance** `[3 / 2 / 2]` (**6**) — it's a processor **output**, miscategorized as a sidecar.
- **Due Process Config partial merge → Workflow Governance** `[5 / 3 / 4]` (**20**) — residual sections duplicate Governance §3.1/§3.5.

Companion decisions from session-9 agent dispatch: M-1 Drift Monitor + Agent Config merge remains BLOCKED (standalone fixture); M-2 Notification Template + Due Process merge remains REJECTED (categories don't align).

### Release + benchmarking — ready, lower priority

- **§4.4 Release trains Tasks 4-5** `[5 / 4 / 3]` (**15**) — Changesets tooling + GitHub Actions release workflow. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-release-trains.md). Tasks 1-3 landed session 8.
- **§5.5 `wos-bench` synthesis benchmark** `[6 / 5 / 3]` (**18**) — Claim A falsification harness; pairs with [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) Q6 / synth split. Plan: [2026-04-16](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Spike open questions: [2026-04-20](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md#open-questions) (Q-V0-1..4 need **live** Anthropic runs; update that doc with numbers). **Sub-deliverables:** scaffold `crates/wos-bench` (`wos-synth-core` + `wos-synth-mock`, optional Anthropic flag); problem statements + `benchmarks/runs/<date>-<model>/results.json`; rubric library + CLI; `BENCHMARK.md` leaderboard + methodology; scheduled/manual CI with secrets; pick **inline `ConformanceFixture` wrapper vs** upstream `wos_conformance::smoke_test_document`-style API (spike Option B — reduces duplication with synth-core / spike).

### ADR 0065 — authoring stack closure (MCP / synth / spike)

**Anchors:** [ADR 0065](thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md) · MCP plan [2026-04-17](thoughts/plans/2026-04-17-wos-mcp-crate.md) · Synth plan [2026-04-16](thoughts/plans/2026-04-16-wos-synth-crate.md) · Spike retrospective [2026-04-20](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md). Plan markdown checkboxes in those files are **stale vs `main`** in places; this subsection is the working backlog until checkboxes are rebased.

**Production seam (ADR D-3; MCP plan completion §2)** `[7 / 5 / 4]` (**28**)

- [ ] **#65a `ToolContext` via shared MCP handlers** — Implement a **second** `ToolContext` implementation whose lint/conformance behavior matches `wos_mcp` tools (`wos_lint`, `wos_run_conformance` code paths), not a parallel copy in `wos-synth-core` only. Needs a small adapter design: synth loop holds **document JSON**; `wos_mcp::dispatch` expects **`ProjectRegistry` + `project_id`**. Options: implicit scratch project per session, or a thin `wos_mcp` (or adjacent) type that implements `ToolContext` by delegating to existing `tools::*` internals.
- [ ] **#65b `wos-synth-cli` default wiring** — Switch CLI from **`DirectToolContext`** to the MCP-aligned **`ToolContext`** from #65a (ADR: production wiring injects MCP-backed dispatch, not stopgap-only).
- [ ] **#65c Optional purity pass** — Once #65a/#65b are real, consider **removing direct `wos-lint` / `wos-conformance` dependencies from `wos-synth-core`** so the loop crate stays provider- and lint-free at the crate edge (ADR D-2/D-3 intent). Only if no in-crate stopgap remains required for tests.

**MCP transport + docs** `[5 / 4 / 3]` (**15**)

- [x] **#65d `crates/wos-mcp/NOTES.md`** — **CLOSED 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 16). NOTES.md written; extracts hand-rolled JSON-RPC vs `rust-mcp-sdk` rationale + retraction (2026-04-18 feature analysis) + revisit triggers + ADR 0065 D-3 production seam pointer.
- [ ] **#65e SDK migration follow-up** — `wos-mcp` `Cargo.toml` **TODO**: revisit `rust-mcp-sdk` with `default-features = false` + minimal features vs current hand-rolled stdio (~transport swap).
- [ ] **#65f Real MCP client validation** — Exercise `wos-mcp` binary under a **real** MCP host (e.g. Claude Desktop). Plan addendum + spike both state: **v0 spike never touched MCP**; silence is not proof of dual-entry correctness.

**Spike + synth quality (research 2026-04-20; synth plan addendum)** `[6 / 4 / 4]` (**24**)

- [ ] **#65g Q-V0-1..4 live closure** — Run synth against Anthropic on the PO fixture; record iteration counts, dominant first-pass diagnostics, whether conformance repair fires, schema vs FEL vs governance fix mix. Update [spike findings](thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md) in place.
- [ ] **#65h Structured repair prompt** — `wos-synth-core`: feed **`rule_id`**, **`suggested_fix`**, **`spec_ref`** (structured block or JSON), not only `LintDiagnostic` `Display` text (cheapest prompt-engineering win per spike).
- [ ] **#65i Conformance document gate API** — Prefer upstream **`wos_conformance::smoke_test_document` (or equivalent)** over ad-hoc inline `ConformanceFixture` wrappers duplicated across spike, synth-core, and future bench (spike Option B).
- [ ] **#65j `wos-lint` parse error typing** — Replace **substring** matching for missing `$wos*` marker with a typed discriminant or stable error code (spike finding).
- [ ] **`wos-synth-spike` disposition** — Per spike doc: crate-level **`[spike — do not extend]`** on entrypoint, **port** inline-fixture + `classify_lint_error` coverage to `wos-synth-core` tests, **delete** spike on 2–3 month horizon. Snapshot row already notes keep-with-deletion.

**Synth provider + schema hygiene** `[5 / 3 / 3]` (**15**)

- [ ] **#65k Anthropic prompt caching** — `AnthropicPrompter` currently folds `CacheAnchor` data into the system prompt verbatim until the Anthropic SDK exposes cache control; wire real cache blocks when available (`crates/wos-synth-anthropic/src/lib.rs`).
- [ ] **#65l `SynthTrace` schema drift test** — `schemas/synth/wos-synth-trace.schema.json` exists; add/verify **schemars (or equivalent) round-trip validation test** per synth plan Task 7 if not already present.
- [ ] **#65m `ToolContext` trait discipline** — Synth plan addendum: **do not extend** `ToolContext` with speculative methods until a second implementation (#65a) proves the shape.

**Authoring plan vs shipped crate** `[4 / 3 / 2]` (**8**)

- [ ] **#65n Reconcile `2026-04-17-wos-authoring-crate.md`** — Plan file layout (`handlers/*.rs`, long checkbox ladder) **diverges** from shipped `raw.rs` / `command.rs` / `project.rs`. Either update the plan to match reality or extract a **gap list** (MCP tools ↔ `WosProject` helpers) so obsolete steps are not re-executed.

**Hygiene**

- [ ] **#65o Plan checkbox refresh** — After #65n, mark landed MCP/synth/authoring plan `- [x]` steps against `main` (or add banner: *checkboxes frozen — see `wos-spec/TODO.md` ADR 0065 section*).

### Behavioral / governance (1.0 scope under minutes-not-days)

Per repo-root [`VISION.md`](../VISION.md) operating frame: no "defer to 1.1" bucket on greenfield. These all land at 1.0 unless explicit architectural prerequisite unresolved.

**Stack contracts (ADRs 0066, 0067, and 0073):**

- **ADR 0073 implementation — case initiation and intake handoff** `[8 / 4 / 6]` (**48**) — WOS owns governed case identity, intake acceptance, and `case.created`; Formspec owns intake sessions, canonical responses, validation reports, and respondent-ledger evidence. **Landed:** typed `IntakeHandoff` parser/classifier in `wos-formspec-binding`; runtime intake-acceptance seam in `wos-runtime` (`IntakeAcceptanceAdapter`, `IntakeAcceptancePolicy`, durable `accept_intake_handoff(...)`); default intake policies; durable replay/persistence; canonical runtime `intakeAccepted|Rejected|Deferred` provenance emission; binding-owned `caseCreated` finalization for accepted create flows; Runtime Companion normative `acceptIntakeHandoff` algorithm; Kernel provenance prose + `schemas/kernel/wos-provenance-record.schema.json` registration for `CaseCreated`, `IntakeAccepted`, `IntakeRejected`, and `IntakeDeferred`; and Trellis-backed workflow-initiated attach / public-intake create export verification vectors. **What is still open:** one parent-owned shared fixture bundle that lets a cold reader consume the same canonical response / handoff artifacts from the top-level repo without spelunking into Trellis generators or submodule fixture IDs. **Done:** a WOS implementer can build intake acceptance from the normative docs/schema, emit the same provenance shape as the reference runtime, and verify both accepted handoff paths offline through Trellis. **Gate: none — ADR accepted 2026-04-23.**
- **Actor authorization shape (`AuthorizationAttestation`)** `[7 / 4 / 5]` (**35**) — stack contract per [ADR 0066](../thoughts/adr/0066-stack-amendment-and-supersession.md) D-2. Declares the Facts-tier record shape for a human act performed under a named policy; parallel to AI deontic constraints. Binds Workflow Governance policies (`amendmentPolicy`, `rescissionPolicy`, future `authorizationPolicy`) to the attestation record. Cheap — IAM is adapter; claim shape is center. **Gate: ADR 0066 accepted.** Tracked in [ADR 0066 — execution checklist](#adr-0066-exec-checklist) items 1–2.
- **Identity attestation shape — generalize beyond signatures** `[5 / 3 / 4]` (**20**) — WOS-T4 runtime emission now has `SignatureAffirmation.identityBinding` as the first concrete shape. This item generalizes that shape for reuse across non-signature evidence (reviewer-policy assurance refs, amendment-authority attestations, review-gate credentials). Lifts the Signature Profile per-field shape into a reusable `$def` after runtime emission proves the shape is sufficient. **Coordinates with parent PLN-0381** (synthesis-merge promoted identity attestation from Trigger to P0 center commitment 2026-04-27 per VISION §V open-contracts list; PLN-0310 closed-by-supersession). Cross-stack contract is the parent stack ADR (next free number); WOS-side action surfaces here as the reusable `$def` extraction. Coordinates with PLN-0380 signer-authority claim shape (capacity-to-bind, distinct from authentication-method) and PLN-0384 (`wos.identity.*` namespace ratification). **Gate: T4 runtime emission landed; parent stack ADR ratification pending.**
- **ADR 0066 implementation — amendment / supersession / rescission / correction** `[7 / 6 / 5]` (**35**) — umbrella item; decomposed into [ADR 0066 — execution checklist](#adr-0066-exec-checklist) above. **Gate: ADR 0066 accepted.**
- **ADR 0067 implementation — statutory clocks** `[7 / 5 / 5]` (**35**) — umbrella item; decomposed into [ADR 0067 — execution checklist](#adr-0067-exec-checklist) above. **Gate: ADR 0067 accepted.**

**Prior behavioral items:**

- **#35 Equity Config enforcement semantics** `[7 / 5 / 4]` (**28**) — processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime. Prerequisite: #36 resolved (stack vision: FEL + restricted-domain profile).
- **#36 Equity RemediationTrigger expression language** `[6 / 4 / 4]` (**24**) — FEL + restricted-domain profile per [`VISION.md`](../VISION.md) / WOS §X; no windowing escape hatch. Implementation.
- **#26a `AccessControl.canRead` enforcement semantics** `[6 / 3 / 4]` (**24**) — normative processor behavior on `canRead → false`: redact / null / raise / skip. Prerequisite to #26b.
- **#26b `caseFieldPolicy` schema** `[6 / 6 / 4]` (**24**) — per-field read/write scopes by actor role.
- **#43 Assurance × impact-level composition** `[6 / 5 / 4]` (**24**) — minimum Assurance floor per impact level (rights-impacting ≥ `high`; safety-impacting ≥ `high`; operational ≥ `standard`) per stack vision. **§4.7:** normative home for the signature-class ↔ assurance-level binding (ESIGN=L1, eIDAS-advanced=L3, QES=L4+QSCD); resolves Open Q15.
- **#24b + #25 joint design** `[#24b 7/6/4 · #25 6/7/6]` — Reasoning tier rule-firing trace + Catala-style defeasibility. Vision model: `workflow-governance` with `(sourceAuthority, priority)` lexicographic. After ADR.
- **#67 Configuration-warning provenance kind (typed path)** — **CLOSED 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 16). `ProvenanceKind::ConfigurationWarning` + audit-tier=Facts + `ConfigurationWarningInput<'a>` + `ProvenanceRecord::configuration_warning(input)` constructor + 4 unit tests landed; reserved `data.subject` literals `drift-monitor.policyRef` | `governance.continuationPolicyRef` | `notification-template.key` | `notification-template.render` (vendor extensions via `x-` prefix). **Open follow-up:** runtime emission wiring at the four MUST sites — `specs/ai/drift-monitor.md:77`, `specs/governance/workflow-governance.md:154`, `specs/sidecars/notification-template.md:199, 222`. Typed Rust path now ready; per-site wiring lands when the relevant runtime path is implemented.
- **#38 G-064 Assertion Library resolution lint** `[5 / 3 / 3]` (**15**) — implementation of the lint designed in session 8.
- **#40 Task SLA runtime implementation** — beyond the session-8 authoring surface; wire §10.3 runtime obligations. **§4.7:** the spec home for envelope reminders + expirations once runtime fires `slaDefinitions` / `warningThresholds` / `breachPolicy`.
- **Bulk Operations spec** (relocated from Future specs) — admin-portal-driven; parallel case instantiation + bulk state transitions.
- **#28 Claim-check artifact references** `[4 / 4 / 5]` (**20**) — typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }`.
- **#30 WS-HumanTask lifecycle completion** `[5 / 5 / 2]` (**10**) — task-level `Suspended`, distinct `Cancelled`, explicit `Return` with rework counter. **§4.7:** task-level decline / return is half of signer-decline semantics; pairs with #58 envelope-status for instance-level decline / void / expire.
- **#27 Cancellation regions** `[4 / 6 / 3]` (**12**) — YAWL-style named regions distinct from `cancellationPolicy` join policy.
- **#29b Milestone reactive transition firing (GSM-style)** `[6 / 5 / 2]` (**12**) — ships after #29a (landed session 4).
- **#3 Policy-based migration routing** `[5 / 6 / 2]` (**10**) — `migrationPolicy: grandfather | migrateAll | migrateByState | expression`. Tenant-scope sub-question finalizes with `DurableRuntime` tenant contract. **§4.7:** tenant-scope sub-question blocks multi-tenant envelope deployments (Open Q7 refers).

### Hygiene / refactors

Sequenced for module-bottleneck relief, not delayed by it.

- **#22 Crate split along tier boundaries** `[5 / 3 / 3]` (**15**) — `wos-core` → `wos-{kernel,governance,ai,advanced}`; `wos-runtime/runtime.rs` (still a large single module; ≈3.7k lines) split along action-kind dispatch; CI fence. (**#22a** provenance module split + `ProvenanceAuditTier` landed 2026-04-21 — see [COMPLETED.md](COMPLETED.md).)
- **#69 `wos-export` `camel_cases_all_record_kinds` test extension** — **CLOSED 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 16). Both `prov_o.rs` and `xes.rs` smoke tests rewritten to enumerate all 101 variants and assert serde-camelCase round-trip. No exporter bugs surfaced; generic dispatch is correct for all variants.

- **#68 Schema↔enum drift lint for ProvenanceKind (forward, all schemas)** — **CLOSED 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 16). `scripts/check-recordkind-parity.py` walks all `schemas/**/*.json`, finds every `$def` whose `properties.recordKind.const` (or single-element `enum`) pins a literal string (and `allOf[].if.properties.recordKind.const` if/then guards), asserts each maps to a `ProvenanceKind` variant. Wired into `.github/workflows/schema-regression.yml`. Reverse-direction (variants without schema binding) reports informationally; `--strict` upgrades to error. **Open follow-up:** reverse-direction enforcement once per-kind schema $defs proliferate.

### Reference server (`crates/wos-server`)

Reference-server **crate DAG, ports, and adapter sequencing** are in [`crates/wos-server/VISION.md`](crates/wos-server/VISION.md) (defers stack-wide Q&A to repo-root [`VISION.md`](../VISION.md)). JWT/auth hardening + Phase 1 sprint closed in-session 2026-04-25..27 (auth tightening WS-002/003/008/009 + WS-057/058/083, `AppRuntimeConfig` WS-080, typed `HoldService` WS-082, session hygiene WS-052, HTTP integration WS-011/012, policy-resolve `GET` WS-034, calibration + route mount WS-038/041); details in [`crates/wos-server/README.md`](crates/wos-server/README.md). **Parent MVP foundation cluster** ([parent PLANNING.md](../PLANNING.md) PLN-0331..0367, owner-decided 2026-04-27) coordinates Phase 4+ adapter rows as execution home: WS-020 Postgres operational `Storage` (gated by parent PLN-0368 cross-submodule path-dep), WS-090 Postgres-append-only `AuditSink`, **WS-095** `eventstore-embedded` (per `wos-server/VISION.md` §VI; coordinated by parent **PLN-0387**), WS-093 Trellis exporter, WS-094 Restate `RuntimeAdapter`, plus WS-* rows the adapter author adds when sequencing reaches authz-openfga / identity-{webauthn,oidc} / kms-cloud / processing-audited / blobstore-s3 / `CRYPTO_OWNER` fence. **Reconcile** interim `Storage`+`AuditSink` plan with root `VISION.md` single-`EventStore` / no–dual-store commitment as Trellis composition lands. **Parent stack closure cluster** (synthesis-merge 2026-04-27, PLN-0379..0398) adds: **PLN-0388** (`agent-sdk` peer crate + CRYPTO_OWNER fence — extends PLN-0340 fence pattern to the agent-sdk crate; agent-sdk MUST NOT import crypto libs), **PLN-0389** (three-app frontend split: Studio + Caseworker + Admin — ratify ADR or VISION amendment; product/UI work, not center), **PLN-0391** (IntakeHandoff transport-orthogonal guard — ADR 0073 amendment + lint preserving artifact-vs-transport distinction, P2). **Continue / iterate** (Phase 1–7 dependency-ordered sequence, #62–#64, session hygiene, doc mirrors): single source of truth is **[`crates/wos-server/TODO.md`](crates/wos-server/TODO.md)** — update there only to avoid drift.

### Runtime Companion §15 / Phase 11 — reference `wos-runtime` parity

Normative prose (`specs/companions/runtime.md` §12.9 / §15), kernel + case-instance schemas, and `wos-formspec-binding` landed for the Formspec coprocessor handoff ([Phase 11 backlog closure §G.1](../../thoughts/plans/2026-04-11-phase11-coprocessor-open-backlog.md)). **`wos-runtime` still diverges** on several MUST-level submit-path behaviors (auth/agent rejection surface, optional Governance **`contractHook`** / S5 ordering, abandon vs skip lifecycle, amendment-task automation). **`wos-server`** §15.7 ledger gating is layered on `ContractValidator`; architecture review ([`thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`](thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md) placement note) prefers default enforcement at the **runtime submit boundary** where `impactLevel` is known — reconcile and implement one coherent story.

- **#66 Runtime §15 processor parity — `wos-runtime` center** `[7 / 5 / 5]` (**35**) — umbrella; decomposed below. **Gate: none** (spec already published). HTTP/tests: [`crates/wos-server/TODO.md`](crates/wos-server/TODO.md) **WS-011**, **WS-075**; PARITY table refresh **WS-074** when seams match shipped code. (**WS-072** remains ADR 0066 reference-server follow-on per parent [`TODO-STACK.md`](../TODO-STACK.md).)
  - [ ] **#66a Typed submit rejections + replay** — Map failed actor authorization to `TaskSubmissionResult::Rejected` with `taskSubmitterUnauthorized` (not `RuntimeError::Unauthorized`); persist replay entries per Runtime §15.5 step 3.
  - [ ] **#66b Agent submitters** — `actorExtension` registration, `agentSubmitterUnauthorized`, provenance `actorType: "agent"` + model/version metadata, rights/safety human-delegation rules per §15.5 step 4 / backlog §G P11-BL-004.
  - [ ] **#66c `ledgerEvidenceMissing` placement** — Decide single enforcement layer: extend submit path in `wos-runtime` (kernel/instance `impactLevel`) vs validator-only; align with `PolicyLayeredValidator` so headless runtime and HTTP agree.
  - [ ] **#66d `contractHook` / Governance S5 post-pass** — After mapping computes proposed mutations, optional case-level hooks on the completion bundle, then atomic commit; hook failure → `taskFailed`, unchanged case, `failureEvent` when configured (§15.5 step 16).
  - [ ] **#66e Abandonment + skip semantics** — `dismissTask` vs `persistTaskDraft` + `stopped` vs explicit abandon; `claimed → failed` + `taskFailed` + `failureEvent`; skip path → `skipped` + `taskSkipped` + removal from `activeTasks` without completion/failure events (backlog §G P11-BL-040). **Confirmed gap 2026-04-28 audit** ([`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md) Gap 2): `ProvenanceKind::TaskSkipped` is the only enum variant in 95 with zero live emission across `wos-runtime` + `wos-formspec-binding` + `wos-core`; spec MUST at Runtime Companion `:863` + outcome row `:929` + Workflow Governance `:496` is currently un-met.
  - [ ] **#66f Amendment task linkage** — New task after completed submit with `amendmentRef` / supersession-related fields per §15.9; coordinates with [ADR 0066](#adr-0066--execution-checklist-wos-center) when that ADR is accepted.
  - [ ] **#66g Conformance fixtures** — `wos-conformance` (or runtime integration tests) covering the matrix: auth reject, agent reject, ledger missing, hook fail, skip vs fail, mapping absent (no silent projection — already partial in binding).

### Audit + evidence products

Build on the stable provenance export surface. #48 Merkle provenance moved to Trellis scope; see "Moved to Trellis" below.

- **#52 Simulation trace format** `[4 / 3 / 2]` (**8**) — normative replay contract + conformance fixtures. Event log format already shipped via `wos-export::xes`.

### Verifiability closure (1.0)

Per [`VISION.md`](../VISION.md) stack Q4 / verifiability posture (reference impl as oracle): every promoted normative MUST should have executable evidence; CI lint-matrix gates cover rule → fixture. These close the remaining verifiability claims.

- **Provenance emission completeness audit** — **CLOSED 2026-04-28.** Audit at [`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md) verified all 99 `ProvenanceKind` variants at audit time (100 at HEAD post-Session 14) against runtime + binding + core emission sites and cross-checked spec `recordKind:` literals against the enum. Two enum-level gaps surfaced: **Gap 1** `CapabilityInvocation` (no variant — promoted to Do-next #2); **Gap 2** `TaskSkipped` (variant exists, no emission — already tracked at backlog #66e). Plus **Gap 3** under-specified configuration-warning emission (4 spec MUSTs, no recordKind discipline — filed in Behavioral / governance below). All other variants emit through `wos-core/src/event_handler.rs` (governance / AI / DCR paths) or `wos-runtime/src/runtime/*.rs` (lifecycle / tasks / signature / intake / custody).
- **`K-DET-001` determination-snapshot conformance + fixture migration** `[6 / 3 / 5]` (**30**) — conformance rule `K-DET-001` (determination transitions require `caseFileSnapshot`) plus the fixture migration that populates `caseFileSnapshot` on every determination-bearing fixture. `Last audited` line claims `#24a` Facts-tier snapshot closed; the schema/runtime/test layers landed but the conformance gate didn't — narrative deeper than enforcement. Determinations bear legal weight; missing snapshots make rights-impacting decisions unverifiable. Discovered 2026-04-24 audit ([`thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md`](thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md) verdict #3 against `plans/2026-04-18-wos-facts-tier-input-snapshot.md`).
- **Seeded LoadBearing-promotion batch + rule-coverage CI step** `[6 / 4 / 4]` (**24**) — Snapshot lint-matrix shows `1 LoadBearing · 11 Tested · 104 Draft`; K-049 is the only LoadBearing rule. Plan `2026-04-16-wos-rule-coverage-conformance.md` (audit verdict #13) seeded a broader promotion set that hasn't landed, and `.github/workflows/wos-coverage.yml` + `ratchet-check` binary remain open. Land both together — promotion without CI gate is unverified; CI gate without promotions is silent. Turns the verifiability claim from aspirational ("116 rules") into evidence ("N rules with at-least-2 fixtures, gated"). Discovered 2026-04-24 audit ([`thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md`](thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md) verdict #13).
- **Kernel-Basic conformance profile LoadBearing declaration** — **RESCOPED 2026-04-28 (LARGER-THAN-CLAIMED).** Cross-stack-scout validation found "Kernel-Basic" appears as a *name* only in `RELEASE-STREAMS.md:11` claims-map row; no fixtures bound to the profile in `crates/wos-conformance/`, no profile definition file, no kernel-block selection. Real Cx is 5+ — you'd be writing the profile (definition + fixture binding + conformance crate wiring) before adding the LoadBearing flag. Re-file when the profile artifact actually exists. Backlog row `WOS-B2` (line 51 above) marks this as Backlog, consistent with the rescope.

### ADR 0064 + architecture-review handoff (methodology closure)

**Anchors:** [ADR 0064](../thoughts/adr/0064-wos-granularity-and-ai-native-positioning.md) (granularity, AI normative layer, named sidecars, lifecycle/runtime split) · [architecture-review-handoff](thoughts/archive/reviews/2026-04-16-architecture-review-handoff.md) §4–§6 · [open-questions synthesis](thoughts/archive/reviews/2026-04-16-architecture-review-open-questions.md). D-1–D-4 are **accepted architecture**, not a checklist; this subsection indexes **residual execution** so those decisions stay operationally true.

- [x] **§4.1 — `x-` extension seam vs JSON Schema** — **CLOSED 2026-04-28** ([COMPLETED.md](COMPLETED.md) Session 16). 2 schemas patched: `schemas/conformance/conformance-trace.schema.json` and `schemas/mcp/wos-mcp-tools.schema.json` gained top-level `patternProperties: { "^x-": { ... } }`. **Open separately:** `K-EXT-001` lint rule (T1 unknown non-`x-` property) — referenced in `LINT-MATRIX.md` jumps (K-005 → K-EXT-002 skips K-EXT-001) but not landed; surfaced by cross-stack-scout 2026-04-28, file as a discrete lint-rule item if pursued.
- [ ] **§4.2 — rule-coverage metrics + promotion ratchet** — **Consolidated** into *Verifiability closure* above (**Seeded LoadBearing-promotion batch + rule-coverage CI step**). Adds honest per-tier coverage %, fixture linkage gates, optional “disable rule → conformance fails” promotion job, and **K-012 / K-017** audit per open-questions Q4.
- [ ] **§4.3 — companion drift lint `COMP-001`** `[4 / 2 / 4]` (**8**) — T2 scan for duplicated normative claims between `lifecycle-detail.md` and `runtime.md` (handoff §4.3 alternative to merging companions). **Trigger-gated:** ship only if drift shows up in review or after precedence clause proves insufficient.
- [ ] **§4.4 — `COMPAT.md` + per-stream tags + CI staleness** — **Consolidated** into *Release + benchmarking* (**§4.4 Release trains Tasks 4-5**) plus open-questions Q3/Q5 action items (`wos-spec/COMPAT.md` mirroring parent [`COMPAT.md`](../COMPAT.md) conventions).
- [ ] **§5.2 — structured `LintDiagnostic` as stable output contract** `[6 / 5 / 4]` (**24**) — Handoff §5.2: machine-stable JSON for every rule (`ruleId`, `path`, `suggestedFix`, `relatedDocs`, …); prerequisite for LLM repair loops (pairs with **#65h** which optimizes prompts given diagnostics exist).
- [ ] **§5.3 — trace-emitting conformance** `[6 / 5 / 5]` (**30**) — Handoff §5.3: conformance returns teachable traces/deltas, not only pass/fail (feeds Claim A loop).
- [ ] **§5.6 — repositioning / demo artifacts** `[4 / 3 / 2]` (**8**) — Handoff §5.6: README lead with two-claim framing; demo narrative once `wos-synth` / bench exist. Partially satisfied by [`POSITIONING.md`](POSITIONING.md); close gap vs handoff list explicitly.
- [ ] **Open-questions action items (Q1–Q6)** — **Q1:** benchmark-regression policy text in synth/bench READMEs. **Q2:** `cargo tree` (or equivalent) guard that `wos-synth-core` has no LLM-client crates. **Q6:** boundary docs for `wos-bench` ↔ `wos-synth-core` (largely overlaps ADR 0065 / **#65** series — keep one owner).
- [ ] **Optional — ADR 0064 Neutral / Alternatives** — Relocate `DRAFTS/` to `history/` with pointer ADRs; **revisit merging** `lifecycle-detail` + `runtime` only if `COMP-001` or practice shows sustained drift; **revisit collapsing schemas** only if §4.2 metrics show rules routinely cross schema boundaries.

### Regulatory — 1.0 separate-spec deliverables

Per [`VISION.md`](../VISION.md) operating frame, these are 1.0 deliverables (not deferred) because spec writing is cheap under minutes-not-days and the compliance posture is load-bearing for the SBA adopter.

- **#50 EU AI Act alignment** `[7 / 5 / 4]` (**28**) — Art. 13-14 alignment spec.
- **#53 OMB M-24-10 compliance** `[6 / 4 / 3]` (**18**) — process-documentation-shaped; overlaps Assurance + impact-level plumbing.

### Interoperability + speculative (trigger-gated)

- **SCXML interoperability** `[3 / 6 / 2]` (**6**) — bidirectional WOS ↔ SCXML mapping. Trigger: ecosystem demand.
- **#51 Statutory deadline chains** `[4 / 7 / 5]` (**20**) — must compose with #31 business calendars + typed kernel events (`TransitionEvent`, #20). Trigger: first production deployment exposes concrete need.

---

## Moved to Trellis (scope-out)

Per [`VISION.md`](../VISION.md) §XI, Trellis is the integrity layer and owns these concerns. WOS emits records via `custodyHook`; Trellis anchors them. Tracked here only to close the loop on items that used to be listed as WOS work.

- **#48 Merkle provenance chains** — Trellis. Hash-chaining + SCITT alignment are Trellis primitives.
- **Federation Profile** (cooperative trust-anchor network) — Trellis. Previously tracked as WOS Future spec.
- **SCITT strictness** (full vs. adjacent) — Trellis decides.
- **Checkpoint seal protocol** — Trellis.
- **Proof-of-inclusion + transparency-log submission tooling** — Trellis.
- **Certificate-of-completion export bundle format** — Trellis export-bundle primitive.

---

## Blocked / needs decision

Items that can't move without a verdict or an external trigger.

### §4.5 PR packaging

`VISION.md` / practice recommends three discrete PRs; sidecar audit recommended one. Owner picks. See Do-next-adjacent "Structural merges" section above.

### Engine adapters — trigger-gated (commercial request)

WOS's first production runtime target is now the Restate adapter selected by WOS-T3. Additional adapters are trigger-gated on commercial adopter request or SDK maturity.

- **#49a Camunda 8 Worker** `[5 / 8 / 3]` — BPMN target; broadest external fixture diversity.
- **#49c AWS Step Functions** `[5 / 8 / 3]` — broadest commercial reach; narrowest semantic fit.

(#49b Temporal was evaluated by WOS-T3 and deferred until the Rust workflow API stabilizes.)

### Ontology field identity — design not started

`ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and §6 regulatory specs. Prerequisite design: JSON-LD `@context` decision, semantic-field-identity protocol, cross-document alignment. Move to active only once a draft exists.

---

## Deferred (with triggers)

Captured but not active; re-score when the trigger fires.

| IDEA # | Item | Imp/Cx/Debt | Trigger |
|---|---|---|---|
| #1 | Agent Behavioral Attestations | 2/7/1 | SLSA-style AI-agent attestation ecosystem matures. |
| #4 | Tripartite Object Model | 2/9/3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1/8/0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2/9/5 | Multi-object mutation emerges, or flat→OCEL export shows systematic loss. |
| #9 | JSON-LD Export Surface | 5/5/3 | Ontology spec drafts begin OR shipped PROV-O pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6/7/5 | #20 landed — unblocked. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3/5/2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

Federation Profile and Bulk Operations relocated — see "Moved to Trellis" and Backlog / behavioral items respectively.

| Spec | Description | Trigger |
|---|---|---|
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy. |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding); reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN expression language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; shipped PROV-O is JSON-LD. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Event taxonomy adopted normatively via #20. |

---

## Parked

- Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- JSON Patch for fine-grained provenance.
- FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open architectural questions

Most prior open questions (OQ1, OQ4, #21 Registry composition, #25 Defeasibility, #36 Equity expression, #43 Assurance × impact-level, #9 JSON-LD authoring) are now **resolved** per [`VISION.md`](../VISION.md) §X (WOS settled commitments) and linked ADRs. Remaining genuinely-open decisions:

1. **§4.5 PR packaging** (sidecar-audit Q1). One PR (audit recommendation) or three (`VISION.md` / historical practice: discrete PRs for review hygiene)?
2. **`custodyHook` evidence path to Trellis** — WOS→Trellis **authored append wire** is fixed by [ADR-0061](thoughts/adr/0061-custody-hook-trellis-wire-format.md) and WOS-T1 closeout (four-field input, `(caseId, recordId)` idempotency, `CustodyAppendReceipt` → `canonical_event_hash` on provenance). **Open** is cross-stack **proof**: the landed Formspec canonical signed-response artifacts still need Trellis append/export vectors staying byte-aligned with live WOS emitters, plus Studio authoring gates. This is the same bundle as WOS-T4 “Next slice” / Trellis verification maintenance, not an undecided WOS wire-format ADR.

For stack-wide active uncertainties (DocuSign parity scope, multi-tenant on Restate/Temporal, rendering service for signature artifacts), see [`VISION.md`](../VISION.md) §X *Active uncertainties (WOS-scope)*.

---

*Closed-out work is archived in [COMPLETED.md](COMPLETED.md). Append there, not here.*
