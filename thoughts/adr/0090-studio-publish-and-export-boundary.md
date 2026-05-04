# ADR-0090: Studio publish + export boundary — signing, ApprovalPackage, ExportBundle, ExportSink

**Status:** Proposed 2026-05-04
**Date:** 2026-05-04
**Deciders:** WOS Working Group (Studio sub-team)
**Author:** Studio authoring layer (Stage 7)
**Supersedes:** None
**Amends:** [`studio/specs/review-and-approval.md`](../../studio/specs/review-and-approval.md) — pins the multi-target ApprovalPackage assembly + signing contract; [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md) — generalizes Phase-9 ExportBundle to multi-target, signed.

**Related:**
- ADR 0086 (parent — reference architecture)
- ADR 0087 (sibling — persistence; defines Ed25519 per-actor key custody)
- ADR 0089 (sibling — projection-target model; defines what gets bundled)
- Parent [`specs/kernel/custody-hook-encoding.md`](../../specs/kernel/custody-hook-encoding.md) (PLN-0385 — Trellis anchoring)

---

## 1. Context

Studio's publication boundary is the single point at which
authoring artifacts cross from internal workspace state into
externally consumable, audit-bearing form. The boundary is
load-bearing for:

- Trust handoff to `crates/wos-server` (which consumes
  `$wosWorkflow.json`).
- Trellis anchoring of the case-relevant artifacts.
- Auditor / regulator inspection (the `ApprovalPackage` is the
  unit auditors examine).
- Reproducibility (the same workspace state at the same compile
  date MUST produce a byte-identical bundle).

Today, [`studio/specs/review-and-approval.md`](../../studio/specs/review-and-approval.md)
pins the ApprovalPackage shape (`wosVersionPin`,
ComplianceAttestations, IdentitySigningKeyRefs,
custodyAnchorReceipt). [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md)
Phase 9 emits a workflow-only ExportBundle. This ADR generalizes:

1. **Multi-target ExportBundle.** Composes outputs from multiple
   `ProjectionTarget`s (per ADR 0089) into a single signed
   bundle.
2. **Signing model.** Bundle-level signature distinct from per-
   actor authoring-row signatures (ADR 0087).
3. **`ExportSink` port.** Pluggable destinations (filesystem,
   Trellis network, future federation).
4. **Reproducibility contract.** Replay-test guarantee
   (cross-references ADR 0087).

## 2. Decision

### 2.1 Multi-target ExportBundle composition

A Stage 8 ExportBundle Builder composes:

- All projected artifacts (one per `ProjectionTarget` exercised).
- The ApprovalPackage covering them (one per published-as-a-set).
- A **manifest** enumerating: artifact paths, content hashes,
  source-version pins, prompt/model/parser/projection version
  pins, custody anchor receipts.
- The **canonical workspace export** (sources + PolicyObjects +
  mappings + scenarios + provenance log) per existing Phase-9
  contract.

Bundle layout:

```
bundle/
  manifest.json                       -- multi-target manifest
  manifest.signature                  -- bundle-level signature
  approvals/
    approval-package.json             -- per ApprovalPackage spec
  artifacts/
    wos-workflow.json                 -- WOS workflow projection
    formspec-form.json                -- Formspec form projection
    decision-table.json               -- decision projection (when present)
    integration-bindings.json         -- binding projection (when present)
  workspace-export/
    sources/                          -- per Phase-9
    policy-objects/
    mappings/
    scenarios/
    provenance/
    custody-receipts/
```

### 2.2 Bundle-level signing

The bundle carries a single Ed25519 signature over the canonical-
JSON serialization of `manifest.json`. The signing key is the
**publication signing key** — a workspace-level key distinct from
per-actor authoring-row signing keys (ADR 0087).

Publication signing keys:
- Are issued through the `KeyManager` port.
- Stage 8 ships file-backed dev-mode; Stage 9+ wires HSM/KMS.
- Are referenced from the `ApprovalPackage.IdentitySigningKeyRefs`
  field (existing).
