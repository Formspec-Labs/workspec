# WOS Licensing

WOS uses an open-core licensing model. The specification, schemas, and runtime libraries are permissively licensed to maximize adoption. The authoring and tooling packages are source-available under a commercial-use restriction that converts to Apache-2.0 after four years.

## License history

This project follows the licensing model of the parent Formspec project:

1. **Apache-2.0 / BSL 1.1** — current open-core model

## Apache-2.0 — Specification, Schemas, and Runtime

The following are licensed under the [Apache License 2.0](../LICENSE):

**Specification and schemas:**
- `specs/` — All WOS specification documents
- `schemas/` — All WOS JSON Schema files

**Rust crates (Release Streams):**
- `wos-core` — Domain model and evaluation logic
- `wos-lint` — Static analysis engine
- `wos-conformance` — Scenario runner and test fixtures
- `wos-runtime` — Orchestration and persistence
- `wos-formspec-binding` — Formspec coprocessor implementation
- `wos-export` — Data export utilities
- `wos-synth-*` — LLM-authoring research and provider crates

You may use, modify, and distribute these packages freely under the terms of Apache-2.0, including in proprietary and commercial applications. See the root [LICENSE](../LICENSE) for full terms.

## BSL 1.1 — Authoring and Tooling

The following are licensed under the [Business Source License 1.1](../LICENSE-BSL):

**Rust crates:**
- `wos-authoring` — Project model and authoring-specific logic
- `wos-mcp` — MCP server for AI-driven WOS authoring

**What the BSL allows:**
- Internal use within your organization
- Development, testing, and evaluation
- Non-commercial and academic use
- Building internal tools, even commercially, as long as they are not offered to third parties as a workflow authoring product
- Any use that is not a competing "Workflow Authoring Product" (hosted *or* packaged)

**What requires a commercial license:**
- Offering a product — hosted, managed, on-premises, or packaged — that allows third parties to create, edit, or manage WOS workflow definitions using the BSL-licensed components

**Change date:** April 7, 2030 — on this date, all BSL-licensed code converts automatically to Apache-2.0.

For commercial licensing inquiries, contact Michael.Deeb@tealwolf.consulting.

## Workflow definitions are your data

Your JSON workflow definitions, governance policies, and all other WOS documents you create are **your data**. They are not derivative works of WOS. They are not covered by any WOS license. You own them completely regardless of which tools you used to create them.

## Trademarks

"WOS," "Workflow Orchestration Standard," and "Formspec" are trademarks of Michel Deeb / TealWolf Consulting LLC. The Apache-2.0 and BSL 1.1 licenses grant rights to the *code*, not to the name or brand.

## Contributing

By submitting a contribution to this repository, you agree to license your contribution under the same license that applies to the file(s) you are modifying, and you acknowledge that the maintainers may offer the project (including your contributions) under commercial license terms.
