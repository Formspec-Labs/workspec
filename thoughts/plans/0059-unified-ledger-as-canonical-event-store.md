# Unified ledger as canonical case store — stack narrative (ADR-0059)

**Status:** Locked narrative — 2026-04-22  
**Authority:** Cross-repo program intent (Formspec + WOS + Trellis).  
**Normative technical detail:** Parent ADR [`../../thoughts/adr/0059-unified-ledger-as-canonical-event-store.md`](../../thoughts/adr/0059-unified-ledger-as-canonical-event-store.md) (full requirements, taxonomy tables, encryption model, consequences).

---

## One paragraph

The stack **will** converge on **one append-only evidentiary spine per case**: intake (respondent ledger events), the **coprocessor bridge** (`case.created` and related), WOS governance and provenance, and lifecycle events (`ledger.*`)—with **hash chain / Merkle / signed checkpoints**, **encrypt-then-hash** for payloads where policy requires it, and **disposable projections** (Postgres dashboards, materialized views) over that spine. **Execution** (Restate/Temporal, timers, retries) stays **out of band** as orchestration durability; **evidence** lives on the ledger. That end state is **Phase 3 — portable case files** in [`../../trellis/thoughts/product-vision.md`](../../trellis/thoughts/product-vision.md).

---

## What was *not* rejected

- **ADR-0059’s architecture** (single spine, projections, encrypt-then-hash story, regulatory framing) remains the **north star**.
- What was **rejected as sequencing** was **“ship a unified immutable database for everything before a byte-exact Phase 1 export + verifier.”** That would have blocked ratification (G-3–G-5), frozen the wrong layer first, and forced format churn under real records.

---

## What ships first (locked sequencing)

| Phase | Delivers | Relation to ADR-0059 |
|-------|----------|------------------------|
| **Phase 1** | Attested export bundles, offline verify, `custodyHook` ingestion, maximalist envelope + restrictive runtime lint ([`../../trellis/thoughts/specs/2026-04-20-trellis-phase-1-mvp-principles-and-format-adrs.md`](../../trellis/thoughts/specs/2026-04-20-trellis-phase-1-mvp-principles-and-format-adrs.md)) | Same **envelope bytes** Phase 3 will compose; reserves §22/§24 hooks without populating them early. |
| **Phase 2** | Runtime-time attestation (every write path uses Trellis library semantics) | Moves append discipline **closer** to 0059’s “every actor writes one kind of event” without merging stores yet. |
| **Phase 3** | One **case ledger** per case + agency log | **0059 implementation target**: unified taxonomy on one chain (storage may be immudb/Trillian/append-only Postgres + Merkle batches—spike decides). |

Invariant from product vision: **Phase 1 export event shape is the Phase 3 case-ledger event shape** (strict superset later, no wire break).

---

## WOS responsibilities toward 0059

- **Today:** Emit **Facts-tier** provenance (e.g. `SignatureAffirmation`) and other records; **`wos-export`** maps to Trellis **`custodyHook`** append shape ([`../adr/0061-custody-hook-trellis-wire-format.md`](../adr/0061-custody-hook-trellis-wire-format.md)). Provenance remains **`ProvenanceRecord`** in runtime storage; **semantic alignment** with future unified ledger event types is tracked as spec work (`case.created`, governance event type registry).
- **Toward Phase 3:** Each provenance / governance emission **MUST** be mappable to a **stable event type + payload** in the unified taxonomy (ADR-0059 Part 4); no ad hoc Postgres-only shapes that cannot serialize to the Trellis envelope.

---

## Trellis responsibilities toward 0059

- **Today:** Canonical **append / checkpoint / export / verify** for custody records; extension members (`061`, `062`, manifest `extensions`, …) under Phase-1 lint.
- **Toward Phase 3:** Case-scoped composition and agency-log heads use **reserved envelope fields** (§22 / §24) when Federation Profile scoping lifts Phase-1 `MUST NOT populate` rules—**no second envelope format**.

---

## Formspec responsibilities toward 0059

- **Today:** Respondent Ledger add-on (hash-chained events, checkpoints deferred to Trellis); canonical **Response**.
- **Toward Phase 3:** Same ledger **continues** as intake segment inside the unified per-case log; **no duplicate integrity model** for “form side” vs “workflow side.”

---

## Revisit triggers

- Phase 3 program kickoff (portable case file epic).
- Substrate spike conclusion (immutable store + proof UX).
- G-5 stranger commissioning complete (Phase-1-shape issuance policy).

---

## Related

- ADR-0054 (privacy-preserving client/server chain): [`../../thoughts/adr/0054-privacy-preserving-client-server-ledger-chain.md`](../../thoughts/adr/0054-privacy-preserving-client-server-ledger-chain.md)
- Trellis `README.md` (repo role + upstream links)
- WOS `TODO.md` / `T4-TODO.md` (active cross-repo gates)
