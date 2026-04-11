# Enterprise Implementation Roadmap

**Date:** 2026-04-07
**Author:** Strategy (with gap analysis input)
**Status:** Draft
**Context:** Formspec is a 1-2 person team building an AI-native intake and decisioning platform targeting government agencies as the primary buyer, with nonprofits and commercial teams as secondary segments. The business model is open-source core (formspec engine) driving SaaS conversion through managed hosting, governance tiers, and regulated deployment (Shared Cloud / Regulated Cloud / Dedicated). The product has a mature specification suite (~8,000 lines of normative spec across 9 specs), working engine implementations in TypeScript, Rust/WASM, and Python, and a comprehensive architecture decision set (ADR-0001 through ADR-0016). No production SaaS exists. No paying customers exist. No active pipeline or LOI exists. This is a supply-driven bet based on market analysis: government intake workflows are underserved by Adobe AEM Forms (expensive, proprietary, no AI-native authoring) and poorly served by ServiceNow (strong workflow, weak intake surfaces). This proposal is the complete sequenced implementation plan -- it replaces the prior product roadmap.

---

## Guiding Principle

Every phase must produce something a government program office could evaluate in a live demo and a nonprofit could pilot in production. No phase exists only as infrastructure for a later phase. If a phase cannot answer "what can a buyer do now that they could not do before?" it is scoped wrong.

---

## Reading This Proposal

### Capability Badges

| Badge | Meaning |
|-------|---------|
| ✅ | **Built** — working implementation exists in the engine (needs SaaS hosting/integration) |
| 📋 | **Spec-complete** — normative spec exists, needs implementation (design done, needs code) |
| 🆕 | **New build** — no spec exists, must design and build |
| 🏛️ | **Certification / procurement gate** — calendar-bound, cannot compress with engineering |
| 🔴 | **Critical gap** — blocks first government sale |
| 🟠 | **High gap** — blocks enterprise expansion |
| 🟡 | **Medium gap** — competitive parity |

### Spec and Engine Status Tracker

| Spec / Engine | Lines | Phase | Capability | Status |
|---------------|-------|-------|------------|--------|
| Formspec Engine (TS/Rust/WASM) | — | 1 | 1.2 Intake Runtime | ✅ Shipped — validation, FEL, branching, repeat groups, offline eval |
| Component Library | — | 1 | 1.2 Intake Runtime | ✅ Shipped — Signature, MoneyInput, FileUpload, ARIA mandates |
| iOS/SwiftUI Renderer | — | — | (not in proposal) | ✅ Shipped |
| USWDS Adapter | — | — | (not in proposal) | ✅ Shipped |
| `<formspec-render>` Web Component | — | 1 | 1.6 SaaS Foundation | ✅ Shipped — single JS import, works on any host |
| **WOS Kernel Spec** | ~2,500 | **1** | **1.3 Case Management** | 📋 Spec + 🟦 partial engine (wos-core typed models, evaluation algorithm) |
| **WOS Governance Spec** | ~2,800 | **1, 4** | **1.3, 4.5 Workflow** | 📋 Spec + 🟦 partial lint (189 rules, 76 tested) |
| **WOS AI Integration Spec** | ~2,600 | **3, 4** | **3.3, 4.5 AI Governance** | 📋 Spec only |
| **WOS Runtime Companion** | ~2,200 | **1** | **1.3 Case Instance** | 📋 Spec + 🟦 partial engine (CaseInstance model, host traits) |
| **WOS sidecars (9 total)** | ~3,500 | **1, 4** | **1.3, 4.5** | 📋 Spec + schema + fixtures (Business Calendar, Notification Template, Policy Params, etc.) |
| **Formspec Coprocessor** | 0 | **1** | **1.3 Handoff** | ⚪ Not yet specified — **blocks Phase 1.3** |
| Respondent Ledger | 969 | 2 | 2.1 Cryptographic Audit Ledger | 📋 Spec only |
| Locale | 1,253 | 2 | 2.2 Localization | 📋 Spec only |
| Screener | 1,508 | 3 | 3.4 Pre-Submission Routing | 📋 Spec only |
| References | 697 | 3 | 3.1 Knowledge Grounding | 📋 Spec only |
| Ontology | 782 | 3 | 3.1 Knowledge Grounding | 📋 Spec only |
| Mapping DSL | 2,023 | 4 | 4.4 Integration Ecosystem | 📋 Spec only |
| Assist | progressive | 1–3 | 1.2, 3.3, 3.1 | 📋 Spec + ✅ partial (Formy prototype, formspec-chat) |
| Extension Registry | 584 | 6 | 6.3 Ecosystem Development | 📋 Spec only |

---

## Phase 1: Core Intake Loop -- Author, Fill, Submit, Review

