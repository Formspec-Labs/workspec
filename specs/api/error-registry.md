# WOS Public API Error Registry

This registry owns stable `wosErrorCode` values for the public REST API. Error responses use RFC 7807 `application/problem+json` and conform to `schemas/api/error.schema.json`.

| Code | HTTP | Title | Meaning |
|---|---:|---|---|
| `WOS-1400` | 400 | Bad request | The request was syntactically valid HTTP but invalid for this endpoint. |
| `WOS-1401` | 401 | Unauthorized | The caller did not present valid authentication. |
| `WOS-1403` | 403 | Forbidden | The authenticated principal lacks authority for the requested resource or operation. |
| `WOS-1404` | 404 | Resource not found | The addressed resource does not exist or is not visible in the caller's scope. |
| `WOS-1408` | 408 | Timeout exceeded | A long-running operation exceeded its server-enforced wall-clock ceiling and was terminated. Currently emitted by report runs whose `maxDurationSeconds` is exceeded — the run transitions to `status == failed` with `failure.wosErrorCode == "WOS-1408"` and the runner records a `reportTimedOut` Facts-tier provenance literal with typed timeout payload. Cross-cite [`reports.md`](./reports.md). |
| `WOS-1409` | 409 | Conflict | The request conflicts with current resource state. |
| `WOS-1410` | 410 | Cursor expired | The supplied pagination cursor is no longer valid; restart pagination. |
| `WOS-1413` | 413 | Payload too large | The request body exceeds the endpoint's accepted size. |
| `WOS-1422` | 422 | Validation failed | The request body failed schema or semantic validation. |
| `WOS-1500` | 500 | Internal server error | The server hit an unexpected internal failure. |
| `WOS-1503` | 503 | Service unavailable | A required backing service is unavailable or refused the operation. |
