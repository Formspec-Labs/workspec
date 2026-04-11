# Enterprise Feature Gaps

**Date:** 2026-04-07
**Context:** Gaps between the proposed Formspec platform and enterprise competitors (Adobe AEM Forms, DocuSign, Adobe Sign, Form.io, ServiceNow) that are not yet addressed by the Formspec spec suite or existing implementations.
**Benchmarks:** DocuSign for agreement/signature workflows; Adobe AEM Forms for enterprise forms, document generation, and government-scale intake; ServiceNow for workflow/case management, integration ecosystem, and enterprise platform maturity.

> **Reading this document:** Many items listed as "gaps" have partial or full coverage in the Formspec specification suite (specs/) or existing implementations, even if no SaaS platform exists yet. Items marked with 📋 📋 **[SPEC]** have normative spec coverage. Items marked with ✅ ✅ **[IMPL]** have working implementation. Items marked with ✅ ✅ **[SPEC+IMPL]** have both. Unmarked items are 🆕 genuine gaps.

### Status Badges

| Badge | Meaning |
|-------|---------|
| ✅ | **Built** — working implementation exists in the engine |
| 📋 | **Spec-complete** — normative spec exists, needs implementation |
| 🔶 | **Partial** — spec + some impl, needs SaaS layer |
| 🆕 | **Genuine gap** — no spec or implementation |
| 🏛️ | **Certification / procurement gate** — calendar-bound |
| ⭐ | **Differentiator** — no competitor equivalent |

### Gap Domain → Roadmap Cross-Reference

| # | Domain | Worst Gap | Coverage | Roadmap Phase |
|---|--------|-----------|----------|---------------|
| 1 | Signatures | 🟡 Critical only if competing | 📋 1 spec · ✅ 1 built · 🆕 4 | P4.4 (DocuSign/Adobe Sign integration) |
| 2 | Identity Verification | 🔴 Critical for gov | 📋 3 spec · 🆕 4 | P4.3 (identity expansion) |
| 3 | Form & Intake Runtime | ✅ Mostly resolved | ✅ 5 built/partial · 📋 1 spec | P1.2, P2.2, P2.3, P5.1 |
| 4 | Document & Evidence | 🔴 Critical (storage) | 🆕 all genuine gaps | P1.4, P3.2 |
| 5 | Case & Workflow | 🔴 Critical (largest gap) | 📋 3 spec · 🆕 9 | P1.3, P3.4, P4.5 |
| 6 | Integration Ecosystem | 🟠 High | 📋 1 spec · ✅ 1 built · 🆕 9 | P4.4, P6.3 |
| 7 | Crypto & Audit | 📋 Spec-complete | 📋 4 spec-complete · 🆕 5 | P2.1, P2.4 |
| 8 | Compliance & Procurement | 🔴 Critical (×4) | 🏛️ all calendar-gated | P2.3, P2.5, P4.1, P4.2 |
| 9 | AI Governance | 🟠 High | 📋 2 spec · 🔶 1 partial · 🆕 3 | P3.1, P3.3 ⭐ |
| 10 | Deployment & Infra | 🟠 High | 🆕 all genuine gaps | P1.6, P2.6 |
| 11 | Collaboration | 🟡 Medium | 🆕 all genuine gaps | P6.1 |
| 12 | Reporting & Output | 🟡 Medium | 📋 1 spec · 🔶 2 partial · 🆕 5 | P3.5, P5.2, P6.2 |
| 13 | Market & Ecosystem | 🟠 High | 🆕 all genuine gaps | P4.2, P6.3 |

---

## 1. 🟡 Legal Signature and Agreement Finality

Formspec is an intake/case platform, not an agreement execution tool. But many government intake flows end with a signature — benefits applications, permit approvals, attestations. Without this, customers need a second tool for the last mile.

| Gap | Severity | Notes |
|---|---|---|
| Legally binding electronic signatures (ESIGN Act, UETA, eIDAS) | Critical if competing head-to-head | DocuSign's core product; Adobe Sign is the primary alternative. Both accepted in courts worldwide |
| Signature ceremony UX (sign here, initial here, date fields) | High | Table stakes for any agreement workflow; Adobe Sign and DocuSign both mature here |
| Witness and counter-signature workflows | Medium | Required for notarization, multi-party agreements |
| Remote Online Notarization (RON) | Low | Niche but growing in gov/legal |
| Signature audit certificate (court-admissible evidence) | High | 📋 **[SPEC]** Respondent Ledger S13 defines LedgerCheckpoint with `batchHash`, `previousCheckpointHash`, `signature`, `keyId`, and `anchorRef` for external notarization — the *evidence infrastructure* for court-admissible records exists, but no signing ceremony or legal attestation workflow |
| Wet signature capture (draw/type/upload) | ~~Medium~~ **Covered** | ✅ **[SPEC+IMPL]** Signature component (Component S6.8) captures drawn signatures as image attachments. Props: `strokeColor`, `height`, `penWidth`, `clearable`. Falls back to FileUpload. Not legally binding, but the capture UX exists. |

**Strategic question:** Build a lightweight attestation/acknowledgment feature, or treat DocuSign/Adobe Sign as a first-class integration target? The Ledger's identity attestation (DID, verifiable credentials, delegated access — Ledger S6.6) and checkpoint signing (Ledger S13) provide the evidence infrastructure that an e-signature integration would need. Note: Adobe bundles Sign with AEM Forms as a single platform sale — competing head-to-head on signatures is unnecessary if the integration story is strong.

---

## 2. 🔴 Identity Verification and Proofing

The Respondent Ledger spec (969 lines) provides a full provider-neutral identity attestation model — not just a sketch. Ledger S6.6 defines DID, verifiable credentials, OpenID Connect, ID.me, proof-of-personhood, delegated access, assurance levels, and selective disclosure profiles. Three deployment profiles (Ledger S15A): local/server (no identity), pseudonymous integrity-anchored, and identity-bound high-assurance for regulated workflows. Four privacy tiers: anonymous, pseudonymous, identified, fully attributable (Ledger S6.7). **The spec is comprehensive; no implementation exists.**

