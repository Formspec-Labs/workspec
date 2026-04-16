// Rust guideline compliant 2026-02-21

//! IEEE 1849-2016 XES XML serializer (WOS Semantic Profile §6.3).
//!
//! Serializes a [`wos_core::provenance::ProvenanceLog`] into an XES XML
//! document as a single `<trace>` whose `<event>` elements correspond, in
//! order, to the records in the log.
//!
//! # Lifecycle attribute choice
//!
//! Per §6.3, WOS deliberately does NOT map its lifecycle states onto the
//! standard XES `lifecycle:transition` attribute. The standard XES Lifecycle
//! extension defines a fixed vocabulary (`start`, `complete`, `suspend`,
//! `resume`, …) that does not correspond to workflow-specific WOS state
//! names. We therefore emit a custom `wos:lifecycleState` attribute instead;
//! standards-compliant XES consumers will preserve it as an unknown
//! extension attribute rather than mis-interpreting WOS states as transition
//! codes.
//!
//! # Missing timestamps
//!
//! Records whose `timestamp` is empty have never been persistence-stamped
//! (see [`wos_core::provenance::ProvenanceRecord`]). For those events we
//! OMIT the `<date key="time:timestamp" ...>` element entirely — an empty
//! string is not a valid `xs:dateTime` value and would poison any
//! downstream tool. Consumers should treat the missing element as
//! "timestamp unknown".

use quick_xml::events::{BytesDecl, Event};
use quick_xml::writer::Writer;

use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};

use crate::{ExportConfig, camel_case_record_kind};

/// Serialize a provenance log to an XES XML document (§6.3).
///
/// Writes into an in-memory `Vec<u8>` via [`quick_xml::writer::Writer`] and
/// returns the UTF-8 rendered document.
///
/// # Panics
///
/// Never panics in practice: `quick-xml` only returns errors on I/O
/// failures from the underlying writer, and writing into a `Vec<u8>`
/// cannot fail. Any surprise here is a contract violation in `quick-xml`
/// itself.
#[must_use]
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> String {
    let mut buffer: Vec<u8> = Vec::new();
    // Indent with two spaces so snapshot fixtures diff cleanly. The
    // XES spec does not mandate a specific whitespace style.
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);

    // Writing into a `Vec<u8>` is infallible, so every `?`-style call below
    // is expressed via `expect("write to Vec<u8> cannot fail")`.
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .expect("write to Vec<u8> cannot fail");

    writer
        .create_element("log")
        .with_attributes([
            ("xes.version", "2.0"),
            ("xes.features", "nested-attributes"),
            ("xmlns", "http://www.xes-standard.org/"),
        ])
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_extension_declarations(w)?;
            write_trace(w, log, config)?;
            Ok(())
        })
        .expect("write to Vec<u8> cannot fail");

    String::from_utf8(buffer).expect("quick-xml emits UTF-8 by construction")
}

/// Declare the XES standard extensions used by this exporter (§6.3).
///
/// Concept, Time, and Lifecycle are MUST; Organizational and Identity are
/// SHOULD, but we always emit them so round-trips stay lossless.
fn write_extension_declarations(
    writer: &mut Writer<&mut Vec<u8>>,
) -> Result<(), quick_xml::Error> {
    for (name, prefix, uri) in [
        (
            "Concept",
            "concept",
            "http://www.xes-standard.org/concept.xesext",
        ),
        ("Time", "time", "http://www.xes-standard.org/time.xesext"),
        (
            "Lifecycle",
            "lifecycle",
            "http://www.xes-standard.org/lifecycle.xesext",
        ),
        (
            "Organizational",
            "org",
            "http://www.xes-standard.org/org.xesext",
        ),
        (
            "Identity",
            "identity",
            "http://www.xes-standard.org/identity.xesext",
        ),
    ] {
        writer
            .create_element("extension")
            .with_attributes([("name", name), ("prefix", prefix), ("uri", uri)])
            .write_empty()?;
    }
    Ok(())
}

