# WOS Positioning

What makes WOS distinct from the domain literatures it draws from. Not a roadmap — a statement of the load-bearing claims behind the specification.

---

## The One-Line Thesis

> Other workflow standards were designed for humans with canvases. WOS is designed for LLMs with schemas. Agents are a reference extension of the same design.

Incumbents cannot retrofit this. BPMN's canvas model assumes human modelers. SCXML's XML serialization predates schema-first tooling. Temporal is code-first, not spec-first. The defensible lane is schema-first orchestration whose reference authoring surface is an LLM.

---

## Two Claims

WOS is AI-native in two distinct, separable senses. Buyers can adopt Claim A without Claim B.

### Claim A — LLM-authored workflows (the generation story)

Workflows are structured data. An LLM can generate them directly against schemas; static lint gives immediate structural feedback; conformance fixtures give immediate behavioral feedback; the author sees impact before deployment. The spec → schema → lint → conformance loop *is* the LLM's authoring loop, compressed to seconds.

This addresses every organization that writes workflows, not only organizations that want agents to execute them. The fine-grained schemas (18), three-tier verification (T1/T2/T3), and schema-enforced prose are load-bearing for this claim: coarse schemas produce weak lint signal, weak lint signal produces low-confidence authoring, low-confidence authoring defeats the point.

### Claim B — Agents as first-class runtime actors (the execution story)

When the workflow runs, agents are declarable participants alongside humans and services, with autonomy levels, confidence gates, deontic constraints, and drift monitoring. Optional, but native to the design via the kernel's `actorExtension` seam.

This addresses the subset of organizations that want agent-executed workflows. The claim is disclosed in the AI Integration spec and its conformance fixtures.

### Why the separation matters

Previously these were conflated under an "AI-native" tagline and marketed primarily as Claim B. Claim A is the larger market and the sharper differentiator: every org writing workflows benefits from LLM-authorability; only a subset want agent runtimes. Treating the two as separable makes the offering legible to both audiences.

---

## The Genuine Invention

Every capability below has component prior art in domain literatures. The consistent novelty across all of them is **"declarative encoding as schema-enforceable workflow primitives"** — not the concepts themselves.

1. **Deontic operators as schema-enforced primitives.** Prior art: LegalRuleML (operators), von Wright deontic logic (1951). Novel: processor-enforced null behavior with impact-level defaults, wired to autonomy caps.

2. **Structured oversight modes as declarative schema.** `independentFirst`, `considerOpposite`. Prior art: cognitive debiasing literature, QA frameworks. Novel: declarative encoding in workflow spec.

3. **Due process as schema-enforceable workflow primitives.** Prior art: administrative law (Goldberg, Mathews), FedRAMP/OMB policy encoding. Novel: spec-level enforcement with conformance fixtures.

4. **4-tier provenance layering (Facts / Reasoning / Decision / Narrative).** Prior art: PROV-O agent/entity/activity. Novel: distinct epistemic-tier layering vs. PROV's actor-role model.

5. **Authority-ranked reasoning with confidence composition.** Prior art: LegalRuleML authority hierarchy, evidential reasoning. Novel: composition of authority rank × calibrated confidence as a reasoning-trace primitive.

6. **Impact-level-dependent behavior as schema-enforced processor obligations.** Prior art: OMB M-24-10, EU AI Act risk tiers, FedRAMP Low/Mod/High. Novel: declarative workflow-spec encoding.

7. **Normative civil-rights monitoring as workflow-spec primitive.** Prior art: EEOC 4/5ths rule (1978), AIF360, Fairlearn, disparate-impact statistical methods. Novel: declarative encoding with automated remediation triggers scoped to both human AND AI decisions.

8. **Normative binding of drift detection to autonomy demotion.** Prior art: ML-ops drift detection (Evidently, Arize, WhyLabs). Novel: normative binding in workflow spec — drift detection alone is not novel.

---

## Related documents

- [IDEA_SCRATCH.md](IDEA_SCRATCH.md) — design direction, axes, rejects, confirmed decisions.
- [TODO.md](TODO.md) — active backlog.
- [README.md](README.md) — feature matrix and intellectual ancestry.