*Goal: A program administrator can create an intake form, a respondent can complete and submit it through a hosted web experience, and a reviewer can process the submission as a structured case -- not a pile of raw answers.*

This phase proves the product thesis: that conversational intake can produce structured, reviewable case data. It must ship the minimum credible version of every layer -- authoring, intake runtime, case management, and trust baseline -- because a product that collects data but cannot route it to a reviewer is a form builder, not an intake platform. Case management is here, not deferred, because without it Formspec has no answer to "what happens after submit?"

### 1.1 Form Authoring Studio 🆕

- Prompt-to-form generation from natural language descriptions
- Visual schema editor for fields, sections, pages, and repeat groups
- Rule editor for validation, branching, and conditional visibility
- Publish lifecycle: draft, published, archived
- Form versioning with diff view (Core S6.4 version pinning)

### 1.2 Conversational Intake Runtime ✅ Engine 📋 `Assist`

- Hosted web form rendering using the formspec engine (TypeScript/WASM)
- Adaptive questioning with conditional sections and branching
- Save and resume (session persistence with draft recovery)
- Summary-before-submit review screen
- Mixed structured input and free text where the spec supports it
- File upload (client-side capture; server storage is 1.4)

### 1.3 Case Management Baseline 📋 🔴 `WOS Kernel` `WOS Governance` `Formspec Coprocessor`

> **WOS dependency (added 2026-04-10):** The WOS spec suite (18 specs in `wos-spec/`) provides comprehensive governance and workflow semantics that cover most case management requirements at the specification level. This phase should implement WOS governance specs — not design a parallel governance system. The critical blocker is the **Formspec Coprocessor** handoff protocol (how a Formspec submission becomes a WOS case instance), which must be specified before this phase begins. See `WOS-FEATURE-MATRIX.md` for the full competitive comparison and `wos-spec/TODO.md` for the Coprocessor gap.

- 📋 **[WOS]** Case object created from each completed submission — WOS Kernel S4-S5 defines lifecycle topology and case state model; CaseInstance serialization in Runtime S3. **Requires:** Formspec Coprocessor handoff spec (not yet written).
- 📋 **[WOS]** Case status lifecycle — WOS provides the framework for arbitrary state definitions (atomic, compound, parallel, final). Specific status names (submitted, in-review, etc.) are workflow authoring decisions using WOS primitives.
- 📋 **[WOS]** Reviewer queue with assignment — WOS Governance S10 defines 8-state task lifecycle and 5 assignment roles (owner, nominee, potentialOwner, businessAdministrator, excludedOwner). **🆕 Needs:** reviewer dashboard UI.
- 📋 **[WOS]** RFI workflow — WOS Governance S12 defines `pending-applicant-response` hold policy with resume triggers and timeout actions. Notification Template sidecar defines structured notices. **🆕 Needs:** notification delivery infrastructure (email/SMS), Coprocessor handoff for respondent amendments.
- 🆕 Case notes for internal reviewer annotations — not in WOS; genuine UI feature gap.
- 📋 **[WOS]** Post-submission status notifications — WOS Correspondence Metadata and Notification Template sidecars define notice structure and templates. **🆕 Needs:** delivery infrastructure.

### 1.4 Document Storage and Upload Backend 🆕 🔴 `ADR-0009`

- Server-side storage for file uploads (S3-compatible object store)
- Virus/malware scanning pipeline for uploaded files
- Document preview (PDF, image) in reviewer interface
- Immutable original preservation (per ADR-0009 evidence model)

### 1.5 Trust Baseline 🆕 `ADR-0003` `ADR-0004`

- Immutable submission snapshot at time of submit
- Application-level audit trail (who did what, when -- not yet cryptographic)
- Organization and workspace RBAC (per ADR-0004 identity model)
- Support access logging with actor attribution

### 1.6 SaaS Foundation 🆕 ✅ `<formspec-render>` `ADR-0001`

- Multi-tenant Shared Cloud deployment (per ADR-0001)
- User authentication (email/password, SSO/SAML for orgs)
- Webhook outbound events (submission created, case status changed)
- One database/automation connector (Postgres or Zapier) for data sync

**Phase 1 delivers:** A complete intake loop -- from authoring through respondent completion through reviewer case processing -- that can be demoed to a government program office and piloted by a nonprofit. A reviewer works from a structured case with status tracking, not a spreadsheet of raw form responses.

---

## Phase 2: Trustworthy Intake -- Audit, Compliance Readiness, and Localization

*Goal: A compliance officer can review the platform's audit trail, data handling, and accessibility posture and approve it for a government pilot without hand-waving around logs, retention, or access controls.*

