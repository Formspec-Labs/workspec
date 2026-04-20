# Archived Kernel Drafts

Historical WOS Core drafts (v0.x through v7) and tier-spec drafts, preserved as
loop artifacts per the [2026-04-16 architecture review handoff](../reviews/2026-04-16-architecture-review-handoff.md)
("iteration made visible"). All content has been superseded by the canonical
`specs/kernel/spec.md` (v1.0.0-draft.1, 2026-04-09) and the tier specs under
`specs/ai/`, `specs/governance/`, `specs/companions/`, and `specs/profiles/`.

**Moved here:** 2026-04-20 (TODO §4.1 DRAFTS triage). The files are inert
markdown — not referenced from any schema, crate, or canonical spec. No content
was lost; git history preserves provenance.

## Classification

### Superseded — kernel version iterations

These are snapshot proposals that were merged, rejected, or replaced during the
v2 → v7 → v1.0 reframe. Kept as historical record of the design loop.

| File | Supersession |
|------|--------------|
| `wos-core-spec.md` | v0.1 baseline → replaced by v2 |
| `wos-core-v2.md` | 8-layer architecture + 21 profiles → replaced by v3 |
| `wos-core-v3.md` | JSON-LD + SHACL direction → replaced by v4 |
| `wos-core-v4.md` | Constraint-enhanced layered kernel → replaced by v5 |
| `wos-core-v5.md` | Formspec-as-interface-contract → replaced by v6 |
| `wos-core-v6.md` | Community-review draft → replaced by v7 reframe |
| `wos-core-agent-amendments.md` | Agent amendments to v0.1 → folded into v2+ |

### Aspirational/planning — tier-spec ancestors

These predate the current tier-spec layout. Content was redistributed across
canonical specs; drafts retained for cross-reference.

| File | Current home |
|------|--------------|
| `wos-core-v7-kernel.md` | Shaped `specs/kernel/spec.md` |
| `wos-core-v7-proposal.md` | Shaped kernel + profile split |
| `wos-agent-tier-spec.md` | Content in `specs/ai/ai-integration.md`, `specs/ai/agent-config.md`, `specs/ai/drift-monitor.md` |
| `wos-agent-tier-v7.md` | Near-duplicate of the agent-tier draft |
| `wcos-lifecycle-spec.md` | Harel statechart semantics folded into kernel §4 lifecycle topology |

### Schema snapshot

| File | Supersession |
|------|--------------|
| `wos-core-v7.schema.json` | Replaced by `schemas/kernel/wos-kernel.schema.json` + the 20 other production schemas |

## Not for editing

Files in this directory are frozen references. Do not edit in place — update the
canonical spec under `specs/` instead.
