<!-- relocated-from: companions/runtime.md §13 Security Model per ADR 0076 D-8 — non-normative reference document. The kernel does not pin specific security mechanisms; this captures the threat model + posture recommendations a WOS host SHOULD consider. -->

# WOS Security Model (non-normative)
This section is normative.

## 1 Engine Isolation

The evaluation engine MUST NOT have direct network access. All external communication flows through the ExternalService interface (S12.4). This constraint ensures the engine is a pure computational component: given the same inputs (documents, events, host interface responses), it produces the same outputs. Network access would introduce non-determinism.

## 2 Expression Sandboxing

FEL expressions are inherently sandboxed -- FEL has no I/O operations, no network access, no filesystem access, and no ability to invoke external services (Formspec Core S3). This sandboxing is a property of FEL itself, not an implementation requirement on the processor.

## 3 Data Protection

Case state containing personally identifiable information (PII) SHOULD be encrypted at rest by the host (via the InstanceStore implementation), not by the engine. The engine processes case state in memory; the host is responsible for storage-level encryption.

## 4 Provenance Immutability

Provenance records SHOULD be immutable at the storage level. The host SHOULD implement provenance storage as write-once (append-only), preventing modification or deletion of existing records. This is a SHOULD, not a MUST, because some regulatory frameworks require provenance expungement under specific legal orders.

---