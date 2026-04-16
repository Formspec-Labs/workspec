//! PROV-O JSON-LD serializer (WOS Semantic Profile §5).
//!
//! Serializes a [`ProvenanceLog`] into a W3C PROV-O graph expressed as JSON-LD.
//! Every Facts tier record becomes a `prov:Activity` node linked (when an actor
//! is known) to a deduplicated `prov:Agent` node, per §5.3.
//!
//! # Actor type limitation
//!
//! §5.5 maps WOS actor types (`human` / `system` / `agent`) to specific
//! `prov:Agent` subclasses (`prov:Person`, `prov:SoftwareAgent`, plus WOS
//! domain types `wos:HumanAgent` / `wos:SystemAgent` / `wos:AIAgent`).
//! [`ProvenanceRecord`] currently carries only an opaque `actor_id` with no
//! actor-type discriminator, so this exporter emits the defensible
//! "unknown actor type" fallback: every agent node is typed as plain
//! `prov:Agent`. Consumers that need the richer subclass annotation must
//! enrich the graph out of band (e.g., via an actor registry join).

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};

use crate::ExportConfig;

/// A PROV-O graph serialized as JSON-LD (§5.6).
#[derive(Debug, Serialize)]
pub struct ProvODocument {
    /// JSON-LD `@context` declaring the `prov:`, `xsd:`, and `wos:` prefixes.
    #[serde(rename = "@context")]
    pub context: Value,
    /// The PROV-O graph: activities and deduplicated agents.
    #[serde(rename = "@graph")]
    pub graph: Vec<Value>,
}

/// Serialize a provenance log to a PROV-O JSON-LD document (§5.3, §5.6).
///
/// Each [`ProvenanceRecord`] becomes a `prov:Activity`. Agents are
/// deduplicated: each distinct `actor_id` produces one `prov:Agent` node
/// regardless of how many activities reference it.
///
/// Records with an empty `timestamp` (never persistence-stamped; see
/// [`ProvenanceRecord::timestamp`] docs) omit `prov:atTime` because the
/// canonical time is unknown — emitting an empty string would poison any
/// downstream `xsd:dateTime` consumer.
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> ProvODocument {
    let records = log.records();

    // Agents are emitted in first-seen order (the `agents` Vec records the
    // order; `seen_actors` is only a membership filter). Stable ordering
    // keeps snapshot diffs clean for Task 5 fixtures.
    let mut seen_actors = BTreeSet::new();
    let mut activities = Vec::with_capacity(records.len());
    let mut agents = Vec::new();

    for (index, record) in records.iter().enumerate() {
        activities.push(activity_node(index, record, config));

        if let Some(actor_id) = record.actor_id.as_deref()
            && seen_actors.insert(actor_id.to_owned())
        {
            agents.push(agent_node(actor_id, config));
        }
    }

    let mut graph = activities;
    graph.extend(agents);

    ProvODocument {
        context: context_object(),
        graph,
    }
}

/// Build the `@context` object. The three prefixes called out in §5.6 /
/// Appendix C are mandatory for a valid PROV-O JSON-LD emission.
fn context_object() -> Value {
    json!({
        "prov": "http://www.w3.org/ns/prov#",
        "xsd": "http://www.w3.org/2001/XMLSchema#",
        "wos": "https://wos-spec.org/ns/",
    })
}

/// Emit a `prov:Activity` node for a single record (§5.3).
fn activity_node(index: usize, record: &ProvenanceRecord, config: &ExportConfig) -> Value {
    // `ProvenanceRecord` has no stable id today; mint a deterministic one
    // from the record's position in the log. When a record id is added
    // upstream this is the one site that needs to change.
    let activity_id = format!("{}{index}", config.provenance_namespace);
    let action_type = camel_case_record_kind(record);

    let mut node = serde_json::Map::new();
    node.insert("@id".into(), Value::String(activity_id));
    node.insert("@type".into(), Value::String("prov:Activity".into()));
    node.insert("wos:actionType".into(), Value::String(action_type));

    if !record.timestamp.is_empty() {
        node.insert(
            "prov:atTime".into(),
            Value::String(record.timestamp.clone()),
        );
    }

    if let Some(actor_id) = record.actor_id.as_deref() {
        node.insert(
            "prov:wasAssociatedWith".into(),
            Value::String(agent_iri(actor_id, config)),
        );
    }

    Value::Object(node)
}

/// Emit a `prov:Agent` node for a unique actor id (§5.3, §5.5 fallback).
fn agent_node(actor_id: &str, config: &ExportConfig) -> Value {
    json!({
        "@id": agent_iri(actor_id, config),
        "@type": "prov:Agent",
    })
}