| Gap | Severity | Notes |
|---|---|---|
| Government identity proofing (ID.me, Login.gov, NIST IAL2) | Critical for gov | 📋 **[SPEC]** Ledger S6.6 explicitly names ID.me, models assurance levels, and defines adapter boundaries for provider-specific payloads. Implementation needed. |
| Knowledge-based authentication (KBA) | Medium | DocuSign offers this as signer verification |
| Phone/SMS verification | Medium | Common second-factor for respondent identity |
| Biometric verification | Low | Emerging, not standard |
| ID document scanning and verification | Medium | DocuSign ID Verification scans licenses, passports |
| Proof of personhood (beyond CAPTCHA) | Medium | 📋 **[SPEC]** Ledger S6.6 explicitly models proof-of-personhood as an identity attestation type. Not conceptual — normatively specified. |
| Delegated authority / power of attorney flows | Medium | 📋 **[SPEC]** Ledger S6.6 models delegated access with 5 actor kinds: `respondent`, `delegate`, `system`, `support-agent`, `unknown` (Ledger S6.4). |

---

## 3. ✅ Form and Intake Runtime

| Gap | Severity | Notes |
|---|---|---|
| Offline / intermittent connectivity support | ✅ ~~High~~ **Covered** | ✅ **[SPEC+IMPL]** Core architecture, not a feature flag. Rust/WASM kernel evaluates locally — validation, FEL calculations, conditional logic, repeat groups, page navigation all work offline. iOS Swift/SwiftUI renderer shipped. |
| Native mobile apps (iOS/Android) | 🔶 ~~High~~ **Partial** | ✅ **[IMPL]** iOS/SwiftUI shipped (WebView bridge). Android/Compose architecture finalized (ADR accepted). UniFFI roadmap replaces WebView with direct Rust FFI. |
| Kiosk mode | Medium | In product roadmap; not implemented |
| Call-center / staff-assisted intake mode | Medium | Agent fills form on behalf of caller; in product roadmap, not implemented |
| Pre-fill from external data sources at runtime | Medium | 📋 **[SPEC]** Assist spec S6 defines profile matching with ontology-based concept identity, confidence scoring (0.95 exact → 0.30 fallback), and mandatory user confirmation. Mapping DSL exists but no live connector to hydrate forms at load time. |
| Respondent-facing conversational completion | 🔶 ~~Medium~~ **Partial** | ✅ **[SPEC+IMPL]** Assist spec defines 15 tools across 4 categories (introspection, mutation, profile, navigation) for form-filling interoperability. Transport-agnostic: WebMCP, MCP, postMessage, HTTP. Formy browser extension prototyping. formspec-chat exists but is authoring-focused. |
| Multi-language / localization | 📋 ~~Critical~~ **Spec-complete** | 📋 **[SPEC]** Full Locale spec (1253 lines): BCP 47, CLDR plural forms via `pluralCategory()`, FEL interpolation `{{expr}}` in strings, `@context` suffixes (`@accessibility`, `@pdf`, `@short`), 4-step cascade fallback, cross-tier keys (`$page.*`, `$component.*`), 3 FEL functions (`locale()`, `formatNumber()`, `formatDate()`), version compatibility scoping, 2 conformance levels. Implementation needed for SaaS runtime; EO 13166 still requires it. Adobe AEM Forms has production localization with translation workflows — mature here. |
| Accessibility compliance (WCAG 2.1 AA / Section 508) | 🔶 ~~Critical~~ **Partial** | ✅ **[SPEC+IMPL]** Component spec defines `AccessibilityBlock` (role, description, liveRegion) on every component. Per-component ARIA mandates: Select (`aria-expanded`, `aria-controls`, `aria-activedescendant`), Modal (`role="dialog"`, `aria-modal`, focus trap), Alert (`role="alert"`/`"status"`), ProgressBar (`aria-valuenow/min/max`). Theme S9.2: WCAG 2.2 AA guidance (4.5:1 contrast, font minimums). Locale `@accessibility` context suffix. USWDS adapter shipped. React `useField` returns pre-built ARIA `inputProps`. **Still needs:** formal WCAG audit, VPAT, screen reader testing. |
| PDF form rendering / print-optimized output | 🔶 ~~High~~ **Partial** | ✅ **[SPEC+IMPL]** Locale `@pdf` context suffix enables PDF-specific label variants. Theme sidecar exists as hints. Prototype PDF form generation module on a branch — not production-ready. Adobe's XDP/XFA PDF rendering is the industry benchmark here. |
| Form branching preview / "what if" mode for respondents | Low | Some competitors allow respondents to preview paths |
| Payment collection within forms | 🔶 ~~Medium~~ **Partial** | ✅ **[SPEC+IMPL]** First-class `money` data type (Core S3.4.1) with `{ amount, currency }` as string-serialized decimal for precision. Base-10 arithmetic (`0.1 + 0.2 = 0.3`). ISO 4217 currency codes. 6 money FEL functions (`money()`, `moneyAmount()`, `moneyCurrency()`, `moneyAdd()`, `moneySum()`, `moneySumWhere()`). MoneyInput component (Component S6.5). `defaultCurrency` form-level setting. **Not covered:** payment processing, PCI-DSS, Stripe/payment gateway integration. |
| Appointment / scheduling within forms | Low | Common in healthcare and government intake |

---

## 4. 🔴 Document and Evidence Handling

