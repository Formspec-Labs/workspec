# Native authoring and provenance qualification

The 2026-10-11 canonical integration makes the selected framework capabilities
available from the maintained package: independent form-version pins, faithful
workflow export, exact queued-event origins, admitted intake actors, complete
native provenance and custody admission, and public task and completion APIs.
The existing absolute-timer behavior remains intact.

Compound `foreach` body transitions now reuse the normal transition-recording
helper. Each actual iteration retains its own `$bodyAuto` record; transition,
source-exit and destination-entry emissions reference that record. The native
regression first failed against the previous body implementation because its
origin named the outer `ready` transition. The fixed implementation passed both
initial-entry and event-entry controls.

| Check | Result |
| --- | --- |
| `wos-core` library, evaluator, kernel deserialization and export | 258 passed |
| `wos-events` library | 32 passed |
| `wos-authoring` library | 53 passed |
| `wos-runtime` library | 313 passed |
| `wos-mcp` library and stdio transport | 78 passed |
| API and kernel schema suite | 678 passed; one existing strict semantic-only expected failure |
| Completion-result wire equivalence | 256 combinations plus missing/extra fields preserved |
| Maintained Portal generation and identical TypeScript controls | Previous output failed; current output passed without casts |
| OpenAPI dual-surface parity | Two missing public routes repaired; zero drift across 41 registered operations |
| Independent semi-formal code and architecture reviews | APPROVE, including final scope addenda |

The custody test fixture initially omitted required provenance metadata. Its
final control verifies refusal of that incomplete record, then applies the
maintained metadata helper before publication. Production validation stays
strict; the complete native-window and actual router controls passed.

Native commands used `--offline --locked`, one build job, maintained sibling
repositories and the existing shared target. The tested dependency cut was
Common `9a0712714a31918de5b2846ea6b398d7c4fc07c4`, Integrity
`e5be2622bffc68aaf47c021c471b5f30f979d0f8`, and Trellis
`246610e517a2dc369e018ef223c62190328023fa`.

WOS owns generated-registry emission and host configuration qualification.
Its canonical emitter subsequently supplied exact registry bytes with SHA-256
`141c0303174aaa1b01ec92e03b88413210af1e7a8fc022ef786155b349dda8bb`.
Final parity passed across 44 registered operations, and 59 affected schema/API
checks passed against those emitted bytes.
Source and WOS own accepted-record authority; Trellis owns append and archive
verification. These package checks qualify the framework components. The
deployment owners qualify runtime selection and the completed application
journey. The authored `0316` notice successor remains inactive.