/// Compose the IRI used for a `prov:Agent` node so activities can link to it.
fn agent_iri(actor_id: &str, config: &ExportConfig) -> String {
    format!("{}agent/{actor_id}", config.provenance_namespace)
}

/// Render `record_kind` in camelCase, reusing the serde rename already on
/// [`ProvenanceKind`] — guarantees the emitted string matches the spec's
/// on-the-wire form (§5.3 example: `"stateTransition"`).
fn camel_case_record_kind(record: &ProvenanceRecord) -> String {
    match serde_json::to_value(record.record_kind) {
        Ok(Value::String(name)) => name,
        // The enum is `#[serde(rename_all = "camelCase")]` over plain
        // unit variants, so serialization cannot fail or produce a
        // non-string value. Treat any surprise as a bug.
        other => unreachable!("ProvenanceKind must serialize as a string, got {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            instance_id: "test-instance".to_string(),
        }
    }

    fn stamped_transition(actor: Option<&str>) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", actor);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        record
    }

    #[test]
    fn context_includes_prov_xsd_wos_namespaces() {
        let log = ProvenanceLog::default();
        let document = export(&log, &config());

        let context = &document.context;
        assert_eq!(context["prov"], "http://www.w3.org/ns/prov#");
        assert_eq!(context["xsd"], "http://www.w3.org/2001/XMLSchema#");
        assert_eq!(context["wos"], "https://wos-spec.org/ns/");
    }

    #[test]
    fn exports_state_transition_as_prov_activity() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-42")));

        let document = export(&log, &config());

        // One activity + one agent.
        assert_eq!(document.graph.len(), 2);

        let activity = &document.graph[0];
        assert_eq!(activity["@id"], "urn:wos:prov:test:0");
        assert_eq!(activity["@type"], "prov:Activity");
        assert_eq!(activity["wos:actionType"], "stateTransition");
        assert_eq!(activity["prov:atTime"], "2026-01-01T00:00:00Z");
        assert_eq!(
            activity["prov:wasAssociatedWith"],
            "urn:wos:prov:test:agent/user-42"
        );

        let agent = &document.graph[1];
        assert_eq!(agent["@id"], "urn:wos:prov:test:agent/user-42");
        assert_eq!(agent["@type"], "prov:Agent");
    }

    #[test]
    fn omits_prov_at_time_when_timestamp_is_empty() {
        let mut log = ProvenanceLog::default();
        // Unstamped: timestamp is empty. §5.3 documents this as "unknown".
        log.push(ProvenanceRecord::state_transition(
            "a",
            "b",
            "ev",
            Some("user-1"),
        ));

        let document = export(&log, &config());

        let activity = &document.graph[0];
        assert!(
            activity.get("prov:atTime").is_none(),
            "empty timestamp must not emit prov:atTime, got: {activity}"
        );
        // Other properties still present.
        assert_eq!(activity["@type"], "prov:Activity");
        assert_eq!(activity["wos:actionType"], "stateTransition");
    }

    #[test]
    fn deduplicates_agents_across_records() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped_transition(Some("user-42")));
        }

        let document = export(&log, &config());

        // 3 activities + exactly 1 agent.
        assert_eq!(document.graph.len(), 4);
        let agents: Vec<_> = document
            .graph
            .iter()
            .filter(|node| node["@type"] == "prov:Agent")
            .collect();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["@id"], "urn:wos:prov:test:agent/user-42");
    }

    #[test]
    fn omits_agent_link_when_actor_id_absent() {
        let mut log = ProvenanceLog::default();
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        log.push(record);

        let document = export(&log, &config());

        // No agent node, just the activity.
        assert_eq!(document.graph.len(), 1);
        let activity = &document.graph[0];
        assert!(
            activity.get("prov:wasAssociatedWith").is_none(),
            "actor-less record must not emit prov:wasAssociatedWith"
        );
        assert!(
            !document
                .graph
                .iter()
                .any(|node| node["@type"] == "prov:Agent"),
            "actor-less record must not emit any prov:Agent node"
        );
    }

    #[test]
    fn camel_cases_all_record_kinds() {
        // Spot-check a few variants beyond StateTransition to guard against
        // future enum additions that might accidentally break the rename.
        let mut log = ProvenanceLog::default();
        for kind in [
            ProvenanceKind::CaseStateMutation,
            ProvenanceKind::TimerFired,
            ProvenanceKind::DeonticViolation,
        ] {
            let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
            record.record_kind = kind;
            record.timestamp = "2026-01-01T00:00:00Z".into();
            log.push(record);
        }

        let document = export(&log, &config());
        assert_eq!(document.graph[0]["wos:actionType"], "caseStateMutation");
        assert_eq!(document.graph[1]["wos:actionType"], "timerFired");
        assert_eq!(document.graph[2]["wos:actionType"], "deonticViolation");
    }
}