| Gap | Severity | Notes |
|---|---|---|
| Server-side document storage | Critical | File upload component exists client-side only; no storage backend |
| Document preview (PDF, image, office docs) | High | No attachment preview per requirements matrix |
| Document type classification (AI-powered) | High | In product roadmap Phase 2; not implemented. Adobe has Sensei-powered document classification in AEM. |
| Document data extraction (OCR, structured extraction from PDFs) | High | Core thesis but no extraction pipeline exists. Adobe has Sensei-powered extraction; AEM Forms can pre-fill from uploaded PDFs. |
| Evidence packet assembly | Medium | ADR-0009 describes evidence bundles; nothing implemented |
| Document redaction tools | Medium | In product roadmap; not implemented |
| Virus/malware scanning of uploads | High | ADR-0009 describes file security lifecycle; nothing built |
| Bulk document upload | Medium | Common need for application packets |
| Document versioning (replace with audit trail) | Medium | Respondent Ledger spec tracks this; not implemented |
| E-fax / email-to-document ingestion | Low | Government agencies still receive faxes |

---

## 5. ~~🔴~~ 🟠 Case and Workflow Management

No case management **platform layer** exists. However, the WOS (Workflow Orchestration Standard) spec suite — 18 specs, 18 schemas, 189 lint rules developed in `wos-spec/` — provides comprehensive governance and workflow semantics that cover most of the post-submission workflow requirements at the specification level. The Formspec spec suite provides pre-submission routing. The gap is **SaaS implementation and UI**, not spec design.

> **WOS context (added 2026-04-10):** The WOS spec suite was developed in parallel with this gap analysis. Many items listed below as 🆕 gaps have since been addressed by WOS specifications. See `WOS-FEATURE-MATRIX.md` for the full competitive comparison. The critical remaining gap is the **Formspec Coprocessor** — the handoff protocol between Formspec form submissions and WOS workflow instances (see `wos-spec/TODO.md`).

| Gap | Severity | Notes |
|---|---|---|
| Case object and lifecycle | 📋 ~~Critical~~ **WOS spec-complete** | 📋 **[WOS SPEC]** WOS Kernel S4-S5 defines lifecycle topology (4 state types, transitions, guards, fork/join, milestones), case state model (typed fields, append-only mutation history), and CaseInstance serialization (Runtime S3). WOS Governance S10 defines 8-state task lifecycle, 5 assignment roles, SLA with 4 breach policies. **Implementation gap:** no SaaS case management layer exists. **Architecture gap:** Formspec Coprocessor handoff protocol (how a submission becomes a WOS case) is not yet specified. |
| Reviewer dashboard | Critical | 🆕 No reviewer UI or dashboard. This is a SaaS/UI concern, not a spec gap — WOS provides the data model; the dashboard is implementation. |
| Routing rules (auto-assign based on criteria) | 📋 ~~High~~ **Dual spec-complete** | 📋 **[SPEC]** Pre-submission: Screener spec (1508 lines) with 3 strategies and Determination Records. Post-submission: 📋 **[WOS SPEC]** WOS Governance S10.2 defines 5 assignment roles (owner, nominee, potentialOwner pool, businessAdministrator, excludedOwner). WOS tag-based governance (S4.3) attaches routing rules to semantic categories. **Implementation gap:** no routing engine or assignment UI. |
| Request for more information (RFI) workflow | 📋 ~~High~~ **WOS partially covered** | 📋 **[WOS SPEC]** WOS Governance S12 defines typed hold policies including `pending-applicant-response` with expected duration, resume trigger, and timeout action. Correspondence Metadata and Notification Template sidecars exist for structured notices. **Implementation gap:** the end-to-end RFI sequence (reviewer requests → respondent notified → respondent amends → reviewer sees update) requires SaaS workflow + the Formspec Coprocessor handoff. |
| Approval chains (sequential and parallel) | 📋 ~~High~~ **WOS primitives exist** | 📋 **[WOS SPEC]** WOS Kernel S4.3-4.8 provides compound states, parallel states with completion policies (wait-all, cancel-siblings, fail-fast), and deterministic transition evaluation. Governance S4 defines 5 review protocols. Governance S11 defines delegation of authority with legal instrument references. **Implementation gap:** no pre-built approval chain templates; workflow authors must compose these from WOS primitives. |
| SLA timers and escalation | 📋 ~~Medium~~ **WOS spec-complete** | 📋 **[WOS SPEC]** WOS Governance S10.3 defines task SLA with `targetDuration`, `warningThreshold`, and 4 breach policies (escalate, reassign, notify, extend). Business Calendar sidecar defines business-day computation. Kernel S9.7 defines 5 timeout categories. **Implementation gap only.** |
| Case notes and internal comments | Medium | 🆕 Genuine UI gap. WOS provenance `log` actions with extensions could model annotations, but "case notes" as a first-class feature is not specified. |
| Bulk triage / batch operations | Medium | 🆕 Genuine gap. WOS TODO explicitly: "2A. Batch Operations — Rejected — Implementation concern." |
| Assignment and workload balancing | 🔶 ~~Medium~~ **WOS partially covered** | 📋 **[WOS SPEC]** WOS Governance S10.2 defines potentialOwner pools (first-claim-wins). **Genuine gap:** workload *balancing* algorithms (even distribution) are implementation-defined, not specified. |
| Post-submission status notifications to respondents | 📋 ~~High~~ **WOS sidecar exists** | 📋 **[WOS SPEC]** WOS Notification Template sidecar defines structured notice templates for hold/adverse/appeal notices. Correspondence Metadata sidecar defines notice delivery records. **Implementation gap:** notification delivery mechanism (email/SMS) is SaaS infrastructure. |
| Amendment / resubmission workflow | 🔶 ~~High~~ **Partial** | 📋 **[SPEC]** Core S6.3 defines response status lifecycle: `in-progress` → `completed` → `amended` → `stopped`. `response.amendment-opened` and `response.amended` are normative Ledger event types (Ledger S8.1). The status model and audit trail exist; amendment UI/workflow is not built. |
| Decision recording with rationale | 📋 ~~High~~ **Dual spec-complete** | 📋 **[SPEC]** Screener Determination Records (Screener S8) for pre-submission. 📋 **[WOS SPEC]** WOS Governance S6 defines Reasoning tier (rules applied, evidence consulted, criteria checked) and Counterfactual tier (what would change the outcome). Authority ranking: statute > regulation > policy > guideline. **Implementation gap only.** |
| Appeal workflow | 📋 ~~Medium~~ **WOS spec-complete** | 📋 **[WOS SPEC]** WOS Governance S3.5-3.6 defines appeal requirements: independent adjudicator, appeal provenance, continuation of service during appeal. Due process S3.2-3.4 defines mandatory notice, individualized explanation, and counterfactual explanation for adverse decisions in rights-impacting workflows. **Implementation gap:** specific appeal workflow topology must be authored using WOS primitives. |
| AI governance for case decisions | 📋 **WOS spec-complete** | 📋 **[WOS SPEC]** WOS AI Integration spec (18 specs total) defines deontic constraints, 4 autonomy levels with impact-level caps, confidence framework with decay, mandatory fallback chains, drift detection, agent disclosure, and Narrative provenance tier. No competitor has equivalent AI governance. See `WOS-FEATURE-MATRIX.md` Section 5. **Implementation gap only.** |
| Review quality controls | 📋 **WOS spec-complete** | 📋 **[WOS SPEC]** WOS Governance S4 defines 5 review protocols (independent-first, consider-opposite, calibrated confidence, dual-blind, unassisted). S7 defines quality sampling, separation of duties, and structured override authority. No competitor has equivalent review protocols. **Implementation gap only.** |

