# Browser completion exports

`completion-exports.schema.json` (`https://schemas.formspec.io/wos-api/completion-exports/v1`) defines `CompletionExportOptions`, `CompletionExportRequest`, `CompletionExportJob`, `CompletionExportJobList` and `CompletionExportRequestConflict`. The server runs exports as durable jobs so the requester can close the browser and check the result later.

## Requests

`GET /api/v1/completion-exports/options` returns a closed `CompletionExportOptions` document (`Cache-Control: no-store`) listing the deployment's declared `purposes` and `recipientClasses` for the admitted scope so clients render declared options without hard-coding deployment vocabulary members.

`POST /api/v1/completion-exports` accepts a closed `CompletionExportRequest` with `processId`, `purpose` and `recipientClass`. `processId` selects an actual workflow process by full process URN or bare process TypeID using `ProcessPathId` from `_common.schema.json`. The authenticated requester declares a purpose and a recipient class, for example `quality-review` and `oversight`. The server MUST validate each declaration against its respective deployment vocabulary and record it with the authorization decision. The declarations grant no authority.

[`CompletionExportVocabularyId`](../../schemas/api/completion-exports.schema.json#/$defs/CompletionExportVocabularyId) defines the identifier grammar for both declarations and their deployment vocabulary members.

The server MUST authorize the current request before returning an existing job or admitting a new one. Within the deployment scope, the server MUST check and admit a job atomically for the resolved process and authenticated requester. An active job is one with no recorded terminal outcome. If the requester has an active job for the process, the server MUST return that job with HTTP 202 only when both submitted declarations exactly match the job's retained declarations. If either value differs, the server MUST return HTTP 409 with a `CompletionExportRequestConflict` problem (`WOS-2409`). The conflict admits no job. If the requester has no active job for the process, the server mints the job identity, starts the job and returns HTTP 202 with its `CompletionExportJob`.

## Authorization refusals

Authorization refusals use `application/problem+json` and the registered `error_code` values in the [public error registry](error-registry.md). Both authorization problem types MUST carry the decision's public reason in `context.reason_code`. The schema defines the closed set of refusal reasons from the Authorization Contract §§4.2–4.3 under ADR 0176. The context contains only that reason.

| Outcome | HTTP | Problem type | Problem code |
|---|---:|---|---|
| Authorization denies the request | 403 | `CompletionExportAuthorizationDenied` | `WOS-1403` |
| Authorization cannot decide | 503 | `CompletionExportAuthorizationIndeterminate` | `WOS-1503` |

The server MUST include a `Retry-After` header on an indeterminate authorization response. A well-formed declaration outside its deployment vocabulary yields `facts_invalid`. An unready authorization configuration yields `package_unavailable`. Both outcomes refuse the request before any job is admitted or returned.

## Job execution

The server signs a time-limited grant for the new job. The grant binds the requester, process, job identity, both declarations, the active policy digest and its activation revision. The server uses the human bearer only to handle the current HTTP request and runs the job under its own credentials. Each attempt first checks with custody for a committed export and adopts the result if present. Before any new export work, the server reevaluates the requester's current authority once for that attempt. The server retries transient failures during the grant's time window. The job identity is the retry identity for the retained request.

## Status and download

`GET /api/v1/processes/{process_id}/completion-exports` lists the caller's jobs for the process, newest first, so a new browser session finds running and finished exports without any client-held state.

`GET /api/v1/processes/{process_id}/completion-exports/{job_id}` reads one job under current caller authority. The job identity `job_id` consists of exactly 32 lowercase hexadecimal characters. `CompletionExportJob` has three complete closed variants keyed by `status`. Every variant, including entries returned by the list, carries the canonical process URN, job identity, `purpose` and `recipientClass` from the accepted request.

- `succeeded` (carries `checkpointRef`): the retained custody receipt keyed by the job identity exists. Custody owns success, so a receipt always reads `succeeded`, even when the job also recorded a failure.
- `failed` (carries a stable `failureCode`): no receipt exists and the job ended. `export-forbidden` means a bad grant, lost permission or an operator cancel; `export-refused` means retained evidence or qualification refused; `export-conflict` means changed inputs; `export-retries-exhausted` means the bounded retry window ended; `export-receipt-missing` means the job recorded success but custody holds no receipt, an integrity fault. A job that ended `failed` reads `succeeded` if its custody receipt appears later: custody owns success.
- `running`: no receipt and no recorded failure.

The authorization refusal rules also apply to status and list reads. A provider outage during a status or list read, including a timeout reading the job or its custody receipt, returns the shared `Problem` body with HTTP 503 (`WOS-1503`). The client MUST keep polling after a 503 and honor `Retry-After` when present.

`GET /api/v1/processes/{process_id}/completion-exports/{job_id}/download` uses the same selectors to return the receipt-pinned ZIP of a `succeeded` job under current human archive authority; download is the only disclosure and is always live. Successful downloads use `Content-Type: application/zip`, `Cache-Control: no-store`, and `Content-Disposition: attachment; filename=case-completion.zip`. Job results also use `Cache-Control: no-store`.

The owning WOS service checks actual process association, retained request inputs, current human completion or archive permission, and retained evidence. The schemas validate the public wire format. The server owner emits the OpenAPI document from the implemented routes and canonical schemas.

`CompletionExportJob` represents each state as a complete closed object so maintained generators retain the exact fields required in that state.
