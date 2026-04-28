<!-- relocated-from: companions/lifecycle-detail.md §7 SCXML Interoperability Mapping per ADR 0076 D-8 + 2026-04-28 deletion. Non-normative reference for translating WOS Kernel Documents to/from W3C SCXML. The mapping is bidirectional with documented losses for WOS-specific concepts (tags, cancellation policies, milestones, kernel-generated events). -->

# WOS ↔ SCXML Interoperability Mapping (non-normative)
This section is informative.

## 1 Purpose

This section defines how WOS Kernel Documents map to and from W3C SCXML documents, enabling interoperability with existing SCXML-based workflow engines. The mapping is bidirectional: a WOS document can be translated to SCXML for execution, and an SCXML document can be imported as a WOS kernel document (with some loss of WOS-specific metadata).

## 2 State Type Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `atomic` | `<state>` (no child states) | Direct mapping. |
| `compound` | `<state>` (with child `<state>` elements) | WOS `initialState` maps to SCXML `initial` attribute. |
| `parallel` | `<parallel>` | WOS regions map to child `<state>` elements within `<parallel>`. |
| `final` | `<final>` | Direct mapping. |

## 3 Transition Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `event` | `event` attribute | Direct mapping. |
| `target` | `target` attribute | Direct mapping. |
| `guard` | `cond` attribute | FEL expression must be translated to the SCXML datamodel's expression language. |
| `actions` | `<script>` or executable content | WOS actions map to SCXML executable content. `setData` maps to `<assign>`. `emitEvent` maps to `<send>`. `startTimer` maps to `<send>` with delay. |
| `tags` | No SCXML equivalent. | Tags are WOS-specific metadata. Dropped on export, ignored on import. |

## 4 Action Mapping

| WOS Action | SCXML Element | Notes |
|------------|---------------|-------|
| `setData` | `<assign>` | `path` maps to `location`, `value` maps to `expr`. |
| `emitEvent` | `<send>` | `eventType` maps to `event`, `data` maps to content. |
| `startTimer` | `<send>` with `delay`/`delayexpr` | Timer semantics differ: SCXML `<send>` with delay is less structured than WOS durable timers. |
| `cancelTimer` | `<cancel>` | `timerId` maps to `sendid`. |
| `log` | `<log>` | Direct mapping. |
| `createTask` | No SCXML equivalent. | Task creation is WOS-specific. |
| `invokeService` | `<invoke>` | `serviceRef` maps to `src` or `type`. |

## 5 History State Mapping

| WOS Kernel | SCXML | Notes |
|------------|-------|-------|
| `historyState: "shallow"` | `<history type="shallow">` | Direct mapping. |
| `historyState: "deep"` | `<history type="deep">` | Direct mapping. |

## 6 Cancellation Policy Mapping

WOS `cancellationPolicy` has no direct SCXML equivalent. SCXML `<parallel>` always uses `wait-all` semantics. The `cancel-siblings` and `fail-fast` policies require SCXML extensions or post-processing.

## 7 Limitations

The following WOS kernel concepts have no SCXML equivalent and are dropped on export:

- Semantic transition tags (`tags`)
- Cancellation policies other than `wait-all`
- Impact level classification
- Case file schema
- Provenance configuration
- Contract references
- Milestone conditions (SCXML has no milestone concept)
- Kernel-generated events with `$` prefix (must be renamed)

SCXML concepts not used by WOS:

- `<script>` executable content (WOS uses typed actions)
- ECMAScript/XPath expressions (WOS uses FEL)
- `<invoke>` platform-specific type identifiers
- `<donedata>` (WOS uses case state)

---