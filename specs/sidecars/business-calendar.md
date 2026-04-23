---
title: WOS Business Calendar Sidecar
version: 1.0.0-draft.1
date: 2026-04-10
status: draft
---

# WOS Business Calendar Sidecar v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-10
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Business Calendar Sidecar defines the business-day, holiday, and operating-hours model that WOS processors use for SLA evaluation and temporal parameter resolution. Government workflows measure deadlines in business days, not wall-clock time: a 30-day response window excludes weekends and federal holidays, and operating hours constrain when timers advance. The kernel's timer mechanism (Kernel S9.7) and the governance layer's SLA evaluation (Governance S10.3) and temporal parameter resolution (Governance S13.3) reference this sidecar when it is present.

This is a sidecar document, not a layer. It provides configuration data consumed by existing governance mechanisms without introducing new seams, document types, or processing concepts. When no Business Calendar sidecar is present, SLA evaluation uses wall-clock time (Governance S10.3).

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Kernel Specification v1.0 and the WOS Workflow Governance Specification v1.0. It defines business calendar configuration consumed by governance processors for SLA enforcement and temporal resolution.

---

## 1. Introduction

### 1.1 Purpose

Government agencies and regulated organizations operate on business calendars that differ from the Gregorian calendar in three ways:

- **Business days.** Weekends and holidays are non-working days. A "30-day" deadline means 30 business days, not 30 calendar days.
- **Operating hours.** Some SLAs measure elapsed working hours within a business day. A 4-hour response SLA means 4 hours during which the office is open.
- **Holiday schedules.** Federal, state, and agency-specific holidays vary by jurisdiction. A single workflow may span jurisdictions with different holiday calendars.

The Workflow Governance Specification references business calendar semantics in two places:

- **Governance S10.3** (Task SLA Definitions): "SLA evaluation uses business calendar days when a Business Calendar sidecar is present."
- **Governance S13.3** (Composition with Business Calendar): "Temporal parameter resolution composes with the Business Calendar sidecar."

This sidecar provides the calendar data those mechanisms consume.

### 1.2 Scope

**Within scope:** business day definitions (work week), holiday schedules (fixed and floating), operating hours, timezone, and composition rules for multi-calendar scenarios.

**Out of scope:** timer implementation (Kernel S9.7, Lifecycle Detail S6); SLA enforcement algorithms (Governance S10.3); temporal parameter resolution algorithms (Governance S13.2); timezone conversion logic (implementation concern).

### 1.3 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Document Structure

A Business Calendar sidecar is a JSON document identified by the `$wosBusinessCalendar` document type marker. It targets a WOS Kernel Document via the `targetWorkflow` property and provides calendar data consumed by governance processors.

### 2.1 Required Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosBusinessCalendar` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document this calendar applies to. |
| `timezone` | string (IANA) | REQUIRED | IANA timezone identifier for this calendar. All time calculations use this timezone. |
| `workWeek` | array of string | REQUIRED | Ordered list of working days. Values from the set: `monday`, `tuesday`, `wednesday`, `thursday`, `friday`, `saturday`, `sunday`. |

### 2.2 Optional Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `version` | string | OPTIONAL | Version of this Business Calendar document. |
| `title` | string | OPTIONAL | Human-readable name. |
| `description` | string | OPTIONAL | Human-readable description. |
| `holidays` | array of Holiday | OPTIONAL | Holiday schedule. Days listed here are non-working regardless of `workWeek`. |
| `operatingHours` | OperatingHours | OPTIONAL | Working hours within a business day. When absent, a business day is the full 24-hour period. |
| `effectiveDate` | string (date) | OPTIONAL | Date this calendar becomes effective. |
| `expirationDate` | string (date) | OPTIONAL | Date this calendar expires. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

---

## 3. Work Week

The `workWeek` property declares which days of the week are business days. The processor uses this to determine whether a given date is a working day.

### 3.1 Day Names

Day names MUST be lowercase English day names from the set: `monday`, `tuesday`, `wednesday`, `thursday`, `friday`, `saturday`, `sunday`.

### 3.2 Standard Work Week

The standard US federal work week is `["monday", "tuesday", "wednesday", "thursday", "friday"]`. Agencies with non-standard schedules (e.g., 4-day work weeks, Saturday operations) declare their actual work week.

### 3.3 Evaluation

A date is a **business day** if and only if:

1. The day of the week appears in `workWeek`, AND
2. The date does not appear in the `holidays` array.

---

## 4. Holiday Schedule

### 4.1 Holiday Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | REQUIRED | Human-readable holiday name. |
| `date` | string (date) | CONDITIONAL | Fixed date (ISO 8601). Required when `rule` is not specified. |
| `rule` | string | CONDITIONAL | Recurrence rule for floating holidays. Required when `date` is not specified. |
| `observed` | boolean | OPTIONAL | Whether this is an observed date (e.g., Monday observance of a weekend holiday). Default: `false`. |

### 4.2 Fixed vs. Floating Holidays

**Fixed holidays** specify a `date` property with an ISO 8601 date. These are one-time entries. A calendar covering multiple years requires one entry per year per fixed holiday.

**Floating holidays** specify a `rule` property with a recurrence descriptor. Standard rules:

| Rule | Meaning | Example |
|------|---------|---------|
| `nthWeekday(n, weekday, month)` | The nth occurrence of weekday in month. | `nthWeekday(3, monday, january)` = Martin Luther King Jr. Day |
| `lastWeekday(weekday, month)` | The last occurrence of weekday in month. | `lastWeekday(monday, may)` = Memorial Day |

Processors MUST support at least `nthWeekday` and `lastWeekday` rules. Extension rules use the `x-` prefix.

