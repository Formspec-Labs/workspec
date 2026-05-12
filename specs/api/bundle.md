# WOS Public API Bundle

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/bundle.schema.json`](../../schemas/api/bundle.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/bundle/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-4 (URN), D-7 (cursor pagination), D-10 (`never` sentinel), D-12 (closed taxonomies), D-15 step 6 (bundle + audit close the export surface), D-16 (Idempotency-Key on POST).
**Gating ADR:** [Trellis ADR 0007 — Certificate-of-Completion Composition](../../../trellis/thoughts/adr/0007-certificate-of-completion-composition.md) (the bundle's authoritative byte composition); [Trellis Core §18 Export Package Layout](../../../trellis/specs/trellis-core.md) (`trellis-export/1` ZIP archive); [Trellis Core §19 Verification Algorithm](../../../trellis/specs/trellis-core.md) (offline verification of the byte stream).

## Purpose

A `Bundle` is the case-export download seam. The **bundle itself is Trellis-emitted CBOR** (`trellis-export/1`; Trellis Core §18.3 `ExportManifestPayload.format`) — a byte-exact, append-only, offline-verifiable archive carrying the case's events, checkpoints, signing-key registry, inclusion / consistency proofs, and (when applicable) the certificate-of-completion presentation artifact (Trellis ADR 0007). This API surface is the **metadata + download seam** around that byte stream: the JSON resource that lets a client enumerate available bundles, kick off a build, poll its status, and stream the bytes.

The bundle is a derived projection of authoritative Trellis events — verification runs against the byte stream's COSE_Sign1 signatures and Merkle proofs (Trellis Core §19), not against any JSON metadata this API surfaces. The `Bundle.certificateOfCompletionDigest` and `Bundle.byteSize` are convenience metadata that a verifier MAY cross-check; the byte stream is canon.

Per ADR 0082 D-15 step 6, this domain pairs with `audit.schema.json` (cross-case query) to close the export / cross-case API surface.

## Resource Shape

`Bundle` carries identity, case linkage, lifecycle status, sealing metadata, and the certificate-of-completion digest. Required fields: `id`, `processId`, `status`, `lifecycleStateAtExport`, `exportedAt`, `tierInclusion`, `mediaType`. Optional fields, omitted when absent: `requestedAt`, `expiresAt`, `byteSize`, `certificateOfCompletionDigest`, `verifierProfile`, `failure`.

`status` is a closed lifecycle: `pending` (queued, build not started), `building` (generator running, bytes not yet sealed), `available` (byte stream sealed and downloadable), `expired` (retention window elapsed, bytes purged but metadata retained for audit), `failed` (build terminated with error). Closed enum with no extension seam — bundle lifecycle is normative.

`tierInclusion` is a closed-with-vendor-extension taxonomy (ADR 0082 D-12): `facts-only` (Kernel S8 only), `facts-reasoning` (Kernel S8 + Governance S6.2), `full-tier-set` (all four tiers including Governance S6.4 and AI Integration S13). The kernel/governance/AI tier semantics are mirrored from `provenance.schema.json` — bundles compose, they do not redefine.

`mediaType` is a closed enum currently pinned to `application/cbor` (Phase 1 Trellis export). Future formats register here under a major schema bump, NOT a vendor extension — bundle byte format is normative.

`expiresAt` uses the ADR 0082 D-10 `never` sentinel for retention-indefinite bundles. After expiry the byte stream MAY be purged; metadata is retained for audit, status flips to `expired`.

`certificateOfCompletionDigest` is the lowercase SHA-256 hex digest of the Trellis presentation artifact computed under domain tag `trellis-presentation-artifact-v1` (Trellis ADR 0007 `presentation_artifact.content_hash`). Omitted when the case is signing-only / no certificate-of-completion was emitted.

## Identifiers

`Bundle.id` is a `urn:wos:<typeid>` URN per ADR 0092 D-1. `Bundle.processId` is a `urn:wos:<typeid>` URN of the owning case.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `GET` | `/api/v1/bundles` | `BundleListOptions` (query) -> `BundlePage` | n/a |
| `GET` | `/api/v1/bundles/{urn}` | -> `Bundle` (metadata only) | n/a |
| `POST` | `/api/v1/bundles` | `BundleCreateRequest` -> `Bundle` (status `pending` or `building`) | **REQUIRED** |
| `GET` | `/api/v1/bundles/{urn}/download` | -> `application/cbor` byte stream | n/a |

`GET /api/v1/bundles` is cursor-paginated per ADR 0082 D-7. Filters: `processId` (one case), `lifecycleStateAtExport` (e.g., `decided`), `status` (most commonly `available` for download-ready bundles), `createdAfter` (UTC lower bound on `requestedAt`). No `total`, no `page`. Cursors are deploy-lifetime stable; `WOS-1410` on expiry triggers client restart from the top.

`GET /api/v1/bundles/{urn}` returns the metadata envelope only — never the byte stream. Use this to poll a `pending` / `building` bundle until `status == available`.

**Completion notification seam.** Bundle completion (transition to `available`, `expired`, or `failed`) emits a `Notification` with `type == bundle-completed` (`notification.schema.json`); the carrying `Notification.bundleId` is REQUIRED on that type per the conditional `if`/`then` block on `Notification` (ADR 0082 D-11) and points at the addressable bundle resource. Clients SHOULD subscribe to the notification feed (`GET /api/v1/notifications`) instead of polling `Bundle.status` per case — at scale the feed is the discoverable observable for bundle completion. Polling stays valid (the metadata envelope at `GET /api/v1/bundles/{urn}` is authoritative); the notification seam is the recommended path.

`POST /api/v1/bundles` kicks off the bundle build. **`Idempotency-Key` is REQUIRED** per ADR 0082 D-16 because bundle builds are externally-visible side effects (Trellis events emit, key-registry snapshots, attachment lineage resolution). A repeat request within the retention window returns the original `Bundle` resource unchanged. Body: `BundleCreateRequest { processId, tierFilter?, verifierProfile? }`. Response: a `Bundle` resource with `status` typically `pending` or `building`. Clients poll `GET /api/v1/bundles/{urn}` until `status == available`.

`GET /api/v1/bundles/{urn}/download` is the **binary content seam** — distinct from every other endpoint in the public API. Response `Content-Type: application/cbor`; response body is the Trellis CBOR byte stream verbatim. **This is NOT a JSON response.** OpenAPI describes it as `{"type": "string", "format": "binary"}` under the `application/cbor` content key:

```yaml
/bundles/{urn}/download:
  get:
    operationId: downloadBundle
    responses:
      '200':
        description: Trellis-emitted CBOR export archive (`trellis-export/1`).
        content:
          application/cbor:
            schema:
              type: string
              format: binary
