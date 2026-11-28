# WOS Public API Pinned Records

**Status:** Draft
**Schema:** [`api/pinned-records.schema.json`](../../schemas/api/pinned-records.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/pinned-records/v1`

## Purpose

This contract provides a domain-neutral, schema-driven reviewer read surface for listing and inspecting exact pinned filed Responses (`{responseId, responseHash}`) associated with an authorized WOS case (`fs-8s94`, ADR 0165, ADR 0170, ADR 0172, ADR 0176):

- A filed parent Response accepted into WOS may cite pinned child or referenced Responses in its `resolvedInstances[]` array (each citing `{responseId, definitionVersion, responseHash}`).
- Reviewers with deployment-configured authorization on the case (and on each referenced filing through the shared OpenFGA + Cedar authorization coordinator) can list the authorized pinned records for that case and inspect any exact `{responseId, responseHash}` record without ever hard-coding program- or domain-specific reviewer roles.
- Every read enforces strict ADR 0170 canonical byte-hash verification (`responseHash` over exact retained Response bytes, never re-serializing parsed JSON values) and fails closed on mismatch, corruption, or unauthorized access.

## Exported Models

All models live in [`api/pinned-records.schema.json`](../../schemas/api/pinned-records.schema.json) (`https://schemas.formspec.io/wos-api/pinned-records/v1`) and are closed (`additionalProperties: false`):

- `PinnedSubjectKey`: one typed subject key-value entry (`key`, `dataType` in `string` | `integer`, and matching `stringValue` or I-JSON `integerValue`).
- `PinnedSubject`: verified ADR 0172 subject identity (`id`, `type`, `keys: PinnedSubjectKey[]`).
- `PinnedRecordReference`: one authorized, byte-hash-verified pinned filing summary (`responseId`, `responseHash`, `definitionUrl`, `definitionVersion`, optional `instance`, optional `acceptedAt`, optional `subject`, optional `caseLedgerId`).
- `CasePinnedRecordsResponse`: case-scoped listing of authorized pinned records referenced by the case's verified parent filing (`caseLedgerId`, `parentResponseId`, `parentResponseHash`, `parentDefinitionUrl`, `parentDefinitionVersion`, `items: PinnedRecordReference[]`).
- `ExactPinnedRecordReadRequest`: single-record exact read request (`responseId`, `responseHash`, optional `expectedDefinitionUrl`, optional `expectedDefinitionVersion`).
- `ExactPinnedRecordReadResponse`: single-record exact read response (`caseLedgerId`, `responseId`, `responseHash`, `definitionUrl`, `definitionVersion`, optional `acceptedAt`, optional `subject`, `canonicalResponse` using the exact `AcceptedResponse` ADR 0170 schema, and `pinnedReferences: PinnedRecordReference[]`).

## Endpoints

```
GET  /api/v1/cases/{case_id}/pinned-records      -> CasePinnedRecordsResponse
POST /api/v1/cases/{case_id}/pinned-records/read -> ExactPinnedRecordReadResponse
```

Both endpoints require `CaseRead` scope eligibility and set `Cache-Control: no-store`. `POST /api/v1/cases/{case_id}/pinned-records/read` carries `{responseId, responseHash}` in the JSON request body so filing identifiers and content digests never appear in URL query strings or proxy access logs.

## Authorization and Integrity Verification

1. **Parent case authorization first**: Both routes evaluate `task.read` through the shared OpenFGA + Cedar authorization coordinator against `case:<case_id>` (with canonical `scope` relationship fact and `authority-head` freshness) BEFORE inspecting `responseHash` or `resolvedInstances[]`.
   - Coordinator `Deny` -> `403 Forbidden` (`WOS-1403` with `context.reason_code`). Unauthorized callers cannot probe hash validity or discover whether child references exist.
   - Coordinator `Indeterminate` -> `503 Service Unavailable` (`WOS-1503` with `Retry-After` and `context.reason_code`).
   - Unknown or cross-scope `case_id` -> `404 Not Found` (`WOS-1404`).
2. **ADR 0170 canonical byte-hash verification on the parent filing**: The server reuses `CaseIntakeService::evidence_view` and `original_response().handoff_hash()` over the exact immutable retained bytes (never re-hashing a parsed `Value`). Any mismatch between retained bytes and `IntakeHandoff.responseHash` fails closed with `409 Conflict` (`WOS-1409`).
3. **Batch read authorization and zero unentitled disclosure (`GET /api/v1/cases/{case_id}/pinned-records`)**:
   - Extracts completed `{responseId, responseHash}` citations from the verified parent Response's `resolvedInstances[]`.
   - Resolves and authorizes referenced records in batch: through the configured Formspec Server exact-record list seam (`POST /responses/exact-read/list`, which evaluates `unit.read` with `filed = true` under one freshness gate) and/or against scoped WOS intake reservations under one batched `task.read` coordinator gate (`authorize_case_batch`).
   - Unauthorized, missing, cross-scope, or mismatched-pin child records are omitted indistinguishably with zero hidden counts.
4. **Exact single-record read (`POST /api/v1/cases/{case_id}/pinned-records/read`)**:
   - Admits reading either the case's own verified parent filing OR a child filing explicitly pinned in the parent filing's verified `resolvedInstances[]` (authorized via Source's `POST /responses/exact-read` or WOS's scoped reservation under `task.read`).
   - Unreferenced, missing, or cross-scope `responseId` -> `404 Not Found` (`WOS-1404`).
   - Unauthorized child record -> `403 Forbidden` (`WOS-1403`) or `404 Not Found` (zero disclosure).
   - Requested `responseHash`, `expectedDefinitionUrl`, or `expectedDefinitionVersion` mismatch, or stored ADR 0170 byte-hash corruption -> `409 Conflict` (`WOS-1409`) without disclosing unverified Response bytes.
