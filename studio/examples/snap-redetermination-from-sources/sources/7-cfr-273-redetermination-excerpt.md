# 7 CFR §273.10 — Redetermination of eligibility (illustrative excerpt)

> **Note:** This is an illustrative excerpt for the Studio vertical slice. Real text lives at eCFR.gov in JSON-LD form; canonical `referencedUri` would be `https://www.ecfr.gov/api/renderer/v1/title-7/section-273.10`.

**SourceDocument id:** `src-cfr-7-273`
**Effectiveness ref:** `eff-fed-cfr-273-current` (federal, US, final, intervals=[start: 2024-10-01], no sunset)
**Ingest format:** `json-ld` (canonical-source path; `canonicalSourceRef.referencedUri` set)
**Content locales:** `["en-US", "es-US"]` (CFR is bilingually published)

---

## §273.10(a) — Periodic redetermination

(1) Each State agency MUST redetermine each household's SNAP eligibility at intervals not to exceed:

  - **12 months** for households with a stable income source (e.g., fixed-income retirees, applicants with documented disability income).
  - **6 months** for households with earned income or any income source subject to fluctuation.

(2) The State agency MUST issue written notice of the redetermination requirement no later than **30 calendar days** before the redetermination due date. The notice MUST include:

  - Identification of the household.
  - The redetermination due date.
  - The information and documentation the household must provide.
  - The right to request an interview by phone, in-person, or in writing.
  - The right to appeal an adverse redetermination decision under §273.15.

## §273.10(g) — Adverse redetermination decisions

(1) When a State agency determines that a household is no longer eligible (or eligible at a reduced benefit) following redetermination, the State agency MUST provide a **Notice of Adverse Action** at least **10 calendar days** before the adverse action takes effect.

(2) The Notice of Adverse Action MUST be in plain language and MUST include:

  - The reason for the adverse action with citation to the regulatory basis.
  - The effective date.
  - The household's right to a fair hearing under §273.15.
  - The 90-day window to request a fair hearing.
  - Whether the household may continue receiving benefits during the hearing (continuation of services).
  - Contact information for assistance.

(3) The Notice MUST be provided in the household's primary language when the household has indicated language-other-than-English. State agencies MUST maintain Spanish-language notices at minimum.

## §273.15 — Fair hearings (excerpt)

(a) A household has the **right to request a fair hearing** within **90 calendar days** after a written notice of adverse action.

(b) When a household timely requests a fair hearing AND continuation of services is requested, benefits MUST continue at the prior level pending the hearing decision UNLESS the household specifically requests termination.

(c) A fair hearing MUST be held within **60 calendar days** of the household's request, unless extended by mutual agreement.

---

## Studio-side metadata

- **Authority:** federal regulation (highest authority for SNAP redetermination).
- **Cited by:** state SNAP manual Ch.8; office memo recert v3.2.
- **Anchors:** §273.10(a)(1), §273.10(a)(2), §273.10(g)(1), §273.10(g)(2), §273.10(g)(3), §273.15(a), §273.15(b), §273.15(c).
- **DPV-relevant data classes touched:** `dpv:GovernmentBenefit`, `dpv:FinancialPreference`, `dpv:Demographic` (language-spoken), `dpv:Disability`.
