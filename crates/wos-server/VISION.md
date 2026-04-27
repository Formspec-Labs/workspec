# WOS Server — Architectural Vision

**Status:** Architectural commitment, 2026-04-25. Target architecture; not an inventory of crates already split from the current server.
**Authoritative spec for Formspec changes:** [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md).
**Authoritative spec for Trellis byte protocol:** [`trellis/specs/trellis-core.md`](../../../trellis/specs/trellis-core.md).

This document owns the cross-spec architectural framing for the WOS Server reference implementation and its place in the Formspec / WOS / Trellis stack. It defers Formspec-side specification to ADR-0074 and Trellis-side specification to `trellis-core.md`. It captures the wos-server commitments and the engineering discipline that makes them coherent.

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

**Confidential compute is pluggable, not architecturally fixed.** The `processing-audited` adapter is the SBA reference: explicit server-side decryption, KMS authorization, and ledgered purpose. It does not satisfy Federal or Sovereign claims by itself. Stronger siblings under the same `ProcessingService` port supply those claims:

- **`processing-tee`** — TEE-attested processing (AWS Nitro Enclaves, Intel SGX, Confidential VMs). Hardware-rooted confidentiality with attestation chain.
- **`processing-fhe`** — Fully Homomorphic Encryption. Math-rooted confidentiality; computation on ciphertext without decryption. Tractable for narrow operations (predicate evaluation, simple aggregates), maturing for broader workloads.
- **`processing-mpc`** — Multi-Party Computation. No single party holds plaintext; computation is distributed across non-colluding services.

TEE, FHE, and MPC are peer options for stronger processing confidentiality. The architecture admits all three without making any one of them load-bearing. Federal and Sovereign deployments must not claim "platform cannot reconstruct plaintext outside an attested or math-bound boundary" until the selected `ProcessingService` adapter actually delivers that property.

This stack is **data-and-workflow zero trust** layered on conventional **identity-and-network zero trust**. NIST SP 800-207, CISA ZTMM v2.0 Data pillar, OMB M-22-09, and FedRAMP rev5 cross-reference cleanly.

---

## IV. The Architecture, in Three Stories

### Story 1 — Data governance is structural

**Trellis is the canonical event log.** One Postgres database per tenant; two schemas:

- **`canonical`** — Trellis-shaped events: hash-chained, signed (COSE_Sign1), payloads encrypted per access class. Immutable. Append-only. The legal-grade artifact. Trellis verifiers read only this schema.
- **`projections`** — derived metadata views. Mutable. Rebuildable from events by replay. SQL-friendly. Application-evolvable. **Plaintext content NEVER lives here** — only metadata (IDs, states, tags, timestamps, counts, opaque references).

**Each event payload is a key-bagged set of access-class buckets.** Field-level classification is declared in the Formspec Definition (per ADR-0074 `accessControl`). Each bucket has its own DEK. Each DEK is wrapped to the recipients authorized for that class. Crypto-shredding is GDPR Art. 17's structural mechanism: destroy the key, the bound content becomes irrecoverable, the chain stays intact.

**Clients decrypt; servers broker.** The server returns ciphertext events plus the requesting user's wrapped key-bag entries. The client unwraps DEKs using its authenticator (WebAuthn PRF for respondents, hardware token / PIV / CAC / YubiKey for staff, OIDC-mediated wrapped key for non-government staff) and decrypts in browser memory. Routine reads never give the server plaintext content.

**Two-layer access control on data, structurally enforced:**

1. **OpenFGA** (Zanzibar-style ReBAC) decides metadata access AND per-class decryption authority. Reference model ships with `case`, `task`, `evidence`, `attachment` entities; `applicant`, `caseworker`, `supervisor`, `auditor`, `equity_service`, `medical_caseworker`, `financial_caseworker` user types; `can_list`, `can_decrypt_class:<class>` relations.
2. **Key-bag membership** is the cryptographic enforcement of OpenFGA's authority. OpenFGA grants → server releases the wrapped DEK → client decrypts. OpenFGA denies → no DEK released → client cannot decrypt.

OpenFGA misconfiguration alone cannot decrypt a class for an identity absent from the key bag. It can leak metadata, and it can release any wrapped class key the misconfigured identity already has. Key-bag issuance therefore remains governed and ledgered. A stolen recipient key reveals only the classes and scopes wrapped to that recipient, not the whole case.

