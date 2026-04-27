;'# WOS Server — Architectural Vision

**Status:** Architectural commitment, 2026-04-25 (reaggregated).
**Authoritative spec for Formspec changes:** [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md).
**Authoritative spec for Trellis byte protocol:** [`trellis/specs/trellis-core.md`](../../../trellis/specs/trellis-core.md).

This document owns the cross-spec architectural framing for the WOS Server reference implementation and its place in the Formspec / WOS / Trellis stack. It defers Formspec-side specification to ADR-0074 and Trellis-side specification to `trellis-core.md`. It captures the wos-server-side commitments and the engineering discipline that makes them coherent.

---

## I. Operating Frame

**Optimization target: architectural elegance and minimum conceptual debt.** Tokens, AI agents, and frontier-model inference are unlimited. Calendar time and tech debt are scarce. The bottleneck is not single-threaded human typing.

This produces specific stances:

- **No phasing.** Phased delivery as a developer-time-saving move is rejected. Phasing as architectural-risk reduction (validate an assumption first) is accepted; nothing else qualifies.
- **No backwards compatibility.** Nothing is released. Existing AI-authored specs and code are exploration. Rewrite freely when the architecture demands it.
- **AI-authored documents are input, not authority.** "Locked," "ratified," "normative" labels in earlier exploratory documents are framing. Substance is evaluated independently.
- **Build the end state directly.** The first thing that ships is the right thing. Interim versions become migrations; we don't ship them.
- **Minimize concept count.** Each architectural concept earns its place by doing one thing the others don't. Naming converges on existing terms (e.g., Trellis's "case ledger") rather than inventing parallel ones.
- **Cargo and Cargo features enforce architectural seams.** Conventions stop being load-bearing; the dep graph is the architecture diagram.

Everything else follows.

---

## II. Stack Composition

Three composable specs, one verifiable artifact:

```
┌──────────────────────────────────────────────────────────────┐
│  Formspec (intake)                                            │
│  Definition + Response + FEL + accessControl (per ADR-0074)   │
│  Field-level access classification declared at source          │
│  Bucketed Response wire format                                 │
└──────────────────────┬───────────────────────────────────────┘
                       │ IntakeHandoff (per ADR 0073)
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  WOS (governance)                                             │
│  Kernel + Governance + AI + Advanced + Signature              │
│  Extends accessControl taxonomy with wos.* class namespace    │
│  Emits wos.governance events into the case ledger             │
└──────────────────────┬───────────────────────────────────────┘
                       │ wos.governance events
                       ▼
┌──────────────────────────────────────────────────────────────┐
│  Trellis (integrity substrate — we ship)                      │
│  COSE_Sign1 + dCBOR + Merkle + checkpoint + export package    │
│  Per-class DEK key bag; encrypt-then-hash; client decrypt     │
│  trellis-core, trellis-cose, trellis-store-postgres,          │
│  trellis-verify, trellis-export — our crates                  │
└──────────────────────────────────────────────────────────────┘
```

**Trellis is our work.** The byte protocol, the Phase-1 envelope invariants, and the Rust reference implementation are commitments we ship — not third-party dependencies we wait on. The wos-server EventStore composes Trellis crates we author.

**Restate is the production runtime adapter.** Our in-memory `wos-runtime` is the test and conformance oracle. WASM-compiled `wos-runtime` runs in browsers for client-side guard evaluation. Same Rust source, three adapter targets, shared conformance fixtures.

---

## III. Trust Postures