Phase 1 proves the product works. Phase 2 proves it can be trusted. Government buyers do not pilot systems that lack audit trails, retention policies, or accessibility documentation. This phase implements the spec-complete capabilities (audit ledger, localization) that are already designed but not built, and starts the certification clock that is calendar-gated and cannot be compressed later.

### 2.1 Cryptographic Audit Ledger 📋 `Respondent Ledger` `ADR-0003` `ADR-0007`

- Implement the Respondent Ledger spec: append-only event log with 13 required event types
- Signed audit checkpoints with hash chain integrity (LedgerCheckpoint per Ledger S13)
- Three deployment profiles: local/server, pseudonymous integrity-anchored, identity-bound (Ledger S15A)
- Respondent-facing change history / timeline with privacy-tiered disclosure
- External integrity anchoring via timestamp authority (LedgerCheckpoint anchorRef)

### 2.2 Localization 📋 `Locale` 🔴

- Implement the Locale spec: BCP 47 language tags, CLDR plural forms, FEL interpolation in strings
- Four-step cascade fallback for translation resolution
- Cross-tier locale keys ($page.*, $component.*)
- Three FEL locale functions: locale(), formatNumber(), formatDate()
- At least two language packs (English + Spanish) for government intake compliance (EO 13166)

### 2.3 Accessibility Audit and VPAT 🏛️ 🔴 ✅ ARIA primitives

- Formal WCAG 2.1 AA audit against the existing component ARIA mandates
- Screen reader testing across major assistive technologies
- Publish VPAT (Voluntary Product Accessibility Template) -- required for Section 508 procurement
- Remediate findings from the audit

### 2.4 Governance and Data Lifecycle 🆕 🟠 `ADR-0012`

- Retention controls with configurable policies per object class (per ADR-0012)
- Legal hold capability on retained data
- Configurable deletion semantics: hard delete, soft delete, anonymize (GDPR, records management)
- Support-access workflow with JIT approval protocol (per ADR-0004)

### 2.5 SOC 2 Preparation 🏛️ 🔴

- Begin SOC 2 Type II observation period (requires 6-12 months of operating history)
- Security whitepaper and procurement response pack for enterprise sales conversations
- Tier-qualified compliance posture document mapping platform-owned vs. customer-configurable controls (per ADR-0015)

### 2.6 Infrastructure Hardening 🆕 🟠 `ADR-0011`

- Monitoring, alerting, and observability baseline (per ADR-0011)
- Disaster recovery and backup procedures
- Rate limiting and abuse protection for public-facing intake
- Environment separation: sandbox and production per workspace

**Phase 2 delivers:** A platform that a government security and compliance team can evaluate with real audit logs, a published VPAT, a compliance posture document, and data lifecycle controls. The SOC 2 clock is running. Localized intake is live for at least English and Spanish. The system is hardened enough for a real pilot, not just a demo.

---

## Phase 3: Intelligent Intake -- Knowledge Grounding, Document Intelligence, and AI Governance

*Goal: A program administrator can connect policy documents to their intake forms so that respondents get grounded guidance and reviewers get AI-assisted document classification and extraction -- with explicit controls over which AI providers touch the data and under what terms.*

This phase is the differentiation play. Competitors have forms and workflows. Formspec's thesis is that intake should be intelligent: every field knows what it means, what regulations govern it, and where to find help. The References and Ontology specs are unique in the market -- no competitor has structured per-field knowledge grounding. AI governance (tier-aware provider routing, no-training posture) is a government trust requirement that Adobe and ServiceNow do not offer.

### 3.1 Knowledge Grounding Layer 📋 `References` `Ontology` ⭐ Differentiator

- Implement References spec: per-field bibliography with 12 reference types including vector-store, knowledge-base, retrieval
- Audience tagging (human/agent/both) and priority tiers for contextual help
- URI schemes for AI infrastructure (vectorstore:, kb:, formspec-fn:)
- Eight-step agent context assembly algorithm (References S5.1)
- Ontology spec implementation: concept URIs, SKOS equivalences, vocabulary bindings, JSON-LD fragments

### 3.2 Document Intelligence 🆕 🟠 ⭐ Differentiator

- Document type classification (AI-powered, with confidence scores)
- Structured data extraction from uploaded PDFs and images (OCR + layout-aware extraction)
- Missing document detection against intake requirements
- Evidence packet assembly for reviewer handoff (per ADR-0009)

### 3.3 AI Governance Controls 🆕 🟠 `ADR-0008` ⭐ Differentiator

- Tier-aware AI provider routing (per ADR-0008): which models are permitted in which deployment tiers
- No-training / no-retention posture enforcement as product default
- Human review gates for AI-generated classifications and pre-fill suggestions
- Prompt injection protection for AI-powered intake fields

### 3.4 Pre-Submission Routing 📋 `Screener`

