# Federal Corrective Action Letter — Texas SNAP, 2024-Q3 Audit

**Issuing authority:** USDA Food and Nutrition Service (FNS)
**Letter id:** `src-fns-corrective-action-2024-q3`
**Effectiveness ref:** `eff-fns-corrective-2024-q3` (federal, US, final, intervals=[start: 2024-11-15], no sunset)
**Ingest format:** `pdf`
**Content locales:** `["en-US"]`

---

## To: Texas Department of Health and Human Services, SNAP Operations

## Re: 2024-Q3 Audit Findings — Adverse-Action Handling Deficiencies

The Food and Nutrition Service (FNS) has completed its 2024-Q3 audit of Texas SNAP redetermination operations. Two deficiencies require corrective action effective immediately:

## Finding 1: Restrictive appeal-request channels

Travis County DHS Office Procedure Memo "Recertification Process v3.2" §4(a) requires fair-hearing requests be submitted in writing. This restricts the household's right under 7 CFR §273.15(a) to request a fair hearing by ANY reasonable means. Phone, in-person, AND in writing MUST be equivalently acceptable channels. Exception-only handling of phone or in-person requests violates federal regulation.

**Corrective action:** Office Procedure Memo §4(a) is **superseded** as of this letter's effective date. Travis County DHS MUST accept fair-hearing requests by phone, in person, or in writing on equivalent terms. Documentation of the request method MUST be added to the case file but MUST NOT gate processing.

## Finding 2: Insufficient appeal-rights notice content

A sample of 47 Notices of Adverse Action issued during 2024-Q2 contained appeal-rights language that omitted the **continuation-of-services** election, in violation of 7 CFR §273.10(g)(2)(iii). 31 of 47 notices reviewed lacked the language; 16 of 47 had it.

**Corrective action:** Every Notice of Adverse Action MUST include explicit language describing the household's right to elect continuation of services pending the hearing decision. Texas DHS MUST update its notice templates within 60 days. Template re-publication MUST be reviewed by FNS regional counsel.

## Effective dates and reporting

Both corrective actions are effective immediately (2024-11-15). Texas DHS MUST report compliance within 90 days, including:

- Updated Office Procedure Memo §4(a) language.
- Revised Notice of Adverse Action template with appropriate appeal-rights content (Findings 1 and 2 both addressed).
- Sample of 50 notices issued post-update for FNS verification.

## Authority and supersession

This letter is issued under FNS Director's regulatory authority per 7 CFR §271 and supersedes any conflicting state or local guidance for Texas SNAP cases until otherwise modified.

---

## Studio-side metadata

- **Authority:** federal (FNS sub-regulatory letter; supersedes any conflicting state/local guidance).
- **Cross-document supersession trigger:** §1 of office memo recert v3.2 is partially superseded.
- **Required workflow updates:**
  - NoticeRequirement (Notice of Adverse Action) MUST include continuation-of-services election language.
  - WorkflowIntent's appeal-request element MUST accept phone, in-person, AND written equivalently.
- **Drives Studio-side `Supersession` PolicyObject** per `policy-object-model.md` `Supersession` kind + `source-vault.md` `SA-MUST-source-006/007` (cross-document supersession via PolicyObject, reviewer-driven).
