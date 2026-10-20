// Rust guideline compliant 2026-02-21
//! Retains actual fixture transport outcomes independently of runtime recreation.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use wos_core::model::due_process::*;

/// Immutable notice lookup and actual qualified local fixture execution.
pub trait NoticeTransport {
    /// Looks up a retained successful result without consuming an attempt.
    ///
    /// # Errors
    /// Refuses immutable key conflicts and unavailable retained storage.
    fn lookup(&self, work: &PendingNoticeWork) -> Result<Option<TransportResult>, RefusalCode>;
    /// Executes exactly the saved work at the supplied shared native clock.
    ///
    /// # Errors
    /// Refuses key conflicts and unavailable retained storage before invocation.
    fn execute(&self, work: &PendingNoticeWork, now: &str) -> Result<TransportResult, RefusalCode>;
}

#[derive(Debug, Clone)]
struct RetainedAttempt {
    work: PendingNoticeWork,
    invocations: usize,
    successful: Option<TransportResult>,
}

/// Deterministic local transport with recoverable successful original clock facts.
#[derive(Debug, Clone)]
pub struct DeterministicNoticeTransport {
    retained: Arc<Mutex<HashMap<(String, String), RetainedAttempt>>>,
    failures: Arc<HashSet<usize>>,
}
impl DeterministicNoticeTransport {
    /// Creates an explicitly isolated result store and numbered no-effect failures.
    ///
    /// Clones share retained work and results across runtime recreation.
    #[must_use]
    pub fn new(failure_attempts: impl IntoIterator<Item = usize>) -> Self {
        Self {
            retained: Arc::default(),
            failures: Arc::new(failure_attempts.into_iter().collect()),
        }
    }
    /// Counts actual new transport invocations for this immutable work key.
    ///
    /// # Errors
    /// Returns a store refusal when retained storage cannot be read.
    pub fn invocations(&self, process_id: &str, transport_key: &str) -> Result<usize, RefusalCode> {
        Ok(self
            .retained
            .lock()
            .map_err(|_| RefusalCode::StoreRejected)?
            .get(&(process_id.into(), transport_key.into()))
            .map_or(0, |a| a.invocations))
    }
}
impl NoticeTransport for DeterministicNoticeTransport {
    fn lookup(&self, work: &PendingNoticeWork) -> Result<Option<TransportResult>, RefusalCode> {
        let saved = self
            .retained
            .lock()
            .map_err(|_| RefusalCode::StoreRejected)?;
        match saved.get(&(work.process_id.clone(), work.transport_key.clone())) {
            Some(attempt) if attempt.work != *work => Err(RefusalCode::ReplayConflict),
            Some(attempt) => Ok(attempt.successful.clone()),
            None => Ok(None),
        }
    }
    fn execute(&self, work: &PendingNoticeWork, now: &str) -> Result<TransportResult, RefusalCode> {
        let mut saved = self
            .retained
            .lock()
            .map_err(|_| RefusalCode::StoreRejected)?;
        let attempt = saved
            .entry((work.process_id.clone(), work.transport_key.clone()))
            .or_insert_with(|| RetainedAttempt {
                work: work.clone(),
                invocations: 0,
                successful: None,
            });
        if attempt.work != *work {
            return Err(RefusalCode::ReplayConflict);
        }
        if let Some(result) = &attempt.successful {
            return Ok(result.clone());
        }
        attempt.invocations += 1;
        if self.failures.contains(&attempt.invocations) {
            return Ok(TransportResult::Failed {
                failure: TransportFailure {
                    kind: FailureKind::NoEffectFailure,
                    code: FailureCode::InjectedFailure,
                },
            });
        }
        let result_id = format!("receipt.{}", work.transport_key);
        let acknowledgement = TransportAcknowledgement {
            kind: AcknowledgementKind::Accepted,
            notice_id: work.notice_id.clone(),
            transport_key: work.transport_key.clone(),
            correlation: work.correlation.clone(),
            artifact_sha256: work.artifact_sha256.clone(),
            channel: work.channel.clone(),
            accepted_at: now.into(),
            adapter_result_id: result_id.clone(),
        };
        let receipt = QualifiedNoticeReceipt {
            kind: ReceiptKind::QualifiedReceipt,
            notice_id: work.notice_id.clone(),
            transport_key: work.transport_key.clone(),
            correlation: work.correlation.clone(),
            artifact_sha256: work.artifact_sha256.clone(),
            channel: work.channel.clone(),
            evidence_class: EvidenceClass::FixtureQualifiedReceipt,
            qualified_at: now.into(),
            adapter_result_id: result_id,
        };
        let result = TransportResult::Qualified {
            acknowledgement,
            receipt,
        };
        attempt.successful = Some(result.clone());
        Ok(result)
    }
}