**What the specs provide for workflow integration:** In addition to the Formspec integration surfaces (Shape rules, per-shape timing, external validation injection, extension properties), the **WOS spec suite provides the full governance framework**: lifecycle topology, case state model, task management, review protocols, due process, delegation of authority, hold policies, structured audit, AI governance, data validation pipelines, temporal parameter resolution, and formal verification. The SaaS platform should implement WOS governance specs — not design a parallel governance system.

**Critical dependency: Formspec Coprocessor.** The handoff protocol between Formspec form submissions and WOS case instances is not yet specified (`wos-spec/TODO.md` "Formspec Coprocessor gap"). This is the single most important spec to write before building the case management SaaS layer. Without it, the platform must invent its own submission-to-case bridge, risking divergence from WOS semantics.

**Revised competitive position:** With WOS, the case management gap changes from "no spec, no design, must build everything" to "comprehensive governance spec exists, must implement SaaS layer + UI + Coprocessor handoff." The spec-level gap vs. ServiceNow and Adobe is much narrower than this document originally assessed — WOS governance (review protocols, due process, AI governance, structured audit) exceeds what ServiceNow or Adobe offer. The implementation gap remains significant.

**ServiceNow context:** ServiceNow's case management maturity is the strongest of any competitor in scope. Its workflow engine supports ITSM, CSM, HR Service Delivery, and custom process apps. However, ServiceNow's intake surfaces (catalog items, record producers) are static field lists designed for internal requestors, not adaptive public-facing intake. The competitive question is whether to build a workflow engine or to treat ServiceNow as an integration target for post-submission processing. WOS governance specs provide a third option: implement WOS as the governance layer with ServiceNow as one possible execution substrate (see `WOS-FEATURE-MATRIX.md` Section 17).

---

## 6. 🟠 Integration Ecosystem

The Formspec spec suite provides strong integration *primitives* — Mapping DSL, transport bindings, extension registry, ontology alignments — but zero pre-built connectors or platform integrations.

| Gap | Severity | Notes |
|---|---|---|
| Pre-built CRM connectors (Salesforce, HubSpot, Dynamics) | High | DocuSign has 400+; Adobe has Experience Cloud integrations and Form Data Model with REST/SOAP/JDBC. ServiceNow IntegrationHub has 1000+ spokes with bidirectional connectors for Salesforce, SAP, Workday, Jira, and others. Formspec has mapping DSL but zero pre-built connectors. |
| Pre-built government system connectors (SAM.gov, Grants.gov, MAX.gov) | High | Critical for the government wedge |
| SMS notifications | Medium | Common for respondent communication |
| Storage connectors (S3, Azure Blob, GCS) | High | Required for document handling |
| Database connectors (write submissions to external DBs) | Medium | 📋 **[SPEC]** Mapping DSL (2023 lines) defines 10 transform types (preserve, drop, expression, coerce, valueMap, flatten, nest, constant, concat, split), bidirectional semantics with round-trip fidelity, and 3 format adapters (JSON, XML/namespaces, CSV/RFC 4180). Custom adapters via `x-` prefix. Auto-mapping for uncovered fields. No live connectors. |
| LDAP / Active Directory integration | Medium | On-prem identity for Dedicated tier. ServiceNow has mature LDAP/AD integration with scheduled imports, group mapping, and MID Server for on-prem directory traversal. |
| Zapier / Power Automate / Make integrations | Medium | Low-code integration for smaller orgs |
| OAuth2 provider (allow third-party apps) | Medium | Developer platform feature |
| Event streaming (Kafka, EventBridge) | Low | Enterprise integration pattern. ServiceNow supports Kafka via IntegrationHub spokes and has native event management in ITOM. |
| Embedded form hosting infrastructure | Medium | ✅ **[IMPL]** `<formspec-render>` web component is a single JS import. Works on any static host. But no hosted embed service with analytics, versioning, or CDN. |
| Webhook infrastructure | High | Outbound event delivery; in product roadmap Phase 1 but not implemented |

**Spec-level integration surface:** Assist spec defines 4 transport bindings: WebMCP (browser-native via `navigator.modelContext`), MCP (server-mediated), Browser Messaging (`postMessage`), HTTP REST (`GET /formspec/tools`, `POST /formspec/tools/{name}`). Extension Registry (584 lines) provides machine-readable catalog with well-known URL discovery (`/.well-known/formspec-extensions.json`), 7 entry categories, and 4-state lifecycle. Ontology spec adds JSON-LD context fragments for linked data export and cross-system alignments with bidirectional mapping. These are integration *protocols*, not pre-built integrations.

---

## 7. 📋 Cryptographic Trust and Audit Infrastructure