**Decision authority is an orthogonal axis.** Data access (above) and decision authority are distinct concerns. An identity may hold an OpenFGA grant AND a key-bag entry for a class — and still be denied authority to act on the decrypted content by a WOS deontic constraint or impact-tier autonomy cap (AI Integration §S4–§S5). Crypto/FGA say "you can decrypt"; WOS says "in this case state, with this autonomy posture, you may or may not act." The three concerns fail independently.

**Recipient rotation across multi-year cases.** Trellis Phase-1 invariant #7 (key-bag immutability) applies per event, not per case. Recipient turnover (caseworker leaves, agency reorganizes) is handled by emitting subsequent events with key bags scoped to current recipients; superseded recipients remain in the chain (chain integrity preserved) but no new content is wrapped to them. Departed-recipient revocation is a governance event (named in `wos-event-types.md`); subsequent events MUST NOT wrap to revoked recipients. Historical decryption capability for already-emitted events is by design — a former caseworker who legitimately accessed an event in 2026 cannot have that access "un-granted" in 2030. Crypto-shredding via class-DEK destruction remains the mechanism for irrecoverability. Recipient revocation is a Privacy Profile concern; `lawfulBasis` (per-class, not per-recipient per ADR-0074 §1) carries no parallel retraction obligation.

### Story 2 — Adapters are plural; the seam is at the port

Each capability is a port (a trait in `wos-server-ports`) with multiple concrete adapter crates. Cargo features at the composition root select which adapters ship. The target reference server admits all three trust postures via configuration.

| Capability | Port | Adapters |
|---|---|---|
| Canonical event log + projections | `EventStore` | `eventstore-postgres`, `eventstore-sqlite` (composed with our `trellis-store-*` crates) |
| Encrypted blob storage (evidence attachments, content-addressed) | `BlobStore` | `blobstore-s3`, `blobstore-azure`, `blobstore-gcs`, `blobstore-fs` |
| Durable execution runtime | `RuntimeOps` + `SeamAccess` + `TimerCoord` (layered) | `runtime-restate` (production), `runtime-local` (test/conformance oracle) |
| Authorization (relationship-based access control) | `AuthzService` | `authz-openfga`, `authz-spicedb`, `authz-mock` |
| Identity (respondent and staff) | `AuthProvider` | `identity-webauthn` (respondent), `identity-oidc` (staff multi-provider), `identity-mock` |
| Key management | `KmsAdapter` | `kms-vault`, `kms-cloud` (AWS / GCP / Azure / GovCloud), `kms-local` |
| Automated processing (validation, agent inference, aggregation) | `ProcessingService` | `processing-audited` (SBA reference; KMS-logged server-side decryption for explicit purposes), plus stronger siblings `processing-tee`, `processing-fhe`, and `processing-mpc` for Federal / Sovereign claims (see §III) |
| Observability | (not a port — direct) | `wos-server-otel` (OTLP exporter) |
| Trellis export packaging | (not a port — direct) | `wos-server-trellis-export` |

**Adding a new capability means adding a new adapter crate, not editing existing ones.** A future Temporal runtime, Cedar authz, OpenTimestamps anchor — each lands as a sibling crate that satisfies the same port.

### Story 3 — Engineering discipline keeps debt low

- **Audit ⊥ observability.** Trellis events answer "who/what/why" for the regulator; OpenTelemetry answers "what failed" for the operator. Distinct concerns, distinct substrates, distinct verifiers. Conflating produces both bad audit and bad observability.
- **Verifier independence is structural.** The `canonical` schema is read-only at the deployment role level. Trellis verifiers MUST NOT depend on workflow runtime, mutable databases, or derived artifacts (Trellis Core §16). Projections are explicitly application state.
- **Crypto is fenced.** Following ADR-0074's `formspec-bucketing` precedent, the wos-server workspace adopts a `CRYPTO_OWNER` fence in `scripts/check-dep-fences.mjs` (or wos-server's equivalent). Only the crates that *must* perform cryptographic operations may import crypto libraries: `eventstore-postgres` (envelope encryption), `kms-*` (key wrapping/release), `identity-webauthn` (PRF derivation), and any `processing-*` adapter that performs decryption or attested computation (`processing-audited`, future `processing-tee` / `processing-fhe` / `processing-mpc`). HTTP handlers, services, runtime adapters, and the composition root MUST NOT import crypto directly. The dep graph is the security boundary.
- **Cargo features select trust posture.** The composition root declares per-mode feature bundles (SBA / Federal / Sovereign); CI ratchets prove `cargo check -p wos-server --no-default-features` compiles against ports only.
- **Conformance fixtures pass against all runtime adapters.** Three-way agreement (spec + in-memory `runtime-local` + production `runtime-restate`) is the verification posture; conformance is non-negotiable.

