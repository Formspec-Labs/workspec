use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::ProvenanceRow;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("{0}")]
    Backend(String),
}

pub type AuditResult<T> = Result<T, AuditError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEnvelope {
    pub case_id: String,
    pub record_id: String,
    pub event_type: String,
    pub record: serde_json::Value,
}

#[async_trait]
pub trait AuditSink: Send + Sync + 'static {
    async fn append_provenance(&self, records: &[ProvenanceRow]) -> AuditResult<()>;
    async fn append_export(&self, envelope: ExportEnvelope) -> AuditResult<()>;
}

#[derive(Debug, Default)]
pub struct NoopAuditSink;

#[async_trait]
impl AuditSink for NoopAuditSink {
    async fn append_provenance(&self, _records: &[ProvenanceRow]) -> AuditResult<()> {
        Ok(())
    }

    async fn append_export(&self, _envelope: ExportEnvelope) -> AuditResult<()> {
        Ok(())
    }
}