Three deployment modes describe the **property** the deployment commits to, not the implementation. Declared per deployment, structurally enforced (per Trellis Phase-1 invariant #15: trust posture honesty floor). Same architecture, different configuration.

| Posture | What the deployment commits to | Procurement target |
|---|---|---|
| **SBA** | Platform may decrypt for explicit, audited purposes; plaintext never persists at rest; every decryption is a KMS-logged event | Small agencies, nonprofits |
| **Federal** | Platform cannot reconstruct plaintext outside an attested or math-bound boundary | FedRAMP-Moderate+; HIPAA-regulated; rights-impacting |
| **Sovereign** | As Federal, plus respondent's content uses client-origin keys (no platform-side custody for respondent-self class) | EU eIDAS 2.0; civil-liberties contexts |

**Confidential compute is pluggable, not architecturally fixed.** Today's reference adapter is `processing-audited` — the simpler posture sufficient for SBA mode, and a viable interim for Federal mode under stricter KMS / role-separation policy. Future siblings under the same `ProcessingService` port:

- **`processing-tee`** — TEE-attested processing (AWS Nitro Enclaves, Intel SGX, Confidential VMs). Hardware-rooted confidentiality with attestation chain.
- **`processing-fhe`** — Fully Homomorphic Encryption. Math-rooted confidentiality; computation on ciphertext without decryption. Production-ready today for narrow operations (predicate evaluation, simple aggregates), maturing for broader workloads.
- **`processing-mpc`** — Multi-Party Computation. No single party holds plaintext; computation is distributed across non-colluding services.

TEE and FHE are peer **future options for true zero trust**. Our job is to build structures that admit them when they ship, not to depend on any specific one. Federal mode today commits to the property; the adapter that *delivers* the property evolves.

This stack is **data-and-workflow zero trust** layered on conventional **identity-and-network zero trust**. NIST SP 800-207, CISA ZTMM v2.0 Data pillar, OMB M-22-09, and FedRAMP rev5 cross-reference cleanly.

---

## IV. The Architecture, in Three Stories

### Story 1 — Data governance is structural

**Trellis is the canonical event log.** One Postgres database per tenant; two schemas:

- **`canonical`** — Trellis-shaped events: hash-chained, signed (COSE_Sign1), payloads encrypted per access class. Immutable. Append-only. The legal-grade artifact. Trellis verifiers read only this schema.
- **`projections`** — derived metadata views. Mutable. Rebuildable from events by replay. SQL-friendly. Application-evolvable. **Plaintext content NEVER lives here** — only metadata (IDs, states, tags, timestamps, counts, opaque references).

**Each event payload is a key-bagged set of access-class buckets.** Field-level classification is declared in the Formspec Definition (per ADR-0074 `accessControl`). Each bucket has its own DEK. Each DEK is wrapped to the recipients authorized for that class. Crypto-shredding is GDPR Art. 17's structural mechanism: destroy the key, the bound content becomes irrecoverable, the chain stays intact.

**Clients decrypt; servers broker.** The server returns ciphertext events plus the requesting user's wrapped key-bag entries. The client unwraps DEKs using its authenticator (WebAuthn PRF for respondents, hardware token / PIV / CAC / YubiKey for staff, OIDC-mediated wrapped key for non-government staff) and decrypts in browser memory. Server never holds plaintext content.

**Two-layer access control, structurally enforced:**

1. **OpenFGA** (Zanzibar-style ReBAC) decides metadata access AND per-class decryption authority. Reference model ships with `case`, `task`, `evidence`, `attachment` entities; `applicant`, `caseworker`, `supervisor`, `auditor`, `equity_service`, `medical_caseworker`, `financial_caseworker` user types; `can_list`, `can_decrypt_class:<class>` relations.
2. **Key-bag membership** is the cryptographic enforcement of OpenFGA's authority. OpenFGA grants → server releases the wrapped DEK → client decrypts. OpenFGA denies → no DEK released → client cannot decrypt.

OpenFGA misconfig leaks metadata, not content. Stolen key reveals one class for one recipient, not the case.

### Story 2 — Adapters are plural; the seam is at the port

Each capability is a port (a trait in `wos-server-ports`) with multiple concrete adapter crates. Cargo features at the composition root select which adapters ship. The reference server admits all three trust postures via configuration.

| Capability | Port | Adapters |
|---|---|---|
| Canonical event log + projections | `EventStore` | `eventstore-postgres`, `eventstore-sqlite` (composed with our `trellis-store-*` crates) |
| Encrypted blob storage (evidence attachments, content-addressed) | `BlobStore` | `blobstore-s3`, `blobstore-azure`, `blobstore-gcs`, `blobstore-fs` |
| Durable execution runtime | `RuntimeOps` + `SeamAccess` + `TimerCoord` (layered) | `runtime-restate` (production), `runtime-local` (test/conformance oracle) |
| Authorization (relationship-based access control) | `AuthzService` | `authz-openfga`, `authz-spicedb`, `authz-mock` |
| Identity (respondent and staff) | `AuthProvider` | `identity-webauthn` (respondent), `identity-oidc` (staff multi-provider), `identity-mock` |
| Key management | `KmsAdapter` | `kms-vault`, `kms-cloud` (AWS / GCP / Azure / GovCloud), `kms-local` |
| Automated processing (validation, agent inference, aggregation) | `ProcessingService` | `processing-audited` (today's reference; KMS-logged server-side decryption for explicit purposes) — port admits `processing-tee` / `processing-fhe` / `processing-mpc` as future siblings for stronger zero-trust postures (see §III) |
| Observability | (not a port — direct) | `wos-server-otel` (OTLP exporter) |
| Trellis export packaging | (not a port — direct) | `wos-server-trellis-export` |

**Adding a new capability means adding a new adapter crate, not editing existing ones.** A future Temporal runtime, Cedar authz, OpenTimestamps anchor — each lands as a sibling crate that satisfies the same port.

### Story 3 — Engineering discipline keeps debt low

- **Audit ⊥ observability.** Trellis events answer "who/what/why" for the regulator; OpenTelemetry answers "what failed" for the operator. Distinct concerns, distinct substrates, distinct verifiers. Conflating produces both bad audit and bad observability.
- **Verifier independence is structural.** The `canonical` schema is read-only at the deployment role level. Trellis verifiers MUST NOT depend on workflow runtime, mutable databases, or derived artifacts (Trellis Core §16). Projections are explicitly application state.
- **Crypto is fenced.** Following ADR-0074's `formspec-bucketing` precedent, the wos-server workspace adopts a `CRYPTO_OWNER` fence in `scripts/check-dep-fences.mjs` (or wos-server's equivalent). Only the crates that *must* perform cryptographic operations may import crypto libraries: `eventstore-postgres` (envelope encryption), `kms-*` (key wrapping/release), `identity-webauthn` (PRF derivation), and any `processing-*` adapter that performs decryption or attested computation (`processing-audited`, future `processing-tee` / `processing-fhe` / `processing-mpc`). HTTP handlers, services, runtime adapters, and the composition root MUST NOT import crypto directly. The dep graph is the security boundary.
- **Cargo features select trust posture.** The composition root declares per-mode default-feature sets (SBA / Federal / Sovereign); CI ratchets prove `cargo check -p wos-server --no-default-features` compiles against ports only.
- **Conformance fixtures pass against all runtime adapters.** Three-way agreement (spec + in-memory `runtime-local` + production `runtime-restate`) is the verification posture; conformance is non-negotiable.

---

## V. Cross-Spec Changes

The architecture spans three specs. Each owns one fact; nobody redefines below their layer.

### Formspec — `accessControl` extension (per ADR-0074)

Authoritative spec is [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md). Summary of the cross-stack-relevant points:

- **`accessControl` is a normative item property** on `field` and `group` items. Nested shape: `{ class, audience?, lawfulBasis?, cardinalityRationale? }`.
- **Class names are opaque to Core.** The taxonomy is registry-tier infrastructure (`specs/registry/access-class-registry.md`). Core treats class tokens as opaque strings; presence of a token activates routing, not its semantics.
- **Privacy Profile sidecar** is the per-deployment policy layer. It defines audience lists per class, lawful-basis declarations, class overrides, and `flClassCompatibility` declarations. Optional; no Profile loaded → flat Response, identical to pre-version-bump Formspec.
- **Bucketed Response wire shape** when a Privacy Profile is loaded. Each event payload is a set of class-bucketed ciphertexts plus a key bag of per-class wrapped DEKs.
- **Sensitivity ordering is audience-subset-defined.** Class A is more sensitive than class B iff `audience(A) ⊊ audience(B)`. Profile-loaded only.
- **Cross-class FEL is a definition error at Core**, relaxable only via Profile `flClassCompatibility` with literal audience-set equality (verified at both lint time and processor load time).
- **Phase 5 (Emission)** projects flat Instance into per-class plaintext, encrypts per bucket, wraps DEKs. Pure projection-and-encrypt; no FEL evaluation; no Instance mutation.
- **Mapping spec** gains `reclassification` requirement: class-crossing Field Rules require explicit `targetClass` + `rationale` + optional `reviewer`, validated at mapping-document load time.

### Case Ledger (replaces Respondent Ledger spec)

Trellis Core §1.2 already defines the **case ledger** as one of three nested append-only scopes: a hash-chained sequence of governance events composing one or more sealed response-ledger heads with WOS governance events into one adjudicatory matter.

The existing `specs/audit/respondent-ledger-spec.md` is renamed and rewritten as `specs/audit/case-ledger-spec.md`. Adopting Trellis's existing term avoids parallel naming.

Changes:

- **Re-scope** from `responseId`-keyed to `caseId`-keyed. A case may have many responses across years (initial, RFI, amendment, appeal); the ledger spans them.
- **Extend event taxonomy** with `wos.*` governance events (~25 types from ADR-0059 §4: `wos.transition.fired`, `wos.task.created/claimed/completed`, `wos.governance.evaluated`, `wos.deontic.evaluated`, `wos.delegation.verified`, `wos.review.protocol`, `wos.hold.entered/resumed`, `wos.provenance.reasoning/counterfactual/narrative`, `wos.explanation.assembled`, `wos.appeal.filed`, `wos.agent.invoked/fallback`, `wos.drift.detected`, `wos.autonomy.changed`, `wos.equity.alert`, `wos.timer.created/fired/cancelled`); add `case.created` (per ADR 0073); add lifecycle events (`ledger.checkpoint`, `ledger.archived`, `ledger.key.destroyed`, `ledger.redacted`, `ledger.exported`, `ledger.sealed`).
- **Per-field classification inherits from `accessControl.class`** in the originating Formspec definition. WOS governance events also classify their fields per the `wos.*` namespace registered in the Access-Class Registry.
- **Encrypt-then-hash is normative.**
- **Header tag policy is explicit** per Trellis Phase-1 invariant #9: family-level `event_type` plaintext (e.g., `wos.transition.fired`); specific tags, outcome values, actor identities live in the encrypted payload.

WOS authors own `wos.*` event-type definitions in `wos-spec/specs/audit/wos-event-types.md` (new); Formspec authors own `formspec.*` and `respondent.*`.

### Trellis Phase 1 — we ship it

Per Trellis Core, the Phase-1 envelope is a normative byte commitment. We author the Rust reference implementation (`trellis-core`, `trellis-cose`, `trellis-store-postgres`, `trellis-store-memory`, `trellis-verify`, `trellis-cli`, `trellis-conformance`) as part of our build sequence — not as a third-party dependency. The 15 Phase-1 envelope invariants (dCBOR canonicalization, `suite_id` registry, signing-key registry in export, hash-over-ciphertext, ordering model, registry-snapshot binding, key-bag immutability, redaction-aware commitment slots, plaintext-vs-committed header policy, Phase 1 envelope IS Phase 3 case-ledger event format, namespace deconfliction, head-format superset, append idempotency, snapshots/watermarks, trust-posture honesty floor) are the engineering commitments we honor.

The wos-server `EventStore` composes `trellis-store-postgres` (canonical events table) with the projections-management layer.

---

## VI. Crate Structure

```
crates/
├── wos-server-ports                # Trait crate (foundation)
│                                   #   EventStore, BlobStore, ProcessingService
│                                   #   AuthProvider, KmsAdapter, AuthzService
│                                   #   RuntimeOps, SeamAccess, TimerCoord
│
├── wos-server-eventstore-postgres  # Production EventStore (composes trellis-store-postgres)
├── wos-server-eventstore-sqlite    # Dev / tests / single-tenant on-prem
│
├── wos-server-blobstore-s3         # S3-compatible (AWS / MinIO / GovCloud)
├── wos-server-blobstore-azure      # Azure Blob Storage
├── wos-server-blobstore-gcs        # Google Cloud Storage
├── wos-server-blobstore-fs         # Filesystem (dev / on-prem)
│
├── wos-server-runtime-restate      # Production runtime
├── wos-server-runtime-local        # Test / conformance oracle
│
├── wos-server-authz-openfga        # Default authorization
├── wos-server-authz-spicedb        # Alternate
├── wos-server-authz-mock           # Test / dev
│
├── wos-server-identity-webauthn    # Respondent identity (WebAuthn PRF + DID)
├── wos-server-identity-oidc        # Staff identity (OIDC + VC); multi-provider
├── wos-server-identity-mock        # Test / dev
│
├── wos-server-kms-vault            # HashiCorp Vault
├── wos-server-kms-cloud            # Cloud KMS (AWS / GCP / Azure / GovCloud)
├── wos-server-kms-local            # Local key file (test / dev only)
│
├── wos-server-processing-audited   # Today's reference: KMS-logged audited decryption
│                                   #   Future siblings under the same ProcessingService port:
│                                   #     wos-server-processing-tee   (TEE-attested; hardware-rooted)
│                                   #     wos-server-processing-fhe   (FHE; math-rooted)
│                                   #     wos-server-processing-mpc   (MPC; distributed)
│
├── wos-server-otel                 # OpenTelemetry observability
├── wos-server-trellis-export       # Trellis export packages (Trellis Core §18)
│
└── wos-server                      # Composition root (Axum HTTP, services, ServerConfig)
```

Per-deployment-mode default features (today's shippable configuration; future-adapter columns reflect what becomes available as TEE / FHE / MPC adapters land):

| Mode | Today (shippable) | Future-target |
|---|---|---|
| **SBA** | `eventstore-postgres + blobstore-{fs,s3} + runtime-restate + authz-openfga + identity-{oidc,webauthn} + kms-vault + processing-audited + otel` | unchanged (SBA's commitment is met by audited-decryption posture) |
| **Federal** | same as SBA + `kms-cloud` (replaces `kms-vault`) + stricter KMS / role-separation policy | swap `processing-audited` → `processing-tee` when TEE adapter ships; selectively `processing-fhe` for FHE-eligible operations |
| **Sovereign** | same as Federal + client-origin sovereign respondent flows on Studio side | same future-adapter swaps; sovereignty axis (respondent client-origin keys) is identity-side, independent of `ProcessingService` selection |

CI ratchets enforce abstraction:

- `cargo check -p wos-server --no-default-features` proves the composition root needs no concrete adapter to compile (only ports).
- `CRYPTO_OWNER` fence (mirroring ADR-0074's `formspec-bucketing` pattern) keeps crypto imports scoped to the four crates that need them.
- `WASM_OWNER` fence (existing pattern from `formspec-engine`) keeps WASM imports scoped to the runtime crates that compile to WASM.

---

## VII. Consumer Topology

The architecture serves multiple consumer shapes, not just browsers. Each is the same architecture with a different decryption surface.

| Consumer | Decryption surface | Mechanism |
|---|---|---|
| **Browser-driven reads** (caseworker, applicant, respondent self-service) | Browser memory, WebCrypto API | Server returns ciphertext + wrapped key-bag entries; client unwraps DEKs via WebAuthn PRF (respondent) or hardware token / KMS-mediated wrapped key (staff); decrypts authorized classes in browser; never persists plaintext locally beyond session |
| **CLI tools** (operator scripts, debugging, batch operations) | CLI process memory | OIDC-authenticated CLI receives wrapped key-bag entries; PIV/CAC card or KMS-fetched wrapped key resident in the CLI session; decrypts in process; plaintext lifetime bounded by session |
| **Mobile clients** (caseworker on tablet, fieldwork) | Mobile webview / native app | WebAuthn + WebCrypto in mobile webview; native apps use platform-native equivalent (iOS Secure Enclave + CryptoKit; Android StrongBox + Tink) |
| **M2M / API integrations** (state-to-federal data sharing, FAFSA → state grants, child support → tax intercept) | Receiving system | Sender returns bucketed Response with key bag wrapping a DEK to the receiving system's identity (registered as a per-class recipient in the Privacy Profile); receiver decrypts authorized classes; explicit `access.granted` ledger event records the cross-system handoff |
| **Batch processing** (overnight re-eligibility runs, equity audits, SLA breach detection) | `ProcessingService` adapter selected at deployment (`processing-audited` today; `processing-tee` / `processing-fhe` / `processing-mpc` as future siblings) | Background job dispatches to the selected adapter; per-purpose KMS authorization releases the appropriate keys (or, for FHE/MPC adapters, computation runs without key release); output (aggregate or per-case decision) is itself emitted as a signed ledger event with the adapter-specific verification surface (KMS log entry, TEE attestation, FHE proof) |
| **Analytics / equity monitoring** (federal disparate-impact reporting, OMB M-24-10) | Analytics service identity | Analytics service is a registered recipient with key-bag entries for `demographic` class; computes aggregates over decrypted demographic dimensions; published aggregate is a `wos.equity.report` ledger event signed by the analytics service |
| **Trellis export** (FOIA, litigation discovery, auditor) | Recipient device | Standard Trellis export package per Core §18 — self-contained, machine-readable, verifiable on an air-gapped laptop with `trellis-cli verify` |

The wire format is the same across all surfaces: Trellis envelopes with bucketed payloads and key bags. The decryption mechanism varies; the architecture does not.

---

## VIII. Build Sequence

A single dependency DAG. No phases. Tracks run in parallel where dependencies permit.

```
                        ┌──────────────────────────────────┐
                        │  Trellis Phase 1                 │
                        │  trellis-core, trellis-cose,     │
                        │  trellis-store-postgres,         │
                        │  trellis-store-memory,           │
                        │  trellis-verify, trellis-cli     │
                        │  (we ship)                       │
                        └────────────────┬─────────────────┘
                                         │
                                         ▼
                        ┌──────────────────────────────────┐
                        │  wos-server-ports (trait crate)  │
                        │  Composes Trellis abstractions   │
                        └────────────────┬─────────────────┘
                                         │
   ┌──────────────┬──────────────┬───────┴──────┬──────────────┬──────────────┐
   ▼              ▼              ▼              ▼              ▼              ▼
   eventstore     blobstore      runtime        authz          identity       kms /
   × 2            × 4            × 2            × 3            × 3            processing
                                                                              × 2

   Cross-spec parallel tracks:
     - ADR-0074 Formspec spec edits + access-class-registry + privacy-profile
     - Case Ledger spec rewrite + WOS event-type definitions
     - WASM compile of wos-runtime (in wos-runtime crate)
     - Studio-side: client-side decryption, per-class rendering, WebAuthn registration UI
                                         │
                                         ▼
                        ┌──────────────────────────────────┐
                        │  wos-server (composition root)   │
                        │  Axum HTTP, services, configs    │
                        └──────────────────────────────────┘
```

**Trellis Phase 1 ships first** because the EventStore composes its crates. This is our work, on our build track — not an external dependency.

**Adapter cluster runs in parallel** after the foundation closes. Each adapter is independent of the others. CI ratchets prove the composition root compiles against ports only.

**Cross-spec work runs in parallel** with implementation. Spec edits don't gate adapter work; adapters target the trait surface in `wos-server-ports`.

**Composition root closes last.** Wires the adapters together; finalizes ServerConfig, OTEL configuration, default-feature sets, CI ratchet declarations.

---

## IX. What We Reject

| Anti-pattern | Reason rejected |
|---|---|
| Phase 1 / Phase 2 / Phase 3 sequencing as developer-time economy | Wrong economic model — calendar time, architectural debt, conceptual debt are scarce; tokens aren't |
| Two-store split (`Storage` + separate `AuditSink` ports) | Trellis IS the database; one EventStore port covers both |
| Parallel hash chains (WOS-internal `previous_hash` alongside Trellis chain) | One chain — Trellis. Postgres WAL covers torn writes; replay determinism is Trellis chain semantics |
| Plaintext at rest in operational store | Metadata-only projections; plaintext lives only in encrypted events + transient confidential-compute memory (audited process today; TEE / FHE / MPC adapter as future stronger postures) |
| Server-side plaintext outside the declared `ProcessingService` boundary | Strict modes (Federal/Sovereign) commit to "platform cannot reconstruct plaintext outside an attested or math-bound boundary." Today's reference adapter is `processing-audited` with KMS-logged events; future TEE / FHE / MPC adapters tighten the boundary. The trust posture declares the property; the adapter delivers it. |
| Hardcoding TEE as the architectural pillar | TEE is one confidential-compute strategy among several (peer to FHE, MPC). The architecture admits all via a pluggable `ProcessingService` port; no specific adapter is load-bearing for the architecture's correctness. Deployments select; the spec doesn't dictate. |
| Application-layer dual-write to multiple stores | Outbox / event-sourcing pattern; never dual-write |
| Server decrypts content for routine reads | Clients decrypt; server brokers wrapped DEKs |
| Bespoke per-actor scoping in handlers | OpenFGA service handles relationship-based access |
| Event-level encryption only (one DEK per event) | Per-class encryption — granular by access class within an event |
| AI-authored "Locked narrative" treated as architectural authority | Treat as input; evaluate substance independently |
| In-memory storage as production posture | Test / conformance oracle only; production is Postgres |
| JSONFS / "datalake-as-Storage" interpretations | Operational ≠ analytical; conflating them is the design mistake |
| Per-field DEKs (one DEK per field, not per class) | Per-class DEKs are right granularity; per-field is key explosion |
| "Subject Ledger" as a parallel name for Trellis's "case ledger" | Adopt Trellis's term; one canonical name per concept |
| Crypto distributed across the codebase | CRYPTO_OWNER fence concentrates crypto in adapter crates that need it; the dep graph is the security boundary |

---

## X. Open Decisions

Four resolved by ADR-0074:

| Item | Status |
|---|---|
| Formspec extension home | **Resolved.** Core spec edit + two companion specs (Access-Class Registry + Privacy Profile) per ADR-0074. |
| Default-class strictness | **Resolved.** Schema-optional with `unclassified` default; lint enforces under Profile-loaded conformance; explicit `unclassified` distinguished from schema-omitted. |
| Pre-allocated namespaces | **Resolved.** ADR specifies `wos.*`, `hipaa.*`, `ferpa.*`, `itar.*` as registry namespaces. |
| Studio scope | **Resolved.** Per ADR-0074 §13: Theme `access.*` token names normative, visual implementation-defined; class-affordances UI is Studio work. |

Six remaining (with first-order leans, deserve real evaluation before committing):

1. **Anchor substrate adapter set.** Lean: ship OpenTimestamps + Sigstore Rekor + Trillian as siblings via Trellis's `AnchorAdapter` trait; default to Rekor for development.
2. **Authz adapter default.** Lean: OpenFGA as reference impl, SpiceDB as sibling for enterprise procurement that demands it; Cedar as a third future sibling for analyzable-policy use cases.
3. **WASM-compile track ownership.** Lean: lives in `wos-runtime` crate (not `wos-server`); pattern follows `formspec-engine`. Affects Studio more than wos-server.
4. **wos-server existing TODO disposition.** Lean: discard and replace with fresh `BACKLOG.md` keyed to end-state crate cluster.
5. **PARITY.md disposition.** Lean: rewrite in place once the new specs (ADR-0074 + Case Ledger + WOS event types) exist.
6. **Confidential-compute adapter sequencing.** When does each future `ProcessingService` adapter ship? Lean: `processing-audited` is today's reference, sufficient for SBA and interim-Federal; `processing-tee` ships when a deployment requires hardware-rooted attestation (likely first FedRAMP-High customer); `processing-fhe` ships when a workload's operations are FHE-tractable and the customer values math-rooted confidentiality over hardware-rooted; `processing-mpc` ships when a multi-operator deployment surfaces. Trigger-driven, not calendar-driven; the architecture's commitment is admitting them via the port surface, not pre-shipping them.

---

## XI. Compliance / Positioning

| Framework | Mapping |
|---|---|
| NIST SP 800-207 (Zero Trust Architecture) | All seven tenets satisfied; "all data sources and computing services are considered resources" structurally enforced by per-class encryption + key-bag access |
| CISA ZTMM v2.0 — Data pillar | Maturity Level 4–5 (Optimal): customer-managed keys, granular per-resource access, encrypted in transit + at rest |
| OMB M-22-09 (Federal Zero Trust Strategy) | Data-layer requirements satisfied via per-class DEKs + KMS audit + ledger integrity |
| OMB M-24-10 (AI in Federal Government) | Agent governance via WOS deontic constraints + Trellis attestation; per-class agent access boundaries |
| FedRAMP rev5 | Federal-mode commitment ("platform cannot reconstruct plaintext outside an attested or math-bound boundary") + customer-managed keys + audit log integrity. Today's adapter is `processing-audited` with stricter KMS / role separation; future TEE / FHE / MPC adapters tighten the boundary. |
| GDPR Art. 17 (right to erasure) | Crypto-shredding via key destruction; structurally provable |
| GDPR Art. 20 (data portability) | Trellis export package self-contained, machine-readable, verifiable on air-gapped laptop |
| HIPAA | Per-class encryption isolates PHI; medical class access restricted by key bag |
| FRE 803(6) (business records exception) | Systematic, contemporaneous, attributed, routine, tamper-evident — by construction |
| Title VI / disparate-impact analysis | Demographic class accessible to equity service; aggregates published as signed ledger events. Verification surface depends on the deployed `ProcessingService` adapter — KMS log entry (`processing-audited`), TEE attestation (`processing-tee`), or FHE proof (`processing-fhe`). |
| NIST SP 800-63 (Digital Identity Guidelines) | IAL2/IAL3 supported via OIDC + Verifiable Credentials |

**Positioning:**

> Zero-trust workflow governance for high-stakes public-sector adjudication: the server can't read your data, you can prove who saw which fields, the operator can't quietly rewrite history, and a 2045 verifier can prove on an air-gapped laptop that the 2026 record is genuine. Open spec, Rust + WASM, federal-conformant.

---

## XII. Authoritative References

| Concern | Authoritative spec |
|---|---|
| Formspec field-level access classification, bucketed Response, per-class encryption mechanics | [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md) |
| Case ledger event taxonomy and wire format | `specs/audit/case-ledger-spec.md` (rewrite of `respondent-ledger-spec.md`) |
| WOS governance event-type definitions | `wos-spec/specs/audit/wos-event-types.md` (new) |
| Access-class taxonomy + lint rules | `specs/registry/access-class-registry.md` (new, per ADR-0074) |
| Per-deployment audience policy | `specs/privacy/privacy-profile.md` (new, per ADR-0074) |
| Trellis byte protocol (envelope, hash construction, signing, export) | `trellis/specs/trellis-core.md` |
| Trellis operational discipline (projections, watermarks, snapshots) | `trellis/specs/trellis-operational-companion.md` |
| Case-creation boundary (Formspec → WOS handoff) | [ADR 0073](../../../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md) |
| Stack evidence integrity (attachment binding) | [ADR 0072](../../../thoughts/adr/0072-stack-evidence-integrity-and-attachment-binding.md) |
| Selective disclosure (BBS+) for FOIA / cross-agency export | ADR-0080 (follow-on) |

---

## XIII. Provenance

This vision is the synthesis of an architectural conversation (2026-04-25) that progressed by owner correction through several drafts. The shape that survived:

- **Operating frame correction.** Phasing as developer-hours economy was rejected; tokens are unlimited; calendar time and architectural/conceptual debt are scarce; elegance is the optimization target.
- **AI-authored "locked narrative" framing rejected** as authority. Substance evaluated independently.
- **Two-store outbox collapsed to single EventStore** when the coordination problem dissolved under "Trellis IS the database."
- **Server-side decryption rejected for routine reads** in favor of client-side decryption with KMS-mediated key brokerage.
- **Per-class encryption** chosen over event-level encryption when "not every person needs to see every field" became architecturally explicit.
- **Field-level classification placed in Formspec** because Formspec is the source of truth for what fields are. ADR-0074 owns the spec specifics; this document references.
- **Trellis framed as our work**, not as an external substrate dependency. We ship the Phase-1 envelope, the Rust reference implementation, and the storage adapters. The wos-server EventStore composes them.
- **"Subject Ledger" naming dropped in favor of Trellis's existing "case ledger" term** to keep the conceptual surface minimal.
- **CRYPTO_OWNER fence pattern adopted** from ADR-0074's `formspec-bucketing` precedent. The dep graph is the security boundary.
- **Calendar-time and "implementation realism" scaffolding dropped.** Architectural debt and conceptual debt are the real costs; build the elegant end state.

The architecture that resulted is data-and-workflow zero trust on top of conventional identity-and-network zero trust — emergent from honest engagement with each architectural question, not engineered toward as a goal.