The Respondent Ledger spec (969 lines) provides a complete, normative audit trail specification — not just an ADR sketch. These gaps are about the implementation of the full spec.

| Gap | Severity | Notes |
|---|---|---|
| Cryptographic append-only audit ledger | 📋 ~~Critical~~ **Spec-complete** | 📋 **[SPEC]** Respondent Ledger spec defines: 4 canonical objects (RespondentLedger, Event, ChangeSetEntry, LedgerCheckpoint), 13 required event types (`session.started`, `draft.saved`, `draft.resumed`, `response.completed`, `response.amendment-opened`, `response.amended`, `response.stopped`, `attachment.added/replaced/removed`, `prepopulation.applied`, `system.merge-resolved`, `validation.snapshot-recorded`), 8 optional types, 7 ChangeSet operation types, 7 value class types (Ledger S4–S8). Three deployment profiles: Profile A (local/server, no identity), Profile B (pseudonymous integrity-anchored), Profile C (identity-bound high-assurance). **Implementation needed.** |
| Signed audit checkpoints | 📋 ~~High~~ **Spec-complete** | 📋 **[SPEC]** LedgerCheckpoint (Ledger S13): `batchHash`, `previousCheckpointHash`, `signature`, `keyId`, `algorithm`. Hash chain integrity verification algorithm defined. **Implementation needed.** |
| External integrity anchoring (timestamping authority) | 📋 ~~Medium~~ **Spec-complete** | 📋 **[SPEC]** LedgerCheckpoint `anchorRef` field for external transparency log / notarization anchoring (Ledger S13). **Implementation needed.** |
| Respondent-facing change history / timeline | 📋 ~~High~~ **Spec-complete** | 📋 **[SPEC]** Full event taxonomy with ChangeSetEntry tracking what changed, when, by whom, under what rules, with what validation state. Privacy tiered disclosure (anonymous → fully attributable, Ledger S6.7). Sensitive value minimization (hashed priors, display summaries, redaction policy). **Implementation needed.** |
| Exportable verification packages (proof bundles) | High | ADR-0003, ADR-0016; ledger spec provides the evidence structure, but no packaging/export format defined |
| Support-access logging with JIT approval | High | ADR-0004; Ledger `support-agent` actor kind exists (Ledger S6.4) but no JIT approval protocol |
| Legal hold on retained data | High | ADR-0012; retention policies designed but not implemented, legal hold not addressed |
| Configurable deletion semantics by object class | Medium | ADR-0012; GDPR, records management |
| Tenant data export / portability packaging | High | ADR-0013; designed in detail, not implemented |

---

## 8. 🔴 Compliance Certifications and Procurement

No amount of good architecture substitutes for actual certifications. These are all absent. However, the spec suite provides the *mechanisms* compliance frameworks need: non-relevant field handling for data minimization (Core S5.6, 3 modes: `remove`/`empty`/`keep`), response version pinning (Core S6.4), append-only audit trail with tamper-evident checkpoints (Respondent Ledger), Ontology-based PII tracing (concept URIs on fields → automated PII classification), References with `type: "regulation"` and `rel: "constrains"` for per-field regulatory justification, and Screener S14.1 explicitly referencing HIPAA/FERPA/GDPR for sensitive screening data.

| Gap | Severity | Notes |
|---|---|---|
| FedRAMP Moderate authorization | Critical for gov | ADR-0015 structures the posture; authorization is 12-18 months minimum. Adobe AEM Cloud Service has FedRAMP Moderate. ServiceNow holds FedRAMP High and IL4/IL5 authorization — the strongest compliance posture of any competitor in scope. |
| SOC 2 Type II | Critical for enterprise | No production system to audit yet. Adobe, DocuSign, and ServiceNow all hold SOC 2 Type II. |
| HIPAA compliance (BAA capability) | High | Healthcare-adjacent intake |
| StateRAMP | Medium | Growing requirement for state agencies |
| VPAT / Accessibility conformance report | Critical for gov | Section 508 compliance documentation |
| ISO 27001 | Medium | International enterprise requirement. ServiceNow holds ISO 27001. |
| GSA Schedule / procurement vehicle | Critical for gov sales | Without this, agencies cannot easily buy |
| Security whitepaper / procurement response pack | High | ADR-0015 structures the posture but no customer-facing document |

---

## 9. 🟠 AI Governance and Intelligence ⭐

The spec suite provides substantial knowledge grounding and human-in-the-loop mechanisms. AI usage metering, billing, and BYOK are not yet implemented.

| Gap | Severity | Notes |
|---|---|---|
| Tier-aware AI provider routing | High | ADR-0008; architecture designed, nothing implemented. Adobe has no equivalent — AI features are Adobe-controlled with no customer-facing provider routing. ServiceNow Now Intelligence is similarly ServiceNow-controlled; customers cannot choose underlying AI providers or configure per-tier routing. This is a Formspec differentiator once implemented. |
| No-training / no-retention posture enforcement | High | ADR-0008; critical for government trust. Adobe's Sensei operates under Adobe's data policies; ServiceNow's Now Intelligence operates under ServiceNow's data policies. Neither allows customers to configure provider-level data handling posture. |
| Human review gates for AI-generated decisions | 🔶 ~~High~~ **Partial** | 📋 **[SPEC]** Assist spec S3.5/S4.3: `profile.apply` with `confirm: true` MUST trigger user confirmation. Without a confirmation mechanism, MUST return `x-confirmation-required` error — MUST NOT silently apply. Profile data MUST NOT leave device without consent (Assist S6.3). This covers auto-fill; broader decision gates (screener routing) are the host app's concern. |
| Knowledge grounding from policy documents (RAG) | 📋 ~~High~~ **Spec-complete** | 📋 **[SPEC]** References spec (697 lines) provides: 12 reference types including `vector-store`, `knowledge-base`, `retrieval`, `tool`, `api`; audience tagging (`human`/`agent`/`both`); priority tiers; 8 relationship types (`constrains`, `defines`, `exemplifies`, etc.); URI schemes for AI infra (`vectorstore:{provider}/{collection-id}`, `kb:{provider}/{base-id}`, `formspec-fn:{function-name}`); per-field tool invocation schemas (inline OpenAPI/MCP); 8-step agent context assembly algorithm (References S5.1). Ontology spec (782 lines) adds concept URIs, SKOS equivalences, vocabulary bindings, JSON-LD fragments. **Implementation needed for SaaS.** |
| AI model fallback / degraded mode | Medium | 📋 **[SPEC]** Assist spec is LLM-optional by design — structured help works without AI; conversational layer is additive. Core intake must work without AI. |
| Prompt injection protection | Medium | Security risk for any AI-powered intake |
| AI confidence calibration and monitoring | Medium | 📋 **[SPEC]** Assist profile matching uses explicit confidence scores (0.95 exact → 0.30 fallback) with recommended threshold 0.50 (Assist S6.2). No monitoring pipeline. |