/// Emit the single `<trace>` and its events.
fn write_trace(
    writer: &mut Writer<&mut Vec<u8>>,
    log: &ProvenanceLog,
    config: &ExportConfig,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("trace")
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_string_attribute(w, "concept:name", &config.instance_id)?;
            for (index, record) in log.records().iter().enumerate() {
                write_event(w, index, record)?;
            }
            Ok(())
        })?;
    Ok(())
}

/// Emit one `<event>` for a provenance record (§6.3 mapping table).
fn write_event(
    writer: &mut Writer<&mut Vec<u8>>,
    index: usize,
    record: &ProvenanceRecord,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("event")
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_string_attribute(w, "concept:name", &camel_case_record_kind(record))?;

            if !record.timestamp.is_empty() {
                write_date_attribute(w, "time:timestamp", &record.timestamp)?;
            }

            // §6.3: actorId → org:resource. Records without an actor are
            // attributed to the runtime itself ("system"), matching PROV-O
            // sibling behaviour where the actor becomes a synthetic agent.
            let resource = record.actor_id.as_deref().unwrap_or("system");
            write_string_attribute(w, "org:resource", resource)?;

            // ProvenanceRecord has no stable id; the record's position in the
            // log is a deterministic identifier that also doubles as a sort
            // key for consumers.
            write_string_attribute(w, "identity:id", &index.to_string())?;

            // CUSTOM attribute (see module docs) — deliberately NOT
            // lifecycle:transition. WOS from_state carries the workflow
            // state at event time; empty string is the spec-compliant
            // representation for "no source state" (e.g. initial entry).
            let lifecycle_state = record.from_state.as_deref().unwrap_or("");
            write_string_attribute(w, "wos:lifecycleState", lifecycle_state)?;

            Ok(())
        })?;
    Ok(())
}

/// Helper: `<string key="..." value="..."/>`.
fn write_string_attribute(
    writer: &mut Writer<&mut Vec<u8>>,
    key: &str,
    value: &str,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("string")
        .with_attributes([("key", key), ("value", value)])
        .write_empty()?;
    Ok(())
}

