# WOS Public API Review Dispositions

**Status:** Draft
**Schema:** [`api/review-dispositions.schema.json`](../../schemas/api/review-dispositions.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/review-dispositions/v1`

## Purpose

This contract provides an authorized, domain-neutral read surface for querying WOS review disposition outcomes and decision-record comments keyed by exact filing identity (`{responseId, responseHash}`) under ADR 0170 and ADR 0172.

- WOS outcome URIs are 100% **deployment-configured** and remain strictly outside FormSpec Core `ResponseStatus`.
- Every native filing lookup verifies that the immutable stored Response bytes hash to `IntakeHandoff.responseHash` under ADR 0170 (by reusing exact retained canonical Response bytes rather than re-serializing parsed JSON snapshots) and match the requested `responseHash`.
- Imported determinations (ADR 0172 §4.5) are served through the same closed wire shape and exact `{responseId, responseHash}` identity contract.

## Exported Models

All models live in [`api/review-dispositions.schema.json`](../../schemas/api/review-dispositions.schema.json) (`https://schemas.formspec.io/wos-api/review-dispositions/v1`) and are closed (`additionalProperties: false`):

- `CanonicalDispositionState`: closed enum of canonical surface disposition postures (`returned`, `accepted`, `closed-without-acceptance`, `reopened`).
- `DispositionMappingStatus`: closed enum reporting whether the deployment configuration mapped the recorded `outcomeUri` (`mapped`) or left it unmapped (`unmapped`).
- `DispositionSourceKind`: closed enum distinguishing native WOS decision records (`decision-record`) from scoped imported determinations (`imported-determination`).
- `DispositionFilingKey`: `{ responseId, responseHash }` exact filing identity (`responseHash` matches `^sha256:[0-9a-f]{64}$`).
- `DispositionComment`: one authorized review comment extracted from a decision record or imported determination (`id`, `body`, `recordedAt`, optional `authorRef`, optional `provenanceRecordRef`).
- `ReviewDisposition`: verified disposition record for one exact filing (`responseId`, `responseHash`, `outcomeUri`, `mappingStatus`, optional `state`, `sourceKind`, `decidedAt`, optional `decidedBy`, optional `caseLedgerId`, optional `processId`, optional `provenanceRecordRef`, optional `importedSource`, `comments`).
- `ReviewDispositionLookupRequest`: single-filing exact lookup body (`responseId`, `responseHash`, optional `includeComments`).
- `ReviewDispositionQueryRequest`: batch lookup body (`filings: DispositionFilingKey[]`, 1..200 items, optional `includeComments`) aligned with FormSpec `DispositionAuthority`.
- `ReviewDispositionQueryResponse`: batch lookup result (`items: ReviewDisposition[]`).

## Mapping and Precedence Rules

1. **Deployment-configured outcome URI mapping**: Deployments supply an explicit mapping from their WOS outcome URIs to `CanonicalDispositionState` (`returned`, `accepted`, `closed-without-acceptance`, `reopened`). Servers MUST NOT hard-code program- or domain-specific outcome URIs or reviewer role vocabularies.
2. **Unmapped outcome URIs**: When a verified filing has a recorded `outcomeUri` absent from deployment mapping configuration, the server MUST preserve `outcomeUri`, set `mappingStatus: "unmapped"`, and omit `state`. Unmapped outcomes are never silently guessed or coerced into a canonical state.
3. **Latest disposition per exact filing**: When a filing has multiple sequential decision-record entries (for example, `reopened` or `returned` recorded after an earlier `accepted` entry on the same filing), the latest chronological disposition for that `{responseId, responseHash}` wins. Dispositions on a newer filing (`responseId_2`) never overwrite the verified disposition on an older accepted filing (`responseId_1`).
4. **Authorized decision-record comments**: Comments are read solely from the authorized decision record or imported determination associated with the verified `{responseId, responseHash}`. When `includeComments` is `false`, `comments` is returned as an empty array.

## Endpoints

```
POST /api/v1/review-dispositions/query  -> ReviewDispositionQueryResponse
POST /api/v1/review-dispositions/lookup -> ReviewDisposition
```

Both endpoints accept JSON bodies (`POST`) so filing identifiers and content digests are never exposed in URL query strings or proxy access logs, and both set `Cache-Control: no-store`.

## Authorization and Integrity Verification

- Every lookup evaluates `disposition.read` through the shared authorization coordinator against the owning case in the caller's admitted scope (with `scope` relationship fact and authority-head freshness).
- **Single-filing lookup (`POST /api/v1/review-dispositions/lookup`)**:
  - Coordinator `Deny` -> `403 Forbidden` (`WOS-1403` with `context.reason_code`). Authorization is evaluated before comparing `responseHash` so unauthorized callers cannot probe hash validity.
  - Coordinator `Indeterminate` -> `503 Service Unavailable` (`WOS-1503` with `Retry-After` and `context.reason_code`).
  - Missing `responseId`, cross-scope filing, or wrong `responseHash` (for an authorized caller) -> `404 Not Found` (`WOS-1404`).
  - Retained original Response bytes failing ADR 0170 byte-hash verification -> `409 Conflict` (`WOS-1409`) or fail-closed refusal without disclosing unverified outcomes.
- **Batch query (`POST /api/v1/review-dispositions/query`)**:
  - Returns only authorized, ADR 0170 byte-hash-verified matches in `items`.
  - Unknown `responseId`, cross-scope filings, wrong `responseHash`, and coordinator `Deny` items are omitted indistinguishably without disclosing hidden records.
  - Coordinator `Indeterminate` fails the query closed with `503 Service Unavailable` (`WOS-1503` with `Retry-After`).