---

## 10. 🟠 Deployment and Infrastructure

No production SaaS infrastructure exists. These gaps are about production-grade operations.

| Gap | Severity | Notes |
|---|---|---|
| Cell-based architecture | High | ADR-0002; control plane / data plane separation designed, not implemented |
| File / object storage infrastructure | High | Required for document handling; not implemented |
| Monitoring, alerting, and observability | High | ADR-0011; not implemented. ServiceNow has mature platform-level observability and also sells ITOM/AIOps as products — monitoring is a core competency. |
| Disaster recovery / backup procedures | High | Enterprise requirement; not implemented. ServiceNow provides automated backups, instance cloning, and DR procedures as part of platform operations. |
| Rate limiting and abuse protection | Medium | Required for any public-facing SaaS |
| CDN / edge caching for form delivery | Medium | Performance at scale |
| SCIM (automated user provisioning) | Medium | In product roadmap; not implemented |
| Environment separation (sandbox / production) | High | Preview exists in studio; no real environment management |

---

## 11. 🟡 Collaboration

Multi-party form completion and inter-org sharing are in the product roadmap but not implemented.

| Gap | Severity | Notes |
|---|---|---|
| Real-time collaborative editing (studio) | Low | Nice-to-have, not critical |
| Comments and annotations on form definitions | Medium | Admin collaboration on form design |
| Review task assignment and tracking | High | Part of the case management gap |

---

## 12. 🟡 Reporting and Output

No data export pipeline exists. The spec suite provides analytics building blocks and multi-format output.

| Gap | Severity | Notes |
|---|---|---|
| Submission analytics (completion rates, drop-off, time-to-complete) | 🔶 ~~High~~ **Partial** | 📋 **[SPEC]** Assist `formspec.form.progress` (Assist S3.2) returns `FormProgress`: `total`, `filled`, `valid`, `required`, `requiredFilled` — real-time completion metrics. Respondent Ledger lifecycle events (`session.started`, `draft.saved`, `response.completed`, `response.stopped`) provide the event stream for drop-off and time-to-complete analysis. Core S8.4 extension example: `"x-analytics": { "trackFocus": true, "trackDuration": true }`. ValidationReport `extensions` envisions performance metrics. **No analytics pipeline or dashboard.** |
| Reviewer throughput / workload metrics | Medium | Operational reporting |
| PDF generation from submissions | 🔶 ~~High~~ **Partial** | ✅ **[SPEC+IMPL]** Locale `@pdf` context suffix for PDF-specific labels. Theme sidecar exists. Prototype PDF form generation module on a branch. Pipeline not production-ready. |
| Summary packets / decision letters | Medium | In product roadmap Phase 3; not implemented. Adobe's Correspondence Management handles batch letter/notice generation at scale — mature here. |
| Narrative generation (grant narratives, compliance reports) | Medium | In product roadmap Phase 4; not implemented. No competitor has AI-native narrative generation; this is a differentiator if built. |
| Scheduled / automated reports | Low | Typically handled by BI tools |
| Custom dashboards / report builder | Low | Typically handled by BI tools |
| Template gallery / starter packs | Medium | In product roadmap; not implemented |

---

## 13. 🟠 Market and Ecosystem Readiness

| Gap | Severity | Notes |
|---|---|---|
| Documentation site (API docs, guides, tutorials) | High | Developer and admin onboarding |
| Customer support infrastructure | High | Ticketing, knowledge base |
| Partner / systems integrator program | Medium | Channel for government delivery |
| Template / solution marketplace | Medium | Accelerate adoption. Adobe has industry solution accelerators and reference templates. |
| Community and ecosystem development | Medium | Open-source core is a structural advantage here — Adobe's AEM ecosystem is proprietary and partner-gated. |
| Case studies and reference customers | High | Government buyers need social proof. Adobe has IRS-scale references and 15+ years of deployed history. |

---

## Gap Severity Summary

| | Severity | Count | Domains | Roadmap |
|---|---|---|---|---|
| 🔴 | **Critical** (blocks first gov sale) | ~4 | FedRAMP, SOC 2, GSA Schedule, reviewer dashboard (UI), document storage | P1.4, P2.3, P2.5, P4.1, P4.2 |
| 📋 | **Spec-complete** (specified, needs implementation) | ~16 | Localization, cryptographic audit ledger, signed checkpoints, external anchoring, respondent change history, knowledge grounding/RAG, routing (pre/post-submission), **case lifecycle (WOS)**, **task management (WOS)**, **SLA/escalation (WOS)**, **review protocols (WOS)**, **due process/appeal (WOS)**, **delegation (WOS)**, **AI governance (WOS)**, **structured audit (WOS)**, Formspec Coprocessor handoff (not yet specified) | P1.3, P2.1, P2.2, P3.1, P3.4 |
| 🟠 | **High** (blocks enterprise expansion) | ~20 | Identity proofing impl, PDF pipeline, document handling, RFI workflow, approval chains, connectors, monitoring, analytics pipeline, webhooks | P1.3, P2.4, P2.6, P3.2, P3.5, P4.3, P4.4, P4.5 |
| 🔶 | **Partial** (spec + some impl, needs SaaS layer) | ~8 | Accessibility (needs audit/VPAT), offline (shipped), mobile (iOS shipped), payment data capture, amendment lifecycle, analytics metrics, human review gates, AI confidence | P2.3, P3.3 |
| 🟡 | **Medium** (competitive parity) | ~20 | Kiosk mode, SMS, event streaming, comments, narrative generation, partner program | P5.1, P6.1, P6.2, P6.3 |
| ⚪ | **Low** (acceptable gaps) | ~8 | Notarization, e-fax, real-time collab, scheduling, biometric, custom dashboards | Deferred |

