# WOS Stream Compatibility Matrix

Workflow conformance uses `$wosWorkflow@X.Y` plus explicit embedded-block and
conformance-profile claims, as described in [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md).
Repository package versions identify builds; they are not separate authored
workflow compliance versions.

## Matrix

The following row is retained from the former four-stream model as historical
planning data. It has no linked exact-source or execution receipt here and is
**not a current supported-combination claim**.

| Kernel | Governance | AI | Advanced | Notes |
|---|---|---|---|---|
| 1.0.x | 1.0.x | 0.5.x | 0.1.x | Historical stream ranges; current compatibility unqualified by this row. |

This document currently records no exact-build compatibility qualification.
Ticket-specific test evidence remains scoped to its tested sources, artifacts
and behavior; it does not qualify every version in these ranges.

## How to read a row

A historical SemVer range such as `1.0.x` names the proposed stream series. It
cannot establish that every patch combination has been exercised together.
A current qualification must identify:

- the `$wosWorkflow` format version and embedded blocks exercised;
- the conformance profile or suite and concrete behavior checked;
- exact source commits and consumed artifact identities;
- the test receipt and its result, including known limitations.

For example, `$wosWorkflow@1.0 [governance, signature]` identifies a claim's format
and blocks. Evidence must still establish the claimed governance and signature
behavior. A bare-core processor claims only the applicable core conformance;
it does not acquire untested block support through the common format marker.

## Known-broken pairings

No exact-build incompatibility receipts are recorded in this document. An absent
receipt proves neither compatibility nor incompatibility. Record a discovered
failure with its exact inputs, affected behavior and test evidence; do not infer
a blanket failure from an untested stream range.

## Lifecycle rules

1. Preserve historical rows as history with their qualification status explicit.
2. Add current qualifications only with the evidence listed above. Source changes
   require an impact check before an earlier receipt can qualify the new sources.
3. Keep format conformance, package release identity and runtime adoption separate.
   This matrix does not assert deployment or live-provider acceptance.

## References

- [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md) — workflow claims and repository release identities.
- [ADR 0076](../thoughts/adr/0076-product-tier-consolidation.md) — accepted consolidation.
- [Release-trains implementation plan](thoughts/plans/2026-04-16-wos-release-trains.md) — historical four-stream proposal.