- Implement Screener spec: multi-phase evaluation pipeline with three strategies (first-match, fan-out, score-threshold)
- Override routes for safety-critical classifications
- Named outcomes (ineligible, closed, review, referral) feeding into case management
- Structured Determination Records (Screener S4-S9)
- Auto-assignment rules: route cases to reviewers based on screener outcomes

### 3.5 Analytics Foundation 🆕 🟠

- Submission analytics: completion rates, drop-off points, time-to-complete (using Ledger event stream)
- Reviewer throughput and workload metrics
- Low-confidence field reporting (which extracted values need human review most often)
- Respondent Ledger lifecycle events powering the analytics pipeline

**Phase 3 delivers:** An intake platform that is meaningfully smarter than a form builder plus chatbot. Policy documents ground the respondent experience. Uploaded documents are classified and extracted automatically. Cases route to the right reviewer based on intake content. AI usage is governed with controls government buyers require. The competitive narrative -- "built for the intake and decisioning era, not the document era" -- is now demonstrable.

---

## Phase 4: Government-Ready Platform -- Certifications, Regulated Cloud, and Integration Ecosystem

*Goal: A government agency can procure Formspec through standard channels (GSA Schedule), deploy it in a regulated environment (FedRAMP-authorized), and connect it to their existing systems (identity providers, case management tools, document repositories) without custom integration work.*

This phase converts a product that government buyers can evaluate into one they can actually buy and deploy. FedRAMP authorization, GSA Schedule listing, and pre-built connectors are procurement gates -- without them, even a superior product cannot enter the government sales pipeline at scale. The integration ecosystem begins here because the prior phases provide the stable APIs that connectors require.

### 4.1 FedRAMP Authorization 🏛️ 🔴 `ADR-0002` `ADR-0015`

- Complete FedRAMP Moderate authorization process (3PAO engagement, SSP, SAR, POA&M)
- Regulated Cloud deployment tier with cell-based architecture (per ADR-0002)
- Regional deployment options for data residency requirements
- HIPAA compliance posture and BAA capability for healthcare-adjacent intake

### 4.2 GSA Schedule and Procurement Readiness 🏛️ 🔴

- GSA Schedule listing (enables streamlined government purchasing)
- StateRAMP authorization (growing requirement for state agencies)
- Published security whitepaper, procurement response pack, and shared responsibility matrix
- Case studies from Phase 1-3 pilots (assuming pilot success)

### 4.3 Identity and Access Expansion 📋 `Ledger S6.6` 🔴 `ADR-0004`

- Government identity proofing integration: ID.me, Login.gov (implementing Ledger S6.6 adapters)
- LDAP / Active Directory integration for Dedicated tier on-premises identity
- SCIM automated user provisioning
- Phone/SMS verification as second factor

### 4.4 Integration Ecosystem 📋 `Mapping DSL` 🟠 `ADR-0010`

- Pre-built government system connectors: SAM.gov, Grants.gov
- Storage connectors: S3, Azure Blob, GCS (formalizing what the platform uses internally)
- DocuSign / Adobe Sign integration for signature ceremonies on intake attestations
- ServiceNow integration: Formspec as intake frontend, ServiceNow as workflow backend
- Mapping DSL runtime: implement the 10 transform types for bidirectional data mapping
- Webhook infrastructure hardening: retry, dead-letter, delivery guarantees

### 4.5 Advanced Workflow 📋 🟠 `WOS Governance` `WOS AI Integration`

> **WOS dependency (added 2026-04-10):** Most capabilities in this phase are spec-complete in WOS. The effort is implementing WOS governance specs in the SaaS layer, not designing new governance systems.

- 📋 **[WOS]** Approval chains — WOS Kernel S4 provides compound/parallel states with completion policies; Governance S4 defines 5 review protocols; Governance S11 defines delegation of authority. **🆕 Needs:** pre-built workflow templates for common approval patterns.
- 📋 **[WOS]** SLA timers and escalation — WOS Governance S10.3 defines SLA with 4 breach policies. Business Calendar sidecar (now spec-complete with schema, fixture, model) defines business-day computation. Kernel S9.7 defines 5 timeout categories. **Implementation only.**
- 🆕 Bulk triage and batch operations — WOS explicitly rejected batch operations as an "implementation concern" (ADR-0058). Genuine platform feature.
- 📋 **[WOS partial]** Assignment and workload balancing — WOS Governance S10.2 defines potentialOwner pools. **🆕 Needs:** workload balancing algorithms (even distribution is implementation-defined).
- 📋 **[WOS]** Appeal workflow — WOS Governance S3.5-3.6 defines appeal requirements (independent adjudicator, continuation of service, appeal provenance, counterfactual explanation). **🆕 Needs:** appeal workflow template authored using WOS primitives.
- 📋 **[WOS]** Decision recording with structured rationale — WOS Governance S6 defines Reasoning tier (rules applied, evidence consulted, authority ranking) and Counterfactual tier. Extends Screener Determination Records for pre-submission. **Implementation only.**
- 📋 **[WOS]** AI agent governance — WOS AI Integration spec defines deontic constraints, autonomy levels, confidence framework, fallback chains, drift detection, and agent disclosure. **Implementation only; no competitor has equivalent.**
- 📋 **[WOS]** Review quality controls — WOS Governance S7 defines quality sampling, separation of duties, and structured override authority. S4 defines 5 review protocols (independent-first, consider-opposite, calibrated confidence, dual-blind, unassisted). **Implementation only; no competitor has equivalent.**

