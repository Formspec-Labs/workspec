//! Offline export CLI: `wos-server export <instance-id> --format prov-o|xes|ocel`.
//!
//! Reads stored provenance rows, reconstructs `wos_core::ProvenanceLog`, and
//! feeds it through the `wos-export` crate. Useful for producing the PROV-O /
//! XES / OCEL artifacts the Semantic Profile requires without spinning up the
//! HTTP server.

use std::io::Write;

use wos_core::provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};
use wos_export::{ExportConfig, ocel, prov_o, xes};

use crate::config::{ExportArgs, ExportFormat};
use crate::storage::{self, ProvenanceRow};

pub async fn run(args: ExportArgs) -> anyhow::Result<()> {
    let storage = storage::build(&args.server).await?;
    let rows = storage.list_provenance(&args.instance_id).await?;
    if rows.is_empty() {
        anyhow::bail!(
            "no provenance records for instance `{}` (is this the right id?)",
            args.instance_id
        );
    }

    let log = rows_to_log(&rows);
    let cfg = ExportConfig {
        provenance_namespace: normalise_namespace(&args.namespace),
        instance_id: args.instance_id.clone(),
    };

    let payload = match args.format {
        ExportFormat::ProvO => serde_json::to_string_pretty(&prov_o::export(&log, &cfg))?,
        ExportFormat::Xes => xes::export(&log, &cfg),
        ExportFormat::Ocel => serde_json::to_string_pretty(&ocel::export(&log, &cfg))?,
    };

    write_output(&args.out, payload.as_bytes())?;
    Ok(())
}

/// Ensure the namespace ends with a separator so PROV-O IRI minting stays
/// well-formed. The wos-export crate documents this requirement on
/// `ExportConfig.provenance_namespace`.
fn normalise_namespace(ns: &str) -> String {
    if ns.ends_with(':') || ns.ends_with('/') {
        ns.to_string()
    } else if ns.starts_with("urn:") {
        format!("{ns}:")
    } else {
        format!("{ns}/")
    }
}

fn write_output(out: &str, bytes: &[u8]) -> anyhow::Result<()> {
    if out == "-" {
        std::io::stdout().write_all(bytes)?;
        std::io::stdout().write_all(b"\n")?;
    } else {
        std::fs::write(out, bytes)?;
    }
    Ok(())
}

/// Convert server-native `ProvenanceRow`s back to `wos_core::ProvenanceRecord`s.
/// The row's loose `payload` JSON carries the fields populated by
/// `eval_service` (`event`, `sourceState`, `targetState`, `actor`, `facts`).
pub fn rows_to_log(rows: &[ProvenanceRow]) -> ProvenanceLog {
    let mut log = ProvenanceLog::default();
    for r in rows {
        log.push(row_to_record(r));
    }
    log
}

fn row_to_record(r: &ProvenanceRow) -> ProvenanceRecord {
    let payload = &r.payload;
    let event = payload.get("event").and_then(|v| v.as_str()).map(String::from);
    let from_state = payload
        .get("sourceState")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let to_state = payload
        .get("targetState")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let actor_obj = payload.get("actor");
    let actor_id = actor_obj
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let actor_type = actor_obj
        .and_then(|a| a.get("type"))
        .and_then(|v| v.as_str())
        .map(String::from);

    ProvenanceRecord {
        record_kind: kind_for(event.as_deref()),
        timestamp: r.timestamp.to_rfc3339(),
        actor_id,
        from_state,
        to_state,
        event,
        data: payload.get("facts").cloned(),
        audit_layer: Some(r.tier.clone()),
        actor_type,
        lifecycle_state: None,
        definition_version: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        input_digest: None,
        output_digest: None,
    }
}

fn kind_for(_event: Option<&str>) -> ProvenanceKind {
    // Every row we write through `EvalService::submit_event` corresponds to
    // a state transition or a case-state mutation at the WOS semantics
    // layer. Classifying as `StateTransition` is faithful to the payloads we
    // currently mint; specialised kinds (Milestone*, Timer*, Deontic*) can
    // be plumbed through `ProvenanceRow.tier` in a follow-up once the eval
    // service surfaces the wos-core provenance cursor directly.
    ProvenanceKind::StateTransition
}
