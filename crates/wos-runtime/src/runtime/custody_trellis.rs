// Rust guideline compliant 2026-02-21

//! Trellis-backed custody append orchestration.
//!
//! This module keeps the durable runtime trait stable. Callers still obtain
//! custody append windows through [`crate::DurableRuntime`] and stamp receipts
//! through the same trait, while the append itself is delegated to the shared
//! Trellis substrate client.

use async_trait::async_trait;
use stack_common_http::tenant::TenantScope;
use trellis_service_client::{
    AppendActor, ComputeContext, SubstrateAppendRequest, SubstrateAppendResult, SubstrateClient,
    wos_provenance_append_request,
};
use trellis_types::ArtifactType;
use wos_events::custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendReceipt,
};
use wos_events::{NativeProvenanceEvent, ProvenanceRecord, WOS_NATIVE_PROVENANCE_EVENT_LITERAL};

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
    ledger_scope: Option<String>,
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
            ledger_scope: None,
        }
    }

    /// Creates an appender with independently selected immutable ledger routing.
    ///
    /// The host must qualify this namespace from original protected evidence.
    /// Native case identity and stable append keys remain unchanged.
    ///
    /// # Errors
    /// Refuses blank routing, NUL or a namespace containing a slash.
    pub fn new_for_ledger_scope(
        client: &'a dyn SubstrateClient,
        tenant_scope: TenantScope,
        actor: AppendActor,
        compute_context: ComputeContext,
        ledger_scope: String,
    ) -> Result<Self, RuntimeError> {
        // Match the client's documented single-segment scope rule. The actual
        // request constructor revalidates routing before any network call.
        if ledger_scope.trim().is_empty()
            || ledger_scope.contains('/')
            || ledger_scope.contains('\0')
        {
            return Err(RuntimeError::Service(
                "custody ledger scope must be a nonblank single segment".into(),
            ));
        }
        let mut appender = Self::new(client, tenant_scope, actor, compute_context);
        appender.ledger_scope = Some(ledger_scope);
        Ok(appender)
    }

    /// Appends custody records with current admission and owner persistence.
    ///
    /// Requires independently selected routing. The handler commits the complete
    /// verified association and native hash atomically; this method never calls
    /// the legacy hash-only stamp. It stops on the first error and never resends.
    ///
    /// # Errors
    /// Retains actual verified results when post-append admission or persistence
    /// fails. A persistence error may follow a committed transaction; callers
    /// must qualify current authorized readback before deciding recovery.
    pub async fn append_window_with_handler<R>(
        &self,
        runtime: &mut R,
        handler: &dyn TrellisCustodyAppendHandler,
        process_id: &str,
        cursor: usize,
        limit: usize,
        context: CustodyAppendContext,
    ) -> Result<Vec<TrellisCustodyAppendOutcome>, TrellisCustodyAppendFailure>
    where
        R: DurableRuntime + ?Sized,
    {
        let scope = self.ledger_scope.as_deref().ok_or_else(|| {
            TrellisCustodyAppendFailure::new(
                TrellisCustodyAppendStage::BeforeAppend,
                RuntimeError::Service(
                    "custody handler requires independently selected ledger scope".into(),
                ),
            )
        })?;
        let inputs = runtime
            .load_custody_append_window(process_id, cursor, limit, context)
            .map_err(|error| {
                TrellisCustodyAppendFailure::new(TrellisCustodyAppendStage::BeforeAppend, error)
            })?;
        let mut persisted = Vec::with_capacity(inputs.len());
        for input in inputs {
            let appended = match self
                .append_input_admitted(input, scope, Some(handler))
                .await
            {
                Ok(value) => value,
                Err(mut failure) => {
                    failure.persisted_outcomes = persisted;
                    return Err(failure);
                }
            };
            if let Some((outcome, _receipt)) = appended {
                if let Err(error) = handler.persist_verified(&outcome, scope).await {
                    return Err(TrellisCustodyAppendFailure {
                        stage: TrellisCustodyAppendStage::Persistence,
                        error,
                        verified_outcome: Some(outcome),
                        persisted_outcomes: persisted,
                    });
                }
                persisted.push(outcome);
            }
        }
        Ok(persisted)
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
        let scope = self
            .ledger_scope
            .as_deref()
            .unwrap_or(&input.case_id)
            .to_owned();
        self.append_input_admitted(input, &scope, None)
            .await
            .map_err(|failure| failure.error)
    }

    async fn append_input_admitted(
        &self,
        input: CustodyAppendInput,
        ledger_scope: &str,
        handler: Option<&dyn TrellisCustodyAppendHandler>,
    ) -> Result<
        Option<(TrellisCustodyAppendOutcome, CustodyAppendReceipt)>,
        TrellisCustodyAppendFailure,
    > {
        let request = self.prepare_input(&input, ledger_scope).map_err(|error| {
            TrellisCustodyAppendFailure::new(TrellisCustodyAppendStage::BeforeAppend, error)
        })?;
        let Some(request) = request else {
            return Ok(None);
        };
        if let Some(handler) = handler {
            handler
                .authorize_before_append(&input, ledger_scope)
                .await
                .map_err(|error| {
                    TrellisCustodyAppendFailure::new(TrellisCustodyAppendStage::BeforeAppend, error)
                })?;
        }
        let result = self.client.append_event(request).await.map_err(|error| {
            TrellisCustodyAppendFailure::new(
                TrellisCustodyAppendStage::Append,
                RuntimeError::Service(format!(
                    "trellis custody append failed for record {}: {error}",
                    input.record_id
                )),
            )
        })?;
        validate_trellis_result_for_scope(&input, &result, ledger_scope, handler.is_some())
            .map_err(|error| {
                TrellisCustodyAppendFailure::new(TrellisCustodyAppendStage::ResultValidation, error)
            })?;
        let receipt = CustodyAppendReceipt::new(result.canonical_event_hash.clone());
        let outcome = TrellisCustodyAppendOutcome::from_result(input.clone(), result);
        if let Some(handler) = handler {
            if let Err(error) = handler
                .authorize_after_append(&input, &outcome, ledger_scope)
                .await
            {
                return Err(TrellisCustodyAppendFailure {
                    stage: TrellisCustodyAppendStage::AfterAppendAdmission,
                    error,
                    verified_outcome: Some(outcome),
                    persisted_outcomes: Vec::new(),
                });
            }
        }
        Ok(Some((outcome, receipt)))
    }

    fn prepare_input(
        &self,
        input: &CustodyAppendInput,
        ledger_scope: &str,
    ) -> Result<Option<SubstrateAppendRequest>, RuntimeError> {
        if input.case_id.contains('\0') || input.record_id.contains('\0') {
            return Err(RuntimeError::Service(
                "custody case and record identifiers must not contain NUL".to_string(),
            ));
        }
        let request = if input.event_type == WOS_NATIVE_PROVENANCE_EVENT_LITERAL {
            // Validate original authored bytes before an inspection or typed
            // deserializer can collapse keys, interpret tags or drop fields.
            let native = NativeProvenanceEvent::from_record_dcbor_slice(input.record_bytes())
                .map_err(|error| {
                    RuntimeError::Service(format!("native custody record refused: {error}"))
                })?;
            if native.record_id() != input.record_id {
                return Err(RuntimeError::Service(format!(
                    "custody input recordId {} does not match payload record id {}",
                    input.record_id,
                    native.record_id()
                )));
            }
            if custody_record_is_stamped(
                input,
                native
                    .record()
                    .get("canonicalEventHash")
                    .and_then(serde_json::Value::as_str),
            )? {
                return Ok(None);
            }
            SubstrateAppendRequest::new_json(
                ledger_scope,
                self.tenant_scope.clone(),
                input.event_type.clone(),
                trellis_idempotency_key(input),
                self.actor.clone(),
                native.payload().clone(),
                self.compute_context.clone(),
            )
        } else {
            let record = provenance_record_from_input(input)?;
            validate_input_record_metadata(input, &record)?;
            if custody_record_is_stamped(input, record.canonical_event_hash.as_deref())? {
                return Ok(None);
            }
            wos_provenance_append_request(
                ledger_scope,
                self.tenant_scope.clone(),
                trellis_idempotency_key(input),
                self.actor.clone(),
                record,
                self.compute_context.clone(),
            )
        }
        .map_err(|error| RuntimeError::Service(format!("custody request refused: {error}")))?;
        Ok(Some(request))
    }
}

