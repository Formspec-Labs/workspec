# WOS Studio

A browser-based case management studio for WOS (Workflow Orchestration System). Provides an inbox, form workspace, case viewer, process dashboard, workflow designer, admin console, audit trail, applicant portal, and report builder — all backed by live kernel state synchronized over WebSockets.

## Architecture

- **Hexagonal ports**: UI components depend on typed port interfaces (`src/services/WosPorts.ts`). A stub adapter (`src/services/WosBackend.ts`) provides fixture data for development.
- **Server**: Express + Socket.IO (`server.ts`). Loads WOS kernel fixtures, exposes REST endpoints, and broadcasts kernel changes over WebSockets.
- **Client**: React 19 + Tailwind CSS 4 + Vite. `FormEngine` drives reactive form state. Sonner handles toasts.

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
| `API_TOKEN` | No | Bearer token for `/api/` routes. Unset = no auth |
| `CORS_ORIGIN` | No | CORS origin header. Defaults to `*` |
| `PORT` | No | Server port. Defaults to `3000` |

## Scripts

| Command | Description |
|---|---|
| `npm run dev` | Start dev server with HMR |
| `npm run build` | Production build to `dist/` |
| `npm run preview` | Preview production build |
| `npm run lint` | TypeScript type-check (`tsc --noEmit`) |
| `npm test` | Run unit tests (Vitest) |
| `npm run test:e2e` | Run E2E tests (Playwright) |
| `npm run clean` | Remove `dist/` |

## Testing

- **Unit tests**: Vitest with jsdom. Colocated `*.test.ts(x)` files or `src/__tests__/`.
- **E2E tests**: Playwright. Fixtures in `tests/e2e/fixtures/`, specs in `e2e/`.

## License

BSL-1.1
