# End-to-End Reference Example: County Medicaid Redetermination

**Scenario:** Lincoln County Department of Social Services processes 12,000 Medicaid redeterminations per year. Federal law requires periodic re-evaluation of eligibility. The county currently uses paper forms mailed to recipients, manual data entry by clerks, and a legacy mainframe for eligibility determination. The process takes 45 days on average. Federal regulations require completion within 30 business days, with due process protections for adverse decisions.

**Stack:** Formspec (intake) + WOS (governance) + Temporal (execution)

The customer never sees "Temporal" or "WOS." They see a workflow that collects data, routes cases, enforces rules, and produces auditable decisions.

---

## Act 1: Setup (Program Administrator)

### Story 1.1: Design the intake form

> As a program administrator, I describe what information the redetermination needs to collect, and the system generates an adaptive intake form that asks the right questions based on the household's situation.

Maria, the program administrator, opens Formspec Studio and types: "Medicaid annual redetermination for households. Need household composition, income sources, employment status, disability status, and asset verification. Must comply with 42 CFR 435.916."

The system generates a multi-page form:
- Page 1: Household members (repeating group -- add/remove members)
- Page 2: Income sources per member (conditional -- employment income, self-employment, SSI/SSDI, child support)
- Page 3: Asset verification (conditional -- only if income exceeds threshold)
- Page 4: Disability and special circumstances
- Page 5: Document upload (pay stubs, tax returns, disability verification)
- Page 6: Attestation and signature

Each field knows what regulation governs it. The income threshold references the Federal Poverty Level table, which changes every January. The form's policy parameters are date-indexed -- a redetermination filed in December 2025 uses the 2025 FPL; one filed in January 2026 uses the 2026 FPL. Maria does not configure this manually. The temporal parameter sidecar handles it.

### Story 1.2: Define the workflow

> As a program administrator, I define what happens after someone submits their redetermination, including who reviews it, what rules apply, and what happens if they're found ineligible.

Maria selects "Rights-impacting workflow" as the impact level. The system responds: "Due process protections are required. I'll set up mandatory notice, individualized explanation, appeal rights, and continuation of benefits during appeal."

She defines the workflow states:
- **Submitted** -- redetermination received
- **Document review** -- AI extracts data from uploaded documents; human verifies
- **Eligibility determination** -- caseworker evaluates against current rules
- **Approved** -- continued coverage
- **Adverse determination** -- notice sent, appeal window opens
- **Appeal** -- independent reviewer evaluates
- **Closed** -- final resolution

She tags the states: `document-review` gets the `review` tag. `eligibility-determination` gets the `determination` tag. `adverse-determination` gets the `adverse-decision` tag. The governance rules attach automatically based on tags.

She does not write BPMN XML. She does not configure Temporal workflows. She defines states, tags them, and the platform handles execution.

### Story 1.3: Configure the AI agent

> As a program administrator, I add an AI document extraction agent that reads uploaded pay stubs and tax returns, with guardrails that prevent it from making eligibility decisions.

Maria adds a document extraction agent:
- **Type:** generative (LLM-based)
- **Autonomy:** assistive (recommends, human confirms)
- **Confidence floor:** 0.80 (below this, flag for human review)
- **Permissions:** may extract income amounts, employer names, dates
- **Prohibition:** must not determine eligibility or produce denial recommendations
- **Obligation:** every extracted value must cite the source document page and location
- **Fallback:** retry once, then create a human task

The system warns: "This is a rights-impacting workflow. Agent autonomy is capped at assistive. Agent participation will be disclosed in any adverse decision notice."

Maria accepts. She does not configure Temporal interceptors or deontic constraint evaluation ordering. The governance layer handles enforcement.

### Story 1.4: Set up review protocols

> As a program administrator, I choose how reviewers evaluate cases so they form genuine assessments rather than rubber-stamping agent recommendations.

Maria selects the `independentFirst` review protocol for the document review step. This means: when a caseworker opens an extracted document, they see the original pay stub and a blank form. They enter the values they read. Only after submitting their independent extraction does the system reveal the AI's extraction. Discrepancies are highlighted.

She adds `dualBlind` for eligibility determinations above $50,000 annual household income -- two caseworkers evaluate independently, results reconciled.

She sets quality sampling at 10% -- one in ten cases is randomly selected for supervisory review regardless of outcome.

---

## Act 2: Respondent Experience

### Story 2.1: Receive the redetermination notice

> As a Medicaid recipient, I receive a notice that my annual redetermination is due, with a link to complete it online instead of mailing paper forms.

James, a Medicaid recipient, receives an email: "Your Medicaid coverage is due for annual review. Complete your redetermination online by March 15, 2026. If we don't hear from you, your coverage may be affected. Complete it here: [link]."