/// Current authority and atomic persistence supplied by the owning host.
///
/// Implementations retain original identities and recheck current permission
/// inside persistence after their awaits. Multi-record handlers advance their
/// protected expected row only through exact confirmed custody changes.
#[async_trait]
pub trait TrellisCustodyAppendHandler: Send + Sync {
    /// Checks original callers and routing immediately before the network append.
    ///
    /// # Errors
    /// Refusal prevents the append.
    async fn authorize_before_append(
        &self,
        input: &CustodyAppendInput,
        ledger_scope: &str,
    ) -> Result<(), RuntimeError>;

    /// Rechecks current permission after actual append and result validation.
    ///
    /// # Errors
    /// Refusal prevents owner persistence and retains the verified append result.
    async fn authorize_after_append(
        &self,
        input: &CustodyAppendInput,
        outcome: &TrellisCustodyAppendOutcome,
        ledger_scope: &str,
    ) -> Result<(), RuntimeError>;

    /// Atomically persists the complete verified association and native stamp.
    ///
    /// # Errors
    /// May refuse before or after commit. An error does not prove no local stamp;
    /// the owner must qualify readback without automatic append recovery.
    async fn persist_verified(
        &self,
        outcome: &TrellisCustodyAppendOutcome,
        ledger_scope: &str,
    ) -> Result<(), RuntimeError>;
}