use super::super::{RuntimeError, WosRuntime};
use super::{admission, operations};
use crate::store::RuntimeRecord;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use wos_core::{ProvenanceKind, ProvenanceRecord};

pub(super) fn work(record: &RuntimeRecord, notice: &NoticeState) -> PendingNoticeWork {
    PendingNoticeWork {
        process_id: record.process.process_id.clone(),
        notice_id: notice.id.clone(),
        determination_id: notice.determination_id.clone(),
        transport_key: notice.transport_key.clone(),
        correlation: notice.correlation.clone(),
        artifact_sha256: notice.artifact_sha256.clone(),
        artifact_bytes_base64: notice.artifact_bytes_base64.clone(),
        recipient: notice.recipient.clone(),
        channel: notice.channel.clone(),
        issue_at: notice.issue_at.clone(),
    }
}
pub(super) fn acknowledgement_matches(
    work: &PendingNoticeWork,
    ack: &TransportAcknowledgement,
) -> bool {
    ack.notice_id == work.notice_id
        && ack.transport_key == work.transport_key
        && ack.correlation == work.correlation
        && ack.artifact_sha256 == work.artifact_sha256
        && ack.channel == work.channel
        && admission::token(&ack.adapter_result_id)
}
pub(super) fn receipt_matches(
    work: &PendingNoticeWork,
    ack: &TransportAcknowledgement,
    receipt: &QualifiedNoticeReceipt,
) -> bool {
    receipt.notice_id == work.notice_id
        && receipt.transport_key == work.transport_key
        && receipt.correlation == work.correlation
        && receipt.artifact_sha256 == work.artifact_sha256
        && receipt.channel == work.channel
        && receipt.adapter_result_id == ack.adapter_result_id
        && receipt.qualified_at == work.issue_at
        && admission::timestamp(&ack.accepted_at)
            .ok()
            .zip(admission::timestamp(&receipt.qualified_at).ok())
            .is_some_and(|(accepted, qualified)| accepted <= qualified)
}
impl WosRuntime {
    /// Recovers or executes immutable saved notice work through the selected transport.
    ///
    /// # Errors
    /// Refuses unqualified capabilities or conflicting results; save failures retain
    /// original successful transport results for recovery after runtime recreation.
    pub fn deliver_pending_notices(&mut self, process_id: &str) -> Result<(), RuntimeError> {
        let record = self.store.load_record(process_id)?;
        let pending: Vec<_> = operations::state(&record)
            .notices
            .iter()
            .filter(|n| n.status != NoticeStatus::Qualified && n.status != NoticeStatus::Exhausted)
            .map(|n| n.id.clone())
            .collect();
        if pending.is_empty() {
            return Ok(());
        }
        if self.preparation.is_some() {
            return Ok(());
        }
        for notice_id in pending {
            self.deliver_one_notice(process_id, &notice_id)?;
        }
        Ok(())
    }
    fn deliver_one_notice(
        &mut self,
        process_id: &str,
        notice_id: &str,
    ) -> Result<(), RuntimeError> {
        let mut record = self.store.load_record(process_id)?;
        let kernel = self.resolver.resolve_kernel(
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        let cap = self
            .reference_due_process
            .as_ref()
            .ok_or(RuntimeError::Governance(RefusalCode::UnsupportedProfile))?;
        let (_, policy) =
            admission::validate_record(cap, &kernel, &record).map_err(RuntimeError::Governance)?;
        let mut state = operations::state(&record);
        let index = state
            .notices
            .iter()
            .position(|n| n.id == notice_id)
            .ok_or(RuntimeError::Governance(RefusalCode::TargetRejected))?;
        let saved_work = work(&record, &state.notices[index]);
        let retained = cap
            .notice_transport
            .lookup(&saved_work)
            .map_err(RuntimeError::Governance)?;
        let result = if let Some(result) = retained {
            result
        } else {
            if state.notices[index]
                .transport_results
                .iter()
                .any(|r| !matches!(r, TransportResult::Failed { .. }))
            {
                return Err(RuntimeError::Governance(RefusalCode::ReplayConflict));
            }
            if state.notices[index].status == NoticeStatus::Blocked {
                return Ok(());
            }
            if state.notices[index].attempts >= policy.limits.max_transport_attempts {
                state.notices[index].status = NoticeStatus::Exhausted;
                state.notices[index].failure = Some(TransportFailure {
                    kind: FailureKind::NoEffectFailure,
                    code: FailureCode::AttemptsExhausted,
                });
                operations::retain(&mut record, state);
                self.store.save_record(record)?;
                return Ok(());
            }
            state.notices[index].attempts += 1;
            operations::retain(&mut record, state.clone());
            // This atomic write is a strict fence before a new transport invocation.
            self.store.save_record(record.clone())?;
            let now = super::super::format_timestamp(self.clock.now_ms())?;
            cap.notice_transport
                .execute(&saved_work, &now)
                .map_err(RuntimeError::Governance)?
        };
        admission::validate_transport_result(&result).map_err(RuntimeError::Governance)?;
        let now = super::super::format_timestamp(self.clock.now_ms())?;
        let notice = &mut state.notices[index];
        if notice
            .transport_results
            .iter()
            .any(|prior| !matches!(prior, TransportResult::Failed { .. }) && prior != &result)
        {
            return Err(RuntimeError::Governance(RefusalCode::ReplayConflict));
        }
        if !notice.transport_results.contains(&result) {
            if notice.transport_results.len() >= notice.attempts {
                return Err(RuntimeError::Governance(RefusalCode::StoreRejected));
            }
            notice.transport_results.push(result.clone());
        }
        match result {
            TransportResult::Failed { failure } => {
                notice.failure = Some(failure);
                if notice.attempts >= policy.limits.max_transport_attempts {
                    notice.status = NoticeStatus::Exhausted;
                }
            }
            TransportResult::AcceptedOnly { acknowledgement } => {
                if !admit_ack(&saved_work, &acknowledgement, &now) {
                    notice.status = NoticeStatus::Blocked;
                } else if !notice.acknowledgements.contains(&acknowledgement) {
                    notice.acknowledgements.push(acknowledgement);
                }
            }
            TransportResult::Qualified {
                acknowledgement,
                receipt,
            } => {
                if !admit_ack(&saved_work, &acknowledgement, &now)
                    || !receipt_matches(&saved_work, &acknowledgement, &receipt)
                    || admission::timestamp(&receipt.qualified_at).is_err()
                {
                    notice.status = NoticeStatus::Blocked;
                    if admit_ack(&saved_work, &acknowledgement, &now)
                        && !notice.acknowledgements.contains(&acknowledgement)
                    {
                        notice.acknowledgements.push(acknowledgement);
                    }
                } else {
                    if notice.receipt.as_ref().is_some_and(|r| r != &receipt) {
                        return Err(RuntimeError::Governance(RefusalCode::ReplayConflict));
                    }
                    if !notice.acknowledgements.contains(&acknowledgement) {
                        notice.acknowledgements.push(acknowledgement.clone());
                    }
                    notice.status = NoticeStatus::Qualified;
                    notice.receipt = Some(receipt.clone());
                    let bytes = STANDARD
                        .decode(&notice.artifact_bytes_base64)
                        .map_err(|_| RuntimeError::Governance(RefusalCode::StoreRejected))?;
                    let artifact: serde_json::Value = serde_json::from_slice(&bytes)
                        .map_err(|_| RuntimeError::Governance(RefusalCode::StoreRejected))?;
                    let determination = state
                        .determinations
                        .iter()
                        .find(|d| d.id == saved_work.determination_id)
                        .ok_or(RuntimeError::Governance(RefusalCode::TargetRejected))?;
                    let mut provenance = ProvenanceRecord::blank(ProvenanceKind::NoticeSent);
                    provenance.actor_id = determination
                        .signature
                        .as_ref()
                        .map(|s| s.supervisor_human.clone());
                    provenance.timestamp = receipt.qualified_at.clone();
                    provenance.data = Some(
                        serde_json::json!({"source":"deterministic","noticeId":notice_id,
                        "determinationId":determination.id,"snapshotSha256":determination.snapshot.sha256,
                        "machineReadable":artifact["machineReadable"],"acknowledgement":acknowledgement,"receipt":receipt}),
                    );
                    super::super::provenance::populate_process_provenance_fields(
                        std::slice::from_mut(&mut provenance),
                        &kernel,
                        &record.process,
                    )?;
                    record.provenance_log.push(provenance);
                    record.process.provenance_position = record.provenance_log.len() as u64;
                }
            }
        }
        record.process.updated_at = now;
        operations::retain(&mut record, state);
        self.store.save_record(record)?;
        Ok(())
    }
}
fn admit_ack(work: &PendingNoticeWork, ack: &TransportAcknowledgement, now: &str) -> bool {
    acknowledgement_matches(work, ack)
        && admission::timestamp(&ack.accepted_at)
            .ok()
            .is_some_and(|time| {
                admission::timestamp(&work.issue_at)
                    .ok()
                    .is_some_and(|issue| time >= issue)
                    && admission::timestamp(now)
                        .ok()
                        .is_some_and(|now| time <= now)
            })
}
