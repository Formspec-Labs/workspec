---
title: WOS Signing Adapter Catalog
date: 2026-10-20
status: normative
---

# WOS Signing Adapter Catalog

This catalog defines the fourteen replaceable boundaries used by routed document signing: their defaults, alternatives and evidence required for a deployment to claim support. A **port** here is a logical boundary that an implementation can replace; it is not a new Rust trait, HTTP API or JSON document.

The catalog is normative project specification text. Its acceptance does not ratify a stable WOS release, the entire [research design](../../../thoughts/research/2026-05-12-content-attestation-envelope-research.md), or a deployment. Naming a default does not establish that its adapter exists. [Implementation coverage](#5-current-owners-and-coverage) and [conformance](#4-conformance) keep those claims separate.

## 1. Scope and authority

An **envelope** in this document means documents routed to recipients for signing, with an identity and the recipient/aggregate states in [Signature Profile §2.4.1](signature.md#241-envelope-and-recipient-state-model). This catalog does not add kernel state kinds or replace that state model.

The [Signature Profile](signature.md) owns governed signing: roles, order, admission, consent, intent, authentication and authority floors, provenance and final-seal observation. [Formspec Core](../../../formspec/specs/core/spec.md#216-response) owns authored signature evidence and its signed bytes. [ADR 0085](../../../thoughts/adr/0085-wos-formspec-signature-consumption-binding.md) names their binding; the records remain distinct. [Trellis Core](../../../trellis/specs/trellis-core.md) owns cryptographic bytes, integrity and export verification. A conforming adapter MUST preserve these owners' requirements; selecting an adapter cannot weaken them.

Signature remains an embedded workflow block. This prose catalog introduces no `$wos*` marker, `targetWorkflow`, independent authored version, signed payload or new cryptographic primitive. Its date identifies the prose revision, not a wire-format version. See [ADR 0076](../../../thoughts/adr/0076-product-tier-consolidation.md) and [WOS ADR 0063](../../thoughts/adr/0063-embedded-vs-sidecar-identity-boundary.md).

The key words MUST, MUST NOT, SHOULD and MAY describe conformance requirements. Requirements whose implementation or executable evidence is absent remain gaps under [§5](#5-current-owners-and-coverage); prose does not satisfy them. [§3.3](#33-boundary-write-identity-and-replay) defines write-deduplication keys and replay requirements. [§3.4](#34-conditional-lifecycle-writes-and-races) defines cross-operation ordering. Seal-failure recovery remains separate work under [§3.5](#35-seal-failure-recovery).

## 2. Normative Contract

### P-1. DocumentBytesStore

**Default — intake cache, then Trellis content-addressed storage.** The store owns pinned document bytes from intake through abandonment, signing, sealing and the sealed bundle's retention period. At sealing, bytes move into the Trellis content-addressed store; cache garbage collection becomes eligible only after that durable handoff. Abandoned drafts that never reach `sent` MUST NOT create Trellis chain records merely to retain their cached documents. After sealing begins, admission MUST check the pinned Trellis bytes/digest rather than mutable cache content.

**Alternative tiers.** Content-addressed storage from intake retains even abandoned draft bytes at higher storage cost. Server-only storage omits the Trellis commit and offers deployment-local evidence; it cannot claim this catalog's offline-verifiable `basic-esign` profile.

**Conformance.** Preserve the intake bytes when a source URL later changes; refuse a digest mismatch; demonstrate the sealing handoff and safe cache deletion; show that an abandoned unsent draft adds no chain record under the default. A storage receipt alone does not prove document admission or export inclusion. Compose with P-11 resolution and P-14 sealing.

### P-2. ViewEvidenceCollector

**Default — chain-anchored, per-session summary.** One open/scroll/close viewing session produces an aggregated evidence event with viewer version, document references, selected locale, viewport class, scroll completion and dwell time. The collector MUST bind the summary to the actual session and viewed documents and MUST NOT represent it as proof that a person read or understood them. Storage grows by summarized sessions rather than individual scroll events.

**Alternative tiers.** A Response-side block carries evidence only with signed responses and loses pre-signing views. A kernel-only event retains an in-process audit without portable bundle evidence. These tiers MUST disclose their evidence limits; they do not acquire the default's portable process claims.

**Conformance.** Exercise a completed session, an unsigned/pre-signing session and a retry; preserve document/locale association and demonstrate bounded aggregation. Reject association with another recipient or document. Verify that exported process claims reproduce the retained summary; absence is explicit, not a fabricated zero-duration view. P-13 interprets the evidence, not cognition.

### P-3. VoidingPolicy

**Default — process claim enforced for governed use.** An authorized void writes one append-only process event. Admission MUST refuse later signing under the voided identity, and the verifier MUST report the void when deciding governed-use validity. The event does not erase prior signatures or change whether their historical cryptographic bytes verify.

**Alternative tiers.** Orphan-marker voiding adds a marker for each prior signature. It retains the same authority and governed-use refusal while making each affected signature's status explicit at higher event cost.

**Conformance.** Demonstrate authorized void, unauthorized refusal, subsequent signing refusal and offline status interpretation. The original signed records remain unchanged. Exercise every affected signature under the orphan-marker tier. Conditional ordering against access, signing or sealing needs the separate race rules in §3.4; this paragraph does not settle those races.

### P-4. ReassignmentAuthority

**Default — originator-only.** Only the authorized originator can reroute a pending assignment. Reassignment MUST retain original signer, replacement signer, authorizing actor, time and reason; it MUST NOT rewrite the original assignment as if the replacement had always held it. The Signature Profile's admission and reassignment rules still apply.

**Alternative tiers.** A delegated recipient can reassign within a valid delegation grant. A role-based adapter admits a configured workflow administrator override. Neither tier permits an unbounded grant or bypasses current authority checks.

**Conformance.** Admit the authorized replacement, refuse another actor and a revoked/out-of-scope grant, retain the assignment history, and refuse signing through the superseded assignment. Test each claimed alternative independently; a generic administrator permission is not evidence of this port's policy.

### P-5. LocaleResolver

**Default — negotiate at session open.** Show the declared document variants and record the signer's choice in session evidence. Resolution MUST identify the exact `(documentId, locale)` and pinned bytes used for admission and signing. A Formspec form-string Locale selection alone does not resolve declared document variants.

**Alternative tiers.** Strict mismatch handling refuses when no acceptable variant exists. Default-locale fallback selects the declared default automatically, without an interactive choice, and records that selection. An adapter MUST NOT substitute an undeclared variant or silently change pinned content.

**Conformance.** Exercise explicit selection, unavailable locale, each claimed fallback/refusal policy and consistent document/session/signature references. Changing the display language MUST NOT retrospectively change an earlier signed reference. Definition/Response document declarations remain owned by Formspec.

### P-6. IntentRegistry

**Default — full maintained WOS baseline plus labeled local extensions.** Use [Signature Profile §2.13.1](signature.md#2131-registered-uris-baseline-set) rather than copy its URI meanings or floors. A deployment-local intent MUST carry a human-readable label in exported evidence. Recognition is not admission: authentication, signer authority and jurisdictional posture still govern the act.

**Alternative tiers.** Canonical-only admits no local extensions. Deployment-local-only is an adapter design choice with weaker portability; it does not waive WOS conformance. A WOS-conforming processor MUST still recognize the complete baseline, preserve its reserved namespace and floors, and MUST NOT reclassify an omitted WOS URI as local. A configuration unable to meet those requirements cannot claim WOS Signature Profile conformance.

**Conformance.** Demonstrate baseline recognition, authorized admission and insufficient-floor refusal; reject a malformed URI, unauthorized reserved-namespace override and unlabeled local extension. Export a valid local label. Use the maintained owner's registry/floor controls, not a second table in this catalog.

### P-7. SignerSessionPort

**Default — opaque bearer, one-shot redemption.** Redeem a link into the intended recipient's signing session. Concurrent/retried redemption for the same envelope, signer and token MUST return the same admitted access outcome rather than create a second access act. IP address and user-agent are evidence, not deduplication identity. Possession of a link does not by itself establish P-8 identity proofing.

**Alternative tiers.** Signed JSON Web Token (JWT) sessions bind expiry and allow verification of the token signature without the issuer. Capability URLs trade access-token exposure risk for simpler access. Chain-anchored credentials record issuance in the chain. An alternative MUST enforce its declared recipient, scope and expiry; offline token verification does not prove current workflow permission.

**Conformance.** Exercise a valid recipient, wrong recipient/scope, expired token, concurrent scanner/signer redemption and exact retry. Retain the same access outcome on retry without treating a changed effect request as equivalent, as specified in §3.3. Ordinary login or task access does not qualify this signing-session port.

### P-8. RecipientIdentityProofingProvider

**Default — named proofing primitives.** The supported vocabulary is email-control, phone-control, knowledge-based authentication, identity-document capture and selfie matching. A selected primitive emits typed proofing evidence associated with the signer; its claim, provider and result MUST remain distinguishable from link redemption. A deployment MUST declare which primitives/providers it actually supports and MUST refuse an unsupported required method. `basic-esign` selects email-control; it does not claim every primitive was performed.

**Alternative tiers.** A profile composer translates choices such as basic, knowledge-based or remote online notarization into primitive requirements. Vendor adapters implement those same evidence boundaries. Labels such as a vendor's “verified” result MUST NOT replace the Signature Profile's required authentication or authority checks.

**Conformance.** Demonstrate the selected primitive's successful evidence and failure/unsupported outcomes; refuse a result bound to another signer or session. Check applicable profile floors and preserve provider-specific evidence needed for interpretation. A primitive result is not counsel-pinned legal sufficiency.

### P-9. DeliveryChannel

**Default — email and in-app.** Both channels make the routed signing request available under the selected recipient and disclosure policy. The adapter MUST distinguish attempted delivery, transport acceptance and any separately evidenced recipient receipt; a successful SMTP capture cannot prove external receipt.

**Alternative tiers.** SMS, postal and push delivery use the same recipient/evidence boundary, with their own provider and regulatory requirements. A deployment MUST identify the channel actually exercised rather than infer its success from another channel.

**Conformance.** Exercise authorized send, incorrect/revoked recipient refusal, retained transport outcome and uncertain-send handling without an uncontrolled duplicate effect. Demonstrate both default channels or each specifically claimed alternative. Preserve original attempt identities and evidence under §3.3; the existence of delivery code alone does not qualify those replay rules.

### P-10. EnvelopeTimerPolicy

**Default — bounded expiry and aggregated reminders.** Own expiry, reminder cadence, dwell windows and stale-state escalation in one selected policy. At sealing, reminders form a `deliveryHistory` summary rather than one chain record per reminder. The adapter MUST declare its bounds and show that retained summaries do not grow merely with repeated polling or elapsed reminder days. Signed-document evidence still grows with actual signer/document pairs.

**Alternative tiers.** Per-event anchoring records every reminder. Telemetry-only omits reminder evidence from the bundle. Custom per-recipient expiry and escalation-to-administrator policies change the declared timing/authority behavior. Evidence limits and cost differences MUST remain explicit.

**Conformance.** Exercise expiry before completion, the selected reminder/escalation policy, replay without extra effects and summary bounds. Existing expiry observations MUST NOT be rewritten on restoration. Verify each claimed alternative's exported event/summary behavior. Native timer support alone does not qualify the proposed reminder history.

### P-11. DocumentSourceResolver

**Default — server-side allowlisted fetch and pin.** Resolve declared URL sources at intake and pass exact bytes to P-1; inline sources already supply their bytes. The selected resolver MUST enforce deployment URL restrictions, including server-side request-forgery protections, and MUST NOT re-fetch a mutable source during later admission. Admission checks the pinned digest instead.

**Alternative tiers.** Browser-side fetching supports intranet sources the server cannot reach. Inline-only refuses non-inline declarations. Both retain the same pinned-bytes requirement; browser access does not establish server authority or integrity by itself.

**Conformance.** Exercise allowed source, disallowed target/redirect, failure and inline input. Change a URL's content after intake and demonstrate that signing/admission retains the original pinned bytes. A failed resolution MUST NOT become an empty successful document. Test the selected browser/inline restriction where claimed.

### P-12. PresentationHasher

**Default — exact source bytes.** Hash the exact source bytes selected for this port; different bytes produce different commitments. When used as `presentationHash`, retain the [Trellis presentation-artifact domain](../../../trellis/thoughts/adr/0007-certificate-of-completion-composition.md), distinct from document-content hashing. This default does not prove pixel reconstruction, equivalence between source and a browser rendering, or that an external resource remained unchanged.

**Alternative tiers.** Canonicalized DOM after script stripping requires an identified canonicalization algorithm and version. Source bytes plus an embedded-resource manifest also bind the listed resources. An adapter MUST name its algorithm and exact input coverage; neither alternative may silently reuse the default's meaning.

**Conformance.** Use the owning hash/domain implementation and positive/altered-byte/wrong-domain controls. For DOM mode, supply shared canonicalization vectors; for resource mode, changing a bound resource MUST change/refuse the commitment. Do not infer presentation qualification from an existing fallback to `documentHash`.

### P-13. EnvelopeVerifierFacade

**Default — structural and process questions.** Compose maintained Trellis verification and WOS domain results into: who signed, what document/reference was signed, when, associated view evidence, void status and proofing assertions. Also expose a structural inspector for evidence outside those six questions. Each answer MUST identify its verified evidence and distinguish absent, unchecked or failed evidence from a positive result. View duration is a process claim, not cognition; a signer name is not stronger identity proof than the underlying assertion.

**Alternative tiers.** Structural-only omits process interpretation. Posture interpretation adds the declared policy at signing time where evidence supports it. Neither tier may turn unchecked authority or missing policy closure into a positive policy claim. Unreadable encrypted content cannot support a readability or plaintext-content validation claim; independently established cryptographic integrity and signature results remain reportable.

**Conformance.** Run the owning offline verifier on the original exported bytes with independently selected trust material; map answers to its records/reports and preserve failures. Alter a signature/document association and remove view/proofing/policy evidence to demonstrate refusal or explicit incompleteness. Detailed question-to-report mappings and the facade interface remain `fs-jjwc`; this catalog defines no parallel verifier or substitute proof report.

### P-14. SealingOrchestration

**Default — asynchronous Trellis seal and observation.** After admitted signing, the UI may show “signed, finalizing.” Aggregate state MUST NOT become `completed` until the durable seal/closure event is observed as required by Signature Profile §2.4.1. The design target is a single-digit-second sealing budget; deployments MUST declare their actual budget and measure it before claiming that performance. This catalog neither sets a universal timeout constant nor claims that current code meets that target.

**Alternative tiers.** Synchronous sealing keeps the final session waiting for durable completion. WOS-resident sealing omits Trellis and provides in-instance evidence without this catalog's offline-verifiable bundle claim. Neither alternative may equate the last signer's success with an unobserved seal.

**Conformance.** Exercise accepted signing with delayed seal, successful observation, denied/failed seal and uncertain outcome/retry; verify that state does not report completion early. Export original durable bytes and verify them offline where claimed. Measure any declared latency on the actual path, separating service time, polling and download. Recovery/abandonment details remain §3.5 work; an old successful case archive does not qualify a new signing product.

## 3. Composition

### 3.1 Binding and precedence

A deployment MUST identify its selected adapter for each port, including algorithm/provider versions, supported methods and evidence limits. The profile table supplies defaults; a profile's stated overrides replace only those bindings. Additional changes constitute an explicitly described custom composition, not an unchanged named profile.

An adapter MUST preserve Formspec signed-byte semantics, WOS governed admission and Trellis integrity rules. Where a binding cannot satisfy a required floor or evidence claim, the deployment MUST refuse that act or withhold the conformance claim; it MUST NOT silently fall back to weaker semantics. Existing Signature Profile rules govern signing validity. This catalog governs adapter choice and qualification, not a competing admission ladder.

### 3.2 Named profiles

Unlisted ports inherit their defaults. Optional bindings are declared explicitly, never assumed tested.

| Profile | Overrides and required interpretation |
|---|---|
| `basic-esign` | All defaults; P-8 uses email-control. Email plus in-app delivery, exact-source-byte hashing, structural plus process verification. |
| `ron` | Remote online notarization: P-8 video adapter, P-7 signed JWT, P-13 posture interpretation; P-2 remains default. |
| `compliance-strict` | P-3 orphan markers, P-10 per-event anchoring, P-12 canonicalized DOM, P-13 posture interpretation. |
| `in-person-notary` | Existing K-6 dual-identity annotation; P-8 in-person witness primitive. P-9 MAY add physical delivery and MUST disclose whether it does. |
| `intranet` | P-11 browser-side fetch. P-1 MAY select server-only storage; that choice removes the offline bundle claim and MUST be disclosed. |
| `single-locale` | P-5 declared-default automatic fallback. |
| `audit-maxed` | P-1 content-addressed storage from intake; P-3 orphan markers; P-10 per-event anchoring; P-12 canonicalized DOM; P-13 posture interpretation; P-14 synchronous sealing. Every operational event is chain-anchored; higher storage and tail-latency cost is part of the composition. |

A profile name alone proves neither adapter support nor jurisdictional legal sufficiency. A deployment MUST qualify every selected override and inherited default under §4. Variants without portable evidence cannot satisfy the offline `basic-esign` acceptance merely because another profile permits them.

### 3.3 Boundary-write identity and replay

**Key, request and permission are different.** Each boundary below MUST use its logical key within the owning tenant, deployment/resource scope and operation kind. Components retain their admitted values: implementations MUST distinguish the structured tuple, rather than use ambiguous delimiter concatenation, implicit case folding or locale fallback. For `documentRef`, compare the exact `documentId` and `locale` string values; object property order is not another document identity. `tokenId` is an opaque identifier, not permission to store a raw bearer credential in keys or logs. These are logical identities, not new JSON fields, wire encodings or signed-payload formats.

The owner MUST retain the admitted effect request's bytes or authoritative identity and associate its outcome with that key. Equality MUST preserve all effect-relevant content and bindings; it MUST NOT discard a changed recipient, document, signer, intent, channel, authorization scope or selected evidence. Use the owning format's existing byte/identity rules, not a second canonicalizer. Current credentials and volatile transport observations are checked separately from immutable effect content. A matching key alone neither proves equal requests nor authorizes disclosure of a stored result.

| Boundary write | Logical deduplication key | Bound request and replay result |
|---|---|---|
| P-7 session redemption | `(envelopeId, signerId, tokenId)` | Bind the original recipient/session access request and token scope/expiry. Exact redemption retries retain the original access/session outcome, not a new `accessed` event. IP/user-agent observations are transport evidence, not key or effect-equality fields; scanner and signer observations do not create separate access acts. |
| P-2 finalized view summary | `(envelopeId, signerId, sessionId)` | Bind the final document/locale/view summary. Pre-finalization collection is separate from this one anchored write. Exact retry retains its original event; a changed finalized summary conflicts. A genuinely new viewing session uses a new session identity. |
| P-9 delivery attempt | `(envelopeId, recipientAddress, attemptId)` | Bind the retained recipient, selected channel, content and disclosure context. One attempt identifies one channel/effect. A changed channel or message under the same key conflicts; replay retains the original transport outcome. A new authorized attempt is a new operation, not an automatic workaround for an uncertain send. |
| P-3 void | `(envelopeId, voidRequestId)` | Bind the authorized requester, target scope and reason. Exact retry retains the original void record, actor and time. A changed void request conflicts; ordering against another operation remains §3.4. |
| P-4 reassignment | `(envelopeId, signerId, reassignRequestId)` | Bind the original assignment, replacement, authorizing actor/grant reference and reason. Retry retains the original assignment and audit-edge identities. A changed replacement or admitted grant/intention conflicts; current permission checks do not rewrite the original grant evidence. |
| P-14 sealing | `(envelopeId, sealAttemptId)` | Bind the original immutable selected signing/evidence prefix, configuration and seal request. Retry returns or reconciles that original selection/receipt; it MUST NOT substitute the latest head, refresh an evidence deadline or silently allocate a new seal. |
| K-2 affirmation append | `(envelopeId, signingActId, documentRef)` | Bind the exact admitted signer, source-signature evidence, digest/algorithm, intent and witness edge for that document. Minimal `documentRef` remains `{documentId, locale}`. One gesture shares an act ID across distinct document keys; changed signer/content under an existing key conflicts. Retry retains each original provenance record's ID, timestamp and result. |

**Exact replay and conflict.** A committed exact duplicate MUST produce no second mutation and MUST retain the original result/evidence bytes and identities. The owner MUST refuse a differing effect request under that key as a conflict, rather than overwrite it, return fabricated success or silently switch keys. Rechecking current permission may refuse disclosure of the retained result; replay does not restore revoked authority or confer permission through cached output. Historical outcome bytes remain unchanged even when a later authorized view or refusal differs.

**Concurrency and uncertainty.** Same-key concurrent requests MUST NOT independently perform the effect. The actual durable owner must reserve/compare the request and associate the result; an in-process mutex or HTTP response cache is not proof of durable external-effect coordination. After an uncertain remote outcome, recovery MUST reconcile the original effect or use an already qualified remote idempotent operation with that same original request. It MUST NOT blindly resend or mint a new identity. Expiry, purge or restart MUST NOT make a committed or unresolved effect fresh again; retention must preserve its replay/refusal meaning. Operational backoff, incident recovery and abandonment thresholds remain §3.5 work. This specification promises neither eventual completion nor unconditional exactly-once external execution.

**Qualification.** For each claimed boundary, demonstrate exact replay, changed-request conflict, same-key concurrency, scope isolation, lost-response recovery and current output authorization. Check that original record IDs/times and effect counts remain stable, and that an unresolved result does not cause another effect after restart. Distinct-document keys under one signing act MUST remain distinct; do not collapse them merely to fit an existing catalog implementation. Pre-admission refusal is not evidence that an external effect committed.

**Existing owners and gaps.** [Stack Common's replay primitives](../../../stack-common/crates/stack-common-idempotency/src/lib.rs) already distinguish byte-identical replay from payload conflict and provide scoped replay/locks. Its in-memory store is not a durable effect owner. [Browser completion](../../../workspec-server/crates/wos-server/src/services/browser_completion.rs) retains exact selectors under current permission; [notice delivery](../../../workspec-server/crates/wos-server/src/services/notice_delivery.rs) and its [Postgres store](../../../workspec-server/crates/wos-server-postgres/src/notice_delivery.rs) retain claimed attempts and refuse uncertain resends. These are reuse candidates, not qualification of the seven product keys.

The current [generic task replay](../../crates/wos-runtime/src/runtime/tasks.rs) can return a stored task/actor/token result before comparing the incoming response. It therefore does not, by itself, establish this section's changed-request conflict requirement. The current [Trellis signed-acts correlation](../../../trellis/crates/trellis-verify-wos/src/signed_acts.rs) groups by act ID and compares document reference/hash fields; these key rules do not qualify multiple distinct documents under one gesture against that implementation. Actual owner controls and any required correction remain with `fs-jjwc` / `fs-6pb7`. This is a source-identified coverage gap, not an executed product-failure claim or a change to intended per-document keys. No runtime conformance follows from acceptance of this subsection (`fs-6nik`).

### 3.4 Conditional lifecycle writes and races

**One durable order.** The WOS lifecycle owner MUST condition a new transition on `(envelopeId, currentState, expectedVersion)` within the owning tenant/deployment scope. `currentState` is the expected aggregate state under [Signature Profile §2.4.1](signature.md#241-envelope-and-recipient-state-model); `expectedVersion` identifies the committed lifecycle revision against which the operation was authorized. Every accepted lifecycle mutation MUST produce a distinguishable revision, including an intervening change that leaves the visible state equal. This logical precondition introduces no new wire field, kernel state kind or revision encoding.

The owner MUST compare that precondition against one current committed view, check the affected assignment, current authorization and selected policy, and durably commit the accepted transition with its operation/result association and required provenance. The check and local commit MUST be atomic with respect to competing lifecycle writes for that identity. An in-process mutex, UI tag, client timestamp or response-arrival order does not establish this durable order. Remote delivery or sealing is not made atomic merely by this local requirement; its original request and uncertain outcome remain governed by §3.3.

**Stale requests and replay.** Different operations against the same revision MUST NOT both succeed against that revision. A stale or illegal transition MUST receive an explicit precondition refusal without its proposed lifecycle effect. Rereading current facts may explain the refusal, but MUST NOT silently refresh `expectedVersion` under the retained key. A changed conditional request requires an explicitly authorized new operation identity or an already specified owner protocol; it is not exact replay. New identity MUST NOT bypass an unresolved original effect (§3.3). Conversely, an already committed exact replay retains its original outcome under current disclosure permission; it is not a new transition evaluated as though the earlier effect had never happened.

**Precedence follows accepted facts.** The following table covers different operations. Each later operation still needs its own valid current precondition and authority; winning an earlier comparison grants no future permission. The owner MUST apply its configured expiry predicate at fresh admission rather than accept a backdated caller `signedAt` or delayed callback as an override. Clock, deadline and trigger semantics remain the selected timer policy's responsibility (P-10), not a universal timeout introduced here.

| Concurrent actions | First accepted action | Required later behavior |
|---|---|---|
| Authorized void / recipient access | Void commits first. | Refuse a new valid signing-session/access act under the voided ceremony and refuse later signature admission. An exact replay of older access evidence does not restore permission. |
| Authorized void / recipient access | Access commits first. | Retain the truthful access event. An authorized void may still terminate the nonterminal ceremony and prevent later signing; access does not outrank cancellation. |
| Configured expiry / admitted signing | Expiry commits first. | Refuse a new signature for the expired assignment/ceremony. Preserve earlier admitted signatures and their original evidence. |
| Configured expiry / admitted signing | Signing commits first. | Retain that admitted signature. If required pairs or closure are still outstanding, configured expiry may still terminate the ceremony and expire outstanding unsigned assignments. A signature alone does not establish aggregate completion. If valid completion already committed, a delayed expiry MUST NOT replace it. |
| Authorized void / seal work or observation | Void commits while the ceremony remains nonterminal, including after a seal request or remote effect but before valid completion observation commits. | Retain any real seal/signature bytes and truthful remote outcome; a late receipt MUST NOT overwrite `voided` with `completed`, admit new signing or authorize disclosure. A seal request or remote success response alone is not aggregate completion. |
| Authorized void / seal work or observation | Every required signer/document pair and actual final seal/closure observation have already committed `completed`. | Refuse a lifecycle cancel that would reopen or replace that terminal state. Any separately authorized post-completion revocation/orphan reporting retains its existing governed-status meaning; it MUST NOT fabricate a `completed`-to-`voided` lifecycle edge. |

These rules MUST preserve the existing terminal meanings, configured decline/reassignment behavior and actor/reason/audit obligations in Signature Profile §§2.4.1 and 2.11. They establish neither a blanket priority ranking among terminal tokens nor last-writer-wins. A stale timer, receipt or access callback cannot erase an admitted fact or reopen a terminal ceremony.

**Seal evidence and current status.** Completion observation MUST refer to the original immutable selected prefix and actual durable closure, and its lifecycle write MUST satisfy the current precondition and authority. An uncertain response MUST be reconciled against that original request, not resolved by sealing a newer head. A historical receipt can remain valid evidence after current lifecycle permission changes; retaining it is not permission to make a new completion transition or disclose it.

An immutable earlier bundle proves its original selected prefix. It cannot reveal a later void absent corresponding later authoritative evidence. A claim about current governed status, or an updated offline status claim, MUST include the relevant later evidence under P-3/P-13; do not relabel an older archive as proof of events outside its prefix.

**Qualification.** For each race, exercise both accepted orders, same-revision competition, stale callback/timer refusal and exact committed replay. Retain the authoritative revisions, accepted/refused decisions and original effect identities; demonstrate that receipt reconciliation preserves history without overwriting terminal state or bypassing current authority. Also exercise expiry while admitted signing awaits closure, and remote seal success whose response arrives after void. These are required owner controls, not claims that current code implements them (`fs-a9dn`). The current source gaps in §5 and full-product conformance in §4 remain.

### 3.5 Seal-failure recovery

Pending-seal-failure retry, observability and abandonment rules remain separately tracked by `fs-fihy`. They MUST compose with §§3.3–3.4 and the existing state model rather than create a second one. Signature tag/state mapping remains owned by [Signature Profile §2.4](signature.md#24-lifecycle-tags).

Until the necessary semantics and executable owner evidence exist, a deployment MUST NOT claim complete retry/race/recovery conformance or complete routed-signing product conformance from this catalog alone. A receipt can still qualify its explicitly bounded behavior. Later work must preserve original failed/uncertain effects and existing admitted bytes; catalog acceptance does not authorize resets or live-business replay.

## 4. Conformance

To claim “deployment ships P-X,” a qualification MUST identify the selected adapter/methods, exact implementation and configuration, input/evidence identities, positive and refusal controls from that port, and their actual results. It MUST list unresolved requirements and MUST NOT call a port fully supported while required controls are absent. Merely implementing a storage, timer, authentication or verifier building block is insufficient.

Schema validation checks existing declared wire shapes; it does not demonstrate these adapter behaviors. Lint can check available references/declared constraints without proving external delivery, human viewing, durability or eventual completion. Runtime tests must exercise the real selected adapter boundary. Source inspection and static review establish only their stated scope.

A named-profile qualification MUST exercise all inherited defaults and selected overrides, retain the original output bundle and invoke the maintained offline verifier where portable evidence is claimed. `basic-esign` requires actual document declaration, recipient email access, browser viewing/signing, durable seal observation and stranger-machine offline verification. `audit-maxed` additionally demonstrates its stated changes; renaming the default fixture is insufficient. All six nondefault profiles require their own deviation evidence.

The conformance corpus remains `fs-6pb7` (default) and `fs-o1lw` (nondefault profiles). Existing D1 exports demonstrate a narrower current-case export/verifier composition. They do not establish declared-document routing, complete policy evidence, encrypted-content readability, new-profile behavior or fresh-case performance. Tests and product adoption remain separate claims.

## 5. Current owners and coverage

The following source map identifies reuse candidates, not completed adapters. This catalog introduces no runtime implementation or executable port qualification. Open requirements remain tracked by the named child tickets; default/nondefault execution evidence remains `fs-6pb7` / `fs-o1lw` unless a narrower owner is listed.

| Port | Maintained owner or building block | Qualification gap |
|---|---|---|
| P-1 | [Trellis export writer](../../../trellis/crates/trellis-export-writer/src/lib.rs), admitted evidence members | Declared-document cache/content-addressed lifecycle and retention/handoff controls are unqualified. |
| P-2 | WOS provenance and the same export writer | Browser session/view collection and portable summary association are unqualified. |
| P-3 | [Signature runtime](../../crates/wos-runtime/src/runtime/signature.rs), void authority/cancellation/provenance | Complete product void interpretation and races remain unqualified; `fs-a9dn`. |
| P-4 | Same runtime, reassignment authority and retained assignment history | Selected product delegation/originator policies and superseded-access controls are unqualified. |
| P-5 | [Formspec Locale](../../../formspec/specs/locale/locale-spec.md); current WOS affirmation uses `und` | Form-string localization is not declared-document resolution; `fs-7gl1` / `fs-ih9d`. |
| P-6 | [Signature registry/floors](signature.md#213-signing-intent-uri-registry) and same runtime | Existing recognition/admission owners are reusable; no full P-6 product adapter is qualified by this document. |
| P-7 | Signature task admission; existing application authentication | No signing-link token/session adapter qualification. |
| P-8 | [WOS Formspec binding](../../crates/wos-formspec-binding/src/lib.rs), verified identity/primitive evidence | No claim that every listed primitive or vendor is implemented. |
| P-9 | [NoticeDelivery](../../../workspec-server/crates/wos-server/src/services/notice_delivery.rs), current admission/store/channel | Governed notices are reusable, not proof of full signing-product email/in-app delivery or §3.3 replay qualification. |
| P-10 | [Native timers](../../crates/wos-runtime/src/runtime/timers.rs), signature expiry | Proposed reminder summaries, bounds and each alternative need product evidence. |
| P-11 | Existing byte/attachment owners | Declared source resolution, allowlist and fetch-and-pin qualification are missing; `fs-7gl1` / `fs-ih9d`. |
| P-12 | [Trellis presentation artifact](../../../trellis/thoughts/adr/0007-certificate-of-completion-composition.md); [binding fallback](../../crates/wos-runtime/src/binding.rs) | A fallback document digest is not the proposed presentation qualification; alternative canonicalization needs vectors. |
| P-13 | [WOS-aware verifier](../../../trellis/crates/trellis-verify-wos/src/lib.rs) and its catalogs/domain findings | Six-question mappings and inspector remain `fs-jjwc`; absent evidence cannot become a verified answer. |
| P-14 | [Browser completion](../../../workspec-server/crates/wos-server/src/services/browser_completion.rs), Trellis writer/verifier | New product seal observation, budget and recovery remain unqualified; `fs-fihy`. |

Formspec document declarations and resolved-document evidence remain `fs-7gl1` / `fs-ih9d`; their records must compose under ADR 0085. Trellis primitive clarification remains `fs-82ke`. These gaps prevent treating catalog publication as completion of `fs-m8ic`.
