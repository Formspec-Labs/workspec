# Parallel Sub-Agent Dispatch Discipline

**Date:** 2026-04-17
**Status:** Active

## The problem

Sub-agents dispatched in parallel commit their own work to the same branch. Git has no native "reject conflicting write" mechanism for this pattern — if two agents modify the same file, both commits land, and the later silently supersedes the earlier.

Two concrete incidents in this session (see Finding 6 of the 2026-04-17 conceptual review): two cases of duplicate rewrites of the same plan file, with the later commit overwriting the earlier without any alarm raised by the dispatch process.

- Commit `c3f20a0` in wos-spec overwrote `c11620e` — both were rewrites of `thoughts/plans/2026-04-16-wos-synth-crate.md` by two parallel sonnet dispatches targeting the same file. The second dispatch's task appeared to have errored initially; it returned its result asynchronously later and committed on top of the first, silently replacing 304 lines with a different 360-line version.

- Commit `45fe0ca` in wos-spec overwrote `495746a` — same pattern, both rewrites of `thoughts/plans/2026-04-17-wos-authoring-crate.md`.

Both collisions happened to involve agents authoring similar content, so the loss was mitigated (later version subsumed earlier). For load-bearing code changes, this would be silent data loss.

## Root causes

1. **Retry ambiguity.** When a dispatch appears to error, the orchestrator may retry, unaware the original is still in flight. Both eventually complete, both commit.
2. **File-surface overlap.** Agent instructions don't always make it clear which files are exclusive to which agent; agents can grow scope during execution.
3. **No write-lock.** The submodule is a single branch with no commit-level lock protecting specific files.

## Dispatch rules (apply when sending 2+ agents in parallel in the same repo)

### Rule 1 — One writer per file

Each file modified or created in a parallel-dispatch batch MUST be assigned to exactly one agent. State this explicitly in each agent's prompt:

> **Your files (NO overlap with other agents):**
> 1. `path/to/file1.md`
> 2. `path/to/file2.md`
>
> Do NOT touch any other file. Do NOT create new files except those listed.

### Rule 2 — Do not retry in parallel

If an agent appears to error or time out:

- **Preferred:** inspect the commit log to see if it wrote anything (`git log --all --since='10 minutes ago'`) before deciding it failed.
- **If certain it failed:** dispatch a single replacement agent with EXACT instructions to check first whether a commit already landed with the expected shape (e.g., `git show <sha> -- <file>`), and abort if so.
- **NEVER** dispatch a parallel retry of the same task without confirming the first is truly dead.

### Rule 3 — Prefer worktrees for high-risk parallelism

When dispatching parallel agents that WILL modify shared state (e.g., multiple agents extending the same module, or agents in a hot codebase under other activity), use `isolation: "worktree"` on the Agent tool. Each agent runs in an isolated git worktree and returns a branch for review; the orchestrator merges or cherry-picks manually. Cost: slower (per-worktree build cache); benefit: no silent overwrites.

Reserved for: simultaneously modifying crate source (vs. plan docs), in-flight active codebases with >3 developers, any case where the cost of overwriting is >1 hour of work to recover.

### Rule 4 — Sequence for hot files

Some files are inherently shared (TODO.md, README.md, workspace Cargo.toml). When multiple agents need to edit these:

- **Preferred:** one agent owns the hot file and aggregates all changes. Other agents report what they WOULD change; owner applies.
- **Alternative:** serialize — run the hot-file agent last, after all parallel work completes, so it can see the final state.

### Rule 5 — Post-dispatch reconciliation check

After any parallel dispatch batch completes, the orchestrator runs:

```bash
git log --since='1 hour ago' --name-only --pretty=format:'%h %s'
```

and asserts that no filename appears in more than one commit. If any does, the orchestrator inspects the contents to ensure the later commit INTENDED to build on the earlier (not overwrite it).

## What NOT to do

- Do NOT dispatch >6 sonnet agents in parallel to the same submodule without worktrees. Even with non-overlapping file surfaces, git's commit rate limiter can interact poorly.
- Do NOT retry an apparently-failed agent without confirming its commit status.
- Do NOT assume commit ordering reflects dispatch ordering — a late-completing agent can land a commit hours after dispatch.

## Exception: single-writer + pure-append patterns

A file like a ledger (`benchmarks/runs/*.json`) is safe for parallel writes if each agent owns a unique filename (e.g., `<date>-<model>-<run-id>.json`). The file-surface-overlap rule doesn't apply when filenames are distinct by construction.

## Checklist (copy into orchestrator dispatch messages)

Before dispatching N parallel agents:

- [ ] Every file to be modified is assigned to exactly one agent.
- [ ] Each agent's prompt names its files + an explicit "don't touch other files" warning.
- [ ] Hot files (TODO.md, Cargo.toml workspace list, shared READMEs) have a designated owner.
- [ ] If any file surface looks contested, use `isolation: "worktree"` OR serialize.
- [ ] Post-dispatch: run the reconciliation check (Rule 5).

After dispatch completes:

- [ ] Reconciliation check shows no filename appearing in multiple commits (or if it does, the later commit intentionally builds on the earlier).
- [ ] All assigned agents reported success OR their failures were investigated (Rule 2).
