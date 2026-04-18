//! Semantic profile surfaces: PROV-O / XES / OCEL export of a hash-chained
//! provenance log for a given instance. Wraps the `wos-export` crate so the
//! CLI subcommand and the HTTP surface share one code path.

use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};
use wos_export::{ExportConfig, ocel, prov_o, xes};

use crate::error::{ApiError, ApiResult};
use crate::services::provenance_service::ProvenanceService;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    ProvO,
    Xes,
    Ocel,
}

pub struct SemanticService;

impl SemanticService {
    /// Produce a canonical semantic-profile export for the given instance.
    /// `namespace` is used as the PROV-O IRI prefix; if `None`, a default
    /// `urn:wos:prov:wos-server:` namespace is used.
    pub async fn export(
        provenance: &ProvenanceService,
        instance_id: &str,
        format: Format,
        namespace: Option<String>,
    ) -> ApiResult<ExportPayload> {
        let responses = provenance.list(instance_id).await?;
        if responses.is_empty() {
            return Err(ApiError::NotFound);
        }
        let records: Vec<ProvenanceRecord> =
            responses.into_iter().map(|r| r.record).collect();
        let mut log = ProvenanceLog::default();
        for r in records {
            log.push(r);
        }
        let cfg = ExportConfig {
            provenance_namespace: namespace
                .unwrap_or_else(|| "urn:wos:prov:wos-server:".to_string()),
            instance_id: instance_id.to_string(),
        };
        let payload = match format {
            Format::ProvO => {
                let doc = prov_o::export(&log, &cfg);
                ExportPayload::Json {
                    content_type: "application/ld+json".into(),
                    body: serde_json::to_string_pretty(&doc).map_err(|e| {
                        ApiError::ServiceUnavailable(format!("prov-o serialise: {e}"))
                    })?,
                }
            }
            Format::Xes => ExportPayload::Text {
                content_type: "application/xml".into(),
                body: xes::export(&log, &cfg),
            },
            Format::Ocel => ExportPayload::Json {
                content_type: "application/json".into(),
                body: serde_json::to_string_pretty(&ocel::export(&log, &cfg)).map_err(|e| {
                    ApiError::ServiceUnavailable(format!("ocel serialise: {e}"))
                })?,
            },
        };
        Ok(payload)
    }
}

pub enum ExportPayload {
    Json { content_type: String, body: String },
    Text { content_type: String, body: String },
}

impl ExportPayload {
    pub fn content_type(&self) -> &str {
        match self {
            Self::Json { content_type, .. } => content_type,
            Self::Text { content_type, .. } => content_type,
        }
    }

    pub fn body(self) -> String {
        match self {
            Self::Json { body, .. } => body,
            Self::Text { body, .. } => body,
        }
    }
}
