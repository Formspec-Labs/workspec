// Rust guideline compliant 2026-02-21

//! Trellis-backed custody append orchestration.
//!
//! This module keeps the durable runtime trait stable. Callers still obtain
//! custody append windows through [`crate::DurableRuntime`] and stamp receipts
//! through the same trait, while the append itself is delegated to the shared
//! Trellis substrate client.

use stack_common_http::tenant::TenantScope;
use trellis_service_client::{
    AppendActor, ComputeContext, SubstrateAppendResult, SubstrateClient, SubstrateClientExt,
    WosProvenanceAppend,
};
use trellis_types::ArtifactType;
use wos_events::ProvenanceRecord;
use wos_events::custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendReceipt,
};

use crate::DurableRuntime;
use crate::runtime::RuntimeError;

const IDEMPOTENCY_KEY_PREFIX: &str = "wos-runtime:provenance";

/// Appends WOS custody windows through Trellis.
///
/// The appender is deliberately small: it owns only request routing context
/// and a borrowed Trellis substrate client. Runtime state stays behind the
/// existing [`crate::DurableRuntime`] trait.
pub struct TrellisCustodyAppender<'a> {
    client: &'a dyn SubstrateClient,
    tenant_scope: TenantScope,
    actor: AppendActor,
    compute_context: ComputeContext,
}

impl<'a> TrellisCustodyAppender<'a> {
    /// Creates a Trellis custody appender.
    #[must_use]
    pub fn new(
        client: &'a dyn SubstrateClient,
        tenant_scope: TenantScope,
        actor: AppendActor,
        compute_context: ComputeContext,
    ) -> Self {
        Self {
            client,
            tenant_scope,
            actor,
            compute_context,
        }
    }

    /// Appends and stamps one durable custody window.
    ///
    /// The method first asks `runtime` for a custody append window. Records
    /// that already carry a Trellis receipt are skipped; remaining records are
    /// appended with a stable WOS idempotency key and then stamped through
    /// [`crate::DurableRuntime::apply_custody_receipt`].
    ///
    /// # Errors
    /// Returns an error when the runtime cannot load or stamp records, when a
    /// custody record cannot be decoded as WOS provenance, or when the Trellis
    /// append fails or returns an invalid receipt.
    pub async fn append_window<R>(
        &self,
        runtime: &mut R,
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<TrellisCustodyAppendOutcome>, RuntimeError>
    where
        R: DurableRuntime + ?Sized,
    {
        let inputs = runtime.load_custody_append_window(process_id, cursor, limit, context)?;
        let mut outcomes = Vec::with_capacity(inputs.len());
        for input in inputs {
            if let Some((outcome, receipt)) = self.append_input(input).await? {
                runtime.apply_custody_receipt(process_id, &outcome.record_id, receipt)?;
                outcomes.push(outcome);
            }
        }
        Ok(outcomes)
    }

    async fn append_input(
        &self,
        input: CustodyAppendInput,
    ) -> Result<Option<(TrellisCustodyAppendOutcome, CustodyAppendReceipt)>, RuntimeError> {
        let record = provenance_record_from_input(&input)?;
        validate_input_record_metadata(&input, &record)?;
        if let Some(canonical_event_hash) = record.canonical_event_hash.as_deref() {
            if canonical_event_hash.trim().is_empty() {
                return Err(RuntimeError::Service(format!(
                    "custody record {} already carries an empty canonical_event_hash",
                    input.record_id
                )));
            }
            return Ok(None);
        }
        let result = self
            .client
            .append_wos_provenance(WosProvenanceAppend {
                scope: input.case_id.clone(),
                tenant_scope: self.tenant_scope.clone(),
                idempotency_key: trellis_idempotency_key(&input),
                actor: self.actor.clone(),
                record,
                compute_context: self.compute_context.clone(),
            })
            .await
            .map_err(|error| {
                RuntimeError::Service(format!(
                    "trellis custody append failed for record {}: {error}",
                    input.record_id
                ))
            })?;
        validate_trellis_result(&input, &result)?;
        let receipt = CustodyAppendReceipt::new(result.canonical_event_hash.clone());
        Ok(Some((
            TrellisCustodyAppendOutcome::from_result(input, result),
            receipt,
        )))
    }
}

/// Trellis append result projected for WOS runtime callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrellisCustodyAppendOutcome {
    /// WOS case ledger scope used for the Trellis append.
    pub case_id: String,
    /// WOS provenance record identifier stamped by the receipt.
    pub record_id: String,
    /// Trellis event type admitted for the WOS record.
    pub event_type: String,
    /// Trellis event identifier.
    pub event_id: String,
    /// Trellis sequence assigned within the case scope.
    pub sequence: u64,
    /// Trellis canonical event hash text.
    pub canonical_event_hash: String,
    /// Trellis checkpoint reference.
    pub checkpoint_ref: String,
    /// Trellis proof bundle reference.
    pub bundle_ref: String,
}

