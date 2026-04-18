# Purchase Order Approval

## Problem Statement

An employee (the **requester**) submits a purchase order within an internal procurement
system. A manager (the **approver**) must review and act on that submission. The workflow
must enforce a dollar-threshold decision fork and produce auditable outcome records.

## Actors

- **Requester** — a human employee who initiates the purchase order with a description,
  vendor name, and total dollar amount.
- **Approver** — a human manager who reviews the submitted order and renders a determination.

## Decision Points

1. **Threshold fork** — if the order amount is at or below $50,000, the approver may
   approve directly. If the amount exceeds $50,000, the order must route to director
   review before approval can proceed.
2. **Approval gate** — the approver can reject the order at any point, regardless of
   amount.

## Terminal Outcomes

| State | Meaning |
|---|---|
| `approved` | Order cleared; procurement system is notified to process it. |
| `rejected` | Order denied; requester may revise and resubmit. |
| `cancelled` | Requester withdraws after rejection. |
| `completed` | Procurement system confirms the order was processed. |

## Compliance Constraint

Every determination (approve, reject) must record the acting actor's ID and a timestamp in
the case file. This ensures a full audit trail independent of any external logging system.

## Reference Fixture

See `fixtures/kernel/purchase-order-approval.json` for the canonical WOS kernel document
that this problem maps to. The synthesizer's output should be structurally equivalent to
that fixture when given this problem statement.

## Success Criteria

- Generated document passes `wos-lint::lint_document` with zero error-severity diagnostics.
- The `lifecycle` contains at least four distinct states including at least one `final` state.
- The `$50,000` threshold guard expression appears in a transition guard.
- Actor IDs `requester` and `approver` are declared.
