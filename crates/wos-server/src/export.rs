//! Offline export CLI: `wos-server export <instance-id> --format prov-o|xes|ocel`.
//!
//! Reads stored provenance rows (whose payload is a serialised
//! `wos_core::ProvenanceRecord`), reconstructs a `ProvenanceLog`, and
//! feeds it through the `wos-export` crate.

use std::io::Write;

use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};
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

    let log = rows_to_log(&rows)?;
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

pub fn rows_to_log(rows: &[ProvenanceRow]) -> anyhow::Result<ProvenanceLog> {
    let mut log = ProvenanceLog::default();
    for r in rows {
        let record: ProvenanceRecord = serde_json::from_value(r.payload.clone())
            .map_err(|e| anyhow::anyhow!(
                "stored payload for seq {} is not a ProvenanceRecord: {e}",
                r.seq,
            ))?;
        log.push(record);
    }
    Ok(log)
}
