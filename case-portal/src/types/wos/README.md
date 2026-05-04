# WOS Types

These types are generated from JSON Schema files via `json-schema-to-typescript`.
They correspond to the WOS (Workflow Orchestration Standard) specification.

These types will eventually be extracted to a shared package (`@wos/types` or similar)
so they can be consumed by both the Studio frontend, the server, and external tooling
without duplicating type definitions.

Until that extraction happens:
- Do not hand-edit generated type files (they have `DO NOT MODIFY IT BY HAND` headers).
- To regenerate, modify the source JSON Schema and run `json-schema-to-typescript`.
- The runtime views in `WosBackend.ts` and `WosPorts.ts` reference these types but
  define their own view interfaces for API responses.