/// Last boundary reached by a refused custody append window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrellisCustodyAppendStage {
    /// Input, routing, window loading or pre-append authority was refused.
    BeforeAppend,
    /// Network append failed; external append status can be unknown.
    Append,
    /// Actual append returned a result that did not qualify as verified.
    ResultValidation,
    /// Verified append exists, but post-append permission was refused.
    AfterAppendAdmission,
    /// Owner persistence failed and may already have committed.
    Persistence,
}

/// Custody failure retaining observed effects without granting authority.
#[derive(Debug, thiserror::Error)]
#[error("custody append failed at {stage:?}: {error}")]
pub struct TrellisCustodyAppendFailure {
    /// Last boundary reached; Persistence does not establish rollback.
    pub stage: TrellisCustodyAppendStage,
    /// Original refusal or service error.
    #[source]
    pub error: RuntimeError,
    /// Actual verified result when admission or persistence failed after append.
    pub verified_outcome: Option<TrellisCustodyAppendOutcome>,
    /// Earlier outcomes confirmed persisted by successful handler calls.
    pub persisted_outcomes: Vec<TrellisCustodyAppendOutcome>,
}

impl TrellisCustodyAppendFailure {
    fn new(stage: TrellisCustodyAppendStage, error: RuntimeError) -> Self {
        Self {
            stage,
            error,
            verified_outcome: None,
            persisted_outcomes: Vec::new(),
        }
    }
}

/// Trellis append result projected for WOS runtime callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrellisCustodyAppendOutcome {
    /// Original native WOS case ledger identity, independent of URL routing.
    pub case_id: String,
    /// WOS provenance record identifier stamped by the receipt.
    pub record_id: String,
    /// Trellis event type admitted for the WOS record.
    pub event_type: String,
    /// Trellis event identifier.
    pub event_id: String,
    /// Trellis sequence assigned within the selected ledger namespace.
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