The link opens the Formspec form. James sees Page 1: "Who lives in your household?" He adds himself, his spouse, and two children.

### Story 2.2: Fill out the form with AI assistance

> As a respondent, I upload my pay stubs and the system reads them for me, asking me to confirm the numbers it found.

James reaches the income page. He uploads three pay stubs. The system processes them and shows: "I found the following from your pay stubs. Please confirm these are correct."

| Document | Employer | Gross Pay | Pay Period |
|----------|----------|-----------|------------|
| Stub 1 | Lincoln Hardware | $1,847.23 | Jan 1-15 | 
| Stub 2 | Lincoln Hardware | $1,923.45 | Jan 16-31 |
| Stub 3 | Lincoln Hardware | $1,847.23 | Feb 1-15 |

Each value shows a confidence indicator. The employer name shows high confidence. One gross pay figure shows medium confidence (the scan was slightly blurry). James corrects it from $1,923.45 to $1,923.54 and confirms.

James does not know that a governance proxy intercepted the extraction, validated each value against the Formspec contract, checked deontic constraints (the agent was permitted to extract these fields), and recorded provenance noting the agent's confidence per field and James's confirmation.

### Story 2.3: Complete and submit

> As a respondent, I review a summary of everything I entered, sign the attestation, and submit.

James reaches the review page. The system shows a summary of all entered data, flags one missing item ("You didn't upload a disability verification for your spouse -- is this still applicable?"), and presents the attestation: "I certify under penalty of perjury that the information provided is true and complete."

James signs using the signature pad. He submits. The system confirms: "Your redetermination has been submitted. You will receive a decision within 30 business days. Your current coverage continues until a decision is made. Reference number: MED-2026-0847."

Behind the scenes: the Formspec submission is validated against the contract, the Coprocessor creates a WOS case instance, maps the response fields to the case file, and fires the `submitted` event. Temporal persists the case state and starts the 30-business-day SLA timer.

---

## Act 3: Case Processing (Caseworker)

### Story 3.1: Claim a case from the queue

> As a caseworker, I open my queue, see pending redeterminations sorted by deadline, and claim one to review.