impl TrellisCustodyAppendOutcome {
    fn from_result(input: CustodyAppendInput, result: SubstrateAppendResult) -> Self {
        Self {
            case_id: input.case_id,
            record_id: input.record_id,
            event_type: input.event_type,
            event_id: result.event_id,
            sequence: result.sequence,
            canonical_event_hash: result.canonical_event_hash,
            checkpoint_ref: result.checkpoint_ref,
            bundle_ref: result.bundle_ref,
        }
    }
}

fn provenance_record_from_input(
    input: &CustodyAppendInput,
) -> Result<ProvenanceRecord, RuntimeError> {
    let json = input.record_json_view()?;
    serde_json::from_value(json).map_err(|error| {
        RuntimeError::CustodyAppend(CustodyAppendError::JsonSerialization(format!(
            "failed to decode custody record JSON for Trellis append: {error}"
        )))
    })
}

fn validate_input_record_metadata(
    input: &CustodyAppendInput,
    record: &ProvenanceRecord,
) -> Result<(), RuntimeError> {
    if record.id != input.record_id {
        return Err(RuntimeError::Service(format!(
            "custody input recordId {} does not match payload record id {}",
            input.record_id, record.id
        )));
    }
    let Some(expected_event_type) = record.record_kind.canonical_event_literal() else {
        return Err(RuntimeError::Service(format!(
            "custody record {} kind {:?} is not registered for Trellis WOS admission",
            input.record_id, record.record_kind
        )));
    };
    if input.event_type != expected_event_type {
        return Err(RuntimeError::Service(format!(
            "custody input eventType {} does not match payload kind {:?}",
            input.event_type, record.record_kind
        )));
    }
    let record_event = record.event.as_deref().or(Some(expected_event_type));
    if record_event != Some(input.event_type.as_str()) {
        return Err(RuntimeError::Service(format!(
            "custody payload event {:?} does not match input eventType {}",
            record.event, input.event_type
        )));
    }
    Ok(())
}

fn trellis_idempotency_key(input: &CustodyAppendInput) -> String {
    format!(
        "{IDEMPOTENCY_KEY_PREFIX}:{}:{}",
        input.case_id, input.record_id
    )
}

fn validate_trellis_result(
    input: &CustodyAppendInput,
    result: &SubstrateAppendResult,
) -> Result<(), RuntimeError> {
    if !is_sha256_hash_text(&result.canonical_event_hash) {
        return Err(RuntimeError::Service(format!(
            "trellis custody append returned an invalid canonical_event_hash for record {}",
            input.record_id
        )));
    }
    if result.event_id.trim().is_empty() {
        return Err(RuntimeError::Service(format!(
            "trellis custody append returned an empty event_id for record {}",
            input.record_id
        )));
    }
    let checkpoint_prefix = format!("trellis://{}/checkpoints/", input.case_id);
    if !result.checkpoint_ref.starts_with(&checkpoint_prefix) {
        return Err(RuntimeError::Service(format!(
            "trellis custody append checkpoint_ref is outside case scope {} for record {}",
            input.case_id, input.record_id
        )));
    }
    if result.bundle_ref.trim().is_empty() {
        return Err(RuntimeError::Service(format!(
            "trellis custody append returned an empty bundle_ref for record {}",
            input.record_id
        )));
    }
    if !result.verification_receipt.verified {
        return Err(RuntimeError::Service(format!(
            "trellis custody append returned an unverified receipt for record {}",
            input.record_id
        )));
    }
    if result.verification_receipt.artifact_type != ArtifactType::Event {
        return Err(RuntimeError::Service(format!(
            "trellis custody append receipt artifact type {} is not event for record {}",
            result.verification_receipt.artifact_type, input.record_id
        )));
    }
    if result.verification_receipt.event_type != input.event_type {
        return Err(RuntimeError::Service(format!(
            "trellis custody append receipt event type {} does not match requested {} for record {}",
            result.verification_receipt.event_type, input.event_type, input.record_id
        )));
    }
    Ok(())
}

