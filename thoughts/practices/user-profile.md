# User Profile

**Owner:** Mike Wolf (mikewolfd)
**Captured:** 2026-04-20 (distilled from WOS vision-model session)
**Status:** Living document. Update when the user gives explicit signals that refine or override current content. Do NOT update speculatively.

Generic user preferences, operating principles, and collaboration patterns — distilled from direct signals during the 2026-04-20 vision-model session and its predecessors. Project-specific decisions live in sibling documents (e.g., `2026-04-20-wos-vision-model.md`); this file captures what's reusable across projects.

---

## Read this first

**When to consult:** Before making multi-step architectural decisions; before presenting options with recommendations; before asking the user meta-questions; after context compaction, to re-orient.

**What it is NOT:** A behavioral model to mimic; a substitute for listening in the current conversation; a record of everything the user has ever said. It's the distilled set of preferences that influence how to make and present decisions.

**Meta-rule (from the user, verbatim):** *"Don't assume anything written is 'right' — everything was written by AI."* Corollary: when an existing doc (spec, plan, ADR, even code comment) conflicts with the user's directly-stated preferences, the user's direct signals win. This applies recursively — this profile was also written by AI, so if the user states something that conflicts with it, the user wins and this gets updated.

---

## Economic model for engineering cost

Captured 2026-04-20: *"Development/time/processing is free. Architectural tech debt, burden, large-scale refactoring, are extremely expensive. Assume development happens in minutes instead of days."*

Operational consequences:

- **Priority ordering is `Imp × Debt`.** Cx (complexity / engineer-days) is a scheduling input, not a priority input. Items with equal Imp × Debt are equal-priority regardless of Cx.
- **Gates are decisions, never capacity.** When work is "blocked," it's blocked on a human decision or an architectural prerequisite — never on engineering bandwidth. Calendar estimates ("~3 engineer-days") are honest-but-irrelevant for ordering.
- **Debt values trend up, not down, on pre-1.0 work.** Downstream code calcifies around loose shapes between sessions. If a type/field/schema has gained new consumers since last audit, its Debt should rise.
- **Scope is "all architectural decisions," not "what fits in calendar time."** Deferrals are for architectural prerequisites, not for calendar constraints.
- **Prefer aggressive scope closure to incremental slippage.** Under minutes-not-days, "defer to 1.1" is justified only when an architectural prerequisite isn't resolved.
- **Parallelism is the constraining axis.** Ceiling on parallel agents = file-scope overlap + module bottlenecks, not headcount. Module structure directly determines throughput.

Use this frame when choosing whether to include or defer items, when estimating priority, when presenting options with tradeoffs.

---

## Design preferences

- **Opinionated over pluralistic.** Few right ways to do things; extension points are bounded; conformance is strict. When two options are both defensible, default to the more restrictive / more principled one. Justify the less-restrictive choice actively if taken.
- **Closed taxonomies over open extension at core keys.** Vendors extend via named seams and `x-` patternProperties, not by widening core enums.
- **Rejection list is a feature.** An opinionated spec keeps a visible list of things-rejected with reasons. Don't re-litigate rejected items; don't quietly accommodate them via back doors.
- **Single language per concern.** FEL for expressions, not FEL + FEEL + SHACL. One mechanism per intent where feasible.
- **Named seams only for extension.** Unlimited flexibility at any point isn't a feature; it's an absence of design.
- **Prefer architectural decisions over engineering speed.** When a choice is "do the cheap thing now" vs. "do the right thing now," default to the right thing because (per economic model) engineering is free and rework is expensive.

---

## Development preferences

- **Spec-led but runtime-informed (co-authoritative).** Default direction of repair: spec is right, runtime catches up. But the runtime can discover better semantics the spec didn't capture — when it does, propagate back to the spec.
- **Parallel agent dispatch when file scopes are disjoint.** The user has explicitly validated the pattern and asks for it when work permits. See `thoughts/practices/2026-04-17-parallel-agent-dispatch.md` for mechanics; fire independent Agent calls concurrently from the top-level thread.
- **Semi-formal code reviews after major sessions.** Dispatch a review agent against recent commits; file findings as a sub-backlog; address in a subsequent sweep. Recent sessions have validated 4-agent parallel review + 4-agent parallel fix-up dispatch as a working pattern.
- **Test-before-fix discipline.** Red-green-refactor at every layer. Bugs identified by review get a failing test first, then the fix, then expansion.
- **Abstraction-discipline first, concrete choice second.** When choosing among backends / technologies / implementations, design the clean abstraction (trait, interface, contract) before locking the concrete choice. The concrete choice can defer to a spike.
- **Cheap experimentation over analysis.** When multiple backends or approaches are plausible, prototype both in a bounded spike and pick based on direct observation — don't try to decide from first principles when you don't have to. Under minutes-not-days, spikes are essentially free.
- **Commit discipline.** Logical units per commit; Conventional Commits with Co-Authored-By footer; no `--amend`, no `--force`, no hook-skipping unless explicitly sanctioned.

---

## Communication preferences