---

## Strategic Observations

### The spec suite is a genuine competitive moat — if implemented

The gap analysis reveals a pattern: many items listed as "enterprise gaps" are actually spec-complete but unimplemented. The Respondent Ledger (969 lines), References (697 lines), Ontology (782 lines), Locale (1253 lines), Screener (1508 lines), Mapping (2023 lines), and Assist specs collectively represent thousands of lines of normative specification covering audit trails, knowledge grounding, localization, routing, integration transforms, and AI-assisted filling. No competitor has this level of *specified* behavior — DocuSign's features evolved organically over 20 years. Formspec's advantage is that the architecture is pre-designed for these capabilities; the gap is implementation, not design.

### The true critical gaps are fewer than they appear

After accounting for spec coverage — including the WOS spec suite (18 specs covering governance, workflow lifecycle, task management, review protocols, due process, AI governance, and structured audit) — the *genuinely unspecified* critical gaps are: reviewer dashboard (UI), document storage backend, Formspec Coprocessor handoff protocol, and FedRAMP/SOC 2/GSA Schedule certifications. Case management, which this document originally assessed as the "largest gap," is now largely spec-complete via WOS — the gap is implementation and UI, not design. This further strengthens the build strategy: the SaaS effort is about *implementing existing Formspec + WOS specs*, not *designing new capabilities*.

### The case management gap has been substantially closed by WOS

The WOS spec suite (developed in `wos-spec/`) addresses most of the case management requirements identified in Section 5 at the specification level: lifecycle topology, case state model, task lifecycle with assignment roles, SLA with breach policies, review protocols, due process, delegation of authority, hold policies, structured audit with authority ranking, and AI agent governance. The remaining gaps are: Formspec Coprocessor handoff (how a submission becomes a case), reviewer UI/dashboard, bulk triage, workload balancing algorithms, and case notes as a first-class feature. See `WOS-FEATURE-MATRIX.md` for the full competitive comparison.

### The signature question

Formspec is not an agreement tool, but many intake flows terminate in an attestation or acknowledgment. The Signature component captures drawn signatures, and the Ledger provides identity attestation and checkpoint signing infrastructure. The platform needs either a lightweight native attestation feature layered on these primitives, or a first-class integration with DocuSign/Adobe Sign. The evidence infrastructure exists; the ceremony and legal binding do not.

### The case management gap is the largest single risk

The entire post-submission workflow — review, RFI, approval, decision, notification — is absent from the open-source engine. This is the core of the product thesis. However, the Screener spec provides pre-submission routing with named outcomes, and Core validation provides submission gating with severity levels and external validation injection — these are the integration surfaces that a case management layer would consume.

### Certifications are time-gated, not effort-gated

FedRAMP takes 12-18 months minimum. SOC 2 Type II requires 6-12 months of operating history. VPAT requires a production system to audit. These cannot be compressed by adding engineers — they are calendar-bound. Starting the clock matters more than perfecting the architecture. The spec suite's compliance mechanisms (data minimization, version pinning, PII tracing, regulatory field references) create a strong foundation for the narrative these certifications require.

### Localization and accessibility are closer than previously assessed

The Locale spec is complete and thorough (CLDR plurals, cascade fallback, cross-tier keys, FEL interpolation, version compatibility). The Component spec has per-component ARIA mandates, and the Theme spec provides WCAG guidance. Implementation is the gap, not design. For accessibility, the highest-priority action is a formal WCAG audit and VPAT — the underlying primitives exist.

### The integration ecosystem is a cold start problem

DocuSign has 400+ connectors built over 20 years. Adobe has Experience Cloud integrations and AEM's Form Data Model with REST/SOAP/JDBC data source connectors. Formspec has a comprehensive Mapping DSL (10 transforms, bidirectional, 3 adapters), 4 Assist transport bindings, an Extension Registry with well-known discovery, and Ontology-based cross-system alignments — but zero pre-built connectors. The spec-level integration surface is stronger than any competitor's *specification* of integration, but enterprises buy connectors, not specifications. The government wedge can survive with webhooks and CSV export initially; Mapping DSL implementation is the bridge to more.

### Knowledge grounding is a genuine differentiator, not vaporware

The References + Ontology specs provide per-field bibliography with audience tagging, vector store URIs, tool invocation schemas, concept identity with SKOS equivalences, and a normative agent context assembly algorithm. No competitor has anything close to this level of structured knowledge grounding. This is the AI-native story — not "we added a chatbot" but "every field knows what it means, what regulations govern it, and where to find help." Implementation priority should be high.

### Adobe AEM Forms is the real competitive benchmark for government intake

DocuSign dominates agreement execution, but for the government intake/case workflows Formspec targets, Adobe AEM Forms is the incumbent to displace. Adobe's strengths — PDF rendering, BPM workflow, FedRAMP authorization, 15 years of scale — are real. But Adobe's weaknesses map directly to Formspec's thesis: no AI-native authoring, proprietary formats, bifurcated deployment model (cloud vs. on-prem are different products), no cryptographic audit, no open specification, and no structured knowledge grounding.