```

The download endpoint returns `409 Conflict` (`WOS-1409`) when `status != available`, `404` (`WOS-1404`) when the bundle URN is not visible to the caller's scope, `410 Gone` when the bundle has `expired` and bytes are purged.

## Pagination

`GET /api/v1/bundles` uses cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410`.

## Errors

All non-2xx responses (except `/download`'s binary stream which uses HTTP status codes alone) use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes:

- `WOS-1404`: bundle URN does not exist or is not in the caller's scope.
- `WOS-1409`: download requested before `status == available`, or bundle is `expired` and bytes were purged.
- `WOS-1410`: cursor expired, or download attempted on an expired bundle.
- `WOS-1422`: `BundleCreateRequest.processId` is not a valid `urn:wos:<typeid>` URN, or the requested `tierFilter` is rejected by the deployment posture.
- `WOS-1503`: bundle build backend (Trellis exporter) unavailable.

## Greenfield Discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: prior `case-portal` and `workspec-server` bundle DTOs are NOT preserved. The schema makes the worst shapes that prior art accumulated structurally inexpressible:

1. **No inline byte stream.** `Bundle` carries metadata only; the byte stream is a separate endpoint. There is no `bytes: string (base64)` escape hatch tempting clients to ship CBOR through JSON.
2. **No open `metadata: Record<string, unknown>` bag.** Every field is named; vendor extensions are constrained to `tierInclusion`'s `^x-[a-z]+-` arm. `mediaType` is closed (no vendor extension) because byte format is normative.
3. **No nullable timestamp ambiguity.** `expiresAt` uses the ADR 0082 D-10 `never` sentinel, not `null`, so "indefinite retention" and "not yet known" do not collapse onto the same wire value.
4. **`certificateOfCompletionDigest` is the SHA-256 of the artifact, not an opaque string.** Pattern-locked to `^[a-f0-9]{64}$` so a verifier can compute the comparison without dispatch on encoding.

## Non-Goals

- **Multi-case bundles.** One `Bundle` exports exactly one case. Cross-case audit queries are owned by `api/audit.schema.json`; cross-case bundle composition (federation, multi-tenant exports) is a future ADR slot.
- **Anchor target binding.** The Trellis export internally references its anchor target (Trellis Core §16.3 `external_anchors`); the API does not surface anchor configuration. Operators pick OpenTimestamps / Rekor / Trillian per-deployment per Trellis `AnchorAdapter`.
- **C2PA interop sidecar.** Tracked at Trellis ADR 0008 / TODO #21. When ratified, presentation-artifact C2PA composition lands in the Trellis byte stream — the API surface here is unchanged because the bundle is a Trellis byte stream by reference.
- **Append paths.** Bundles are read-shaped from the API perspective. The Trellis exporter generates the bytes; the API never accepts a client-supplied bundle.
- **Verification.** A verifier runs against the downloaded byte stream offline (Trellis Core §19). The API does not run verification on the caller's behalf.

## ADR Amendments

None required. The bundle/audit pair closes ADR 0082 D-15 step 6 alongside the parallel dashboard / applicant / auth agents. The binary download endpoint (`application/cbor`, `format: binary`) is the first non-JSON response in the API surface — the OpenAPI snapshot follow-on declares the response per the wire-format pattern shown in Endpoints above.
