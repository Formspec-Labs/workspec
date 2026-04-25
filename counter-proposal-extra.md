X31. Timeout Actions Must Create Explicit Edges
Source: IMP-1 implementation — timeout "action": "expired" referenced a node ID but didn't create a graph edge, making the target node unreachable per §5 validation.
Decision: Timeout actions that route to nodes MUST create explicit edges.
When a timeout's action or escalation entry's action references a node ID, a corresponding edge MUST exist in the edges array with source as the node declaring the timeout and target as the referenced node. The edge SHOULD be annotated to indicate it's a timeout-triggered path (e.g., "trigger": "timeout" or a label like "timeout: 48h").
This means:

Timeout actions are routing decisions, and all routing must be visible in the edge graph
Visual builders can render timeout paths as distinct edge styles
Reachability validation (§5.1.3) works without special-casing timeouts
The event log records which edge was followed, whether it was a normal transition or a timeout

Spec changes needed: Update §2.5 (Timeout Schema) to state that timeout actions referencing node IDs MUST have corresponding edges. Update §5 validation rules to check this. Add "trigger" as an OPTIONAL property on edges to distinguish timeout paths from normal transitions.

X32. JSON Schema Version — Draft 2020-12 with Discriminator
Source: IMP-1 implementation — oneOf discriminator pattern for node types produces poor error messages in ajv with draft-07.
Decision: The spec SHOULD recommend JSON Schema draft 2020-12 with discriminator support.
The node type config validation uses oneOf with type: { const: "ai" } etc. In draft-07, validation failures report "should match exactly one schema in oneOf" without indicating which type was attempted. Draft 2020-12's discriminator keyword (or the OpenAPI-style discriminator extension) produces actionable error messages.
Impact: This is a RECOMMENDED, not a MUST — implementations can use any JSON Schema version that validates the spec. But the canonical flowspec.schema.json published with the spec should use12 features where they improve tooling.

X33. Conditional Node Routing vs. Edge-Based Routing — Collapse or Clarify?
Source: IMP-1 implementation — the PFML workflow used conditional node conditions[].target for fraud routing AND conditional edges for approval/denial routing, creating two routing mechanisms.
The pattern: Two ways to express conditional routing exist in the spec:

Conditional node: A dedicated node with conditions[] array, each with expression + target. The node itself is the routing decision point.
Conditional edge: An edge with a condition FEL expression. The edge is only followed when the condition is true.

Both are valid. The LLM generated them correctly. But a human reading the spec might be confused about when to use which.
Observation: The LLM figured this out without difficulty — it used conditional nodes for multi-way routing (fraud: 3 paths) and conditional edges for binary post-action routing (approved → send-approval). This is actually the right heuristic: conditional noN-way branching, conditional edges for simple binary splits.
Direction: This is a documentation/guidance issue, not a structural issue. The spec should include a RECOMMENDED guidance section:

Use conditional nodes when routing has 3+ paths or when the routing decision is a meaningful audit point (eligibility determination, risk classification)
Use conditional edges when routing is a simple binary split (e.g., an edge from a human node that only fires when the action was "approve")
Conditional nodes MUST be used for eligibility determinations (FP-04) — conditional edges are for convenience routing, not determinations

No spec structural change needed. Add guidance to §3.6 and/or a non-normative appendix.

X34. Escalation as a First-Class Pattern
Source: IMP-1 implementation — standard → senior → manager escalation required fragile $exec.nodeStates.standard-review.status checks in conditionals. Node ID changes break expressions.
The problem: Multi-level escalation (standard adjudicator → senior ad manager) is a universal pattern in government workflows (confirmed by all 3 interviews). Currently modeled as:

Multiple human nodes (one per level)
A conditional node checking which review node completed
Timeout-based escalation on each human node

This works but has three issues:

Fragile $exec references — conditional expressions reference specific node IDs. Renaming a node breaks downstream routing.
Repetitive config — each escalation level has nearly identical human node config (same form sections, same actions) with minor differences (different assignee pool, maybe different visible context).
Not self-documenting — reading the graph, you can't tell that standard/senior/manager are an escalation chain rather than three independent review paths.

Possible approaches:
A. Escalation as a human node config property:
json{
  "id": "review",
  "type": "human",
  "label": "Application Review",
  "config": {
    "formSections": ["..."],
    "actions": ["approve", "deny", "request-info"],
    "escalation     "levels": [
        { "id": "standard", "assignee": { "role": "adjudicator", "pool": "standard" }, "timeout": { "duration": "PT48H" } },
        { "id": "senior", "assignee": { "role": "senior-adjudicator", "pool": "senior" }, "timeout": { "duration": "PT24H" } },
        { "id": "manager", "assignee": { "role": "manager" } }
      ]
    }
  }
}
This collapses three nodes into one. The engine manages level progression. The execution state tracks which level is active. No fragile $exec references.
B. Escalation edge type:
Keep separate human nodes but add "type": "escalation" on the edges connecting them, plus a "fromLevel" / "toLevel" annotation. The graph structure is preserved for visualization, but the escalation relationship is explicit.
C. Escalation as a wrapper node type:
A new escalation node type that wraps multiple human steps with automatic level progression. Similar to how parallel wraps multiple branches.
Tentative position: Approach A is cleanest — it collapses a repetitive multi-node ptern into a single node with levels. The execution state would track $exec.nodeStates.review.currentLevel, $exec.nodeStates.review.levelHistory[]. This also connects to X21 (case-level assignment) since escalation often means reassigning the case.
Relates to: X21 (Case-level assignment), human node §3.4, timeout §2.5