**Specific invariants the architecture protects** (wos-server-unique cases not covered by Trellis Core or ADR-0074):

- **Projection rebuild against evolved Privacy Profile version.** Per ADR-0074 §3/§10, `profileUrl` + `profileVersion` are bound into AAD per event. Projection rebuilds read events at their pinned Profile version; a current-version Profile is never silently substituted.
- **Deontic prohibition firing after key bag was wrapped.** A `prohibition` evaluated true after content was already wrapped to a recipient does NOT retract the wrapped DEK; the recipient retains historical decryption capability for already-emitted events (see Story 1 recipient rotation). The prohibition gates *new* events, not past wraps.
- **KMS unavailability during decryption (`processing-audited`).** Returns explicit `kms.unavailable`; never falls back to a plaintext path; the access attempt is ledgered.
- **TEE attestation failure mid-batch (Federal mode).** Batch aborts; partial results discarded; the boundary is recorded as a signed event.

Trellis Core §16 (verifier independence) and the Phase-1 invariants cover corrupted key bag, chain verification failure, and key-destruction race; ADR-0074 §6/§11 cover cross-class FEL and non-relevant-field bucket emission. Cross-reference, do not duplicate.

---

## V. Cross-Spec Bindings

This vision binds three companion specs; each owns its content in full. VISION.md states the binding only and does not restate normative semantics.

- **Formspec** — [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md) is authoritative for `accessControl` semantics, the Privacy Profile sidecar, the bucketed Response wire shape, sensitivity ordering, Phase-5 emission, and the cross-class FEL definition error. Two callouts that load-bear on the WOS layer: (1) `flClassCompatibility` (ADR-0074 §7) is the only mechanism that relaxes cross-class FEL across `wos.*` + `formspec.*` namespaces — WOS guard authoring inherits this constraint, enforced at both lint time and processor load time; (2) the schema-omitted vs. explicit `unclassified` distinction is lint-relevant (ADR-0074 §1, §12) — only schema omission fires `every-field-classified`. Implementations MUST preserve both states distinctly.

- **Respondent Ledger** — `specs/audit/respondent-ledger-spec.md` §7.7 (draft v0.2.0) already inverts inheritance: each `ChangeSetEntry` derives `accessClass` from the source field's `accessControl.class` when a Privacy Profile is loaded, and raw values for a class MUST NOT be exposed to a reader who lacks authority. Distinct identifiers — source = `accessControl.class` (ADR-0074 §1, on the item); derived = `accessClass` (ledger, on `ChangeSetEntry`). The broader class-aware redaction surface (groups, repeats, calculated fields, non-relevant fields) remains forward work per ADR-0074 §9.