**Where Formspec already leads Adobe architecturally:** AI-native authoring (prompt-to-form, conversational intake vs. bolt-on Sensei); open JSON-native spec with multi-runtime execution (TS/Rust/Python) vs. proprietary XDP/XFA on Java; cryptographic audit integrity (Respondent Ledger spec) vs. application-level logs; unified deployment model (Shared/Regulated/Dedicated as one product) vs. bifurcated Cloud Service and on-prem JEE/OSGi; sandboxed cross-runtime expression language (FEL) vs. AEM-coupled rule editor; tier-aware AI governance (ADR-0008) vs. no customer-configurable AI controls; structured knowledge grounding (References + Ontology specs) vs. static help text; and offline-first Rust/WASM evaluation vs. WebView-wrapped mobile app.

**Where Adobe leads Formspec in maturity:** PDF/print output (XDP/XFA is the benchmark), BPM workflow (full engine with human tasks and SLA timers), production localization, save/resume session management, Form Data Model integrations (REST/SOAP/JDBC), Adobe Analytics, AEM user/group RBAC with LDAP/SAML, industry solution accelerators, FedRAMP/SOC 2/HIPAA certifications, and 15+ years of scale track record at IRS-level volume.

**Rough parity today:** adaptive forms (Formspec's 3-tier presentation model is more principled), validation (comparable with Formspec's explainable traces), versioning (Formspec's automated diffing and migration maps are more rigorous), accessibility primitives (both have ARIA support and WCAG guidance; both need formal VPAT).

**Parity estimate:** Formspec has ~30-40% of Adobe's enterprise surface area implemented today (engine layer is strong; management and SaaS layers are mostly Not Started). The 24-month roadmap targets ~70-80% of feature parity on the capabilities that matter, while adding capabilities Adobe lacks. The competitive narrative is not "we do what Adobe does, cheaper." It is: "Adobe built forms for the document era; Formspec is built for the intake and decisioning era." That narrative requires case management, document generation, and at least one compliance certification to be credible — those are the table stakes Adobe clears that Formspec does not yet.

### ServiceNow is the workflow/case management benchmark — and the integration ceiling

ServiceNow and Formspec approach the intake→workflow→case pipeline from opposite directions. ServiceNow is a workflow and service management platform that happens to have forms (catalog items, record producers). Formspec is an intake platform where the form *is* the product — adaptive, AI-native, document-aware — that feeds into case management. They collide in government/enterprise intake workflows, but the competitive dynamics are different from Adobe.

**Where ServiceNow leads decisively:** Case/workflow engine (Flow Designer, Orchestration, SLA timers, parallel review, escalation, human task management — 15+ years of BPM maturity); deep vertical process apps (ITSM, CSM, HR Service Delivery); IntegrationHub with 1000+ spokes; compliance certifications (FedRAMP High, IL4/IL5, SOC 2 Type II, HIPAA, ISO 27001, StateRAMP); Performance Analytics and configurable dashboards; LDAP/SAML/SSO/MFA with ACLs on every table/field/row; App Engine low-code platform; 7,700+ enterprise customers with government-wide deployments.

**Where Formspec leads architecturally (if implemented):** Adaptive/conversational intake (prompt-to-form authoring, conversational runtime, mixed free text + structured input — ServiceNow catalog items are static field lists; Virtual Agent routes but doesn't do adaptive intake); structured extraction from documents (core thesis — ServiceNow Doc Intelligence exists but is bolted on, not integrated into form filling); cryptographic audit integrity (Respondent Ledger spec with append-only, signed checkpoints, hash chain verification, 4 privacy tiers — ServiceNow has application-level audit logs with no tamper-evidence or export-verifiable proof bundles); open specification (JSON-native, multi-runtime — ServiceNow is deeply proprietary with platform lock-in as the business model); AI governance (tier-aware provider routing, customer-configurable data handling — ServiceNow Now Intelligence is platform-controlled); offline/disconnected (Rust/WASM kernel evaluates locally — ServiceNow is cloud-only); progressive deployment isolation (Shared→Regulated→Dedicated as one product line — ServiceNow offers a single cloud platform with a GovCloud option but no customer-controlled isolation tiers); respondent experience (purpose-built for public-facing applicants — ServiceNow portals are designed for employees and internal requestors); per-field knowledge grounding (References + Ontology specs — ServiceNow KB articles are global, not field-scoped).

**Rough parity:** Validation (both handle required/pattern/cross-field; Formspec has explainable traces, ServiceNow has server-side business rules); versioning (Formspec has automated diffing and migration maps, ServiceNow has Update Sets and app versioning); multi-language (Formspec Locale spec is thorough, ServiceNow has production i18n — both capable, Formspec unimplemented).

**Where Formspec should not compete with ServiceNow:** ITSM, CSM, HR Service Delivery — ServiceNow owns these verticals. Customers who need a general-purpose workflow platform today. Any buyer who requires FedRAMP High or IL4/IL5 certification now.

**Where Formspec can win against ServiceNow:**
- Intake-heavy workflows where ServiceNow catalog items are too rigid (benefits applications, grant intake, permitting, document-heavy onboarding)
- Trust-sensitive use cases where cryptographic audit, AI governance, and deployment flexibility matter more than raw workflow breadth
- Cost-sensitive buyers who cannot afford ServiceNow licensing for what is fundamentally an intake problem
- Open-spec procurement where agencies want to avoid proprietary lock-in

**The realistic competitive position:** Formspec is not a ServiceNow replacement. It is a best-in-class intake frontend that could *integrate with* ServiceNow (or replace the intake portion of workflows where ServiceNow's forms are the weakest link). ServiceNow's catalog items and record producers are adequate but generic input surfaces — they are not adaptive, not AI-native, not document-aware, and not designed for complex public-facing intake. Formspec's 24-month roadmap targets ~40-50% of ServiceNow's relevant surface area (intake + case management + integrations) while adding capabilities ServiceNow lacks entirely (adaptive intake, structured extraction, cryptographic audit, open spec, offline evaluation, AI governance). The integration story may matter more than the competition story: Formspec as the intake layer feeding ServiceNow workflows is a credible positioning that avoids head-to-head competition on workflow maturity.
