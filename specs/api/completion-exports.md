# Browser completion exports

`completion-exports.schema.json` (`https://schemas.formspec.io/wos-api/completion-exports/v1`) exports `CompletionExportRequest` and `CompletionExportResult` for requesting, reading, and downloading retained whole-case completion exports.

`POST /api/v1/completion-exports` accepts the closed `CompletionExportRequest` object containing only `processId`. It selects an actual workflow process by its full process URN or bare process TypeID, using the maintained `_common.schema.json` `ProcessPathId` definition. Human bearer credentials and the required `Idempotency-Key` header remain transport inputs. Caller-supplied actors, scopes, credentials, request identities, and checkpoint references are not request-body fields. A successful POST returns HTTP 200 with `ready`.

`GET /api/v1/processes/{process_id}/completion-exports/{request_id}` reads one retained request under current caller authority. Its `process_id` accepts the same full URN or bare TypeID; `request_id` contains exactly 32 lowercase hexadecimal characters. `CompletionExportResult` always carries the canonical full process URN, retained request identity, closed `status`, and `checkpointRef`. `pending` requires a null checkpoint reference; `ready` requires a nonempty string. Missing fields, other states, and additional fields refuse schema validation.

`GET /api/v1/processes/{process_id}/completion-exports/{request_id}/download` uses the same process and request selectors to return the existing receipt-pinned ZIP under current human archive authority. Successful downloads use `Content-Type: application/zip`, `Cache-Control: no-store`, and `Content-Disposition: attachment; filename=case-completion.zip`. Request and status results also use `Cache-Control: no-store`.

The owning WOS service checks actual process association, retained request inputs, current human completion or archive permission, and retained evidence. The schemas validate selectors and result shapes; they do not establish authority, successful completion, archive custody, or delivery. The OpenAPI snapshot declares the three routes. The server owner registers implemented routes and emits its generated registry after the canonical schema is available.

`CompletionExportResult` represents `pending` and `ready` as two complete closed objects, in that order. Each variant requires all four fields. This representation preserves the accepted wire values and lets maintained generators retain required process and request identities together with the exact status and checkpoint type.