- Are rotated per
  [`studio/specs/review-and-approval.md`](../../studio/specs/review-and-approval.md)
  §"Key-rotation handling" (existing).

### 2.3 `ExportSink` port

The bundle is delivered to an `ExportSink`:

```text
trait ExportSink {
    fn write(&self, bundle: &SignedExportBundle) -> Result<ExportReceipt, ExportError>;
}
```

| Sink | Stage |
|---|---|
| Filesystem (writes to a directory; returns the path) | Stage 8 |
| Trellis network (PUTs to a Trellis ingest endpoint with the four-field append shape per parent PLN-0385) | Stage 9+ |
| Federation sink (cross-org publication) | Stage 10+ (maximalist) |

`ExportSink` is the same trait as `ProjectionTarget` viewed from
the destination side — Stage 7 names them as aliases for
expressivity (`ExportSink` reads better when the consumer is
external storage; `ProjectionTarget` reads better when the
consumer is an artifact emitter). The trait shape is identical.

### 2.4 Reproducibility contract

For a given workspace state and compile date:

```text
project_all() + assemble_bundle() + sign() + write_to(filesystem_sink)
```

MUST produce a byte-identical bundle. The Stage 8 replay test (per
ADR 0087 §2.5) asserts this via teardown + replay + bundle hash
comparison.

Sources of nondeterminism are eliminated:
- Compile date is captured in the manifest.
- Map iteration is over `IndexMap` / `BTreeMap` per existing
  compiler discipline.
- Recorded AI outputs are reused verbatim (per ADR 0088 §2.5).
- Bundle layout is content-ordered (lexicographic by path within
  each subdirectory).

### 2.5 Trellis anchoring composition

The bundle includes per-artifact custody receipts under
`workspace-export/custody-receipts/`. Each receipt is the
parent `custodyHook` four-field append output (PLN-0385) for the
corresponding artifact's anchor event.

Trellis anchoring happens **before** bundle signing — anchor
receipts are part of what the bundle signature covers. This
composition mirrors the existing Phase-9 contract.

## 3. Rejected Alternatives

- **No bundle-level signature (only per-artifact signatures).**
  Rejected; an unsigned manifest admits artifact-substitution
  attacks at the bundle boundary.
- **Single signing key for both authoring rows and bundle.**
  Rejected; per-actor authoring keys are personal, bundle keys
  are workspace-level. Conflating them muddies the audit story.
- **Bundle as a tarball.** Rejected (for v1); a content-addressed
  directory is easier to inspect, diff, and audit. Tarball
  packaging is a Stage 9+ optional convenience.
- **`ExportSink` and `ProjectionTarget` as distinct traits with
  distinct shapes.** Rejected; they are the same shape viewed from
  opposite sides. Aliasing them is cheaper than maintaining two
  trait families.
- **Per-projection-target signing keys.** Rejected; one bundle
  signature with one workspace-level key suffices.

## 4. Consequences

### Positive

- The bundle boundary is the audit unit; one signature, one
  manifest, one set of receipts.
- Trellis anchoring is structurally inside the bundle.
- Multi-target publication doesn't fork the boundary.
- Reproducibility is testable.

### Negative

- The publication signing key is a high-value secret; production
  hardening (HSM/KMS) is a Stage 9+ blocker for non-dev deployment.
- Open: `ExportSink` impls beyond filesystem are deferred to
  Stage 9+.

### Neutral

- The bundle layout extends the existing Phase-9 layout; not a
  reshape.

## 5. Conformance

- `SA-MUST-arch-064`, `SA-MUST-arch-073` in
  [`studio/specs/reference-architecture.md`](../../studio/specs/reference-architecture.md).
- Stage 8 deliverables 12, 13, 14 in
  [`studio/specs/stage-8-vertical-slice.md`](../../studio/specs/stage-8-vertical-slice.md).
- Replay test (cross-ADR 0087 §2.5).
- Existing Phase-9 conformance (per
  [`studio/specs/compiler-contract.md`](../../studio/specs/compiler-contract.md)).