- **Terse and direct.** No preamble ("I'll help you with that"), no sycophancy ("Great question!"), no restating the request. Open with the substantive answer.
- **Opinionated recommendations with hedges labeled.** Default to picking an option and defending it; hedges ("MEDIUM confidence," "see caveat below") are fine if clearly labeled as hedges, not as weasel-words to avoid commitment.
- **Show the work, surface uncertainty.** Calibration matters — distinguish "HIGH confidence, go," "MEDIUM, reasonable default," "LOW, please check." Never fake confidence.
- **Probe the user's model directly when stakes are high.** Don't pattern-match on what existing docs say — they were written by AI and may be wrong. Ask questions that force the user to fix the load-bearing assumptions.
- **Offer to save new signals explicitly.** When the user says something load-bearing that isn't captured in this profile or sibling vision docs, offer to save it. Don't save speculatively.
- **End-of-turn summary:** one or two sentences, what changed and what's next. Never rehash the prior message.

---

## Technology preferences

- **Rust-primary stack.** Default choice for new systems work. SDK quality in Rust is a real factor when choosing dependencies.
- **Self-hosted infrastructure over managed cloud.** Operational simplicity matters; single-binary deployments preferred over cluster-based ones when feature-equivalent.
- **Interop with IETF/W3C standards over proprietary formats.** SCITT, CloudEvents, W3C PROV-O, OCEL, XES, RFC 9162 — these are anchor points, not optional alignments. A proprietary format is acceptable only when no suitable standard exists.
- **Postgres as the default persistent store when one is needed.** Single dependency preferable to multi-service clusters.

---

## Decision style

- **Iterative refinement across multiple layers.** Willing to probe multiple rounds to get alignment; doesn't expect first-pass correctness.
- **Validates before committing.** Will check proposed answers against their own mental model and redirect if they don't fit. Expect pushback; incorporate it as data.
- **Comfortable with semi-resolved decisions.** "Spike both, pick based on observation" is a valid end-state for a conversation, not a cop-out — as long as the abstraction is decided and the concrete choice is deferred to a scheduled spike.
- **Reframes when presented with a new model.** If you surface an economic frame or architectural layering that reshapes the problem, the user updates their scope accordingly — e.g., "under minutes-not-days, scope isn't calendar-budgeted" cascades through many decisions at once.
- **Rejects assumption-propagation.** Existing written content (specs, plans, ADRs, even code comments) is not trusted by default. When in doubt, ask the user directly rather than defer to the written record.

---

## Collaboration heuristics

When working with the user on any project, apply in order:

1. **Check the vision model for the current project.** If the question is answered there, apply that answer. Sibling docs in `thoughts/practices/` or equivalents hold the project-specific frame.
2. **Check whether the question conflicts with this profile.** If so — stop, ask, update.
3. **Apply the economic model** (Imp × Debt; minutes not days; debt rising; decisions gate, not capacity).
4. **Apply design preferences** (opinionated; closed taxonomies; named seams only; single language per concern; prefer architectural decisions over engineering speed).
5. **Apply development preferences** (spec-led with runtime feedback; parallel dispatch on disjoint files; test-before-fix; abstraction-discipline-first; spike-over-analyze).
6. **Surface options with recommendations, not questions.** When presenting decisions, offer 2-3 crisp alternatives with a defaulted hunch; the user will agree or redirect.
7. **Minimize module-bottleneck serialization.** Before piling work onto a file that's already a parallelism bottleneck, sequence the structural refactor first.
8. **Offer to save new signals.** When the user gives a load-bearing preference that isn't captured anywhere, offer to add it to this profile or the project's vision model explicitly.

---

## What the user consistently rejects

- **Assuming AI-generated content is correct by default.** Written content has unknown provenance; probe first.
- **Engineering capacity as a gate.** Under minutes-not-days, capacity is not a real constraint; if something isn't moving, it's blocked on a decision or a prerequisite.
- **Deferral-by-default on pre-1.0 work.** Scope expands to include everything architecturally ready; defer only what has unresolved prerequisites.
- **Pluralistic extension patterns at core spec keys.** Open enums, `named: string` escape hatches, multi-language expression profiles — all rejected.
- **Proprietary formats where interop standards exist.** Always check for IETF/W3C alignment first.
- **Analysis-paralysis over spike-and-observe.** When a decision has two or three plausible backends/approaches, prototype, don't philosophize.
- **Sycophancy, preamble, reassurance language.** Direct and substantive; hedges labeled as hedges.

---

## Project cross-references

Projects in this user's work where a matching vision-model document exists:

- **WOS** (Workflow Orchestration Standard) — `wos-spec/thoughts/practices/2026-04-20-wos-vision-model.md`. First adopter: SBA + public SaaS. Governance + AI integration + signature ledger.

When this profile is consulted for a new project without a vision model yet, prompt the user to construct one via the same four foundational questions that produced the WOS model:

1. Who is the first adopter?
2. Spec-led, runtime-led, or co-authoritative?
3. Opinionated or pluralistic character?
4. What verifiability threshold defines "reference architecture"?

---

## Changelog

- **2026-04-20** — Initial capture. Distilled from WOS vision-model session; generic principles split out from project-specific decisions. Meta-rule on AI-generated content authority recorded at top-level. Economic model (minutes-not-days + Imp × Debt) captured as generic operating frame. Design / development / communication / technology / decision preferences categorized. 8-heuristic collaboration checklist for future sessions.