**Phase 4 delivers:** A platform that government agencies can procure through GSA Schedule, deploy in a FedRAMP-authorized environment, connect to their existing identity and case management infrastructure, and operate with mature review workflows. The procurement barrier -- the single largest obstacle to government SaaS sales -- is removed.

---

## Phase 5: Operator Leverage -- Adaptive Rendering, Document Output, and Platform Maturity

*Goal: A single intake definition can serve multiple respondent contexts (wizard, dense, mobile, staff-assisted, accessibility-first) and produce structured output documents (decision letters, summary packets) without manual formatting.*

### 5.1 Adaptive Rendering 🆕 🟡

- Multiple rendering modes from one form definition: wizard, dense/power-user, staff-assisted, accessibility-first, mobile-first
- Kiosk mode for in-person intake
- Call-center / staff-assisted intake mode

### 5.2 Document Output 🆕 🟠 ✅ PDF prototype on branch

- PDF generation from submissions (production-ready, building on prototype branch)
- Summary packets and decision letters from case data
- Batch letter/notice generation for program-wide communications
- Document redaction tools for FOIA and records requests

### 5.3 Platform Maturity 🆕 🟠 `ADR-0013`

- Tenant data export and portability packaging (per ADR-0013)
- Tier migration tooling: Shared to Regulated to Dedicated (per ADR-0013)
- Template gallery and starter packs for common intake workflows
- CDN / edge caching for form delivery performance at scale

**Phase 5 delivers:** Operators get more leverage from every intake definition -- one form serves many contexts, produces finished documents, and scales across deployment tiers. The platform moves from "each form is a project" to "each form is a reusable asset."

---

## Phase 6: Ecosystem and Expansion

*Goal: The platform supports multi-party workflows, AI-native narrative generation, and a partner ecosystem for vertical delivery.*

### 6.1 Multi-Party Completion 🆕 🟡

- Contributor invites with section ownership
- Delegated uploads and consolidated review
- Witness and counter-signature workflows

### 6.2 Narrative Generation 🆕 🟡 ⭐ Differentiator

- Grant narrative generation from structured case data
- Compliance report generation
- Editable generated outputs with round-tripping

### 6.3 Ecosystem Development 📋 `Extension Registry` 🟡

- Zapier / Power Automate / Make integrations for low-code connectivity
- OAuth2 provider for third-party application development
- Extension Registry runtime (implementing the 584-line Extension Registry spec)
- Partner / systems integrator program for government delivery channel
- Community development leveraging open-source core

### 6.4 Advanced Trust 📋 `Ledger Chain` `ADR-0016`

- End-to-end respondent ledger chain (per ADR-0016): client capture through server authority to export proof packages
- Selective disclosure and verification without full data exposure
- Exportable verification packages (proof bundles) for appeals and external audits

**Phase 6 delivers:** The platform supports complex multi-party intake, generates narrative documents from structured data, and has an ecosystem of integrations and partners. The product is a platform, not just a tool.

---

## Summary: Priority Order

| Phase | Name | Key Unlock | Mix |
|-------|------|------------|-----|
| 1 | Core Intake Loop | End-to-end demo: author, fill, submit, review as a case | 📋📋🆕🆕🆕🆕 |
| 2 | Trustworthy Intake | Compliance team can approve a pilot — audit ledger, VPAT, SOC 2 clock | 📋📋🏛️🏛️🆕🆕 |
| 3 | Intelligent Intake | Differentiation — knowledge grounding, doc intelligence, AI governance | 📋📋🆕🆕🆕 |
| 4 | Government-Ready Platform | Procurement barrier removed — FedRAMP, GSA, integrations | 🏛️🏛️📋📋📋📋🆕 |
| 5 | Operator Leverage | Adaptive rendering, document output, platform maturity | 🆕🆕🆕 |
| 6 | Ecosystem and Expansion | Multi-party, narrative generation, partner channel | 📋🆕🆕🆕 |

---

## Notes on Sequencing

