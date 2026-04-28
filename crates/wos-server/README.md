# wos-server

Reference HTTP + Socket.IO backend for **WOS** (Workflow Orchestration Standard). Wraps `wos-runtime`'s evaluator and exposes the REST + realtime contract the `studio/` React app consumes.

**Status:** 0.1 reference implementation. Spec-correct response shapes across every endpoint; several seams ship with no-op defaults pending real implementations. Not production-hardened — see [`PARITY.md`](PARITY.md) for the per-feature status table.

---

## Quick start

```bash
# Default: SQLite in-memory, JWT auth, port 4000, no seed data.
cargo run -p wos-server

# Seed from fixtures/ on first boot.
WOS_SEED=true cargo run -p wos-server

# Persistent SQLite + mock auth for local studio dev.
WOS_DATABASE_URL=sqlite://wos.db \
WOS_AUTH=mock \
cargo run -p wos-server

# Export provenance for a specific instance.
cargo run -p wos-server -- export <instance-id> --format prov-o
```

The server listens on `http://0.0.0.0:$PORT` (default `4000`). Health probe is `GET /healthz`.

---

## Architecture

```text
        studio/ (React)                     external clients
              │                                     │
              └──────── HTTP + Socket.IO ───────────┘
                              │
                      ┌───────▼────────┐
                      │  wos-server    │  ← this crate
                      │  axum + sioxide│
                      │  SQLite + JWT  │
                      └───────┬────────┘
                              │ DI seams (ten host-interface traits)
                      ┌───────▼────────┐
                      │  wos-runtime   │  evaluator loop, timer ticks, seam composition
                      └───────┬────────┘
                              │
                      ┌───────▼────────┐
                      │   wos-core     │  pure evaluation, typed models, provenance
                      └────────────────┘
```

`wos-server` is the **composition root** — it wires storage, auth, services, and the runtime together, then serves HTTP + Socket.IO. All actual workflow semantics live in `wos-runtime` / `wos-core`. Everything server-specific (SQLite storage, JWT auth, Socket.IO realtime) plugs into wos-runtime via dependency injection.

See the DI seam surface in [`thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`](../../thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md).

---

## API surface

Everything mounted under `/api/*`. Route groups:

| Group | Module | Scope |
|---|---|---|
| `/auth/*` | `http/auth.rs` | Login, refresh, logout, `/me` |
| `/bundles/*` | `http/bundles.rs` | Kernel + companion document bundles |
| `/instances/*` | `http/instances.rs` | Case instance CRUD, events, provenance |
| `/tasks/*` | `http/tasks.rs` | Task lifecycle (assign, claim, complete) |
| `/governance/*` | `http/governance.rs` | Governance reads, delegations, deontic violations |
| `/dashboard/*` | `http/dashboard.rs` | Reviewer dashboard aggregations |
| `/applicant/*` | `http/applicant.rs` | Applicant-facing projections |
| `/ai-chat/*` | `http/ai_chat.rs` | Gemini-backed chat (optional) |
| `/lint/*` | `http/lint.rs` | Document lint diagnostics |
| `/conformance/*` | `http/conformance.rs` | Conformance fixture runner |
| `/calendar/*` | `http/calendar.rs` | Business calendar sidecar |
| `/notifications/*` | `http/notifications.rs` | Notification template rendering |
| `/deontic/*` | `http/deontic.rs` | Deontic constraint evaluation |
| `/assurance/*` | `http/assurance.rs` | Assurance layer + subject continuity chains |
| `/integration/*` | `http/integration.rs` | Integration profile inbound + invoke |
| `/agents/*` | `http/agents.rs` | Agent registry + AI lifecycle |
| `/advanced/*` | `http/advanced.rs` | L3 stubs (SMT, equity, zones) |

**Auth scope (WS-003 close-out):** the writes-require-Supervisor rule applies to instances / tasks / holds / agents / assurance / governance / equity / integration. The `lint`, `notifications`, `calendar`, and `conformance` POST routes remain anonymous (WS-014 / WS-091 territory).

**Realtime**: Socket.IO at `/socket.io/`. Namespaces registered in `realtime/`. Task events, cursor presence, governance updates.

**Reference wire contract**: `studio/src/services/WosBackend.ts` + `WosPorts.ts`. Handler response shapes match these contracts.

### Pagination semantics

Two distinct categories — caller-driven cursors vs server-side fleet scans — and they are not interchangeable:

- **Caller-paginated (cursor-driven).** `GET /api/instances` walks one SQLite page at a time; the caller supplies `page` / `pageSize` and decides when to stop. `Page<T>::total` is best-effort under concurrent writes (the `COUNT(*)` and the paged `SELECT … LIMIT/OFFSET` are separate statements; clients that need stable pagination should request the next page promptly or page by id).
- **Server-side fleet-scan aggregates.** `GET /api/tasks`, `GET /api/dashboard/metrics`, `GET /api/dashboard/stage-metrics`, and `POST /api/equity/evaluate` aggregate across **every** matching instance. Internally they call [`storage::list_instances_all_pages`](src/storage/mod.rs), which loops the underlying paged read until exhaustion (200-row SQLite clamp per page). Results are not capped at the first page.

The two categories share the same SQLite read path; the difference is who drives the loop. See [`PARITY.md`](PARITY.md) ▎ Server aggregation + surface refresh for the audit row.

### Realtime auth model

Under `WOS_AUTH=jwt`, each `kernel:update` socket event re-runs `AuthProvider::verify` against the connect-time access token (the same revocation model as HTTP — see [`tests/auth_jwt.rs`](tests/auth_jwt.rs)). A token revoked or epoch-bumped after the socket connects is rejected on the **next** event, not retroactively. Studio clients **must reconnect** to attach a fresh access token after logout, role change, or password reset; otherwise the existing socket continues to fail every event until the connection is dropped. Mock auth ignores the token and is permissive for local studio.

---

## Configuration

All config via flags or env vars (flags win). Full list in `src/config.rs`.

| Env var | Flag | Default | Purpose |
|---|---|---|---|
| `PORT` | `--port` | `4000` | TCP listen port |
| `WOS_FIXTURES_DIR` | `--fixtures-dir` | `fixtures` | Seed + conformance fixture root |
| `WOS_STORAGE` | `--storage` | `sqlite` | Storage backend (`sqlite` only today) |
| `WOS_DATABASE_URL` | `--database-url` | `sqlite::memory:` | SQLite connection string |
| `WOS_AUTH` | `--auth` | `jwt` | Auth provider (`jwt` \| `mock`) |
| `WOS_JWT_SECRET` | `--jwt-secret` | *(required for jwt)* | HS256 secret (raw or hex) |
| `WOS_JWT_ACCESS_TTL_SECS` | `--jwt-access-ttl-secs` | `900` | Access token lifetime |
| `WOS_JWT_REFRESH_TTL_SECS` | `--jwt-refresh-ttl-secs` | `2592000` | Refresh token lifetime (30d) |
| `WOS_CORS_ORIGIN` | `--cors-origin` | `http://localhost:3000` | CORS allow-origin (`*` disables credentials; a valid origin enables credentials) |
| `WOS_CORS_STRICT` | `--cors-strict` | `false` | When `true`, refuse startup if `WOS_CORS_ORIGIN` is not `*` and not a valid HTTP header value (otherwise invalid origins log a warning and fall back to permissive CORS) |
| `WOS_BEARER_STRICT` | `--bearer-strict` | `false` | When `true`, any `Authorization` header must be `Bearer <token>` with a token that verifies; otherwise `401` (default ignores bad bearer and treats the caller as anonymous) |
| `WOS_SEED` | `--seed` | `false` | Seed DB from `fixtures/` on empty (demo users share password **`wos-dev`** — dev only) |
| `WOS_AI_CHAT` | `--ai-chat` | `disabled` | AI chat backend (`disabled` \| `gemini`) |
| `GEMINI_API_KEY` | `--gemini-api-key` | *(required for gemini)* | Gemini API key |
| `WOS_CURSOR_THROTTLE_MS` | `--cursor-throttle-ms` | `50` | Socket.IO cursor throttle |
| `WOS_TIMER_POLL_MS` | `--timer-poll-ms` | `1000` | Timer tick interval |

Planned additions (from the active plan, not shipped yet):

- `WOS_SIGNER` — signer backend (`noop` \| `ed25519-file` \| `external`). Track A1.
- `WOS_RENDERER` — report renderer (`json` \| `html`). Track A2.
- `WOS_SUBMIT_POLICY` — `default` (ledger-gated per §15.7) \| `permissive`. Track B1 placement.

---

## Storage

SQLite only today. Schema under `migrations/`:

- `0001_init.sql` — users, sessions, kernels, instances, provenance, delegations.
- `0002_runtime_tables.sql` — runtime aux, event queue, tasks, agents, identity facts, equity cache, inbound integration.
- `0003_intake_records.sql` — durable intake-acceptance records.
- `0004_user_auth_epoch.sql` — per-user `auth_epoch` for coordinated JWT invalidation.

User rows: on `id` conflict, `upsert_user` updates only `email`, `name`, `role`, and `avatar` — it **does not** overwrite `password_hash` or `auth_epoch`, so profile edits cannot bypass token invalidation. Password changes must use `Storage::set_user_password_hash`; logout uses `bump_user_auth_epoch` plus session revocation.

Planned: `TaskStore` trait extraction (plan G8), drift-report storage (plan B8). Backend-pluggability beyond SQLite waits until a consumer asks.

---

## Auth

Two providers ship today:

- **`jwt`** — HS256 tokens, local user table, argon2 passwords. Default.
- **`mock`** — anonymous reads work; send `Authorization: Bearer <any>` to attach the fixed Jane Doe supervisor context (required for mutating routes such as `PUT /api/bundles/{url}/kernel`). For studio dev only.

**JWT logout (`POST /api/auth/logout` with Bearer access token)** is a **global sign-out** for that user: it increments `users.auth_epoch`, revokes every `sessions` row for that user, and embeds `auth_epoch` in new tokens so in-flight refresh cannot mint a valid pair after logout (refresh and verify compare the claim to the row).

**Password or admin credential changes** should go through `Storage::set_user_password_hash`, which updates the hash, bumps `auth_epoch`, and revokes sessions in one transaction so old passwords and old tokens both stop working.

Pending (Track G1 in the active plan): narrow the trait to `AuthVerifier` (drop `login`), widen `AuthUser` to `roles + groups + claims`, add OIDC support. Today's trait shape forecloses real external IdPs — fix is trait-shape compounding debt (D=5).

---

## Provenance export

Three formats via `wos-server export <instance-id> --format <prov-o|xes|ocel>` or `GET /api/instances/:id/provenance/export?format=<...>`. Formats defined in the `wos-export` crate; server is a thin caller.

Legal-sufficiency `wosDisclosure` block on every export (Track A3 in the active plan) — moved to wos-export per the DI review's placement correction.

---

## Testing

```bash
cargo test -p wos-server
```

Test harness:

- `tests/http_smoke.rs` — route reachability + schema validation across the whole API surface.
- `tests/auth_jwt.rs` — JWT auth flows.
- `tests/bundle_validation.rs` — kernel + companion validation round-trip.
- `tests/runtime_lifecycle.rs` — end-to-end case lifecycle.
- `tests/provenance_chain.rs` + `tests/provenance_spec_shape.rs` — provenance output shape + hash-chain integrity.
- `tests/storage_sqlite.rs` — migration + query round-trips.

---

## Development

Runs alongside the studio:

```bash
# terminal 1
cargo run -p wos-server

# terminal 2
cd studio && npm run dev
```

Studio defaults to `http://localhost:4000` as the API base (configurable via studio's env).

---

## Related documents

- [`PARITY.md`](PARITY.md) — per-feature status: shipped, stubbed, or pending. The authoritative "what works today" table.
- Active plan: [`thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`](../../thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md) — remaining work organised by DI seam, with placement corrections per the two DI review passes.
- Open architectural questions: [`thoughts/reviews/2026-04-22-di-review-open-questions.md`](../../thoughts/reviews/2026-04-22-di-review-open-questions.md).
- Reference engine-adapter target: [`thoughts/examples/temporal-reference-implementation.md`](../../thoughts/examples/temporal-reference-implementation.md).
- Spec (what the runtime implements): [`specs/kernel/spec.md`](../../specs/kernel/spec.md) — §1-§16 (post-ADR-0076 merged kernel; runtime companion content absorbed into §11 Runtime Serialization, §12 Evaluation Modes, §13 Formspec Coprocessor, §16 Host Interfaces, plus §4.x / §5.5 / §9.x within-section expansions). [`specs/companions/runtime.md`](../../specs/companions/runtime.md) retained as a redirect-stub home for external citations during the citation sweep.

## Studio wire contract

Response shapes match [`studio/src/services/WosPorts.ts`](../../studio/src/services/WosPorts.ts). When adding an endpoint, keep the shape there first — the TypeScript contract is the single source of truth the studio builds against.

## License

Apache-2.0. See `LICENSE` at the repo root.
