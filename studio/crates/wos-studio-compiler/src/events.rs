// Rust guideline compliant 2026-05-02

//! Compiler lifecycle event emission per `SA-MUST-cmp-070..073`.
//!
//! The compiler emits structured events at each phase boundary so
//! downstream tooling (workspace dashboards, audit logs, future parent
//! event-bus integrations per PLN-0384) can observe its progress
//! without parsing stdout. Today the events flow through a `Vec<Event>`
//! buffer collected in-memory; the `EventSink` trait abstracts the
//! emission target so a future swap to a real event bus is one type
//! change.
//!
//! Event shape (PLN-0384-aligned):
//!
//! ```json
//! {
//!   "kind": "wos.compiler.phase-started",
//!   "phase": 4,
//!   "timestamp": "2026-05-02T12:00:00Z",
//!   "payload": { ... }
//! }
//! ```
//!
//! The compiler does NOT timestamp events with wall-clock time today
//! (would defeat `SA-MUST-cmp-001` deterministic output); event
//! ordering by sequence number stands in. A future lift to a real
//! event bus will surface `timestamp` from the bus's ingestion clock,
//! which is fine because timestamps land in operator-side observability,
//! not the compile artifact itself.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One compiler lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerEvent {
    /// Stable kind string, dot-namespaced under `wos.compiler.`.
    pub kind: String,
    /// Phase number the event corresponds to (1..=8). Compile-level
    /// events (`compile-succeeded`, `compile-failed`) carry 0.
    pub phase: u8,
    /// Monotonically-increasing sequence number per compile, starting
    /// at 0. Replaces wall-clock time for deterministic output.
    pub sequence: u32,
    /// Phase-specific payload. Open-shape; downstream consumers parse
    /// per `kind`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// Sink for compiler events. The default implementation buffers
/// events in memory; production swaps would replace this with a
/// real bus client (NATS, Kafka, parent PLN-0384 substrate, etc.).
pub trait EventSink {
    fn emit(&mut self, event: CompilerEvent);
}

/// In-memory event buffer. Used by the compile pipeline to collect
/// every emitted event and surface them in `CompileArtifact.events`
/// for the CLI / dry-run paths.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventBuffer {
    events: Vec<CompilerEvent>,
    next_sequence: u32,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience: build + emit a phase-started event.
    pub fn phase_started(&mut self, phase: u8) {
        self.emit(CompilerEvent {
            kind: "wos.compiler.phase-started".to_string(),
            phase,
            sequence: 0, // overwritten by emit()
            payload: None,
        });
    }

    /// Convenience: build + emit a phase-completed event.
    pub fn phase_completed(&mut self, phase: u8) {
        self.emit(CompilerEvent {
            kind: "wos.compiler.phase-completed".to_string(),
            phase,
            sequence: 0,
            payload: None,
        });
    }

    /// Convenience: build + emit a phase-halted event.
    pub fn phase_halted(&mut self, phase: u8, kind: &str, message: &str) {
        self.emit(CompilerEvent {
            kind: "wos.compiler.phase-halted".to_string(),
            phase,
            sequence: 0,
            payload: Some(serde_json::json!({
                "failureKind": kind,
                "message": message,
            })),
        });
    }

    /// Convenience: build + emit a gate-passed or gate-failed event.
    pub fn gate(&mut self, gate: &str, passed: bool, finding_count: usize) {
        let kind = if passed {
            "wos.compiler.gate-passed"
        } else {
            "wos.compiler.gate-failed"
        };
        self.emit(CompilerEvent {
            kind: kind.to_string(),
            phase: 7,
            sequence: 0,
            payload: Some(serde_json::json!({
                "gate": gate,
                "findingCount": finding_count,
            })),
        });
    }

    /// Convenience: build + emit a compile-succeeded event with the
    /// final manifest hash.
    pub fn compile_succeeded(&mut self, manifest_hash: &str) {
        self.emit(CompilerEvent {
            kind: "wos.compiler.compile-succeeded".to_string(),
            phase: 0,
            sequence: 0,
            payload: Some(serde_json::json!({
                "manifestHash": manifest_hash,
            })),
        });
    }

    /// Convenience: build + emit a compile-failed event.
    pub fn compile_failed(&mut self, reason: &str) {
        self.emit(CompilerEvent {
            kind: "wos.compiler.compile-failed".to_string(),
            phase: 0,
            sequence: 0,
            payload: Some(serde_json::json!({
                "reason": reason,
            })),
        });
    }

    /// All emitted events, in emission order.
    pub fn events(&self) -> &[CompilerEvent] {
        &self.events
    }

    /// Drain into JSON-Lines format suitable for writing alongside
    /// the compile artifacts. Each line is one event's JSON.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for e in &self.events {
            if let Ok(line) = serde_json::to_string(e) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }
}

impl EventSink for EventBuffer {
    fn emit(&mut self, mut event: CompilerEvent) {
        event.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_assigns_sequence_in_emission_order() {
        let mut buf = EventBuffer::new();
        buf.phase_started(1);
        buf.phase_completed(1);
        buf.phase_started(2);
        let events = buf.events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence, 0);
        assert_eq!(events[1].sequence, 1);
        assert_eq!(events[2].sequence, 2);
        assert_eq!(events[0].kind, "wos.compiler.phase-started");
        assert_eq!(events[2].phase, 2);
    }

    #[test]
    fn jsonl_serializes_one_event_per_line() {
        let mut buf = EventBuffer::new();
        buf.phase_started(1);
        buf.compile_succeeded("sha256:abc");
        let jsonl = buf.to_jsonl();
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line is valid JSON.
        for line in &lines {
            let _: CompilerEvent =
                serde_json::from_str(line).expect("each line is valid JSON");
        }
    }

    #[test]
    fn gate_event_payload_carries_finding_count() {
        let mut buf = EventBuffer::new();
        buf.gate("schema-pass", false, 3);
        let e = &buf.events()[0];
        assert_eq!(e.kind, "wos.compiler.gate-failed");
        assert_eq!(e.phase, 7);
        let payload = e.payload.as_ref().expect("payload present");
        assert_eq!(payload["gate"], serde_json::json!("schema-pass"));
        assert_eq!(payload["findingCount"], serde_json::json!(3));
    }
}