**Phases 1-2 are the MVP.** A government pilot requires both a working product (Phase 1) and a trustworthy product (Phase 2). Phase 1 alone is sufficient for nonprofit pilots and commercial demos. Phase 2 is the gate for government pilot approval -- without an audit ledger, VPAT, and compliance posture document, a government security review will reject the platform regardless of capability.

**Phase 3 is the core differentiator for the target audience.** Knowledge grounding, document intelligence, and AI governance are what separate Formspec from "another form builder" and from Adobe/ServiceNow. However, Phase 3 is a differentiator, not a table stake -- pilots can proceed without it. It should not delay Phase 2 completion.

**Phase 4 is the revenue gate for government at scale.** Individual pilots can happen on Shared Cloud without FedRAMP, but agency-wide procurement requires FedRAMP authorization and GSA Schedule listing. The 12-18 month FedRAMP timeline means the 3PAO engagement should begin as soon as Phase 2 infrastructure is stable -- roughly 9-12 months into the plan. The authorization window then overlaps with Phase 3 development.

**Certification parallel track.** SOC 2 observation period starts in Phase 2. FedRAMP 3PAO engagement starts during Phase 3. VPAT is published in Phase 2. These are calendar-gated processes that run alongside product development:

| Certification | Clock starts | Expected completion | Blocks |
|--------------|-------------|-------------------|--------|
| 🏛️ VPAT | Phase 2 (after WCAG audit) | Phase 2 | 🔴 Government pilot evaluation |
| 🏛️ SOC 2 Type II | Phase 2 (observation period) | Phase 3–4 boundary | 🔴 Enterprise procurement |
| 🏛️ FedRAMP Moderate | Phase 3 (3PAO engagement) | Phase 4 | 🔴 Agency-wide deployment |
| 🏛️ GSA Schedule | Phase 3–4 (application) | Phase 4 | 🔴 Streamlined government purchasing |
| 🏛️ StateRAMP | Phase 4 (leverages FedRAMP) | Phase 4–5 | 🟡 State agency procurement |

**Phases 5-6 can be deferred.** Adaptive rendering and document output are operator convenience, not buyer gates. Multi-party workflows and narrative generation are expansion features. Manual workarounds exist: single-mode rendering covers most pilots, PDF export from the browser covers document output needs, and partners can be recruited informally before a formal program exists. Build these when Phase 1-3 pilots generate demand signals.

**Phases 5-6 are independent of each other.** They can be reordered or partially implemented based on customer pull. Narrative generation could move earlier if grant-writing use cases emerge from pilots. Adaptive rendering could move earlier if a specific government agency requires kiosk or staff-assisted intake.

**The integration ecosystem (Phase 4) is partially deferrable.** Webhooks (Phase 1) and the Mapping DSL (Phase 4) cover most data movement needs. Pre-built connectors for SAM.gov and Grants.gov are government-pull features -- do not build until a pilot customer needs them. DocuSign/ServiceNow integrations are higher priority because they answer the "how does this fit into our existing stack?" question that government IT teams ask in every evaluation.

**Spec implementations dominate Phases 1-4.** ~~Approximately 8 capabilities in Phases 2-3 are implementing existing normative specs~~ **[Updated 2026-04-10]** With WOS governance specs now covering case management, task lifecycle, review protocols, due process, delegation, SLA, AI governance, and structured audit, approximately **16+ capabilities across Phases 1-4** are implementing existing normative specs (WOS Kernel, WOS Governance, WOS AI Integration, WOS Runtime, WOS sidecars, Respondent Ledger, Locale, Screener, References, Ontology, Mapping DSL). Case management — originally the largest 🆕 "new build" — is now largely 📋 "spec-complete, needs implementation." This further strengthens the spec suite advantage. The **critical pre-requisite** is the Formspec Coprocessor handoff spec, which must be written before Phase 1.3 implementation begins.

---

## ADR and Spec Dependencies