Angela, a caseworker, opens her reviewer dashboard. She sees 47 pending redeterminations. Cases approaching their 30-day deadline are highlighted. She claims MED-2026-0847 (James's case).

The system assigns the case to Angela and starts the task SLA timer. If Angela doesn't complete the review within 5 business days, the case escalates to her supervisor.

### Story 3.2: Review extracted documents (independent-first)

> As a caseworker, I review the uploaded documents and enter my own extraction before seeing what the AI found.

Angela opens the document review screen. She sees James's three pay stubs displayed on the left. On the right, blank fields: Employer, Gross Pay, Pay Period. No AI extraction is visible.

Angela reads the pay stubs and enters her values. She submits her independent extraction.

Now the system reveals the AI's extraction alongside hers. Two of three values match exactly. One differs by $0.09 (the corrected value James confirmed). The system highlights the discrepancy. Angela checks the original stub, confirms James's correction was right, and accepts the merged result.

Angela does not know that the `independentFirst` protocol was enforced by WOS governance, that the interface suppression was mandated by AI Integration S10.2, or that her independent extraction and the reconciliation are both recorded in the Reasoning provenance tier.

### Story 3.3: Make an eligibility determination

> As a caseworker, I review the household's financial data against the current eligibility rules and record my determination with the specific reasons.

Angela reviews the case file: household of four, combined annual income $47,200. The current FPL for a household of four is $31,800. The Medicaid threshold is 138% FPL = $43,884.

James's household income exceeds the threshold. Angela selects "Ineligible -- income exceeds 138% FPL" and enters her rationale: "Household income of $47,200 exceeds the 138% FPL threshold of $43,884 for a household of four. Three pay stubs verified. No deductions applicable."

The system checks: Angela holds a valid delegation of authority for Medicaid eligibility determinations (Governance S11). The determination is not her own case (separation of duties). The system records the Reasoning tier: rules applied (42 CFR 435.916, state Medicaid plan Section 4.2), evidence consulted (3 pay stubs, household composition attestation), criteria checked (income vs 138% FPL).

The system generates the Counterfactual tier: "If household income were $43,884 or below (a reduction of $3,316), the household would qualify. The determination was based solely on income relative to the FPL threshold. Household composition, disability status, race, ethnicity, and age did not affect this determination."

### Story 3.4: The adverse decision notice generates automatically

> As a caseworker, I finalize the adverse determination and the system generates the legally required notice with specific reasons, appeal instructions, and continuation-of-benefits language.

Angela clicks "Finalize determination." The system generates a notice:

> **Notice of Adverse Determination -- Medicaid Redetermination**
>
> Dear James [Last Name],
>
> Your Medicaid coverage has been determined ineligible effective April 15, 2026.
>
> **Reason:** Your household income of $47,200 per year exceeds the Medicaid eligibility threshold of $43,884 (138% of the Federal Poverty Level for a household of four).
>
> **What you could change:** If your household income decreases to $43,884 or below, you may reapply and would qualify under current rules.
>
> **What did NOT affect this decision:** Your age, race, ethnicity, disability status, and household composition did not affect this determination.
>
> **AI disclosure:** An AI document extraction system assisted in reading your uploaded pay stubs. A human caseworker independently verified all extracted values and made the eligibility determination.
>
> **Your right to appeal:** You may appeal this decision within 30 days by [instructions]. During the appeal period, your current Medicaid coverage will continue unchanged.
>
> Reference: MED-2026-0847 | Caseworker: Angela [Last Name] | Authority: State Medicaid Plan Section 4.2

This notice was assembled by the Explanation Assembly algorithm (Runtime S9) from the Reasoning and Counterfactual provenance tiers, ranked by authority (federal statute first, then state plan). The AI disclosure was injected because the workflow is rights-impacting and an agent participated (AI Integration S12). The continuation-of-benefits language was required because `continuationOfServices` is true on the appeal mechanism (Governance S3.6).

Angela did not write this notice. She made a determination with rationale. The governance layer produced the notice.

---

## Act 4: Appeal (Respondent + Independent Reviewer)

### Story 4.1: File an appeal

> As a respondent who received an adverse decision, I file an appeal through the same system, and my current benefits continue during the appeal.

James receives the notice by email and mail. He believes his income calculation is wrong -- one of the pay stubs was from overtime that has since ended. He clicks "Appeal this decision" and enters: "The pay stub from January 16-31 included overtime that is no longer available. My regular biweekly pay is $1,847.23, which projects to $48,028 annually, but my actual income is lower because overtime was temporary."

James uploads his most recent pay stub showing no overtime: $1,847.23 for the current period.

The system creates an appeal case linked to MED-2026-0847 (parent/child case relationship). James's Medicaid coverage continues. The system starts a new SLA timer for the appeal review.

Behind the scenes: the `$related.stateChanged` event notifies the parent case that an appeal was filed. The hold policy `pending-related-case` activates. Temporal persists the linked case state and manages the appeal SLA timer.

### Story 4.2: Independent appeal review

> As a supervisory reviewer, I evaluate the appeal independently. I cannot see the original caseworker's reasoning until I form my own assessment.

David, a supervisory reviewer who was not involved in the original determination, is assigned the appeal. The system enforces independence: David cannot be Angela (separation of duties) and does not see Angela's original reasoning until he completes his own evaluation.

David reviews the original pay stubs and James's new pay stub. He calculates: if overtime was temporary and recent pay is $1,847.23 biweekly, annual projected income is $48,028 -- still above the threshold. But David notes the overtime period was limited and requests verification from the employer.

David places the appeal on hold: `pending-external-verification`, expected duration 14 business days. The system sends a verification request to Lincoln Hardware.

The employer responds: James's overtime ended in February and is not expected to recur. Regular annual salary: $48,028.

David's determination: income of $48,028 still exceeds $43,884. Appeal denied. The notice includes: the specific calculation, the employer verification, and instructions for further appeal to the state hearing office.

---

## Act 5: Ongoing Operations (Supervisor + Compliance Officer)

### Story 5.1: Quality sampling catches a pattern

> As a supervisor, I'm notified that the quality sampling system flagged a pattern in Angela's cases.

The system randomly selected 10% of Angela's determinations for supervisory review. Over the past quarter, the quality reviews reveal: Angela's denial rate for households reporting self-employment income is 34% higher than the team average, and she consistently rounds self-employment income estimates upward.

The system generates a quality alert (not an equity violation -- this is caseworker-specific, not demographic). Angela's supervisor schedules a calibration session.

### Story 5.2: Equity monitoring detects a disparity

> As an equity officer, I receive an alert that denial rates differ across regions in a way that exceeds our threshold.

The equity guardrail (Advanced Governance S3) monitors denial rates by county region. After 6 months: the northern region denial rate is 28%, the southern region is 19%. The disparity exceeds the configured 0.10 threshold.

The equity officer receives an alert with the statistical details. This is asynchronous -- no individual case was blocked. The alert triggers a structured remediation review: are the regions served by different caseworkers? Do northern-region applicants have systematically different income profiles? Is the threshold calibrated correctly for rural vs urban cost of living?

The equity officer's investigation and findings are recorded as provenance. The investigation may lead to policy parameter adjustments (different FPL adjustments by region), caseworker training, or a determination that the disparity reflects legitimate income differences.

### Story 5.3: Drift detection demotes the AI agent

> As a program administrator, I'm notified that the document extraction agent's accuracy has dropped and it's been automatically demoted.

The drift monitor (Drift Monitor sidecar) tracks the extraction agent's accuracy against human reviewer corrections. Over the past 30 days, the agreement rate dropped from 94% to 81%. The PSI (Population Stability Index) crossed the configured threshold.

The system automatically demotes the agent from assistive to manual (it can provide context but its extractions are no longer shown as recommendations). A provenance record captures the demotion trigger. Maria receives a notification: "Document extraction agent demoted due to accuracy drift. Review the agent configuration or schedule recalibration."

Maria investigates: the agent's model version was updated by the provider (model version policy was `approved`). The new version handles handwritten documents differently. She schedules a shadow deployment of the new version against 100 cases before restoring assistive autonomy.

### Story 5.4: Rubber-stamp detection

> As a supervisor, I'm alerted that a caseworker appears to be confirming AI extractions without genuine review.

The rubber-stamp monitor (Drift Monitor sidecar) tracks reviewer behavior: time spent on independent extraction, modification rate, disagreement rate. One caseworker averages 12 seconds per document review with a 99.2% agreement rate (team average: 3 minutes, 91% agreement).

The supervisor receives an alert. This is a governance finding, not a disciplinary one -- the system provides evidence, not judgment. The supervisor reviews the flagged cases and determines whether the caseworker needs retraining on the independent-first protocol.

---

## Act 6: Audit and Compliance

### Story 6.1: Federal audit of the redetermination process

> As a compliance officer responding to a federal audit, I produce a complete audit package for any case showing every decision, every rule applied, and every actor involved.

A CMS auditor requests the complete record for MED-2026-0847. The compliance officer pulls the provenance:

**Facts tier:** Every state transition, every action, timestamps, actors, inputs, outputs. "Case created at 14:23 UTC by system. Document extraction agent invoked at 14:24 UTC with 3 documents. Caseworker Angela claimed case at 09:15 UTC. Independent extraction recorded at 09:32 UTC. AI extraction revealed at 09:33 UTC. Reconciliation completed at 09:35 UTC. Eligibility determination recorded at 09:47 UTC. Adverse notice generated at 09:47 UTC. Appeal filed at 16:02 UTC by respondent."

**Reasoning tier:** "Rules applied: 42 CFR 435.916 (authority: federal statute), State Medicaid Plan Section 4.2 (authority: state regulation). Evidence consulted: 3 pay stubs (employer-verified), household composition attestation. Criteria: household income $47,200 vs threshold $43,884 (138% FPL for household of 4). Determination: ineligible."

**Counterfactual tier:** "Positive: income reduction of $3,316 would change outcome. Negative: age, race, ethnicity, disability status, household composition did not affect determination."

**Narrative tier (non-authoritative):** "The AI extraction agent read three pay stubs and extracted income figures with 0.92 average confidence. [Marked: this is a model-generated summary, not an authoritative record.]"

The auditor receives a structured JSON package with every record linked by provenance ID. The Explanation Assembly algorithm produces a human-readable summary ranked by authority (federal statute first). The auditor can verify that due process was followed, that the AI's role was disclosed, that an independent review occurred, and that the appeal was handled by someone other than the original caseworker.

### Story 6.2: The auditor asks "what if?"

> As an auditor, I ask whether the same case processed under the new 2026 income thresholds would have a different outcome.

The auditor requests a what-if analysis: replay MED-2026-0847 against the 2026 FPL table. The temporal parameter sidecar resolves the 2026 threshold: 138% of $32,580 = $44,960. James's income of $47,200 still exceeds this threshold. The outcome does not change.

The auditor notes this in their report. The analysis was possible because WOS separates the policy parameters from the workflow logic -- the same case can be replayed against any parameter version without modifying the workflow definition.

---

## What the customer never sees

Throughout this narrative, the customer interacted with:
- A form (Formspec)
- A reviewer dashboard (SaaS UI)
- Notifications (email)
- Reports (provenance exports)

They never configured:
- Temporal workflows, signals, or activities
- BPMN XML, sequence flows, or gateways
- Deontic constraint evaluation ordering
- Provenance tier composition
- Timer management or crash recovery
- Event sourcing or deterministic replay

The program administrator defined states, tagged them, chose review protocols, and configured an AI agent. The governance layer enforced due process, review protocols, separation of duties, confidence thresholds, drift monitoring, equity guardrails, and explanation assembly. The execution engine kept the workflow running reliably across months of calendar time, surviving server restarts, handling concurrent cases, and firing SLA timers on schedule.

The technology is invisible. The governance is the product.
