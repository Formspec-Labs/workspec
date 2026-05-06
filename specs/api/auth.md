# WOS Public API Auth

**Status:** Schema authored — pending implementation pair (server + portal).
**Schema:** [`api/auth.schema.json`](../../schemas/api/auth.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/auth/v1`
**OpenAPI:** [`wos-public-api.openapi.json`](../../api/wos-public-api.openapi.json) (snapshot follow-on per ADR 0082 D-13 today; auto-emit per PLN-0401).
**ADR anchor:** [ADR 0082](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) D-9 (identity and scope; JWT bearer; `ActorRef`), D-12 (closed-with-vendor-extension `LoginKind` / `MfaChallengeKind` / `AuthAssuranceLevel`), D-14 (no inline redefinition; `ActorScope` cross-`$ref` from `actor.schema.json`), D-15 step 6 (dashboard / applicant / auth / bundle / audit close the user-facing entry-point surface), D-16 (`Idempotency-Key` REQUIRED on `POST /login` and `POST /scope-swap`).
**Gating ADR (load-bearing):** [ADR 0068 — Stack Tenant and Scope Composition](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) (**Proposed**) — D-1 (tenant outermost), D-2 (scope bundle four-tuple), D-3 (actors span tenants; authorization is per-tenant), D-3.1 (`assuranceLevel` taxonomy reused as `AuthAssuranceLevel`). The auth domain is the API surface where ADR 0068 lands first because every login carries scope and every scope-swap exercises ADR 0068 D-3 per-tenant authority. This spec authors against the **proposed** shape per ADR 0082 D-15 greenfield discipline; citations note the ADR 0068 D-X (Proposed) anchor inline. The schema does not block on ADR 0068 acceptance.

## Purpose

The auth domain owns **token + session lifecycle** — login, logout, refresh, scope-swap, introspection, and current-scope read. Authorization-as-mechanism (the JWT bearer token + scope headers `X-WOS-Tenant`, `X-WOS-Organization`, `X-WOS-Workspace`, `X-WOS-Environment` per ADR 0082 D-9) is settled and unchanged; this spec authors the lifecycle endpoints around that mechanism.

The scope shape (`ScopeContext`) gates on ADR 0068 D-2 (Proposed): Tenant -> Organization -> Workspace -> Environment per VISION §V scope hierarchy. Per ADR 0082 D-15 greenfield discipline, the schema authors against the proposed shape and tolerates server-side absence of fields the deployment has not yet adopted. The `ActorScope` shape is `$ref`d directly from `actor.schema.json#/$defs/ActorScope` — the canonical home for the scope-tuple wire form (ADR 0082 D-14).

## Resource Shape

### `LoginRequest` / `LoginResponse`

`LoginRequest` is the credential body. The `kind` field is the discriminator for the credential payload's interpretation: `password` → `username` + `password`; `webauthn` → `webauthnAssertion`; `oidc-sso` → `oidcCode`. Per ADR 0082 D-12 `kind` is closed-with-vendor-extension; reserved literals exhaust the deployment-relevant first-factor surface.

The contract intentionally carries credential payloads as named-seam fields (`password`, `webauthnAssertion`, `oidcCode`) rather than a discriminated-union body so the schema reads as a single closed object — adding a kind requires extending this object plus the `LoginKind` enum. Per ADR 0082 D-11 the credential fields are optional on the wire (presence depends on `kind`); the server validates the `kind`/payload combination at receipt and rejects mismatches with `WOS-1422`.

`LoginRequest.requestedScope` is OPTIONAL — when omitted the server resolves a default scope from the principal's primary membership. When present, it MUST be a tenant the principal has authority in (ADR 0068 D-3 — Proposed).

`LoginResponse` carries the issued bearer token, its expiry, the active scope, the principal's `actorRef`, the `assuranceLevel` attested by the session, and (when MFA is required) an `mfaChallenge` envelope. When `mfaChallenge` is present, `accessToken` is omitted: the client redeems the challenge via a follow-up `POST /api/v1/auth/login` carrying the same `Idempotency-Key` plus `mfaChallengeId` + `mfaResponse`. When `mfaChallenge` is absent, the token is immediately usable.

### `LogoutRequest`

`scope` is closed-no-extension: `this-session | all-sessions`. Closed by design — vendor-specific logout scopes would create audit ambiguity. `all-sessions` corresponds to the global-logout pattern that bumps `auth_epoch` per the WOS server auth contract (`workspec-server/crates/wos-server/PARITY.md` ▎ Auth contract): one transaction bumps the epoch, revokes sessions, and the realtime `kernel:update` re-runs `verify` per event so revocation applies without waiting for token expiry.

### `RefreshRequest` / `RefreshResponse`

`RefreshRequest.refreshToken` is the opaque token previously issued by `LoginResponse.refreshToken` or a prior `RefreshResponse.refreshToken`. Single-use within the deployment retention window; the server SHOULD rotate the refresh token on each redemption. The contract intentionally keeps the refresh token in the body (not a header or a cookie) so it survives mTLS-style transports without depending on browser context.

`RefreshResponse.scope` MUST equal the redeemed token's prior scope — refresh does NOT swap scope. Use `POST /api/v1/auth/scope-swap` for scope changes.

### `ScopeSwapRequest`

The `targetScope` field is the tuple the session swaps to. Per ADR 0068 D-2 (Proposed) the **case** scope bundle `(Tenant, DefinitionId, KernelId, LedgerId)` is immutable for the case lifetime; this endpoint swaps the *session's* scope for actors who hold authority across multiple Organization / Workspace / Environment scopes within a tenant. Per ADR 0068 D-1 + D-3 (Proposed) cross-tenant swaps are rejected with `WOS-1403` — actors span tenants but authority is per-tenant; cross-tenant reads are impossible by construction.

`ScopeSwapRequest.targetScope.tenant` MUST equal the bearer token's current tenant. The endpoint emits a governance audit event and issues a fresh token bound to the new scope.

### `ScopeContext`

Returned by `GET /api/v1/auth/scope`. Carries `actorRef` (URN of the principal authenticated by the token), `scope` (the `ActorScope` tuple per ADR 0068 D-2 — Proposed), `assuranceLevel` (`AuthAssuranceLevel`), `loginKind` (the credential family that obtained the original session), and `mfaSatisfied`. Useful when the client needs the canonical reference shape for the principal alongside the scope.

### `TokenIntrospection`

Returned by `GET /api/v1/auth/introspect` for the bearer token presented on the request. Modeled after RFC 7662 (OAuth 2.0 Token Introspection) with WOS-specific scope and assurance fields. The `active` flag is the load-bearing field: when false, all other fields except possibly `expiresAt` MAY be omitted (the server is asserting the token does not authenticate). Service-account and workload calls (per VISION §V) use this endpoint to verify their token's continuing validity without round-tripping a real read.

## Closed Taxonomies

| Taxonomy | Source | Extension |
|---|---|---|
| `LoginKind` | new — `password \| webauthn \| oidc-sso` | closed-with-vendor-extension `^x-[a-z]+-` (D-12) |
| `MfaChallengeKind` | new — `totp \| webauthn \| sms-otp \| email-otp \| push` | closed-with-vendor-extension `^x-[a-z]+-` (D-12) |
| `TokenKind` | new — `bearer` | closed-no-extension (single-element enum; major bump for second token kind) |
| `AuthAssuranceLevel` | aligned with ADR 0068 D-3.1 (Proposed) `IdentityAttestation.assuranceLevel` taxonomy: `low \| standard \| high \| very-high` | closed-with-vendor-extension `^x-[a-z]+-` (D-12) |
| `LogoutRequest.scope` | new — `this-session \| all-sessions` | closed-no-extension (audit clarity) |
| `ActorScope` | `$ref` to `actor.schema.json#/$defs/ActorScope` | reused — gates on ADR 0068 D-2 (Proposed) |
| `ActorScope.environment` | inherited from `actor.schema.json` — `sandbox \| staging \| prod` | closed-no-extension (VISION §V) |

`MfaChallengeKind` reserved literals at v1: `totp` (RFC 6238 time-based OTP); `webauthn` (FIDO2 / passkey assertion as second factor when first factor is `password`); `sms-otp` (SMS one-time code); `email-otp` (email one-time code); `push` (out-of-band push approval). `sms-otp` is recorded as the lowest-assurance literal the vocabulary exposes; high-assurance deployments SHOULD prefer `webauthn` or `totp`.

`LoginKind.webauthn` is normative for the respondent applicant flow per VISION §V: respondent identity is bound via WebAuthn PRF for per-class encryption (ADR 0074 native field-level transparency). Staff identity typically uses `oidc-sso` against an external IdP. Service-account / workload identity does NOT log in through this surface — those principals carry their tokens via mTLS or workload-attested issuance per VISION §V; `auth.schema.json` is the human-facing surface.

## Endpoints

| Method | Path | Body in / out | Idempotency-Key |
|---|---|---|---|
| `POST` | `/api/v1/auth/login` | `LoginRequest` -> `LoginResponse` | **REQUIRED** (D-16) |
| `POST` | `/api/v1/auth/logout` | `LogoutRequest` -> 204 | OPTIONAL (naturally idempotent on visible state) |
| `POST` | `/api/v1/auth/refresh` | `RefreshRequest` -> `RefreshResponse` | OPTIONAL (idempotent on `refreshToken`) |
| `POST` | `/api/v1/auth/scope-swap` | `ScopeSwapRequest` -> `LoginResponse` (re-issued bearer + new scope) | **REQUIRED** (D-16) |
| `GET` | `/api/v1/auth/introspect` | -> `TokenIntrospection` (for the request's bearer token) | n/a |
| `GET` | `/api/v1/auth/scope` | -> `ScopeContext` | n/a |

`POST /api/v1/auth/login` requires `Idempotency-Key` per ADR 0082 D-16: login is externally side-effecting (it issues a token, opens a session, may emit an authentication-record event) and a network retry without idempotency would create duplicate sessions. A repeat request within the retention window returns the original `LoginResponse` unchanged. The same `Idempotency-Key` is reused on the MFA-redemption follow-up (the second `POST /login` carrying `mfaChallengeId` + `mfaResponse`) so the two-request flow is idempotent end-to-end.

`POST /api/v1/auth/scope-swap` requires `Idempotency-Key` per ADR 0082 D-16: scope-swap is externally side-effecting (it emits a governance audit event and issues a fresh token bound to the new scope).

`POST /api/v1/auth/logout` and `POST /api/v1/auth/refresh` are naturally idempotent on the visible state (the session is gone; the refresh token redeems to the same fresh pair within the deployment retention window). `Idempotency-Key` is OPTIONAL but RECOMMENDED.

`GET` endpoints are idempotent by construction. `GET /api/v1/auth/introspect` returns the introspection of the bearer token presented on the request; there is no `?token=` query parameter to avoid token leakage in URL logs.

## Identifiers

The auth domain does not mint resource URNs. `LoginResponse.actorRef` and `ScopeContext.actorRef` are `actor:(human|service-account|workload|support):...` URNs from `_common.schema.json` per ADR 0082 D-9; `accessToken` and `refreshToken` are opaque server-issued strings (not URNs).

## Pagination

The auth surface has no list endpoints. Cursor pagination per ADR 0082 D-7 does not apply.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1401`: `accessToken` invalid, expired, or revoked (introspection returns `active == false` with HTTP 200 instead).
- `WOS-1403`: scope-swap rejected — for example a cross-tenant `targetScope.tenant` (ADR 0068 D-1 — Proposed), or a tenant the principal lacks authority in (ADR 0068 D-3 — Proposed).
- `WOS-1404`: bearer principal not found (rare — implies the underlying actor record was retired during the session).
- `WOS-1409`: MFA-redemption mismatch — `mfaChallengeId` not found or expired.
- `WOS-1410`: refresh-token expired or already redeemed in a non-rotating-refresh deployment.
- `WOS-1422`: request failed schema validation, including `kind`/credential-payload mismatch (`kind: password` without `username`/`password`; `kind: webauthn` without `webauthnAssertion`; etc.) or `requestedScope` malformed.
- `WOS-1429`: login rate-limited (the auth surface is the canonical brute-force defense surface).
- `WOS-1503`: identity-provider backend unavailable (typical of `kind: oidc-sso` when the upstream IdP is down).

## Greenfield Discipline

Per ADR 0082 Context section and the owner's greenfield-contracts memory: prior `case-portal` / `workspec-server` auth DTOs are NOT preserved. The schema makes the worst shapes prior art tends to accumulate structurally inexpressible:

1. **No `Record<string, string>` credential bag.** The `LoginRequest` shape has named fields per credential family (`username`/`password`/`webauthnAssertion`/`oidcCode`); vendor extensions are constrained to the closed-with-vendor-extension `LoginKind` enum's `^x-[a-z]+-` arm.
2. **No nested `actor: { id, type, name }` shape on `LoginResponse`.** The response carries `actorRef` (URN per ADR 0082 D-9); identity details live once in the identity/governance subsystem and consumers dereference via the actor resource.
3. **No `nullable scope` ambiguity.** Either the response carries a structured `ScopeContext.scope` tuple per ADR 0068 D-2 (Proposed), or the field is OMITTED on early-deployment servers. The wire format never collapses "no scope yet" and "scope is null" onto the same value.
4. **No anonymous `permissions: string[]` array.** Authorization grants live in the governance/RBAC ladder (`Owner / Admin / Author / Reviewer / Analyst / Submitter` per VISION §V) and are exposed through governance endpoints, not flattened into the auth response body.
5. **No "remember me" optional cookie field.** Session lifetime is governed by `expiresAt` on the access token and the rotation cadence on the refresh token; cookie-based persistence is a deployment concern (ADR 0082 D-16 "Compression and CORS are deployment concerns").

## Non-Goals

- **Authorization implementation details.** The RBAC ladder, OpenFGA tuples, scope-membership grants — owned by the governance domain (ADR 0082 D-15 step 5). The auth domain authenticates and reports scope; it does not author scope grants.
- **IdP federation configuration.** The `oidc-sso` kind assumes an external IdP is configured per-deployment; the configuration surface (issuer URLs, client IDs, JWKS endpoints) is a deployment / operations concern, not part of the public API.
- **Service-account / workload token issuance.** Per VISION §V workload identity for service-to-service calls (e.g. wos-server → trellis-store) is required and never API-key. Issuance happens through workload-attested mechanisms (mTLS, SPIFFE-style attestation), not through `POST /api/v1/auth/login`. The introspection endpoint serves all token kinds equally.
- **Password reset / credential recovery.** A future amendment slot — likely a `POST /api/v1/auth/recover` endpoint with its own MFA flow — when the wedge requires it. Currently out of scope.
- **WebAuthn registration.** The applicant or staff member registers WebAuthn credentials via the identity/governance subsystem; this surface only **uses** registered credentials (via `LoginKind.webauthn`) for authentication.
- **Cross-tenant linking.** ADR 0068 D-1 (Proposed) makes cross-tenant reads impossible by construction; the auth surface scopes by tenant only. Actors span tenants per ADR 0068 D-3 (Proposed) but authority is per-tenant.
- **Streaming session events.** Out of scope per ADR 0082 D-16.

## ADR Amendments

None required at the contract layer. The auth domain is the first surface where ADR 0068 D-2 / D-3 / D-3.1 (Proposed) shape becomes load-bearing on the public API; this spec **cites** ADR 0068 inline rather than blocking on its acceptance, per ADR 0082 D-15 greenfield discipline. When ADR 0068 promotes to Accepted, the inline "(Proposed)" annotations move to "(Accepted)" without any wire-shape change — the schema is already authored to the proposed shape.

The auth surface closes ADR 0082 D-15 step 6 alongside the parallel dashboard / applicant / bundle / audit agents. The "credential payload as named-seam fields" pattern (closed `kind` discriminator + per-kind named property) is a precedent for any future closed-discriminator-with-payload-variants shape.