| Document | Relationship | Notes |
|----------|-------------|-------|
| ADR-0001 (Tenancy and Deployment) | Foundation for all phases | Phase 1 implements Shared Cloud; Phase 4 implements Regulated Cloud and Dedicated. Foundational ADR -- changes require multi-team review. |
| ADR-0002 (Control Plane / Data Plane) | Foundation for Phase 1+, fully realized in Phase 4 | Cell-based architecture is designed but not implemented. Phase 1 can use simplified single-cell deployment; Phase 4 requires full cell architecture for regulated deployment. |
| ADR-0003 (Audit Ledger) | Implemented in Phase 2 | Phase 2.1 implements the cryptographic ledger and signed checkpoints defined in this ADR. High-coupling ADR -- changes cascade to ADR-0007, ADR-0009, ADR-0012, ADR-0016. |
| ADR-0004 (Identity and Authorization) | Implemented progressively Phase 1-4 | Phase 1 implements basic RBAC. Phase 2 adds JIT support access. Phase 4 adds government identity proofing. Foundational ADR. |
| ADR-0005 (Postgres Isolation) | Foundation for Phase 1+ | Database isolation strategy by tier. Phase 1 implements shared-schema approach; Phase 4 implements tier-specific isolation. Foundational ADR. |
| ADR-0006 (Core Data Model) | Foundation for Phase 1+ | Defines engine-owned vs. instance-owned vs. SaaS-owned objects. Case management (Phase 1.3) must respect these ownership boundaries. Foundational ADR. |
| ADR-0007 (Key Management) | Implemented in Phase 2 | Signing keys for audit ledger integrity. Required for Phase 2.1 signed checkpoints. |
| ADR-0008 (AI Provider Routing) | Implemented in Phase 3 | Phase 3.3 implements tier-aware AI governance. High-coupling ADR -- data handling changes must reconcile. |
| ADR-0009 (Document Storage) | Implemented in Phase 1-3 | Phase 1.4 implements storage backend. Phase 3.2 implements evidence packets. Must reconcile with data handling rules. |
| ADR-0010 (Integration Architecture) | Implemented in Phase 4 | Phase 4.4 implements connector isolation and the integration ecosystem. |
| ADR-0011 (Observability) | Implemented in Phase 2 | Phase 2.6 implements monitoring baseline. |
| ADR-0012 (Data Lifecycle) | Implemented in Phase 2 | Phase 2.4 implements retention, deletion, and legal hold. High-coupling ADR. |
| ADR-0013 (Tenant Portability) | Implemented in Phase 5 | Phase 5.3 implements export packaging and tier migration. Must not assume deployment immobility in earlier phases. |
| ADR-0014 (Release Management) | Implemented progressively Phase 2+ | Environment separation in Phase 2.6, tier-aware rollout in Phase 4. |
| ADR-0015 (Compliance Boundary) | Governs all compliance claims | Every certification and tier claim in this proposal must reconcile with ADR-0015. Phase 2.5 compliance posture document is derived from this ADR. High-coupling ADR. |
| ADR-0016 (Client/Server Ledger Chain) | Implemented in Phase 6 | Phase 6.4 implements the full privacy-preserving ledger chain. Depends on ADR-0003, ADR-0007, ADR-0009, ADR-0012, ADR-0013, ADR-0015. |
| Respondent Ledger Spec | Implemented in Phase 2 | 969 lines of normative spec. Phase 2.1 implements the core ledger, events, checkpoints. |
| Locale Spec | Implemented in Phase 2 | 1253 lines. Phase 2.2 implements localization runtime. |
| Screener Spec | Implemented in Phase 3 | 1508 lines. Phase 3.4 implements pre-submission routing. |
| References Spec | Implemented in Phase 3 | 697 lines. Phase 3.1 implements knowledge grounding. |
| Ontology Spec | Implemented in Phase 3 | 782 lines. Phase 3.1 implements concept identity and vocabulary bindings. |
| Mapping DSL Spec | Implemented in Phase 4 | 2023 lines. Phase 4.4 implements bidirectional data transforms for connectors. |
| Assist Spec | Implemented progressively Phase 1-3 | Conversational tools in Phase 1.2; human review gates in Phase 3.3; profile matching in Phase 3. |
| Extension Registry Spec | Implemented in Phase 6 | 584 lines. Phase 6.3 implements the extension catalog and discovery. |

| WOS Kernel Spec | Governs Phase 1.3, 4.5 | 📋 **[Added 2026-04-10]** Lifecycle topology, case state model, actor model, impact levels, durable execution, provenance Facts tier. Phase 1.3 case management should implement WOS kernel semantics, not invent parallel ones. |
| WOS Governance Spec | Governs Phase 1.3, 4.5 | 📋 **[Added 2026-04-10]** Task lifecycle (8 states, 5 roles), SLA with breach policies, review protocols (5 types), due process, delegation, hold policies, structured audit, quality controls. Most of Phase 4.5 "Advanced Workflow" is spec-complete in WOS. |
| WOS AI Integration Spec | Governs Phase 3.3, 4.5 | 📋 **[Added 2026-04-10]** Deontic constraints, autonomy levels, confidence framework, fallback chains, drift detection, agent disclosure. Extends Phase 3.3 AI governance from ADR-0008 provider routing to full agent behavioral governance. |
| WOS Runtime Companion | Governs Phase 1.3 | 📋 **[Added 2026-04-10]** CaseInstance serialization, event delivery contract, action execution model, host interfaces (InstanceStore, ContractValidator, EventQueue, etc.). |
| Formspec Coprocessor | **Blocks** Phase 1.3 | ⚪ **[Added 2026-04-10]** Handoff protocol between Formspec submissions and WOS case instances. Not yet specified. Must be written before Phase 1.3 implementation — without it, the platform must invent its own submission-to-case bridge. See `wos-spec/TODO.md`. |

