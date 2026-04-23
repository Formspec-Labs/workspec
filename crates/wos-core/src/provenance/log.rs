// Rust guideline compliant 2026-02-21

use super::record::ProvenanceRecord;

/// Append-only provenance log.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceLog {
    records: Vec<ProvenanceRecord>,
}

impl ProvenanceLog {
    /// Append a record.
    pub fn push(&mut self, record: ProvenanceRecord) {
        self.records.push(record);
    }

    /// All records in order.
    pub fn records(&self) -> &[ProvenanceRecord] {
        &self.records
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