/// Helper: `<date key="..." value="..."/>`.
fn write_date_attribute(
    writer: &mut Writer<&mut Vec<u8>>,
    key: &str,
    value: &str,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("date")
        .with_attributes([("key", key), ("value", value)])
        .write_empty()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::Reader;
    use quick_xml::events::Event as ReaderEvent;
    use wos_core::provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            instance_id: "benefits-001".to_string(),
        }
    }

    fn stamped_transition(actor: Option<&str>) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("Draft", "Review", "submit", actor);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        record
    }

    /// Parse the XES string and count occurrences of the named element. Fails
    /// the test if any parse error surfaces.
    fn count_elements(xml: &str, tag: &str) -> usize {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut count = 0usize;
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(ReaderEvent::Eof) => break,
                Ok(ReaderEvent::Start(ref element)) | Ok(ReaderEvent::Empty(ref element)) => {
                    if element.name().as_ref() == tag.as_bytes() {
                        count += 1;
                    }
                }
                Ok(_) => {}
                Err(error) => panic!("XES parse failed at position {}: {error:?}", reader.buffer_position()),
            }
            buffer.clear();
        }
        count
    }

    #[test]
    fn exports_log_produces_valid_xml() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));
        log.push(stamped_transition(Some("user-2")));

        let xml = export(&log, &config());

        // Root + one trace + two events.
        assert_eq!(count_elements(&xml, "log"), 1);
        assert_eq!(count_elements(&xml, "trace"), 1);
        assert_eq!(count_elements(&xml, "event"), 2);
        // XML declaration is present.
        assert!(xml.starts_with("<?xml"), "missing XML declaration: {xml}");
    }

    #[test]
    fn each_event_has_concept_name_and_timestamp() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));
        log.push(stamped_transition(None));

        let xml = export(&log, &config());

        // Split on <event> ... </event> pairs and assert attributes are
        // present inside each event block. A stamped record must emit both
        // concept:name and time:timestamp.
        let events: Vec<&str> = xml.split("<event>").skip(1).collect();
        assert_eq!(events.len(), 2);
        for event_block in events {
            assert!(
                event_block.contains(r#"key="concept:name""#),
                "event missing concept:name: {event_block}"
            );
            assert!(
                event_block.contains(r#"key="time:timestamp""#),
                "event missing time:timestamp: {event_block}"
            );
        }
    }

    #[test]
    fn omits_timestamp_when_record_unstamped() {
        let mut log = ProvenanceLog::default();
        // Unstamped — constructor leaves timestamp empty.
        log.push(ProvenanceRecord::state_transition(
            "Draft",
            "Review",
            "submit",
            Some("user-1"),
        ));

        let xml = export(&log, &config());

        // No <date ...> element anywhere — there is only one event and it is
        // unstamped.
        assert!(
            !xml.contains("<date"),
            "unstamped record must not emit <date>: {xml}"
        );
        // But concept:name still present, so we didn't accidentally skip the event.
        assert!(xml.contains(r#"key="concept:name" value="stateTransition""#));
    }

    #[test]
    fn trace_concept_name_matches_instance_id() {
        let log = ProvenanceLog::default();
        let xml = export(&log, &config());

        // The trace header string must carry the configured instance id.
        assert!(
            xml.contains(r#"<string key="concept:name" value="benefits-001"/>"#),
            "trace concept:name not set to instance id: {xml}"
        );
    }

    #[test]
    fn uses_custom_wos_lifecycle_state_not_standard_lifecycle_transition() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));

        let xml = export(&log, &config());

        assert!(
            xml.contains(r#"key="wos:lifecycleState""#),
            "must emit wos:lifecycleState (custom WOS attribute): {xml}"
        );
        assert!(
            !xml.contains("lifecycle:transition"),
            "must NOT emit standard lifecycle:transition — WOS states are not XES lifecycle vocab: {xml}"
        );
    }

    #[test]
    fn identity_id_is_record_index() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped_transition(Some("user-1")));
        }

        let xml = export(&log, &config());

        // Each event carries its zero-based index as identity:id.
        for expected_index in 0..3 {
            let expected = format!(r#"<string key="identity:id" value="{expected_index}"/>"#);
            assert!(
                xml.contains(&expected),
                "missing identity:id={expected_index}: {xml}"
            );
        }
    }

    #[test]
    fn absent_actor_id_falls_back_to_system_resource() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(None));

        let xml = export(&log, &config());

        assert!(
            xml.contains(r#"<string key="org:resource" value="system"/>"#),
            "missing system fallback for org:resource: {xml}"
        );
    }

    #[test]
    fn camel_cases_all_record_kinds() {
        // Guard against enum additions silently breaking the rename contract.
        let mut log = ProvenanceLog::default();
        for kind in [
            ProvenanceKind::CaseStateMutation,
            ProvenanceKind::TimerFired,
            ProvenanceKind::DeonticViolation,
        ] {
            let mut record = stamped_transition(None);
            record.record_kind = kind;
            log.push(record);
        }

        let xml = export(&log, &config());
        assert!(xml.contains(r#"value="caseStateMutation""#));
        assert!(xml.contains(r#"value="timerFired""#));
        assert!(xml.contains(r#"value="deonticViolation""#));
    }

    #[test]
    fn declares_five_extensions() {
        let log = ProvenanceLog::default();
        let xml = export(&log, &config());
        // Must include all five — Concept, Time, Lifecycle, Organizational, Identity.
        assert_eq!(count_elements(&xml, "extension"), 5);
        for prefix in ["concept", "time", "lifecycle", "org", "identity"] {
            let needle = format!(r#"prefix="{prefix}""#);
            assert!(
                xml.contains(&needle),
                "extension with prefix={prefix} missing: {xml}"
            );
        }
    }
}