fn custody_record_is_stamped(
    input: &CustodyAppendInput,
    canonical_event_hash: Option<&str>,
) -> Result<bool, RuntimeError> {
    match canonical_event_hash {
        Some(hash) if hash.trim().is_empty() => Err(RuntimeError::Service(format!(
            "custody record {} already carries an empty canonical_event_hash",
            input.record_id
        ))),
        Some(_) => Ok(true),
        None => Ok(false),
    }
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
    // Trellis Core §17.2 permits at most 64 wire bytes. Hash the stable
    // domain/case/record tuple; retain original identities in the event payload.
    stack_common_hash::sha256_nul_separated_parts(&[
        IDEMPOTENCY_KEY_PREFIX,
        &input.case_id,
        &input.record_id,
    ])
}

fn validate_trellis_result_for_scope(
    input: &CustodyAppendInput,
    result: &SubstrateAppendResult,
    ledger_scope: &str,
    require_checkpoint_suffix: bool,
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
    let checkpoint_prefix = format!("trellis://{ledger_scope}/checkpoints/");
    if !result.checkpoint_ref.starts_with(&checkpoint_prefix)
        || (require_checkpoint_suffix && result.checkpoint_ref.len() == checkpoint_prefix.len())
    {
        return Err(RuntimeError::Service(format!(
            "trellis custody append checkpoint_ref is outside requested scope {} for record {}",
            ledger_scope, input.record_id
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
        CreateProcessRequest, DismissTaskResult, DrainOnceResult, PersistDraftResult,
        TaskSubmissionResult,
    };

    #[test]
    fn native_custody_key_is_bounded_stable_and_separates_case_and_record() {
        let case_id = typeid::mint_case_ledger_id();
        let record = ProvenanceRecord::state_transition("open", "closed", "close", Some("actor"));
        let context = CustodyAppendContext {
            event_type_prefix: "wos.kernel".into(),
            case_id: Some(case_id.clone()),
            max_inline_record_bytes: None,
            workflow_ref: Some("urn:test:custody-key".into()),
        };
        let metadata = context
            .metadata_for_provenance_record(&case_id, 0, &record)
            .unwrap();
        let input =
            CustodyAppendInput::from_provenance_record(&record, &context, metadata).unwrap();
        let key = trellis_idempotency_key(&input);
        assert!(trellis_types::idempotency_key_length_in_bound(
            key.as_bytes()
        ));
        assert_eq!(key.len(), 64);
        assert_eq!(trellis_idempotency_key(&input), key);
        let mut other_case = input.clone();
        other_case.case_id = typeid::mint_case_ledger_id();
        assert_ne!(trellis_idempotency_key(&other_case), key);
        let mut other_record = input.clone();
        other_record.record_id = typeid::mint_provenance_id();
        assert_ne!(trellis_idempotency_key(&other_record), key);
    }

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
            stack_common_hash::sha256_nul_separated_parts(&[
                IDEMPOTENCY_KEY_PREFIX,
                &outcomes[0].case_id,
                &outcomes[0].record_id,
            ])
        );
        assert!(trellis_types::idempotency_key_length_in_bound(
            requests[0].idempotency_key.as_bytes()
        ));
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
    async fn append_window_rejects_nul_identifiers_before_trellis_call() {
        for nul_in_case in [true, false] {
            let context = test_context(typeid::mint_case_ledger_id());
            let mut input = test_input(&context);
            if nul_in_case {
                input.case_id.push('\0');
            } else {
                input.record_id.push('\0');
            }
            let client = RecordingSubstrateClient::default();
            let mut runtime = RecordingDurableRuntime::new(input);
            let appender = test_appender(&client);

            let err = appender
                .append_window(&mut runtime, "process-1", 0, 25, context)
                .await
                .expect_err("NUL identifiers must fail");

            assert!(matches!(err, RuntimeError::Service(message)
                if message == "custody case and record identifiers must not contain NUL"));
            assert!(client.requests.lock().expect("requests").is_empty());
            assert!(runtime.applied.lock().expect("applied").is_empty());
        }
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

    include!("custody_admission_tests.rs");

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
                prior_event_hash: None,
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

        fn dismiss_task(
            &mut self,
            _task_id: &str,
            _reason: &str,
            _actor_id: &str,
            _idempotency_token: &str,
        ) -> Result<DismissTaskResult, RuntimeError> {
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