### 4.3 Observed Holidays

When a fixed-date holiday falls on a non-working day, the `observed` property on a separate entry marks the actual day off. For example, if July 4 falls on Saturday, the observed holiday entry for Friday July 3 has `observed: true`.

---

## 5. Operating Hours

### 5.1 Purpose

Operating hours define the working period within a business day. When present, SLA duration calculations count only elapsed time during operating hours. When absent, each business day counts as a full day regardless of time.

### 5.2 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `start` | string (time) | REQUIRED | Start of the operating period. ISO 8601 time (HH:MM). |
| `end` | string (time) | REQUIRED | End of the operating period. ISO 8601 time (HH:MM). |

### 5.3 Evaluation

Operating hours are interpreted in the calendar's declared `timezone`. The operating period is `[start, end)` -- inclusive of start, exclusive of end.

When `operatingHours` is present and an SLA `targetDuration` is specified in hours (e.g., `PT4H`), the processor MUST count only the time within operating hours. When the SLA is specified in days (e.g., `P5D`), operating hours are informational but do not change the business-day count.

---

## 6. SLA Composition

### 6.1 Business-Day SLA Evaluation

When a Business Calendar sidecar is present for a workflow, the governance processor (Governance S10.3) MUST evaluate SLA durations using business days instead of wall-clock time:

1. Start the SLA clock at the task creation timestamp.
2. For each elapsed calendar day, increment the SLA counter only if the day is a business day (S3.3).
3. When `operatingHours` is present and the SLA duration is in hours, count only elapsed time within operating hours.
4. The SLA warning threshold and breach policy (Governance S10.3) evaluate against the business-day-adjusted duration.

### 6.2 Temporal Parameter Resolution Composition

The Business Calendar sidecar composes with temporal parameter resolution (Governance S13.3). When resolving a parameter whose `resolutionDateRef` points to a case state date, the processor MAY adjust the resolution date to the nearest prior business day when the referenced date falls on a non-business day. This behavior is OPTIONAL and deployment-specific.

---

## 7. Multi-Calendar Scenarios

A workflow MAY be targeted by multiple Business Calendar sidecars (e.g., federal and state calendars, or one calendar per US state in a national benefits program). When multiple calendars target the same workflow, the processor selects which calendars apply to a given case via the `appliesWhen` FEL expression on each calendar.

### 7.1 Calendar Selection Algorithm

For each SLA evaluation or temporal-parameter resolution, the processor MUST:

1. Collect every Business Calendar sidecar whose `targetWorkflow` matches the active kernel document URI.
2. Filter out calendars whose `expirationDate` is in the past (§8.1 item 5).
3. Filter out calendars whose `effectiveDate` is in the future relative to the SLA evaluation timestamp.
4. For each remaining calendar, evaluate `appliesWhen` against the case file. A calendar with no `appliesWhen` field applies unconditionally (equivalent to `appliesWhen: "true"`).
5. The set of calendars whose expression evaluated true is the **applicable set** for this case.
6. If the applicable set is empty, the processor MUST fall back to wall-clock time (§8.2 absence behavior) and SHOULD record a warning in provenance citing the workflow URI and case-file fields consulted.

The selection is deterministic given the same case file and clock — no implementation-defined ordering or tiebreakers.

### 7.2 Composition When Multiple Calendars Apply

When the applicable set contains more than one calendar:

1. A date is a non-working day if **any** applicable calendar marks it as a holiday or non-working day.
2. Operating hours use the **most restrictive intersection** of all applicable calendars' operating periods.
3. Timezone disagreement is a deployment error — applicable calendars MUST share the same `timezone`. A processor MUST raise a configuration error if they do not.

### 7.3 Worked Example: Multi-State Benefits Workflow

A national benefits adjudication workflow ships three Business Calendar sidecars:

```json
[
  { "targetWorkflow": "https://agency.gov/benefits/v1", "appliesWhen": "applicant.address.state == 'NY'", "timezone": "America/New_York", "...": "..." },
  { "targetWorkflow": "https://agency.gov/benefits/v1", "appliesWhen": "applicant.address.state == 'CA'", "timezone": "America/Los_Angeles", "...": "..." },
  { "targetWorkflow": "https://agency.gov/benefits/v1", "appliesWhen": "true", "timezone": "America/New_York", "title": "Federal fallback", "...": "..." }
]
```

For a case where `applicant.address.state == 'NY'`, the applicable set is the NY calendar plus the federal fallback (timezone agreement OK because both are `America/New_York`). For a case where `applicant.address.state == 'CA'`, the applicable set is the CA calendar plus the federal fallback — and the processor raises a configuration error because the timezones disagree, surfacing the modelling mistake at evaluation time rather than silently picking one.

---

## 8. Conformance

### 8.1 Processor Requirements

A processor that supports Business Calendar sidecars:

1. MUST parse and validate the document against the Business Calendar schema.
2. MUST use `workWeek` and `holidays` to determine business days for SLA evaluation (Governance S10.3).
3. MUST support at least `nthWeekday` and `lastWeekday` holiday rules.
4. SHOULD support `operatingHours` for hour-based SLA calculations.
5. MUST ignore an expired calendar (when `expirationDate` is in the past) and fall back to wall-clock time.

### 8.2 Absence Behavior

When no Business Calendar sidecar targets a workflow, all SLA calculations use wall-clock time. Every day is a working day. This is the default behavior defined by Governance S10.3.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS Governance] Formspec Working Group, "WOS Workflow Governance Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".

### Informative References

- [IANA TZ] IANA, "Time Zone Database", https://www.iana.org/time-zones.
- [ISO 8601] ISO, "Date and time -- Representations for information interchange", ISO 8601:2019.
