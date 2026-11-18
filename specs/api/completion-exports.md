# Browser completion exports

`completion-exports.schema.json` (`https://schemas.formspec.io/wos-api/completion-exports/v1`) exports `CompletionExportRequest`, `CompletionExportJob` and `CompletionExportJobList` for requesting, polling, and downloading retained whole-case completion exports. An export is a durable server-side job: it takes minutes, so no HTTP request waits for it and no client holds retry state. Closing the tab loses nothing.

`POST /api/v1/completion-exports` accepts the closed `CompletionExportRequest` object containing only `processId`. It selects an actual workflow process by its full process URN or bare process TypeID, using the maintained `_common.schema.json` `ProcessPathId` definition. The human bearer credential is a transport input for this request only. Caller-supplied actors, scopes, credentials, request identities, retry keys, and checkpoint references are not request or header inputs. POST checks the caller's current permission, mints the job identity, and returns HTTP 202 with a `CompletionExportJob` immediately. If the caller already has a `running` job for the process, POST returns that job and starts no second export.

The server holds the job in a Restate workflow keyed by the job identity. At POST it signs a job grant: this principal may produce one export of this process as this job until a bounded time. No bearer is stored anywhere. The server runs the export with its own machine credentials, re-checks the caller's current identity and permission on every attempt, and retries transient failures within the grant's bounded window. The job identity is the retry identity of the retained request, so every attempt reconciles the same export instead of repeating it.

`GET /api/v1/processes/{process_id}/completion-exports` lists the caller's jobs for the process, newest first, so a new browser session finds running and finished exports without any client-held state.

`GET /api/v1/processes/{process_id}/completion-exports/{job_id}` reads one job under current caller authority. `job_id` is the job identity: exactly 32 lowercase hexadecimal characters. `CompletionExportJob` is three complete closed objects keyed by `status`; progress is `running` or done and nothing finer. Every variant carries the canonical process URN and the job identity.

- `succeeded` (carries `checkpointRef`): the retained custody receipt keyed by the job identity exists. Custody owns success, so a receipt always reads `succeeded`, even when the job also recorded a failure.
- `failed` (carries a stable `failureCode`): no receipt exists and the job ended. `export-forbidden` means a bad grant, lost permission or an operator cancel; `export-refused` means retained evidence or qualification refused; `export-conflict` means changed inputs; `export-retries-exhausted` means the bounded retry window ended; `export-receipt-missing` means the job recorded success but custody holds no receipt, an integrity fault. A job that ended `failed` reads `succeeded` if its custody receipt appears later: custody owns success.
- `running`: no receipt and no recorded failure.

A provider, verifier, token or policy outage is unavailability: status and list reads answer 503, never 403, and a client keeps polling; only a confirmed loss of current permission answers 403.

`GET /api/v1/processes/{process_id}/completion-exports/{job_id}/download` uses the same selectors to return the receipt-pinned ZIP of a `succeeded` job under current human archive authority; download is the only disclosure and is always live. Successful downloads use `Content-Type: application/zip`, `Cache-Control: no-store`, and `Content-Disposition: attachment; filename=case-completion.zip`. Job results also use `Cache-Control: no-store`.

The owning WOS service checks actual process association, retained request inputs, current human completion or archive permission, and retained evidence. The schemas validate selectors and result shapes; they do not establish authority, successful completion, archive custody, or delivery. The OpenAPI snapshot declares the four routes. The server owner registers implemented routes and emits its generated registry after the canonical schema is available.

`CompletionExportJob` represents each state as a complete closed object so maintained generators retain the exact fields required in that state.