- **Case Ledger composition + WOS event taxonomy** — Trellis Core §1.2 already defines the case ledger as composed sealed response-ledger heads + WOS governance events into one adjudicatory matter. WOS event-type definitions (including the recipient-revocation event referenced in §IV Story 1) live in the planned `wos-spec/specs/audit/wos-event-types.md`. This vision asserts the *requirements* (one chain per case; family-level `event_type` plaintext per Trellis Phase-1 invariant #9; specific tags / outcomes / actor identities encrypted in payload; encrypt-then-hash normative; recipient-revocation event in the taxonomy) without minting names. WOS authors own the `wos.*` namespace; Formspec authors own `formspec.*` and `respondent.*`.

- **Custody seam** — the binding from `wos.*` event-type tags into Trellis envelope tags goes through the kernel `extensions` seam (`wos-spec/specs/kernel/spec.md` §10.6) and the `custodyHook` seam (`wos-spec/specs/kernel/custody-hook-encoding.md`): one authored WOS record per append, dCBOR-canonicalized, ingested into the Trellis chain. The wos-server `EventStore` composes `trellis-store-postgres` + projections through this seam.

- **Trellis** — `trellis/specs/trellis-core.md` is authoritative for envelope format, hash construction, signing, export, and the 15 Phase-1 envelope invariants (we ship the Rust reference implementation per §II).

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
├── wos-server-processing-audited   # SBA reference: KMS-logged audited decryption
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

Target feature bundles. These describe the architecture to build, not the crates currently present in the monolithic server:

| Mode | Minimum claimable bundle | Stronger-processing path |
|---|---|---|
| **SBA** | `eventstore-postgres + blobstore-{fs,s3} + runtime-restate + authz-openfga + identity-{oidc,webauthn} + kms-vault + processing-audited + otel` | Optional; SBA's stated commitment is met by audited-decryption posture |
| **Federal** | `eventstore-postgres + blobstore-{s3,azure,gcs} + runtime-restate + authz-openfga + identity-{oidc,webauthn} + kms-cloud + processing-tee` or another attested / math-bound adapter | Add `processing-fhe` for FHE-eligible operations; add `processing-mpc` when no single operator may hold plaintext |
| **Sovereign** | Federal bundle + client-origin respondent keys for respondent-self classes | Same stronger-processing choices; sovereignty is primarily an identity and key-custody axis, not a separate event format |

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
| **Batch processing** (overnight re-eligibility runs, equity audits, SLA breach detection) | `ProcessingService` adapter selected at deployment (`processing-audited` for SBA; `processing-tee` / `processing-fhe` / `processing-mpc` for stricter modes) | Background job dispatches to the selected adapter; per-purpose KMS authorization releases the appropriate keys (or, for FHE/MPC adapters, computation runs without key release); output (aggregate or per-case decision) is itself emitted as a signed ledger event with the adapter-specific verification surface (KMS log entry, TEE attestation, FHE proof) |
| **Analytics / equity monitoring** (federal disparate-impact reporting, OMB M-24-10) | Analytics service identity | Analytics service is a registered recipient with key-bag entries for `demographic` class; computes aggregates over decrypted demographic dimensions. Reporting wire format (cadence vs. per-report event vs. threshold-breach event) is deferred to `wos-spec/specs/advanced/equity-config.md` and the planned `wos-event-types.md`; this vision does not mint event names |
| **Trellis export** (FOIA, litigation discovery, auditor) | Recipient device | Standard Trellis export package per Core §18 — self-contained, machine-readable, verifiable on an air-gapped laptop with `trellis-cli verify` |

The wire format is the same across all surfaces: Trellis envelopes with bucketed payloads and key bags. The decryption mechanism varies; the architecture does not.

---

## VIII. Build Sequence

A single dependency DAG. **Dependency-ordered sequencing, not calendar phasing** — the foundation must close before adapters can compile against it; tracks within a layer run fully parallel.

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
   eventstore     blobstore      runtime        authz          identity       kms / processing
   × 2            × 4            × 2            × 3            × 3            × 3 / × 4

   Cross-spec parallel tracks:
     - ADR-0074 Formspec spec edits + access-class-registry + privacy-profile
     - Respondent Ledger class-aware edits + Case Ledger composition + WOS event-type definitions
     - WASM compile of wos-runtime (in wos-runtime crate)
     - Studio-side: client-side decryption, per-class rendering, WebAuthn registration UI
                                         │
                                         ▼
                        ┌──────────────────────────────────┐
                        │  wos-server (composition root)   │
                        │  Axum HTTP, services, configs    │
                        └──────────────────────────────────┘
```

**Trellis Phase 1 ships first** because the EventStore composes its crates (per §II — our work, our build track).

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
| Plaintext at rest in operational store | Metadata-only projections; plaintext lives only in encrypted events + transient `ProcessingService` memory (`processing-audited` for SBA; TEE / FHE / MPC adapters for stricter modes) |
| Server-side plaintext outside the declared `ProcessingService` boundary | Strict modes (Federal/Sovereign) commit to "platform cannot reconstruct plaintext outside an attested or math-bound boundary." `processing-audited` is SBA-grade; Federal and Sovereign claims require TEE / FHE / MPC or an equivalent adapter that actually delivers the boundary. |
| Hardcoding TEE as the architectural pillar | TEE is one confidential-compute strategy among several (peer to FHE, MPC). The architecture admits all via a pluggable `ProcessingService` port; no specific adapter is load-bearing for the architecture's correctness. Deployments select; the spec doesn't dictate. |
| Application-layer dual-write to multiple stores | Outbox / event-sourcing pattern; never dual-write |
| Server decrypts content for routine reads | Clients decrypt; server brokers wrapped DEKs |
| Bespoke per-actor scoping in handlers | OpenFGA service handles relationship-based access |
| Event-level encryption only (one DEK per event) | Per-class encryption — granular by access class within an event |
| AI-authored "Locked narrative" treated as architectural authority | Treat as input; evaluate substance independently |
| In-memory storage as production posture | Test / conformance oracle only; production is Postgres |
| Treating the operational EventStore as a generic datalake or JSON object store | The `canonical` schema is a Trellis-shaped artifact with specific Phase-1 invariants and verifier-independence requirements (Trellis Core §16); conflating it with general-purpose blob storage breaks both. Operational ≠ analytical |
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

Three additional commitments (resolved from earlier "leans"):

1. **Anchor substrate adapter set.** Ship OpenTimestamps + Sigstore Rekor + Trillian as siblings via Trellis's `AnchorAdapter` trait; default to Rekor in dev. Production default is per-deployment.
2. **Authz adapter default.** OpenFGA as reference impl; SpiceDB as sibling for procurement that demands it. Cedar moves to roadmap (analyzable-policy use cases), not opens.
3. **WASM-compile track ownership.** Lives in `wos-runtime` crate; pattern follows `formspec-engine`. Affects Studio more than wos-server.

Genuinely open:

4. **Confidential-compute adapter sequencing.** Trigger-driven, not calendar-driven. Each adapter ships when a deployment surfaces it: `processing-tee` when hardware-rooted attestation is required (FedRAMP-Moderate+ workloads); `processing-fhe` when a workload's operations are FHE-tractable AND the customer values math-rooted confidentiality over hardware-rooted; `processing-mpc` when a multi-operator deployment surfaces with no single party permitted to hold plaintext. Architectural commitment: admit all three via the `ProcessingService` port; do not pre-ship.

(`wos-server` TODO/PARITY disposition is task hygiene tracked in the project board, not an architectural decision; not listed here.)

---

## XI. Architectural Constraints from Compliance

The architecture's shape is constrained by a small number of frameworks whose requirements are mechanism-level (not procurement positioning). These are kept here because they explain *why* the architecture has its current shape:

| Framework | What it constrains |
|---|---|
| NIST SP 800-207 (Zero Trust Architecture) | All seven tenets satisfied structurally by per-class encryption + key-bag access; "all data sources and computing services are considered resources" enforced by the data path, not by network policy |
| GDPR Art. 17 (right to erasure) | Crypto-shredding via class-DEK destruction is the structural mechanism; chain integrity preserved (the bound content becomes irrecoverable, the chain stays intact) |
| GDPR Art. 20 (data portability) | Trellis export package self-contained, machine-readable, verifiable on air-gapped laptop per Trellis Core §18 |
| HIPAA | Per-class encryption isolates PHI; medical class access restricted by key bag (mechanism, not procurement) |
| FRE 803(6) (business records exception) | Systematic, contemporaneous, attributed, routine, tamper-evident — by construction of the canonical schema + chain |

**Procurement-facing compliance framework mapping** (FedRAMP rev5, OMB M-22-09, OMB M-24-10, CISA ZTMM v2.0, Title VI / disparate-impact, NIST SP 800-63, etc.) lives in [`STACK.md`](../../../STACK.md) §Proof packages, not here. That mapping is buyer-facing and does not constrain wos-server architecture.

**Positioning:**

> Zero-trust workflow governance for high-stakes public-sector adjudication: routine server reads do not expose case content, every field disclosure is key-gated and ledgered, the operator cannot quietly rewrite history, and a 2045 verifier can prove on an air-gapped laptop that the 2026 record is genuine. Open spec, Rust + WASM, federal-oriented.

---

## XII. Authoritative References

| Concern | Authoritative spec |
|---|---|
| Formspec field-level access classification, bucketed Response, per-class encryption mechanics | [ADR-0074](../../../thoughts/adr/0074-formspec-native-field-level-transparency.md) |
| Formspec response-scoped respondent history, including optional field-level editing changelog | `specs/audit/respondent-ledger-spec.md` |
| Case ledger composition and wire format | `trellis/specs/trellis-core.md` §22 (current) plus planned cross-stack case-ledger binding if split out |
| WOS governance event-type definitions | `wos-spec/specs/audit/wos-event-types.md` (planned) |
| Access-class taxonomy + lint rules | `specs/registry/access-class-registry.md` (planned per ADR-0074) |
| Per-deployment audience policy | `specs/privacy/privacy-profile.md` (planned per ADR-0074) |
| Trellis byte protocol (envelope, hash construction, signing, export) | `trellis/specs/trellis-core.md` |
| Trellis operational discipline (projections, watermarks, snapshots) | `trellis/specs/trellis-operational-companion.md` |
| Case-creation boundary (Formspec → WOS handoff) | [ADR 0073](../../../thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md) |
| Stack evidence integrity (attachment binding) | [ADR 0072](../../../thoughts/adr/0072-stack-evidence-integrity-and-attachment-binding.md) |
| Selective disclosure (BBS+) for FOIA / cross-agency export | ADR-0081 (planned follow-on) |

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