fn is_sha256_hash_text(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use stack_common_error::StackError;
    use stack_common_typeid as typeid;
    use trellis_service_client::{SubstrateAppendRequest, VerificationReceipt};
    use wos_core::instance::{PendingEvent, WorkflowProcess};

    use crate::intake::{IntakeAcceptanceDecision, IntakeAcceptanceRequest};
    use crate::runtime::{
        CreateProcessRequest, DrainOnceResult, PersistDraftResult, TaskSubmissionResult,
    };

    #[tokio::test]
    async fn append_window_delegates_to_trellis_and_stamps_receipt() {
        let case_id = typeid::mint_case_ledger_id();
        let record = ProvenanceRecord::state_transition("open", "closed", "close", Some("actor"));
        let context = CustodyAppendContext {
            event_type_prefix: "wos.kernel".to_string(),
            case_id: Some(case_id.clone()),
            max_inline_record_bytes: None,
            workflow_ref: Some("urn:test:trellis-custody".to_string()),
        };
        let metadata = context
            .metadata_for_provenance_record(&case_id, 0, &record)
            .expect("metadata");
        let input = CustodyAppendInput::from_provenance_record(&record, &context, metadata)
            .expect("custody input");
        let record_id = input.record_id.clone();
        let client = RecordingSubstrateClient::default();
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = TrellisCustodyAppender::new(
            &client,
            TenantScope {
                tenant: "tenant-one".to_string(),
                workspace: "workspace-one".to_string(),
                environment: "test".to_string(),
                cell: "cell-one".to_string(),
            },
            AppendActor::service("wos-runtime"),
            ComputeContext::no_delegated_compute("wos-runtime"),
        );

        let outcomes = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect("append window");

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].case_id, case_id);
        assert_eq!(outcomes[0].record_id, record_id);
        assert_eq!(outcomes[0].event_type, "wos.kernel.state_transition");
        assert_eq!(outcomes[0].canonical_event_hash, TEST_HASH);
        let requests = client.requests.lock().expect("requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].scope, outcomes[0].case_id);
        assert_eq!(
            requests[0].idempotency_key,
            format!(
                "{IDEMPOTENCY_KEY_PREFIX}:{}:{}",
                outcomes[0].case_id, outcomes[0].record_id
            )
        );
        assert_eq!(requests[0].tenant_scope.tenant, "tenant-one");
        assert_eq!(requests[0].event_type, "wos.kernel.state_transition");
        let applied = runtime.applied.lock().expect("applied");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].0, "process-1");
        assert_eq!(applied[0].1, record_id);
        assert_eq!(applied[0].2.canonical_event_hash, TEST_HASH);
    }

    #[tokio::test]
    async fn append_window_skips_records_that_already_have_trellis_receipts() {
        let case_id = typeid::mint_case_ledger_id();
        let mut record =
            ProvenanceRecord::state_transition("open", "closed", "close", Some("actor"));
        record.canonical_event_hash = Some(TEST_HASH.to_string());
        let context = CustodyAppendContext {
            event_type_prefix: "wos.kernel".to_string(),
            case_id: Some(case_id.clone()),
            max_inline_record_bytes: None,
            workflow_ref: Some("urn:test:trellis-custody-replay".to_string()),
        };
        let metadata = context
            .metadata_for_provenance_record(&case_id, 0, &record)
            .expect("metadata");
        let input = CustodyAppendInput::from_provenance_record(&record, &context, metadata)
            .expect("custody input");
        let client = RecordingSubstrateClient::default();
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = TrellisCustodyAppender::new(
            &client,
            TenantScope {
                tenant: "tenant-one".to_string(),
                workspace: "workspace-one".to_string(),
                environment: "test".to_string(),
                cell: "cell-one".to_string(),
            },
            AppendActor::service("wos-runtime"),
            ComputeContext::no_delegated_compute("wos-runtime"),
        );

        let outcomes = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect("append window");

        assert!(outcomes.is_empty());
        assert!(client.requests.lock().expect("requests").is_empty());
        assert!(runtime.applied.lock().expect("applied").is_empty());
    }

    #[tokio::test]
    async fn append_window_rejects_payload_record_id_mismatch_before_trellis_call() {
        let context = test_context(typeid::mint_case_ledger_id());
        let mut input = test_input(&context);
        input.record_id = typeid::mint_provenance_id();
        let client = RecordingSubstrateClient::default();
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = test_appender(&client);

        let err = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect_err("metadata mismatch must fail");

        assert!(
            matches!(err, RuntimeError::Service(message) if message.contains("does not match payload record id"))
        );
        assert!(client.requests.lock().expect("requests").is_empty());
        assert!(runtime.applied.lock().expect("applied").is_empty());
    }

    #[tokio::test]
    async fn append_window_rejects_event_type_mismatch_before_trellis_call() {
        let context = test_context(typeid::mint_case_ledger_id());
        let mut input = test_input(&context);
        input.event_type = "wos.kernel.case_created".to_string();
        let client = RecordingSubstrateClient::default();
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = test_appender(&client);

        let err = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect_err("event mismatch must fail");

        assert!(
            matches!(err, RuntimeError::Service(message) if message.contains("does not match payload kind"))
        );
        assert!(client.requests.lock().expect("requests").is_empty());
        assert!(runtime.applied.lock().expect("applied").is_empty());
    }

    #[tokio::test]
    async fn append_window_accepts_event_artifact_type() {
        let context = test_context(typeid::mint_case_ledger_id());
        let input = test_input(&context);
        let client = RecordingSubstrateClient::default();
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = test_appender(&client);

        let outcomes = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect("event artifact_type must pass");

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].event_type, "wos.kernel.state_transition");
        assert_eq!(client.requests.lock().expect("requests").len(), 1);
        assert_eq!(runtime.applied.lock().expect("applied").len(), 1);
    }

    #[tokio::test]
    async fn append_window_rejects_event_type_echo_mismatch_without_stamping() {
        let context = test_context(typeid::mint_case_ledger_id());
        let input = test_input(&context);
        let client = RecordingSubstrateClient::with_event_type("wos.kernel.case_created");
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = test_appender(&client);

        let err = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect_err("event_type echo mismatch must fail");

        assert!(
            matches!(err, RuntimeError::Service(message) if message.contains("receipt event type") && message.contains("does not match requested"))
        );
        assert_eq!(client.requests.lock().expect("requests").len(), 1);
        assert!(runtime.applied.lock().expect("applied").is_empty());
    }

    #[tokio::test]
    async fn append_window_rejects_non_event_artifact_type_without_stamping() {
        let context = test_context(typeid::mint_case_ledger_id());
        let input = test_input(&context);
        let client = RecordingSubstrateClient::with_artifact_type(ArtifactType::Checkpoint);
        let mut runtime = RecordingDurableRuntime::new(input);
        let appender = test_appender(&client);

        let err = appender
            .append_window(&mut runtime, "process-1", 0, 25, context)
            .await
            .expect_err("non-event artifact_type must fail");

        assert!(
            matches!(err, RuntimeError::Service(message) if message.contains("receipt artifact type") && message.contains("is not event"))
        );
        assert_eq!(client.requests.lock().expect("requests").len(), 1);
        assert!(runtime.applied.lock().expect("applied").is_empty());
    }

    const TEST_HASH: &str =
        "sha256:9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c";

    fn test_context(case_id: String) -> CustodyAppendContext {
        CustodyAppendContext {
            event_type_prefix: "wos.kernel".to_string(),
            case_id: Some(case_id),
            max_inline_record_bytes: None,
            workflow_ref: Some("urn:test:trellis-custody".to_string()),
        }
    }

    fn test_input(context: &CustodyAppendContext) -> CustodyAppendInput {
        let case_id = context.case_id.as_deref().expect("case id");
        let record = ProvenanceRecord::state_transition("open", "closed", "close", Some("actor"));
        let metadata = context
            .metadata_for_provenance_record(case_id, 0, &record)
            .expect("metadata");
        CustodyAppendInput::from_provenance_record(&record, context, metadata)
            .expect("custody input")
    }

    fn test_appender<'a>(client: &'a RecordingSubstrateClient) -> TrellisCustodyAppender<'a> {
        TrellisCustodyAppender::new(
            client,
            TenantScope {
                tenant: "tenant-one".to_string(),
                workspace: "workspace-one".to_string(),
                environment: "test".to_string(),
                cell: "cell-one".to_string(),
            },
            AppendActor::service("wos-runtime"),
            ComputeContext::no_delegated_compute("wos-runtime"),
        )
    }

    struct RecordingSubstrateClient {
        requests: std::sync::Mutex<Vec<SubstrateAppendRequest>>,
        artifact_type: ArtifactType,
        event_type_override: Option<String>,
    }

    impl Default for RecordingSubstrateClient {
        fn default() -> Self {
            Self {
                requests: std::sync::Mutex::new(Vec::new()),
                artifact_type: ArtifactType::Event,
                event_type_override: None,
            }
        }
    }

    impl RecordingSubstrateClient {
        fn with_artifact_type(artifact_type: ArtifactType) -> Self {
            Self {
                artifact_type,
                ..Self::default()
            }
        }

        fn with_event_type(event_type: impl Into<String>) -> Self {
            Self {
                event_type_override: Some(event_type.into()),
                ..Self::default()
            }
        }
    }

    #[async_trait]
    impl SubstrateClient for RecordingSubstrateClient {
        async fn append_event(
            &self,
            request: SubstrateAppendRequest,
        ) -> Result<SubstrateAppendResult, StackError> {
            self.requests
                .lock()
                .expect("requests")
                .push(request.clone());
            let receipt_event_type = self
                .event_type_override
                .clone()
                .unwrap_or_else(|| request.event_type.clone());
            Ok(SubstrateAppendResult {
                event_id: "evt_test".to_string(),
                sequence: 0,
                canonical_event_hash: TEST_HASH.to_string(),
                checkpoint_ref: format!("trellis://{}/checkpoints/cp_test", request.scope),
                bundle_ref: "s3://test-bucket/bundle.zip".to_string(),
                verification_receipt: VerificationReceipt {
                    verified: true,
                    artifact_type: self.artifact_type,
                    event_type: receipt_event_type,
                },
            })
        }

        async fn head_bundle(
            &self,
            _scope: &str,
            _tenant_scope: &TenantScope,
        ) -> Result<Vec<u8>, StackError> {
            unreachable!("not used by custody append test")
        }

        async fn bundle(
            &self,
            _scope: &str,
            _checkpoint_digest: &str,
            _tenant_scope: &TenantScope,
        ) -> Result<Vec<u8>, StackError> {
            unreachable!("not used by custody append test")
        }

        async fn signing_key_registry(
            &self,
            _scope: &str,
            _tenant_scope: &TenantScope,
        ) -> Result<Vec<u8>, StackError> {
            unreachable!("not used by custody append test")
        }

        async fn event_type_registry(
            &self,
            _scope: &str,
            _tenant_scope: &TenantScope,
        ) -> Result<serde_json::Value, StackError> {
            unreachable!("not used by custody append test")
        }
    }

    struct RecordingDurableRuntime {
        inputs: Vec<CustodyAppendInput>,
        applied: std::sync::Mutex<Vec<(String, String, CustodyAppendReceipt)>>,
    }

    impl RecordingDurableRuntime {
        fn new(input: CustodyAppendInput) -> Self {
            Self {
                inputs: vec![input],
                applied: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl DurableRuntime for RecordingDurableRuntime {
        fn create_process(
            &mut self,
            _request: CreateProcessRequest,
        ) -> Result<WorkflowProcess, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn load_process(&self, _process_id: &str) -> Result<WorkflowProcess, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn enqueue_event(
            &mut self,
            _process_id: &str,
            _event: PendingEvent,
        ) -> Result<(), RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn drain_once(&mut self, _process_id: &str) -> Result<DrainOnceResult, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn drain_until_idle(
            &mut self,
            _process_id: &str,
        ) -> Result<Vec<DrainOnceResult>, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn persist_task_draft(
            &mut self,
            _task_id: &str,
            _response: serde_json::Value,
            _actor_id: &str,
            _idempotency_token: Option<&str>,
        ) -> Result<PersistDraftResult, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn dismiss_task(&mut self, _task_id: &str, _reason: &str) -> Result<(), RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn submit_task_response(
            &mut self,
            _task_id: &str,
            _response: serde_json::Value,
            _actor_id: &str,
            _idempotency_token: Option<&str>,
        ) -> Result<TaskSubmissionResult, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn accept_intake_handoff(
            &mut self,
            _binding: &str,
            _request: IntakeAcceptanceRequest,
        ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn load_provenance_window(
            &self,
            _process_id: &str,
            _cursor: usize,
            _limit: usize,
        ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
            unreachable!("not used by custody append test")
        }

        fn load_custody_append_window(
            &self,
            _process_id: &str,
            cursor: usize,
            limit: usize,
            _context: CustodyAppendContext,
        ) -> Result<Vec<CustodyAppendInput>, RuntimeError> {
            Ok(self
                .inputs
                .iter()
                .skip(cursor)
                .take(limit)
                .cloned()
                .collect())
        }

        fn apply_custody_receipt(
            &mut self,
            process_id: &str,
            record_id: &str,
            receipt: CustodyAppendReceipt,
        ) -> Result<(), RuntimeError> {
            self.applied.lock().expect("applied").push((
                process_id.to_string(),
                record_id.to_string(),
                receipt,
            ));
            Ok(())
        }
    }
}
