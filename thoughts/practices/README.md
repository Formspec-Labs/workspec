# WOS Practices

Team discipline documents. Narrower scope than ADRs — these capture HOW we work, not architectural decisions.

## Stack-wide operating docs (at the parent `formspec/.claude/` level)

- [`../../../.claude/vision-model.md`](../../../.claude/vision-model.md) — stack-wide vision model (Formspec + WOS + Trellis). WOS-specific architectural commitments + v1.0 scope + uncertainties live under `## WOS` in that file.
- [`../../../.claude/user_profile.md`](../../../.claude/user_profile.md) — generic user operating preferences.

## WOS-specific practices

- [`2026-04-17-parallel-agent-dispatch.md`](2026-04-17-parallel-agent-dispatch.md) — rules for dispatching sub-agents in parallel without silent file overwrites.