No phase in this proposal contradicts an accepted ADR. Phase 1 case management (1.3) ~~introduces a new domain object (case) that is not yet defined in ADR-0006 (Core Data Model). A new ADR or addendum to ADR-0006 should be drafted before Phase 1.3 implementation begins, defining case object ownership boundaries.~~ **[Updated 2026-04-10]** should implement WOS Kernel and Governance specs as the case management foundation. The WOS CaseInstance (Runtime S3) defines the case object; WOS Governance S10 defines the task lifecycle. The Formspec Coprocessor handoff protocol must be specified before Phase 1.3 begins — this replaces the need for a case object ADR, as WOS provides the domain model.

---

## Assumptions and Open Questions

| Assumption | If wrong, then... |
|-----------|-------------------|
| Government pilots can proceed on Shared Cloud without FedRAMP authorization. | Phase 4 must move earlier, adding 12-18 months before first government deployment. The entire plan shifts right by a year. |
| A solo/small team (1-2 developers) can deliver Phase 1 in approximately 6-9 months. | Phases compress only with additional headcount. If Phase 1 takes 12+ months, the plan is a 3-4 year horizon and may require fundraising to sustain. |
| Nonprofit pilots provide useful learning loops for government readiness. | If nonprofit intake workflows diverge significantly from government workflows, pilot learnings do not transfer and the nonprofit wedge adds distraction without strategic value. |
| Spec-complete capabilities (audit ledger, localization, screener) require significantly less implementation time than genuinely new capabilities (case management, document intelligence). | If implementing specs is as slow as designing from scratch, Phases 2-3 timelines are underestimated by 50-100%. The spec suite advantage is weaker than assumed. |
| DocuSign/Adobe Sign integration is more credible than building a native signature ceremony. | If government buyers require native signatures (not integration), a signature capability must be added to Phase 2 or 3, displacing other work. |
| ServiceNow integration is more valuable than competing on workflow maturity. | If government buyers want an all-in-one platform and reject "best intake frontend + ServiceNow backend," case management scope in Phase 4.5 must expand significantly, potentially becoming its own phase. |
| The open-source core is a procurement advantage, not a liability, for government buyers. | If agencies view open-source as risk rather than advantage, the commercial narrative must shift to emphasize the SaaS platform and de-emphasize the open spec. |
| Manual workarounds are acceptable for billing, advanced reporting, and template galleries through Phase 4. | If paying customers demand self-serve billing or reporting dashboards before Phase 5, those capabilities must be pulled forward, displacing planned work. |
| This plan is a supply-driven bet -- no specific customer has requested these capabilities. | All phases beyond Phase 1 are contingent. If Phase 1 does not produce at least one design partner willing to pilot, the plan should be reconsidered before investing in Phase 2 trust infrastructure. |
| SOC 2 Type II observation can begin on Shared Cloud infrastructure during Phase 2. | If the auditor requires Regulated Cloud infrastructure for the observation period, SOC 2 slides to Phase 4 and enterprise procurement is delayed. |

### Open questions

1. Should case management be built as a platform layer in the open-source core or as a SaaS-only capability? The answer affects ADR-0006 ownership boundaries and the open-source community value proposition. -- affects Phase 1.3 architecture
2. What is the minimum viable case object schema? The gap analysis identifies case lifecycle, RFI, approval chains, SLA timers, appeal workflow, and decision recording. Phase 1 scopes only lifecycle + queue + RFI. Is that sufficient for a pilot, or do specific government workflows require approval chains from day one? -- affects Phase 1.3 scope
3. For localization (Phase 2.2), which languages beyond English and Spanish are needed for the first government pilot? EO 13166 requirements vary by agency and program. -- affects Phase 2.2 scope
4. Is the existing prototype PDF generation module (on a branch) close enough to production-ready to pull into Phase 2, or is it a Phase 5 rebuild? -- affects Phase 2 scope and document output timing
5. For the FedRAMP authorization (Phase 4.1), should the target be FedRAMP Moderate (standard for most civilian agencies) or should FedRAMP High / IL4 be on the roadmap for DoD-adjacent work? -- affects Phase 4 scope and 3PAO selection
6. Does the team intend to build the document intelligence / extraction pipeline (Phase 3.2) in-house, or integrate with an existing service (AWS Textract, Azure Document Intelligence, Google Document AI)? The build-vs-integrate decision changes Phase 3 timeline by months. -- affects Phase 3.2 scope and timeline
