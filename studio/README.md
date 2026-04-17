# WOS Studio

A browser-based case management studio for WOS (Workflow Orchestration Standard). Provides an inbox, form workspace, case viewer, process dashboard, workflow designer, admin console, audit trail, applicant portal, and report builder — all backed by live kernel state synchronized over WebSockets.

The studio is a **reference implementation** that consumes `wos-spec` JSON Schemas and fixtures. It is not a Formspec renderer — the `@formspec-org/` npm scope is shared with the Formspec ecosystem, but this package has no runtime dependency on any `formspec-*` package. The `contractRef` / `binding` markers on tasks are surfaced as UI metadata; actual Formspec-backed forms are out of scope for this project at this time.

## Architecture

- **Hexagonal ports** (`src/services/WosPorts.ts`): UI components depend on typed port interfaces. Two adapter families implement them:
  - `FixtureAdapter.ts` — in-memory bundle loaded from `../fixtures/**/*.json` at compile time. Default.
  - `HttpWosBackend.ts` — REST + Socket.IO client against this project's own Express server.
- **Server** (`server.ts`): Express + Socket.IO. Loads every kernel under `../fixtures/kernel/*.json` into a URL-keyed registry, builds companion bundles on demand, and routes `PUT /api/bundles/:url/kernel` through an Ajv-backed JSON Schema validator (`src/services/wos-kernel-validator.ts`) before persisting.
- **Client**: React 19 + Tailwind CSS 4 + Vite. Sonner handles toasts.
- **Kernel round-trip** (`src/services/KernelToDesigner.ts`): structure-preserving. Compound `initialState` and parallel `regions` survive designer edits; the round-tripped kernel is verified against `wos-kernel.schema.json` in `KernelToDesigner.test.ts`.

## Setup

```bash
npm install
npm run dev
```

The server starts on `http://localhost:3000` (or `$PORT`).

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `GEMINI_API_KEY` | No | Enables AI chat features (proxied server-side) |
| `API_TOKEN` | No | Bearer token for `/api/` routes. Unset = no auth (dev only) |
| `CORS_ORIGIN` | No | CORS origin header. Defaults to `*` |
| `PORT` | No | Server port. Defaults to `3000` |
| `VITE_WOS_BACKEND` | No | `fixture` (default) or `http`. Selects which adapter set the client uses |

Hardening rules applied to `/api/ai/chat`:

- If `NODE_ENV=production` **or** `CORS_ORIGIN=*` and `API_TOKEN` is unset, the endpoint returns `503 Endpoint requires API_TOKEN to be configured`.
- Request body is capped at 64 kB and must be shaped `{ contents: [...] }`.

## Scripts

| Command | Description |
|---|---|
| `npm run dev` | Start dev server with HMR |
| `npm run build` | Production build to `dist/` |
| `npm run preview` | Preview production build |
| `npm run lint` | TypeScript type-check (`tsc --noEmit`) |
| `npm test` | Run unit and integration tests (Vitest) |
| `npm run test:e2e` | Run E2E tests (Playwright) |
| `npm run types:gen` | Regenerate `src/types/wos/*.ts` from `../schemas/**/*.schema.json` |
| `npm run types:check` | Verify committed types match the schemas (fails if stale) |
| `npm run clean` | Remove `dist/` |

## Testing

- **Unit tests**: Vitest with jsdom. Colocated `*.test.ts(x)` files or `src/__tests__/`.
- **Integration tests** (`tests/integration/`): boot the real Express app via `startServer({ port: 0, attachVite: false })` on an ephemeral port and exercise the HTTP surface with `fetch`. No handler duplication.
- **E2E tests** (`e2e/`): Playwright. Selectors use `data-stage-id` on designer nodes and `data-testid="task-item"` on the inbox. Fixtures in `tests/e2e/fixtures/`, specs in `e2e/`.

## License

BSL-1.1
